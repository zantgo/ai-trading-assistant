<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { createChart, CrosshairMode, HistogramSeries } from 'lightweight-charts';
    import type { IChartApi, ISeriesApi, Time } from 'lightweight-charts';
    import { useAppStore } from '../state.svelte';
    import { registerChart, unregisterChart } from '../chartRegistry.svelte';

    const app = useAppStore();
    let { pairKey, timeframe = 60, onDoubleClick, onScreenshotReady }: { pairKey: string; timeframe?: number; onDoubleClick?: () => void; onScreenshotReady?: (fn: () => void) => void } = $props();
    const pair = $derived(app.instancesMap[pairKey]);
    const tf = $derived(
        timeframe === 300 ? pair?.smallTerm :
        timeframe === 900 ? pair?.mediumTerm :
        timeframe === 3600 ? pair?.largeTerm :
        pair?.microTerm
    );

    let container: HTMLDivElement;
    let chart: IChartApi;
    let ro: ResizeObserver;
    let rvolSeries: ISeriesApi<'Histogram'>;

    function rvolColor(rvol: number): string {
        if (rvol >= 3.0) return '#e040fb';
        if (rvol >= 1.5) return '#26c6da';
        if (rvol < 1.0) return 'rgba(143, 146, 157, 0.25)';
        return '#3b82f6';
    }

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

        rvolSeries = chart.addSeries(HistogramSeries, {
            color: '#3b82f6',
            base: 0,
            priceLineVisible: false
        });

        rvolSeries.createPriceLine({
            price: 1.0,
            color: '#4c525e',
            lineWidth: 1,
            lineStyle: 2,
            axisLabelVisible: true,
            title: 'CONSOLIDATION (1.0)',
        });

        rvolSeries.createPriceLine({
            price: 1.5,
            color: '#26c6da',
            lineWidth: 1,
            lineStyle: 2,
            axisLabelVisible: true,
            title: 'INSTITUTIONAL (1.5)',
        });

        registerChart(chart);

        if (onDoubleClick) chart.subscribeDblClick(onDoubleClick);

        if (onScreenshotReady) {
            onScreenshotReady(() => {
                if (!chart) return;
                const canvas = chart.takeScreenshot();
                const dataUrl = canvas.toDataURL('image/png');
                const link = document.createElement('a');
                link.download = `${pairKey}_${timeframe}s_rvol.png`;
                link.href = dataUrl;
                link.click();
            });
        }

        (async () => {
            if (!pair) return;
            try {
                const res = await fetch(`/api/history?symbol=${encodeURIComponent(pairKey)}&timeframe_secs=${timeframe}`);
                const data = await res.json();
                const ih = data.indicator_history;
                if (ih && ih.rvol && ih.rvol.length > 0) {
                    const rawRvolData = ih.times.map((t: number, i: number) => {
                        const val = parseFloat(ih.rvol[i]) || 0;
                        return {
                            time: t as Time,
                            value: val,
                            color: rvolColor(val),
                        };
                    });

                    const seenTimes = new Set<number>();
                    const cleanedRvolData: { time: Time; value: number; color: string }[] = [];
                    for (const item of rawRvolData) {
                        const tNum = item.time as number;
                        if (item && tNum && !seenTimes.has(tNum)) {
                            seenTimes.add(tNum);
                            cleanedRvolData.push(item);
                        }
                    }
                    cleanedRvolData.sort((a, b) => (a.time as number) - (b.time as number));

                    rvolSeries.setData(cleanedRvolData);
                    chart.timeScale().fitContent();
                }
            } catch (err) {
                console.error("Error bootstrapping RVOL chart history:", err);
            }
        })();

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

    $effect(() => {
        if (!pair) return;
        const snap = tf?.latestSnapshot;
        if (!snap) return;
        const timeSec = snap.timestamp as number;
        if (snap.rvol != null) {
            const val = parseFloat(String(snap.rvol));
            rvolSeries.update({
                time: timeSec as Time,
                value: val,
                color: rvolColor(val)
            });
        }
    });
</script>

<div class="chart-container" bind:this={container}></div>

<style>
    .chart-container { width: 100%; height: 100%; }
</style>
