<script lang="ts">
    import { emaStackState, vwapBias, iSub, iRaw } from '../lib/telemetry';
    import type { IndicatorMap, LiquidationClusterMatrix, VolumeProfileSnapshot } from '../types';
    import { onMount, onDestroy } from 'svelte';
    import { createChart, CrosshairMode, CandlestickSeries, LineSeries, LineStyle } from 'lightweight-charts';
    import type { IChartApi, ISeriesApi, Time } from 'lightweight-charts';
    import { useAppStore } from '../state.svelte';
    import { registerChart, unregisterChart } from '../chartRegistry.svelte';
    import {
        fetchIndicatorHistoryOnce,
        pairsFromHistory,
        alignedSeriesFromHistory,
    } from '../lib/indicatorHistory';
    import { attachVolumeProfile, type VolumeProfilePrimitive } from '../lib/volumeProfile';
    import { attachHeatmap, type LiquidationHeatmapPrimitive } from '../lib/liquidationHeatmap';
    import { attachFvgZones, type FvgZonesPrimitive } from '../lib/fvgZones';
    import { attachOrderBlocks, type OrderBlocksPrimitive } from '../lib/orderBlocks';
    import { makeChartCoalescer } from '../lib/chartCoalesce';
    import { vwapPickKey } from '../lib/vwapAnchor';
    import { createSmcMarkers, type SmcMarkerController } from '../lib/smcMarkers';
    import styles from './PriceChart.module.css';

    const app = useAppStore();
    let { pairKey, slot, onDoubleClick, onScreenshotReady }: { pairKey: string; slot: 'micro' | 'fast' | 'slow' | 'macro'; onDoubleClick?: () => void; onScreenshotReady?: (fn: () => void) => void } = $props();

    /// Number of recent candles + overlay data points seeded at bootstrap.
    /// Bump this to see more history; drop it for faster first paint.
    /// All price overlays (EMA, Bollinger, VWAP, Supertrend, Donchian,
    /// Ichimoku, Keltner, Hull MA, StdDev, PSAR) share the same window so
    /// the candle chart and its indicator lines stay aligned.
    const PRICE_CHART_SEED_COUNT = 1000;
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
    let anchoredVwapSeries: ISeriesApi<'Line'> | null = null;
    let supertrendSeries: ISeriesApi<'Line'> | null = null;
    let donchianUpperSeries: ISeriesApi<'Line'> | null = null;
    let donchianMiddleSeries: ISeriesApi<'Line'> | null = null;
    let donchianLowerSeries: ISeriesApi<'Line'> | null = null;
    let ichimokuTenkanSeries: ISeriesApi<'Line'> | null = null;
    let ichimokuKijunSeries: ISeriesApi<'Line'> | null = null;
    let ichimokuSenkouASeries: ISeriesApi<'Line'> | null = null;
    let ichimokuSenkouBSeries: ISeriesApi<'Line'> | null = null;
    let priceLineSeries: ISeriesApi<'Line'>;
    let volumeProfilePrim: VolumeProfilePrimitive | null = null;
    let liqHeatmapPrim: LiquidationHeatmapPrimitive | null = null;
    let fvgPrim: FvgZonesPrimitive | null = null;
    let obPrim: OrderBlocksPrimitive | null = null;
    let smcMarkers: SmcMarkerController | null = null;
    // Newly-added price-overlay series (toggle-controlled).
    let keltnerUpperSeries: ISeriesApi<'Line'> | null = null;
    let keltnerMiddleSeries: ISeriesApi<'Line'> | null = null;
    let keltnerLowerSeries: ISeriesApi<'Line'> | null = null;
    let stddevUpperSeries: ISeriesApi<'Line'> | null = null;
    let stddevMiddleSeries: ISeriesApi<'Line'> | null = null;
    let stddevLowerSeries: ISeriesApi<'Line'> | null = null;
    let psarSeries: ISeriesApi<'Line'> | null = null;
    // Price-level handles — recreated on level-value change so we can
    // cleanly apply new prices without leaks.
    let fibLines: ReturnType<typeof candleSeries.createPriceLine>[] = [];
    let liqLines: ReturnType<typeof candleSeries.createPriceLine>[] = [];
    let pivotLine: ReturnType<typeof candleSeries.createPriceLine> | null = null;
    let pivotLevelValue = $state<number | null>(null);
    let srLine: ReturnType<typeof candleSeries.createPriceLine> | null = null;
    let srLevelValue = $state<number | null>(null);
    /// v6.5: cluster / volume-profile snapshots fetched via
    /// `/api/history` on first-mount. Used as a **fallback** when the WS
    /// stream hasn't yet populated `tf.cluster` / `tf.volumeProfile`
    /// (i.e. on a fresh daemon restart, before the first per-TF refresh
    /// tick fires).
    let historyCluster: LiquidationClusterMatrix | null = $state(null);
    let historyVolumeProfile: VolumeProfileSnapshot | null = $state(null);

    let _chartReady = $state(false);
    let _bootstrapComplete = $state(false);
    let _lastHistoryTime = $state(-Infinity);

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

        // Volume profile overlay (right-edge stacked buy/sell histogram).
        volumeProfilePrim = attachVolumeProfile(chart, candleSeries);
        // Liquidation heatmap primitive (LIQ HEATMAP toggle, v6.5+).
        // Renders coloured horizontal bands per cluster on the candle
        // pane. Decoupled from the legacy per-cluster `createPriceLine`
        // approach — the price-line path remains wired as a fallback so
        // existing dashboards don't regress if the primitive ever fails
        // to attach.
        liqHeatmapPrim = attachHeatmap(chart, candleSeries);
        // SMC Fair Value Gap zone primitive (toggle-controlled).
        fvgPrim = attachFvgZones(chart, candleSeries);
        // SMC Order Block zone primitive (toggle-controlled).
        obPrim = attachOrderBlocks(chart, candleSeries);

        ema10Series = chart.addSeries(LineSeries, { color: '#fdd835', lineWidth: 1.0, lineStyle: LineStyle.Solid, priceLineVisible: false, crosshairMarkerVisible: false });
        ema50Series = chart.addSeries(LineSeries, { color: '#ff9800', lineWidth: 1.0, lineStyle: LineStyle.Solid, priceLineVisible: false, crosshairMarkerVisible: false });
        ema100Series = chart.addSeries(LineSeries, { color: '#ef5350', lineWidth: 1.0, lineStyle: LineStyle.Solid, priceLineVisible: false, crosshairMarkerVisible: false });
        ema200Series = chart.addSeries(LineSeries, { color: '#9c27b0', lineWidth: 1.0, lineStyle: LineStyle.Solid, priceLineVisible: false, crosshairMarkerVisible: false });
        bbUpperSeries = chart.addSeries(LineSeries, { color: '#00e5ff', lineWidth: 1.0, lineStyle: LineStyle.Dotted, priceLineVisible: false, crosshairMarkerVisible: false });
        bbMiddleSeries = chart.addSeries(LineSeries, { color: '#00e5ff', lineWidth: 1.0, lineStyle: LineStyle.Dotted, priceLineVisible: false, crosshairMarkerVisible: false });
        bbLowerSeries = chart.addSeries(LineSeries, { color: '#00e5ff', lineWidth: 1.0, lineStyle: LineStyle.Dotted, priceLineVisible: false, crosshairMarkerVisible: false });
        vwapSeries = chart.addSeries(LineSeries, { color: '#2962ff', lineWidth: 1, lineStyle: LineStyle.Dotted, priceLineVisible: false, crosshairMarkerVisible: false });
        anchoredVwapSeries = chart.addSeries(LineSeries, { color: '#ffab40', lineWidth: 1, lineStyle: LineStyle.Dotted, priceLineVisible: false, crosshairMarkerVisible: false });
        supertrendSeries = chart.addSeries(LineSeries, { color: '#26a69a', lineWidth: 2, lineStyle: LineStyle.Solid, priceLineVisible: false, crosshairMarkerVisible: false });
        donchianUpperSeries = chart.addSeries(LineSeries, { color: '#ec407a', lineWidth: 1, lineStyle: LineStyle.Dashed, priceLineVisible: false, crosshairMarkerVisible: false });
        donchianMiddleSeries = chart.addSeries(LineSeries, { color: '#ec407a', lineWidth: 1, lineStyle: LineStyle.Solid, priceLineVisible: false, crosshairMarkerVisible: false });
        donchianLowerSeries = chart.addSeries(LineSeries, { color: '#ec407a', lineWidth: 1, lineStyle: LineStyle.Dashed, priceLineVisible: false, crosshairMarkerVisible: false });
        ichimokuTenkanSeries = chart.addSeries(LineSeries, { color: '#7e57c2', lineWidth: 1, lineStyle: LineStyle.Solid, priceLineVisible: false, crosshairMarkerVisible: false });
        ichimokuKijunSeries = chart.addSeries(LineSeries, { color: '#9575cd', lineWidth: 1, lineStyle: LineStyle.Solid, priceLineVisible: false, crosshairMarkerVisible: false });
        ichimokuSenkouASeries = chart.addSeries(LineSeries, { color: 'rgba(126, 87, 194, 0.5)', lineWidth: 1, lineStyle: LineStyle.Solid, priceLineVisible: false, crosshairMarkerVisible: false });
        ichimokuSenkouBSeries = chart.addSeries(LineSeries, { color: 'rgba(120, 144, 156, 0.5)', lineWidth: 1, lineStyle: LineStyle.Solid, priceLineVisible: false, crosshairMarkerVisible: false });
        keltnerUpperSeries = chart.addSeries(LineSeries, { color: '#78909c', lineWidth: 1, lineStyle: LineStyle.Dashed, priceLineVisible: false, crosshairMarkerVisible: false });
        keltnerMiddleSeries = chart.addSeries(LineSeries, { color: '#78909c', lineWidth: 1, lineStyle: LineStyle.Solid, priceLineVisible: false, crosshairMarkerVisible: false });
        keltnerLowerSeries = chart.addSeries(LineSeries, { color: '#78909c', lineWidth: 1, lineStyle: LineStyle.Dashed, priceLineVisible: false, crosshairMarkerVisible: false });
        stddevUpperSeries = chart.addSeries(LineSeries, { color: '#a1887f', lineWidth: 1, lineStyle: LineStyle.Dashed, priceLineVisible: false, crosshairMarkerVisible: false });
        stddevMiddleSeries = chart.addSeries(LineSeries, { color: '#a1887f', lineWidth: 1, lineStyle: LineStyle.Solid, priceLineVisible: false, crosshairMarkerVisible: false });
        stddevLowerSeries = chart.addSeries(LineSeries, { color: '#a1887f', lineWidth: 1, lineStyle: LineStyle.Dashed, priceLineVisible: false, crosshairMarkerVisible: false });
        psarSeries = chart.addSeries(LineSeries, { color: '#ffab40', lineWidth: 1, lineStyle: LineStyle.Dotted, priceLineVisible: false, crosshairMarkerVisible: false });
        priceLineSeries = chart.addSeries(LineSeries, { color: '#ffffff', lineWidth: 1, lineStyle: LineStyle.Solid, priceLineVisible: false, crosshairMarkerVisible: false, lastValueVisible: false });

        // SMC markers (selective: confidence ≥ 0.7 only). Attach to the
        // candle series so the markers follow candle time/price alignment.
        smcMarkers = createSmcMarkers(candleSeries);

        chart.priceScale('right').applyOptions({ alignLabels: true });
        chart.timeScale().applyOptions({ rightOffset: 12, barSpacing: 6 });

        registerChart(chart, container);

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

        ro = new ResizeObserver(() => {
            const w = container.clientWidth, h = container.clientHeight; if (chart && w > 0 && h > 0) chart.resize(w, h);
        });
        if (container?.parentElement) ro.observe(container.parentElement);

        _chartReady = true;
    });

    $effect(() => {
    // Historical bootstrap. Re-runs whenever `pairKey` or `timeframe`
    // changes (per Svelte 5 `$effect` semantics), so a slow daemon
    // start or a fast `pairKey` swap both recover automatically once
    // the data shows up — unlike the legacy `onMount` IIFE which
    // would race-condition and never re-fire.
    if (!timeframe) return;
    let cancelled = false;
    (async () => {
        try {
            const hist = await fetchIndicatorHistoryOnce(pairKey, timeframe);
            if (cancelled || !hist) { _bootstrapComplete = true; return; }
            const step = tf?.barDurationSec || 60;

            const seenTimes = new Set<number>();
            const historicalCandles: { time: Time; open: number; high: number; low: number; close: number }[] = [];
            for (let i = 0; i < hist.candleTimes.length; i++) {
                const t = hist.candleTimes[i];
                const o = hist.candles.open[i];
                const h = hist.candles.high[i];
                const l = hist.candles.low[i];
                const c = hist.candles.close[i];
                if (t == null || o == null || h == null || l == null || c == null) continue;
                if (seenTimes.has(t)) continue;
                seenTimes.add(t);
                historicalCandles.push({ time: t as Time, open: o, high: h, low: l, close: c });
            }

            // Fallback for endpoints that ship only `prices[]` and
            // no structured candles — synthesise a flat line of OHLC
            // so the user sees at least a price track.
            if (historicalCandles.length === 0 && hist.prices && hist.prices.length > 0) {
                const now = Math.floor(Date.now() / 1000);
                for (let i = 0; i < hist.prices.length; i++) {
                    const val = parseFloat(hist.prices[i]) || 0;
                    const t = (now - (hist.prices.length - i) * step) as number;
                    if (seenTimes.has(t)) continue;
                    seenTimes.add(t);
                    historicalCandles.push({ time: t as Time, open: val, high: val, low: val, close: val });
                }
            }

            historicalCandles.sort((a, b) => Number(a.time) - Number(b.time));
            const visibleCap = Math.min(historicalCandles.length, PRICE_CHART_SEED_COUNT);
            const recent = <T extends { time: Time; value: number }>(arr: T[]) => arr.slice(-visibleCap);
            const recentCandles = historicalCandles.slice(-visibleCap);
            if (recentCandles.length > 0) {
                candleSeries.setData(recentCandles);
                priceLineSeries.setData(
                    recentCandles.map((c) => ({ time: c.time, value: c.close }))
                );
            }

            // Pull all historical indicator series in one shot via
            // the unified helper. Each result is independently
            // aligned to hist.times and dedup-sorted.
            const [
                emaFast, emaMed, emaSlow, emaLong,
                bbUp, bbMid, bbLo,
                supertrendPts,
                donchUp, donchMid, donchLo,
                ichiTenkan, ichiKijun, ichiSA, ichiSB,
                avwapW, avwapM, avwapS,
                kelUp, kelMid, kelLo,
                stdUp, stdMid, stdLo,
                psarPts,
            ] = alignedSeriesFromHistory(hist, [
                ['ema_stack', 'fast'],
                ['ema_stack', 'medium'],
                ['ema_stack', 'slow'],
                ['ema_stack', 'long'],
                ['bollinger', 'upper'],
                ['bollinger', 'middle'],
                ['bollinger', 'lower'],
                ['supertrend'],
                ['donchian', 'upper'],
                ['donchian', 'middle'],
                ['donchian', 'lower'],
                ['ichimoku', 'tenkan'],
                ['ichimoku', 'kijun'],
                ['ichimoku', 'senkou_a'],
                ['ichimoku', 'senkou_b'],
                ['anchored_vwap', 'weekly'],
                ['anchored_vwap', 'monthly'],
                ['anchored_vwap', 'swing'],
                ['keltner', 'upper'],
                ['keltner', 'middle'],
                ['keltner', 'lower'],
                ['stddev_channel', 'upper'],
                ['stddev_channel', 'center'],
                ['stddev_channel', 'lower'],
                ['psar', 'sar'],
            ]);

            if (emaFast.length > 0) ema10Series.setData(recent(emaFast));
            if (emaMed.length > 0) ema50Series.setData(recent(emaMed));
            if (emaSlow.length > 0) ema100Series.setData(recent(emaSlow));
            if (emaLong.length > 0) ema200Series.setData(recent(emaLong));
            if (bbUp.length > 0) bbUpperSeries.setData(recent(bbUp));
            if (bbMid.length > 0) bbMiddleSeries.setData(recent(bbMid));
            if (bbLo.length > 0) bbLowerSeries.setData(recent(bbLo));

            // VWAP: daily for < 1 h, weekly for 1 h ≤ tf < 12 h, monthly for ≥ 12 h.
            const vwapSeed = vwapPickKey(timeframe);
            const vwapHist =
                vwapSeed.iSubKey === 'weekly' ? avwapW :
                vwapSeed.iSubKey === 'monthly' ? avwapM :
                pairsFromHistory(hist, 'vwap');
            if (vwapHist.length > 0) vwapSeries.setData(recent(vwapHist));

            // Anchored VWAP — picked from whichever weekly/monthly/swing array the API returned.
            if (anchoredVwapSeries) {
                const avwapAvail = avwapW.length > 0 ? avwapW
                    : avwapM.length > 0 ? avwapM
                    : avwapS;
                if (avwapAvail.length > 0) anchoredVwapSeries.setData(recent(avwapAvail));
            }
            if (supertrendPts.length > 0 && supertrendSeries) supertrendSeries.setData(recent(supertrendPts));
            if (donchUp.length > 0 && donchianUpperSeries) donchianUpperSeries.setData(recent(donchUp));
            if (donchMid.length > 0 && donchianMiddleSeries) donchianMiddleSeries.setData(recent(donchMid));
            if (donchLo.length > 0 && donchianLowerSeries) donchianLowerSeries.setData(recent(donchLo));
            if (ichiTenkan.length > 0 && ichimokuTenkanSeries) ichimokuTenkanSeries.setData(recent(ichiTenkan));
            if (ichiKijun.length > 0 && ichimokuKijunSeries) ichimokuKijunSeries.setData(recent(ichiKijun));
            if (ichiSA.length > 0 && ichimokuSenkouASeries) ichimokuSenkouASeries.setData(recent(ichiSA));
            if (ichiSB.length > 0 && ichimokuSenkouBSeries) ichimokuSenkouBSeries.setData(recent(ichiSB));
            if (kelUp.length > 0 && keltnerUpperSeries) keltnerUpperSeries.setData(recent(kelUp));
            if (kelMid.length > 0 && keltnerMiddleSeries) keltnerMiddleSeries.setData(recent(kelMid));
            if (kelLo.length > 0 && keltnerLowerSeries) keltnerLowerSeries.setData(recent(kelLo));
            if (stdUp.length > 0 && stddevUpperSeries) stddevUpperSeries.setData(recent(stdUp));
            if (stdMid.length > 0 && stddevMiddleSeries) stddevMiddleSeries.setData(recent(stdMid));
            if (stdLo.length > 0 && stddevLowerSeries) stddevLowerSeries.setData(recent(stdLo));
            if (psarPts.length > 0 && psarSeries) psarSeries.setData(recent(psarPts.filter(p => p.value > 0)));

            if (recentCandles.length > 0) {
                _lastHistoryTime = Number(recentCandles[recentCandles.length - 1].time);
            }

            chart.timeScale().fitContent();

            // v6.5: capture per-TF cluster + volume profile from
            // history (used as a fallback if the WS stream hasn't
            // yet populated tf.cluster / tf.volumeProfile).
            const slotKey = slot; // 'micro' | 'fast' | 'slow' | 'macro'
            historyCluster = hist.clusters?.[slotKey] as LiquidationClusterMatrix | null;
            historyVolumeProfile = hist.volumeProfiles?.[slotKey] as VolumeProfileSnapshot | null;
            _bootstrapComplete = true;
        } catch (err) {
            console.error("Error bootstrapping price chart history:", err);
            _bootstrapComplete = true;
        }
    })();
    return () => { cancelled = true; };
    });

    onDestroy(() => {
        candleCoalescer.destroy();
        ro?.disconnect();
        if (chart) {
            unregisterChart(chart);
            chart.remove();
        }
    });

    // NOTE: each $effect below reads all reactive dependencies BEFORE the
    // early-return guard. Per Svelte 5 semantics, an effect only tracks
    // values that are *synchronously read during its execution*; an early
    // return that fires before the dependencies are evaluated would leave
    // them untracked, so subsequent mutations (toggle clicks, WS-driven
    // `tf.*` updates) would never re-trigger the effect. See the official
    // Svelte 5 `$effect` docs, "Understanding dependencies".

    $effect(() => {
        void _chartReady;
        const showFast = pair?.showEmaFast ?? false;
        if (!ema10Series || !pair) return;
        ema10Series.applyOptions({ visible: showFast });
    });

    $effect(() => {
        void _chartReady;
        const showMedium = pair?.showEmaMedium ?? false;
        if (!ema50Series || !pair) return;
        ema50Series.applyOptions({ visible: showMedium });
    });

    $effect(() => {
        void _chartReady;
        const showSlow = pair?.showEmaSlow ?? false;
        if (!ema100Series || !pair) return;
        ema100Series.applyOptions({ visible: showSlow });
    });

    $effect(() => {
        void _chartReady;
        const showLong = pair?.showEmaLong ?? false;
        if (!ema200Series || !pair) return;
        ema200Series.applyOptions({ visible: showLong });
    });

    $effect(() => {
        const priceLineMode = pair?.priceLineMode ?? false;
        if (!candleSeries || !priceLineSeries || !pair) return;
        candleSeries.applyOptions({ visible: !priceLineMode });
        priceLineSeries.applyOptions({ visible: priceLineMode });
    });

    $effect(() => {
        const showBb = tf?.showBb ?? false;
        if (!bbUpperSeries || !bbMiddleSeries || !bbLowerSeries || !pair || !tf) return;
        bbUpperSeries.applyOptions({ visible: showBb });
        bbMiddleSeries.applyOptions({ visible: showBb });
        bbLowerSeries.applyOptions({ visible: showBb });
    });

    $effect(() => {
        const showVwap = tf?.showVwap ?? false;
        if (!vwapSeries || !pair || !tf) return;
        vwapSeries.applyOptions({ visible: showVwap });
    });

    $effect(() => {
        const showAvwap = tf?.showAnchoredVwap ?? false;
        if (!anchoredVwapSeries || !pair || !tf) return;
        anchoredVwapSeries.applyOptions({ visible: showAvwap });
    });

    $effect(() => {
        const showSt = tf?.showSupertrend ?? false;
        if (!supertrendSeries || !pair || !tf) return;
        supertrendSeries.applyOptions({ visible: showSt });
    });

    $effect(() => {
        const showDon = tf?.showDonchian ?? false;
        if (!donchianUpperSeries || !donchianMiddleSeries || !donchianLowerSeries || !pair || !tf) return;
        donchianUpperSeries.applyOptions({ visible: showDon });
        donchianMiddleSeries.applyOptions({ visible: showDon });
        donchianLowerSeries.applyOptions({ visible: showDon });
    });

    $effect(() => {
        const showIchi = tf?.showIchimoku ?? false;
        if (!ichimokuTenkanSeries || !ichimokuKijunSeries || !ichimokuSenkouASeries || !ichimokuSenkouBSeries || !pair || !tf) return;
        ichimokuTenkanSeries.applyOptions({ visible: showIchi });
        ichimokuKijunSeries.applyOptions({ visible: showIchi });
        ichimokuSenkouASeries.applyOptions({ visible: showIchi });
        ichimokuSenkouBSeries.applyOptions({ visible: showIchi });
    });

    $effect(() => {
        const show = tf?.showKeltner ?? false;
        if (!keltnerUpperSeries || !keltnerMiddleSeries || !keltnerLowerSeries) return;
        keltnerUpperSeries.applyOptions({ visible: show });
        keltnerMiddleSeries.applyOptions({ visible: show });
        keltnerLowerSeries.applyOptions({ visible: show });
    });

    $effect(() => {
        const show = tf?.showStddevChan ?? false;
        if (!stddevUpperSeries || !stddevMiddleSeries || !stddevLowerSeries) return;
        stddevUpperSeries.applyOptions({ visible: show });
        stddevMiddleSeries.applyOptions({ visible: show });
        stddevLowerSeries.applyOptions({ visible: show });
    });

    $effect(() => {
        const show = tf?.showPsar ?? false;
        if (!psarSeries) return;
        psarSeries.applyOptions({ visible: show });
    });

    /// Price-level (horizontal line) effects. Each rebuilds the lines when
    /// the underlying values change so the level always shows the most
    /// recent reading from the analyzer. Toggles hide the lines by
    /// removing them.
    ///
    /// Fibonacci — all retracement levels + golden pocket + extensions.
    const fibVals = $derived(tf?.indicators?.['fibonacci']?.values ?? null);
    const fibShow = $derived(tf?.showFib ?? false);

    // [fib-debug] one-shot diagnostic — remove once the chart-overlay data
    // path is confirmed healthy (PR: fibonacci regression).
    $effect(() => {
        const _vals = fibVals;
        const _show = fibShow;
        // eslint-disable-next-line no-console
        console.log('[fib-debug]', {
            showFib: _show,
            keys: _vals ? Object.keys(_vals) : null,
            sample: _vals,
            stateLabel: tf?.indicators?.['fibonacci']?.state_label,
            normalized: tf?.indicators?.['fibonacci']?.normalized,
        });
    });

    $effect(() => {
        for (const line of fibLines) {
            try { candleSeries?.removePriceLine(line); } catch (_) {}
        }
        fibLines = [];

        const vals = fibVals;
        const show = fibShow;
        if (!show || !vals || !candleSeries) return;

        const retracements: Array<{ key: string; label: string; gp: boolean }> = [
            { key: 'fib_0236', label: '0.236', gp: false },
            { key: 'fib_0382', label: '0.382', gp: false },
            { key: 'fib_0500', label: '0.500', gp: false },
            { key: 'fib_0618', label: '0.618', gp: true },
            { key: 'fib_0660', label: '0.660', gp: true },
            { key: 'fib_0786', label: '0.786', gp: false },
        ];
        const extensions: Array<{ key: string; label: string }> = [
            { key: 'ext_1618', label: '1.618' },
            { key: 'ext_2618', label: '2.618' },
        ];

        for (const r of retracements) {
            const v = (vals as Record<string, number | undefined>)[r.key];
            if (typeof v !== 'number' || !isFinite(v) || v <= 0) continue;
            fibLines.push(candleSeries.createPriceLine({
                price: v,
                color: r.gp ? '#ffd54f' : 'rgba(255, 213, 79, 0.55)',
                lineWidth: r.gp ? 2 : 1,
                lineStyle: r.gp ? 1 : 3,
                axisLabelVisible: r.gp,
                title: r.gp ? r.label : '',
            }));
        }

        for (const e of extensions) {
            const v = (vals as Record<string, number | undefined>)[e.key];
            if (typeof v !== 'number' || !isFinite(v) || v <= 0) continue;
            fibLines.push(candleSeries.createPriceLine({
                price: v,
                color: '#00e5ff',
                lineWidth: 1,
                lineStyle: 2,
                axisLabelVisible: true,
                title: e.label,
            }));
        }
    });

    /// Liquidation cluster levels — peak_price + boundaries for
    /// each cluster, grouped by short (ceiling) / long (floor).
    const liqCluster = $derived(tf?.cluster ?? historyCluster ?? null);
    const liqVisible = $derived(tf?.showLiqHeatmap ?? false);

    $effect(() => {
        for (const line of liqLines) {
            try { candleSeries?.removePriceLine(line); } catch (_) {}
        }
        liqLines = [];

        const cluster = liqCluster;
        const show = liqVisible;
        if (!show || !cluster || !candleSeries) return;

        const drawClusters = (
            clusters: Array<{ price_low: number; price_high: number; peak_price: number; dominant_leverage: number; magnet_strength: number }>,
            peakR: number, peakG: number, peakB: number,
        ) => {
            const sorted = [...clusters].sort((a, b) => (b.magnet_strength ?? 0) - (a.magnet_strength ?? 0));
            for (const c of sorted) {
                const mag = Math.max(0, Math.min(100, c.magnet_strength ?? 0));
                const peakAlpha = 0.35 + (mag / 100) * 0.55;
                const boundAlpha = peakAlpha * 0.45;

                if (!isFinite(c.peak_price) || c.peak_price <= 0) continue;
                liqLines.push(candleSeries.createPriceLine({
                    price: c.peak_price,
                    color: `rgba(${peakR},${peakG},${peakB},${peakAlpha.toFixed(2)})`,
                    lineWidth: 2,
                    lineStyle: 2,
                    axisLabelVisible: true,
                    title: `${c.dominant_leverage}\u00d7`,
                }));

                if (c.price_low > 0 && isFinite(c.price_low)) {
                    liqLines.push(candleSeries.createPriceLine({
                        price: c.price_low,
                        color: `rgba(${peakR},${peakG},${peakB},${boundAlpha.toFixed(2)})`,
                        lineWidth: 1,
                        lineStyle: 3,
                        axisLabelVisible: false,
                        title: '',
                    }));
                }

                if (c.price_high > 0 && isFinite(c.price_high)) {
                    liqLines.push(candleSeries.createPriceLine({
                        price: c.price_high,
                        color: `rgba(${peakR},${peakG},${peakB},${boundAlpha.toFixed(2)})`,
                        lineWidth: 1,
                        lineStyle: 3,
                        axisLabelVisible: false,
                        title: '',
                    }));
                }
            }
        };

        drawClusters(cluster.short_clusters ?? [], 255, 68, 68);
        drawClusters(cluster.long_clusters ?? [], 68, 221, 68);
    });

    $effect(() => {
        const show = tf?.showPivotPoints ?? false;
        const vRaw = tf?.indicators?.['pivot_points']?.values?.['pivot'];
        const v = typeof vRaw === 'number' && isFinite(vRaw) && vRaw > 0 ? vRaw : null;
        pivotLevelValue = show ? v : null;
        if (!candleSeries) return;
        if (pivotLine) { try { candleSeries.removePriceLine(pivotLine); } catch (_) {} pivotLine = null; }
        if (pivotLevelValue != null) {
            pivotLine = candleSeries.createPriceLine({
                price: pivotLevelValue,
                color: '#8d6e63',
                lineWidth: 1,
                lineStyle: 3,
                axisLabelVisible: true,
                title: 'PIVOT',
            });
        }
    });

    $effect(() => {
        const show = tf?.showSupportResistance ?? false;
        const raw = tf?.indicators?.['support_resistance'];
        // The normalizer emits the strongest current SR level as `raw`.
        const v = raw && Number.isFinite(raw.raw_value) && raw.raw_value > 0 ? raw.raw_value : null;
        srLevelValue = show ? v : null;
        if (!candleSeries) return;
        if (srLine) { try { candleSeries.removePriceLine(srLine); } catch (_) {} srLine = null; }
        if (srLevelValue != null) {
            srLine = candleSeries.createPriceLine({
                price: srLevelValue,
                color: '#90a4ae',
                lineWidth: 1,
                lineStyle: 3,
                axisLabelVisible: true,
                title: 'S/R',
            });
        }
    });

    /// SMC structure / liquidity markers (BOS↑ / CHoCH↓ / SWEEP↑ / ...).
    /// Same toggle-gate pattern as the three pattern-marker $effects above:
    /// clear() runs whenever the toggle flips (or latestSnapshot advances)
    /// so a stale BO↑ / CH↑ / SP↑ doesn't linger after the user clicks
    /// BOS/CHoCH off.
    $effect(() => {
        const show = tf?.showSmcStructure ?? false;
        if (!smcMarkers || !candleSeries) return;
        smcMarkers.clear();
        if (!show) return;
        const snap = tf?.latestSnapshot;
        const m = (tf?.indicators ?? {}) as IndicatorMap;
        const t = (snap?.timestamp as number) ?? 0;
        smcMarkers.push(t, {
            structure: m['smc_structure'] ?? null,
            liquidity: m['smc_liquidity'] ?? null,
        });
    });

    let _lastUpdateTs = 0;
    const candleCoalescer = makeChartCoalescer(app, () => pairKey, () => slot, (snap, tfVal) => {
        const timeSec: number = typeof snap.timestamp === 'number'
            ? snap.timestamp
            : Number(snap.timestamp ?? 0);
        if (!_bootstrapComplete || !Number.isFinite(_lastHistoryTime) || timeSec < _lastHistoryTime) return;
        const m = (tfVal.indicators ?? {}) as IndicatorMap;

        if (snap.close != null) {
            // Loose gate: accept any tick that has at least a `close`,
            // including in-progress ticks where OHLC are not all settled.
            // For the missing fields, fill with the close so the candle
            // still updates and the price line tracks. If `close` itself
            // is missing it is a malformed payload — log loudly.
            const cl = parseFloat(String(snap.close));
            const o = snap.open != null ? parseFloat(String(snap.open)) : cl;
            const h = snap.high != null ? parseFloat(String(snap.high)) : cl;
            const l = snap.low != null ? parseFloat(String(snap.low))  : cl;
            candleSeries.update({
                time: timeSec as Time,
                open: Number.isFinite(o) ? o : cl,
                high: Number.isFinite(h) ? h : cl,
                low:  Number.isFinite(l) ? l : cl,
                close: cl,
            });
            priceLineSeries.update({
                time: timeSec as Time,
                value: cl
            });
        } else {
            console.warn(`[CHART-DIAG] PriceChart ${pairKey}/${slot}: snapshot at ${timeSec} is missing close; candle not updated`);
        }

        const emaFast = iSub(m, 'ema_stack', 'fast');
        const emaMedium = iSub(m, 'ema_stack', 'medium');
        const emaSlow = iSub(m, 'ema_stack', 'slow');
        const emaLong = iSub(m, 'ema_stack', 'long');
        const bbUpper = iSub(m, 'bollinger', 'upper');
        const bbMiddle = iSub(m, 'bollinger', 'middle');
        const bbLower = iSub(m, 'bollinger', 'lower');
        // Auto-adapt VWAP anchor to the active TF: daily for < 1 h,
        // weekly for 1 h ≤ tf < 12 h, monthly for ≥ 12 h.
        const liveVwap = (() => {
            const k = vwapPickKey(tfVal.barDurationSec).iSubKey;
            return k === 'weekly'  ? iSub(m, 'anchored_vwap', 'weekly')
                 : k === 'monthly' ? iSub(m, 'anchored_vwap', 'monthly')
                                     : iSub(m, 'vwap', 'vwap');
        })();
        const anchoredVwap = iSub(m, 'anchored_vwap', 'weekly')
            ?? iSub(m, 'anchored_vwap', 'monthly')
            ?? iSub(m, 'anchored_vwap', 'swing');
        const stLine = iSub(m, 'supertrend', 'line');
        const donchUp = iSub(m, 'donchian', 'upper');
        const donchMid = iSub(m, 'donchian', 'middle');
        const donchLo = iSub(m, 'donchian', 'lower');
        const ichiTenkan = iSub(m, 'ichimoku', 'tenkan');
        const ichiKijun = iSub(m, 'ichimoku', 'kijun');
        const ichiSenkouA = iSub(m, 'ichimoku', 'senkou_a');
        const ichiSenkouB = iSub(m, 'ichimoku', 'senkou_b');
        const kelUp = iSub(m, 'keltner', 'upper');
        const kelMid = iSub(m, 'keltner', 'middle');
        const kelLo = iSub(m, 'keltner', 'lower');
        const stdUp = iSub(m, 'stddev_channel', 'upper');
        const stdMid = iSub(m, 'stddev_channel', 'center');
        const stdLo = iSub(m, 'stddev_channel', 'lower');
        const psarSar = iSub(m, 'psar', 'sar');
        const supertrendState = m['supertrend']?.state_label ?? '';

        if (emaFast != null) ema10Series.update({ time: timeSec as Time, value: emaFast });
        if (emaMedium != null) ema50Series.update({ time: timeSec as Time, value: emaMedium });
        if (emaSlow != null) ema100Series.update({ time: timeSec as Time, value: emaSlow });
        if (emaLong != null) ema200Series.update({ time: timeSec as Time, value: emaLong });
        if (bbUpper != null) bbUpperSeries.update({ time: timeSec as Time, value: bbUpper });
        if (bbMiddle != null) bbMiddleSeries.update({ time: timeSec as Time, value: bbMiddle });
        if (bbLower != null) bbLowerSeries.update({ time: timeSec as Time, value: bbLower });
        if (liveVwap != null) vwapSeries.update({ time: timeSec as Time, value: liveVwap });
        if (anchoredVwap != null && anchoredVwapSeries) anchoredVwapSeries.update({ time: timeSec as Time, value: anchoredVwap });
        if (stLine != null && supertrendSeries) {
            supertrendSeries.update({ time: timeSec as Time, value: stLine });
            const color = supertrendState.includes('BEARISH') ? '#ef5350'
                : supertrendState.includes('BULLISH') ? '#26a69a'
                : '#26a69a';
            supertrendSeries.applyOptions({ color });
        }
        if (donchUp != null && donchianUpperSeries) donchianUpperSeries.update({ time: timeSec as Time, value: donchUp });
        if (donchMid != null) donchianMiddleSeries?.update({ time: timeSec as Time, value: donchMid });
        if (donchLo != null && donchianLowerSeries) donchianLowerSeries.update({ time: timeSec as Time, value: donchLo });
        if (ichiTenkan != null && ichimokuTenkanSeries) ichimokuTenkanSeries.update({ time: timeSec as Time, value: ichiTenkan });
        if (ichiKijun != null && ichimokuKijunSeries) ichimokuKijunSeries.update({ time: timeSec as Time, value: ichiKijun });
        if (ichiSenkouA != null && ichimokuSenkouASeries) ichimokuSenkouASeries.update({ time: timeSec as Time, value: ichiSenkouA });
        if (ichiSenkouB != null && ichimokuSenkouBSeries) ichimokuSenkouBSeries.update({ time: timeSec as Time, value: ichiSenkouB });
        if (kelUp != null && keltnerUpperSeries) keltnerUpperSeries.update({ time: timeSec as Time, value: kelUp });
        if (kelMid != null && keltnerMiddleSeries) keltnerMiddleSeries.update({ time: timeSec as Time, value: kelMid });
        if (kelLo != null && keltnerLowerSeries) keltnerLowerSeries.update({ time: timeSec as Time, value: kelLo });
        if (stdUp != null && stddevUpperSeries) stddevUpperSeries.update({ time: timeSec as Time, value: stdUp });
        if (stdMid != null && stddevMiddleSeries) stddevMiddleSeries.update({ time: timeSec as Time, value: stdMid });
        if (stdLo != null && stddevLowerSeries) stddevLowerSeries.update({ time: timeSec as Time, value: stdLo });
        if (psarSar != null && Number.isFinite(psarSar) && psarSar > 0 && psarSar < 1_000_000 && psarSeries) psarSeries.update({ time: timeSec as Time, value: psarSar });

        // Push SMC events into the marker consumer (selective: conf >= 0.7).
        if (smcMarkers && tfVal.showSmcStructure) {
            smcMarkers.push(timeSec, {
                structure: m['smc_structure'] ?? null,
                liquidity: m['smc_liquidity'] ?? null,
            });
        }
    });
    $effect(() => {
        // Track broadcast arrival (the gap diagnostic must measure WS gaps,
        // not rAF gaps) and let the coalescer collapse redraws to one per frame.
        const pairVal = app.instancesMap[pairKey];
        if (!pairVal) return;
        const tfVal = slot === 'micro' ? pairVal.microTerm : slot === 'fast' ? pairVal.fastTerm : slot === 'slow' ? pairVal.slowTerm : pairVal.macroTerm;
        const snap = tfVal.latestSnapshot;
        if (!snap) return;
        const now = Date.now();
        const gap = _lastUpdateTs > 0 ? now - _lastUpdateTs : 0;
        _lastUpdateTs = now;
        if (gap > 10_000) {
            console.warn(`[CHART-DIAG] PriceChart ${pairKey}/${slot}: ${gap}ms gap between updates at ${new Date(now).toISOString()}`);
        }
        candleCoalescer.effect();
    });

    /// Last SMC structure / liquidity event across `smc_structure` +
    /// `smc_liquidity`. We pick the minimum `age_bars` across every signal
    /// so the most recent event wins, then resolve a human label
    /// (BOS↑ / CHoCH↓ / SWEEP↑ / ...). Returns `null` when no event has
    /// been emitted yet (SmartMoney needs ≥ 5 bars + 2 swings to fire).
    type SmcDir = 'bullish' | 'bearish' | 'neutral';
    type SmcKind = 'BOS' | 'CHoCH' | 'SWEEP' | null;
    interface SmcEvent {
        ageBars: number;
        kind: SmcKind;
        dir: SmcDir;
    }

    const lastSmcEvent = $derived.by<SmcEvent | null>(() => {
        if (!tf) return null;
        const struct = tf.indicators?.['smc_structure'];
        const liq = tf.indicators?.['smc_liquidity'];
        let ageBars = Number.POSITIVE_INFINITY;
        let kind: SmcKind = null;
        const dir: SmcDir = 'neutral';

        function consider(sigList: ReadonlyArray<any> | undefined | null): void {
            if (!sigList) return;
            for (const sig of sigList) {
                const age = sig.age_bars;
                if (typeof age !== 'number') continue;
                if (age < ageBars) {
                    ageBars = age;
                    const label = (sig.label ?? '').toUpperCase();
                    if (sig.kind === 'Breakout' || label.includes('BOS')) kind = 'BOS';
                    else if (sig.kind === 'TrendFlip' || label.includes('CHOCH')) kind = 'CHoCH';
                    else if (sig.kind === 'PatternForming' || label.includes('SWEEP')) kind = 'SWEEP';
                }
            }
        }
        consider(struct?.signals);
        consider(liq?.signals);
        if (!isFinite(ageBars)) return null;
        return { ageBars, kind, dir };
    });

    const smcDirCls = $derived(
        !lastSmcEvent ? '' :
        'neutral'
    );

    function smcAgeLabel(ageBars: number, timeframeSec: number): string {
        const secs = ageBars * timeframeSec;
        if (secs < 60) return `${secs}s`;
        if (secs < 3600) return `${Math.floor(secs / 60)}m`;
        if (secs < 86400) return `${Math.floor(secs / 3600)}h`;
        return `${Math.floor(secs / 86400)}d`;
    }

    // Volume profile — toggle visibility + data feeding.
    // v6.5 fallback chain: prefer WS-populated tf.volumeProfile; fall back to
    // history-sourced historyVolumeProfile when WS hasn't delivered yet.
    // Visibility is handled separately via `setVisible()` so the snapshot
    // is preserved across toggle flips — flipping the pill on never causes
    // a transient null state, which would otherwise race with the WS push
    // cadence (only completed snapshots carry volume_profile).
    $effect(() => {
        const visible = tf?.showVolumeProfile ?? false;
        const data = tf?.volumeProfile ?? historyVolumeProfile ?? null;
        if (!volumeProfilePrim) return;
        volumeProfilePrim.setVisible(visible);
        volumeProfilePrim.updateData(data);
    });

    // Liquidation heatmap — toggle visibility + data feeding.
    // Mirror of `volumeProfile`'s pattern: prefer the live WS cluster,
    // fall back to history-sourced `historyCluster` until the first
    // per-TF refresh tick fires after a daemon restart.
    //
    // Block C: feeds both the **estimated** cluster matrix AND the
    // **observed** real-event buckets (from `tf.liquidity.recent_real_buckets`).
    // The shared frontend renderer draws them in two layers. When the
    // exchange has no public feed (Hyperliquid without
    // `hyperliquid_user_address`), the real layer stays empty and the
    // HL caveat watermark surfaces above the chart.
    $effect(() => {
        const visible = tf?.showLiqHeatmap ?? false;
        if (!liqHeatmapPrim) return;
        liqHeatmapPrim.setVisible(visible);
        // Always feed data so the toggle can flip back on without race.
        // `updateData` accepts partial inputs and merges against the
        // previous shape — passing `null` is intentional when no data
        // has arrived yet (the primitive suppresses rendering on null).
        // v7.0-prod: also forward the per-TF operator-selected leverage
        // tiers so matching clusters intensify (D5 default `[10]`).
        const cluster = tf?.cluster ?? historyCluster ?? null;
        const flow = tf?.liquidity ?? null;
        const ex = tf?.exchange ?? '';
        const highlightTiers = tf?.heatmapLeverageTiers ?? [10];
        liqHeatmapPrim.updateData({ cluster, flow, exchange: ex, highlightTiers });
    });

    // SMC Fair Value Gap zones — toggle visibility + rolling zone list.
    $effect(() => {
        const visible = tf?.showFvgZones ?? false;
        const dto = tf?.indicators?.['smc_fvg'] ?? null;
        if (!fvgPrim) return;
        fvgPrim.setVisible(visible);
        if (!visible) {
            // Don't accumulate while hidden — clear so toggling back on
            // doesn't surface stale zones.
            fvgPrim.clear();
            return;
        }
        fvgPrim.updateData(dto);
    });

    // SMC Order Block zones — toggle visibility + rolling zone list.
    $effect(() => {
        const visible = tf?.showOrderBlocks ?? false;
        const dto = tf?.indicators?.['smc_order_blocks'] ?? null;
        if (!obPrim) return;
        obPrim.setVisible(visible);
        if (!visible) {
            obPrim.clear();
            return;
        }
        obPrim.updateData(dto);
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
        {#if lastSmcEvent}
            {@const lk = (lastSmcEvent.kind ?? 'EVT') as string}
            <span class={styles.smcFooter}>
                LAST SMC: {lk} · {smcAgeLabel(lastSmcEvent.ageBars, timeframe)} ago
            </span>
        {:else}
            <span class={styles.smcFooter}>SMC: AWAITING SWING</span>
        {/if}
    {/if}
    <div class={styles.chartContainer} bind:this={container}></div>
</div>
