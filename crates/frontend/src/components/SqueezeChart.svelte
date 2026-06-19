<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { createChart, CrosshairMode, HistogramSeries } from 'lightweight-charts';
    import type { IChartApi, ISeriesApi, Time } from 'lightweight-charts';
    import { getState } from '../state.svelte';
    import { registerChart, unregisterChart } from '../chartRegistry.svelte';

    const app = getState();
    let { pairKey, timeframe = 60 }: { pairKey: string; timeframe?: number } = $props();
    const pair = $derived(app.instancesMap[pairKey]);
    const tf = $derived(
        timeframe === 300 ? pair?.smallTerm :
        timeframe === 900 ? pair?.mediumTerm :
        timeframe === 3600 ? pair?.largeTerm :
        pair?.microTerm
    );

    let container: HTMLDivElement;
    let chart: IChartApi;
    let squeezeMomSeries: ISeriesApi<'Histogram'>;
    let squeezeDotSeries: ISeriesApi<'Histogram'>;

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

        squeezeMomSeries = chart.addSeries(HistogramSeries, { base: 0, priceLineVisible: false });
        squeezeDotSeries = chart.addSeries(HistogramSeries, {
            base: 0,
            priceLineVisible: false,
            priceScaleId: 'squeeze-overlay',
        });

        chart.priceScale('right').applyOptions({ alignLabels: true });
        chart.priceScale('squeeze-overlay').applyOptions({ visible: false });
        chart.timeScale().applyOptions({ rightOffset: 12, barSpacing: 6 });

        registerChart(chart);

        (async () => {
            if (!pair) return;
            try {
                const res = await fetch(`/api/history?symbol=${encodeURIComponent(pairKey)}&timeframe_secs=${timeframe}`);
                const data = await res.json();
                const indicatorHistory = data.indicator_history;
                if (indicatorHistory && indicatorHistory.squeeze_momentum && indicatorHistory.squeeze_momentum.length > 0) {
                    const momData = indicatorHistory.times.map((t: number, i: number) => {
                        const val = indicatorHistory.squeeze_momentum[i] ? parseFloat(indicatorHistory.squeeze_momentum[i]) : 0;
                        return {
                            time: t as Time,
                            value: val,
                            color: val >= 0 ? '#26a69a' : '#ef5350'
                        };
                    });
                    const dotData = indicatorHistory.times.map((t: number, i: number) => ({
                        time: t as Time,
                        value: 0.1,
                        color: indicatorHistory.squeeze_on[i] ? '#ef5350' : '#4caf50'
                    }));

                    squeezeMomSeries.setData(momData);
                    squeezeDotSeries.setData(dotData);
                    chart.timeScale().fitContent();
                } else if (data.prices && data.prices.length > 0) {
                    const hasCandles = data.candles && data.candles.length > 0;
                    const source = hasCandles ? data.candles : data.prices;

                    const now = Math.floor(Date.now() / 1000);
                    const step = tf.barDurationSec || 60;
                    const baseTime = now - (data.prices.length * step);

                    const placeholder = source.map((item: any, idx: number) => ({
                        time: hasCandles ? (item.time / 1000) as Time : (baseTime + (idx * step)) as Time,
                        value: 0,
                        color: '#131722'
                    }));

                    squeezeMomSeries.setData(placeholder);
                    squeezeDotSeries.setData(placeholder);
                    chart.timeScale().fitContent();
                }
            } catch (err) {
                console.error("Error bootstrapping squeeze chart history:", err);
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

    function momentumColor(val: number, direction: string): string {
        switch (direction) {
            case 'BullishAcceleration': return '#26a69a';   // Light Green
            case 'BullishDeceleration': return '#00695c';   // Dark Green — warning
            case 'BearishAcceleration': return '#b71c1c';   // Dark Red
            case 'BearishDeceleration': return '#ff1744';   // Bright Red — warning
            default: return val >= 0 ? '#4caf50' : '#ef5350';
        }
    }

    $effect(() => {
        if (!pair) return;
        const snap = tf.latestSnapshot;
        if (!snap) return;
        const timeSec = snap.timestamp as number;
        if (snap.squeeze_momentum != null) {
            const momVal = parseFloat(String(snap.squeeze_momentum));
            const direction = snap.squeeze_momentum_direction != null
                ? String(snap.squeeze_momentum_direction)
                : 'Flat';

            const momColor = momentumColor(momVal, direction);

            squeezeMomSeries.update({ time: timeSec as Time, value: momVal, color: momColor });
            tf.lastSqzMom = momVal;

            let dotColor = snap.squeeze_on ? '#ef5350' : '#4caf50';
            squeezeDotSeries.update({ time: timeSec as Time, value: 0.1, color: dotColor });

            // Update state
            tf.squeezeMomentumDirection = direction;
        }
        if (snap.squeeze_duration != null) {
            tf.squeezeDuration = Number(snap.squeeze_duration);
        }
        if (snap.squeeze_release_trigger != null) {
            tf.squeezeReleaseTrigger = !!snap.squeeze_release_trigger;
        }
    });
</script>

<div class="chart-container" bind:this={container}></div>

<style>
    .chart-container { width: 100%; height: 100%; }
</style>
