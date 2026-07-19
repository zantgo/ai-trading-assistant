<script lang="ts">
    import { flattenHistory } from '../lib/historyAdapter';
    import { iRaw, iSub } from '../lib/telemetry';
    import type { IndicatorMap } from '../types';
    import { onMount, onDestroy } from 'svelte';
    import { createChart, CrosshairMode, LineSeries, HistogramSeries } from 'lightweight-charts';
    import type { IChartApi, ISeriesApi, IPriceLine, Time } from 'lightweight-charts';
    import { useAppStore } from '../state.svelte';
    import { registerChart, unregisterChart } from '../chartRegistry.svelte';
    import { takeChartScreenshot } from '../lib/chartScreenshot';
    import ChartFullscreenOverlay from './ChartFullscreenOverlay.svelte';

    const app = useAppStore();
    let { pairKey, timeframe = 60, onDoubleClick, onScreenshotReady }: { pairKey: string; timeframe?: number; onDoubleClick?: () => void; onScreenshotReady?: (fn: () => void) => void } = $props();
    const pair = $derived(app.instancesMap[pairKey]);
    const tf = $derived(
        timeframe === 180 ? pair?.fastTerm :
        timeframe === 300 ? pair?.slowTerm :
        timeframe === 900 ? pair?.macroTerm :
        pair?.microTerm
    );

    let container: HTMLDivElement;
    let chart: IChartApi = $state(null!);
    let ro: ResizeObserver;
    let macdLineSeries: ISeriesApi<'Line'>;
    let macdSigSeries: ISeriesApi<'Line'>;
    let macdHistSeries: ISeriesApi<'Histogram'>;
    let zeroLine: IPriceLine | null = null;
    let isFullscreen = $state(false);

    function toggleFullscreen() {
        isFullscreen = !isFullscreen;
        if (chart && container) {
            requestAnimationFrame(() => chart.resize(container.clientWidth, container.clientHeight));
        }
    }
    function screenshotChart() { if (chart) takeChartScreenshot(chart, `macd-${pairKey}-${timeframe}s`); }

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

        macdLineSeries = chart.addSeries(LineSeries, { color: '#64ffda', lineWidth: 2, priceLineVisible: false });
        macdSigSeries = chart.addSeries(LineSeries, { color: '#ff9800', lineWidth: 2, priceLineVisible: false });
        macdHistSeries = chart.addSeries(HistogramSeries, { base: 0, priceLineVisible: false });

        // Zero-line reference
        zeroLine = macdHistSeries.createPriceLine({
            price: 0,
            color: '#4c525e',
            lineWidth: 1,
            lineStyle: 1,
            axisLabelVisible: false,
        });

        chart.priceScale('right').applyOptions({ alignLabels: true });
        chart.timeScale().applyOptions({ rightOffset: 12, barSpacing: 6 });

        registerChart(chart);

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
                const res = await fetch(`/api/history?symbol=${encodeURIComponent(pairKey)}&timeframe_secs=${timeframe}&limit=1000`);
                const data = await res.json();
                const indicatorHistory = flattenHistory(data.indicator_history);
                if (indicatorHistory && indicatorHistory.macd_line && indicatorHistory.macd_line.length > 0) {
                    const rawCombined = indicatorHistory.times.map((t: number, i: number) => ({
                        time: t as Time,
                        line: indicatorHistory.macd_line[i],
                        sig: indicatorHistory.macd_signal[i],
                        hist: indicatorHistory.macd_hist[i]
                    }));

                    const seenTimes = new Set<number>();
                    const cleanedCombined: { time: Time; line: string | null; sig: string | null; hist: string | null }[] = [];
                    for (const item of rawCombined) {
                        const tNum = item.time as number;
                        if (item && tNum && !seenTimes.has(tNum)) {
                            seenTimes.add(tNum);
                            cleanedCombined.push(item);
                        }
                    }
                    cleanedCombined.sort((a, b) => (a.time as number) - (b.time as number));

                    const lineData = cleanedCombined.map(x => ({
                        time: x.time,
                        value: x.line != null ? parseFloat(x.line) : 0
                    }));
                    const sigData = cleanedCombined.map(x => ({
                        time: x.time,
                        value: x.sig != null ? parseFloat(x.sig) : 0
                    }));
                    const histData = cleanedCombined.map(x => ({
                        time: x.time,
                        value: x.hist != null ? parseFloat(x.hist) : 0,
                        color: x.hist != null
                            ? (parseFloat(x.hist) >= 0 ? '#26a69a' : '#ef5350')
                            : '#131722'
                    }));

                    macdLineSeries.setData(lineData);
                    macdSigSeries.setData(sigData);
                    macdHistSeries.setData(histData);
                    chart.timeScale().fitContent();
                } else if (data.prices && data.prices.length > 0) {
                    const hasCandles = data.candles && data.candles.length > 0;
                    const source = hasCandles ? data.candles : data.prices;

                    const now = Math.floor(Date.now() / 1000);
                    const step = tf.barDurationSec || 60;
                    const baseTime = now - (data.prices.length * step);

                    const seenTimes = new Set<number>();
                    const placeholderLine: { time: Time; value: number }[] = [];
                    const placeholderHist: { time: Time; value: number; color: string }[] = [];
                    for (let idx = 0; idx < source.length; idx++) {
                        const item = source[idx];
                        const tVal = hasCandles ? Math.floor(item.time / 1000) : (baseTime + (idx * step));
                        if (!seenTimes.has(tVal)) {
                            seenTimes.add(tVal);
                            placeholderLine.push({ time: tVal as Time, value: 0 });
                            placeholderHist.push({ time: tVal as Time, value: 0, color: '#131722' });
                        }
                    }
                    placeholderLine.sort((a, b) => (a.time as number) - (b.time as number));
                    placeholderHist.sort((a, b) => (a.time as number) - (b.time as number));

                    macdLineSeries.setData(placeholderLine);
                    macdSigSeries.setData(placeholderLine);
                    macdHistSeries.setData(placeholderHist);
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
        if (positive && expanding) return '#26a69a';       // Light Green — building
        if (positive && !expanding) return '#00695c';       // Dark Green — warning
        if (!positive && expanding) return '#ef5350';       // Bright Red — building
        return '#b71c1c';                                    // Dark Red — warning
    }

    let prevMacdHist = 0;
    $effect(() => {
        if (!pair) return;
        const snap = tf.latestSnapshot;
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

<div class="chart-wrapper" class:fs-active={isFullscreen} ondblclick={toggleFullscreen} role="presentation">
    <div class="chart-container" bind:this={container}></div>
</div>

<ChartFullscreenOverlay open={isFullscreen} title="MACD — {pairKey} · {timeframe}s" chart={chart} onclose={toggleFullscreen} />

<style>
    .chart-container { width: 100%; height: 100%; }
    .chart-wrapper { width: 100%; height: 100%; }
    .chart-wrapper.fs-active {
        position: fixed; inset: 0; z-index: 990;
        background: #131722; padding: 44px 16px 16px 16px; box-sizing: border-box;
    }
</style>
