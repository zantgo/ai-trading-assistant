<script lang="ts">
    import styles from './IndicatorCard.module.css';
    import { iRaw, iSub, fmt, fmtPrice, isSqueezeOn } from '../../lib/telemetry';
    import type { FineCategory } from '../../lib/decisionStages';
    import type { IndicatorDto, IndicatorMeta, IndicatorSignal, IndicatorMap, SignalDirection } from '../../types';

    interface Props {
        meta: IndicatorMeta;
        map: IndicatorMap | undefined | null;
        category: FineCategory;
        priceRef?: number;
        /** Optional normalized-value trajectory for the sparkline. */
        spark?: number[];
        /** Effective (regime-adjusted) weight, if known. */
        weight?: number | null;
    }
    let { meta, map, category, priceRef = 0, spark = [], weight = null }: Props = $props();

    let expanded = $state(false);

    const dto = $derived<IndicatorDto | undefined>(map ? map[meta.key] : undefined);
    const normalized = $derived(dto?.normalized ?? 0);
    const confidencePct = $derived(Math.round((dto?.confidence ?? 0) * 100));
    const stateLabel = $derived(dto?.state_label ?? 'UNKNOWN');
    const signals = $derived<IndicatorSignal[]>(dto?.signals ?? []);

    const RENDER_LABELS: Record<string, string> = {
        Pane: 'PANE', PriceOverlay: 'OVERLAY', PriceLevels: 'LEVELS', Marker: 'MARKER',
    };
    const SIGNAL_ABBR: Record<string, string> = {
        Divergence: 'DIV', Crossover: '✕', Threshold: 'TH', Breakout: 'BO',
        BandTouch: 'BT', ZeroLineCross: '0X', CompressionRelease: 'SQZ',
        LevelTest: 'LV', TrendFlip: 'FLIP', VolumeClimax: 'VOL',
        StackChange: 'STK', PatternForming: 'PAT',
    };
    const SUB_ABBR: Record<string, string> = {
        line: 'L', signal: 'S', histogram: 'H', histogram_peak: 'PK',
        fast: 'F', medium: 'M', slow: 'S', long: 'L',
        k_line: 'K', d_line: 'D', s_line: 'S',
        plus_di: '+DI', minus_di: '-DI', adx: 'ADX', adx_slope: 'SLP',
        vwap: 'VWAP', upper: 'UP', middle: 'MID', lower: 'LO', gp_top: 'GP', bb_mid: 'BB',
    };

    function rawVal(): number | null {
        if (meta.value_source.startsWith('sub:')) return iSub(map, meta.key, meta.value_source.slice(4));
        return iRaw(map, meta.key);
    }
    const rawText = $derived.by<string>(() => {
        if (meta.value_format === 'onoff') {
            return meta.key === 'squeeze'
                ? (isSqueezeOn(map) ? 'ON' : 'OFF')
                : (rawVal() != null ? 'ON' : 'OFF');
        }
        const v = rawVal();
        switch (meta.value_format) {
            case 'percent1': return v == null ? '--' : `${v.toFixed(1)}%`;
            case 'price': return fmtPrice(v, priceRef);
            case 'ratio2': return (v ?? 1).toFixed(2);
            case 'decimals1': return fmt(v, 1);
            case 'decimals4': return fmt(v, 4);
            default: return fmt(v, 2);
        }
    });

    // Dominant active signal: prefer Confirmed > Active > Potential, then strength.
    const STATUS_RANK: Record<string, number> = { Confirmed: 3, Active: 2, Potential: 1 };
    const dominant = $derived.by<IndicatorSignal | null>(() => {
        if (signals.length === 0) return null;
        return [...signals].sort((a, b) =>
            (STATUS_RANK[b.status] - STATUS_RANK[a.status]) || (b.strength - a.strength),
        )[0];
    });

    function dirColor(d: SignalDirection): string {
        return d === 'Bullish' ? '#10b981' : d === 'Bearish' ? '#ef4444' : '#94a3b8';
    }
    const normColor = $derived.by<string>(() => {
        const mag = Math.min(Math.abs(normalized), 1);
        if (mag >= 0.9) return '#a855f7';
        if (normalized > 0.1) return `rgb(16, ${Math.round(120 + 135 * mag)}, 129)`;
        if (normalized < -0.1) return `rgb(${Math.round(180 + 59 * mag)}, 68, 68)`;
        return '#94a3b8';
    });
    // Fill anchored at centre (0) extending toward the normalized value.
    const gaugeFillWidth = $derived(`${Math.min(Math.abs(normalized), 1) * 50}%`);
    const gaugeFillLeft = $derived(normalized >= 0 ? '50%' : `${50 - Math.min(Math.abs(normalized), 1) * 50}%`);

    const subChips = $derived.by<Array<{ label: string; text: string }>>(() => {
        const vals = dto?.values;
        if (!vals) return [];
        const chips = Object.entries(vals).map(([k, v]) => ({
            label: SUB_ABBR[k] ?? k.slice(0, 4).toUpperCase(),
            text: Number.isFinite(v)
                ? (Math.abs(v) >= 100 ? v.toFixed(1) : Math.abs(v) < 0.0001 && v !== 0 ? v.toExponential(2) : v.toFixed(2))
                : '--',
        }));
        return chips.length > 1 ? chips : [];
    });

    const capabilities = $derived.by<Array<{ text: string; active: boolean; color: string; title: string }>>(() => {
        const activeMap = new Map<string, SignalDirection>();
        for (const s of signals) activeMap.set(s.kind, s.direction);
        return meta.signal_types.map((st) => {
            const dir = activeMap.get(st);
            return {
                text: SIGNAL_ABBR[st] ?? st.slice(0, 3),
                active: dir !== undefined,
                color: dir ? dirColor(dir) : '#64748b',
                title: dir ? `${st} active (${dir})` : `${st} (inactive)`,
            };
        });
    });

    // Sparkline polyline points (normalized series → 0..100 × 0..24 box, y inverted).
    const sparkPoints = $derived.by<string>(() => {
        if (spark.length < 2) return '';
        const n = spark.length;
        return spark
            .map((v, i) => {
                const x = (i / (n - 1)) * 100;
                const y = 12 - Math.max(-1, Math.min(1, v)) * 11;
                return `${x.toFixed(1)},${y.toFixed(1)}`;
            })
            .join(' ');
    });

    function signalRowTitle(s: IndicatorSignal): string {
        const age = (s.age_bars ?? 0) === 0 ? 'now' : `${s.age_bars} bars ago`;
        return `${s.status} · strength ${(s.strength * 100).toFixed(0)}% · ${age}`;
    }
