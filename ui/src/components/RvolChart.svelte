<script lang="ts">
    import { iRaw } from '../lib/telemetry';
    import type { IndicatorMap } from '../types';
    import { fetchChartHistoryOnce, dedupSortByTime } from '../lib/chartHistory';
    import { onMount, onDestroy } from 'svelte';
    import { createChart, CrosshairMode, HistogramSeries } from 'lightweight-charts';
    import type { IChartApi, ISeriesApi, Time } from 'lightweight-charts';
    import { useAppStore } from '../state.svelte';
    import { registerChart, unregisterChart } from '../chartRegistry.svelte';
    import { makeChartCoalescer } from '../lib/chartCoalesce';

    const app = useAppStore();
    let { pairKey, slot, onDoubleClick, onScreenshotReady }: { pairKey: string; slot: 'micro' | 'fast' | 'slow' | 'macro'; onDoubleClick?: () => void; onScreenshotReady?: (fn: () => void) => void } = $props();
    const pair = $derived(app.instancesMap[pairKey]);
    const tf = $derived(
        slot === 'micro' ? pair?.microTerm :
        slot === 'fast'  ? pair?.fastTerm :
        slot === 'slow'  ? pair?.slowTerm :
                          pair?.macroTerm
    );
    const timeframe = $derived(tf?.barDurationSec ?? 60);

    let container: HTMLDivElement;
    let chart: IChartApi;
    let ro: ResizeObserver;
    let rvolSeries: ISeriesApi<'Histogram'>;

    function rvolColor(rvol: number): string {
        if (rvol >= 3.0) return '#e040fb';
        if (rvol >= 1.5) return '#26c6da';
        if (rvol < 1.0) return 'rgba(143, 146, 157, 0.25)';
        return '#3b82f6';
    }

    onMount(() => {
        chart = createChart(container, {
            autoSize: true,
            layout: { background: { color: '#131722' }, textColor: '#8f929d', fontSize: 10 },
            grid: { vertLines: { color: '#1a1d26' }, horzLines: { color: '#1a1d26' } },
            crosshair: { mode: CrosshairMode.Normal, vertLine: { color: '#4c525e', width: 1, style: 3 }, horzLine: { color: '#4c525e', width: 1, style: 3 } },
            rightPriceScale: { borderColor: '#2a2e39', scaleMargins: { top: 0.15, bottom: 0.1 } },
            timeScale: {
                borderColor: '#2a2e39',
                visible: false,
                timeVisible: false,
                secondsVisible: false,
                tickMarkFormatter: (time: any, _tickMarkType: number, _locale: string) => {
                    const date = new Date(time * 1000);
                    const hours = String(date.getHours()).padStart(2, '0');
                    const minutes = String(date.getMinutes()).padStart(2, '0');
                    return `${hours}:${minutes}`;
                }
            },
            handleScale: true,
            handleScroll: true,
        });

        chart.priceScale('right').applyOptions({ alignLabels: true });
        chart.timeScale().applyOptions({ rightOffset: 12, barSpacing: 6 });

        rvolSeries = chart.addSeries(HistogramSeries, {
            color: '#3b82f6',
            base: 0,
            priceLineVisible: false
        });

        rvolSeries.createPriceLine({
            price: 1.0,
            color: '#4c525e',
            lineWidth: 1,
            lineStyle: 2,
            axisLabelVisible: true,
            title: 'CONSOLIDATION (1.0)',
        });

        rvolSeries.createPriceLine({
            price: 1.5,
            color: '#26c6da',
            lineWidth: 1,
            lineStyle: 2,
            axisLabelVisible: true,
            title: 'INSTITUTIONAL (1.5)',
        });

        registerChart(chart, container);

        if (onDoubleClick) chart.subscribeDblClick(onDoubleClick);

        if (onScreenshotReady) {
            onScreenshotReady(() => {
                if (!chart) return;
                const canvas = chart.takeScreenshot();
                const dataUrl = canvas.toDataURL('image/png');
                const link = document.createElement('a');
                link.download = `${pairKey}_${timeframe}s_rvol.png`;
                link.href = dataUrl;
                link.click();
            });
        }

        (async () => {
            if (!pair) return;
            try {
                const data = await fetchChartHistoryOnce(pairKey, timeframe);
                if (!data || !data.indicatorHistory || !data.indicatorHistory.rvol.length) return;
                const ih = data.indicatorHistory;
                const rawRvolData = ih.times.map((t: number, i: number) => {
                    const val = parseFloat(ih.rvol[i] ?? "0") || 0;
                    return {
                        time: t as Time,
                        value: val,
                        color: rvolColor(val),
                    };
                });

                const cleanedRvolData = dedupSortByTime(rawRvolData);

                if (cleanedRvolData.length > 0) {
                    rvolSeries.setData(cleanedRvolData);
                    chart.timeScale().fitContent();
                }
            } catch (err) {
                console.error("Error bootstrapping RVOL chart history:", err);
            }
        })();

        ro = new ResizeObserver(() => {
            const w = container.clientWidth;
            const h = container.clientHeight;
            if (chart && w > 0 && h > 0) {
                chart.resize(w, h);
            }
        });
        if (container?.parentElement) {
            ro.observe(container.parentElement);
        }
    });

    onDestroy(() => {
        ro?.disconnect();
        if (chart) {
            unregisterChart(chart);
            chart.remove();
        }
    });

    const rvolCoalescer = makeChartCoalescer(app, pairKey, slot, (snap) => {
        const timeSec = snap.timestamp as number;
        const val = iRaw((snap.indicators ?? {}) as IndicatorMap, 'rvol');
        if (val != null) {
            rvolSeries.update({
                time: timeSec as Time,
                value: val,
                color: rvolColor(val)
            });
        }
    });
    $effect(rvolCoalescer.effect);
    onDestroy(rvolCoalescer.destroy);
</script>

<div class="chart-container" bind:this={container}></div>

<style>
    .chart-container { width: 100%; height: 100%; }
</style>
