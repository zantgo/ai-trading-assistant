<script lang="ts">
    import { fetchChartHistoryOnce, dedupSortByTime } from '../lib/chartHistory';
    import { iRaw, iSub } from '../lib/telemetry';
    import type { IndicatorMap } from '../types';
    import { onMount, onDestroy } from 'svelte';
    import { createChart, CrosshairMode, LineSeries, HistogramSeries } from 'lightweight-charts';
    import type { IChartApi, ISeriesApi, IPriceLine, Time } from 'lightweight-charts';
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
    let macdLineSeries: ISeriesApi<'Line'>;
    let macdSigSeries: ISeriesApi<'Line'>;
    let macdHistSeries: ISeriesApi<'Histogram'>;
    let zeroLine: IPriceLine | null = null;

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

        macdLineSeries = chart.addSeries(LineSeries, { color: '#2962ff', lineWidth: 2, priceLineVisible: false });
        macdSigSeries = chart.addSeries(LineSeries, { color: '#ff9800', lineWidth: 2, priceLineVisible: false });
        macdHistSeries = chart.addSeries(HistogramSeries, { base: 0, priceLineVisible: false });

        zeroLine = macdHistSeries.createPriceLine({
            price: 0,
            color: '#4c525e',
            lineWidth: 1,
            lineStyle: 1,
            axisLabelVisible: false,
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
                link.download = `${pairKey}_${timeframe}s_macd.png`;
                link.href = dataUrl;
                link.click();
            });
        }

        (async () => {
            if (!pair) return;
            try {
                const data = await fetchChartHistoryOnce(pairKey, timeframe);
                if (!data || !data.indicatorHistory || !data.indicatorHistory.macd_line.length) return;
                const ih = data.indicatorHistory;
                const rawCombined = ih.times.map((t: number, i: number) => ({
                    time: t as Time,
                    line: ih.macd_line[i],
                    sig: ih.macd_signal[i],
                    hist: ih.macd_hist[i]
                }));

                const cleanedCombined = dedupSortByTime(rawCombined);

                const lineData = cleanedCombined
                    .filter((x: any) => x.line != null)
                    .map((x: any) => ({
                        time: x.time,
                        value: parseFloat(x.line!)
                    }));
                const sigData = cleanedCombined
                    .filter((x: any) => x.sig != null)
                    .map((x: any) => ({
                        time: x.time,
                        value: parseFloat(x.sig!)
                    }));
                const histData = cleanedCombined
                    .filter((x: any) => x.hist != null)
                    .map((x: any) => ({
                        time: x.time,
                        value: parseFloat(x.hist!),
                        color: parseFloat(x.hist!) >= 0 ? '#26a69a' : '#ef5350'
                    }));

                if (lineData.length > 0) {
                    macdLineSeries.setData(lineData);
                    macdSigSeries.setData(sigData);
                    macdHistSeries.setData(histData);
                    chart.timeScale().fitContent();
                }
            } catch (err) {
                console.error("Error bootstrapping MACD chart history:", err);
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

    function histogramColor(mHist: number, prevHist: number): string {
        const positive = mHist >= 0;
        const expanding = Math.abs(mHist) >= Math.abs(prevHist);
        if (positive && expanding) return '#26a69a';
        if (positive && !expanding) return '#00695c';
        if (!positive && expanding) return '#ef5350';
        return '#b71c1c';
    }

    let prevMacdHist = 0;
    $effect(() => {
        const pairVal = app.instancesMap[pairKey];
        if (!pairVal) return;
        const tfVal = slot === 'micro' ? pairVal.microTerm : slot === 'fast' ? pairVal.fastTerm : slot === 'slow' ? pairVal.slowTerm : pairVal.macroTerm;
        const snap = tfVal.latestSnapshot;
        if (!snap) return;
        const timeSec = snap.timestamp as number;
        const m = (snap.indicators ?? {}) as IndicatorMap;
        const mLine = iSub(m, 'macd', 'line');
        const mSig = iSub(m, 'macd', 'signal');
        const mHist = iRaw(m, 'macd');
        if (mLine != null && mSig != null && mHist != null) {
            macdLineSeries.update({ time: timeSec as Time, value: mLine });
            macdSigSeries.update({ time: timeSec as Time, value: mSig });
            const color = histogramColor(mHist, prevMacdHist);
            macdHistSeries.update({ time: timeSec as Time, value: mHist, color });
            prevMacdHist = mHist;
        }
    });
</script>

<div class="chart-container" bind:this={container}></div>

<style>
    .chart-container { width: 100%; height: 100%; }
</style>
