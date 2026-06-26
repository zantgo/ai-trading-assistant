<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import styles from './TelemetryTable.module.css';
    import type { TimeframeTelemetry } from '../types';

    const app = useAppStore();
    let { pairKey }: { pairKey: string } = $props();
    const pair = $derived(app.instancesMap[pairKey]);
    let copied = $state(false);
    let expandedTfTable = $state<string | null>(null);

    const timeframes = ['microTerm', 'smallTerm', 'mediumTerm', 'largeTerm'] as const;
    const tfLabels: Record<string, string> = { 
        microTerm: 'MICRO 1m', 
        smallTerm: 'SMALL 5m', 
        mediumTerm: 'MEDIUM 15m', 
        largeTerm: 'LARGE 1h' 
    };

    // --- State Mapping Engine ---

    function getAtrState(tf: TimeframeTelemetry): string {
        const atr = parseFloat(tf.atrText) || 0;
        const price = parseFloat(tf.priceText) || 1;
        if (price > 0 && (atr / price) > 0.05) return 'EXTREME';
        if (tf.atrVolatilityRegime === 'expanding') return 'EXPANDING';
        if (tf.atrVolatilityRegime === 'contracting') return 'CONTRACTING';
        return 'NORMAL';
    }

    function getRsiState(tf: TimeframeTelemetry): string {
        const rsi = parseFloat(tf.rsiText) || 50;
        if (rsi <= 30) return 'OVERSOLD';
        if (rsi > 30 && rsi < 45) return 'WEAK_BEARISH';
        if (rsi >= 45 && rsi <= 55) return 'NEUTRAL';
        if (rsi > 55 && rsi < 70) return 'WEAK_BULLISH';
        return 'OVERBOUGHT';
    }

    function getMacdState(tf: TimeframeTelemetry): string {
        const line = parseFloat(tf.macdLineText) || 0;
        const sig = parseFloat(tf.macdSigText) || 0;
        const hist = parseFloat(tf.macdHistText) || 0;
        if (hist > 0) {
            return (line > 0 && sig > 0) ? 'STRONG_BULLISH' : 'BULLISH';
        } else if (hist < 0) {
            return (line < 0 && sig < 0) ? 'STRONG_BEARISH' : 'BEARISH';
        }
        return 'NEUTRAL';
    }

    function getSqueezeState(tf: TimeframeTelemetry): string {
        if (tf.isSqueezeOn) return 'SQUEEZE_ON';
        const mom = parseFloat(tf.sqzValText) || 0;
        if (tf.squeezeReleaseTrigger) {
            return mom >= 0 ? 'SQUEEZE_RELEASING_BULLISH' : 'SQUEEZE_RELEASING_BEARISH';
        }
        return mom >= 0 ? 'BULLISH_MOMENTUM' : 'BEARISH_MOMENTUM';
    }

    function getAdxState(tf: TimeframeTelemetry): string {
        const adx = parseFloat(tf.adxText) || 0;
        if (adx < 20) return 'RANGE';
        if (adx >= 20 && adx < 25) return 'DEVELOPING_TREND';
        if (adx >= 25 && adx <= 40) return 'STRONG_TREND';
        return 'VERY_STRONG_TREND';
    }

    function getBbwpState(tf: TimeframeTelemetry): string {
        const bbwp = parseFloat(tf.bbwpText) || 50;
        if (bbwp < 10) return 'VOLATILITY_COMPRESSION';
        if (bbwp >= 10 && bbwp < 30) return 'LOW_VOLATILITY';
        if (bbwp >= 30 && bbwp <= 70) return 'NORMAL_VOLATILITY';
        if (bbwp > 70 && bbwp <= 90) return 'HIGH_VOLATILITY';
        return 'VOLATILITY_EXPANSION';
    }

    function getVolumeState(tf: TimeframeTelemetry): string {
        const rvol = tf.rvol || 1.0;
        if (rvol < 0.5) return 'VERY_LOW';
        if (rvol >= 0.5 && rvol < 1.0) return 'LOW';
        if (rvol >= 1.0 && rvol < 1.5) return 'NORMAL';
        if (rvol >= 1.5 && rvol < 3.0) return 'HIGH';
        return 'CLIMACTIC';
    }

    function getPriceActionState(tf: TimeframeTelemetry): string {
        if (tf.emaStackState === 'bullish') return 'STRONG_UPTREND';
        if (tf.emaStackState === 'bearish') return 'STRONG_DOWNTREND';
        const close = parseFloat(tf.priceText) || 0;
        const fast = parseFloat(tf.emaFastText) || 0;
        const med = parseFloat(tf.emaMediumText) || 0;
        if (fast > med) return 'UPTREND';
        if (fast < med) return 'DOWNTREND';
        return 'RANGE';
    }

    function getEmaState(tf: TimeframeTelemetry): string {
        if (tf.emaStackState === 'bullish') {
            const close = parseFloat(tf.priceText) || 0;
            const long = parseFloat(tf.emaLongText) || 1;
            return (close > long * 1.02) ? 'STRONG_BULLISH' : 'BULLISH';
        }
        if (tf.emaStackState === 'bearish') {
            const close = parseFloat(tf.priceText) || 0;
            const long = parseFloat(tf.emaLongText) || 1;
            return (close < long * 0.98) ? 'STRONG_BEARISH' : 'BEARISH';
        }
        return 'NEUTRAL';
    }

    function getVwapState(tf: TimeframeTelemetry): string {
        const close = parseFloat(tf.priceText) || 0;
        const vwap = parseFloat(tf.vwapText) || 0;
        if (vwap === 0) return 'AT_VWAP';
        const pct = (close - vwap) / vwap * 100;
        if (pct > 1.0) return 'FAR_ABOVE_VWAP';
        if (pct > 0.1) return 'ABOVE_VWAP';
        if (pct < -1.0) return 'FAR_BELOW_VWAP';
        if (pct < -0.1) return 'BELOW_VWAP';
        return 'AT_VWAP';
    }

    function getRvolState(tf: TimeframeTelemetry): string {
        const rvol = tf.rvol || 1.0;
        if (rvol < 0.5) return 'VERY_LOW';
        if (rvol >= 0.5 && rvol < 1.0) return 'LOW';
        if (rvol >= 1.0 && rvol < 1.5) return 'NORMAL';
        if (rvol >= 1.5 && rvol < 3.0) return 'HIGH';
        return 'EXTREME';
    }

    // --- High-Level Timeframe Market State Resolver ---

    function getMarketState(tf: TimeframeTelemetry): string {
        const adxState = getAdxState(tf);
        const bbwp = parseFloat(tf.bbwpText) || 50;
        const close = parseFloat(tf.priceText) || 0;
        const vwap = parseFloat(tf.vwapText) || 0;
        
        if (bbwp > 90) {
            if (close > vwap) return 'VOLATILITY_BREAKOUT';
            if (close < vwap) return 'VOLATILITY_CRASH';
        }
        if (tf.emaStackState === 'bullish') {
            return (adxState === 'STRONG_TREND' || adxState === 'VERY_STRONG_TREND') ? 'STRONG_BULL_TREND' : 'BULL_TREND';
        }
        if (tf.emaStackState === 'bearish') {
            return (adxState === 'STRONG_TREND' || adxState === 'VERY_STRONG_TREND') ? 'STRONG_BEAR_TREND' : 'BEAR_TREND';
        }
        return 'RANGE';
    }

    // --- State Color Coordination Style Binder ---

    function getStateStyle(state: string): string {
        const str = state.toUpperCase();
        if (
            str.includes('BULL') || 
            str.includes('EXPANDING') || 
            str.includes('UPTREND') || 
            str.includes('HIGH_VOLATILITY') || 
            str.includes('ABOVE')
        ) {
            return 'color: #10b981; font-weight: 700;';
        }
        if (
            str.includes('BEAR') || 
            str.includes('CONTRACTING') || 
            str.includes('DOWNTREND') || 
            str.includes('LOW_VOLATILITY') || 
            str.includes('BELOW')
        ) {
            return 'color: #ef4444; font-weight: 700;';
        }
        if (
            str.includes('EXTREME') || 
            str.includes('CLIMACTIC') || 
            str.includes('BREAKOUT') || 
            str.includes('CRASH') || 
            str.includes('EXPANSION')
        ) {
            return 'color: #a855f7; font-weight: 800;';
        }
        return 'color: #f59e0b; font-weight: 600;';
    }

    function toggleExpandTable(key: string) {
        expandedTfTable = expandedTfTable === key ? null : key;
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
                market_state: getMarketState(tf),
                atr: getAtrState(tf),
                rsi: getRsiState(tf),
                macd: getMacdState(tf),
                squeeze: getSqueezeState(tf),
                adx: getAdxState(tf),
                bbwp: getBbwpState(tf),
                volume: getVolumeState(tf),
                price_action: getPriceActionState(tf),
                ema: getEmaState(tf),
                vwap: getVwapState(tf),
                rvol: getRvolState(tf)
            };
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
                        <span class={styles.tfCardLabel}>{tfLabels[tfKey]}</span>
                        <div class={styles.headerActions}>
                            <span class={styles.tfCardMarketState} style={getStateStyle(getMarketState(tf))}>
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
                            <tr>
                                <td class={styles.colLabel}>PRICE ACT</td>
                                <td class={styles.colValue}>{tf.priceText}</td>
                                <td class={styles.colState} style={getStateStyle(getPriceActionState(tf))}>{getPriceActionState(tf)}</td>
                            </tr>
                            <tr>
                                <td class={styles.colLabel}>VWAP</td>
                                <td class={styles.colValue}>{tf.vwapText}</td>
                                <td class={styles.colState} style={getStateStyle(getVwapState(tf))}>{getVwapState(tf)}</td>
                            </tr>
                            <tr>
                                <td class={styles.colLabel}>EMA</td>
                                <td class={styles.colValue}>{tf.emaFastText}</td>
                                <td class={styles.colState} style={getStateStyle(getEmaState(tf))}>{getEmaState(tf)}</td>
                            </tr>
                            <tr>
                                <td class={styles.colLabel}>VOLUME</td>
                                <td class={styles.colValue}>{tf.volText}</td>
                                <td class={styles.colState} style={getStateStyle(getVolumeState(tf))}>{getVolumeState(tf)}</td>
                            </tr>
                            <tr>
                                <td class={styles.colLabel}>RVOL</td>
                                <td class={styles.colValue}>{tf.rvol ? tf.rvol.toFixed(2) : '1.0'}</td>
                                <td class={styles.colState} style={getStateStyle(getRvolState(tf))}>{getRvolState(tf)}</td>
                            </tr>
                            <tr>
                                <td class={styles.colLabel}>MACD</td>
                                <td class={styles.colValue}>{tf.macdHistText}</td>
                                <td class={styles.colState} style={getStateStyle(getMacdState(tf))}>{getMacdState(tf)}</td>
                            </tr>
                            <tr>
                                <td class={styles.colLabel}>SQUEEZE</td>
                                <td class={styles.colValue}>{tf.isSqueezeOn ? 'ON' : 'OFF'}</td>
                                <td class={styles.colState} style={getStateStyle(getSqueezeState(tf))}>{getSqueezeState(tf)}</td>
                            </tr>
                            <tr>
                                <td class={styles.colLabel}>RSI</td>
                                <td class={styles.colValue}>{tf.rsiText}</td>
                                <td class={styles.colState} style={getStateStyle(getRsiState(tf))}>{getRsiState(tf)}</td>
                            </tr>
                            <tr>
                                <td class={styles.colLabel}>ADX</td>
                                <td class={styles.colValue}>{tf.adxText}</td>
                                <td class={styles.colState} style={getStateStyle(getAdxState(tf))}>{getAdxState(tf)}</td>
                            </tr>
                            <tr>
                                <td class={styles.colLabel}>BBWP</td>
                                <td class={styles.colValue}>{tf.bbwpText}%</td>
                                <td class={styles.colState} style={getStateStyle(getBbwpState(tf))}>{getBbwpState(tf)}</td>
                            </tr>
                            <tr>
                                <td class={styles.colLabel}>ATR</td>
                                <td class={styles.colValue}>{tf.atrText}</td>
                                <td class={styles.colState} style={getStateStyle(getAtrState(tf))}>{getAtrState(tf)}</td>
                            </tr>
                        </tbody>
                    </table>
                </div>
            {/if}
        {/each}
    </div>
</div>
{/if}
