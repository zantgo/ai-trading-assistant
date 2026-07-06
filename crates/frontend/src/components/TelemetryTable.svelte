<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import styles from './TelemetryTable.module.css';
    import { iRaw, iSub, fmt, fmtPrice, isSqueezeOn } from '../lib/telemetry';
    import type { TimeframeTelemetry, IndicatorMeta, IndicatorSignal } from '../types';

    const app = useAppStore();
    let { pairKey }: { pairKey: string } = $props();
    const pair = $derived(app.instancesMap[pairKey]);
    const registry = $derived<IndicatorMeta[]>((app.indicatorRegistry ?? []) as IndicatorMeta[]);
    let copied = $state(false);
    let expandedTfTable = $state<string | null>(null);
    let groupMode = $state<'class' | 'group'>('class');

    const timeframes = ['microTerm', 'fastTerm', 'slowTerm', 'macroTerm'] as const;

    const CLASS_ORDER = ['Leading', 'Hybrid', 'Lagging'] as const;
    const GROUP_ORDER = ['Trend', 'Momentum', 'Volume', 'Volatility', 'Structure', 'Regime', 'Advanced'] as const;

    // Registry-driven buckets: group by class (Leading/Hybrid/Lagging) or by
    // functional group (Trend/Momentum/...). Preserves registry ordering within.
    const buckets = $derived.by<Array<[string, IndicatorMeta[]]>>(() => {
        const field: 'class' | 'group' = groupMode;
        const order = (groupMode === 'class' ? CLASS_ORDER : GROUP_ORDER) as readonly string[];
        const map = new Map<string, IndicatorMeta[]>();
        for (const b of order) map.set(b, []);
        for (const m of registry) {
            const b = m[field];
            if (!map.has(b)) map.set(b, []);
            map.get(b)!.push(m);
        }
        return order.filter((b) => (map.get(b)?.length ?? 0) > 0).map((b) => [b, map.get(b)!]);
    });

    // ── Pure formatting/label helpers (no reactivity concerns) ──
    function pxf(tf: TimeframeTelemetry, v: number | null): string {
        return fmtPrice(v, parseFloat(tf.priceText) || 0);
    }
    function rawVal(meta: IndicatorMeta, tf: TimeframeTelemetry): number | null {
        if (meta.value_source.startsWith('sub:')) {
            return iSub(tf.indicators, meta.key, meta.value_source.slice(4));
        }
        return iRaw(tf.indicators, meta.key);
    }
    function formatRaw(meta: IndicatorMeta, tf: TimeframeTelemetry): string {
        if (meta.value_format === 'onoff') {
            return meta.key === 'squeeze'
                ? (isSqueezeOn(tf.indicators) ? 'ON' : 'OFF')
                : (rawVal(meta, tf) != null ? 'ON' : 'OFF');
        }
        const v = rawVal(meta, tf);
        switch (meta.value_format) {
            case 'percent1': return v == null ? '--' : `${v.toFixed(1)}%`;
            case 'price': return pxf(tf, v);
            case 'ratio2': return (v ?? 1).toFixed(2);
            case 'decimals1': return fmt(v, 1);
            case 'decimals4': return fmt(v, 4);
            case 'decimals2':
            default: return fmt(v, 2);
        }
    }
    function stateLabel(tf: TimeframeTelemetry, key: string): string {
        return tf.indicators?.[key]?.state_label ?? 'UNKNOWN';
    }
    function normalized(tf: TimeframeTelemetry, key: string): number {
        return tf.indicators?.[key]?.normalized ?? 0;
    }
    function signalsFor(tf: TimeframeTelemetry, key: string): IndicatorSignal[] {
        return tf.indicators?.[key]?.signals ?? [];
    }
    function confidence(tf: TimeframeTelemetry, key: string): number {
        return Math.round((tf.indicators?.[key]?.confidence ?? 0) * 100);
    }
    function signalTitle(s: IndicatorSignal): string {
        const age = (s.age_bars ?? 0) === 0 ? 'now' : `${s.age_bars} bars ago`;
        return `${s.kind} · ${s.direction} · ${s.status} (${age}): ${s.label}`;
    }

    const SIGNAL_ABBR: Record<string, string> = {
        Divergence: 'DIV', Crossover: '✕', Threshold: 'TH', Breakout: 'BO',
        BandTouch: 'BT', ZeroLineCross: '0X', CompressionRelease: 'SQZ',
        LevelTest: 'LV', TrendFlip: 'FLIP', VolumeClimax: 'VOL',
        StackChange: 'STK', PatternForming: 'PAT',
    };
    function signalStyle(s: IndicatorSignal): string {
        if (s.direction === 'Bullish') return 'color:#10b981;border-color:#10b981;';
        if (s.direction === 'Bearish') return 'color:#ef4444;border-color:#ef4444;';
        return 'color:#f59e0b;border-color:#f59e0b;';
    }
    function signalText(s: IndicatorSignal): string {
        return `${SIGNAL_ABBR[s.kind] ?? s.kind}${(s.age_bars ?? 0) === 0 ? '' : `·${s.age_bars}`}`;
    }

    function colorForNormalized(n: number): string {
        const mag = Math.min(Math.abs(n), 1);
        if (mag >= 0.9) return 'color: #a855f7; font-weight: 800;';
        if (n > 0.1) {
            const g = Math.round(120 + 135 * mag);
            return `color: rgb(16, ${g}, 129); font-weight: 700;`;
        }
        if (n < -0.1) {
            const r = Math.round(180 + 59 * mag);
            return `color: rgb(${r}, 68, 68); font-weight: 700;`;
        }
        return 'color: #f59e0b; font-weight: 600;';
    }
    function bucketHeaderClass(bucket: string): string {
        const map: Record<string, string> = {
            Leading: styles.sectionHeaderLeading,
            Hybrid: styles.sectionHeaderHybrid,
            Lagging: styles.sectionHeaderLagging,
        };
        return map[bucket] ?? '';
    }
    function formatTfLabel(secs: number): string {
        if (secs >= 86400) return `${secs / 86400}d`;
        if (secs >= 3600) return `${secs / 3600}h`;
        if (secs >= 60) return `${secs / 60}m`;
        return `${secs}s`;
    }
    function formatTfName(key: string): string {
        if (key === 'microTerm') return 'MICRO';
        if (key === 'fastTerm') return 'FAST';
        if (key === 'slowTerm') return 'SLOW';
        return 'MACRO';
    }
    function getMarketState(tf: TimeframeTelemetry): string {
        const trend = normalized(tf, 'ema_stack');
        const adx = normalized(tf, 'adx');
        const bbwp = tf.indicators?.['bbwp']?.raw_value ?? 50;
        if (bbwp > 90) return trend >= 0 ? 'VOLATILITY_BREAKOUT' : 'VOLATILITY_CRASH';
        if (trend > 0.1) return Math.abs(adx) >= 0.5 ? 'STRONG_BULL_TREND' : 'BULL_TREND';
        if (trend < -0.1) return Math.abs(adx) >= 0.5 ? 'STRONG_BEAR_TREND' : 'BEAR_TREND';
        return 'RANGE';
    }

    // ── Reactive materialization ──────────────────────────────────────────
    // Every deep read of `tf.indicators[...]` happens INSIDE this derivation,
    // so Svelte 5 subscribes to the nested proxy signals. When the WebSocket
    // reassigns `tf.indicators`, this recomputes and the (primitive-only)
    // template re-renders — fixing the previously frozen cells.
    type CellRow = {
        key: string;
        displayName: string;
        directional: boolean;
        raw: string;
        state: string;
        stateStyle: string;
        confidencePct: number;
        signals: Array<{ text: string; style: string; title: string }>;
    };
    type CardBucket = { bucket: string; headerClass: string; rows: CellRow[] };
    type Card = {
        tfKey: string;
        tfName: string;
        tfLabel: string;
        priceText: string;
        volText: string;
        marketState: string;
        marketStateStyle: string;
        sections: CardBucket[];
    };

    const cards = $derived.by<Card[]>(() => {
        if (!pair) return [];
        const out: Card[] = [];
        for (const tfKey of timeframes) {
            const tf = (pair as any)[tfKey] as TimeframeTelemetry;
            if (!tf) continue;
            const sections: CardBucket[] = buckets.map(([bucket, metas]) => ({
                bucket,
                headerClass: bucketHeaderClass(bucket),
                rows: metas.map((meta): CellRow => {
                    const n = normalized(tf, meta.key);
                    return {
                        key: meta.key,
                        displayName: meta.display_name,
                        directional: meta.directional,
                        raw: formatRaw(meta, tf),
                        state: stateLabel(tf, meta.key),
                        stateStyle: colorForNormalized(n),
                        confidencePct: confidence(tf, meta.key),
                        signals: signalsFor(tf, meta.key).map((s) => ({
                            text: signalText(s),
                            style: signalStyle(s),
                            title: signalTitle(s),
                        })),
                    };
                }),
            }));
            out.push({
                tfKey,
                tfName: formatTfName(tfKey),
                tfLabel: formatTfLabel(tf.barDurationSec),
                priceText: tf.priceText,
                volText: tf.volText,
                marketState: getMarketState(tf),
                marketStateStyle: colorForNormalized(normalized(tf, 'ema_stack')),
                sections,
            });
        }
        return out;
    });

    function toggleExpandTable(key: string) {
        expandedTfTable = expandedTfTable === key ? null : key;
    }
    function toggleGroupMode() {
        groupMode = groupMode === 'class' ? 'group' : 'class';
    }

    async function copyJson() {
        if (!pair) return;
        const dump: Record<string, unknown> = {
            pair: app.pairDisplayFor(pair.symbol),
            timestamp: new Date().toISOString(),
            telemetry: {},
        };
        for (const tfKey of timeframes) {
            const tf = (pair as any)[tfKey] as TimeframeTelemetry;
            if (!tf) continue;
            const entry: Record<string, unknown> = {
                price: tf.priceText ?? '--',
                volume: tf.volText ?? '--',
                market_state: getMarketState(tf),
            };
            for (const meta of registry) {
                entry[meta.display_name] = {
                    key: meta.key,
                    group: meta.group,
                    class: meta.class,
                    directional: meta.directional,
                    normalized: normalized(tf, meta.key),
                    confidence: confidence(tf, meta.key),
                    state_label: stateLabel(tf, meta.key),
                    signals: signalsFor(tf, meta.key).map((s) => `${s.kind}:${s.direction}:${s.status}:${s.age_bars ?? 0}`),
                };
            }
            (dump.telemetry as any)[`${formatTfName(tfKey)} (${formatTfLabel(tf.barDurationSec)})`] = entry;
        }
        try {
            await navigator.clipboard.writeText(JSON.stringify(dump, null, 2));
            copied = true;
            setTimeout(() => { copied = false; }, 1500);
        } catch (_) {}
    }
