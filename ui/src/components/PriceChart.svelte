<script lang="ts">
    import { flattenHistory } from '../lib/historyAdapter';
    import { emaStackState, vwapBias, iSub } from '../lib/telemetry';
    import type { IndicatorMap } from '../types';
    import { onMount, onDestroy } from 'svelte';
    import { createChart, CrosshairMode, CandlestickSeries, LineSeries, LineStyle } from 'lightweight-charts';
    import type { IChartApi, ISeriesApi, Time } from 'lightweight-charts';
    import { useAppStore } from '../state.svelte';
    import { registerChart, unregisterChart } from '../chartRegistry.svelte';
    import { fetchChartHistoryOnce } from '../lib/chartHistory';
    import styles from './PriceChart.module.css';

    const app = useAppStore();
    let { pairKey, slot, onDoubleClick, onScreenshotReady }: { pairKey: string; slot: 'micro' | 'fast' | 'slow' | 'macro'; onDoubleClick?: () => void; onScreenshotReady?: (fn: () => void) => void } = $props();
    const pair = $derived(app.instancesMap[pairKey]);
    // Slot identity is positional; never re-derive from duration.
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
    let candleSeries: ISeriesApi<'Candlestick'>;
    let ema10Series: ISeriesApi<'Line'>;
    let ema50Series: ISeriesApi<'Line'>;
    let ema100Series: ISeriesApi<'Line'>;
    let ema200Series: ISeriesApi<'Line'>;
    let bbUpperSeries: ISeriesApi<'Line'>;
    let bbMiddleSeries: ISeriesApi<'Line'>;
    let bbLowerSeries: ISeriesApi<'Line'>;
    let vwapSeries: ISeriesApi<'Line'>;
    let priceLineSeries: ISeriesApi<'Line'>;

    onMount(() => {
        chart = createChart(container, {
            autoSize: true,
            layout: { background: { color: '#131722' }, textColor: '#8f929d', fontSize: 10 },
            grid: { vertLines: { color: '#1a1d26' }, horzLines: { color: '#1a1d26' } },
            crosshair: { mode: CrosshairMode.Normal, vertLine: { color: '#4c525e', width: 1, style: 3 }, horzLine: { color: '#4c525e', width: 1, style: 3 } },
            rightPriceScale: { borderColor: '#2a2e39', scaleMargins: { top: 0.15, bottom: 0.1 } },
            timeScale: { borderColor: '#2a2e39', visible: true, timeVisible: true, secondsVisible: true },
            handleScale: true,
            handleScroll: true,
        });

        candleSeries = chart.addSeries(CandlestickSeries, {
            upColor: '#26a69a', downColor: '#ef5350', borderVisible: false,
            wickUpColor: '#26a69a', wickDownColor: '#ef5350'
        });

        ema10Series = chart.addSeries(LineSeries, { color: '#fdd835', lineWidth: 1.0, lineStyle: LineStyle.Solid, priceLineVisible: false, crosshairMarkerVisible: false });
        ema50Series = chart.addSeries(LineSeries, { color: '#ff9800', lineWidth: 1.0, lineStyle: LineStyle.Solid, priceLineVisible: false, crosshairMarkerVisible: false });
        ema100Series = chart.addSeries(LineSeries, { color: '#ef5350', lineWidth: 1.0, lineStyle: LineStyle.Solid, priceLineVisible: false, crosshairMarkerVisible: false });
        ema200Series = chart.addSeries(LineSeries, { color: '#9c27b0', lineWidth: 1.0, lineStyle: LineStyle.Solid, priceLineVisible: false, crosshairMarkerVisible: false });
        bbUpperSeries = chart.addSeries(LineSeries, { color: '#00e5ff', lineWidth: 1.0, lineStyle: LineStyle.Dotted, priceLineVisible: false, crosshairMarkerVisible: false });
        bbMiddleSeries = chart.addSeries(LineSeries, { color: '#00e5ff', lineWidth: 1.0, lineStyle: LineStyle.Dotted, priceLineVisible: false, crosshairMarkerVisible: false });
        bbLowerSeries = chart.addSeries(LineSeries, { color: '#00e5ff', lineWidth: 1.0, lineStyle: LineStyle.Dotted, priceLineVisible: false, crosshairMarkerVisible: false });
        vwapSeries = chart.addSeries(LineSeries, { color: '#2962ff', lineWidth: 1, lineStyle: LineStyle.Dotted, priceLineVisible: false, crosshairMarkerVisible: false });
        priceLineSeries = chart.addSeries(LineSeries, { color: '#ffffff', lineWidth: 1, lineStyle: LineStyle.Solid, priceLineVisible: false, crosshairMarkerVisible: false, lastValueVisible: false });

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
                link.download = `${pairKey}_${timeframe}s_price.png`;
                link.href = dataUrl;
                link.click();
            });
        }

        (async () => {
            if (!pair) return;
            try {
                const data = await fetchChartHistoryOnce(pairKey, timeframe);
                if (!data) return;
                if (data.prices && data.prices.length > 0) {
                    const now = Math.floor(Date.now() / 1000);
                    const step = tf?.barDurationSec || 60;
                    const baseTime = now - (data.prices.length * step);

                    const hasCandles = data.candles && data.candles.length > 0;

                    const rawCandles = data.prices.map((priceStr: string, idx: number) => {
                        if (hasCandles && data.candles[idx]) {
                            const c = data.candles[idx];
                            return {
                                time: (c.time / 1000) as Time,
                                open: parseFloat(c.open) || 0,
                                high: parseFloat(c.high) || 0,
                                low: parseFloat(c.low) || 0,
                                close: parseFloat(c.close) || 0
                            };
                        }
                        const val = parseFloat(priceStr) || 0;
                        return {
                            time: (baseTime + (idx * step)) as Time,
                            open: val,
                            high: val,
                            low: val,
                            close: val
                        };
                    });

                    const seenTimes = new Set<number>();
                    const historicalCandles: { time: Time; open: number; high: number; low: number; close: number }[] = [];
                    for (const candle of rawCandles) {
                        const tNum = Number(candle.time);
                        if (candle && tNum && !seenTimes.has(tNum)) {
                            seenTimes.add(tNum);
                            historicalCandles.push(candle);
                        }
                    }
                    historicalCandles.sort((a, b) => Number(a.time) - Number(b.time));

                    candleSeries.setData(historicalCandles);
                    priceLineSeries.setData(
                        historicalCandles.map((c: any) => ({ time: c.time, value: c.close }))
                    );
                    chart.timeScale().fitContent();

                    const ind = data.indicatorHistory;
                    if (ind) {
                        const mapIndicator = (arr: (string | null)[] | undefined) => {
                            if (!arr) return [];
                            const raw = arr
                                .map((val, i) => {
                                    if (val != null && ind.times[i] != null) {
                                        return {
                                            time: ind.times[i] as Time,
                                            value: parseFloat(val)
                                        };
                                    }
                                    return null;
                                })
                                .filter((item): item is { time: Time; value: number } => item !== null);

                            const seen = new Set<number>();
                            const cleaned: { time: Time; value: number }[] = [];
                            for (const item of raw) {
                                const t = item.time as number;
                                if (!seen.has(t)) {
                                    seen.add(t);
                                    cleaned.push(item);
                                }
                            }
                            cleaned.sort((a, b) => (a.time as number) - (b.time as number));
                            return cleaned;
                        };
                        ema10Series.setData(mapIndicator(ind.ema_fast));
                        ema50Series.setData(mapIndicator(ind.ema_medium));
                        ema100Series.setData(mapIndicator(ind.ema_slow));
                        ema200Series.setData(mapIndicator(ind.ema_long));
                        bbUpperSeries.setData(mapIndicator(ind.bb_upper));
                        bbMiddleSeries.setData(mapIndicator(ind.bb_middle));
                        bbLowerSeries.setData(mapIndicator(ind.bb_lower));
                        vwapSeries.setData(mapIndicator(ind.vwap));
                    }
                }
            } catch (err) {
                console.error("Error bootstrapping price chart history:", err);
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
        if (!ema10Series || !ema50Series || !ema100Series || !ema200Series || !pair) return;
        ema10Series.applyOptions({ visible: pair.showEmaFast });
        ema50Series.applyOptions({ visible: pair.showEmaMedium });
        ema100Series.applyOptions({ visible: pair.showEmaSlow });
        ema200Series.applyOptions({ visible: pair.showEmaLong });
    });

    $effect(() => {
        if (!candleSeries || !priceLineSeries || !pair) return;
        candleSeries.applyOptions({ visible: !pair.priceLineMode });
        priceLineSeries.applyOptions({ visible: pair.priceLineMode });
    });

    $effect(() => {
        if (!bbUpperSeries || !bbMiddleSeries || !bbLowerSeries || !pair || !tf) return;
        bbUpperSeries.applyOptions({ visible: tf.showBb });
        bbMiddleSeries.applyOptions({ visible: tf.showBb });
        bbLowerSeries.applyOptions({ visible: tf.showBb });
    });

    $effect(() => {
        if (!vwapSeries || !pair || !tf) return;
        vwapSeries.applyOptions({ visible: tf.showVwap });
    });

    $effect(() => {
        if (!pair) return;
        const snap = tf?.latestSnapshot;
        if (!snap) return;
        const timeSec = snap.timestamp as number;
        const m = (snap.indicators ?? {}) as IndicatorMap;

        if (snap.open != null && snap.high != null && snap.low != null && snap.close != null) {
            candleSeries.update({
                time: timeSec as Time,
                open: parseFloat(String(snap.open)),
                high: parseFloat(String(snap.high)),
                low: parseFloat(String(snap.low)),
                close: parseFloat(String(snap.close))
            });
            priceLineSeries.update({
                time: timeSec as Time,
                value: parseFloat(String(snap.close))
            });
        }

        const emaFast = iSub(m, 'ema_stack', 'fast');
        const emaMedium = iSub(m, 'ema_stack', 'medium');
        const emaSlow = iSub(m, 'ema_stack', 'slow');
        const emaLong = iSub(m, 'ema_stack', 'long');
        const bbUpper = iSub(m, 'bollinger', 'upper');
        const bbMiddle = iSub(m, 'bollinger', 'middle');
        const bbLower = iSub(m, 'bollinger', 'lower');
        const vwapVal = iSub(m, 'vwap', 'vwap');

        if (emaFast != null) ema10Series.update({ time: timeSec as Time, value: emaFast });
        if (emaMedium != null) ema50Series.update({ time: timeSec as Time, value: emaMedium });
        if (emaSlow != null) ema100Series.update({ time: timeSec as Time, value: emaSlow });
        if (emaLong != null) ema200Series.update({ time: timeSec as Time, value: emaLong });
        if (bbUpper != null) bbUpperSeries.update({ time: timeSec as Time, value: bbUpper });
        if (bbMiddle != null) bbMiddleSeries.update({ time: timeSec as Time, value: bbMiddle });
        if (bbLower != null) bbLowerSeries.update({ time: timeSec as Time, value: bbLower });
        if (vwapVal != null) vwapSeries.update({ time: timeSec as Time, value: vwapVal });
    });
</script>

<div class={styles.chartWrapper}>
    {#if pair && tf}
        {@const emaStack = emaStackState(tf.indicators)}
        {@const vBias = vwapBias(tf.indicators)}
        <span class="{styles.emaStackLabel} {emaStack === 'bullish' ? styles.bullish : ''} {emaStack === 'bearish' ? styles.bearish : ''}">
            {emaStack.toUpperCase()}
        </span>
        <span class="{styles.vwapBiasLabel} {vBias === 'premium' ? styles.premium : ''} {vBias === 'discount' ? styles.discount : ''}">
            VWAP: {vBias.toUpperCase()}
        </span>
    {/if}
    <div class={styles.chartContainer} bind:this={container}></div>
</div>
