<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import styles from './LiveTerminal.module.css';
    import type { TimeframeTelemetry } from '../types';
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
    import StochasticChart from './StochasticChart.svelte';
    import ChandeMoChart from './ChandeMoChart.svelte';
    import ObvChart from './ObvChart.svelte';
    import CmfChart from './CmfChart.svelte';
    import MfiChart from './MfiChart.svelte';
    import HvChart from './HvChart.svelte';
    import AroonChart from './AroonChart.svelte';
    import ChoppinessChart from './ChoppinessChart.svelte';
    import LinRegSlopeChart from './LinRegSlopeChart.svelte';
    import ZScoreChart from './ZScoreChart.svelte';
    import CciChart from './CciChart.svelte';

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

<!-- Core panes: price overlay chart + primary oscillator/volume panes -->
{#snippet basePanes(tf: TimeframeTelemetry)}
    <div class="{styles.panelBox} {styles.panePrice} {(!tf.showEmas && !tf.showBb && !tf.showVwap && !tf.showAvwap && !tf.showSupertrend && !tf.showKeltner && !tf.showDonchian) ? styles.hiddenPane : ''}">
        <div class={styles.panelLabel}>PRICE</div>
        {#key tfKey(pairKey, tf)}
            <PriceChart pairKey={pairKey} timeframe={tf.barDurationSec} onDoubleClick={() => handleChartDblClick('price', tf.barDurationSec)} />
        {/key}
    </div>
    <div class="{styles.panelBox} {styles.paneVol} {!tf.showVolume ? styles.hiddenPane : ''}">
        <div class={styles.panelLabel}>VOLUME</div>
        {#key `${pairKey}-${tf.barDurationSec}`}
            <VolumeChart pairKey={pairKey} timeframe={tf.barDurationSec} onDoubleClick={() => handleChartDblClick('volume', tf.barDurationSec)} />
        {/key}
    </div>
    <div class="{styles.panelBox} {styles.paneRvol} {!tf.showRvol ? styles.hiddenPane : ''}">
        <div class={styles.panelLabel}>RVOL</div>
        {#key `${pairKey}-${tf.barDurationSec}-rvol`}
            <RvolChart pairKey={pairKey} timeframe={tf.barDurationSec} onDoubleClick={() => handleChartDblClick('rvol', tf.barDurationSec)} />
        {/key}
    </div>
    <div class="{styles.panelBox} {styles.paneMacd} {!tf.showMacd ? styles.hiddenPane : ''}">
        <div class={styles.panelLabel}>MACD</div>
        {#key `${pairKey}-${tf.barDurationSec}-${tf.macdFastVal}-${tf.macdSlowVal}-${tf.macdSignalVal}`}
            <MacdChart pairKey={pairKey} timeframe={tf.barDurationSec} onDoubleClick={() => handleChartDblClick('macd', tf.barDurationSec)} />
        {/key}
    </div>
    <div class="{styles.panelBox} {styles.paneSqueeze} {!tf.showSqueeze ? styles.hiddenPane : ''}">
        <div class={styles.panelLabel}>SQUEEZE</div>
        {#key `${pairKey}-${tf.barDurationSec}-${tf.squeezePeriodVal}`}
            <SqueezeChart pairKey={pairKey} timeframe={tf.barDurationSec} onDoubleClick={() => handleChartDblClick('squeeze', tf.barDurationSec)} />
        {/key}
    </div>
    <div class="{styles.panelBox} {styles.paneRsi} {!tf.showRsi ? styles.hiddenPane : ''}">
        <div class={styles.panelLabel}>RSI</div>
        {#key `${pairKey}-${tf.barDurationSec}-${tf.rsiPeriodVal}`}
            <RsiChart pairKey={pairKey} timeframe={tf.barDurationSec} onDoubleClick={() => handleChartDblClick('rsi', tf.barDurationSec)} />
        {/key}
    </div>
    <div class="{styles.panelBox} {styles.paneAdx} {!tf.showAdx ? styles.hiddenPane : ''}">
        <div class={styles.panelLabel}>ADX</div>
        {#key `${pairKey}-${tf.barDurationSec}-${tf.adxPeriodVal}`}
            <AdxChart pairKey={pairKey} timeframe={tf.barDurationSec} onDoubleClick={() => handleChartDblClick('adx', tf.barDurationSec)} />
        {/key}
    </div>
    <div class="{styles.panelBox} {styles.paneBbwp} {!tf.showBbwp ? styles.hiddenPane : ''}">
        <div class={styles.panelLabel}>BBWP</div>
        {#key `${pairKey}-${tf.barDurationSec}-bbwp`}
            <BbwpChart pairKey={pairKey} timeframe={tf.barDurationSec} onDoubleClick={() => handleChartDblClick('bbwp', tf.barDurationSec)} />
        {/key}
    </div>
    <div class="{styles.panelBox} {styles.paneAtr} {!tf.showAtr ? styles.hiddenPane : ''}">
        <div class={styles.panelLabel}>ATR</div>
        {#key `${pairKey}-${tf.barDurationSec}-${tf.atrPeriodVal}`}
            <AtrChart pairKey={pairKey} timeframe={tf.barDurationSec} onDoubleClick={() => handleChartDblClick('atr', tf.barDurationSec)} />
        {/key}
    </div>
    <div class="{styles.panelBox} {styles.paneStochastic} {!tf.showStochastic ? styles.hiddenPane : ''}">
        <div class={styles.panelLabel}>STOCHASTIC</div>
        {#key `${pairKey}-${tf.barDurationSec}-${tf.stochKPeriodVal}-${tf.stochDPeriodVal}-${tf.stochSPeriodVal}-stoch`}
            <StochasticChart pairKey={pairKey} timeframe={tf.barDurationSec} onDoubleClick={() => handleChartDblClick('stochastic', tf.barDurationSec)} />
        {/key}
    </div>
    <div class="{styles.panelBox} {styles.paneChandemo} {!tf.showChandeMo ? styles.hiddenPane : ''}">
        <div class={styles.panelLabel}>CHANDE MO</div>
        {#key `${pairKey}-${tf.barDurationSec}-${tf.chandemoPeriodVal}-cmo`}
            <ChandeMoChart pairKey={pairKey} timeframe={tf.barDurationSec} onDoubleClick={() => handleChartDblClick('chandemo', tf.barDurationSec)} />
        {/key}
    </div>
{/snippet}

<!-- Volume-family panes -->
{#snippet volPanes(tf: TimeframeTelemetry)}
    <div class="{styles.panelBox} {styles.paneObv} {!tf.showObv ? styles.hiddenPane : ''}">
        <div class={styles.panelLabel}>OBV</div>
        {#key `${pairKey}-${tf.barDurationSec}-${tf.obvSmoothingVal}-obv`}
            <ObvChart pairKey={pairKey} timeframe={tf.barDurationSec} onDoubleClick={() => handleChartDblClick('obv', tf.barDurationSec)} />
        {/key}
    </div>
    <div class="{styles.panelBox} {styles.paneCmf} {!tf.showCmf ? styles.hiddenPane : ''}">
        <div class={styles.panelLabel}>CHAIKIN MF</div>
        {#key `${pairKey}-${tf.barDurationSec}-${tf.cmfPeriodVal}-cmf`}
            <CmfChart pairKey={pairKey} timeframe={tf.barDurationSec} onDoubleClick={() => handleChartDblClick('cmf', tf.barDurationSec)} />
        {/key}
    </div>
    <div class="{styles.panelBox} {styles.paneMfi} {!tf.showMfi ? styles.hiddenPane : ''}">
        <div class={styles.panelLabel}>MFI</div>
        {#key `${pairKey}-${tf.barDurationSec}-${tf.mfiPeriodVal}-mfi`}
            <MfiChart pairKey={pairKey} timeframe={tf.barDurationSec} onDoubleClick={() => handleChartDblClick('mfi', tf.barDurationSec)} />
        {/key}
    </div>
    <div class="{styles.panelBox} {styles.paneHv} {!tf.showHv ? styles.hiddenPane : ''}">
        <div class={styles.panelLabel}>HIST VOL</div>
        {#key `${pairKey}-${tf.barDurationSec}-${tf.hvPeriodVal}-hv`}
            <HvChart pairKey={pairKey} timeframe={tf.barDurationSec} onDoubleClick={() => handleChartDblClick('hv', tf.barDurationSec)} />
        {/key}
    </div>
{/snippet}

<!-- Market-regime panes -->
{#snippet regimePanes(tf: TimeframeTelemetry)}
    <div class="{styles.panelBox} {styles.paneAroon} {!tf.showAroon ? styles.hiddenPane : ''}">
        <div class={styles.panelLabel}>AROON</div>
        {#key `${pairKey}-${tf.barDurationSec}-${tf.aroonPeriodVal}-aroon`}
            <AroonChart pairKey={pairKey} timeframe={tf.barDurationSec} onDoubleClick={() => handleChartDblClick('aroon', tf.barDurationSec)} />
        {/key}
    </div>
    <div class="{styles.panelBox} {styles.paneChoppiness} {!tf.showChoppiness ? styles.hiddenPane : ''}">
        <div class={styles.panelLabel}>CHOPPINESS</div>
        {#key `${pairKey}-${tf.barDurationSec}-${tf.chopPeriodVal}-chop`}
            <ChoppinessChart pairKey={pairKey} timeframe={tf.barDurationSec} onDoubleClick={() => handleChartDblClick('choppiness', tf.barDurationSec)} />
        {/key}
    </div>
    <div class="{styles.panelBox} {styles.paneLinreg} {!tf.showLinregSlope ? styles.hiddenPane : ''}">
        <div class={styles.panelLabel}>LINREG SLOPE</div>
        {#key `${pairKey}-${tf.barDurationSec}-${tf.linregPeriodVal}-linreg`}
            <LinRegSlopeChart pairKey={pairKey} timeframe={tf.barDurationSec} onDoubleClick={() => handleChartDblClick('linreg_slope', tf.barDurationSec)} />
        {/key}
    </div>
    <div class="{styles.panelBox} {styles.paneZscore} {!tf.showZscore ? styles.hiddenPane : ''}">
        <div class={styles.panelLabel}>Z-SCORE</div>
        {#key `${pairKey}-${tf.barDurationSec}-${tf.zscorePeriodVal}-zscore`}
            <ZScoreChart pairKey={pairKey} timeframe={tf.barDurationSec} onDoubleClick={() => handleChartDblClick('zscore', tf.barDurationSec)} />
        {/key}
    </div>
    <div class="{styles.panelBox} {styles.paneCci} {!tf.showCci ? styles.hiddenPane : ''}">
        <div class={styles.panelLabel}>CCI</div>
        {#key `${pairKey}-${tf.barDurationSec}-cci`}
            <CciChart pairKey={pairKey} timeframe={tf.barDurationSec} onDoubleClick={() => handleChartDblClick('cci', tf.barDurationSec)} />
        {/key}
    </div>
{/snippet}

<!-- Full timeframe column (header + all panes) -->
{#snippet column(tf: TimeframeTelemetry, colId: string, visible: boolean)}
    <div class="{styles.timescaleColumn} {!visible ? styles.hiddenPane : ''} {expandedTf === colId ? styles.expandedTfColumn : ''}">
        <div class={styles.timescaleHeader}>
            <span class={styles.timescaleTitle}>{label(tf)}</span>
            <div class={styles.headerActions}>
                <span class={styles.timescalePrice}>{tf.priceText}</span>
                <button class={styles.expandBtn} onclick={() => toggleExpand(colId)} title={expandedTf === colId ? 'Collapse' : 'Expand'}>
                    {expandedTf === colId ? '✕' : '⛶'}
                </button>
            </div>
        </div>
        <div class={styles.timescaleCharts}>
            {@render basePanes(tf)}
            {@render volPanes(tf)}
            {@render regimePanes(tf)}
        </div>
    </div>
{/snippet}

<div class={styles.terminalWorkspace}>
    {#if app.instancesMap[pairKey]}
        {@const pair = app.instancesMap[pairKey]}
        <ChartToggles {pairKey} />
        <div class={styles.mtfGrid}>
            {@render column(pair.microTerm, 'micro', showMicro)}
            {@render column(pair.fastTerm, 'fast', showFast)}
            {@render column(pair.slowTerm, 'slow', showSlow)}
            {@render column(pair.macroTerm, 'macro', showMacro)}
        </div>
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
            {:else if chartType === 'stochastic'}
                <StochasticChart pairKey={pairKey} timeframe={timeframeSec} onDoubleClick={() => { expandedChartKey = null; triggerScreenshot = null; }} onScreenshotReady={(fn) => triggerScreenshot = fn} />
            {:else if chartType === 'chandemo'}
                <ChandeMoChart pairKey={pairKey} timeframe={timeframeSec} onDoubleClick={() => { expandedChartKey = null; triggerScreenshot = null; }} onScreenshotReady={(fn) => triggerScreenshot = fn} />
            {:else if chartType === 'obv'}
                <ObvChart pairKey={pairKey} timeframe={timeframeSec} onDoubleClick={() => { expandedChartKey = null; triggerScreenshot = null; }} onScreenshotReady={(fn) => triggerScreenshot = fn} />
            {:else if chartType === 'cmf'}
                <CmfChart pairKey={pairKey} timeframe={timeframeSec} onDoubleClick={() => { expandedChartKey = null; triggerScreenshot = null; }} onScreenshotReady={(fn) => triggerScreenshot = fn} />
            {:else if chartType === 'mfi'}
                <MfiChart pairKey={pairKey} timeframe={timeframeSec} onDoubleClick={() => { expandedChartKey = null; triggerScreenshot = null; }} onScreenshotReady={(fn) => triggerScreenshot = fn} />
            {:else if chartType === 'hv'}
                <HvChart pairKey={pairKey} timeframe={timeframeSec} onDoubleClick={() => { expandedChartKey = null; triggerScreenshot = null; }} onScreenshotReady={(fn) => triggerScreenshot = fn} />
            {:else if chartType === 'aroon'}
                <AroonChart pairKey={pairKey} timeframe={timeframeSec} onDoubleClick={() => { expandedChartKey = null; triggerScreenshot = null; }} onScreenshotReady={(fn) => triggerScreenshot = fn} />
            {:else if chartType === 'choppiness'}
                <ChoppinessChart pairKey={pairKey} timeframe={timeframeSec} onDoubleClick={() => { expandedChartKey = null; triggerScreenshot = null; }} onScreenshotReady={(fn) => triggerScreenshot = fn} />
            {:else if chartType === 'linreg_slope'}
                <LinRegSlopeChart pairKey={pairKey} timeframe={timeframeSec} onDoubleClick={() => { expandedChartKey = null; triggerScreenshot = null; }} onScreenshotReady={(fn) => triggerScreenshot = fn} />
            {:else if chartType === 'zscore'}
                <ZScoreChart pairKey={pairKey} timeframe={timeframeSec} onDoubleClick={() => { expandedChartKey = null; triggerScreenshot = null; }} onScreenshotReady={(fn) => triggerScreenshot = fn} />
            {:else if chartType === 'cci'}
                <CciChart pairKey={pairKey} timeframe={timeframeSec} onDoubleClick={() => { expandedChartKey = null; triggerScreenshot = null; }} onScreenshotReady={(fn) => triggerScreenshot = fn} />
            {/if}
        </div>
    </div>
{/if}