</script>

{#if pair}
<div class={styles.telemetryTable}>
    <div class={styles.ttHeader}>
        <span class={styles.ttTitle}>TELEMETRY MONITOR</span>
        <span class={styles.ttSymbol}>{app.pairDisplayFor(pair.symbol)}</span>
        <button class={styles.ttCopyBtn} onclick={toggleGroupMode} title="Toggle grouping">
            {groupMode === 'class' ? 'BY CLASS' : 'BY GROUP'}
        </button>
        <button class={styles.ttCopyBtn} onclick={copyJson}>
            {copied ? 'COPIED' : 'EXPORT DATA'}
        </button>
    </div>

    <div class={styles.ttGrid}>
        {#each cards as card (card.tfKey)}
            <div class="{styles.tfTableCard} {expandedTfTable === card.tfKey ? styles.expandedTableCard : ''}">
                <div class={styles.tfCardHeader}>
                    <span class={styles.tfCardLabel}>{card.tfName} ({card.tfLabel})</span>
                    <div class={styles.headerActions}>
                        <span class={styles.tfCardMarketState} style={card.marketStateStyle}>{card.marketState}</span>
                        <button class={styles.expandBtn} onclick={() => toggleExpandTable(card.tfKey)} title={expandedTfTable === card.tfKey ? 'Collapse' : 'Expand'}>
                            {expandedTfTable === card.tfKey ? '✕' : '⛶'}
                        </button>
                    </div>
                </div>
                <div class={styles.tfCardTableWrapper}>
                    <table class={styles.tfCardTable}>
                        <thead>
                            <tr><th colspan="3" class="{styles.sectionHeader}">Market</th></tr>
                        </thead>
                        <tbody>
                            <tr><td class={styles.colLabel}>PRICE</td><td class={styles.colValue} colspan="2">{card.priceText}</td></tr>
                            <tr><td class={styles.colLabel}>VOLUME</td><td class={styles.colValue} colspan="2">{card.volText}</td></tr>
                        </tbody>
                        {#each card.sections as section (section.bucket)}
                            <thead>
                                <tr><th colspan="3" class="{styles.sectionHeader} {section.headerClass}">{section.bucket} Indicators</th></tr>
                                <tr><th>Indicator</th><th>Raw</th><th>State</th></tr>
                            </thead>
                            <tbody>
                                {#each section.rows as row (row.key)}
                                    <tr>
                                        <td class={styles.colLabel}>
                                            {row.displayName}
                                            {#if !row.directional}<span class={styles.gateTag} title="Non-directional gate">◐</span>{/if}
                                        </td>
                                        <td class={styles.colValue}>{row.raw}</td>
                                        <td class={styles.colState} style={row.stateStyle}>
                                            {row.state}
                                            {#if row.directional}<span class={styles.confTag} title="Confidence">{row.confidencePct}%</span>{/if}
                                            {#each row.signals as sig}
                                                <span class={styles.signalBadge} style={sig.style} title={sig.title}>{sig.text}</span>
                                            {/each}
                                        </td>
                                    </tr>
                                {/each}
                            </tbody>
                        {/each}
                    </table>
                </div>
            </div>
        {/each}
    </div>
</div>
{/if}
