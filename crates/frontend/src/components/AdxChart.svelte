<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { createChart, CrosshairMode, LineSeries, LineStyle } from 'lightweight-charts';
    import type { IChartApi, ISeriesApi, IPriceLine, Time } from 'lightweight-charts';
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
    let adxSeries: ISeriesApi<'Line'>;
    let adxPlusSeries: ISeriesApi<'Line'>;
    let adxMinusSeries: ISeriesApi<'Line'>;
    let trendLine: IPriceLine | null = null;
    let exhaustionLine: IPriceLine | null = null;

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

        adxSeries = chart.addSeries(LineSeries, { color: '#f1c40f', lineWidth: 2, lineStyle: LineStyle.Solid, priceLineVisible: false });
        adxPlusSeries = chart.addSeries(LineSeries, { color: '#2ecc71', lineWidth: 1, lineStyle: LineStyle.Solid, priceLineVisible: false });
        adxMinusSeries = chart.addSeries(LineSeries, { color: '#e74c3c', lineWidth: 1, lineStyle: LineStyle.Solid, priceLineVisible: false });

        // Trend threshold line at 20 (dashed gray)
        trendLine = adxSeries.createPriceLine({
            price: 20,
            color: '#4c525e',
            lineWidth: 1,
            lineStyle: LineStyle.Dashed,
            axisLabelVisible: true,
            title: 'TREND',
        });

        // Exhaustion threshold line at 40 (dashed red)
        exhaustionLine = adxSeries.createPriceLine({
            price: 40,
            color: '#ff5252',
            lineWidth: 1,
            lineStyle: LineStyle.Dashed,
            axisLabelVisible: true,
            title: 'EXHAUST',
        });

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
                link.download = `${pairKey}_${timeframe}s_adx.png`;
                link.href = dataUrl;
                link.click();
            });
        }

        (async () => {
            if (!pair) return;
            try {
                const res = await fetch(`/api/history?symbol=${encodeURIComponent(pairKey)}&timeframe_secs=${timeframe}`);
                const data = await res.json();
                const indicatorHistory = data.indicator_history;
                if (indicatorHistory && indicatorHistory.adx_14 && indicatorHistory.adx_14.length > 0) {
                    const rawCombined = indicatorHistory.times.map((t: number, i: number) => ({
                        time: t as Time,
                        adx: indicatorHistory.adx_14[i],
                        plus: indicatorHistory.adx_plus[i],
                        minus: indicatorHistory.adx_minus[i]
                    }));

                    const seenTimes = new Set<number>();
                    const cleanedCombined: { time: Time; adx: string | null; plus: string | null; minus: string | null }[] = [];
                    for (const item of rawCombined) {
                        const tNum = item.time as number;
                        if (item && tNum && !seenTimes.has(tNum)) {
                            seenTimes.add(tNum);
                            cleanedCombined.push(item);
                        }
                    }
                    cleanedCombined.sort((a, b) => (a.time as number) - (b.time as number));

                    const adxData = cleanedCombined.map(x => ({
                        time: x.time,
                        value: x.adx != null ? parseFloat(x.adx) : 0
                    }));
                    const plusData = cleanedCombined.map(x => ({
                        time: x.time,
                        value: x.plus != null ? parseFloat(x.plus) : 0
                    }));
                    const minusData = cleanedCombined.map(x => ({
                        time: x.time,
                        value: x.minus != null ? parseFloat(x.minus) : 0
                    }));

                    adxSeries.setData(adxData);
                    adxPlusSeries.setData(plusData);
                    adxMinusSeries.setData(minusData);
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

                    adxSeries.setData(placeholder);
                    adxPlusSeries.setData(placeholder);
                    adxMinusSeries.setData(placeholder);
                    chart.timeScale().fitContent();
                }
            } catch (err) {
                console.error("Error bootstrapping ADX chart history:", err);
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

    function adxLineColor(val: number, slope: number, regime: string): string {
        if (val > 40) return '#ff5252';         // Pulsing Red — extreme exhaustion
        if (val < 20) return '#4c525e';          // Dull Gray — congestion
        if (slope > 0) return '#f1c40f';          // Bright Yellow/Gold — accelerating
        return '#f97316';                          // Orange — decelerating (above trend)
    }

    $effect(() => {
        if (!pair) return;
        const snap = tf.latestSnapshot;
        if (!snap) return;
        const timeSec = snap.timestamp as number;
        if (snap.adx_14 != null) {
            const adxVal = parseFloat(String(snap.adx_14));
            const slope = snap.adx_slope != null ? parseFloat(String(snap.adx_slope)) : 0;
            const regime = snap.adx_regime != null ? String(snap.adx_regime) as 'congestion' | 'emerging' | 'strong' | 'extreme' : 'congestion';

            adxSeries.update({ time: timeSec as Time, value: adxVal });
            if (snap.adx_plus !== undefined && snap.adx_plus !== null) adxPlusSeries.update({ time: timeSec as Time, value: parseFloat(String(snap.adx_plus)) });
            if (snap.adx_minus !== undefined && snap.adx_minus !== null) adxMinusSeries.update({ time: timeSec as Time, value: parseFloat(String(snap.adx_minus)) });

            // Dynamic ADX line coloring
            const color = adxLineColor(adxVal, slope, regime);
            adxSeries.applyOptions({ color });

            // Update state
            tf.adxSlope = slope;
            tf.adxTrendingRegime = regime;
            tf.adxExhaustionReached = adxVal > 40;
        }
        if (snap.adx_di_crossover_detected != null) {
            tf.adxDiCrossoverDetected = !!snap.adx_di_crossover_detected;
        }
        if (snap.adx_di_crossover_direction != null) {
            tf.adxDiCrossoverDirection = String(snap.adx_di_crossover_direction) as 'BULLISH' | 'BEARISH' | 'NONE';
        }
    });
</script>

<div class="chart-container" bind:this={container}></div>

<style>
    .chart-container { width: 100%; height: 100%; }
</style>
