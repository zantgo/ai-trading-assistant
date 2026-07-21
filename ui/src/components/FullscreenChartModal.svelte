<script lang="ts">
    // FullscreenChartModal — single-chart fullscreen overlay. The dispatcher
    // mirrors the union declared in `LiveTerminal.svelte` (`ChartType`) so
    // adding a pane in one place is enforced in both.
    import { useAppStore } from '../state.svelte';
    import styles from './LiveTerminal.module.css';
    import FullscreenToolbar from './FullscreenToolbar.svelte';
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
    import WilliamsRChart from './WilliamsRChart.svelte';
    import CciChart from './CciChart.svelte';
    import ForceIndexChart from './ForceIndexChart.svelte';
    import ObvChart from './ObvChart.svelte';
    import CmfChart from './CmfChart.svelte';
    import MfiChart from './MfiChart.svelte';
    import HvChart from './HvChart.svelte';
    import AroonChart from './AroonChart.svelte';
    import ChoppinessChart from './ChoppinessChart.svelte';
    import LinRegSlopeChart from './LinRegSlopeChart.svelte';
    import ZScoreChart from './ZScoreChart.svelte';
    import FundingChart from './FundingChart.svelte';
    import OpenInterestChart from './OpenInterestChart.svelte';
    import OiDeltaChart from './OiDeltaChart.svelte';
    import OrderFlowDepthChart from './OrderFlowDepthChart.svelte';

    const app = useAppStore();
    let triggerScreenshot = $state<(() => void) | null>(null);

    function handleFullscreenKeydown(e: KeyboardEvent) {
        if (e.key === 'Escape') {
            app.closeFullscreenChart();
            triggerScreenshot = null;
        }
    }

    $effect(() => {
        if (app.fullscreenChart === null) return;
        window.addEventListener('keydown', handleFullscreenKeydown);
        return () => window.removeEventListener('keydown', handleFullscreenKeydown);
    });

    function handleClose() {
        app.closeFullscreenChart();
        triggerScreenshot = null;
    }

    function handleScreenshot() {
        if (triggerScreenshot) triggerScreenshot();
    }
</script>

{#if app.fullscreenChart !== null}
    {@const { chartType, slot, pairKey } = app.fullscreenChart}
    <div class={styles.singleChartFullscreen}>
        <div class={styles.singleChartBody}>
            {#if chartType === 'price'}
                <PriceChart {pairKey} {slot} onDoubleClick={handleClose} onScreenshotReady={(fn) => triggerScreenshot = fn} />
            {:else if chartType === 'volume'}
                <VolumeChart {pairKey} {slot} onDoubleClick={handleClose} onScreenshotReady={(fn) => triggerScreenshot = fn} />
            {:else if chartType === 'rvol'}
                <RvolChart {pairKey} {slot} onDoubleClick={handleClose} onScreenshotReady={(fn) => triggerScreenshot = fn} />
            {:else if chartType === 'macd'}
                <MacdChart {pairKey} {slot} onDoubleClick={handleClose} onScreenshotReady={(fn) => triggerScreenshot = fn} />
            {:else if chartType === 'squeeze'}
                <SqueezeChart {pairKey} {slot} onDoubleClick={handleClose} onScreenshotReady={(fn) => triggerScreenshot = fn} />
            {:else if chartType === 'rsi'}
                <RsiChart {pairKey} {slot} onDoubleClick={handleClose} onScreenshotReady={(fn) => triggerScreenshot = fn} />
            {:else if chartType === 'adx'}
                <AdxChart {pairKey} {slot} onDoubleClick={handleClose} onScreenshotReady={(fn) => triggerScreenshot = fn} />
            {:else if chartType === 'bbwp'}
                <BbwpChart {pairKey} {slot} onDoubleClick={handleClose} onScreenshotReady={(fn) => triggerScreenshot = fn} />
            {:else if chartType === 'atr'}
                <AtrChart {pairKey} {slot} onDoubleClick={handleClose} onScreenshotReady={(fn) => triggerScreenshot = fn} />
            {:else if chartType === 'stochastic'}
                <StochasticChart {pairKey} {slot} onDoubleClick={handleClose} onScreenshotReady={(fn) => triggerScreenshot = fn} />
            {:else if chartType === 'chandemo'}
                <ChandeMoChart {pairKey} {slot} onDoubleClick={handleClose} onScreenshotReady={(fn) => triggerScreenshot = fn} />
            {:else if chartType === 'williams_r'}
                <WilliamsRChart {pairKey} {slot} onDoubleClick={handleClose} onScreenshotReady={(fn) => triggerScreenshot = fn} />
            {:else if chartType === 'cci'}
                <CciChart {pairKey} {slot} onDoubleClick={handleClose} onScreenshotReady={(fn) => triggerScreenshot = fn} />
            {:else if chartType === 'force_index'}
                <ForceIndexChart {pairKey} {slot} onDoubleClick={handleClose} onScreenshotReady={(fn) => triggerScreenshot = fn} />
            {:else if chartType === 'obv'}
                <ObvChart {pairKey} {slot} onDoubleClick={handleClose} onScreenshotReady={(fn) => triggerScreenshot = fn} />
            {:else if chartType === 'cmf'}
                <CmfChart {pairKey} {slot} onDoubleClick={handleClose} onScreenshotReady={(fn) => triggerScreenshot = fn} />
            {:else if chartType === 'mfi'}
                <MfiChart {pairKey} {slot} onDoubleClick={handleClose} onScreenshotReady={(fn) => triggerScreenshot = fn} />
            {:else if chartType === 'hv'}
                <HvChart {pairKey} {slot} onDoubleClick={handleClose} onScreenshotReady={(fn) => triggerScreenshot = fn} />
            {:else if chartType === 'aroon'}
                <AroonChart {pairKey} {slot} onDoubleClick={handleClose} onScreenshotReady={(fn) => triggerScreenshot = fn} />
            {:else if chartType === 'choppiness'}
                <ChoppinessChart {pairKey} {slot} onDoubleClick={handleClose} onScreenshotReady={(fn) => triggerScreenshot = fn} />
            {:else if chartType === 'linreg'}
                <LinRegSlopeChart {pairKey} {slot} onDoubleClick={handleClose} onScreenshotReady={(fn) => triggerScreenshot = fn} />
            {:else if chartType === 'zscore'}
                <ZScoreChart {pairKey} {slot} onDoubleClick={handleClose} onScreenshotReady={(fn) => triggerScreenshot = fn} />
            {:else if chartType === 'funding'}
                <FundingChart {pairKey} {slot} onDoubleClick={handleClose} onScreenshotReady={(fn) => triggerScreenshot = fn} />
            {:else if chartType === 'open_interest'}
                <OpenInterestChart {pairKey} {slot} onDoubleClick={handleClose} onScreenshotReady={(fn) => triggerScreenshot = fn} />
            {:else if chartType === 'oi_delta'}
                <OiDeltaChart {pairKey} {slot} onDoubleClick={handleClose} onScreenshotReady={(fn) => triggerScreenshot = fn} />
            {:else if chartType === 'order_flow_depth'}
                <OrderFlowDepthChart {pairKey} {slot} onDoubleClick={handleClose} onScreenshotReady={(fn) => triggerScreenshot = fn} />
            {/if}
        </div>
        <FullscreenToolbar onScreenshot={handleScreenshot} onClose={handleClose} />
    </div>
{/if}
