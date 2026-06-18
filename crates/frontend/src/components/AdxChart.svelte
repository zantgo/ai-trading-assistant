<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { createChart, CrosshairMode, LineSeries, LineStyle } from 'lightweight-charts';
    import type { IChartApi, ISeriesApi, IPriceLine, Time } from 'lightweight-charts';
    import { getState } from '../state.svelte';
    import { registerChart, unregisterChart } from '../chartRegistry.svelte';

    const app = getState();
    let { pairKey, timeframe = 60 }: { pairKey: string; timeframe?: number } = $props();
    const pair = $derived(app.instancesMap[pairKey]);
    const tf = $derived(timeframe === 300 ? pair?.smallTerm : pair?.microTerm);

    let container: HTMLDivElement;
    let chart: IChartApi;
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
            timeScale: { borderColor: '#2a2e39', visible: false, timeVisible: true, secondsVisible: true },
            handleScale: true,
            handleScroll: true,
        });

        adxSeries = chart.addSeries(LineSeries, { color: '#f1c40f', lineWidth: 2, lineStyle: LineStyle.Solid, priceLineVisible: false });
        adxPlusSeries = chart.addSeries(LineSeries, { color: '#2ecc71', lineWidth: 1, lineStyle: LineStyle.Solid, priceLineVisible: false });
        adxMinusSeries = chart.addSeries(LineSeries, { color: '#e74c3c', lineWidth: 1, lineStyle: LineStyle.Solid, priceLineVisible: false });

        // Trend threshold line at 20 (dashed gray)
        trendLine = adxSeries.createPriceLine({
            price: 20,
            color: '#4c525e',
            lineWidth: 1,
            lineStyle: LineStyle.Dashed,
            axisLabelVisible: true,
            title: 'TREND',
        });

        // Exhaustion threshold line at 40 (dashed red)
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

        registerChart(chart);

        (async () => {
            if (!pair) return;
            try {
                const res = await fetch(`/api/history?symbol=${encodeURIComponent(pairKey)}&timeframe_secs=${timeframe}`);
                const data = await res.json();
                const indicatorHistory = data.indicator_history;
                if (indicatorHistory && indicatorHistory.adx_14 && indicatorHistory.adx_14.length > 0) {
                    const adxData = indicatorHistory.times.map((t: number, i: number) => ({
                        time: t as Time,
                        value: indicatorHistory.adx_14[i] ? parseFloat(indicatorHistory.adx_14[i]) : 0
                    }));
                    const plusData = indicatorHistory.times.map((t: number, i: number) => ({
                        time: t as Time,
                        value: indicatorHistory.adx_plus[i] ? parseFloat(indicatorHistory.adx_plus[i]) : 0
                    }));
                    const minusData = indicatorHistory.times.map((t: number, i: number) => ({
                        time: t as Time,
                        value: indicatorHistory.adx_minus[i] ? parseFloat(indicatorHistory.adx_minus[i]) : 0
                    }));

                    adxSeries.setData(adxData);
                    adxPlusSeries.setData(plusData);
                    adxMinusSeries.setData(minusData);
                    chart.timeScale().fitContent();
                } else if (data.prices && data.prices.length > 0) {
                    const hasCandles = data.candles && data.candles.length > 0;
                    const source = hasCandles ? data.candles : data.prices;

                    const now = Math.floor(Date.now() / 1000);
                    const step = tf.barDurationSec || 60;
                    const baseTime = now - (data.prices.length * step);

                    const placeholder = source.map((item: any, idx: number) => ({
                        time: hasCandles ? (item.time / 1000) as Time : (baseTime + (idx * step)) as Time,
                        value: 0
                    }));

                    adxSeries.setData(placeholder);
                    adxPlusSeries.setData(placeholder);
                    adxMinusSeries.setData(placeholder);
                    chart.timeScale().fitContent();
                }
            } catch (err) {
                console.error("Error bootstrapping ADX chart history:", err);
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

    function adxLineColor(val: number, slope: number, regime: string): string {
        if (val > 40) return '#ff5252';         // Pulsing Red — extreme exhaustion
        if (val < 20) return '#4c525e';          // Dull Gray — congestion
        if (slope > 0) return '#f1c40f';          // Bright Yellow/Gold — accelerating
        return '#f97316';                          // Orange — decelerating (above trend)
    }

    $effect(() => {
        if (!pair) return;
        const snap = tf.latestSnapshot;
        if (!snap) return;
        const timeSec = snap.timestamp as number;
        if (snap.adx_14 != null) {
            const adxVal = parseFloat(String(snap.adx_14));
            const slope = snap.adx_slope != null ? parseFloat(String(snap.adx_slope)) : 0;
            const regime = snap.adx_regime != null ? String(snap.adx_regime) : 'congestion';

            adxSeries.update({ time: timeSec as Time, value: adxVal });
            if (snap.adx_plus) adxPlusSeries.update({ time: timeSec as Time, value: parseFloat(String(snap.adx_plus)) });
            if (snap.adx_minus) adxMinusSeries.update({ time: timeSec as Time, value: parseFloat(String(snap.adx_minus)) });

            // Dynamic ADX line coloring
            const color = adxLineColor(adxVal, slope, regime);
            adxSeries.applyOptions({ color });

            // Update state
            tf.adxSlope = slope;
            tf.adxTrendingRegime = regime;
            tf.adxExhaustionReached = adxVal > 40;
        }
        if (snap.adx_di_crossover_detected != null) {
            tf.adxDiCrossoverDetected = !!snap.adx_di_crossover_detected;
        }
        if (snap.adx_di_crossover_direction != null) {
            tf.adxDiCrossoverDirection = String(snap.adx_di_crossover_direction);
        }
    });
</script>

<div class="chart-container" bind:this={container}></div>

<style>
    .chart-container { width: 100%; height: 100%; }
</style>
