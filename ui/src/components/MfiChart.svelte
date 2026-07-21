<script lang="ts">
    import { flattenHistory } from '../lib/historyAdapter';
    import { iRaw, formatTimeframeLabel } from '../lib/telemetry';
    import type { IndicatorMap } from '../types';
    import { onMount, onDestroy } from 'svelte';
    import { createChart, CrosshairMode, LineSeries, LineStyle } from 'lightweight-charts';
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
    let mfiSeries: ISeriesApi<'Line'>;
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
        mfiSeries = chart.addSeries(LineSeries, { color: '#ab47bc', lineWidth: 1, priceLineVisible: false });
        mfiSeries.createPriceLine({ price: 80, color: '#e74c3c', lineWidth: 1, lineStyle: LineStyle.Dashed, axisLabelVisible: true, title: 'OB' });
        mfiSeries.createPriceLine({ price: 20, color: '#2ecc71', lineWidth: 1, lineStyle: LineStyle.Dashed, axisLabelVisible: true, title: 'OS' });
        registerChart(chart);
        markers = createSignalMarkers(mfiSeries);
        if (onDoubleClick) chart.subscribeDblClick(onDoubleClick);
        if (onScreenshotReady) {
            onScreenshotReady(() => {
                if (!chart) return;
                const link = document.createElement('a');
                link.download = `${pairKey}_${formatTimeframeLabel(timeframe)}_mfi.png`;
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
                if (ih && ih.mfi && ih.mfi.length > 0) {
                    const seen = new Set<number>();
                    const d: { time: Time; value: number }[] = [];
                    for (let i = 0; i < ih.times.length; i++) {
                        const t = ih.times[i];
                        const v = ih.mfi[i];
                        if (t == null || v == null || seen.has(t)) continue;
                        seen.add(t);
                        d.push({ time: t as Time, value: parseFloat(v) });
                    }
                    if (d.length > 0) { mfiSeries.setData(d); chart.timeScale().fitContent(); }
                }
            } catch (err) {
                console.error('Error bootstrapping MFI history:', err);
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
        const v = iRaw((snap.indicators ?? {}) as IndicatorMap, 'mfi');
        if (v != null) mfiSeries.update({ time: timeSec as Time, value: v });
        markers?.push(timeSec, ((snap.indicators ?? {}) as IndicatorMap)['mfi']?.signals ?? []);
    });
</script>

<div class="chart-container" bind:this={container}></div>

<style>
    .chart-container { width: 100%; height: 100%; }
</style>
