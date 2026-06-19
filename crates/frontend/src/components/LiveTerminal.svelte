<script lang="ts">
    import { getState } from '../state.svelte';
    import type { TimeframeTelemetry, InstanceState } from '../state.svelte';
    import PriceChart from './PriceChart.svelte';
    import VolumeChart from './VolumeChart.svelte';
    import AdxChart from './AdxChart.svelte';
    import AtrChart from './AtrChart.svelte';
    import RsiChart from './RsiChart.svelte';
    import MacdChart from './MacdChart.svelte';
    import SqueezeChart from './SqueezeChart.svelte';
    import BbwpChart from './BbwpChart.svelte';

    const app = getState();
    let { pairKey }: { pairKey: string } = $props();

    function label(tf: TimeframeTelemetry): string {
        const sec = tf.barDurationSec;
        if (sec >= 3600) return 'LARGE (1h)';
        if (sec >= 900) return 'MEDIUM (15m)';
        if (sec >= 300) return 'SMALL (5m)';
        return 'MICRO (1m)';
    }

    function tfKey(pairKey: string, tf: TimeframeTelemetry): string {
        return `${pairKey}-${tf.barDurationSec}-${tf.emaFastVal}-${tf.emaMediumVal}-${tf.emaSlowVal}-${tf.emaLongVal}`;
    }
</script>

