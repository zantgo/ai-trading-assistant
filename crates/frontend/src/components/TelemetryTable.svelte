<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import styles from './TelemetryTable.module.css';
    import type { TimeframeTelemetry } from '../types';

    const app = useAppStore();
    let { pairKey }: { pairKey: string } = $props();
    const pair = $derived(app.instancesMap[pairKey]);
    let copied = $state(false);

    function priceColor(price: string): string {
        return price.startsWith('-') ? 'color: #ef4444' : 'color: #22c55e';
    }

    function emaColor(state: string): string {
        if (state === 'bullish') return 'color: #22c55e';
        if (state === 'bearish') return 'color: #ef4444';
        return 'color: #f59e0b';
    }

    function vwapColor(bias: string): string {
        if (bias === 'premium') return 'color: #f59e0b';
        if (bias === 'discount') return 'color: #ef4444';
        return 'color: #64748b';
    }

    function rsiClass(val: number): string {
        if (val >= 70) return 'rsiOverbought';
        if (val <= 30) return 'rsiOversold';
        return '';
    }

    function adxClass(regime: string): string {
        if (regime === 'extreme') return 'adxExtreme';
        if (regime === 'strong') return 'adxStrong';
        if (regime === 'emerging') return 'adxEmerging';
        return '';
    }

    function squeezeState(tf: TimeframeTelemetry): string {
        if (!tf?.isSqueezeOn) return 'OFF';
        return `ON ${tf.squeezeDuration}`;
    }

    function squeezeColor(tf: TimeframeTelemetry): string {
        if (!tf?.isSqueezeOn) return 'color: #4a5568';
        return tf.squeezeMomentumDirection.startsWith('Bullish') ? 'color: #22c55e' : tf.squeezeMomentumDirection.startsWith('Bearish') ? 'color: #ef4444' : 'color: #a1a1aa';
    }

    function bbwpColor(val: number): string {
        if (val >= 80) return 'color: #ef4444';
        if (val <= 20) return 'color: #22c55e';
        return 'color: #cbd5e1';
    }

    function rvolColor(val: number): string {
        if (val >= 1.5) return 'color: #22d3ee';
        if (val <= 0.5) return 'color: #4a5568';
        return 'color: #cbd5e1';
    }

    async function copyJson() {
        if (!pair) return;
        const dump: Record<string, unknown> = {
            pair: `${pair.symbol}/USDT`,
            timestamp: new Date().toISOString(),
            telemetry: {}
        };

        for (const tfKey of timeframes) {
            const tf = (pair as any)[tfKey] as TimeframeTelemetry;
            if (!tf) continue;
            (dump.telemetry as any)[tfLabels[tfKey]] = {
                price: tf.priceText ?? '--',
                ema_state: tf.emaStackState ?? '--',
                vwap_bias: tf.vwapBias ?? '--',
                rsi: parseFloat(tf.rsiText ?? '0') || 0,
                macd_line: tf.macdLineText ?? '--',
                adx: parseFloat(tf.adxText ?? '0') || 0,
                adx_regime: tf.adxTrendingRegime ?? '--',
                squeeze: tf.isSqueezeOn ? `ON ${tf.squeezeDuration}` : 'OFF',
                squeeze_momentum: tf.squeezeMomentumDirection ?? '--',
                bbwp: parseFloat(tf.bbwpText ?? '0') || 0,
                rvol: tf.rvol ?? 0,
                volume: tf.avgVolText ?? '--',
                atr: tf.atrText ?? '--',
            };
        }

        try {
            await navigator.clipboard.writeText(JSON.stringify(dump, null, 2));
            copied = true;
            setTimeout(() => { copied = false; }, 1500);
        } catch {
            // clipboard may not be available
        }
    }

    const rows = [
        { label: 'PRICE',  key: (tf: TimeframeTelemetry) => tf?.priceText ?? '--',           style: (tf: TimeframeTelemetry) => priceColor(tf?.priceText ?? '') },
        { label: 'EMA',    key: (tf: TimeframeTelemetry) => tf?.emaStackState ?? '--',       style: (tf: TimeframeTelemetry) => emaColor(tf?.emaStackState ?? '') },
        { label: 'VWAP',   key: (tf: TimeframeTelemetry) => tf?.vwapBias ?? '--',            style: (tf: TimeframeTelemetry) => vwapColor(tf?.vwapBias ?? '') },
        { label: 'RSI',    key: (tf: TimeframeTelemetry) => parseFloat(tf?.rsiText ?? '0') || 0, style: (tf: TimeframeTelemetry) => '', class: (tf: TimeframeTelemetry) => rsiClass(parseFloat(tf?.rsiText ?? '0') || 0) },
        { label: 'MACD',   key: (tf: TimeframeTelemetry) => tf?.macdLineText ?? '--',        style: (tf: TimeframeTelemetry) => (parseFloat(tf?.macdLineText ?? '0') >= 0 ? 'color: #22c55e' : 'color: #ef4444') },
        { label: 'ADX',    key: (tf: TimeframeTelemetry) => parseFloat(tf?.adxText ?? '0') || 0, style: (tf: TimeframeTelemetry) => '', class: (tf: TimeframeTelemetry) => adxClass(tf?.adxTrendingRegime ?? '') },
        { label: 'SQZ',    key: (tf: TimeframeTelemetry) => squeezeState(tf),                style: (tf: TimeframeTelemetry) => squeezeColor(tf) },
        { label: 'BBWP',   key: (tf: TimeframeTelemetry) => parseFloat(tf?.bbwpText ?? '0') || 0, style: (tf: TimeframeTelemetry) => bbwpColor(parseFloat(tf?.bbwpText ?? '0') || 0) },
        { label: 'RVOL',   key: (tf: TimeframeTelemetry) => tf?.rvol ?? 0,                   style: (tf: TimeframeTelemetry) => rvolColor(tf?.rvol ?? 0) },
    ];

    const timeframes = ['microTerm', 'smallTerm', 'mediumTerm', 'largeTerm'] as const;
    const tfLabels: Record<string, string> = { microTerm: 'MICRO 1m', smallTerm: 'SMALL 5m', mediumTerm: 'MEDIUM 15m', largeTerm: 'LARGE 1h' };

    function fmt(_v: any, tf: TimeframeTelemetry, row: typeof rows[0]): string {
        const val = row.key(tf);
        if (typeof val === 'number') {
            if (row.label === 'BBWP' || row.label === 'RVOL') return val.toFixed(1);
            if (row.label === 'RSI' || row.label === 'ADX') return val.toFixed(0);
            return val.toFixed(0);
        }
        return String(val ?? '--').toUpperCase().substring(0, 10);
    }
