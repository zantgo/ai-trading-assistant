<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import styles from './TelemetryTable.module.css';
    import { iRaw, iSub, fmt, fmtPrice, isSqueezeOn } from '../lib/telemetry';
    import type { TimeframeTelemetry, IndicatorMeta, IndicatorSignal } from '../types';

    const app = useAppStore();
    let { pairKey, tfKey, tfSecs }: { pairKey: string; tfKey: string; tfSecs: number } = $props();
    const pair = $derived(app.instancesMap[pairKey]);
    const registry = $derived<IndicatorMeta[]>((app.indicatorRegistry ?? []) as IndicatorMeta[]);
    const tf = $derived<TimeframeTelemetry | undefined>((pair as any)?.[tfKey] as TimeframeTelemetry | undefined);

    const CLASS_ORDER = ['Leading', 'Hybrid', 'Lagging'] as const;

    const buckets = $derived.by<Array<[string, IndicatorMeta[]]>>(() => {
        const map = new Map<string, IndicatorMeta[]>();
        for (const b of CLASS_ORDER) map.set(b, []);
        for (const m of registry) {
            const b = m.class;
            if (!map.has(b)) map.set(b, []);
            map.get(b)!.push(m);
        }
        return CLASS_ORDER.filter((b) => (map.get(b)?.length ?? 0) > 0).map((b) => [b, map.get(b)!]);
    });

    function pxf(v: number | null): string {
        return fmtPrice(v, parseFloat(tf?.priceText ?? '0') || 0);
    }
    function rawVal(meta: IndicatorMeta): number | null {
        if (meta.value_source.startsWith('sub:')) {
            return iSub(tf?.indicators ?? {}, meta.key, meta.value_source.slice(4));
        }
        return iRaw(tf?.indicators ?? {}, meta.key);
    }
    function formatRaw(meta: IndicatorMeta): string {
        if (meta.value_format === 'onoff') {
            return meta.key === 'squeeze'
                ? (isSqueezeOn(tf?.indicators ?? {}) ? 'ON' : 'OFF')
                : (rawVal(meta) != null ? 'ON' : 'OFF');
        }
        const v = rawVal(meta);
        switch (meta.value_format) {
            case 'percent1': return v == null ? '--' : `${v.toFixed(1)}%`;
            case 'price': return pxf(v);
            case 'ratio2': return (v ?? 1).toFixed(2);
            case 'decimals1': return fmt(v, 1);
            case 'decimals4': return fmt(v, 4);
            case 'decimals2':
            default: return fmt(v, 2);
        }
    }
    function normalized(key: string): number {
        return tf?.indicators?.[key]?.normalized ?? 0;
    }
    function stateLabel(key: string): string {
        return tf?.indicators?.[key]?.state_label ?? '--';
    }
    function confidence(key: string): number {
        return Math.round((tf?.indicators?.[key]?.confidence ?? 0) * 100);
    }
    function signalsFor(key: string): IndicatorSignal[] {
        return tf?.indicators?.[key]?.signals ?? [];
    }

    const SIGNAL_ABBR: Record<string, string> = {
        Divergence: 'DIV', Crossover: 'CRO', Threshold: 'TH', Breakout: 'BO',
        BandTouch: 'BT', ZeroLineCross: '0X', CompressionRelease: 'SQZ',
        LevelTest: 'LV', TrendFlip: 'FLIP', VolumeClimax: 'VOL',
        StackChange: 'STK', PatternForming: 'PAT',
    };
    function signalBadge(s: IndicatorSignal): { text: string; style: string; title: string } {
        const abbr = SIGNAL_ABBR[s.kind] ?? s.kind;
        const age = (s.age_bars ?? 0) === 0 ? '' : `·${s.age_bars}`;
        const dirStyle = s.direction === 'Bullish' ? 'bull' : s.direction === 'Bearish' ? 'bear' : 'neutral';
        return {
            text: `${abbr}${age}`,
            style: dirStyle,
            title: `${s.kind} · ${s.direction} · ${s.status} (${(s.age_bars ?? 0) === 0 ? 'now' : `${s.age_bars} bars ago`}): ${s.label}`,
        };
    }

    function normColor(n: number): string {
        const mag = Math.min(Math.abs(n), 1);
        if (mag >= 0.9) return 'extreme';
        if (n > 0.1) return 'bull';
        if (n < -0.1) return 'bear';
        return 'neutral';
    }

    function formatTfLabel(secs: number): string {
        if (secs >= 86400) return `${secs / 86400}d`;
        if (secs >= 3600) return `${secs / 3600}h`;
        if (secs >= 60) return `${secs / 60}m`;
        return `${secs}s`;
    }

    function bucketHeaderClass(bucket: string): string {
        const map: Record<string, string> = {
            Leading: styles.leading,
            Hybrid: styles.hybrid,
            Lagging: styles.lagging,
        };
        return map[bucket] ?? '';
    }

    let copied = $state(false);

    async function copyJson() {
        if (!tf) return;
        const dump: Record<string, unknown> = {
            pair: app.pairDisplayFor(pair.symbol),
            tf: `${tfKey} (${formatTfLabel(tfSecs)})`,
            timestamp: new Date().toISOString(),
            indicators: {} as Record<string, unknown>,
        };
        for (const meta of registry) {
            (dump.indicators as any)[meta.display_name] = {
                key: meta.key,
                group: meta.group,
                class: meta.class,
                raw: formatRaw(meta),
                normalized: normalized(meta.key),
                state: stateLabel(meta.key),
                confidence: confidence(meta.key),
                signals: signalsFor(meta.key).map((s) => `${s.kind}:${s.direction}:${s.status}:${s.age_bars ?? 0}`),
            };
        }
        try {
            await navigator.clipboard.writeText(JSON.stringify(dump, null, 2));
            copied = true;
            setTimeout(() => { copied = false; }, 1500);
        } catch (_) {}
    }
