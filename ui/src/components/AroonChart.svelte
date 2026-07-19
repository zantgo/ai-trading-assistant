<script lang="ts">
    import { flattenHistory } from '../lib/historyAdapter';
    import { iSub, formatTimeframeLabel, resolveChartTimeframe } from '../lib/telemetry';
    import type { IndicatorMap } from '../types';
    import { onMount, onDestroy } from 'svelte';
    import { createChart, CrosshairMode, LineSeries, LineStyle } from 'lightweight-charts';
    import type { IChartApi, ISeriesApi, Time } from 'lightweight-charts';
    import { useAppStore } from '../state.svelte';
    import { registerChart, unregisterChart } from '../chartRegistry.svelte';

    const app = useAppStore();
    let { pairKey, timeframe = 60, onDoubleClick, onScreenshotReady }: { pairKey: string; timeframe?: number; onDoubleClick?: () => void; onScreenshotReady?: (fn: () => void) => void } = $props();
    const pair = $derived(app.instancesMap[pairKey]);
    const tf = $derived(
        timeframe === pair?.fastTerm?.barDurationSec ? pair?.fastTerm :
        timeframe === pair?.slowTerm?.barDurationSec ? pair?.slowTerm :
        timeframe === pair?.macroTerm?.barDurationSec ? pair?.macroTerm :
        pair?.microTerm
    );

    let container: HTMLDivElement;
    let chart: IChartApi = $state(null!);
    let upSeries: ISeriesApi<'Line'>;
    let downSeries: ISeriesApi<'Line'>;
    let ro: ResizeObserver;

    onMount(() => {
        chart = createChart(container, {
            autoSize: true,
            layout: { background: { color: '#131722' }, textColor: '#8f929d', fontSize: 10 },
            grid: { vertLines: { color: '#1a1d26' }, horzLines: { color: '#1a1d26' } },
            crosshair: { mode: CrosshairMode.Normal, vertLine: { color: '#4c525e', width: 1, style: 3 }, horzLine: { color: '#4c525e', width: 1, style: 3 } },
            rightPriceScale: { borderColor: '#2a2e39', scaleMargins: { top: 0.15, bottom: 0.1 } },
            timeScale: { borderColor: '#2a2e39', visible: false },
        });
        upSeries = chart.addSeries(LineSeries, { color: '#26a69a', lineWidth: 1, priceLineVisible: false, lastValueVisible: false, crosshairMarkerVisible: false });
        downSeries = chart.addSeries(LineSeries, { color: '#ef5350', lineWidth: 1, priceLineVisible: false, lastValueVisible: false, crosshairMarkerVisible: false });
        upSeries.createPriceLine({ price: 70, color: '#4c525e', lineWidth: 1, lineStyle: LineStyle.Dotted });
        downSeries.createPriceLine({ price: 30, color: '#4c525e', lineWidth: 1, lineStyle: LineStyle.Dotted });
        registerChart(chart);
        if (onDoubleClick) chart.subscribeDblClick(onDoubleClick);
        if (onScreenshotReady) {
            onScreenshotReady(() => {
                if (!chart) return;
                const link = document.createElement('a');
                link.download = `${pairKey}_${formatTimeframeLabel(timeframe)}_aroon.png`;
                link.href = chart.takeScreenshot().toDataURL('image/png');
                link.click();
            });
        }
        (async () => {
            if (!pair) return;
            try {
                const res = await fetch(`/api/history?symbol=${encodeURIComponent(pairKey)}&timeframe_secs=${timeframe}`);
                const data = await res.json();
                const ih = flattenHistory(data.indicator_history);
                if (ih && ih.aroon_up && ih.aroon_up.length > 0) {
                    const seenUp = new Set<number>();
                    const seenDown = new Set<number>();
                    const upData: { time: Time; value: number }[] = [];
                    const downData: { time: Time; value: number }[] = [];
                    for (let i = 0; i < ih.times.length; i++) {
                        const t = ih.times[i];
                        const u = ih.aroon_up[i];
                        const d = ih.aroon_down[i];
                        if (t == null) continue;
                        if (u != null && !seenUp.has(t)) {
                            seenUp.add(t);
                            upData.push({ time: t as Time, value: parseFloat(u) });
                        }
                        if (d != null && !seenDown.has(t)) {
                            seenDown.add(t);
                            downData.push({ time: t as Time, value: parseFloat(d) });
                        }
                    }
                    if (upData.length > 0) { upSeries.setData(upData); }
                    if (downData.length > 0) { downSeries.setData(downData); }
                    if (upData.length > 0 || downData.length > 0) { chart.timeScale().fitContent(); }
                }
            } catch (err) {
                console.error('Error bootstrapping Aroon history:', err);
            }
        })();
        ro = new ResizeObserver(() => {
            const w = container.clientWidth, h = container.clientHeight;
            if (chart && w > 0 && h > 0) chart.resize(w, h);
        });
        if (container?.parentElement) ro.observe(container.parentElement);
    });

    onDestroy(() => {
        ro?.disconnect();
        if (chart) { unregisterChart(chart); chart.remove(); }
    });

    $effect(() => {
        if (!pair) return;
        const snap = tf?.latestSnapshot;
        if (!snap) return;
        const timeSec = snap.timestamp as number;
        const m = (snap.indicators ?? {}) as IndicatorMap;
        const up = iSub(m, 'aroon', 'up');
        const down = iSub(m, 'aroon', 'down');
        if (up != null) upSeries.update({ time: timeSec as Time, value: up });
        if (down != null) downSeries.update({ time: timeSec as Time, value: down });
    });
</script>

<div class="chart-container" bind:this={container}></div>

<style>
    .chart-container { width: 100%; height: 100%; }
</style>
