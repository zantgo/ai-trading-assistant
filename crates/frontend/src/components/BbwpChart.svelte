<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { createChart, type IChartApi, type ISeriesApi, type HistogramData, ColorType, CrosshairMode, HistogramSeries, LineSeries } from 'lightweight-charts';
    import { registerChart, unregisterChart } from '../chartRegistry.svelte';

    let { historyPrices = [], currentBbwp = 0, containerClass = '' }: {
        historyPrices: number[];
        currentBbwp: number;
        containerClass?: string;
    } = $props();

    let chartContainer: HTMLDivElement | null = $state(null);
    let chart: IChartApi | null = $state(null);
    let bbwpSeries: ISeriesApi<'Histogram'> | null = $state(null);
    let compLine: ISeriesApi<'Line'> | null = $state(null);
    let exhaustLine: ISeriesApi<'Line'> | null = $state(null);

    onMount(() => {
        if (!chartContainer) return;

        chart = createChart(chartContainer, {
            height: 120,
            layout: {
                background: { type: ColorType.Solid, color: 'transparent' },
                textColor: '#8b949e',
            },
            grid: {
                vertLines: { color: 'rgba(255,255,255,0.06)' },
                horzLines: { color: 'rgba(255,255,255,0.06)' },
            },
            crosshair: { mode: CrosshairMode.Normal },
            rightPriceScale: {
                scaleMargins: { top: 0.05, bottom: 0.05 },
                borderColor: 'rgba(255,255,255,0.1)',
            },
            timeScale: {
                borderColor: 'rgba(255,255,255,0.1)',
                timeVisible: false,
                secondsVisible: false,
            },
        });

        bbwpSeries = chart.addSeries(HistogramSeries, {
            color: '#00d4aa',
            base: 0,
        });

        compLine = chart.addSeries(LineSeries, {
            color: '#4488ff',
            lineWidth: 1,
            lineStyle: 2,
            priceLineVisible: false,
            lastValueVisible: false,
        });

        exhaustLine = chart.addSeries(LineSeries, {
            color: '#ff4444',
            lineWidth: 1,
            lineStyle: 2,
            priceLineVisible: false,
            lastValueVisible: false,
        });

        if (compLine) {
            compLine.setData([
                { time: (Date.now() / 1000) - 3600 as any, value: 10 },
                { time: (Date.now() / 1000) as any, value: 10 },
            ]);
        }
        if (exhaustLine) {
            exhaustLine.setData([
                { time: (Date.now() / 1000) - 3600 as any, value: 90 },
                { time: (Date.now() / 1000) as any, value: 90 },
            ]);
        }

        if (chart) {
            chart.timeScale().fitContent();
            registerChart(chart);
        }
    });

    onDestroy(() => {
        if (chart) {
            unregisterChart(chart);
            chart.remove();
        }
    });

    $effect(() => {
        if (!bbwpSeries || historyPrices.length === 0) return;
        const timeNow = Date.now() / 1000;
        const interval = 60;
        const data: HistogramData[] = historyPrices.map((val, i) => ({
            time: (timeNow - (historyPrices.length - i) * interval) as any,
            value: val,
            color: val < 10 ? '#4488ff' : val > 90 ? '#ff4444' : '#00d4aa',
        }));
        bbwpSeries.setData(data);
    });

    $effect(() => {
        if (chart && currentBbwp > 0) {
            // Center view around current time
        }
    });
</script>

<div class="bbwp-chart-container {containerClass}">
    <span class="sub-title">BBWP (Volatility Percentile)</span>
    <div bind:this={chartContainer} class="bbwp-chart"></div>
</div>

<style>
    .bbwp-chart-container {
        display: flex;
        flex-direction: column;
        gap: 4px;
    }
    .bbwp-chart {
        width: 100%;
        height: 120px;
    }
</style>
