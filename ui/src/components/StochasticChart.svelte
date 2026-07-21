<script lang="ts">
    import { flattenHistory } from '../lib/historyAdapter';
    import { iSub, formatTimeframeLabel } from '../lib/telemetry';
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
    let kSeries: ISeriesApi<'Line'>;
    let dSeries: ISeriesApi<'Line'>;
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

        kSeries = chart.addSeries(LineSeries, { color: '#64ffda', lineWidth: 1, priceLineVisible: false });
        dSeries = chart.addSeries(LineSeries, { color: '#ff9800', lineWidth: 1, priceLineVisible: false });

        kSeries.createPriceLine({ price: 80, color: '#e74c3c', lineWidth: 1, lineStyle: LineStyle.Dashed, axisLabelVisible: true, title: 'OB' });
        kSeries.createPriceLine({ price: 20, color: '#2ecc71', lineWidth: 1, lineStyle: LineStyle.Dashed, axisLabelVisible: true, title: 'OS' });
        markers = createSignalMarkers(kSeries);

        registerChart(chart);
        if (onDoubleClick) chart.subscribeDblClick(onDoubleClick);

        if (onScreenshotReady) {
            onScreenshotReady(() => {
                if (!chart) return;
                const canvas = chart.takeScreenshot();
                const link = document.createElement('a');
                link.download = `${pairKey}_${formatTimeframeLabel(timeframe)}_stochastic.png`;
                link.href = canvas.toDataURL('image/png');
                link.click();
            });
        }

        (async () => {
            if (!pair) return;
            try {
                const res = await fetch(`/api/history?symbol=${encodeURIComponent(pairKey)}&timeframe_secs=${timeframe}`);
                const data = await res.json();
                const ih = flattenHistory(data.indicator_history);
                if (ih && ih.stoch_k && ih.stoch_k.length > 0) {
                    const seen = new Set<number>();
                    const kData: { time: Time; value: number }[] = [];
                    const dData: { time: Time; value: number }[] = [];
                    for (let i = 0; i < ih.times.length; i++) {
                        const t = ih.times[i];
                        if (t == null || seen.has(t)) continue;
                        const kv = ih.stoch_k[i];
                        const dv = ih.stoch_d[i];
                        if (kv == null) continue;
                        seen.add(t);
                        kData.push({ time: t as Time, value: parseFloat(kv) });
                        dData.push({ time: t as Time, value: dv != null ? parseFloat(dv) : parseFloat(kv) });
                    }
                    if (kData.length > 0) {
                        kSeries.setData(kData);
                        dSeries.setData(dData);
                        chart.timeScale().fitContent();
                    }
                }
            } catch (err) {
                console.error("Error bootstrapping Stochastic history:", err);
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
        const m = (snap.indicators ?? {}) as IndicatorMap;
        const k = iSub(m, 'stochastic', 'k_line');
        const d = iSub(m, 'stochastic', 'd_line');
        if (k != null && d != null) {
            kSeries.update({ time: timeSec as Time, value: k });
            dSeries.update({ time: timeSec as Time, value: d });
        }
        markers?.push(timeSec, m['stochastic']?.signals ?? []);
    });
</script>

<div class="chart-container" bind:this={container}></div>

<style>
    .chart-container { width: 100%; height: 100%; }
</style>
