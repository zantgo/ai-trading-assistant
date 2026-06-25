<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { createChart, CrosshairMode, LineSeries } from 'lightweight-charts';
    import type { IChartApi, ISeriesApi, Time } from 'lightweight-charts';
    import { useAppStore } from '../state.svelte';
    import { registerChart, unregisterChart } from '../chartRegistry.svelte';

    const app = useAppStore();
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
    let ro: ResizeObserver;
    let atrSeries: ISeriesApi<'Line'>;
    let atrVal = $state(0);
    let atrRegime = $state('stable');

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

        atrSeries = chart.addSeries(LineSeries, { color: '#8f929d', lineWidth: 2, priceLineVisible: false });

        chart.priceScale('right').applyOptions({ alignLabels: true });
        chart.timeScale().applyOptions({ rightOffset: 12, barSpacing: 6 });

        registerChart(chart);

        (async () => {
            if (!pair) return;
            try {
                const res = await fetch(`/api/history?symbol=${encodeURIComponent(pairKey)}&timeframe_secs=${timeframe}`);
                const data = await res.json();
                const indicatorHistory = data.indicator_history;
                if (indicatorHistory && indicatorHistory.atr_14 && indicatorHistory.atr_14.length > 0) {
                    const atrData = indicatorHistory.times.map((t: number, i: number) => ({
                        time: t as Time,
                        value: indicatorHistory.atr_14[i] ? parseFloat(indicatorHistory.atr_14[i]) : 0
                    }));

                    atrSeries.setData(atrData);
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

                    atrSeries.setData(placeholder);
                    chart.timeScale().fitContent();
                }
            } catch (err) {
                console.error("Error bootstrapping ATR chart history:", err);
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

    function regimeColor(regime: string): string {
        switch (regime) {
            case 'expanding': return '#10b981';  // Bright Green
            case 'contracting': return '#ef4444'; // Dark Red
            default: return '#8f929d';             // Gray (stable)
        }
    }

    function regimeLabel(regime: string): string {
        switch (regime) {
            case 'expanding': return 'EXPANDING';
            case 'contracting': return 'CONTRACTING';
            default: return 'STABLE';
        }
    }

    $effect(() => {
        if (!pair) return;
        const snap = tf.latestSnapshot;
        if (!snap) return;
        const timeSec = snap.timestamp as number;
        if (snap.atr_14 != null) {
            const val = parseFloat(String(snap.atr_14));
            atrSeries.update({ time: timeSec as Time, value: val });
            atrVal = val;

            const regime = snap.atr_volatility_regime != null
                ? String(snap.atr_volatility_regime) as 'expanding' | 'contracting' | 'stable'
                : 'stable';
            atrRegime = regime;
            tf.atrVolatilityRegime = regime;

            const color = regimeColor(regime);
            atrSeries.applyOptions({ color });
        }
    });
</script>

<div class="atr-pane">
    <div class="chart-container" bind:this={container}></div>
</div>

<style>
    .atr-pane { display: flex; flex-direction: column; height: 100%; }
    .chart-container { flex: 1; width: 100%; min-height: 0; }
</style>
