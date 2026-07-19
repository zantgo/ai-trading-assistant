<script lang="ts">
    import { flattenHistory } from '../lib/historyAdapter';
    import { iRaw, formatTimeframeLabel, resolveChartTimeframe } from '../lib/telemetry';
    import type { IndicatorMap } from '../types';
    import { onMount, onDestroy } from 'svelte';
    import { createChart, CrosshairMode, LineSeries, LineStyle } from 'lightweight-charts';
    import type { IChartApi, ISeriesApi, Time } from 'lightweight-charts';
    import { useAppStore } from '../state.svelte';
    import { registerChart, unregisterChart } from '../chartRegistry.svelte';
    import { createSignalMarkers, type SignalMarkerController } from '../lib/signalMarkers';

    const app = useAppStore();
    let { pairKey, timeframe = 60, onDoubleClick, onScreenshotReady }: { pairKey: string; timeframe?: number; onDoubleClick?: () => void; onScreenshotReady?: (fn: () => void) => void } = $props();
    const pair = $derived(app.instancesMap[pairKey]);
    const tf = $derived(resolveChartTimeframe(timeframe, pair));

    let container: HTMLDivElement;
    let chart: IChartApi = $state(null!);
    let cmoSeries: ISeriesApi<'Line'>;
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

        cmoSeries = chart.addSeries(LineSeries, { color: '#e040fb', lineWidth: 1, priceLineVisible: false });

        cmoSeries.createPriceLine({ price: 50, color: '#e74c3c', lineWidth: 1, lineStyle: LineStyle.Dashed, axisLabelVisible: true, title: '+50' });
        cmoSeries.createPriceLine({ price: 0, color: '#4c525e', lineWidth: 1, lineStyle: LineStyle.Solid });
        cmoSeries.createPriceLine({ price: -50, color: '#2ecc71', lineWidth: 1, lineStyle: LineStyle.Dashed, axisLabelVisible: true, title: '-50' });

        registerChart(chart);
        markers = createSignalMarkers(cmoSeries);
        if (onDoubleClick) chart.subscribeDblClick(onDoubleClick);

        if (onScreenshotReady) {
            onScreenshotReady(() => {
                if (!chart) return;
                const canvas = chart.takeScreenshot();
                const link = document.createElement('a');
                link.download = `${pairKey}_${formatTimeframeLabel(timeframe)}_chandemo.png`;
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
                if (ih && ih.chandemo && ih.chandemo.length > 0) {
                    const seen = new Set<number>();
                    const cmoData: { time: Time; value: number }[] = [];
                    for (let i = 0; i < ih.times.length; i++) {
                        const t = ih.times[i];
                        if (t == null || seen.has(t)) continue;
                        const v = ih.chandemo[i];
                        if (v == null) continue;
                        seen.add(t);
                        cmoData.push({ time: t as Time, value: parseFloat(v) });
                    }
                    if (cmoData.length > 0) {
                        cmoSeries.setData(cmoData);
                        chart.timeScale().fitContent();
                    }
                }
            } catch (err) {
                console.error("Error bootstrapping ChandeMO history:", err);
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
        const cmo = iRaw(m, 'chandemo');
        if (cmo != null) {
            cmoSeries.update({ time: timeSec as Time, value: cmo });
        }
        markers?.push(timeSec, ((snap.indicators ?? {}) as IndicatorMap)['chandemo']?.signals ?? []);
    });
</script>

<div class="chart-container" bind:this={container}></div>

<style>
    .chart-container { width: 100%; height: 100%; }
</style>
