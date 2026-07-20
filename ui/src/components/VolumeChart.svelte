<script lang="ts">
    import { iRaw } from '../lib/telemetry';
    import type { IndicatorMap } from '../types';
    import { fetchChartHistoryOnce, dedupSortByTime } from '../lib/chartHistory';
    import { onMount, onDestroy } from 'svelte';
    import { createChart, CrosshairMode, HistogramSeries } from 'lightweight-charts';
    import type { IChartApi, ISeriesApi, Time } from 'lightweight-charts';
    import { useAppStore } from '../state.svelte';
    import { registerChart, unregisterChart } from '../chartRegistry.svelte';

    const app = useAppStore();
    let { pairKey, slot, onDoubleClick, onScreenshotReady }: { pairKey: string; slot: 'micro' | 'fast' | 'slow' | 'macro'; onDoubleClick?: () => void; onScreenshotReady?: (fn: () => void) => void } = $props();
    const pair = $derived(app.instancesMap[pairKey]);
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
    let volumeSeries: ISeriesApi<'Histogram'>;

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

        volumeSeries = chart.addSeries(HistogramSeries, { base: 0, priceLineVisible: false });

        chart.priceScale('right').applyOptions({ alignLabels: true });
        chart.timeScale().applyOptions({ rightOffset: 12, barSpacing: 6 });

        registerChart(chart);

        if (onDoubleClick) chart.subscribeDblClick(onDoubleClick);

        if (onScreenshotReady) {
            onScreenshotReady(() => {
                if (!chart) return;
                const canvas = chart.takeScreenshot();
                const dataUrl = canvas.toDataURL('image/png');
                const link = document.createElement('a');
                link.download = `${pairKey}_${timeframe}s_volume.png`;
                link.href = dataUrl;
                link.click();
            });
        }

        (async () => {
            if (!pair) return;
            try {
                const data = await fetchChartHistoryOnce(pairKey, timeframe);
                if (!data) return;
                if (data.candles && data.candles.length > 0) {
                    const rvolHistory = data.indicatorHistory?.rvol ?? [];

                    const rawCombined = data.candles.map((c, idx) => ({
                        time: Math.floor(c.time / 1000) as Time,
                        close: parseFloat(c.close) || 0,
                        open: parseFloat(c.open) || 0,
                        volume: parseFloat(c.volume) || 0,
                        rvolRaw: rvolHistory[idx] ?? null
                    }));

                    const cleanedCombined = dedupSortByTime(rawCombined.map((c) => ({
                        time: c.time as unknown as Time,
                        close: c.close,
                        open: c.open,
                        volume: c.volume,
                        rvolRaw: c.rvolRaw,
                    }))) as { time: Time; close: number; open: number; volume: number; rvolRaw: string | null }[];

                    const placeholder = cleanedCombined.map((item) => ({
                        time: item.time,
                        value: item.volume,
                        color: volumeColor(
                            item.rvolRaw != null ? parseFloat(item.rvolRaw) : 1.0,
                            item.close,
                            item.open
                        ),
                    }));

                    volumeSeries.setData(placeholder);
                    chart.timeScale().fitContent();
                }
            } catch (err) {
                console.error("Error bootstrapping volume chart history:", err);
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

    function volumeColor(rvol: number, close: number, open: number): string {
        if (rvol >= 3.0) return '#e040fb';
        if (rvol >= 1.5) return '#26c6da';
        if (rvol < 1.0) return 'rgba(143, 146, 157, 0.25)';
        return close >= open ? '#26a69a' : '#ef5350';
    }

    $effect(() => {
        if (!pair) return;
        const snap = tf?.latestSnapshot;
        if (!snap) return;
        const timeSec = snap.timestamp as number;
        if (snap.open != null && snap.close != null) {
            const close = parseFloat(String(snap.close));
            const open = parseFloat(String(snap.open));
            const vol = parseFloat(String(snap.volume ?? '0')) || 0;
            const rvol = iRaw((snap.indicators ?? {}) as IndicatorMap, 'rvol') ?? 1.0;

            const color = volumeColor(rvol, close, open);
            volumeSeries.update({ time: timeSec as Time, value: vol, color });
        }
    });
</script>

<div class="chart-container" bind:this={container}></div>

<style>
    .chart-container { width: 100%; height: 100%; }
</style>
