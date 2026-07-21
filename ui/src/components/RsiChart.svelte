<script lang="ts">
    import { fetchChartHistoryOnce, dedupSortByTime } from '../lib/chartHistory';
    import { iRaw } from '../lib/telemetry';
    import type { IndicatorMap } from '../types';
    import { onMount, onDestroy } from 'svelte';
    import { createChart, CrosshairMode, LineSeries } from 'lightweight-charts';
    import type { IChartApi, ISeriesApi, Time } from 'lightweight-charts';
    import { useAppStore } from '../state.svelte';
    import { registerChart, unregisterChart } from '../chartRegistry.svelte';
    import { makeChartCoalescer } from '../lib/chartCoalesce';

    const app = useAppStore();
    let { pairKey, slot, onDoubleClick, onScreenshotReady }: { pairKey: string; slot: 'micro' | 'fast' | 'slow' | 'macro'; onDoubleClick?: () => void; onScreenshotReady?: (fn: () => void) => void } = $props();
    const pair = $derived(app.instancesMap[pairKey]);
    // Slot identity is positional and stable; never re-derive from duration.
    const tf = $derived(
        slot === 'micro' ? pair?.microTerm :
        slot === 'fast'  ? pair?.fastTerm :
        slot === 'slow'  ? pair?.slowTerm :
                          pair?.macroTerm
    );
    const timeframe = $derived(tf?.barDurationSec ?? 60);

    let container: HTMLDivElement;
    let chart: IChartApi;
    let ro: ResizeObserver;
    let rsiSeries: ISeriesApi<'Line'>;

    onMount(() => {
        chart = createChart(container, {
            autoSize: true,
            layout: { background: { color: '#131722' }, textColor: '#8f929d', fontSize: 10 },
            grid: { vertLines: { color: '#1a1d26' }, horzLines: { color: '#1a1d26' } },
            crosshair: { mode: CrosshairMode.Normal, vertLine: { color: '#4c525e', width: 1, style: 3 }, horzLine: { color: '#4c525e', width: 1, style: 3 } },
            rightPriceScale: { borderColor: '#2a2e39', scaleMargins: { top: 0.15, bottom: 0.1 } },
            timeScale: {
                borderColor: '#2a2e39',
                visible: false,
                timeVisible: true,
                secondsVisible: true,
                tickMarkFormatter: (time: any, _tickMarkType: number, _locale: string) => {
                    const date = new Date(time * 1000);
                    const hours = String(date.getHours()).padStart(2, '0');
                    const minutes = String(date.getMinutes()).padStart(2, '0');
                    return `${hours}:${minutes}`;
                }
            },
            handleScale: true,
            handleScroll: true,
        });

        rsiSeries = chart.addSeries(LineSeries, { color: '#7e57c2', lineWidth: 2, priceLineVisible: false });

        chart.priceScale('right').applyOptions({ alignLabels: true });
        chart.timeScale().applyOptions({ rightOffset: 12, barSpacing: 6 });

        registerChart(chart, container);

        if (onDoubleClick) chart.subscribeDblClick(onDoubleClick);

        if (onScreenshotReady) {
            onScreenshotReady(() => {
                if (!chart) return;
                const canvas = chart.takeScreenshot();
                const dataUrl = canvas.toDataURL('image/png');
                const link = document.createElement('a');
                link.download = `${pairKey}_${timeframe}s_rsi.png`;
                link.href = dataUrl;
                link.click();
            });
        }

        (async () => {
            if (!pair) return;
            try {
                const data = await fetchChartHistoryOnce(pairKey, timeframe);
                if (!data || !data.indicatorHistory || !data.indicatorHistory.rsi_14.length) return;
                const ih = data.indicatorHistory;
                const rawRsiData = ih.times.map((t: number, i: number) => {
                    const val = ih.rsi_14[i];
                    return {
                        time: t as Time,
                        value: val != null ? parseFloat(val) : null
                    };
                }).filter((d: { value: number | null }) => d.value !== null);

                const cleanedRsiData = dedupSortByTime(rawRsiData as { time: Time; value: number }[]);

                if (cleanedRsiData.length > 0) {
                    rsiSeries.setData(cleanedRsiData);
                    chart.timeScale().fitContent();
                }
            } catch (err) {
                console.error("Error bootstrapping RSI chart history:", err);
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

    let _lastUpdateTs = 0;
    const rsiCoalescer = makeChartCoalescer(app, pairKey, slot, (snap) => {
        const timeSec = snap.timestamp as number;
        const val = iRaw((snap.indicators ?? {}) as IndicatorMap, 'rsi');
        if (val != null) {
            rsiSeries.update({ time: timeSec as Time, value: val });
        }
    });
    $effect(() => {
        // Track broadcast arrival (the gap diagnostic must measure WS gaps,
        // not rAF gaps) and let the coalescer collapse redraws to one per frame.
        const pairVal = app.instancesMap[pairKey];
        if (!pairVal) return;
        const tfVal = slot === 'micro' ? pairVal.microTerm : slot === 'fast' ? pairVal.fastTerm : slot === 'slow' ? pairVal.slowTerm : pairVal.macroTerm;
        const snap = tfVal.latestSnapshot;
        if (!snap) return;
        const now = Date.now();
        const gap = _lastUpdateTs > 0 ? now - _lastUpdateTs : 0;
        _lastUpdateTs = now;
        if (gap > 10_000) {
            console.warn(`[CHART-DIAG] RsiChart ${pairKey}/${slot}: ${gap}ms gap between updates at ${new Date(now).toISOString()}`);
        }
        rsiCoalescer.effect();
    });
    onDestroy(rsiCoalescer.destroy);
</script>

<div class="chart-container" bind:this={container}></div>

<style>
    .chart-container { width: 100%; height: 100%; }
</style>
