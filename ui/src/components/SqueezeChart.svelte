<script lang="ts">
    import { iRaw, isSqueezeOn, squeezeDirection } from '../lib/telemetry';
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
    let squeezeMomSeries: ISeriesApi<'Histogram'>;
    let squeezeDotSeries: ISeriesApi<'Histogram'>;

    onMount(() => {
        chart = createChart(container, {
            autoSize: true,
            layout: { background: { color: '#131722' }, textColor: '#8f929d', fontSize: 10 },
            grid: { vertLines: { color: '#1a1d26' }, horzLines: { color: '#1a1d26' } },
            crosshair: { mode: CrosshairMode.Normal, vertLine: { color: '#4c525e', width: 1, style: 3 }, horzLine: { color: '#4c525e', width: 1, style: 3 } },
            rightPriceScale: { borderColor: '#2a2e39', scaleMargins: { top: 0.15, bottom: 0.1 } },
            timeScale: {
                borderColor: '#2a2e39',
                visible: true,
                timeVisible: true,
                secondsVisible: true,
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

        squeezeMomSeries = chart.addSeries(HistogramSeries, { base: 0, priceLineVisible: false });
        squeezeDotSeries = chart.addSeries(HistogramSeries, {
            base: 0,
            priceLineVisible: false,
            priceScaleId: 'squeeze-overlay',
        });

        chart.priceScale('right').applyOptions({ alignLabels: true });
        chart.priceScale('squeeze-overlay').applyOptions({
            visible: false,
            scaleMargins: {
                top: 0.46,
                bottom: 0.46
            }
        });
        chart.timeScale().applyOptions({ rightOffset: 12, barSpacing: 6 });

        registerChart(chart, container);

        if (onDoubleClick) chart.subscribeDblClick(onDoubleClick);

        if (onScreenshotReady) {
            onScreenshotReady(() => {
                if (!chart) return;
                const canvas = chart.takeScreenshot();
                const dataUrl = canvas.toDataURL('image/png');
                const link = document.createElement('a');
                link.download = `${pairKey}_${timeframe}s_squeeze.png`;
                link.href = dataUrl;
                link.click();
            });
        }

        (async () => {
            if (!pair) return;
            try {
                const data = await fetchChartHistoryOnce(pairKey, timeframe);
                if (!data || !data.indicatorHistory || !data.indicatorHistory.squeeze_momentum.length) return;
                const ih = data.indicatorHistory;
                const rawCombined = ih.times.map((t: number, i: number) => ({
                    time: t as Time,
                    mom: ih.squeeze_momentum[i],
                    on: ih.squeeze_on[i]
                }));

                const cleanedCombined = dedupSortByTime(rawCombined);

                const momData = cleanedCombined
                    .filter((x: any) => x.mom != null)
                    .map((x: any) => {
                        const val = parseFloat(x.mom!);
                        return {
                            time: x.time,
                            value: val,
                            color: val >= 0 ? '#26a69a' : '#ef5350'
                        };
                    });
                const dotData = cleanedCombined.map((x: any) => ({
                    time: x.time,
                    value: 0.1,
                    color: x.on ? '#ef5350' : '#4caf50'
                }));

                if (momData.length > 0) {
                    squeezeMomSeries.setData(momData);
                    squeezeDotSeries.setData(dotData);
                    chart.timeScale().fitContent();
                }
            } catch (err) {
                console.error("Error bootstrapping squeeze chart history:", err);
            }
        })();

        ro = new ResizeObserver(() => {
            const w = container.clientWidth, h = container.clientHeight; if (chart && w > 0 && h > 0) chart.resize(w, h);
        });
        if (container?.parentElement) ro.observe(container.parentElement);
    });

    onDestroy(() => {
        ro?.disconnect();
        if (chart) {
            unregisterChart(chart);
            chart.remove();
        }
    });

    function momentumColor(val: number, direction: string): string {
        switch (direction) {
            case 'BullishAcceleration': return '#26a69a';
            case 'BullishDeceleration': return '#00695c';
            case 'BearishAcceleration': return '#b71c1c';
            case 'BearishDeceleration': return '#ff1744';
            default: return val >= 0 ? '#4caf50' : '#ef5350';
        }
    }

    const squeezeCoalescer = makeChartCoalescer(app, pairKey, slot, (snap) => {
        const timeSec = snap.timestamp as number;
        const m = (snap.indicators ?? {}) as IndicatorMap;
        const momVal = iRaw(m, 'squeeze');
        if (momVal != null) {
            const direction = squeezeDirection(m);
            const momColor = momentumColor(momVal, direction);
            squeezeMomSeries.update({ time: timeSec as Time, value: momVal, color: momColor });

            const dotColor = isSqueezeOn(m) ? '#ef5350' : '#4caf50';
            squeezeDotSeries.update({ time: timeSec as Time, value: 0.1, color: dotColor });
        }
    });
    $effect(squeezeCoalescer.effect);
    onDestroy(squeezeCoalescer.destroy);
</script>

<div class="chart-container" bind:this={container}></div>

<style>
    .chart-container { width: 100%; height: 100%; }
</style>
