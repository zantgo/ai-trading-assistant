<script lang="ts">
    import { iRaw } from '../lib/telemetry';
    import type { IndicatorMap } from '../types';
    import { fetchChartHistoryOnce, dedupSortByTime } from '../lib/chartHistory';
    import { onMount, onDestroy } from 'svelte';
    import { createChart, CrosshairMode, HistogramSeries } from 'lightweight-charts';
    import type { IChartApi, ISeriesApi, Time } from 'lightweight-charts';
    import { useAppStore } from '../state.svelte';
    import { registerChart, unregisterChart } from '../chartRegistry.svelte';

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
    let bbwpSeries: ISeriesApi<'Histogram'>;

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

        bbwpSeries = chart.addSeries(HistogramSeries, {
            color: '#00d4aa',
            base: 0,
            priceLineVisible: false
        });

        bbwpSeries.createPriceLine({
            price: 10,
            color: '#4488ff',
            lineWidth: 1,
            lineStyle: 2,
            axisLabelVisible: true,
            title: 'COMPRESSION',
        });

        bbwpSeries.createPriceLine({
            price: 90,
            color: '#ff4444',
            lineWidth: 1,
            lineStyle: 2,
            axisLabelVisible: true,
            title: 'EXHAUSTION',
        });

        registerChart(chart);

        if (onDoubleClick) chart.subscribeDblClick(onDoubleClick);

        if (onScreenshotReady) {
            onScreenshotReady(() => {
                if (!chart) return;
                const canvas = chart.takeScreenshot();
                const dataUrl = canvas.toDataURL('image/png');
                const link = document.createElement('a');
                link.download = `${pairKey}_${timeframe}s_bbwp.png`;
                link.href = dataUrl;
                link.click();
            });
        }

        (async () => {
            if (!pair) return;
            try {
                const data = await fetchChartHistoryOnce(pairKey, timeframe);
                if (!data || !data.indicatorHistory || !data.indicatorHistory.bbwp.length) return;
                const ih = data.indicatorHistory;
                const rawBbwpData = ih.times.map((t: number, i: number) => {
                    const val = parseFloat(ih.bbwp[i] ?? "0") || 0;
                    return {
                        time: t as Time,
                        value: val,
                        color: val < 10 ? '#4488ff' : val > 90 ? '#ff4444' : '#00d4aa',
                    };
                });

                const cleanedBbwpData = dedupSortByTime(rawBbwpData);

                if (cleanedBbwpData.length > 0) {
                    bbwpSeries.setData(cleanedBbwpData);
                    chart.timeScale().fitContent();
                }
            } catch (err) {
                console.error("Error bootstrapping BBWP chart history:", err);
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

    $effect(() => {
        const pairVal = app.instancesMap[pairKey];
        if (!pairVal) return;
        const tfVal = slot === 'micro' ? pairVal.microTerm : slot === 'fast' ? pairVal.fastTerm : slot === 'slow' ? pairVal.slowTerm : pairVal.macroTerm;
        const snap = tfVal.latestSnapshot;
        if (!snap) return;
        const timeSec = snap.timestamp as number;
        const val = iRaw((snap.indicators ?? {}) as IndicatorMap, 'bbwp');
        if (val != null) {
            bbwpSeries.update({
                time: timeSec as Time,
                value: val,
                color: val < 10 ? '#4488ff' : val > 90 ? '#ff4444' : '#00d4aa'
            });
        }
    });
</script>

<div class="chart-container" bind:this={container}></div>

<style>
    .chart-container { width: 100%; height: 100%; }
</style>
