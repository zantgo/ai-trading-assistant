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

    const app = getState();
    let { pairKey }: { pairKey: string } = $props();

    function label(tf: TimeframeTelemetry): string {
        const sec = tf.barDurationSec;
        if (sec >= 300) return 'LONG (5m)';
        if (sec >= 60) return 'MID (1m)';
        return 'SHORT (15s)';
    }

    function tfKey(pairKey: string, tf: TimeframeTelemetry): string {
        return `${pairKey}-${tf.barDurationSec}-${tf.emaFastVal}-${tf.emaMediumVal}-${tf.emaSlowVal}-${tf.emaLongVal}`;
    }
</script>

<div class="terminal-workspace">
    {#if app.pairsMap[pairKey]}
        {@const pair = app.pairsMap[pairKey]}

        <!-- Short-Term Column -->
        <div class="timescale-column">
            <div class="timescale-header">
                <span class="timescale-title">{label(pair.shortTerm)}</span>
                <span class="timescale-price">{pair.shortTerm.priceText}</span>
            </div>
            <div class="timescale-charts">
                <div class="panel-box pane-price" class:hidden-pane={!pair.shortTerm.showEmas && !pair.shortTerm.showBb && !pair.shortTerm.showVwap}>
                    {#key tfKey(pairKey, pair.shortTerm)}
                        <PriceChart pairKey={pairKey} timeframe={15} />
                    {/key}
                </div>
                <div class="panel-box pane-vol" class:hidden-pane={!pair.shortTerm.showVolume}>
                    {#key `${pairKey}-${pair.shortTerm.barDurationSec}`}
                        <VolumeChart pairKey={pairKey} timeframe={15} />
                    {/key}
                </div>
                <div class="panel-box pane-adx" class:hidden-pane={!pair.shortTerm.showAdx}>
                    {#key `${pairKey}-${pair.shortTerm.barDurationSec}-${pair.shortTerm.adxPeriodVal}`}
                        <AdxChart pairKey={pairKey} timeframe={15} />
                    {/key}
                </div>
                <div class="panel-box pane-atr" class:hidden-pane={!pair.shortTerm.showAtr}>
                    {#key `${pairKey}-${pair.shortTerm.barDurationSec}-${pair.shortTerm.atrPeriodVal}`}
                        <AtrChart pairKey={pairKey} timeframe={15} />
                    {/key}
                </div>
                <div class="panel-box pane-rsi" class:hidden-pane={!pair.shortTerm.showRsi}>
                    {#key `${pairKey}-${pair.shortTerm.barDurationSec}-${pair.shortTerm.rsiPeriodVal}`}
                        <RsiChart pairKey={pairKey} timeframe={15} />
                    {/key}
                </div>
                <div class="panel-box pane-macd" class:hidden-pane={!pair.shortTerm.showMacd}>
                    {#key `${pairKey}-${pair.shortTerm.barDurationSec}-${pair.shortTerm.macdFastVal}-${pair.shortTerm.macdSlowVal}-${pair.shortTerm.macdSignalVal}`}
                        <MacdChart pairKey={pairKey} timeframe={15} />
                    {/key}
                </div>
                <div class="panel-box pane-squeeze" class:hidden-pane={!pair.shortTerm.showSqueeze}>
                    {#key `${pairKey}-${pair.shortTerm.barDurationSec}-${pair.shortTerm.squeezePeriodVal}`}
                        <SqueezeChart pairKey={pairKey} timeframe={15} />
                    {/key}
                </div>
            </div>
        </div>

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
            </div>
        </div>
    {/if}
</div>

<style>
    .terminal-workspace {
        display: grid;
        grid-template-columns: repeat(3, 1fr);
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
    .hidden-pane { display: none; }

    @media (max-width: 1280px) {
        .terminal-workspace {
            grid-template-columns: 1fr;
            grid-template-rows: repeat(3, 400px);
            overflow-y: auto;
        }
    }
</style>
