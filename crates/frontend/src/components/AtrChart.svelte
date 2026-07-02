<script lang="ts">
    import { iRaw, atrVolatilityRegime } from '../lib/telemetry';
    import type { IndicatorMap } from '../types';
    import { flattenHistory } from '../lib/historyAdapter';
    import { onMount, onDestroy } from 'svelte';
    import { createChart, CrosshairMode, LineSeries } from 'lightweight-charts';
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
                const res = await fetch(`/api/history?symbol=${encodeURIComponent(pairKey)}&timeframe_secs=${timeframe}`);
                const data = await res.json();
                const indicatorHistory = flattenHistory(data.indicator_history);
                if (indicatorHistory && indicatorHistory.atr_14 && indicatorHistory.atr_14.length > 0) {
                    const rawAtrData = indicatorHistory.times.map((t: number, i: number) => {
                        const val = indicatorHistory.atr_14[i];
                        return {
                            time: t as Time,
                            value: val != null ? parseFloat(val) : 0
                        };
                    });

                    const seenTimes = new Set<number>();
                    const cleanedAtrData: { time: Time; value: number }[] = [];
                    for (const item of rawAtrData) {
                        const tNum = item.time as number;
                        if (item && tNum && !seenTimes.has(tNum)) {
                            seenTimes.add(tNum);
                            cleanedAtrData.push(item);
                        }
                    }
                    cleanedAtrData.sort((a, b) => (a.time as number) - (b.time as number));

                    atrSeries.setData(cleanedAtrData);
                    chart.timeScale().fitContent();
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
                            placeholder.push({ time: tVal as Time, value: 0 });
                        }
                    }
                    placeholder.sort((a, b) => (a.time as number) - (b.time as number));

                    atrSeries.setData(placeholder);
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
            case 'expanding': return '#10b981';  // Bright Green
            case 'contracting': return '#ef4444'; // Dark Red
            default: return '#8f929d';             // Gray (stable)
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
        const snap = tf.latestSnapshot;
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

<div class="atr-pane">
    <div class="chart-container" bind:this={container}></div>
</div>

<style>
    .atr-pane { display: flex; flex-direction: column; height: 100%; }
    .chart-container { flex: 1; width: 100%; min-height: 0; }
</style>
