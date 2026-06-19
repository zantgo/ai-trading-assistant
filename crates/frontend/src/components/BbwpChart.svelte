<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { createChart, type IChartApi, type ISeriesApi, type HistogramData, ColorType, CrosshairMode, HistogramSeries, LineSeries } from 'lightweight-charts';
    import { registerChart, unregisterChart } from '../chartRegistry.svelte';

    let { pairKey, timeframe = 60, containerClass = '' }: {
        pairKey: string;
        timeframe?: number;
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

        if (chart) registerChart(chart);

        (async () => {
            try {
                const res = await fetch(`/api/history?symbol=${encodeURIComponent(pairKey)}&timeframe_secs=${timeframe}`);
                const data = await res.json();
                const ih = data.indicator_history;
                if (ih && ih.bbwp && ih.bbwp.length > 0 && bbwpSeries) {
                    const bbwpData: HistogramData[] = ih.times.map((t: number, i: number) => ({
                        time: t as any,
                        value: parseFloat(ih.bbwp[i]) || 0,
                        color: (parseFloat(ih.bbwp[i]) || 50) < 10 ? '#4488ff' : (parseFloat(ih.bbwp[i]) || 50) > 90 ? '#ff4444' : '#00d4aa',
                    }));
                    bbwpSeries.setData(bbwpData);
                    chart?.timeScale().fitContent();
                }
            } catch (err) {
                console.error("Error bootstrapping BBWP chart history:", err);
            }
        })();
    });

    onDestroy(() => {
        if (chart) {
            unregisterChart(chart);
            chart.remove();
        }
    });
</script>

<div class="bbwp-chart-container {containerClass}">
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
