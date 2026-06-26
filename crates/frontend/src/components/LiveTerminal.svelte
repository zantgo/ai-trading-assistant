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
    let showSmall = $state(true);
    let showMedium = $state(true);
    let showLarge = $state(true);

    let expandedTf = $state<string | null>(null);

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
                        <PriceChart pairKey={pairKey} timeframe={60} onDoubleClick={() => handleChartDblClick('price', 60)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneVol} {!pair.microTerm.showVolume ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>VOLUME</div>
                    {#key `${pairKey}-${pair.microTerm.barDurationSec}`}
                        <VolumeChart pairKey={pairKey} timeframe={60} onDoubleClick={() => handleChartDblClick('volume', 60)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneRvol} {!pair.microTerm.showRvol ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>RVOL</div>
                    {#key `${pairKey}-${pair.microTerm.barDurationSec}-rvol`}
                        <RvolChart pairKey={pairKey} timeframe={60} onDoubleClick={() => handleChartDblClick('rvol', 60)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneMacd} {!pair.microTerm.showMacd ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>MACD</div>
                    {#key `${pairKey}-${pair.microTerm.barDurationSec}-${pair.microTerm.macdFastVal}-${pair.microTerm.macdSlowVal}-${pair.microTerm.macdSignalVal}`}
                        <MacdChart pairKey={pairKey} timeframe={60} onDoubleClick={() => handleChartDblClick('macd', 60)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneSqueeze} {!pair.microTerm.showSqueeze ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>SQUEEZE</div>
                    {#key `${pairKey}-${pair.microTerm.barDurationSec}-${pair.microTerm.squeezePeriodVal}`}
                        <SqueezeChart pairKey={pairKey} timeframe={60} onDoubleClick={() => handleChartDblClick('squeeze', 60)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneRsi} {!pair.microTerm.showRsi ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>RSI</div>
                    {#key `${pairKey}-${pair.microTerm.barDurationSec}-${pair.microTerm.rsiPeriodVal}`}
                        <RsiChart pairKey={pairKey} timeframe={60} onDoubleClick={() => handleChartDblClick('rsi', 60)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneAdx} {!pair.microTerm.showAdx ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>ADX</div>
                    {#key `${pairKey}-${pair.microTerm.barDurationSec}-${pair.microTerm.adxPeriodVal}`}
                        <AdxChart pairKey={pairKey} timeframe={60} onDoubleClick={() => handleChartDblClick('adx', 60)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneBbwp} {!pair.microTerm.showBbwp ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>BBWP</div>
                    {#key `${pairKey}-${pair.microTerm.barDurationSec}-bbwp`}
                        <BbwpChart pairKey={pairKey} timeframe={60} onDoubleClick={() => handleChartDblClick('bbwp', 60)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneAtr} {!pair.microTerm.showAtr ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>ATR</div>
                    {#key `${pairKey}-${pair.microTerm.barDurationSec}-${pair.microTerm.atrPeriodVal}`}
                        <AtrChart pairKey={pairKey} timeframe={60} onDoubleClick={() => handleChartDblClick('atr', 60)} />
                    {/key}
                </div>
            </div>
        </div>

        <!-- Small-Term Column -->
        <div class="{styles.timescaleColumn} {!showSmall ? styles.hiddenPane : ''} {expandedTf === 'small' ? styles.expandedTfColumn : ''}">
            <div class={styles.timescaleHeader}>
                <span class={styles.timescaleTitle}>{label(pair.smallTerm)}</span>
                <div class={styles.headerActions}>
                    <span class={styles.timescalePrice}>{pair.smallTerm.priceText}</span>
                    <button class={styles.expandBtn} onclick={() => toggleExpand('small')} title={expandedTf === 'small' ? 'Collapse' : 'Expand'}>
                        {expandedTf === 'small' ? '✕' : '⛶'}
                    </button>
                </div>
            </div>
            <div class={styles.timescaleCharts}>
                <div class="{styles.panelBox} {styles.panePrice} {(!pair.smallTerm.showEmas && !pair.smallTerm.showBb && !pair.smallTerm.showVwap) ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>PRICE</div>
                    {#key tfKey(pairKey, pair.smallTerm)}
                        <PriceChart pairKey={pairKey} timeframe={300} onDoubleClick={() => handleChartDblClick('price', 300)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneVol} {!pair.smallTerm.showVolume ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>VOLUME</div>
                    {#key `${pairKey}-${pair.smallTerm.barDurationSec}`}
                        <VolumeChart pairKey={pairKey} timeframe={300} onDoubleClick={() => handleChartDblClick('volume', 300)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneRvol} {!pair.smallTerm.showRvol ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>RVOL</div>
                    {#key `${pairKey}-${pair.smallTerm.barDurationSec}-rvol`}
                        <RvolChart pairKey={pairKey} timeframe={300} onDoubleClick={() => handleChartDblClick('rvol', 300)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneMacd} {!pair.smallTerm.showMacd ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>MACD</div>
                    {#key `${pairKey}-${pair.smallTerm.barDurationSec}-${pair.smallTerm.macdFastVal}-${pair.smallTerm.macdSlowVal}-${pair.smallTerm.macdSignalVal}`}
                        <MacdChart pairKey={pairKey} timeframe={300} onDoubleClick={() => handleChartDblClick('macd', 300)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneSqueeze} {!pair.smallTerm.showSqueeze ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>SQUEEZE</div>
                    {#key `${pairKey}-${pair.smallTerm.barDurationSec}-${pair.smallTerm.squeezePeriodVal}`}
                        <SqueezeChart pairKey={pairKey} timeframe={300} onDoubleClick={() => handleChartDblClick('squeeze', 300)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneRsi} {!pair.smallTerm.showRsi ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>RSI</div>
                    {#key `${pairKey}-${pair.smallTerm.barDurationSec}-${pair.smallTerm.rsiPeriodVal}`}
                        <RsiChart pairKey={pairKey} timeframe={300} onDoubleClick={() => handleChartDblClick('rsi', 300)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneAdx} {!pair.smallTerm.showAdx ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>ADX</div>
                    {#key `${pairKey}-${pair.smallTerm.barDurationSec}-${pair.smallTerm.adxPeriodVal}`}
                        <AdxChart pairKey={pairKey} timeframe={300} onDoubleClick={() => handleChartDblClick('adx', 300)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneBbwp} {!pair.smallTerm.showBbwp ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>BBWP</div>
                    {#key `${pairKey}-${pair.smallTerm.barDurationSec}-bbwp`}
                        <BbwpChart pairKey={pairKey} timeframe={300} onDoubleClick={() => handleChartDblClick('bbwp', 300)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneAtr} {!pair.smallTerm.showAtr ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>ATR</div>
                    {#key `${pairKey}-${pair.smallTerm.barDurationSec}-${pair.smallTerm.atrPeriodVal}`}
                        <AtrChart pairKey={pairKey} timeframe={300} onDoubleClick={() => handleChartDblClick('atr', 300)} />
                    {/key}
                </div>
            </div>
        </div>

        <!-- Medium-Term Column -->
        <div class="{styles.timescaleColumn} {!showMedium ? styles.hiddenPane : ''} {expandedTf === 'medium' ? styles.expandedTfColumn : ''}">
            <div class={styles.timescaleHeader}>
                <span class={styles.timescaleTitle}>{label(pair.mediumTerm)}</span>
                <div class={styles.headerActions}>
                    <span class={styles.timescalePrice}>{pair.mediumTerm.priceText}</span>
                    <button class={styles.expandBtn} onclick={() => toggleExpand('medium')} title={expandedTf === 'medium' ? 'Collapse' : 'Expand'}>
                        {expandedTf === 'medium' ? '✕' : '⛶'}
                    </button>
                </div>
            </div>
            <div class={styles.timescaleCharts}>
                <div class="{styles.panelBox} {styles.panePrice} {(!pair.mediumTerm.showEmas && !pair.mediumTerm.showBb && !pair.mediumTerm.showVwap) ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>PRICE</div>
                    {#key tfKey(pairKey, pair.mediumTerm)}
                        <PriceChart pairKey={pairKey} timeframe={900} onDoubleClick={() => handleChartDblClick('price', 900)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneVol} {!pair.mediumTerm.showVolume ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>VOLUME</div>
                    {#key `${pairKey}-${pair.mediumTerm.barDurationSec}`}
                        <VolumeChart pairKey={pairKey} timeframe={900} onDoubleClick={() => handleChartDblClick('volume', 900)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneRvol} {!pair.mediumTerm.showRvol ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>RVOL</div>
                    {#key `${pairKey}-${pair.mediumTerm.barDurationSec}-rvol`}
                        <RvolChart pairKey={pairKey} timeframe={900} onDoubleClick={() => handleChartDblClick('rvol', 900)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneMacd} {!pair.mediumTerm.showMacd ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>MACD</div>
                    {#key `${pairKey}-${pair.mediumTerm.barDurationSec}-${pair.mediumTerm.macdFastVal}-${pair.mediumTerm.macdSlowVal}-${pair.mediumTerm.macdSignalVal}`}
                        <MacdChart pairKey={pairKey} timeframe={900} onDoubleClick={() => handleChartDblClick('macd', 900)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneSqueeze} {!pair.mediumTerm.showSqueeze ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>SQUEEZE</div>
                    {#key `${pairKey}-${pair.mediumTerm.barDurationSec}-${pair.mediumTerm.squeezePeriodVal}`}
                        <SqueezeChart pairKey={pairKey} timeframe={900} onDoubleClick={() => handleChartDblClick('squeeze', 900)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneRsi} {!pair.mediumTerm.showRsi ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>RSI</div>
                    {#key `${pairKey}-${pair.mediumTerm.barDurationSec}-${pair.mediumTerm.rsiPeriodVal}`}
                        <RsiChart pairKey={pairKey} timeframe={900} onDoubleClick={() => handleChartDblClick('rsi', 900)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneAdx} {!pair.mediumTerm.showAdx ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>ADX</div>
                    {#key `${pairKey}-${pair.mediumTerm.barDurationSec}-${pair.mediumTerm.adxPeriodVal}`}
                        <AdxChart pairKey={pairKey} timeframe={900} onDoubleClick={() => handleChartDblClick('adx', 900)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneBbwp} {!pair.mediumTerm.showBbwp ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>BBWP</div>
                    {#key `${pairKey}-${pair.mediumTerm.barDurationSec}-bbwp`}
                        <BbwpChart pairKey={pairKey} timeframe={900} onDoubleClick={() => handleChartDblClick('bbwp', 900)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneAtr} {!pair.mediumTerm.showAtr ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>ATR</div>
                    {#key `${pairKey}-${pair.mediumTerm.barDurationSec}-${pair.mediumTerm.atrPeriodVal}`}
                        <AtrChart pairKey={pairKey} timeframe={900} onDoubleClick={() => handleChartDblClick('atr', 900)} />
                    {/key}
                </div>
            </div>
        </div>

        <!-- Large-Term Column -->
        <div class="{styles.timescaleColumn} {!showLarge ? styles.hiddenPane : ''} {expandedTf === 'large' ? styles.expandedTfColumn : ''}">
            <div class={styles.timescaleHeader}>
                <span class={styles.timescaleTitle}>{label(pair.largeTerm)}</span>
                <div class={styles.headerActions}>
                    <span class={styles.timescalePrice}>{pair.largeTerm.priceText}</span>
                    <button class={styles.expandBtn} onclick={() => toggleExpand('large')} title={expandedTf === 'large' ? 'Collapse' : 'Expand'}>
                        {expandedTf === 'large' ? '✕' : '⛶'}
                    </button>
                </div>
            </div>
            <div class={styles.timescaleCharts}>
                <div class="{styles.panelBox} {styles.panePrice} {(!pair.largeTerm.showEmas && !pair.largeTerm.showBb && !pair.largeTerm.showVwap) ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>PRICE</div>
                    {#key tfKey(pairKey, pair.largeTerm)}
                        <PriceChart pairKey={pairKey} timeframe={3600} onDoubleClick={() => handleChartDblClick('price', 3600)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneVol} {!pair.largeTerm.showVolume ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>VOLUME</div>
                    {#key `${pairKey}-${pair.largeTerm.barDurationSec}`}
                        <VolumeChart pairKey={pairKey} timeframe={3600} onDoubleClick={() => handleChartDblClick('volume', 3600)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneRvol} {!pair.largeTerm.showRvol ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>RVOL</div>
                    {#key `${pairKey}-${pair.largeTerm.barDurationSec}-rvol`}
                        <RvolChart pairKey={pairKey} timeframe={3600} onDoubleClick={() => handleChartDblClick('rvol', 3600)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneMacd} {!pair.largeTerm.showMacd ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>MACD</div>
                    {#key `${pairKey}-${pair.largeTerm.barDurationSec}-${pair.largeTerm.macdFastVal}-${pair.largeTerm.macdSlowVal}-${pair.largeTerm.macdSignalVal}`}
                        <MacdChart pairKey={pairKey} timeframe={3600} onDoubleClick={() => handleChartDblClick('macd', 3600)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneSqueeze} {!pair.largeTerm.showSqueeze ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>SQUEEZE</div>
                    {#key `${pairKey}-${pair.largeTerm.barDurationSec}-${pair.largeTerm.squeezePeriodVal}`}
                        <SqueezeChart pairKey={pairKey} timeframe={3600} onDoubleClick={() => handleChartDblClick('squeeze', 3600)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneRsi} {!pair.largeTerm.showRsi ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>RSI</div>
                    {#key `${pairKey}-${pair.largeTerm.barDurationSec}-${pair.largeTerm.rsiPeriodVal}`}
                        <RsiChart pairKey={pairKey} timeframe={3600} onDoubleClick={() => handleChartDblClick('rsi', 3600)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneAdx} {!pair.largeTerm.showAdx ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>ADX</div>
                    {#key `${pairKey}-${pair.largeTerm.barDurationSec}-${pair.largeTerm.adxPeriodVal}`}
                        <AdxChart pairKey={pairKey} timeframe={3600} onDoubleClick={() => handleChartDblClick('adx', 3600)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneBbwp} {!pair.largeTerm.showBbwp ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>BBWP</div>
                    {#key `${pairKey}-${pair.largeTerm.barDurationSec}-bbwp`}
                        <BbwpChart pairKey={pairKey} timeframe={3600} onDoubleClick={() => handleChartDblClick('bbwp', 3600)} />
                    {/key}
                </div>
                <div class="{styles.panelBox} {styles.paneAtr} {!pair.largeTerm.showAtr ? styles.hiddenPane : ''}">
                    <div class={styles.panelLabel}>ATR</div>
                    {#key `${pairKey}-${pair.largeTerm.barDurationSec}-${pair.largeTerm.atrPeriodVal}`}
                        <AtrChart pairKey={pairKey} timeframe={3600} onDoubleClick={() => handleChartDblClick('atr', 3600)} />
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
