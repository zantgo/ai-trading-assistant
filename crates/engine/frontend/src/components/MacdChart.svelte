<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { createChart, CrosshairMode, LineSeries, HistogramSeries } from 'lightweight-charts';
    import type { IChartApi, ISeriesApi, Time } from 'lightweight-charts';
    import { getState } from '../state.svelte';
    import { registerChart, unregisterChart } from '../chartRegistry.svelte';

    const app = getState();
    let { pairKey } = $props();
    const pair = $derived(app.pairsMap[pairKey]);

    let container: HTMLDivElement;
    let chart: IChartApi;
    let macdLineSeries: ISeriesApi<'Line'>;
    let macdSigSeries: ISeriesApi<'Line'>;
    let macdHistSeries: ISeriesApi<'Histogram'>;

    onMount(() => {
        chart = createChart(container, {
            autoSize: true,
            layout: { background: { color: '#131722' }, textColor: '#8f929d', fontSize: 10 },
            grid: { vertLines: { color: '#1a1d26' }, horzLines: { color: '#1a1d26' } },
            crosshair: { mode: CrosshairMode.Normal, vertLine: { color: '#4c525e', width: 1, style: 3 }, horzLine: { color: '#4c525e', width: 1, style: 3 } },
            rightPriceScale: { borderColor: '#2a2e39', scaleMargins: { top: 0.15, bottom: 0.1 } },
            timeScale: { borderColor: '#2a2e39', visible: false, timeVisible: true, secondsVisible: true },
            handleScale: true,
            handleScroll: true,
        });

        macdLineSeries = chart.addSeries(LineSeries, { color: '#2962ff', lineWidth: 2, priceLineVisible: false });
        macdSigSeries = chart.addSeries(LineSeries, { color: '#ff9800', lineWidth: 2, priceLineVisible: false });
        macdHistSeries = chart.addSeries(HistogramSeries, { base: 0, priceLineVisible: false });

        chart.priceScale('right').applyOptions({ alignLabels: true });
        chart.timeScale().applyOptions({ rightOffset: 12, barSpacing: 6 });

        registerChart(chart);

        (async () => {
            if (!pair) return;
            try {
                const res = await fetch(`/api/history?symbol=${encodeURIComponent(pairKey)}`);
                const data = await res.json();
                if (data.prices && data.prices.length > 0) {
                    const now = Math.floor(Date.now() / 1000);
                    const step = pair.barDurationSec || 60;
                    const baseTime = now - (data.prices.length * step);

                    const placeholderLine = data.prices.map((_: string, idx: number) => ({
                        time: (baseTime + (idx * step)) as Time,
                        value: 0
                    }));
                    const placeholderHist = data.prices.map((_: string, idx: number) => ({
                        time: (baseTime + (idx * step)) as Time,
                        value: 0,
                        color: '#131722'
                    }));

                    macdLineSeries.setData(placeholderLine);
                    macdSigSeries.setData(placeholderLine);
                    macdHistSeries.setData(placeholderHist);
                    chart.timeScale().fitContent();
                }
            } catch (err) {
                console.error("Error bootstrapping MACD chart history:", err);
            }
        })();

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
        if (!pair) return;
        const snap = pair.latestSnapshot;
        if (!snap) return;
        const timeSec = snap.timestamp as number;
        if (snap.macd_line != null) {
            const mLine = parseFloat(String(snap.macd_line));
            const mSig = parseFloat(String(snap.macd_signal));
            const mHist = parseFloat(String(snap.macd_hist));

            macdLineSeries.update({ time: timeSec as Time, value: mLine });
            macdSigSeries.update({ time: timeSec as Time, value: mSig });

            let histColor = mHist >= 0
                ? (mHist >= pair.lastMacdHist ? '#26a69a' : '#b2dfdb')
                : (mHist < pair.lastMacdHist ? '#ef5350' : '#ffcdd2');

            macdHistSeries.update({ time: timeSec as Time, value: mHist, color: histColor });
            pair.lastMacdHist = mHist;
        }
    });
</script>

<div class="chart-container" bind:this={container}></div>

<style>
    .chart-container { width: 100%; height: 100%; }
</style>
