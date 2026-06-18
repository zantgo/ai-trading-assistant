<script lang="ts">
    import { getState } from '../state.svelte';
    import type { TimeframeTelemetry, PairState } from '../state.svelte';
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
        if (sec >= 3600) return 'SUPER MACRO (1h)';
        if (sec >= 900) return 'MACRO (15m)';
        if (sec >= 300) return 'LONG (5m)';
        return 'MID (1m)';
    }

    function tfKey(pairKey: string, tf: TimeframeTelemetry): string {
        return `${pairKey}-${tf.barDurationSec}-${tf.emaFastVal}-${tf.emaMediumVal}-${tf.emaSlowVal}-${tf.emaLongVal}`;
    }
</script>

<div class="terminal-workspace">
    {#if app.pairsMap[pairKey]}
        {@const pair = app.pairsMap[pairKey]}

        <!-- Mid-Term Column -->
        <div class="timescale-column">
            <div class="timescale-header">
                <span class="timescale-title">{label(pair.midTerm)}</span>
                <span class="timescale-price">{pair.midTerm.priceText}</span>
            </div>
            <div class="timescale-charts">
                <div class="panel-box pane-price" class:hidden-pane={!pair.midTerm.showEmas && !pair.midTerm.showBb && !pair.midTerm.showVwap}>
                    {#key tfKey(pairKey, pair.midTerm)}
                        <PriceChart pairKey={pairKey} timeframe={60} />
                    {/key}
                </div>
                <div class="panel-box pane-vol" class:hidden-pane={!pair.midTerm.showVolume}>
                    {#key `${pairKey}-${pair.midTerm.barDurationSec}`}
                        <VolumeChart pairKey={pairKey} timeframe={60} />
                    {/key}
                </div>
                <div class="panel-box pane-adx" class:hidden-pane={!pair.midTerm.showAdx}>
                    {#key `${pairKey}-${pair.midTerm.barDurationSec}-${pair.midTerm.adxPeriodVal}`}
                        <AdxChart pairKey={pairKey} timeframe={60} />
                    {/key}
                </div>
                <div class="panel-box pane-atr" class:hidden-pane={!pair.midTerm.showAtr}>
                    {#key `${pairKey}-${pair.midTerm.barDurationSec}-${pair.midTerm.atrPeriodVal}`}
                        <AtrChart pairKey={pairKey} timeframe={60} />
                    {/key}
                </div>
                <div class="panel-box pane-rsi" class:hidden-pane={!pair.midTerm.showRsi}>
                    {#key `${pairKey}-${pair.midTerm.barDurationSec}-${pair.midTerm.rsiPeriodVal}`}
                        <RsiChart pairKey={pairKey} timeframe={60} />
                    {/key}
                </div>
                <div class="panel-box pane-macd" class:hidden-pane={!pair.midTerm.showMacd}>
                    {#key `${pairKey}-${pair.midTerm.barDurationSec}-${pair.midTerm.macdFastVal}-${pair.midTerm.macdSlowVal}-${pair.midTerm.macdSignalVal}`}
                        <MacdChart pairKey={pairKey} timeframe={60} />
                    {/key}
                </div>
                <div class="panel-box pane-squeeze" class:hidden-pane={!pair.midTerm.showSqueeze}>
                    {#key `${pairKey}-${pair.midTerm.barDurationSec}-${pair.midTerm.squeezePeriodVal}`}
                        <SqueezeChart pairKey={pairKey} timeframe={60} />
                    {/key}
                </div>
                <div class="panel-box pane-bbwp" class:hidden-pane={!pair.midTerm.showBbwp}>
                    {#key `${pairKey}-${pair.midTerm.barDurationSec}-bbwp`}
                        <BbwpChart historyPrices={pair.midTerm.historyPrices} currentBbwp={pair.midTerm.lastBbwp} />
                    {/key}
                </div>
            </div>
        </div>

        <!-- Long-Term Column -->
        <div class="timescale-column">
            <div class="timescale-header">
                <span class="timescale-title">{label(pair.longTerm)}</span>
                <span class="timescale-price">{pair.longTerm.priceText}</span>
            </div>
            <div class="timescale-charts">
                <div class="panel-box pane-price" class:hidden-pane={!pair.longTerm.showEmas && !pair.longTerm.showBb && !pair.longTerm.showVwap}>
                    {#key tfKey(pairKey, pair.longTerm)}
                        <PriceChart pairKey={pairKey} timeframe={300} />
                    {/key}
                </div>
                <div class="panel-box pane-vol" class:hidden-pane={!pair.longTerm.showVolume}>
                    {#key `${pairKey}-${pair.longTerm.barDurationSec}`}
                        <VolumeChart pairKey={pairKey} timeframe={300} />
                    {/key}
                </div>
                <div class="panel-box pane-adx" class:hidden-pane={!pair.longTerm.showAdx}>
                    {#key `${pairKey}-${pair.longTerm.barDurationSec}-${pair.longTerm.adxPeriodVal}`}
                        <AdxChart pairKey={pairKey} timeframe={300} />
                    {/key}
                </div>
                <div class="panel-box pane-atr" class:hidden-pane={!pair.longTerm.showAtr}>
                    {#key `${pairKey}-${pair.longTerm.barDurationSec}-${pair.longTerm.atrPeriodVal}`}
                        <AtrChart pairKey={pairKey} timeframe={300} />
                    {/key}
                </div>
                <div class="panel-box pane-rsi" class:hidden-pane={!pair.longTerm.showRsi}>
                    {#key `${pairKey}-${pair.longTerm.barDurationSec}-${pair.longTerm.rsiPeriodVal}`}
                        <RsiChart pairKey={pairKey} timeframe={300} />
                    {/key}
                </div>
                <div class="panel-box pane-macd" class:hidden-pane={!pair.longTerm.showMacd}>
                    {#key `${pairKey}-${pair.longTerm.barDurationSec}-${pair.longTerm.macdFastVal}-${pair.longTerm.macdSlowVal}-${pair.longTerm.macdSignalVal}`}
                        <MacdChart pairKey={pairKey} timeframe={300} />
                    {/key}
                </div>
                <div class="panel-box pane-squeeze" class:hidden-pane={!pair.longTerm.showSqueeze}>
                    {#key `${pairKey}-${pair.longTerm.barDurationSec}-${pair.longTerm.squeezePeriodVal}`}
                        <SqueezeChart pairKey={pairKey} timeframe={300} />
                    {/key}
                </div>
                <div class="panel-box pane-bbwp" class:hidden-pane={!pair.longTerm.showBbwp}>
                    {#key `${pairKey}-${pair.longTerm.barDurationSec}-bbwp`}
                        <BbwpChart historyPrices={pair.longTerm.historyPrices} currentBbwp={pair.longTerm.lastBbwp} />
                    {/key}
                </div>
            </div>
        </div>

        <!-- Macro-Term Column -->
        <div class="timescale-column">
            <div class="timescale-header">
                <span class="timescale-title">{label(pair.macroTerm)}</span>
                <span class="timescale-price">{pair.macroTerm.priceText}</span>
            </div>
            <div class="timescale-charts">
                <div class="panel-box pane-price" class:hidden-pane={!pair.macroTerm.showEmas && !pair.macroTerm.showBb && !pair.macroTerm.showVwap}>
                    {#key tfKey(pairKey, pair.macroTerm)}
                        <PriceChart pairKey={pairKey} timeframe={900} />
                    {/key}
                </div>
                <div class="panel-box pane-vol" class:hidden-pane={!pair.macroTerm.showVolume}>
                    {#key `${pairKey}-${pair.macroTerm.barDurationSec}`}
                        <VolumeChart pairKey={pairKey} timeframe={900} />
                    {/key}
                </div>
                <div class="panel-box pane-adx" class:hidden-pane={!pair.macroTerm.showAdx}>
                    {#key `${pairKey}-${pair.macroTerm.barDurationSec}-${pair.macroTerm.adxPeriodVal}`}
                        <AdxChart pairKey={pairKey} timeframe={900} />
                    {/key}
                </div>
                <div class="panel-box pane-atr" class:hidden-pane={!pair.macroTerm.showAtr}>
                    {#key `${pairKey}-${pair.macroTerm.barDurationSec}-${pair.macroTerm.atrPeriodVal}`}
                        <AtrChart pairKey={pairKey} timeframe={900} />
                    {/key}
                </div>
                <div class="panel-box pane-rsi" class:hidden-pane={!pair.macroTerm.showRsi}>
                    {#key `${pairKey}-${pair.macroTerm.barDurationSec}-${pair.macroTerm.rsiPeriodVal}`}
                        <RsiChart pairKey={pairKey} timeframe={900} />
                    {/key}
                </div>
                <div class="panel-box pane-macd" class:hidden-pane={!pair.macroTerm.showMacd}>
                    {#key `${pairKey}-${pair.macroTerm.barDurationSec}-${pair.macroTerm.macdFastVal}-${pair.macroTerm.macdSlowVal}-${pair.macroTerm.macdSignalVal}`}
                        <MacdChart pairKey={pairKey} timeframe={900} />
                    {/key}
                </div>
                <div class="panel-box pane-squeeze" class:hidden-pane={!pair.macroTerm.showSqueeze}>
                    {#key `${pairKey}-${pair.macroTerm.barDurationSec}-${pair.macroTerm.squeezePeriodVal}`}
                        <SqueezeChart pairKey={pairKey} timeframe={900} />
                    {/key}
                </div>
                <div class="panel-box pane-bbwp" class:hidden-pane={!pair.macroTerm.showBbwp}>
                    {#key `${pairKey}-${pair.macroTerm.barDurationSec}-bbwp`}
                        <BbwpChart historyPrices={pair.macroTerm.historyPrices} currentBbwp={pair.macroTerm.lastBbwp} />
                    {/key}
                </div>
            </div>
        </div>

        <!-- SuperMacro-Term Column -->
        <div class="timescale-column">
            <div class="timescale-header">
                <span class="timescale-title">{label(pair.supermacroTerm)}</span>
                <span class="timescale-price">{pair.supermacroTerm.priceText}</span>
            </div>
            <div class="timescale-charts">
                <div class="panel-box pane-price" class:hidden-pane={!pair.supermacroTerm.showEmas && !pair.supermacroTerm.showBb && !pair.supermacroTerm.showVwap}>
                    {#key tfKey(pairKey, pair.supermacroTerm)}
                        <PriceChart pairKey={pairKey} timeframe={3600} />
                    {/key}
                </div>
                <div class="panel-box pane-vol" class:hidden-pane={!pair.supermacroTerm.showVolume}>
                    {#key `${pairKey}-${pair.supermacroTerm.barDurationSec}`}
                        <VolumeChart pairKey={pairKey} timeframe={3600} />
                    {/key}
                </div>
                <div class="panel-box pane-adx" class:hidden-pane={!pair.supermacroTerm.showAdx}>
                    {#key `${pairKey}-${pair.supermacroTerm.barDurationSec}-${pair.supermacroTerm.adxPeriodVal}`}
                        <AdxChart pairKey={pairKey} timeframe={3600} />
                    {/key}
                </div>
                <div class="panel-box pane-atr" class:hidden-pane={!pair.supermacroTerm.showAtr}>
                    {#key `${pairKey}-${pair.supermacroTerm.barDurationSec}-${pair.supermacroTerm.atrPeriodVal}`}
                        <AtrChart pairKey={pairKey} timeframe={3600} />
                    {/key}
                </div>
                <div class="panel-box pane-rsi" class:hidden-pane={!pair.supermacroTerm.showRsi}>
                    {#key `${pairKey}-${pair.supermacroTerm.barDurationSec}-${pair.supermacroTerm.rsiPeriodVal}`}
                        <RsiChart pairKey={pairKey} timeframe={3600} />
                    {/key}
                </div>
                <div class="panel-box pane-macd" class:hidden-pane={!pair.supermacroTerm.showMacd}>
                    {#key `${pairKey}-${pair.supermacroTerm.barDurationSec}-${pair.supermacroTerm.macdFastVal}-${pair.supermacroTerm.macdSlowVal}-${pair.supermacroTerm.macdSignalVal}`}
                        <MacdChart pairKey={pairKey} timeframe={3600} />
                    {/key}
                </div>
                <div class="panel-box pane-squeeze" class:hidden-pane={!pair.supermacroTerm.showSqueeze}>
                    {#key `${pairKey}-${pair.supermacroTerm.barDurationSec}-${pair.supermacroTerm.squeezePeriodVal}`}
                        <SqueezeChart pairKey={pairKey} timeframe={3600} />
                    {/key}
                </div>
                <div class="panel-box pane-bbwp" class:hidden-pane={!pair.supermacroTerm.showBbwp}>
                    {#key `${pairKey}-${pair.supermacroTerm.barDurationSec}-bbwp`}
                        <BbwpChart historyPrices={pair.supermacroTerm.historyPrices} currentBbwp={pair.supermacroTerm.lastBbwp} />
                    {/key}
                </div>
            </div>
        </div>
    {/if}
</div>

<style>
    .terminal-workspace {
        display: grid;
        grid-template-columns: repeat(4, 1fr);
        gap: 12px;
        height: 100%;
        width: 100%;
        padding: 12px;
        box-sizing: border-box;
        overflow-y: auto;
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
            grid-template-columns: repeat(3, 1fr);
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