</script>

{#if pair && tf}
<div class={styles.table}>
    <div class={styles.header}>
        <span class={styles.title}>ALL INDICATORS · {formatTfLabel(tfSecs)}</span>
        <button class={styles.exportBtn} onclick={copyJson}>
            {copied ? 'COPIED' : 'EXPORT'}
        </button>
    </div>

    <div class={styles.body}>
        {#each buckets as [bucket, metas] (bucket)}
            <div class={styles.bucketSection}>
                <div class="{styles.bucketHeader} {bucketHeaderClass(bucket)}">{bucket}</div>
                <table class={styles.indicatorTable}>
                    <thead>
                        <tr class={styles.colHeaders}>
                            <th class={styles.colName}>Indicator</th>
                            <th class={styles.colRaw}>Raw</th>
                            <th class={styles.colNorm}>Norm</th>
                            <th class={styles.colState}>State</th>
                            <th class={styles.colConf}>Conf</th>
                            <th class={styles.colSig}>Signals</th>
                        </tr>
                    </thead>
                    <tbody>
                        {#each metas as meta (meta.key)}
                            {@const n = normalized(meta.key)}
                            {@const sigs = signalsFor(meta.key)}
                            <tr class={styles.row}>
                                <td class={styles.nameCell}>
                                    {meta.display_name}
                                    {#if !meta.directional}<span class={styles.gateTag}>◐</span>{/if}
                                </td>
                                <td class={styles.rawCell}>{formatRaw(meta)}</td>
                                <td class="{styles.normCell} {styles[normColor(n)]}">{n.toFixed(2)}</td>
                                <td class={styles.stateCell}>{stateLabel(meta.key)}</td>
                                <td class={styles.confCell}>{confidence(meta.key)}%</td>
                                <td class={styles.signalsCell}>
                                    {#each sigs as sig}
                                        {@const b = signalBadge(sig)}
                                        <span class="{styles.signalBadge} {styles[b.style]}" title={b.title}>{b.text}</span>
                                    {/each}
                                </td>
                            </tr>
                        {/each}
                    </tbody>
                </table>
            </div>
        {/each}
    </div>
</div>
{/if}
