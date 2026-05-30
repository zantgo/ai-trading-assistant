<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { createChart, CrosshairMode, HistogramSeries } from 'lightweight-charts';
    import type { IChartApi, ISeriesApi, Time } from 'lightweight-charts';
    import { getState } from '../state.svelte';
    import { registerChart, unregisterChart } from '../chartRegistry.svelte';

    const app = getState();
    let container: HTMLDivElement;
    let chart: IChartApi;
    let volumeSeries: ISeriesApi<'Histogram'>;

    onMount(() => {
        chart = createChart(container, {
            autoSize: true,
            layout: { background: { color: '#131722' }, textColor: '#8f929d', fontSize: 10 },
            grid: { vertLines: { color: '#1a1d26' }, horzLines: { color: '#1a1d26' } },
            crosshair: { mode: CrosshairMode.Normal, vertLine: { color: '#4c525e', width: 1, style: 3 }, horzLine: { color: '#4c525e', width: 1, style: 3 } },
            rightPriceScale: { borderColor: '#2a2e39', scaleMargins: { top: 0.15, bottom: 0.1 } },
            timeScale: { borderColor: '#2a2e39', visible: false, timeVisible: true, secondsVisible: true },
            handleScale: false,
            handleScroll: false,
        });

        volumeSeries = chart.addSeries(HistogramSeries, { base: 0, priceLineVisible: false });

        chart.priceScale('right').applyOptions({ alignLabels: true });
        chart.timeScale().applyOptions({ rightOffset: 12, barSpacing: 6 });

        registerChart(chart);

        const ro = new ResizeObserver(() => {
            if (container && chart) chart.resize(container.clientWidth, container.clientHeight);
        });
        if (container?.parentElement) ro.observe(container.parentElement);

        return () => ro.disconnect();
    });

    onDestroy(() => {
        if (chart) {
            unregisterChart(chart);
            chart.remove();
        }
    });

    $effect(() => {
        const snap = app.latestSnapshot;
        if (!snap) return;
        const timeSec = snap.timestamp as number;
        if (snap.open != null && snap.close != null) {
            let volColor = parseFloat(String(snap.close)) >= parseFloat(String(snap.open)) ? '#26a69a' : '#ef5350';
            volumeSeries.update({ time: timeSec as Time, value: parseFloat(String(snap.volume)), color: volColor });
        }
    });
</script>

<div class="chart-container" bind:this={container}></div>

<style>
    .chart-container { width: 100%; height: 100%; }
</style>
