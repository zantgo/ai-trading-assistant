<script lang="ts">
    import { getState } from '../state.svelte';
    import type { TimeframeTelemetry } from '../state.svelte';

    const app = getState();
    let { pairKey }: { pairKey: string } = $props();
    const pair = $derived(app.instancesMap[pairKey]);

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
        if (val >= 70) return 'rsi-overbought';
        if (val <= 30) return 'rsi-oversold';
        return '';
    }

    function adxClass(regime: string): string {
        if (regime === 'extreme') return 'adx-extreme';
        if (regime === 'strong') return 'adx-strong';
        if (regime === 'emerging') return 'adx-emerging';
        return '';
    }

    function squeezeState(tf: TimeframeTelemetry): string {
        if (!tf?.isSqueezeOn) return 'OFF';
        return `ON ${tf.squeezeDuration}`;
    }

    function squeezeColor(tf: TimeframeTelemetry): string {
        if (!tf?.isSqueezeOn) return 'color: #4a5568';
        return tf.squeezeMomentumDirection === 'Up' ? 'color: #22c55e' : 'color: #ef4444';
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
<div class="telemetry-table">
    <div class="tt-header">
        <span class="tt-title">TELEMETRY MONITOR</span>
        <span class="tt-symbol">{pairObj.symbol}/USDT</span>
    </div>
    <div class="tt-scroll">
        <table>
            <thead>
                <tr>
                    <th class="tt-row-label"></th>
                    {#each timeframes as tfKey}
                        <th class="tt-tf-header">{tfLabels[tfKey]}</th>
                    {/each}
                </tr>
            </thead>
            <tbody>
                {#each rows as row}
                    <tr>
                        <td class="tt-row-label">{row.label}</td>
                        {#each timeframes as tfKey}
                            {@const tf = (pairObj as any)[tfKey] as TimeframeTelemetry}
                            {#if tf}
                            <td class="tt-cell {row?.class?.(tf) ?? ''}" style={row?.style?.(tf) ?? ''}>
                                {fmt(null, tf, row)}
                            </td>
                            {:else}
                            <td class="tt-cell">--</td>
                            {/if}
                        {/each}
                    </tr>
                {/each}
            </tbody>
        </table>
    </div>
</div>
{/if}

<style>
    .telemetry-table {
        width: 100%;
        height: 170px;
        flex-shrink: 0;
        background: #0f111a;
        border-top: 1px solid #1e293b;
        display: flex;
        flex-direction: column;
        overflow: hidden;
    }
    .tt-header {
        flex-shrink: 0;
        padding: 6px 10px;
        border-bottom: 1px solid #1e293b;
        display: flex;
        align-items: center;
        gap: 12px;
    }
    .tt-title {
        font-size: 10px;
        font-weight: 700;
        color: #64ffda;
        font-family: 'Courier New', monospace;
        letter-spacing: 0.1em;
        text-transform: uppercase;
    }
    .tt-symbol {
        font-size: 9px;
        color: #64748b;
        font-family: 'Courier New', monospace;
    }
    .tt-scroll {
        flex: 1;
        overflow-y: auto;
        overflow-x: hidden;
    }
    table {
        width: 100%;
        border-collapse: collapse;
        font-family: 'Courier New', monospace;
        font-size: 10px;
    }
    thead {
        position: sticky;
        top: 0;
        z-index: 2;
    }
    .tt-tf-header {
        background: #0a0d14;
        color: #64748b;
        padding: 4px 8px;
        text-align: center;
        font-weight: 700;
        font-size: 9px;
        letter-spacing: 0.05em;
        border-bottom: 1px solid #1e293b;
    }
    .tt-row-label {
        color: #4a5568;
        padding: 3px 8px;
        text-align: left;
        font-size: 8px;
        font-weight: 700;
        letter-spacing: 0.08em;
        white-space: nowrap;
        background: #0a0d14;
        border-right: 1px solid #1a1d26;
    }
    .tt-cell {
        padding: 3px 8px;
        text-align: center;
        font-size: 9px;
        font-weight: 600;
        border-bottom: 1px solid #14142a;
        white-space: nowrap;
    }
    tbody tr:hover .tt-cell {
        background: #1a1d26;
    }
    tbody tr:hover .tt-row-label {
        background: #14142a;
        color: #cbd5e1;
    }
    .rsi-overbought { background: rgba(239, 68, 68, 0.15); }
    .rsi-oversold { background: rgba(34, 197, 94, 0.15); }
    .adx-extreme { background: rgba(239, 68, 68, 0.2); }
    .adx-strong { background: rgba(249, 115, 22, 0.2); }
    .adx-emerging { background: rgba(34, 197, 94, 0.1); }
</style>
