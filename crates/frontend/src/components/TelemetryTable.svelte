<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import styles from './TelemetryTable.module.css';
    import { iRaw, iSub, fmt, isSqueezeOn } from '../lib/telemetry';
    import type { TimeframeTelemetry } from '../types';

    const app = useAppStore();
    let { pairKey }: { pairKey: string } = $props();
    const pair = $derived(app.instancesMap[pairKey]);
    let copied = $state(false);
    let expandedTfTable = $state<string | null>(null);

    const timeframes = ['microTerm', 'fastTerm', 'slowTerm', 'macroTerm'] as const;

    // Table rows: [display label, indicator map key, raw-value accessor].
    // Semantic state strings now come directly from the backend `state_label`.
    const ROWS: Array<[string, string, (tf: TimeframeTelemetry) => string]> = [
        ['PRICE ACT', 'ema_stack', (tf) => tf.priceText],
        ['VWAP', 'vwap', (tf) => fmt(iSub(tf.indicators, 'vwap', 'vwap') ?? iRaw(tf.indicators, 'vwap'), 2)],
        ['EMA', 'ema_stack', (tf) => fmt(iSub(tf.indicators, 'ema_stack', 'fast'), 2)],
        ['VOLUME', 'rvol', (tf) => tf.volText],
        ['RVOL', 'rvol', (tf) => (iRaw(tf.indicators, 'rvol') ?? 1).toFixed(2)],
        ['MACD', 'macd', (tf) => fmt(iRaw(tf.indicators, 'macd'), 4)],
        ['SQUEEZE', 'squeeze', (tf) => (isSqueezeOn(tf.indicators) ? 'ON' : 'OFF')],
        ['RSI', 'rsi', (tf) => fmt(iRaw(tf.indicators, 'rsi'), 2)],
        ['ADX', 'adx', (tf) => fmt(iSub(tf.indicators, 'adx', 'adx') ?? iRaw(tf.indicators, 'adx'), 2)],
        ['BBWP', 'bbwp', (tf) => `${fmt(iRaw(tf.indicators, 'bbwp'), 1)}%`],
        ['ATR', 'atr', (tf) => fmt(iRaw(tf.indicators, 'atr'), 2)],
    ];

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

    // --- Backend-provided semantic labels ---
    function stateLabel(tf: TimeframeTelemetry, key: string): string {
        return tf.indicators?.[key]?.state_label ?? 'UNKNOWN';
    }
    function normalized(tf: TimeframeTelemetry, key: string): number {
        return tf.indicators?.[key]?.normalized ?? 0;
    }

    // --- Continuous color coordination driven by the normalized float ---
    function colorForNormalized(n: number): string {
        const mag = Math.min(Math.abs(n), 1);
        if (mag >= 0.9) {
            // Climactic extreme — purple glow.
            return 'color: #a855f7; font-weight: 800;';
        }
        if (n > 0.1) {
            const g = Math.round(120 + 135 * mag); // brighter green with conviction
            return `color: rgb(16, ${g}, 129); font-weight: 700;`;
        }
        if (n < -0.1) {
            const r = Math.round(180 + 59 * mag);
            return `color: rgb(${r}, 68, 68); font-weight: 700;`;
        }
        return 'color: #f59e0b; font-weight: 600;';
    }

    // --- High-level market state (continuous, from ema_stack + adx) ---
    function getMarketState(tf: TimeframeTelemetry): string {
        const trend = normalized(tf, 'ema_stack');
        const adx = normalized(tf, 'adx');
        const bbwp = tf.indicators?.['bbwp']?.raw_value ?? 50;
        if (bbwp > 90) return trend >= 0 ? 'VOLATILITY_BREAKOUT' : 'VOLATILITY_CRASH';
        if (trend > 0.1) return Math.abs(adx) >= 0.5 ? 'STRONG_BULL_TREND' : 'BULL_TREND';
        if (trend < -0.1) return Math.abs(adx) >= 0.5 ? 'STRONG_BEAR_TREND' : 'BEAR_TREND';
        return 'RANGE';
    }
    function marketStateStyle(tf: TimeframeTelemetry): string {
        return colorForNormalized(normalized(tf, 'ema_stack'));
    }

    function toggleExpandTable(key: string) {
        expandedTfTable = expandedTfTable === key ? null : key;
    }

    async function copyJson() {
        if (!pair) return;
        const dump: Record<string, unknown> = {
            pair: `${pair.symbol}/USDT`,
            timestamp: new Date().toISOString(),
            telemetry: {},
        };
        for (const tfKey of timeframes) {
            const tf = (pair as any)[tfKey] as TimeframeTelemetry;
            if (!tf) continue;
            const entry: Record<string, unknown> = {
                price: tf.priceText ?? '--',
                market_state: getMarketState(tf),
            };
            for (const [label, key] of ROWS) {
                entry[label] = {
                    normalized: normalized(tf, key),
                    state_label: stateLabel(tf, key),
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
        <span class={styles.ttSymbol}>{pair.symbol}/USDT</span>
        <button class={styles.ttCopyBtn} onclick={copyJson}>
            {copied ? 'COPIED' : 'EXPORT DATA'}
        </button>
    </div>

    <div class={styles.ttGrid}>
        {#each timeframes as tfKey}
            {@const tf = (pair as any)[tfKey] as TimeframeTelemetry}
            {#if tf}
                <div class="{styles.tfTableCard} {expandedTfTable === tfKey ? styles.expandedTableCard : ''}">
                    <div class={styles.tfCardHeader}>
                        <span class={styles.tfCardLabel}>{formatTfName(tfKey)} ({formatTfLabel(tf.barDurationSec)})</span>
                        <div class={styles.headerActions}>
                            <span class={styles.tfCardMarketState} style={marketStateStyle(tf)}>
                                {getMarketState(tf)}
                            </span>
                            <button class={styles.expandBtn} onclick={() => toggleExpandTable(tfKey)} title={expandedTfTable === tfKey ? 'Collapse' : 'Expand'}>
                                {expandedTfTable === tfKey ? '✕' : '⛶'}
                            </button>
                        </div>
                    </div>
                    <table class={styles.tfCardTable}>
                        <thead>
                            <tr>
                                <th>Indicator</th>
                                <th>Raw</th>
                                <th>State</th>
                            </tr>
                        </thead>
                        <tbody>
                            {#each ROWS as [label, key, rawFn]}
                                <tr>
                                    <td class={styles.colLabel}>{label}</td>
                                    <td class={styles.colValue}>{rawFn(tf)}</td>
                                    <td class={styles.colState} style={colorForNormalized(normalized(tf, key))}>
                                        {stateLabel(tf, key)}
                                    </td>
                                </tr>
                            {/each}
                        </tbody>
                    </table>
                </div>
            {/if}
        {/each}
    </div>
</div>
{/if}