</script>

{#if pair}
{@const pairObj = pair}
<div class={styles.telemetryTable}>
    <div class={styles.ttHeader}>
        <span class={styles.ttTitle}>TELEMETRY MONITOR</span>
        <span class={styles.ttSymbol}>{pairObj.symbol}/USDT</span>
        <button class={styles.ttCopyBtn} onclick={copyJson}>
            {copied ? 'COPIED' : 'JSON'}
        </button>
    </div>
    <div class={styles.ttScroll}>
        <table>
            <thead>
                <tr>
                    <th class={styles.ttRowLabel}></th>
                    {#each timeframes as tfKey}
                        <th class={styles.ttTfHeader}>{tfLabels[tfKey]}</th>
                    {/each}
                </tr>
            </thead>
            <tbody>
                {#each rows as row}
                    <tr>
                        <td class={styles.ttRowLabel}>{row.label}</td>
                        {#each timeframes as tfKey}
                            {@const tf = (pairObj as any)[tfKey] as TimeframeTelemetry}
                            {#if tf}
                            <td class="{styles.ttCell} {row?.class?.(tf) ? styles[row.class(tf) as keyof typeof styles] || '' : ''}" style={row?.style?.(tf) ?? ''}>
                                {fmt(null, tf, row)}
                            </td>
                            {:else}
                            <td class={styles.ttCell}>--</td>
                            {/if}
                        {/each}
                    </tr>
                {/each}
            </tbody>
        </table>
    </div>
</div>
{/if}
