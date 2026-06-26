<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { createChart, CrosshairMode, LineSeries, HistogramSeries } from 'lightweight-charts';
    import type { IChartApi, ISeriesApi, IPriceLine, Time } from 'lightweight-charts';
    import { useAppStore } from '../state.svelte';
    import { registerChart, unregisterChart } from '../chartRegistry.svelte';

    const app = useAppStore();
    let { pairKey, timeframe = 60, onDoubleClick }: { pairKey: string; timeframe?: number; onDoubleClick?: () => void } = $props();
    const pair = $derived(app.instancesMap[pairKey]);
    const tf = $derived(
        timeframe === 300 ? pair?.smallTerm :
        timeframe === 900 ? pair?.mediumTerm :
        timeframe === 3600 ? pair?.largeTerm :
        pair?.microTerm
    );

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

        (async () => {
            if (!pair) return;
            try {
                const res = await fetch(`/api/history?symbol=${encodeURIComponent(pairKey)}&timeframe_secs=${timeframe}`);
                const data = await res.json();
                const indicatorHistory = data.indicator_history;
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

    $effect(() => {
        if (!pair) return;
        const snap = tf.latestSnapshot;
        if (!snap) return;
        const timeSec = snap.timestamp as number;
        if (snap.macd_line != null) {
            const mLine = parseFloat(String(snap.macd_line));
            const mSig = parseFloat(String(snap.macd_signal));
            const mHist = parseFloat(String(snap.macd_hist));

            macdLineSeries.update({ time: timeSec as Time, value: mLine });
            macdSigSeries.update({ time: timeSec as Time, value: mSig });

            const color = histogramColor(mHist, tf.lastMacdHist);

            macdHistSeries.update({ time: timeSec as Time, value: mHist, color });
            tf.lastMacdHist = mHist;
        }

        // Update MACD momentum state from snapshot
        if (snap.macd_histogram_peak != null) {
            tf.macdHistPeak = parseFloat(String(snap.macd_histogram_peak));
        }
        if (snap.macd_crossover_detected != null) {
            tf.macdCrossoverDetected = !!snap.macd_crossover_detected;
        }
        if (snap.macd_crossover_direction != null) {
            tf.macdCrossoverDirection = String(snap.macd_crossover_direction) as 'BULLISH' | 'BEARISH' | 'NONE';
        }
        if (snap.macd_trend_state === 'decelerating') {
            tf.macdContractionTriggered = true;
        } else {
            tf.macdContractionTriggered = false;
        }
    });
</script>

<div class="chart-container" bind:this={container}></div>

<style>
    .chart-container { width: 100%; height: 100%; }
</style>
