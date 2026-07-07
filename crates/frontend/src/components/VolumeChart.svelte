<script lang="ts">
    import { iRaw } from '../lib/telemetry';
    import type { IndicatorMap } from '../types';
    import { flattenHistory } from '../lib/historyAdapter';
    import { onMount, onDestroy } from 'svelte';
    import { createChart, CrosshairMode, HistogramSeries } from 'lightweight-charts';
    import type { IChartApi, ISeriesApi, Time } from 'lightweight-charts';
    import { useAppStore } from '../state.svelte';
    import { registerChart, unregisterChart } from '../chartRegistry.svelte';

    const app = useAppStore();
    let { pairKey, timeframe = 60, onDoubleClick, onScreenshotReady }: { pairKey: string; timeframe?: number; onDoubleClick?: () => void; onScreenshotReady?: (fn: () => void) => void } = $props();
    const pair = $derived(app.instancesMap[pairKey]);
    const tf = $derived(
        timeframe === 300 ? pair?.fastTerm :
        timeframe === 900 ? pair?.slowTerm :
        timeframe === 3600 ? pair?.macroTerm :
        pair?.microTerm
    );

    let container: HTMLDivElement;
    let chart: IChartApi;
    let volumeSeries: ISeriesApi<'Histogram'>;

    onMount(() => {
        chart = createChart(container, {
            autoSize: true,
            layout: { background: { color: '#0b0c10' }, textColor: '#94a3b8', fontSize: 10 },
            grid: { vertLines: { color: '#1c212e' }, horzLines: { color: '#1c212e' } },
            crosshair: { mode: CrosshairMode.Normal, vertLine: { color: '#334155', width: 1, style: 3 }, horzLine: { color: '#334155', width: 1, style: 3 } },
            rightPriceScale: { borderColor: '#2d3448', scaleMargins: { top: 0.15, bottom: 0.1 } },
            timeScale: {
                borderColor: '#2d3448',
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
                const res = await fetch(`/api/history?symbol=${encodeURIComponent(pairKey)}&timeframe_secs=${timeframe}`);
                const data = await res.json();
                if (data.prices && data.prices.length > 0) {
                    const hasCandles = data.candles && data.candles.length > 0;
                    const source = hasCandles ? data.candles : data.prices;

                    const now = Math.floor(Date.now() / 1000);
                    const step = tf.barDurationSec || 60;
                    const baseTime = now - (data.prices.length * step);

                    const rvolHistory = flattenHistory(data.indicator_history).rvol;

                    const rawCombined = source.map((item: any, idx: number) => ({
                        time: hasCandles ? Math.floor(item.time / 1000) : (baseTime + (idx * step)),
                        close: hasCandles ? (parseFloat(item.close) || 0) : 0,
                        open: hasCandles ? (parseFloat(item.open) || 0) : 0,
                        volume: hasCandles ? (parseFloat(item.volume) || 0) : 0,
                        rvolRaw: rvolHistory[idx] ?? null
                    }));

                    const seenTimes = new Set<number>();
                    const cleanedCombined: { time: number; close: number; open: number; volume: number; rvolRaw: string | null }[] = [];
                    for (const item of rawCombined) {
                        if (item && item.time && !seenTimes.has(item.time)) {
                            seenTimes.add(item.time);
                            cleanedCombined.push(item);
                        }
                    }
                    cleanedCombined.sort((a, b) => a.time - b.time);

                    const placeholder = cleanedCombined.map(item => ({
                        time: item.time as Time,
                        value: item.volume,
                        color: hasCandles
                            ? volumeColor(item.rvolRaw != null ? parseFloat(item.rvolRaw) : 1.0, item.close, item.open)
                            : '#131722'
                    }));

                    volumeSeries.setData(placeholder);
                    chart.timeScale().fitContent();
                }
            } catch (err) {
                console.error("Error bootstrapping volume chart history:", err);
            }
        })();

        const ro = new ResizeObserver(() => {
            const w = container.clientWidth, h = container.clientHeight; if (chart && w > 0 && h > 0) chart.resize(w, h);
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

    function volumeColor(rvol: number, close: number, open: number): string {
        if (rvol >= 3.0) return '#e040fb';       // Magenta — Exhaustion Climax
        if (rvol >= 1.5) return '#26c6da';       // Cyan — Institutional
        if (rvol < 1.0) return 'rgba(143, 146, 157, 0.25)'; // Translucent gray — Consolidation
        return close >= open ? '#26a69a' : '#ef5350'; // Standard green/red — Normal
    }

    $effect(() => {
        if (!pair) return;
        const snap = tf.latestSnapshot;
        if (!snap) return;
        const timeSec = snap.timestamp as number;
        if (snap.open != null && snap.close != null) {
            const close = parseFloat(String(snap.close));
            const open = parseFloat(String(snap.open));
            const vol = parseFloat(String(snap.volume));
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
