<script lang="ts">
    import { flattenHistory } from '../lib/historyAdapter';
    import { iRaw } from '../lib/telemetry';
    import type { IndicatorMap } from '../types';
    import { onMount, onDestroy } from 'svelte';
    import { createChart, CrosshairMode, LineSeries } from 'lightweight-charts';
    import type { IChartApi, ISeriesApi, Time } from 'lightweight-charts';
    import { useAppStore } from '../state.svelte';
    import { registerChart, unregisterChart } from '../chartRegistry.svelte';
    import { takeChartScreenshot } from '../lib/chartScreenshot';
    import ChartFullscreenOverlay from './ChartFullscreenOverlay.svelte';

    const app = useAppStore();
    let { pairKey, timeframe = 60, onDoubleClick, onScreenshotReady }: { pairKey: string; timeframe?: number; onDoubleClick?: () => void; onScreenshotReady?: (fn: () => void) => void } = $props();
    const pair = $derived(app.instancesMap[pairKey]);
    const tf = $derived(
        timeframe === 180 ? pair?.fastTerm :
        timeframe === 300 ? pair?.slowTerm :
        timeframe === 900 ? pair?.macroTerm :
        pair?.microTerm
    );

    let container: HTMLDivElement;
    let chart: IChartApi = $state(null!);
    let ro: ResizeObserver;
    let rsiSeries: ISeriesApi<'Line'>;
    let isFullscreen = $state(false);

    function toggleFullscreen() {
        isFullscreen = !isFullscreen;
        if (chart && container) {
            requestAnimationFrame(() => chart.resize(container.clientWidth, container.clientHeight));
        }
    }
    function screenshotChart() { if (chart) takeChartScreenshot(chart, `rsi-${pairKey}-${timeframe}s`); }

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

        registerChart(chart);

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
                const res = await fetch(`/api/history?symbol=${encodeURIComponent(pairKey)}&timeframe_secs=${timeframe}&limit=1000`);
                const data = await res.json();
                const indicatorHistory = flattenHistory(data.indicator_history);
                if (indicatorHistory && indicatorHistory.rsi_14 && indicatorHistory.rsi_14.length > 0) {
                    const rawRsiData = indicatorHistory.times.map((t: number, i: number) => {
                        const val = indicatorHistory.rsi_14[i];
                        return {
                            time: t as Time,
                            value: val != null ? parseFloat(val) : null
                        };
                    }).filter((d: { value: number | null }) => d.value !== null);

                    const seenTimes = new Set<number>();
                    const cleanedRsiData: { time: Time; value: number }[] = [];
                    for (const item of rawRsiData) {
                        const tNum = item.time as number;
                        if (item && tNum && !seenTimes.has(tNum)) {
                            seenTimes.add(tNum);
                            cleanedRsiData.push(item as { time: Time; value: number });
                        }
                    }
                    cleanedRsiData.sort((a, b) => (a.time as number) - (b.time as number));

                    if (cleanedRsiData.length > 0) {
                        rsiSeries.setData(cleanedRsiData);
                        chart.timeScale().fitContent();
                    }
                } else if (data.prices && data.prices.length > 0) {
                    const hasCandles = data.candles && data.candles.length > 0;
                    const source = hasCandles ? data.candles : data.prices;

                    const now = Math.floor(Date.now() / 1000);
                    const step = tf.barDurationSec || 60;
                    const baseTime = now - (data.prices.length * step);

                    const seenTimes = new Set<number>();
                    const placeholder: { time: Time; value: number }[] = [];
                    for (let idx = 0; idx < source.length; idx++) {
                        const item = source[idx];
                        const tVal = hasCandles ? Math.floor(item.time / 1000) : (baseTime + (idx * step));
                        if (!seenTimes.has(tVal)) {
                            seenTimes.add(tVal);
                            placeholder.push({ time: tVal as Time, value: 50 });
                        }
                    }
                    placeholder.sort((a, b) => (a.time as number) - (b.time as number));

                    rsiSeries.setData(placeholder);
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

    $effect(() => {
        if (!pair) return;
        const snap = tf.latestSnapshot;
        if (!snap) return;
        const timeSec = snap.timestamp as number;
        const val = iRaw((snap.indicators ?? {}) as IndicatorMap, 'rsi');
        if (val != null) {
            rsiSeries.update({ time: timeSec as Time, value: val });
        }
    });
</script>

<div class="chart-wrapper" class:fs-active={isFullscreen} ondblclick={toggleFullscreen} role="presentation">
    <div class="chart-container" bind:this={container}></div>
</div>

<ChartFullscreenOverlay open={isFullscreen} title="RSI 14 — {pairKey} · {timeframe}s" chart={chart} onclose={toggleFullscreen} />

<style>
    .chart-container { width: 100%; height: 100%; }
    .chart-wrapper { width: 100%; height: 100%; }
    .chart-wrapper.fs-active {
        position: fixed; inset: 0; z-index: 990;
        background: #131722; padding: 44px 16px 16px 16px; box-sizing: border-box;
    }
</style>
