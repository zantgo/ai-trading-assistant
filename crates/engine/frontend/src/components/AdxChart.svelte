<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { createChart, CrosshairMode, LineSeries, LineStyle } from 'lightweight-charts';
    import type { IChartApi, ISeriesApi, Time } from 'lightweight-charts';
    import { getState } from '../state.svelte';
    import { registerChart, unregisterChart } from '../chartRegistry.svelte';

    const state = getState();
    let container: HTMLDivElement;
    let chart: IChartApi;
    let adxSeries: ISeriesApi<'Line'>;
    let adxPlusSeries: ISeriesApi<'Line'>;
    let adxMinusSeries: ISeriesApi<'Line'>;

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

        adxSeries = chart.addSeries(LineSeries, { color: '#f1c40f', lineWidth: 2, lineStyle: LineStyle.Solid, priceLineVisible: false });
        adxPlusSeries = chart.addSeries(LineSeries, { color: '#2ecc71', lineWidth: 1, lineStyle: LineStyle.Solid, priceLineVisible: false });
        adxMinusSeries = chart.addSeries(LineSeries, { color: '#e74c3c', lineWidth: 1, lineStyle: LineStyle.Solid, priceLineVisible: false });

        adxSeries.createPriceLine({
            price: 20,
            color: '#4c525e',
            lineWidth: 1,
            lineStyle: LineStyle.Dotted,
            axisLabelVisible: true,
            title: 'KEY LEVEL'
        });

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
        const snap = state.latestSnapshot;
        if (!snap) return;
        const timeSec = snap.timestamp as number;
        if (snap.adx_14 != null) {
            adxSeries.update({ time: timeSec as Time, value: parseFloat(String(snap.adx_14)) });
            if (snap.adx_plus) adxPlusSeries.update({ time: timeSec as Time, value: parseFloat(String(snap.adx_plus)) });
            if (snap.adx_minus) adxMinusSeries.update({ time: timeSec as Time, value: parseFloat(String(snap.adx_minus)) });
        }
    });
</script>

<div class="chart-container" bind:this={container}></div>

<style>
    .chart-container { width: 100%; height: 100%; }
</style>
