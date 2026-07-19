<script lang="ts">
    import { iRaw } from '../lib/telemetry';
    import type { IndicatorMap } from '../types';
    import { flattenHistory } from '../lib/historyAdapter';
    import { onMount, onDestroy } from 'svelte';
    import { createChart, CrosshairMode, HistogramSeries } from 'lightweight-charts';
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
    let bbwpSeries: ISeriesApi<'Histogram'>;

    let isFullscreen = $state(false);

    function toggleFullscreen() {
        isFullscreen = !isFullscreen;
        if (chart && container) {
            requestAnimationFrame(() => chart.resize(container.clientWidth, container.clientHeight));
        }
    }
    function screenshotChart() { if (chart) takeChartScreenshot(chart, `bbwp-${pairKey}-${timeframe}s`); }

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
                timeVisible: false,
                secondsVisible: false,
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

        chart.priceScale('right').applyOptions({ alignLabels: true });
        chart.timeScale().applyOptions({ rightOffset: 12, barSpacing: 6 });

        bbwpSeries = chart.addSeries(HistogramSeries, {
            color: '#00d4aa',
            base: 0,
            priceLineVisible: false
        });

        // 10% Compression line (dashed blue)
        bbwpSeries.createPriceLine({
            price: 10,
            color: '#8f929d',
            lineWidth: 1,
            lineStyle: 2,
            axisLabelVisible: true,
            title: 'COMPRESSION',
        });

        // 90% Exhaustion line (dashed red)
        bbwpSeries.createPriceLine({
            price: 90,
            color: '#ff4444',
            lineWidth: 1,
            lineStyle: 2,
            axisLabelVisible: true,
            title: 'EXHAUSTION',
        });

        registerChart(chart);

        if (onDoubleClick) chart.subscribeDblClick(onDoubleClick);

        if (onScreenshotReady) {
            onScreenshotReady(() => {
                if (!chart) return;
                const canvas = chart.takeScreenshot();
                const dataUrl = canvas.toDataURL('image/png');
                const link = document.createElement('a');
                link.download = `${pairKey}_${timeframe}s_bbwp.png`;
                link.href = dataUrl;
                link.click();
            });
        }

        // Bootstrap historical data
        (async () => {
            if (!pair) return;
            try {
                const res = await fetch(`/api/history?symbol=${encodeURIComponent(pairKey)}&timeframe_secs=${timeframe}`);
                const data = await res.json();
                const ih = flattenHistory(data.indicator_history);
                if (ih && ih.bbwp && ih.bbwp.length > 0) {
                    const rawBbwpData = ih.times.map((t: number, i: number) => {
                        const val = parseFloat(ih.bbwp[i] ?? "0") || 0;
                        return {
                            time: t as Time,
                            value: val,
                            color: val < 10 ? '#8f929d' : val > 90 ? '#ff4444' : '#00d4aa',
                        };
                    });

                    const seenTimes = new Set<number>();
                    const cleanedBbwpData: { time: Time; value: number; color: string }[] = [];
                    for (const item of rawBbwpData) {
                        const tNum = item.time as number;
                        if (item && tNum && !seenTimes.has(tNum)) {
                            seenTimes.add(tNum);
                            cleanedBbwpData.push(item);
                        }
                    }
                    cleanedBbwpData.sort((a, b) => (a.time as number) - (b.time as number));

                    bbwpSeries.setData(cleanedBbwpData);
                    chart.timeScale().fitContent();
                }
            } catch (err) {
                console.error("Error bootstrapping BBWP chart history:", err);
            }
        })();

        // Watch the parent element dimension adjustments
        ro = new ResizeObserver(() => {
            const w = container.clientWidth;
            const h = container.clientHeight;
            if (chart && w > 0 && h > 0) {
                chart.resize(w, h);
            }
        });
        if (container?.parentElement) {
            ro.observe(container.parentElement);
        }
    });

    onDestroy(() => {
        ro?.disconnect();
        if (chart) {
            unregisterChart(chart);
            chart.remove();
        }
    });

    // Handle real-time WebSockets data changes
    $effect(() => {
        if (!pair) return;
        const snap = tf?.latestSnapshot;
        if (!snap) return;
        const timeSec = snap.timestamp as number;
        const val = iRaw((snap.indicators ?? {}) as IndicatorMap, 'bbwp');
        if (val != null) {
            bbwpSeries.update({
                time: timeSec as Time,
                value: val,
                color: val < 10 ? '#8f929d' : val > 90 ? '#ff4444' : '#00d4aa'
            });
        }
    });
</script>

<div class="chart-wrapper" class:fs-active={isFullscreen} ondblclick={toggleFullscreen} role="presentation">
    <div class="chart-container" bind:this={container}></div>
</div>

<ChartFullscreenOverlay open={isFullscreen} title="BBWP — {pairKey} · {timeframe}s" chart={chart} onclose={toggleFullscreen} />

<style>
    .chart-container { width: 100%; height: 100%; }
    .chart-wrapper { width: 100%; height: 100%; }
    .chart-wrapper.fs-active {
        position: fixed; inset: 0; z-index: 990;
        background: #131722; padding: 44px 16px 16px 16px; box-sizing: border-box;
    }
</style>
