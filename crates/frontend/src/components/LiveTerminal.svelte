<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import styles from './LiveTerminal.module.css';
    import type { TimeframeTelemetry, InstanceState } from '../types';
    import TelemetryTable from './TelemetryTable.svelte';
    import ChartToggles from './ChartToggles.svelte';
    import PriceChart from './PriceChart.svelte';
    import VolumeChart from './VolumeChart.svelte';
    import RvolChart from './RvolChart.svelte';
    import MacdChart from './MacdChart.svelte';
    import SqueezeChart from './SqueezeChart.svelte';
    import RsiChart from './RsiChart.svelte';
    import AdxChart from './AdxChart.svelte';
    import BbwpChart from './BbwpChart.svelte';
    import AtrChart from './AtrChart.svelte';

    const app = useAppStore();
    let { pairKey }: { pairKey: string } = $props();
    let showMicro = $state(true);
    let showFast = $state(true);
    let showSlow = $state(true);
    let showMacro = $state(true);

    let expandedTf = $state<string | null>(null);

    function label(tf: TimeframeTelemetry): string {
        const sec = tf.barDurationSec;
        let suffix: string;
        if (sec >= 86400) suffix = `${sec / 86400}d`;
        else if (sec >= 3600) suffix = `${sec / 3600}h`;
        else if (sec >= 60) suffix = `${sec / 60}m`;
        else suffix = `${sec}s`;

        if (sec >= 900) return `MACRO (${suffix})`;
        if (sec >= 300) return `SLOW (${suffix})`;
        if (sec >= 180) return `FAST (${suffix})`;
        return `MICRO (${suffix})`;
    }

    function tfKey(pairKey: string, tf: TimeframeTelemetry): string {
        return `${pairKey}-${tf.barDurationSec}-${tf.emaFastVal}-${tf.emaMediumVal}-${tf.emaSlowVal}-${tf.emaLongVal}`;
    }

    function toggleExpand(key: string) {
        expandedTf = expandedTf === key ? null : key;
    }

    let expandedChartKey = $state<string | null>(null);
    let triggerScreenshot = $state<(() => void) | null>(null);

    function handleChartDblClick(chartType: string, timeframe: number) {
        if (expandedTf === null) return;
        expandedChartKey = `${chartType}-${timeframe}`;
        triggerScreenshot = null;
    }

    function handleFullscreenKeydown(e: KeyboardEvent) {
        if (e.key === 'Escape') { expandedChartKey = null; triggerScreenshot = null; }
    }

    $effect(() => {
        if (expandedChartKey === null) return;
        window.addEventListener('keydown', handleFullscreenKeydown);
        return () => window.removeEventListener('keydown', handleFullscreenKeydown);
    });
</script>

