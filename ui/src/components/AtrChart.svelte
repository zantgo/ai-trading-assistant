<script lang="ts">
    import { iRaw, atrVolatilityRegime } from '../lib/telemetry';
    import type { IndicatorMap } from '../types';
    import { fetchChartHistoryOnce, dedupSortByTime } from '../lib/chartHistory';
    import { onMount, onDestroy } from 'svelte';
    import { createChart, CrosshairMode, LineSeries } from 'lightweight-charts';
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
    let atrSeries: ISeriesApi<'Line'>;
    let atrVal = $state(0);
    let atrRegime = $state('stable');

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

        atrSeries = chart.addSeries(LineSeries, { color: '#8f929d', lineWidth: 2, priceLineVisible: false });

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
                link.download = `${pairKey}_${timeframe}s_atr.png`;
                link.href = dataUrl;
                link.click();
            });
        }

        (async () => {
            if (!pair) return;
            try {
                const data = await fetchChartHistoryOnce(pairKey, timeframe);
                if (!data || !data.indicatorHistory || !data.indicatorHistory.atr_14.length) return;
                const ih = data.indicatorHistory;
                const rawAtrData = ih.times.map((t: number, i: number) => {
                    const val = ih.atr_14[i];
                    if (val == null) return null;
                    return {
                        time: t as Time,
                        value: parseFloat(val)
                    };
                }).filter((x): x is { time: Time; value: number } => x != null);

                const cleanedAtrData = dedupSortByTime(rawAtrData);

                if (cleanedAtrData.length > 0) {
                    atrSeries.setData(cleanedAtrData);
                    chart.timeScale().fitContent();
                }
            } catch (err) {
                console.error("Error bootstrapping ATR chart history:", err);
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

    function regimeColor(regime: string): string {
        switch (regime) {
            case 'expanding': return '#10b981';
            case 'contracting': return '#ef4444';
            default: return '#8f929d';
        }
    }

    function regimeLabel(regime: string): string {
        switch (regime) {
            case 'expanding': return 'EXPANDING';
            case 'contracting': return 'CONTRACTING';
            default: return 'STABLE';
        }
    }

    $effect(() => {
        if (!pair) return;
        const snap = tf?.latestSnapshot;
        if (!snap) return;
        const timeSec = snap.timestamp as number;
        const m = (snap.indicators ?? {}) as IndicatorMap;
        const val = iRaw(m, 'atr');
        if (val != null) {
            atrSeries.update({ time: timeSec as Time, value: val });
            atrVal = val;

            const regime = atrVolatilityRegime(m);
            atrRegime = regime;

            const color = regimeColor(regime);
            atrSeries.applyOptions({ color });
        }
    });
</script>

<div class="chart-container" bind:this={container}></div>

<style>
    .chart-container { width: 100%; height: 100%; }
</style>
