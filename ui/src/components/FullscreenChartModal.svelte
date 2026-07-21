<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import styles from './LiveTerminal.module.css';
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
            {/if}
        </div>
    </div>
{/if}