<div class={styles.terminalWorkspace}>
    {#if app.instancesMap[pairKey]}
        {@const pair = app.instancesMap[pairKey]}

        <ChartToggles {pairKey} />
        <div class={styles.mtfGrid}>
        <!-- Micro-Term Column -->
        <div class="{styles.timescaleColumn} {!showMicro ? styles.hiddenPane : ''} {expandedTf === 'micro' ? styles.expandedTfColumn : ''}">
            <div class={styles.timescaleHeader}>
                <span class={styles.timescaleTitle}>{label(pair.microTerm)}</span>
                <div class={styles.headerActions}>
                    <span class={styles.timescalePrice}>{pair.microTerm.priceText}</span>
                    <button class={styles.expandBtn} onclick={() => toggleExpand('micro')} title={expandedTf === 'micro' ? 'Collapse' : 'Expand'}>
                        {expandedTf === 'micro' ? '✕' : '⛶'}
                    </button>
                </div>
            </div>
            <div class={styles.timescaleCharts}>
                <div class="{styles.panelBox} {styles.panePrice} {(!pair.microTerm.showEmas && !pair.microTerm.showBb && !pair.microTerm.showVwap) ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>PRICE</div>
                    {#key tfKey(pairKey, pair.microTerm)}
                        <PriceChart pairKey={pairKey} timeframe={pair.microTerm.barDurationSec} onDoubleClick={() => handleChartDblClick('price', pair.microTerm.barDurationSec)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneVol} {!pair.microTerm.showVolume ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>VOLUME</div>
                    {#key `${pairKey}-${pair.microTerm.barDurationSec}`}
                        <VolumeChart pairKey={pairKey} timeframe={pair.microTerm.barDurationSec} onDoubleClick={() => handleChartDblClick('volume', pair.microTerm.barDurationSec)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneRvol} {!pair.microTerm.showRvol ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>RVOL</div>
                    {#key `${pairKey}-${pair.microTerm.barDurationSec}-rvol`}
                        <RvolChart pairKey={pairKey} timeframe={pair.microTerm.barDurationSec} onDoubleClick={() => handleChartDblClick('rvol', pair.microTerm.barDurationSec)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneMacd} {!pair.microTerm.showMacd ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>MACD</div>
                    {#key `${pairKey}-${pair.microTerm.barDurationSec}-${pair.microTerm.macdFastVal}-${pair.microTerm.macdSlowVal}-${pair.microTerm.macdSignalVal}`}
                        <MacdChart pairKey={pairKey} timeframe={pair.microTerm.barDurationSec} onDoubleClick={() => handleChartDblClick('macd', pair.microTerm.barDurationSec)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneSqueeze} {!pair.microTerm.showSqueeze ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>SQUEEZE</div>
                    {#key `${pairKey}-${pair.microTerm.barDurationSec}-${pair.microTerm.squeezePeriodVal}`}
                        <SqueezeChart pairKey={pairKey} timeframe={pair.microTerm.barDurationSec} onDoubleClick={() => handleChartDblClick('squeeze', pair.microTerm.barDurationSec)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneRsi} {!pair.microTerm.showRsi ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>RSI</div>
                    {#key `${pairKey}-${pair.microTerm.barDurationSec}-${pair.microTerm.rsiPeriodVal}`}
                        <RsiChart pairKey={pairKey} timeframe={pair.microTerm.barDurationSec} onDoubleClick={() => handleChartDblClick('rsi', pair.microTerm.barDurationSec)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneAdx} {!pair.microTerm.showAdx ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>ADX</div>
                    {#key `${pairKey}-${pair.microTerm.barDurationSec}-${pair.microTerm.adxPeriodVal}`}
                        <AdxChart pairKey={pairKey} timeframe={pair.microTerm.barDurationSec} onDoubleClick={() => handleChartDblClick('adx', pair.microTerm.barDurationSec)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneBbwp} {!pair.microTerm.showBbwp ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>BBWP</div>
                    {#key `${pairKey}-${pair.microTerm.barDurationSec}-bbwp`}
                        <BbwpChart pairKey={pairKey} timeframe={pair.microTerm.barDurationSec} onDoubleClick={() => handleChartDblClick('bbwp', pair.microTerm.barDurationSec)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneAtr} {!pair.microTerm.showAtr ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>ATR</div>
                    {#key `${pairKey}-${pair.microTerm.barDurationSec}-${pair.microTerm.atrPeriodVal}`}
                        <AtrChart pairKey={pairKey} timeframe={pair.microTerm.barDurationSec} onDoubleClick={() => handleChartDblClick('atr', pair.microTerm.barDurationSec)} />
                    {/key}
                </div>
            </div>
        </div>

        <!-- Small-Term Column -->
        <div class="{styles.timescaleColumn} {!showFast ? styles.hiddenPane : ''} {expandedTf === 'fast' ? styles.expandedTfColumn : ''}">
            <div class={styles.timescaleHeader}>
                <span class={styles.timescaleTitle}>{label(pair.fastTerm)}</span>
                <div class={styles.headerActions}>
                    <span class={styles.timescalePrice}>{pair.fastTerm.priceText}</span>
                    <button class={styles.expandBtn} onclick={() => toggleExpand('fast')} title={expandedTf === 'fast' ? 'Collapse' : 'Expand'}>
                        {expandedTf === 'fast' ? '✕' : '⛶'}
                    </button>
                </div>
            </div>
            <div class={styles.timescaleCharts}>
                <div class="{styles.panelBox} {styles.panePrice} {(!pair.fastTerm.showEmas && !pair.fastTerm.showBb && !pair.fastTerm.showVwap) ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>PRICE</div>
                    {#key tfKey(pairKey, pair.fastTerm)}
                        <PriceChart pairKey={pairKey} timeframe={pair.fastTerm.barDurationSec} onDoubleClick={() => handleChartDblClick('price', pair.fastTerm.barDurationSec)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneVol} {!pair.fastTerm.showVolume ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>VOLUME</div>
                    {#key `${pairKey}-${pair.fastTerm.barDurationSec}`}
                        <VolumeChart pairKey={pairKey} timeframe={pair.fastTerm.barDurationSec} onDoubleClick={() => handleChartDblClick('volume', pair.fastTerm.barDurationSec)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneRvol} {!pair.fastTerm.showRvol ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>RVOL</div>
                    {#key `${pairKey}-${pair.fastTerm.barDurationSec}-rvol`}
                        <RvolChart pairKey={pairKey} timeframe={pair.fastTerm.barDurationSec} onDoubleClick={() => handleChartDblClick('rvol', pair.fastTerm.barDurationSec)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneMacd} {!pair.fastTerm.showMacd ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>MACD</div>
                    {#key `${pairKey}-${pair.fastTerm.barDurationSec}-${pair.fastTerm.macdFastVal}-${pair.fastTerm.macdSlowVal}-${pair.fastTerm.macdSignalVal}`}
                        <MacdChart pairKey={pairKey} timeframe={pair.fastTerm.barDurationSec} onDoubleClick={() => handleChartDblClick('macd', pair.fastTerm.barDurationSec)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneSqueeze} {!pair.fastTerm.showSqueeze ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>SQUEEZE</div>
                    {#key `${pairKey}-${pair.fastTerm.barDurationSec}-${pair.fastTerm.squeezePeriodVal}`}
                        <SqueezeChart pairKey={pairKey} timeframe={pair.fastTerm.barDurationSec} onDoubleClick={() => handleChartDblClick('squeeze', pair.fastTerm.barDurationSec)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneRsi} {!pair.fastTerm.showRsi ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>RSI</div>
                    {#key `${pairKey}-${pair.fastTerm.barDurationSec}-${pair.fastTerm.rsiPeriodVal}`}
                        <RsiChart pairKey={pairKey} timeframe={pair.fastTerm.barDurationSec} onDoubleClick={() => handleChartDblClick('rsi', pair.fastTerm.barDurationSec)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneAdx} {!pair.fastTerm.showAdx ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>ADX</div>
                    {#key `${pairKey}-${pair.fastTerm.barDurationSec}-${pair.fastTerm.adxPeriodVal}`}
                        <AdxChart pairKey={pairKey} timeframe={pair.fastTerm.barDurationSec} onDoubleClick={() => handleChartDblClick('adx', pair.fastTerm.barDurationSec)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneBbwp} {!pair.fastTerm.showBbwp ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>BBWP</div>
                    {#key `${pairKey}-${pair.fastTerm.barDurationSec}-bbwp`}
                        <BbwpChart pairKey={pairKey} timeframe={pair.fastTerm.barDurationSec} onDoubleClick={() => handleChartDblClick('bbwp', pair.fastTerm.barDurationSec)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneAtr} {!pair.fastTerm.showAtr ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>ATR</div>
                    {#key `${pairKey}-${pair.fastTerm.barDurationSec}-${pair.fastTerm.atrPeriodVal}`}
                        <AtrChart pairKey={pairKey} timeframe={pair.fastTerm.barDurationSec} onDoubleClick={() => handleChartDblClick('atr', pair.fastTerm.barDurationSec)} />
                    {/key}
                </div>
            </div>
        </div>

        <!-- Medium-Term Column -->
        <div class="{styles.timescaleColumn} {!showSlow ? styles.hiddenPane : ''} {expandedTf === 'slow' ? styles.expandedTfColumn : ''}">
            <div class={styles.timescaleHeader}>
                <span class={styles.timescaleTitle}>{label(pair.slowTerm)}</span>
                <div class={styles.headerActions}>
                    <span class={styles.timescalePrice}>{pair.slowTerm.priceText}</span>
                    <button class={styles.expandBtn} onclick={() => toggleExpand('slow')} title={expandedTf === 'slow' ? 'Collapse' : 'Expand'}>
                        {expandedTf === 'slow' ? '✕' : '⛶'}
                    </button>
                </div>
            </div>
            <div class={styles.timescaleCharts}>
                <div class="{styles.panelBox} {styles.panePrice} {(!pair.slowTerm.showEmas && !pair.slowTerm.showBb && !pair.slowTerm.showVwap) ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>PRICE</div>
                    {#key tfKey(pairKey, pair.slowTerm)}
                        <PriceChart pairKey={pairKey} timeframe={pair.slowTerm.barDurationSec} onDoubleClick={() => handleChartDblClick('price', pair.slowTerm.barDurationSec)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneVol} {!pair.slowTerm.showVolume ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>VOLUME</div>
                    {#key `${pairKey}-${pair.slowTerm.barDurationSec}`}
                        <VolumeChart pairKey={pairKey} timeframe={pair.slowTerm.barDurationSec} onDoubleClick={() => handleChartDblClick('volume', pair.slowTerm.barDurationSec)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneRvol} {!pair.slowTerm.showRvol ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>RVOL</div>
                    {#key `${pairKey}-${pair.slowTerm.barDurationSec}-rvol`}
                        <RvolChart pairKey={pairKey} timeframe={pair.slowTerm.barDurationSec} onDoubleClick={() => handleChartDblClick('rvol', pair.slowTerm.barDurationSec)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneMacd} {!pair.slowTerm.showMacd ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>MACD</div>
                    {#key `${pairKey}-${pair.slowTerm.barDurationSec}-${pair.slowTerm.macdFastVal}-${pair.slowTerm.macdSlowVal}-${pair.slowTerm.macdSignalVal}`}
                        <MacdChart pairKey={pairKey} timeframe={pair.slowTerm.barDurationSec} onDoubleClick={() => handleChartDblClick('macd', pair.slowTerm.barDurationSec)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneSqueeze} {!pair.slowTerm.showSqueeze ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>SQUEEZE</div>
                    {#key `${pairKey}-${pair.slowTerm.barDurationSec}-${pair.slowTerm.squeezePeriodVal}`}
                        <SqueezeChart pairKey={pairKey} timeframe={pair.slowTerm.barDurationSec} onDoubleClick={() => handleChartDblClick('squeeze', pair.slowTerm.barDurationSec)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneRsi} {!pair.slowTerm.showRsi ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>RSI</div>
                    {#key `${pairKey}-${pair.slowTerm.barDurationSec}-${pair.slowTerm.rsiPeriodVal}`}
                        <RsiChart pairKey={pairKey} timeframe={pair.slowTerm.barDurationSec} onDoubleClick={() => handleChartDblClick('rsi', pair.slowTerm.barDurationSec)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneAdx} {!pair.slowTerm.showAdx ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>ADX</div>
                    {#key `${pairKey}-${pair.slowTerm.barDurationSec}-${pair.slowTerm.adxPeriodVal}`}
                        <AdxChart pairKey={pairKey} timeframe={pair.slowTerm.barDurationSec} onDoubleClick={() => handleChartDblClick('adx', pair.slowTerm.barDurationSec)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneBbwp} {!pair.slowTerm.showBbwp ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>BBWP</div>
                    {#key `${pairKey}-${pair.slowTerm.barDurationSec}-bbwp`}
                        <BbwpChart pairKey={pairKey} timeframe={pair.slowTerm.barDurationSec} onDoubleClick={() => handleChartDblClick('bbwp', pair.slowTerm.barDurationSec)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneAtr} {!pair.slowTerm.showAtr ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>ATR</div>
                    {#key `${pairKey}-${pair.slowTerm.barDurationSec}-${pair.slowTerm.atrPeriodVal}`}
                        <AtrChart pairKey={pairKey} timeframe={pair.slowTerm.barDurationSec} onDoubleClick={() => handleChartDblClick('atr', pair.slowTerm.barDurationSec)} />
                    {/key}
                </div>
            </div>
        </div>

        <!-- Large-Term Column -->
        <div class="{styles.timescaleColumn} {!showMacro ? styles.hiddenPane : ''} {expandedTf === 'macro' ? styles.expandedTfColumn : ''}">
            <div class={styles.timescaleHeader}>
                <span class={styles.timescaleTitle}>{label(pair.macroTerm)}</span>
                <div class={styles.headerActions}>
                    <span class={styles.timescalePrice}>{pair.macroTerm.priceText}</span>
                    <button class={styles.expandBtn} onclick={() => toggleExpand('macro')} title={expandedTf === 'macro' ? 'Collapse' : 'Expand'}>
                        {expandedTf === 'macro' ? '✕' : '⛶'}
                    </button>
                </div>
            </div>
            <div class={styles.timescaleCharts}>
                <div class="{styles.panelBox} {styles.panePrice} {(!pair.macroTerm.showEmas && !pair.macroTerm.showBb && !pair.macroTerm.showVwap) ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>PRICE</div>
                    {#key tfKey(pairKey, pair.macroTerm)}
                        <PriceChart pairKey={pairKey} timeframe={pair.macroTerm.barDurationSec} onDoubleClick={() => handleChartDblClick('price', pair.macroTerm.barDurationSec)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneVol} {!pair.macroTerm.showVolume ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>VOLUME</div>
                    {#key `${pairKey}-${pair.macroTerm.barDurationSec}`}
                        <VolumeChart pairKey={pairKey} timeframe={pair.macroTerm.barDurationSec} onDoubleClick={() => handleChartDblClick('volume', pair.macroTerm.barDurationSec)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneRvol} {!pair.macroTerm.showRvol ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>RVOL</div>
                    {#key `${pairKey}-${pair.macroTerm.barDurationSec}-rvol`}
                        <RvolChart pairKey={pairKey} timeframe={pair.macroTerm.barDurationSec} onDoubleClick={() => handleChartDblClick('rvol', pair.macroTerm.barDurationSec)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneMacd} {!pair.macroTerm.showMacd ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>MACD</div>
                    {#key `${pairKey}-${pair.macroTerm.barDurationSec}-${pair.macroTerm.macdFastVal}-${pair.macroTerm.macdSlowVal}-${pair.macroTerm.macdSignalVal}`}
                        <MacdChart pairKey={pairKey} timeframe={pair.macroTerm.barDurationSec} onDoubleClick={() => handleChartDblClick('macd', pair.macroTerm.barDurationSec)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneSqueeze} {!pair.macroTerm.showSqueeze ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>SQUEEZE</div>
                    {#key `${pairKey}-${pair.macroTerm.barDurationSec}-${pair.macroTerm.squeezePeriodVal}`}
                        <SqueezeChart pairKey={pairKey} timeframe={pair.macroTerm.barDurationSec} onDoubleClick={() => handleChartDblClick('squeeze', pair.macroTerm.barDurationSec)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneRsi} {!pair.macroTerm.showRsi ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>RSI</div>
                    {#key `${pairKey}-${pair.macroTerm.barDurationSec}-${pair.macroTerm.rsiPeriodVal}`}
                        <RsiChart pairKey={pairKey} timeframe={pair.macroTerm.barDurationSec} onDoubleClick={() => handleChartDblClick('rsi', pair.macroTerm.barDurationSec)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneAdx} {!pair.macroTerm.showAdx ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>ADX</div>
                    {#key `${pairKey}-${pair.macroTerm.barDurationSec}-${pair.macroTerm.adxPeriodVal}`}
                        <AdxChart pairKey={pairKey} timeframe={pair.macroTerm.barDurationSec} onDoubleClick={() => handleChartDblClick('adx', pair.macroTerm.barDurationSec)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneBbwp} {!pair.macroTerm.showBbwp ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>BBWP</div>
                    {#key `${pairKey}-${pair.macroTerm.barDurationSec}-bbwp`}
                        <BbwpChart pairKey={pairKey} timeframe={pair.macroTerm.barDurationSec} onDoubleClick={() => handleChartDblClick('bbwp', pair.macroTerm.barDurationSec)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneAtr} {!pair.macroTerm.showAtr ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>ATR</div>
                    {#key `${pairKey}-${pair.macroTerm.barDurationSec}-${pair.macroTerm.atrPeriodVal}`}
                        <AtrChart pairKey={pairKey} timeframe={pair.macroTerm.barDurationSec} onDoubleClick={() => handleChartDblClick('atr', pair.macroTerm.barDurationSec)} />
                    {/key}
                </div>
            </div>
        </div>
        </div>
        <TelemetryTable {pairKey} />
    {/if}
</div>

{#if expandedChartKey !== null}
    {@const lastDash = expandedChartKey.lastIndexOf('-')}
    {@const chartType = expandedChartKey.slice(0, lastDash)}
    {@const timeframeSec = parseInt(expandedChartKey.slice(lastDash + 1))}
    <div class={styles.singleChartFullscreen}>
        <div class={styles.timescaleHeader}>
            <span class={styles.timescaleTitle}>{chartType.toUpperCase()}</span>
            <div class={styles.headerActions}>
                {#if triggerScreenshot}
                    <button class={styles.expandBtn} onclick={() => triggerScreenshot?.()} title="Save Screenshot">📸 SCREENSHOT</button>
                {/if}
                <button class={styles.expandBtn} onclick={() => { expandedChartKey = null; triggerScreenshot = null; }} title="Close">✕</button>
            </div>
        </div>
        <div class={styles.singleChartBody}>
            {#if chartType === 'price'}
                <PriceChart pairKey={pairKey} timeframe={timeframeSec} onDoubleClick={() => { expandedChartKey = null; triggerScreenshot = null; }} onScreenshotReady={(fn) => triggerScreenshot = fn} />
            {:else if chartType === 'volume'}
                <VolumeChart pairKey={pairKey} timeframe={timeframeSec} onDoubleClick={() => { expandedChartKey = null; triggerScreenshot = null; }} onScreenshotReady={(fn) => triggerScreenshot = fn} />
            {:else if chartType === 'rvol'}
                <RvolChart pairKey={pairKey} timeframe={timeframeSec} onDoubleClick={() => { expandedChartKey = null; triggerScreenshot = null; }} onScreenshotReady={(fn) => triggerScreenshot = fn} />
            {:else if chartType === 'macd'}
                <MacdChart pairKey={pairKey} timeframe={timeframeSec} onDoubleClick={() => { expandedChartKey = null; triggerScreenshot = null; }} onScreenshotReady={(fn) => triggerScreenshot = fn} />
            {:else if chartType === 'squeeze'}
                <SqueezeChart pairKey={pairKey} timeframe={timeframeSec} onDoubleClick={() => { expandedChartKey = null; triggerScreenshot = null; }} onScreenshotReady={(fn) => triggerScreenshot = fn} />
            {:else if chartType === 'rsi'}
                <RsiChart pairKey={pairKey} timeframe={timeframeSec} onDoubleClick={() => { expandedChartKey = null; triggerScreenshot = null; }} onScreenshotReady={(fn) => triggerScreenshot = fn} />
            {:else if chartType === 'adx'}
                <AdxChart pairKey={pairKey} timeframe={timeframeSec} onDoubleClick={() => { expandedChartKey = null; triggerScreenshot = null; }} onScreenshotReady={(fn) => triggerScreenshot = fn} />
            {:else if chartType === 'bbwp'}
                <BbwpChart pairKey={pairKey} timeframe={timeframeSec} onDoubleClick={() => { expandedChartKey = null; triggerScreenshot = null; }} onScreenshotReady={(fn) => triggerScreenshot = fn} />
            {:else if chartType === 'atr'}
                <AtrChart pairKey={pairKey} timeframe={timeframeSec} onDoubleClick={() => { expandedChartKey = null; triggerScreenshot = null; }} onScreenshotReady={(fn) => triggerScreenshot = fn} />
            {/if}
        </div>
    </div>
{/if}
