<script lang="ts">
    import { flattenHistory } from '../lib/historyAdapter';
    import { emaStackState, vwapBias, divStatus, iSub, getPriceFormat, getDecimalCount } from '../lib/telemetry';
    import type { IndicatorMap } from '../types';
    import { onMount, onDestroy } from 'svelte';
    import { createChart, CrosshairMode, CandlestickSeries, LineSeries, LineStyle, createSeriesMarkers } from 'lightweight-charts';
    import type { IChartApi, ISeriesApi, IPriceLine, Time } from 'lightweight-charts';
    import { useAppStore } from '../state.svelte';
    import { registerChart, unregisterChart } from '../chartRegistry.svelte';
    import styles from './PriceChart.module.css';
    import { buildTradeMarkers } from '../lib/tradeMarkerHelper';

    const app = useAppStore();
    let { pairKey, timeframe = 60, onDoubleClick, onScreenshotReady }: {
        pairKey: string;
        timeframe?: number;
        onDoubleClick?: () => void;
        onScreenshotReady?: (fn: () => void) => void;
    } = $props();
    const pair = $derived(app.instancesMap[pairKey]);
    const tf = $derived(
        timeframe === pair?.fastTerm?.barDurationSec ? pair?.fastTerm :
        timeframe === pair?.slowTerm?.barDurationSec ? pair?.slowTerm :
        timeframe === pair?.macroTerm?.barDurationSec ? pair?.macroTerm :
        pair?.microTerm
    );

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
    let supertrendSeries: ISeriesApi<'Line'>;
    let keltnerUpperSeries: ISeriesApi<'Line'>;
    let keltnerMiddleSeries: ISeriesApi<'Line'>;
    let keltnerLowerSeries: ISeriesApi<'Line'>;
    let donchianUpperSeries: ISeriesApi<'Line'>;
    let donchianMiddleSeries: ISeriesApi<'Line'>;
    let donchianLowerSeries: ISeriesApi<'Line'>;

    let supportLines: IPriceLine[] = [];
    let resistanceLines: IPriceLine[] = [];
    let divergenceLines: IPriceLine[] = [];
    let fibGpTopLine: IPriceLine | null = null;
    let fibGpBottomLine: IPriceLine | null = null;
    let fibExt1618Line: IPriceLine | null = null;
    let fibExt2618Line: IPriceLine | null = null;
    let entryLine: IPriceLine | null = null;
    let stopLossLine: IPriceLine | null = null;
    let markersApi: any = null;

    // Track the active price-scale precision so we only reconfigure the chart
    // series when the asset crosses a decimal tier (not on every tick).
    let lastPriceDecimals = -1;
    function applyPriceScale(refPrice: number): void {
        if (!candleSeries || refPrice <= 0) return;
        const decimals = getDecimalCount(refPrice);
        if (decimals === lastPriceDecimals) return;
        lastPriceDecimals = decimals;
        const priceFormat = getPriceFormat(refPrice);
        for (const s of [
            candleSeries, priceLineSeries,
            ema10Series, ema50Series, ema100Series, ema200Series,
            bbUpperSeries, bbMiddleSeries, bbLowerSeries, vwapSeries,
            supertrendSeries,
            keltnerUpperSeries, keltnerMiddleSeries, keltnerLowerSeries,
            donchianUpperSeries, donchianMiddleSeries, donchianLowerSeries,
        ]) {
            s?.applyOptions({ priceFormat });
        }
    }

    async function updateMarkers() {
        if (!candleSeries || !markersApi || !pair) return;
        const markers = await buildTradeMarkers(pairKey, pair.symbol, tf?.barDurationSec || 60);
        markersApi.setMarkers(markers);
    }

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

        markersApi = createSeriesMarkers(candleSeries, []);

        ema10Series = chart.addSeries(LineSeries, { color: '#fdd835', lineWidth: 1.0, lineStyle: LineStyle.Solid, priceLineVisible: false, crosshairMarkerVisible: false });
        ema50Series = chart.addSeries(LineSeries, { color: '#ff9800', lineWidth: 1.0, lineStyle: LineStyle.Solid, priceLineVisible: false, crosshairMarkerVisible: false });
        ema100Series = chart.addSeries(LineSeries, { color: '#ef5350', lineWidth: 1.0, lineStyle: LineStyle.Solid, priceLineVisible: false, crosshairMarkerVisible: false });
        ema200Series = chart.addSeries(LineSeries, { color: '#9c27b0', lineWidth: 1.0, lineStyle: LineStyle.Solid, priceLineVisible: false, crosshairMarkerVisible: false });
        bbUpperSeries = chart.addSeries(LineSeries, { color: '#00e5ff', lineWidth: 1.0, lineStyle: LineStyle.Dotted, priceLineVisible: false, crosshairMarkerVisible: false });
        bbMiddleSeries = chart.addSeries(LineSeries, { color: '#00e5ff', lineWidth: 1.0, lineStyle: LineStyle.Dotted, priceLineVisible: false, crosshairMarkerVisible: false });
        bbLowerSeries = chart.addSeries(LineSeries, { color: '#00e5ff', lineWidth: 1.0, lineStyle: LineStyle.Dotted, priceLineVisible: false, crosshairMarkerVisible: false });
        vwapSeries = chart.addSeries(LineSeries, { color: '#2962ff', lineWidth: 1, lineStyle: LineStyle.Dotted, priceLineVisible: false, crosshairMarkerVisible: false });
        supertrendSeries = chart.addSeries(LineSeries, { color: '#26a69a', lineWidth: 2, lineStyle: LineStyle.Solid, priceLineVisible: false, crosshairMarkerVisible: false });
        keltnerUpperSeries = chart.addSeries(LineSeries, { color: '#78909c', lineWidth: 1, lineStyle: LineStyle.Dashed, priceLineVisible: false, crosshairMarkerVisible: false });
        keltnerMiddleSeries = chart.addSeries(LineSeries, { color: '#78909c', lineWidth: 1, lineStyle: LineStyle.Dashed, priceLineVisible: false, crosshairMarkerVisible: false });
        keltnerLowerSeries = chart.addSeries(LineSeries, { color: '#78909c', lineWidth: 1, lineStyle: LineStyle.Dashed, priceLineVisible: false, crosshairMarkerVisible: false });
        donchianUpperSeries = chart.addSeries(LineSeries, { color: '#ec407a', lineWidth: 1, lineStyle: LineStyle.Solid, priceLineVisible: false, crosshairMarkerVisible: false });
        donchianMiddleSeries = chart.addSeries(LineSeries, { color: '#ec407a', lineWidth: 1, lineStyle: LineStyle.Dotted, priceLineVisible: false, crosshairMarkerVisible: false });
        donchianLowerSeries = chart.addSeries(LineSeries, { color: '#ec407a', lineWidth: 1, lineStyle: LineStyle.Solid, priceLineVisible: false, crosshairMarkerVisible: false });
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
                const res = await fetch(`/api/history?symbol=${encodeURIComponent(pairKey)}&timeframe_secs=${timeframe}`);
                const data = await res.json();
                if (data.prices && data.prices.length > 0) {
                    const now = Math.floor(Date.now() / 1000);
                    const step = tf.barDurationSec || 60;
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

                    // Deduplicate and sort timestamps to prevent lightweight-charts rendering crashes
                    const seenTimes = new Set<number>();
                    const historicalCandles: { time: Time; open: number; high: number; low: number; close: number }[] = [];
                    for (const candle of rawCandles) {
                        if (candle && candle.time && !seenTimes.has(candle.time)) {
                            seenTimes.add(candle.time);
                            historicalCandles.push(candle);
                        }
                    }
                    historicalCandles.sort((a, b) => (a.time as number) - (b.time as number));

                    candleSeries.setData(historicalCandles);
                    priceLineSeries.setData(
                        historicalCandles.map((c: any) => ({ time: c.time, value: c.close }))
                    );
                    chart.timeScale().fitContent();

                    const lastClose = historicalCandles[historicalCandles.length - 1]?.close;
                    if (lastClose != null) applyPriceScale(lastClose);

                    const ind = flattenHistory(data.indicator_history);
                    if (ind) {
                        const mapIndicator = (arr: (string | null)[] | undefined) => {
                            if (!arr) return [];
                            // Align each indicator value to its own timestamp in
                            // `ind.times` (NOT the de-duplicated candle array,
                            // whose length differs and causes index drift), then
                            // de-duplicate + sort so lightweight-charts accepts it.
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
                        supertrendSeries.setData(mapIndicator(ind.supertrend));
                        keltnerUpperSeries.setData(mapIndicator(ind.keltner_upper));
                        keltnerMiddleSeries.setData(mapIndicator(ind.keltner_middle));
                        keltnerLowerSeries.setData(mapIndicator(ind.keltner_lower));
                        donchianUpperSeries.setData(mapIndicator(ind.donchian_upper));
                        donchianMiddleSeries.setData(mapIndicator(ind.donchian_middle));
                        donchianLowerSeries.setData(mapIndicator(ind.donchian_lower));
                    }
                }
            } catch (err) {
                console.error("Error bootstrapping price chart history:", err);
            }

            updateMarkers();
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
        if (!supertrendSeries || !pair || !tf) return;
        supertrendSeries.applyOptions({ visible: tf.showSupertrend });
    });

    $effect(() => {
        if (!keltnerUpperSeries || !keltnerMiddleSeries || !keltnerLowerSeries || !pair || !tf) return;
        keltnerUpperSeries.applyOptions({ visible: tf.showKeltner });
        keltnerMiddleSeries.applyOptions({ visible: tf.showKeltner });
        keltnerLowerSeries.applyOptions({ visible: tf.showKeltner });
    });

    $effect(() => {
        if (!donchianUpperSeries || !donchianMiddleSeries || !donchianLowerSeries || !pair || !tf) return;
        donchianUpperSeries.applyOptions({ visible: tf.showDonchian });
        donchianMiddleSeries.applyOptions({ visible: tf.showDonchian });
        donchianLowerSeries.applyOptions({ visible: tf.showDonchian });
    });

    $effect(() => {
        if (!pair) return;
        const snap = tf.latestSnapshot;
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
            applyPriceScale(parseFloat(String(snap.close)));
        }

        // v2.0 nested indicator map — the flat top-level fields no longer exist.
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

        const stLine = iSub(m, 'supertrend', 'line');
        const stDir = iSub(m, 'supertrend', 'direction');
        if (stLine != null) {
            supertrendSeries.update({ time: timeSec as Time, value: stLine });
            supertrendSeries.applyOptions({ color: (stDir ?? 1) >= 0 ? '#26a69a' : '#ef5350' });
        }
        const kU = iSub(m, 'keltner', 'upper');
        const kM = iSub(m, 'keltner', 'middle');
        const kL = iSub(m, 'keltner', 'lower');
        if (kU != null) keltnerUpperSeries.update({ time: timeSec as Time, value: kU });
        if (kM != null) keltnerMiddleSeries.update({ time: timeSec as Time, value: kM });
        if (kL != null) keltnerLowerSeries.update({ time: timeSec as Time, value: kL });
        const dU = iSub(m, 'donchian', 'upper');
        const dM = iSub(m, 'donchian', 'middle');
        const dL = iSub(m, 'donchian', 'lower');
        if (dU != null) donchianUpperSeries.update({ time: timeSec as Time, value: dU });
        if (dM != null) donchianMiddleSeries.update({ time: timeSec as Time, value: dM });
        if (dL != null) donchianLowerSeries.update({ time: timeSec as Time, value: dL });
    });

    // Support level price lines
    $effect(() => {
        if (!candleSeries) return;
        supportLines.forEach(l => candleSeries.removePriceLine(l));
        supportLines = [];
        const supports = pair ? app.markedSupportLevels : [];
        for (const level of supports.slice(0, 2)) {
            const pl = candleSeries.createPriceLine({
                price: level,
                color: '#22c55e',
                lineWidth: 1,
                lineStyle: 2,
                axisLabelVisible: true,
                title: 'S',
            });
            supportLines.push(pl);
        }
    });

    // Resistance level price lines
    $effect(() => {
        if (!candleSeries) return;
        resistanceLines.forEach(l => candleSeries.removePriceLine(l));
        resistanceLines = [];
        const resistances = pair ? app.markedResistanceLevels : [];
        for (const level of resistances.slice(0, 2)) {
            const pl = candleSeries.createPriceLine({
                price: level,
                color: '#ef4444',
                lineWidth: 1,
                lineStyle: 2,
                axisLabelVisible: true,
                title: 'R',
            });
            resistanceLines.push(pl);
        }
    });

    // Entry price line (when position is active)
    $effect(() => {
        if (!candleSeries) return;
        if (entryLine) { candleSeries.removePriceLine(entryLine); entryLine = null; }
        const pos = pair ? app.activePaperPosition as Record<string, unknown> | null : null;
        if (pos?.entry_price) {
            const price = parseFloat(String(pos.entry_price));
            if (price > 0) {
                entryLine = candleSeries.createPriceLine({
                    price,
                    color: '#60a5fa',
                    lineWidth: 1,
                    lineStyle: 1,
                    axisLabelVisible: true,
                    title: 'Entry',
                });
            }
        }
    });

    // Stop-loss price line
    $effect(() => {
        if (!candleSeries) return;
        if (stopLossLine) { candleSeries.removePriceLine(stopLossLine); stopLossLine = null; }
        const level = pair ? app.paperInvalidationLevel : 0;
        if (level > 0) {
            stopLossLine = candleSeries.createPriceLine({
                price: level,
                color: '#f59e0b',
                lineWidth: 1,
                lineStyle: 3,
                axisLabelVisible: true,
                title: 'SL',
            });
        }
    });

    // Divergence extrema price lines
    $effect(() => {
        if (!candleSeries) return;
        divergenceLines.forEach(l => candleSeries.removePriceLine(l));
        divergenceLines = [];
        if (!pair) return;

        const rsiDivStatus = divStatus(tf.indicators, 'rsi_divergence');
        const parseCoords = (raw: unknown): { firstPrice: number; secondPrice: number; status: string } | null => {
            if (!raw) return null;
            try {
                const parsed = typeof raw === 'string' ? JSON.parse(raw) : raw;
                return {
                    firstPrice: parsed.first_extreme?.price ? parseFloat(parsed.first_extreme.price) : 0,
                    secondPrice: parsed.second_extreme?.price ? parseFloat(parsed.second_extreme.price) : 0,
                    status: rsiDivStatus,
                };
            } catch (_) {
                return null;
            }
        };

        const rsiCoords = parseCoords((tf.latestSnapshot as Record<string, unknown> | null)?.rsi_divergence_coords ?? null);
        if (rsiCoords && rsiCoords.firstPrice > 0 && rsiCoords.secondPrice > 0) {
            const isConfirmed = rsiDivStatus === 'confirmed';
            const lineColor = isConfirmed ? '#22c55e' : '#f59e0b';
            const lineStyle: 0 | 1 | 2 | 3 | 4 = isConfirmed ? 1 : 2;
            divergenceLines.push(candleSeries.createPriceLine({
                price: rsiCoords.firstPrice,
                color: lineColor,
                lineWidth: 1,
                lineStyle,
                axisLabelVisible: true,
                title: 'RSI↓1',
            }));
            divergenceLines.push(candleSeries.createPriceLine({
                price: rsiCoords.secondPrice,
                color: lineColor,
                lineWidth: 1,
                lineStyle,
                axisLabelVisible: true,
                title: 'RSI↓2',
            }));
        }
    });

    // Fibonacci Golden Pocket + Extension lines
    $effect(() => {
        if (!candleSeries) return;
        if (fibGpTopLine) { candleSeries.removePriceLine(fibGpTopLine); fibGpTopLine = null; }
        if (fibGpBottomLine) { candleSeries.removePriceLine(fibGpBottomLine); fibGpBottomLine = null; }
        if (fibExt1618Line) { candleSeries.removePriceLine(fibExt1618Line); fibExt1618Line = null; }
        if (fibExt2618Line) { candleSeries.removePriceLine(fibExt2618Line); fibExt2618Line = null; }
        if (!pair || !tf.showFib) return;

        const snap = tf.latestSnapshot;
        if (!snap) return;

        const fm = (snap.indicators ?? {}) as IndicatorMap;
        const gpLow = iSub(fm, 'fibonacci', 'gp_bottom');
        const gpHigh = iSub(fm, 'fibonacci', 'gp_top');
        const ext1618 = iSub(fm, 'fibonacci', 'ext_1618');
        const ext2618 = iSub(fm, 'fibonacci', 'ext_2618');

        if (gpHigh != null && gpHigh > 0) {
            fibGpTopLine = candleSeries.createPriceLine({
                price: gpHigh,
                color: '#f1c40f',
                lineWidth: 1,
                lineStyle: 2,
                axisLabelVisible: true,
                title: 'GP 61.8%',
            });
        }
        if (gpLow != null && gpLow > 0) {
            fibGpBottomLine = candleSeries.createPriceLine({
                price: gpLow,
                color: '#f1c40f',
                lineWidth: 1,
                lineStyle: 2,
                axisLabelVisible: true,
                title: 'GP 66.0%',
            });
        }
        if (ext1618 != null && ext1618 > 0) {
            fibExt1618Line = candleSeries.createPriceLine({
                price: ext1618,
                color: '#4caf50',
                lineWidth: 1,
                lineStyle: 1,
                axisLabelVisible: true,
                title: '1.618 Ext',
            });
        }
        if (ext2618 != null && ext2618 > 0) {
            fibExt2618Line = candleSeries.createPriceLine({
                price: ext2618,
                color: '#00e676',
                lineWidth: 1,
                lineStyle: 1,
                axisLabelVisible: true,
                title: '2.618 Ext',
            });
        }
    });

    $effect(() => {
        const _pos = app.activePaperPosition;
        const _tab = app.activeTab;
        const _tf = timeframe;
        void _pos; void _tab; void _tf;

        if (candleSeries && pair) {
            updateMarkers();
        }
    });
</script>

<div class={styles.chartWrapper}>
    {#if pair}
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
