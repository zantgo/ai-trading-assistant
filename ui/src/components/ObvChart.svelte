<script lang="ts">
    import { flattenHistory } from '../lib/historyAdapter';
    import { iRaw, formatTimeframeLabel } from '../lib/telemetry';
    import type { IndicatorMap } from '../types';
    import { onMount, onDestroy } from 'svelte';
    import { createChart, CrosshairMode, LineSeries } from 'lightweight-charts';
    import type { IChartApi, ISeriesApi, Time } from 'lightweight-charts';
    import { useAppStore } from '../state.svelte';
    import { registerChart, unregisterChart } from '../chartRegistry.svelte';
    import { createSignalMarkers, type SignalMarkerController } from '../lib/signalMarkers';

    const app = useAppStore();
    let { pairKey, slot, onDoubleClick, onScreenshotReady }: { pairKey: string; slot: 'micro' | 'fast' | 'slow' | 'macro'; onDoubleClick?: () => void; onScreenshotReady?: (fn: () => void) => void } = $props();
    const pair = $derived(app.instancesMap[pairKey]);
    const tf = $derived(slot === 'micro' ? pair?.microTerm : slot === 'fast' ? pair?.fastTerm : slot === 'slow' ? pair?.slowTerm : pair?.macroTerm); const timeframe = $derived(tf?.barDurationSec ?? 60);

    let container: HTMLDivElement;
    let chart: IChartApi = $state(null!);
    let obvSeries: ISeriesApi<'Line'>;
    let markers: SignalMarkerController;
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
        obvSeries = chart.addSeries(LineSeries, { color: '#bfc1c8', lineWidth: 1, priceLineVisible: false });
        registerChart(chart);
        markers = createSignalMarkers(obvSeries);
        if (onDoubleClick) chart.subscribeDblClick(onDoubleClick);
        if (onScreenshotReady) {
            onScreenshotReady(() => {
                if (!chart) return;
                const link = document.createElement('a');
                link.download = `${pairKey}_${formatTimeframeLabel(timeframe)}_obv.png`;
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
                if (ih && ih.obv && ih.obv.length > 0) {
                    const seen = new Set<number>();
                    const d: { time: Time; value: number }[] = [];
                    for (let i = 0; i < ih.times.length; i++) {
                        const t = ih.times[i];
                        const v = ih.obv[i];
                        if (t == null || v == null || seen.has(t)) continue;
                        seen.add(t);
                        d.push({ time: t as Time, value: parseFloat(v) });
                    }
                    if (d.length > 0) { obvSeries.setData(d); chart.timeScale().fitContent(); }
                }
            } catch (err) {
                console.error('Error bootstrapping OBV history:', err);
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
        const pairVal = app.instancesMap[pairKey];
        if (!pairVal) return;
        const tfVal = slot === 'micro' ? pairVal.microTerm : slot === 'fast' ? pairVal.fastTerm : slot === 'slow' ? pairVal.slowTerm : pairVal.macroTerm;
        const snap = tfVal.latestSnapshot;
        if (!snap) return;
        const timeSec = snap.timestamp as number;
        const v = iRaw((snap.indicators ?? {}) as IndicatorMap, 'obv');
        if (v != null) obvSeries.update({ time: timeSec as Time, value: v });
        markers?.push(timeSec, ((snap.indicators ?? {}) as IndicatorMap)['obv']?.signals ?? []);
    });
</script>

<div class="chart-container" bind:this={container}></div>

<style>
    .chart-container { width: 100%; height: 100%; }
</style>