<div class="terminal-workspace">
    {#if app.instancesMap[pairKey]}
        {@const pair = app.instancesMap[pairKey]}

        <!-- Micro-Term Column -->
        <div class="timescale-column">
            <div class="timescale-header">
                <span class="timescale-title">{label(pair.microTerm)}</span>
                <span class="timescale-price">{pair.microTerm.priceText}</span>
            </div>
            <div class="timescale-charts">
                <div class="panel-box pane-price" class:hidden-pane={!pair.microTerm.showEmas && !pair.microTerm.showBb && !pair.microTerm.showVwap}>
                    <div class="panel-label">PRICE</div>
                    {#key tfKey(pairKey, pair.microTerm)}
                        <PriceChart pairKey={pairKey} timeframe={60} />
                    {/key}
                </div>
                <div class="panel-box pane-vol" class:hidden-pane={!pair.microTerm.showVolume}>
                    <div class="panel-label">VOLUME</div>
                    {#key `${pairKey}-${pair.microTerm.barDurationSec}`}
                        <VolumeChart pairKey={pairKey} timeframe={60} />
                    {/key}
                </div>
                <div class="panel-box pane-adx" class:hidden-pane={!pair.microTerm.showAdx}>
                    <div class="panel-label">ADX</div>
                    {#key `${pairKey}-${pair.microTerm.barDurationSec}-${pair.microTerm.adxPeriodVal}`}
                        <AdxChart pairKey={pairKey} timeframe={60} />
                    {/key}
                </div>
                <div class="panel-box pane-atr" class:hidden-pane={!pair.microTerm.showAtr}>
                    <div class="panel-label">ATR</div>
                    {#key `${pairKey}-${pair.microTerm.barDurationSec}-${pair.microTerm.atrPeriodVal}`}
                        <AtrChart pairKey={pairKey} timeframe={60} />
                    {/key}
                </div>
                <div class="panel-box pane-rsi" class:hidden-pane={!pair.microTerm.showRsi}>
                    <div class="panel-label">RSI</div>
                    {#key `${pairKey}-${pair.microTerm.barDurationSec}-${pair.microTerm.rsiPeriodVal}`}
                        <RsiChart pairKey={pairKey} timeframe={60} />
                    {/key}
                </div>
                <div class="panel-box pane-macd" class:hidden-pane={!pair.microTerm.showMacd}>
                    <div class="panel-label">MACD</div>
                    {#key `${pairKey}-${pair.microTerm.barDurationSec}-${pair.microTerm.macdFastVal}-${pair.microTerm.macdSlowVal}-${pair.microTerm.macdSignalVal}`}
                        <MacdChart pairKey={pairKey} timeframe={60} />
                    {/key}
                </div>
                <div class="panel-box pane-squeeze" class:hidden-pane={!pair.microTerm.showSqueeze}>
                    <div class="panel-label">SQUEEZE</div>
                    {#key `${pairKey}-${pair.microTerm.barDurationSec}-${pair.microTerm.squeezePeriodVal}`}
                        <SqueezeChart pairKey={pairKey} timeframe={60} />
                    {/key}
                </div>
                <div class="panel-box pane-bbwp" class:hidden-pane={!pair.microTerm.showBbwp}>
                    <div class="panel-label">BBWP</div>
                    {#key `${pairKey}-${pair.microTerm.barDurationSec}-bbwp`}
                        <BbwpChart historyPrices={pair.microTerm.historyPrices} currentBbwp={pair.microTerm.lastBbwp} />
                    {/key}
                </div>
            </div>
        </div>

        <!-- Small-Term Column -->
        <div class="timescale-column">
            <div class="timescale-header">
                <span class="timescale-title">{label(pair.smallTerm)}</span>
                <span class="timescale-price">{pair.smallTerm.priceText}</span>
            </div>
            <div class="timescale-charts">
                <div class="panel-box pane-price" class:hidden-pane={!pair.smallTerm.showEmas && !pair.smallTerm.showBb && !pair.smallTerm.showVwap}>
                    <div class="panel-label">PRICE</div>
                    {#key tfKey(pairKey, pair.smallTerm)}
                        <PriceChart pairKey={pairKey} timeframe={300} />
                    {/key}
                </div>
                <div class="panel-box pane-vol" class:hidden-pane={!pair.smallTerm.showVolume}>
                    <div class="panel-label">VOLUME</div>
                    {#key `${pairKey}-${pair.smallTerm.barDurationSec}`}
                        <VolumeChart pairKey={pairKey} timeframe={300} />
                    {/key}
                </div>
                <div class="panel-box pane-adx" class:hidden-pane={!pair.smallTerm.showAdx}>
                    <div class="panel-label">ADX</div>
                    {#key `${pairKey}-${pair.smallTerm.barDurationSec}-${pair.smallTerm.adxPeriodVal}`}
                        <AdxChart pairKey={pairKey} timeframe={300} />
                    {/key}
                </div>
                <div class="panel-box pane-atr" class:hidden-pane={!pair.smallTerm.showAtr}>
                    <div class="panel-label">ATR</div>
                    {#key `${pairKey}-${pair.smallTerm.barDurationSec}-${pair.smallTerm.atrPeriodVal}`}
                        <AtrChart pairKey={pairKey} timeframe={300} />
                    {/key}
                </div>
                <div class="panel-box pane-rsi" class:hidden-pane={!pair.smallTerm.showRsi}>
                    <div class="panel-label">RSI</div>
                    {#key `${pairKey}-${pair.smallTerm.barDurationSec}-${pair.smallTerm.rsiPeriodVal}`}
                        <RsiChart pairKey={pairKey} timeframe={300} />
                    {/key}
                </div>
                <div class="panel-box pane-macd" class:hidden-pane={!pair.smallTerm.showMacd}>
                    <div class="panel-label">MACD</div>
                    {#key `${pairKey}-${pair.smallTerm.barDurationSec}-${pair.smallTerm.macdFastVal}-${pair.smallTerm.macdSlowVal}-${pair.smallTerm.macdSignalVal}`}
                        <MacdChart pairKey={pairKey} timeframe={300} />
                    {/key}
                </div>
                <div class="panel-box pane-squeeze" class:hidden-pane={!pair.smallTerm.showSqueeze}>
                    <div class="panel-label">SQUEEZE</div>
                    {#key `${pairKey}-${pair.smallTerm.barDurationSec}-${pair.smallTerm.squeezePeriodVal}`}
                        <SqueezeChart pairKey={pairKey} timeframe={300} />
                    {/key}
                </div>
                <div class="panel-box pane-bbwp" class:hidden-pane={!pair.smallTerm.showBbwp}>
                    <div class="panel-label">BBWP</div>
                    {#key `${pairKey}-${pair.smallTerm.barDurationSec}-bbwp`}
                        <BbwpChart historyPrices={pair.smallTerm.historyPrices} currentBbwp={pair.smallTerm.lastBbwp} />
                    {/key}
                </div>
            </div>
        </div>

        <!-- Medium-Term Column -->
        <div class="timescale-column">
            <div class="timescale-header">
                <span class="timescale-title">{label(pair.mediumTerm)}</span>
                <span class="timescale-price">{pair.mediumTerm.priceText}</span>
            </div>
            <div class="timescale-charts">
                <div class="panel-box pane-price" class:hidden-pane={!pair.mediumTerm.showEmas && !pair.mediumTerm.showBb && !pair.mediumTerm.showVwap}>
                    <div class="panel-label">PRICE</div>
                    {#key tfKey(pairKey, pair.mediumTerm)}
                        <PriceChart pairKey={pairKey} timeframe={900} />
                    {/key}
                </div>
                <div class="panel-box pane-vol" class:hidden-pane={!pair.mediumTerm.showVolume}>
                    <div class="panel-label">VOLUME</div>
                    {#key `${pairKey}-${pair.mediumTerm.barDurationSec}`}
                        <VolumeChart pairKey={pairKey} timeframe={900} />
                    {/key}
                </div>
                <div class="panel-box pane-adx" class:hidden-pane={!pair.mediumTerm.showAdx}>
                    <div class="panel-label">ADX</div>
                    {#key `${pairKey}-${pair.mediumTerm.barDurationSec}-${pair.mediumTerm.adxPeriodVal}`}
                        <AdxChart pairKey={pairKey} timeframe={900} />
                    {/key}
                </div>
                <div class="panel-box pane-atr" class:hidden-pane={!pair.mediumTerm.showAtr}>
                    <div class="panel-label">ATR</div>
                    {#key `${pairKey}-${pair.mediumTerm.barDurationSec}-${pair.mediumTerm.atrPeriodVal}`}
                        <AtrChart pairKey={pairKey} timeframe={900} />
                    {/key}
                </div>
                <div class="panel-box pane-rsi" class:hidden-pane={!pair.mediumTerm.showRsi}>
                    <div class="panel-label">RSI</div>
                    {#key `${pairKey}-${pair.mediumTerm.barDurationSec}-${pair.mediumTerm.rsiPeriodVal}`}
                        <RsiChart pairKey={pairKey} timeframe={900} />
                    {/key}
                </div>
                <div class="panel-box pane-macd" class:hidden-pane={!pair.mediumTerm.showMacd}>
                    <div class="panel-label">MACD</div>
                    {#key `${pairKey}-${pair.mediumTerm.barDurationSec}-${pair.mediumTerm.macdFastVal}-${pair.mediumTerm.macdSlowVal}-${pair.mediumTerm.macdSignalVal}`}
                        <MacdChart pairKey={pairKey} timeframe={900} />
                    {/key}
                </div>
                <div class="panel-box pane-squeeze" class:hidden-pane={!pair.mediumTerm.showSqueeze}>
                    <div class="panel-label">SQUEEZE</div>
                    {#key `${pairKey}-${pair.mediumTerm.barDurationSec}-${pair.mediumTerm.squeezePeriodVal}`}
                        <SqueezeChart pairKey={pairKey} timeframe={900} />
                    {/key}
                </div>
                <div class="panel-box pane-bbwp" class:hidden-pane={!pair.mediumTerm.showBbwp}>
                    <div class="panel-label">BBWP</div>
                    {#key `${pairKey}-${pair.mediumTerm.barDurationSec}-bbwp`}
                        <BbwpChart historyPrices={pair.mediumTerm.historyPrices} currentBbwp={pair.mediumTerm.lastBbwp} />
                    {/key}
                </div>
            </div>
        </div>

        <!-- Large-Term Column -->
        <div class="timescale-column">
            <div class="timescale-header">
                <span class="timescale-title">{label(pair.largeTerm)}</span>
                <span class="timescale-price">{pair.largeTerm.priceText}</span>
            </div>
            <div class="timescale-charts">
                <div class="panel-box pane-price" class:hidden-pane={!pair.largeTerm.showEmas && !pair.largeTerm.showBb && !pair.largeTerm.showVwap}>
                    <div class="panel-label">PRICE</div>
                    {#key tfKey(pairKey, pair.largeTerm)}
                        <PriceChart pairKey={pairKey} timeframe={3600} />
                    {/key}
                </div>
                <div class="panel-box pane-vol" class:hidden-pane={!pair.largeTerm.showVolume}>
                    <div class="panel-label">VOLUME</div>
                    {#key `${pairKey}-${pair.largeTerm.barDurationSec}`}
                        <VolumeChart pairKey={pairKey} timeframe={3600} />
                    {/key}
                </div>
                <div class="panel-box pane-adx" class:hidden-pane={!pair.largeTerm.showAdx}>
                    <div class="panel-label">ADX</div>
                    {#key `${pairKey}-${pair.largeTerm.barDurationSec}-${pair.largeTerm.adxPeriodVal}`}
                        <AdxChart pairKey={pairKey} timeframe={3600} />
                    {/key}
                </div>
                <div class="panel-box pane-atr" class:hidden-pane={!pair.largeTerm.showAtr}>
                    <div class="panel-label">ATR</div>
                    {#key `${pairKey}-${pair.largeTerm.barDurationSec}-${pair.largeTerm.atrPeriodVal}`}
                        <AtrChart pairKey={pairKey} timeframe={3600} />
                    {/key}
                </div>
                <div class="panel-box pane-rsi" class:hidden-pane={!pair.largeTerm.showRsi}>
                    <div class="panel-label">RSI</div>
                    {#key `${pairKey}-${pair.largeTerm.barDurationSec}-${pair.largeTerm.rsiPeriodVal}`}
                        <RsiChart pairKey={pairKey} timeframe={3600} />
                    {/key}
                </div>
                <div class="panel-box pane-macd" class:hidden-pane={!pair.largeTerm.showMacd}>
                    <div class="panel-label">MACD</div>
                    {#key `${pairKey}-${pair.largeTerm.barDurationSec}-${pair.largeTerm.macdFastVal}-${pair.largeTerm.macdSlowVal}-${pair.largeTerm.macdSignalVal}`}
                        <MacdChart pairKey={pairKey} timeframe={3600} />
                    {/key}
                </div>
                <div class="panel-box pane-squeeze" class:hidden-pane={!pair.largeTerm.showSqueeze}>
                    <div class="panel-label">SQUEEZE</div>
                    {#key `${pairKey}-${pair.largeTerm.barDurationSec}-${pair.largeTerm.squeezePeriodVal}`}
                        <SqueezeChart pairKey={pairKey} timeframe={3600} />
                    {/key}
                </div>
                <div class="panel-box pane-bbwp" class:hidden-pane={!pair.largeTerm.showBbwp}>
                    <div class="panel-label">BBWP</div>
                    {#key `${pairKey}-${pair.largeTerm.barDurationSec}-bbwp`}
                        <BbwpChart historyPrices={pair.largeTerm.historyPrices} currentBbwp={pair.largeTerm.lastBbwp} />
                    {/key}
                </div>
            </div>
        </div>
    {/if}
</div>

<style>
    .terminal-workspace {
        display: grid;
        grid-template-columns: repeat(2, 1fr);
        grid-template-rows: repeat(2, 1fr);
        gap: 12px;
        height: 100%;
        width: 100%;
        padding: 12px;
        box-sizing: border-box;
        overflow: hidden;
    }
    .timescale-column {
        background-color: #131722;
        border: 1px solid #2a2e39;
        border-radius: 8px;
        display: flex;
        flex-direction: column;
        overflow: hidden;
        min-height: 0;
    }
    .timescale-header {
        background-color: #0f111a;
        border-bottom: 1px solid #1e293b;
        padding: 8px 12px;
        display: flex;
        justify-content: space-between;
        align-items: center;
        flex-shrink: 0;
    }
    .timescale-title {
        font-size: 11px;
        font-weight: 700;
        letter-spacing: 0.05em;
        color: #cbd5e1;
        text-transform: uppercase;
        font-family: 'Courier New', monospace;
    }
    .timescale-price {
        font-size: 12px;
        font-weight: 700;
        color: #64ffda;
        font-family: 'Courier New', monospace;
    }
    .timescale-charts {
        flex: 1;
        overflow-y: auto;
        display: flex;
        flex-direction: column;
        gap: 4px;
        padding: 4px;
    }
    .panel-label {
        position: absolute;
        top: 3px;
        left: 6px;
        z-index: 5;
        font-size: 8px;
        font-weight: 700;
        letter-spacing: 0.1em;
        color: #4a5568;
        text-transform: uppercase;
        font-family: 'Courier New', monospace;
        pointer-events: none;
    }
    .panel-box {
        position: relative;
        background: #0f111a;
        border-radius: 4px;
        min-height: 80px;
        flex-shrink: 0;
    }
    .panel-box.pane-price { height: 120px; }
    .panel-box.pane-vol { height: 60px; }
    .panel-box.pane-adx { height: 60px; }
    .panel-box.pane-atr { height: 60px; }
    .panel-box.pane-rsi { height: 60px; }
    .panel-box.pane-macd { height: 60px; }
    .panel-box.pane-squeeze { height: 60px; }
    .panel-box.pane-bbwp { height: 60px; }
    .hidden-pane { display: none; }

    @media (max-width: 1600px) {
        .terminal-workspace {
            grid-template-columns: repeat(2, 1fr);
            grid-template-rows: repeat(2, 1fr);
        }
    }
    @media (max-width: 1280px) {
        .terminal-workspace {
            grid-template-columns: 1fr;
            grid-template-rows: repeat(4, 400px);
            overflow-y: auto;
        }
    }
</style>
