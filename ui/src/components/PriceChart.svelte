<script lang="ts">
    import { emaStackState, vwapBias, iSub } from '../lib/telemetry';
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
    import { attachHeatmap, type LiquidationHeatmapPrimitive } from '../lib/liquidationHeatmap';
    import { attachVolumeProfile, type VolumeProfilePrimitive } from '../lib/volumeProfile';
    import { attachFvgZones, type FvgZonesPrimitive } from '../lib/fvgZones';
    import { attachOrderBlocks, type OrderBlocksPrimitive } from '../lib/orderBlocks';
    import { makeChartCoalescer } from '../lib/chartCoalesce';
    import { vwapPickKey } from '../lib/vwapAnchor';
    import { createSmcMarkers, type SmcMarkerController } from '../lib/smcMarkers';
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
    let anchoredVwapSeries: ISeriesApi<'Line'> | null = null;
    let supertrendSeries: ISeriesApi<'Line'> | null = null;
    let donchianUpperSeries: ISeriesApi<'Line'> | null = null;
    let donchianLowerSeries: ISeriesApi<'Line'> | null = null;
    let ichimokuTenkanSeries: ISeriesApi<'Line'> | null = null;
    let ichimokuKijunSeries: ISeriesApi<'Line'> | null = null;
    let ichimokuSenkouASeries: ISeriesApi<'Line'> | null = null;
    let ichimokuSenkouBSeries: ISeriesApi<'Line'> | null = null;
    let priceLineSeries: ISeriesApi<'Line'>;
    let heatmap: LiquidationHeatmapPrimitive | null = null;
    let volumeProfilePrim: VolumeProfilePrimitive | null = null;
    let fvgPrim: FvgZonesPrimitive | null = null;
    let obPrim: OrderBlocksPrimitive | null = null;
    let smcMarkers: SmcMarkerController | null = null;
    /// v6.5: cluster / volume-profile snapshots fetched via
    /// `/api/history` on first-mount. Used as a **fallback** when the WS
    /// stream hasn't yet populated `tf.cluster` / `tf.volumeProfile`
    /// (i.e. on a fresh daemon restart, before the first per-TF refresh
    /// tick fires).
    let historyCluster: LiquidationClusterMatrix | null = $state(null);
    let historyVolumeProfile: VolumeProfileSnapshot | null = $state(null);

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

        // Liquidity heatmap overlay (toggle-controlled; data supplied later via $effect).
        heatmap = attachHeatmap(chart, candleSeries);
        // Volume profile overlay (right-edge stacked buy/sell histogram).
        volumeProfilePrim = attachVolumeProfile(chart, candleSeries);
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
        donchianLowerSeries = chart.addSeries(LineSeries, { color: '#ec407a', lineWidth: 1, lineStyle: LineStyle.Dashed, priceLineVisible: false, crosshairMarkerVisible: false });
        ichimokuTenkanSeries = chart.addSeries(LineSeries, { color: '#7e57c2', lineWidth: 1, lineStyle: LineStyle.Solid, priceLineVisible: false, crosshairMarkerVisible: false });
        ichimokuKijunSeries = chart.addSeries(LineSeries, { color: '#9575cd', lineWidth: 1, lineStyle: LineStyle.Solid, priceLineVisible: false, crosshairMarkerVisible: false });
        ichimokuSenkouASeries = chart.addSeries(LineSeries, { color: 'rgba(126, 87, 194, 0.5)', lineWidth: 1, lineStyle: LineStyle.Solid, priceLineVisible: false, crosshairMarkerVisible: false });
        ichimokuSenkouBSeries = chart.addSeries(LineSeries, { color: 'rgba(120, 144, 156, 0.5)', lineWidth: 1, lineStyle: LineStyle.Solid, priceLineVisible: false, crosshairMarkerVisible: false });
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

        (async () => {
            if (!pair) return;
            try {
                const hist = await fetchIndicatorHistoryOnce(pairKey, timeframe);
                if (!hist) return;
                // Prefer real OHLC candles over the fallback `data.prices`
                // string array so sub-minute bootstraps render immediately
                // rather than waiting for the first live WS frame.
                if (hist.candleTimes.length > 0) {
                    const step = tf?.barDurationSec || 60;
                    const baseTime = Math.floor(Date.now() / 1000) - (hist.candleTimes.length * step);

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
                    historicalCandles.sort((a, b) => Number(a.time) - Number(b.time));
                    if (historicalCandles.length === 0) {
                        // Fall back to flat `prices[]` array if no candles shipped.
                        for (let i = 0; i < (hist as any).prices?.length; i++) {
                            const val = parseFloat((hist as any).prices[i]) || 0;
                            const t = baseTime + (i * step);
                            if (!seenTimes.has(t)) {
                                seenTimes.add(t);
                                historicalCandles.push({ time: t as Time, open: val, high: val, low: val, close: val });
                            }
                        }
                    }
                    candleSeries.setData(historicalCandles);
                    priceLineSeries.setData(
                        historicalCandles.map((c) => ({ time: c.time, value: c.close }))
                    );
                    chart.timeScale().fitContent();

                    // Pull all historical indicator series in one shot via
                    // the unified helper. Each result is independently
                    // aligned to hist.times and dedup-sorted.
                    const [
                        emaFast, emaMed, emaSlow, emaLong,
                        bbUp, bbMid, bbLo,
                        supertrendPts,
                        donchUp, donchLo,
                        ichiTenkan, ichiKijun, ichiSA, ichiSB,
                        avwapW, avwapM, avwapS,
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
                        ['donchian', 'lower'],
                        ['ichimoku', 'tenkan'],
                        ['ichimoku', 'kijun'],
                        ['ichimoku', 'senkou_a'],
                        ['ichimoku', 'senkou_b'],
                        ['anchored_vwap', 'weekly'],
                        ['anchored_vwap', 'monthly'],
                        ['anchored_vwap', 'swing'],
                    ]);

                    if (emaFast.length > 0) ema10Series.setData(emaFast);
                    if (emaMed.length > 0) ema50Series.setData(emaMed);
                    if (emaSlow.length > 0) ema100Series.setData(emaSlow);
                    if (emaLong.length > 0) ema200Series.setData(emaLong);
                    if (bbUp.length > 0) bbUpperSeries.setData(bbUp);
                    if (bbMid.length > 0) bbMiddleSeries.setData(bbMid);
                    if (bbLo.length > 0) bbLowerSeries.setData(bbLo);

                    // VWAP: daily for < 1 h, weekly for 1 h–12 h, monthly for ≥ 12 h.
                    const vwapSeed = vwapPickKey(timeframe);
                    const vwapHist =
                        vwapSeed.iSubKey === 'weekly' ? avwapW :
                        vwapSeed.iSubKey === 'monthly' ? avwapM :
                        pairsFromHistory(hist, 'vwap');
                    if (vwapHist.length > 0) vwapSeries.setData(vwapHist);

                    // Anchored VWAP — picked from whichever weekly/monthly/swing array the API returned.
                    if (anchoredVwapSeries) {
                        const avwapAvail = avwapW.length > 0 ? avwapW
                            : avwapM.length > 0 ? avwapM
                            : avwapS;
                        if (avwapAvail.length > 0) anchoredVwapSeries.setData(avwapAvail);
                    }
                    if (supertrendPts.length > 0 && supertrendSeries) supertrendSeries.setData(supertrendPts);
                    if (donchUp.length > 0 && donchianUpperSeries) donchianUpperSeries.setData(donchUp);
                    if (donchLo.length > 0 && donchianLowerSeries) donchianLowerSeries.setData(donchLo);
                    if (ichiTenkan.length > 0 && ichimokuTenkanSeries) ichimokuTenkanSeries.setData(ichiTenkan);
                    if (ichiKijun.length > 0 && ichimokuKijunSeries) ichimokuKijunSeries.setData(ichiKijun);
                    if (ichiSA.length > 0 && ichimokuSenkouASeries) ichimokuSenkouASeries.setData(ichiSA);
                    if (ichiSB.length > 0 && ichimokuSenkouBSeries) ichimokuSenkouBSeries.setData(ichiSB);
                }
                // v6.5: capture per-TF cluster + volume profile from
                // history (used as a fallback if the WS stream hasn't
                // yet populated tf.cluster / tf.volumeProfile).
                const slotKey = slot; // 'micro' | 'fast' | 'slow' | 'macro'
                historyCluster = hist.clusters?.[slotKey] as LiquidationClusterMatrix | null;
                historyVolumeProfile = hist.volumeProfiles?.[slotKey] as VolumeProfileSnapshot | null;
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
        const showFast = pair?.showEmaFast ?? false;
        const showMedium = pair?.showEmaMedium ?? false;
        const showSlow = pair?.showEmaSlow ?? false;
        const showLong = pair?.showEmaLong ?? false;
        if (!ema10Series || !ema50Series || !ema100Series || !ema200Series || !pair) return;
        ema10Series.applyOptions({ visible: showFast });
        ema50Series.applyOptions({ visible: showMedium });
        ema100Series.applyOptions({ visible: showSlow });
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
        if (!donchianUpperSeries || !donchianLowerSeries || !pair || !tf) return;
        donchianUpperSeries.applyOptions({ visible: showDon });
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

    let _lastUpdateTs = 0;
    const candleCoalescer = makeChartCoalescer(app, pairKey, slot, (snap, tfVal) => {
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
        const donchLo = iSub(m, 'donchian', 'lower');
        const ichiTenkan = iSub(m, 'ichimoku', 'tenkan');
        const ichiKijun = iSub(m, 'ichimoku', 'kijun');
        const ichiSenkouA = iSub(m, 'ichimoku', 'senkou_a');
        const ichiSenkouB = iSub(m, 'ichimoku', 'senkou_b');
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
            // Color tracks the supertrend direction: bullish (green) vs bearish (red).
            const color = supertrendState.includes('BEARISH') ? '#ef5350'
                : supertrendState.includes('BULLISH') ? '#26a69a'
                : '#26a69a';
            supertrendSeries.applyOptions({ color });
        }
        if (donchUp != null && donchianUpperSeries) donchianUpperSeries.update({ time: timeSec as Time, value: donchUp });
        if (donchLo != null && donchianLowerSeries) donchianLowerSeries.update({ time: timeSec as Time, value: donchLo });
        if (ichiTenkan != null && ichimokuTenkanSeries) ichimokuTenkanSeries.update({ time: timeSec as Time, value: ichiTenkan });
        if (ichiKijun != null && ichimokuKijunSeries) ichimokuKijunSeries.update({ time: timeSec as Time, value: ichiKijun });
        if (ichiSenkouA != null && ichimokuSenkouASeries) ichimokuSenkouASeries.update({ time: timeSec as Time, value: ichiSenkouA });
        if (ichiSenkouB != null && ichimokuSenkouBSeries) ichimokuSenkouBSeries.update({ time: timeSec as Time, value: ichiSenkouB });

        // Push SMC events into the marker consumer (selective: conf >= 0.7).
        if (smcMarkers) {
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

    // Liquidity heatmap — toggle visibility + data feeding.
    // v6.5 fallback chain: prefer WS-populated tf.cluster; fall back to
    // history-sourced historyCluster when WS hasn't delivered yet.
    // Dependencies are read BEFORE the primitive-guard so they stay tracked
    // across mount (see note on the EMA effect above).
    $effect(() => {
        const visible = tf?.showLiqHeatmap ?? false;
        const data = tf?.cluster ?? historyCluster ?? null;
        if (!heatmap) return;
        heatmap.updateData(visible ? data : null);
    });

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
