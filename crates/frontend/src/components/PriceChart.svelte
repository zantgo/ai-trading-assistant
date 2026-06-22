<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { createChart, CrosshairMode, CandlestickSeries, LineSeries, LineStyle } from 'lightweight-charts';
    import type { IChartApi, ISeriesApi, IPriceLine, Time } from 'lightweight-charts';
    import { getState } from '../state.svelte';
    import { registerChart, unregisterChart } from '../chartRegistry.svelte';

    const app = getState();
    let { pairKey, timeframe = 60 }: { pairKey: string; timeframe?: number } = $props();
    const pair = $derived(app.instancesMap[pairKey]);
    const tf = $derived(
        timeframe === 300 ? pair?.smallTerm :
        timeframe === 900 ? pair?.mediumTerm :
        timeframe === 3600 ? pair?.largeTerm :
        pair?.microTerm
    );

    let container: HTMLDivElement;
    let chart: IChartApi;
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

    let supportLines: IPriceLine[] = [];
    let resistanceLines: IPriceLine[] = [];
    let divergenceLines: IPriceLine[] = [];
    let fibGpTopLine: IPriceLine | null = null;
    let fibGpBottomLine: IPriceLine | null = null;
    let fibExt1618Line: IPriceLine | null = null;
    let fibExt2618Line: IPriceLine | null = null;
    let entryLine: IPriceLine | null = null;
    let stopLossLine: IPriceLine | null = null;

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

                    const historicalCandles = data.prices.map((priceStr: string, idx: number) => {
                        if (hasCandles && data.candles[idx]) {
                            const c = data.candles[idx];
                            return {
                                time: (c.time / 1000) as Time,
                                open: parseFloat(c.open),
                                high: parseFloat(c.high),
                                low: parseFloat(c.low),
                                close: parseFloat(c.close)
                            };
                        }
                        const val = parseFloat(priceStr);
                        return {
                            time: (baseTime + (idx * step)) as Time,
                            open: val,
                            high: val,
                            low: val,
                            close: val
                        };
                    });

                    candleSeries.setData(historicalCandles);
                    priceLineSeries.setData(
                        historicalCandles.map((c: any) => ({ time: c.time, value: c.close }))
                    );
                    chart.timeScale().fitContent();

                    const ind = data.indicator_history;
                    if (ind) {
                        const mapIndicator = (arr: (string | null)[] | undefined) =>
                            arr?.map((val, i) => val != null ? { time: historicalCandles[i].time, value: parseFloat(val) } : null)
                               .filter(Boolean) ?? [];
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
        if (!bbUpperSeries || !bbMiddleSeries || !bbLowerSeries || !pair) return;
        bbUpperSeries.applyOptions({ visible: tf.showBb });
        bbMiddleSeries.applyOptions({ visible: tf.showBb });
        bbLowerSeries.applyOptions({ visible: tf.showBb });
    });

    $effect(() => {
        if (!vwapSeries || !pair) return;
        vwapSeries.applyOptions({ visible: tf.showVwap });
    });

    $effect(() => {
        if (!pair) return;
        const snap = tf.latestSnapshot;
        if (!snap) return;
        const timeSec = snap.timestamp as number;

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

        if (snap.ema_fast) ema10Series.update({ time: timeSec as Time, value: parseFloat(String(snap.ema_fast)) });
        if (snap.ema_medium) ema50Series.update({ time: timeSec as Time, value: parseFloat(String(snap.ema_medium)) });
        if (snap.ema_slow) ema100Series.update({ time: timeSec as Time, value: parseFloat(String(snap.ema_slow)) });
        if (snap.ema_long) ema200Series.update({ time: timeSec as Time, value: parseFloat(String(snap.ema_long)) });
        if (snap.bb_upper) bbUpperSeries.update({ time: timeSec as Time, value: parseFloat(String(snap.bb_upper)) });
        if (snap.bb_middle) bbMiddleSeries.update({ time: timeSec as Time, value: parseFloat(String(snap.bb_middle)) });
        if (snap.bb_lower) bbLowerSeries.update({ time: timeSec as Time, value: parseFloat(String(snap.bb_lower)) });
        if (snap.vwap) vwapSeries.update({ time: timeSec as Time, value: parseFloat(String(snap.vwap)) });
    });

    // Support level price lines
    $effect(() => {
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
        if (stopLossLine) { candleSeries.removePriceLine(stopLossLine); stopLossLine = null; }
        const level = pair ? pair.paperInvalidationLevel : 0;
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
        divergenceLines.forEach(l => candleSeries.removePriceLine(l));
        divergenceLines = [];
        if (!pair) return;

        const parseCoords = (raw: string | null): { firstPrice: number; secondPrice: number; status: string } | null => {
            if (!raw) return null;
            try {
                const parsed = JSON.parse(raw);
                return {
                    firstPrice: parsed.first_extreme?.price ? parseFloat(parsed.first_extreme.price) : 0,
                    secondPrice: parsed.second_extreme?.price ? parseFloat(parsed.second_extreme.price) : 0,
                    status: tf.rsiDivergenceStatus || 'none',
                };
            } catch (_) {
                return null;
            }
        };

        const rsiCoords = parseCoords(tf.rsiDivergenceCoords);
        if (rsiCoords && rsiCoords.firstPrice > 0 && rsiCoords.secondPrice > 0) {
            const isConfirmed = tf.rsiDivergenceStatus === 'confirmed';
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
        if (fibGpTopLine) { candleSeries.removePriceLine(fibGpTopLine); fibGpTopLine = null; }
        if (fibGpBottomLine) { candleSeries.removePriceLine(fibGpBottomLine); fibGpBottomLine = null; }
        if (fibExt1618Line) { candleSeries.removePriceLine(fibExt1618Line); fibExt1618Line = null; }
        if (fibExt2618Line) { candleSeries.removePriceLine(fibExt2618Line); fibExt2618Line = null; }
        if (!pair || !tf.showFib) return;

        const snap = tf.latestSnapshot;
        if (!snap) return;

        const gpLow = snap.fib_golden_pocket_low != null ? parseFloat(String(snap.fib_golden_pocket_low)) : null;
        const gpHigh = snap.fib_golden_pocket_high != null ? parseFloat(String(snap.fib_golden_pocket_high)) : null;
        const ext1618 = snap.fib_extension_1618 != null ? parseFloat(String(snap.fib_extension_1618)) : null;
        const ext2618 = snap.fib_extension_2618 != null ? parseFloat(String(snap.fib_extension_2618)) : null;

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
</script>

<div class="chart-wrapper">
    {#if pair}
        <span class="ema-stack-label"
            class:bullish={tf.emaStackState === 'bullish'}
            class:bearish={tf.emaStackState === 'bearish'}
        >
            {tf.emaStackState?.toUpperCase() || 'TANGLED'}
        </span>
        <span class="vwap-bias-label"
            class:premium={tf.vwapBias === 'premium'}
            class:discount={tf.vwapBias === 'discount'}
        >
            VWAP: {tf.vwapBias?.toUpperCase() || '--'}
        </span>
    {/if}
    <div class="chart-container" bind:this={container}></div>
</div>

<style>
    .chart-wrapper { position: relative; width: 100%; height: 100%; }
    .chart-container { width: 100%; height: 100%; }
    .ema-stack-label {
        position: absolute;
        top: 4px;
        left: 6px;
        z-index: 10;
        font-size: 8px;
        font-weight: 700;
        font-family: 'Courier New', monospace;
        letter-spacing: 0.05em;
        padding: 1px 5px;
        border-radius: 3px;
        background: rgba(15, 17, 26, 0.85);
        color: #8f929d;
        border: 1px solid #4c525e;
    }
    .ema-stack-label.bullish { color: #26a69a; border-color: rgba(38, 166, 154, 0.4); }
    .ema-stack-label.bearish { color: #ef4444; border-color: rgba(239, 68, 68, 0.4); }
    .vwap-bias-label {
        position: absolute;
        top: 4px;
        left: 64px;
        z-index: 10;
        font-size: 8px;
        font-weight: 700;
        font-family: 'Courier New', monospace;
        letter-spacing: 0.05em;
        padding: 1px 5px;
        border-radius: 3px;
        background: rgba(15, 17, 26, 0.85);
        color: #8f929d;
        border: 1px solid #4c525e;
    }
    .vwap-bias-label.premium { color: #ffb300; border-color: rgba(255, 179, 0, 0.4); }
    .vwap-bias-label.discount { color: #ff5252; border-color: rgba(255, 82, 82, 0.4); }
</style>
