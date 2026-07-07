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
    let ro: ResizeObserver;
    let bbwpSeries: ISeriesApi<'Histogram'>;

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
            color: '#4488ff',
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
                            color: val < 10 ? '#4488ff' : val > 90 ? '#ff4444' : '#00d4aa',
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
                color: val < 10 ? '#4488ff' : val > 90 ? '#ff4444' : '#00d4aa'
            });
        }
    });
</script>

<div class="chart-container" bind:this={container}></div>

<style>
    .chart-container { width: 100%; height: 100%; }
</style>
