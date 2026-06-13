<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { createChart, CrosshairMode, CandlestickSeries, LineSeries, LineStyle } from 'lightweight-charts';
    import type { IChartApi, ISeriesApi, Time } from 'lightweight-charts';
    import { getState } from '../state.svelte';
    import { registerChart, unregisterChart } from '../chartRegistry.svelte';

    const app = getState();
    let { pairKey } = $props();
    const pair = $derived(app.pairsMap[pairKey]);

    let container: HTMLDivElement;
    let chart: IChartApi;
    let candleSeries: ISeriesApi<'Candlestick'>;
    let ema10Series: ISeriesApi<'Line'>;
    let ema50Series: ISeriesApi<'Line'>;
    let ema100Series: ISeriesApi<'Line'>;
    let ema200Series: ISeriesApi<'Line'>;
    let bbUpperSeries: ISeriesApi<'Line'>;
    let bbMiddleSeries: ISeriesApi<'Line'>;
    let bbLowerSeries: ISeriesApi<'Line'>;
    let vwapSeries: ISeriesApi<'Line'>;

    onMount(() => {
        chart = createChart(container, {
            autoSize: true,
            layout: { background: { color: '#131722' }, textColor: '#8f929d', fontSize: 10 },
            grid: { vertLines: { color: '#1a1d26' }, horzLines: { color: '#1a1d26' } },
            crosshair: { mode: CrosshairMode.Normal, vertLine: { color: '#4c525e', width: 1, style: 3 }, horzLine: { color: '#4c525e', width: 1, style: 3 } },
            rightPriceScale: { borderColor: '#2a2e39', scaleMargins: { top: 0.15, bottom: 0.1 } },
            timeScale: { borderColor: '#2a2e39', visible: true, timeVisible: true, secondsVisible: true },
            handleScale: true,
            handleScroll: true,
        });

        candleSeries = chart.addSeries(CandlestickSeries, {
            upColor: '#26a69a', downColor: '#ef5350', borderVisible: false,
            wickUpColor: '#26a69a', wickDownColor: '#ef5350'
        });

        ema10Series = chart.addSeries(LineSeries, { color: '#2962ff', lineWidth: 1.0, lineStyle: LineStyle.Dashed, priceLineVisible: false, crosshairMarkerVisible: false });
        ema50Series = chart.addSeries(LineSeries, { color: '#ff9800', lineWidth: 1.0, lineStyle: LineStyle.Dashed, priceLineVisible: false, crosshairMarkerVisible: false });
        ema100Series = chart.addSeries(LineSeries, { color: '#e91e63', lineWidth: 1.0, lineStyle: LineStyle.Dashed, priceLineVisible: false, crosshairMarkerVisible: false });
        ema200Series = chart.addSeries(LineSeries, { color: '#9c27b0', lineWidth: 1.0, lineStyle: LineStyle.Dashed, priceLineVisible: false, crosshairMarkerVisible: false });
        bbUpperSeries = chart.addSeries(LineSeries, { color: '#00e5ff', lineWidth: 1.0, lineStyle: LineStyle.Solid, priceLineVisible: false, crosshairMarkerVisible: false });
        bbMiddleSeries = chart.addSeries(LineSeries, { color: '#00e5ff', lineWidth: 1.0, lineStyle: LineStyle.Solid, priceLineVisible: false, crosshairMarkerVisible: false });
        bbLowerSeries = chart.addSeries(LineSeries, { color: '#00e5ff', lineWidth: 1.0, lineStyle: LineStyle.Solid, priceLineVisible: false, crosshairMarkerVisible: false });
        vwapSeries = chart.addSeries(LineSeries, { color: '#ffb300', lineWidth: 1, lineStyle: LineStyle.Solid, priceLineVisible: false, crosshairMarkerVisible: false });

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

                    const hasCandles = data.candles && data.candles.length > 0;

                    const historicalCandles = data.prices.map((priceStr: string, idx: number) => {
                        if (hasCandles && data.candles[idx]) {
                            const c = data.candles[idx];
                            return {
                                time: (c.time / 1000) as Time,
                                open: parseFloat(c.open),
                                high: parseFloat(c.high),
                                low: parseFloat(c.low),
                                close: parseFloat(c.close)
                            };
                        }
                        const val = parseFloat(priceStr);
                        return {
                            time: (baseTime + (idx * step)) as Time,
                            open: val,
                            high: val,
                            low: val,
                            close: val
                        };
                    });

                    candleSeries.setData(historicalCandles);
                    chart.timeScale().fitContent();
                }
            } catch (err) {
                console.error("Error bootstrapping price chart history:", err);
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
        if (!ema10Series || !ema50Series || !ema100Series || !ema200Series || !pair) return;
        ema10Series.applyOptions({ visible: pair.showEmas });
        ema50Series.applyOptions({ visible: pair.showEmas });
        ema100Series.applyOptions({ visible: pair.showEmas });
        ema200Series.applyOptions({ visible: pair.showEmas });
    });

    $effect(() => {
        if (!bbUpperSeries || !bbMiddleSeries || !bbLowerSeries || !pair) return;
        bbUpperSeries.applyOptions({ visible: pair.showBb });
        bbMiddleSeries.applyOptions({ visible: pair.showBb });
        bbLowerSeries.applyOptions({ visible: pair.showBb });
    });

    $effect(() => {
        if (!vwapSeries || !pair) return;
        vwapSeries.applyOptions({ visible: pair.showVwap });
    });

    $effect(() => {
        if (!pair) return;
        const snap = pair.latestSnapshot;
        if (!snap) return;
        const timeSec = snap.timestamp as number;

        if (snap.open != null && snap.high != null && snap.low != null && snap.close != null) {
            candleSeries.update({
                time: timeSec as Time,
                open: parseFloat(String(snap.open)),
                high: parseFloat(String(snap.high)),
                low: parseFloat(String(snap.low)),
                close: parseFloat(String(snap.close))
            });
        }

        if (snap.ema_fast) ema10Series.update({ time: timeSec as Time, value: parseFloat(String(snap.ema_fast)) });
        if (snap.ema_medium) ema50Series.update({ time: timeSec as Time, value: parseFloat(String(snap.ema_medium)) });
        if (snap.ema_slow) ema100Series.update({ time: timeSec as Time, value: parseFloat(String(snap.ema_slow)) });
        if (snap.ema_long) ema200Series.update({ time: timeSec as Time, value: parseFloat(String(snap.ema_long)) });
        if (snap.bb_upper) bbUpperSeries.update({ time: timeSec as Time, value: parseFloat(String(snap.bb_upper)) });
        if (snap.bb_middle) bbMiddleSeries.update({ time: timeSec as Time, value: parseFloat(String(snap.bb_middle)) });
        if (snap.bb_lower) bbLowerSeries.update({ time: timeSec as Time, value: parseFloat(String(snap.bb_lower)) });
        if (snap.vwap) vwapSeries.update({ time: timeSec as Time, value: parseFloat(String(snap.vwap)) });
    });
</script>

<div class="chart-container" bind:this={container}></div>

<style>
    .chart-container { width: 100%; height: 100%; }
</style>
