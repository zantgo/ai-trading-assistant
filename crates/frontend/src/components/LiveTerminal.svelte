<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import styles from './LiveTerminal.module.css';
    import type { TimeframeTelemetry, InstanceState } from '../types';
    import TelemetryTable from './TelemetryTable.svelte';
    import ChartToggles from './ChartToggles.svelte';
    import PriceChart from './PriceChart.svelte';
    import VolumeChart from './VolumeChart.svelte';
    import AdxChart from './AdxChart.svelte';
    import AtrChart from './AtrChart.svelte';
    import RsiChart from './RsiChart.svelte';
    import MacdChart from './MacdChart.svelte';
    import SqueezeChart from './SqueezeChart.svelte';
    import BbwpChart from './BbwpChart.svelte';

    const app = useAppStore();
    let { pairKey }: { pairKey: string } = $props();
    let showMicro = $state(true);
    let showSmall = $state(true);
    let showMedium = $state(true);
    let showLarge = $state(true);

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

<div class={styles.terminalWorkspace}>
    {#if app.instancesMap[pairKey]}
        {@const pair = app.instancesMap[pairKey]}

        <ChartToggles {pairKey} />
        <div class={styles.mtfGrid}>
        <!-- Micro-Term Column -->
        <div class="{styles.timescaleColumn} {!showMicro ? styles.hiddenPane : ''}">
            <div class={styles.timescaleHeader}>
                <span class={styles.timescaleTitle}>{label(pair.microTerm)}</span>
                <span class={styles.timescalePrice}>{pair.microTerm.priceText}</span>
            </div>
            <div class={styles.timescaleCharts}>
                <div class="{styles.panelBox} {styles.panePrice} {(!pair.microTerm.showEmas && !pair.microTerm.showBb && !pair.microTerm.showVwap) ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>PRICE</div>
                    {#key tfKey(pairKey, pair.microTerm)}
                        <PriceChart pairKey={pairKey} timeframe={60} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneVol} {!pair.microTerm.showVolume ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>VOLUME</div>
                    {#key `${pairKey}-${pair.microTerm.barDurationSec}`}
                        <VolumeChart pairKey={pairKey} timeframe={60} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneAdx} {!pair.microTerm.showAdx ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>ADX</div>
                    {#key `${pairKey}-${pair.microTerm.barDurationSec}-${pair.microTerm.adxPeriodVal}`}
                        <AdxChart pairKey={pairKey} timeframe={60} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneAtr} {!pair.microTerm.showAtr ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>ATR</div>
                    {#key `${pairKey}-${pair.microTerm.barDurationSec}-${pair.microTerm.atrPeriodVal}`}
                        <AtrChart pairKey={pairKey} timeframe={60} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneRsi} {!pair.microTerm.showRsi ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>RSI</div>
                    {#key `${pairKey}-${pair.microTerm.barDurationSec}-${pair.microTerm.rsiPeriodVal}`}
                        <RsiChart pairKey={pairKey} timeframe={60} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneMacd} {!pair.microTerm.showMacd ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>MACD</div>
                    {#key `${pairKey}-${pair.microTerm.barDurationSec}-${pair.microTerm.macdFastVal}-${pair.microTerm.macdSlowVal}-${pair.microTerm.macdSignalVal}`}
                        <MacdChart pairKey={pairKey} timeframe={60} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneSqueeze} {!pair.microTerm.showSqueeze ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>SQUEEZE</div>
                    {#key `${pairKey}-${pair.microTerm.barDurationSec}-${pair.microTerm.squeezePeriodVal}`}
                        <SqueezeChart pairKey={pairKey} timeframe={60} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneBbwp} {!pair.microTerm.showBbwp ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>BBWP</div>
                    {#key `${pairKey}-${pair.microTerm.barDurationSec}-bbwp`}
                        <BbwpChart pairKey={pairKey} timeframe={60} />
                    {/key}
                </div>
            </div>
        </div>

        <!-- Small-Term Column -->
        <div class="{styles.timescaleColumn} {!showSmall ? styles.hiddenPane : ''}">
            <div class={styles.timescaleHeader}>
                <span class={styles.timescaleTitle}>{label(pair.smallTerm)}</span>
                <span class={styles.timescalePrice}>{pair.smallTerm.priceText}</span>
            </div>
            <div class={styles.timescaleCharts}>
                <div class="{styles.panelBox} {styles.panePrice} {(!pair.smallTerm.showEmas && !pair.smallTerm.showBb && !pair.smallTerm.showVwap) ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>PRICE</div>
                    {#key tfKey(pairKey, pair.smallTerm)}
                        <PriceChart pairKey={pairKey} timeframe={300} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneVol} {!pair.smallTerm.showVolume ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>VOLUME</div>
                    {#key `${pairKey}-${pair.smallTerm.barDurationSec}`}
                        <VolumeChart pairKey={pairKey} timeframe={300} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneAdx} {!pair.smallTerm.showAdx ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>ADX</div>
                    {#key `${pairKey}-${pair.smallTerm.barDurationSec}-${pair.smallTerm.adxPeriodVal}`}
                        <AdxChart pairKey={pairKey} timeframe={300} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneAtr} {!pair.smallTerm.showAtr ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>ATR</div>
                    {#key `${pairKey}-${pair.smallTerm.barDurationSec}-${pair.smallTerm.atrPeriodVal}`}
                        <AtrChart pairKey={pairKey} timeframe={300} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneRsi} {!pair.smallTerm.showRsi ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>RSI</div>
                    {#key `${pairKey}-${pair.smallTerm.barDurationSec}-${pair.smallTerm.rsiPeriodVal}`}
                        <RsiChart pairKey={pairKey} timeframe={300} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneMacd} {!pair.smallTerm.showMacd ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>MACD</div>
                    {#key `${pairKey}-${pair.smallTerm.barDurationSec}-${pair.smallTerm.macdFastVal}-${pair.smallTerm.macdSlowVal}-${pair.smallTerm.macdSignalVal}`}
                        <MacdChart pairKey={pairKey} timeframe={300} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneSqueeze} {!pair.smallTerm.showSqueeze ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>SQUEEZE</div>
                    {#key `${pairKey}-${pair.smallTerm.barDurationSec}-${pair.smallTerm.squeezePeriodVal}`}
                        <SqueezeChart pairKey={pairKey} timeframe={300} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneBbwp} {!pair.smallTerm.showBbwp ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>BBWP</div>
                    {#key `${pairKey}-${pair.smallTerm.barDurationSec}-bbwp`}
                        <BbwpChart pairKey={pairKey} timeframe={300} />
                    {/key}
                </div>
            </div>
        </div>

        <!-- Medium-Term Column -->
        <div class="{styles.timescaleColumn} {!showMedium ? styles.hiddenPane : ''}">
            <div class={styles.timescaleHeader}>
                <span class={styles.timescaleTitle}>{label(pair.mediumTerm)}</span>
                <span class={styles.timescalePrice}>{pair.mediumTerm.priceText}</span>
            </div>
            <div class={styles.timescaleCharts}>
                <div class="{styles.panelBox} {styles.panePrice} {(!pair.mediumTerm.showEmas && !pair.mediumTerm.showBb && !pair.mediumTerm.showVwap) ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>PRICE</div>
                    {#key tfKey(pairKey, pair.mediumTerm)}
                        <PriceChart pairKey={pairKey} timeframe={900} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneVol} {!pair.mediumTerm.showVolume ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>VOLUME</div>
                    {#key `${pairKey}-${pair.mediumTerm.barDurationSec}`}
                        <VolumeChart pairKey={pairKey} timeframe={900} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneAdx} {!pair.mediumTerm.showAdx ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>ADX</div>
                    {#key `${pairKey}-${pair.mediumTerm.barDurationSec}-${pair.mediumTerm.adxPeriodVal}`}
                        <AdxChart pairKey={pairKey} timeframe={900} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneAtr} {!pair.mediumTerm.showAtr ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>ATR</div>
                    {#key `${pairKey}-${pair.mediumTerm.barDurationSec}-${pair.mediumTerm.atrPeriodVal}`}
                        <AtrChart pairKey={pairKey} timeframe={900} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneRsi} {!pair.mediumTerm.showRsi ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>RSI</div>
                    {#key `${pairKey}-${pair.mediumTerm.barDurationSec}-${pair.mediumTerm.rsiPeriodVal}`}
                        <RsiChart pairKey={pairKey} timeframe={900} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneMacd} {!pair.mediumTerm.showMacd ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>MACD</div>
                    {#key `${pairKey}-${pair.mediumTerm.barDurationSec}-${pair.mediumTerm.macdFastVal}-${pair.mediumTerm.macdSlowVal}-${pair.mediumTerm.macdSignalVal}`}
                        <MacdChart pairKey={pairKey} timeframe={900} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneSqueeze} {!pair.mediumTerm.showSqueeze ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>SQUEEZE</div>
                    {#key `${pairKey}-${pair.mediumTerm.barDurationSec}-${pair.mediumTerm.squeezePeriodVal}`}
                        <SqueezeChart pairKey={pairKey} timeframe={900} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneBbwp} {!pair.mediumTerm.showBbwp ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>BBWP</div>
                    {#key `${pairKey}-${pair.mediumTerm.barDurationSec}-bbwp`}
                        <BbwpChart pairKey={pairKey} timeframe={900} />
                    {/key}
                </div>
            </div>
        </div>

        <!-- Large-Term Column -->
        <div class="{styles.timescaleColumn} {!showLarge ? styles.hiddenPane : ''}">
            <div class={styles.timescaleHeader}>
                <span class={styles.timescaleTitle}>{label(pair.largeTerm)}</span>
                <span class={styles.timescalePrice}>{pair.largeTerm.priceText}</span>
            </div>
            <div class={styles.timescaleCharts}>
                <div class="{styles.panelBox} {styles.panePrice} {(!pair.largeTerm.showEmas && !pair.largeTerm.showBb && !pair.largeTerm.showVwap) ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>PRICE</div>
                    {#key tfKey(pairKey, pair.largeTerm)}
                        <PriceChart pairKey={pairKey} timeframe={3600} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneVol} {!pair.largeTerm.showVolume ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>VOLUME</div>
                    {#key `${pairKey}-${pair.largeTerm.barDurationSec}`}
                        <VolumeChart pairKey={pairKey} timeframe={3600} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneAdx} {!pair.largeTerm.showAdx ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>ADX</div>
                    {#key `${pairKey}-${pair.largeTerm.barDurationSec}-${pair.largeTerm.adxPeriodVal}`}
                        <AdxChart pairKey={pairKey} timeframe={3600} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneAtr} {!pair.largeTerm.showAtr ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>ATR</div>
                    {#key `${pairKey}-${pair.largeTerm.barDurationSec}-${pair.largeTerm.atrPeriodVal}`}
                        <AtrChart pairKey={pairKey} timeframe={3600} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneRsi} {!pair.largeTerm.showRsi ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>RSI</div>
                    {#key `${pairKey}-${pair.largeTerm.barDurationSec}-${pair.largeTerm.rsiPeriodVal}`}
                        <RsiChart pairKey={pairKey} timeframe={3600} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneMacd} {!pair.largeTerm.showMacd ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>MACD</div>
                    {#key `${pairKey}-${pair.largeTerm.barDurationSec}-${pair.largeTerm.macdFastVal}-${pair.largeTerm.macdSlowVal}-${pair.largeTerm.macdSignalVal}`}
                        <MacdChart pairKey={pairKey} timeframe={3600} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneSqueeze} {!pair.largeTerm.showSqueeze ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>SQUEEZE</div>
                    {#key `${pairKey}-${pair.largeTerm.barDurationSec}-${pair.largeTerm.squeezePeriodVal}`}
                        <SqueezeChart pairKey={pairKey} timeframe={3600} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneBbwp} {!pair.largeTerm.showBbwp ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>BBWP</div>
                    {#key `${pairKey}-${pair.largeTerm.barDurationSec}-bbwp`}
                        <BbwpChart pairKey={pairKey} timeframe={3600} />
                    {/key}
                </div>
            </div>
        </div>
        </div>
        <TelemetryTable {pairKey} />
    {/if}
</div>
