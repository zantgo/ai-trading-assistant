<script lang="ts">
    import { iRaw, iSub, adxRegime } from '../lib/telemetry';
    import type { IndicatorMap } from '../types';
    import { fetchChartHistoryOnce, dedupSortByTime } from '../lib/chartHistory';
    import { onMount, onDestroy } from 'svelte';
    import { createChart, CrosshairMode, LineSeries, LineStyle } from 'lightweight-charts';
    import type { IChartApi, ISeriesApi, IPriceLine, Time } from 'lightweight-charts';
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
    let adxSeries: ISeriesApi<'Line'>;
    let adxPlusSeries: ISeriesApi<'Line'>;
    let adxMinusSeries: ISeriesApi<'Line'>;
    let trendLine: IPriceLine | null = null;
    let exhaustionLine: IPriceLine | null = null;

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

        adxSeries = chart.addSeries(LineSeries, { color: '#f1c40f', lineWidth: 2, lineStyle: LineStyle.Solid, priceLineVisible: false });
        adxPlusSeries = chart.addSeries(LineSeries, { color: '#2ecc71', lineWidth: 1, lineStyle: LineStyle.Solid, priceLineVisible: false });
        adxMinusSeries = chart.addSeries(LineSeries, { color: '#e74c3c', lineWidth: 1, lineStyle: LineStyle.Solid, priceLineVisible: false });

        trendLine = adxSeries.createPriceLine({
            price: 20,
            color: '#4c525e',
            lineWidth: 1,
            lineStyle: LineStyle.Dashed,
            axisLabelVisible: true,
            title: 'TREND',
        });

        exhaustionLine = adxSeries.createPriceLine({
            price: 40,
            color: '#ff5252',
            lineWidth: 1,
            lineStyle: LineStyle.Dashed,
            axisLabelVisible: true,
            title: 'EXHAUST',
        });

        chart.priceScale('right').applyOptions({ alignLabels: true });
        chart.timeScale().applyOptions({ rightOffset: 12, barSpacing: 6 });

        registerChart(chart, container);

        if (onDoubleClick) chart.subscribeDblClick(onDoubleClick);

        if (onScreenshotReady) {
            onScreenshotReady(() => {
                if (!chart) return;
                const canvas = chart.takeScreenshot();
                const dataUrl = canvas.toDataURL('image/png');
                const link = document.createElement('a');
                link.download = `${pairKey}_${timeframe}s_adx.png`;
                link.href = dataUrl;
                link.click();
            });
        }

        (async () => {
            if (!pair) return;
            try {
                const data = await fetchChartHistoryOnce(pairKey, timeframe);
                if (!data || !data.indicatorHistory || !data.indicatorHistory.adx_14.length) return;
                const ih = data.indicatorHistory;
                const rawCombined = ih.times.map((t: number, i: number) => ({
                    time: t as Time,
                    adx: ih.adx_14[i],
                    plus: ih.adx_plus[i],
                    minus: ih.adx_minus[i]
                }));

                const cleanedCombined = dedupSortByTime(rawCombined);

                const adxData = cleanedCombined
                    .filter((x: any) => x.adx != null)
                    .map((x: any) => ({
                        time: x.time,
                        value: parseFloat(x.adx!)
                    }));
                const plusData = cleanedCombined
                    .filter((x: any) => x.plus != null)
                    .map((x: any) => ({
                        time: x.time,
                        value: parseFloat(x.plus!)
                    }));
                const minusData = cleanedCombined
                    .filter((x: any) => x.minus != null)
                    .map((x: any) => ({
                        time: x.time,
                        value: parseFloat(x.minus!)
                    }));

                if (adxData.length > 0) {
                    adxSeries.setData(adxData);
                    adxPlusSeries.setData(plusData);
                    adxMinusSeries.setData(minusData);
                    chart.timeScale().fitContent();
                }
            } catch (err) {
                console.error("Error bootstrapping ADX chart history:", err);
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

    function adxLineColor(val: number, slope: number, regime: string): string {
        if (val > 40) return '#ff5252';
        if (val < 20) return '#4c525e';
        if (slope > 0) return '#f1c40f';
        return '#f97316';
    }

    const adxCoalescer = makeChartCoalescer(app, pairKey, slot, (snap) => {
        const timeSec = snap.timestamp as number;
        const m = (snap.indicators ?? {}) as IndicatorMap;
        const adxVal = iSub(m, 'adx', 'adx') ?? iRaw(m, 'adx');
        if (adxVal != null) {
            const slope = iSub(m, 'adx', 'adx_slope') ?? 0;
            const regime = adxRegime(m);
            const plus = iSub(m, 'adx', 'plus_di');
            const minus = iSub(m, 'adx', 'minus_di');

            adxSeries.update({ time: timeSec as Time, value: adxVal });
            if (plus != null) adxPlusSeries.update({ time: timeSec as Time, value: plus });
            if (minus != null) adxMinusSeries.update({ time: timeSec as Time, value: minus });

            const color = adxLineColor(adxVal, slope, regime);
            adxSeries.applyOptions({ color });
        }
    });
    $effect(adxCoalescer.effect);
    onDestroy(adxCoalescer.destroy);
</script>

<div class="chart-container" bind:this={container}></div>

<style>
    .chart-container { width: 100%; height: 100%; }
</style>