</script>

<div class={styles.card} style="border-left-color:{meta.color}">
    <button class={styles.glance} onclick={() => (expanded = !expanded)} aria-expanded={expanded}>
        <span class={styles.dot} style="background:{meta.color}"></span>
        <span class={styles.name}>{meta.display_name}</span>
        <span class={styles.classTag} data-class={meta.class}>{meta.class.slice(0, 3).toUpperCase()}</span>
        {#if !meta.directional}<span class={styles.gate} title="Non-directional gate">◐</span>{/if}

        {#if meta.directional}
            <span class={styles.gauge} title="Normalized {normalized.toFixed(2)}">
                <span class={styles.gaugeZero}></span>
                <span class={styles.gaugeFill} style="left:{gaugeFillLeft};width:{gaugeFillWidth};background:{normColor}"></span>
            </span>
            <span class={styles.normVal} style="color:{normColor}">
                {normalized >= 0 ? '+' : ''}{normalized.toFixed(2)}
            </span>
            <span class={styles.conf} title="Confidence">{confidencePct}%</span>
        {:else}
            <span class={styles.rawInline}>{rawText}</span>
        {/if}

        {#if dominant}
            <span class={styles.sig} style="color:{dirColor(dominant.direction)};border-color:{dirColor(dominant.direction)}"
                  title="{dominant.kind} · {dominant.direction} · {signalRowTitle(dominant)}">
                {SIGNAL_ABBR[dominant.kind] ?? dominant.kind}
            </span>
        {/if}
        <span class={styles.chevron} data-open={expanded}>▸</span>
    </button>

    {#if expanded}
        <div class={styles.drawer}>
            <div class={styles.metaRow}>
                <span class={styles.category}>{category}</span>
                <span class={styles.stateLabel} style="color:{normColor}">{stateLabel}</span>
                {#if meta.directional}<span class={styles.rawInline}>raw {rawText}</span>{/if}
            </div>

            {#if sparkPoints}
                <svg class={styles.spark} viewBox="0 0 100 24" preserveAspectRatio="none">
                    <line x1="0" y1="12" x2="100" y2="12" class={styles.sparkZero} />
                    <polyline points={sparkPoints} class={styles.sparkLine} style="stroke:{normColor}" />
                </svg>
            {/if}

            {#if subChips.length > 0}
                <div class={styles.chips}>
                    {#each subChips as c}
                        <span class={styles.chip}><span class={styles.chipKey}>{c.label}</span>{c.text}</span>
                    {/each}
                </div>
            {/if}

            {#if signals.length > 0}
                <div class={styles.signalList}>
                    {#each signals as s}
                        <div class={styles.signalRow} title={signalRowTitle(s)}>
                            <span class={styles.sig} style="color:{dirColor(s.direction)};border-color:{dirColor(s.direction)}">
                                {SIGNAL_ABBR[s.kind] ?? s.kind}
                            </span>
                            <span class={styles.signalKind}>{s.kind}</span>
                            <span class={styles.signalStatus} data-status={s.status}>{s.status}</span>
                            <span class={styles.signalAge}>{(s.age_bars ?? 0) === 0 ? 'now' : `${s.age_bars}b`}</span>
                        </div>
                    {/each}
                </div>
            {/if}

            {#if meta.signal_types.length > 0}
                <div class={styles.caps}>
                    <span class={styles.capsLabel}>signals:</span>
                    {#each capabilities as cap}
                        <span class={styles.cap} class:capOff={!cap.active} style="color:{cap.color};border-color:{cap.color}" title={cap.title}>{cap.text}</span>
                    {/each}
                </div>
            {/if}

            <div class={styles.footRow}>
                <span class={styles.footTag}>{RENDER_LABELS[meta.render] ?? meta.render}</span>
                {#if weight != null}<span class={styles.footTag}>w {weight.toFixed(2)}</span>{/if}
                {#if meta.supports_divergence}<span class={styles.footTag}>DIV-CAP</span>{/if}
            </div>
        </div>
    {/if}
</div>
