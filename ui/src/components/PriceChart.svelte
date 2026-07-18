<script lang="ts">
    import { flattenHistory } from '../lib/historyAdapter';
    import { iRaw, iSub, getPriceFormat } from '../lib/telemetry';
    import type { IndicatorMap } from '../types';
    import { onMount, onDestroy } from 'svelte';
    import { createChart, CrosshairMode, CandlestickSeries, HistogramSeries, LineSeries } from 'lightweight-charts';
    import type { IChartApi, ISeriesApi, Time } from 'lightweight-charts';
    import { useAppStore } from '../state.svelte';
    import { registerChart, unregisterChart } from '../chartRegistry.svelte';

    const app = useAppStore();
    let {
        pairKey,
        timeframe = 60,
        onDoubleClick,
        onScreenshotReady,
    }: {
        pairKey: string;
        timeframe?: number;
        onDoubleClick?: () => void;
        onScreenshotReady?: (fn: () => void) => void;
    } = $props();

    const pair = $derived(app.instancesMap[pairKey]);

    const tf = $derived(
        timeframe === 180 ? pair?.fastTerm :
        timeframe === 300 ? pair?.slowTerm :
        timeframe === 900 ? pair?.macroTerm :
        pair?.microTerm
    );

    const priceLineMode = $derived((pair as any)?.priceLineMode ?? false);
    const showEmaFast    = $derived((pair as any)?.showEmaFast ?? false);
    const showEmaMedium  = $derived((pair as any)?.showEmaMedium ?? false);
    const showEmaSlow    = $derived((pair as any)?.showEmaSlow ?? false);
    const showEmaLong    = $derived((pair as any)?.showEmaLong ?? false);

    const showVwap       = $derived(tf?.showVwap ?? false);
    const showBb         = $derived(tf?.showBb ?? false);
    const showSupertrend = $derived(tf?.showSupertrend ?? false);
    const showKeltner    = $derived(tf?.showKeltner ?? false);
    const showDonchian   = $derived(tf?.showDonchian ?? false);
    const showIchimoku   = $derived(tf?.showIchimoku ?? false);
    const showHullMa     = $derived(tf?.showHullMa ?? false);
    const showPsar       = $derived(tf?.showPsar ?? false);
    const showStddevChan = $derived(tf?.showStddevChan ?? false);
    const showFib        = $derived(tf?.showFib ?? false);

    let container: HTMLDivElement;
    let chart: IChartApi;
    let ro: ResizeObserver;

    let candleSeries: ISeriesApi<'Candlestick'>;
    let lineSeries: ISeriesApi<'Line'>;
    let volumeSeries: ISeriesApi<'Histogram'>;

    let emaFastSeries: ISeriesApi<'Line'> | undefined;
    let emaMediumSeries: ISeriesApi<'Line'> | undefined;
    let emaSlowSeries: ISeriesApi<'Line'> | undefined;
    let emaLongSeries: ISeriesApi<'Line'> | undefined;

    let vwapSeries: ISeriesApi<'Line'> | undefined;
    let bbUpper: ISeriesApi<'Line'> | undefined;
    let bbLower: ISeriesApi<'Line'> | undefined;
    let supertrendSeries: ISeriesApi<'Line'> | undefined;
    let keltnerUpper: ISeriesApi<'Line'> | undefined;
    let keltnerLower: ISeriesApi<'Line'> | undefined;
    let donchianUpper: ISeriesApi<'Line'> | undefined;
    let donchianLower: ISeriesApi<'Line'> | undefined;

    let ichimokuTenkan: ISeriesApi<'Line'> | undefined;
    let ichimokuKijun: ISeriesApi<'Line'> | undefined;
    let ichimokuSenkouA: ISeriesApi<'Line'> | undefined;
    let ichimokuSenkouB: ISeriesApi<'Line'> | undefined;
    let hullMaSeries: ISeriesApi<'Line'> | undefined;
    let psarSeries: ISeriesApi<'Line'> | undefined;
    let stddevUpper: ISeriesApi<'Line'> | undefined;
    let stddevLower: ISeriesApi<'Line'> | undefined;
    let fibLines: ReturnType<IChartApi['createPriceLine']>[] = [];

    let prevLineMode = $state(false);
    let prevShowEmaFast = $state(false);
    let prevShowEmaMedium = $state(false);
    let prevShowEmaSlow = $state(false);
    let prevShowEmaLong = $state(false);
    let prevShowVwap = $state(false);
    let prevShowBb = $state(false);
    let prevShowSupertrend = $state(false);
    let prevShowKeltner = $state(false);
    let prevShowDonchian = $state(false);
    let prevShowIchimoku = $state(false);
    let prevShowHullMa = $state(false);
    let prevShowPsar = $state(false);
    let prevShowStddevChan = $state(false);
    let prevShowFib = $state(false);

    function ensureEmaFast() {
        if (emaFastSeries) return;
        emaFastSeries = chart.addSeries(LineSeries, { color: '#fdd835', lineWidth: 1, priceLineVisible: false });
    }
    function ensureEmaMedium() {
        if (emaMediumSeries) return;
        emaMediumSeries = chart.addSeries(LineSeries, { color: '#ff9800', lineWidth: 1, priceLineVisible: false });
    }
    function ensureEmaSlow() {
        if (emaSlowSeries) return;
        emaSlowSeries = chart.addSeries(LineSeries, { color: '#e91e63', lineWidth: 1, priceLineVisible: false });
    }
    function ensureEmaLong() {
        if (emaLongSeries) return;
        emaLongSeries = chart.addSeries(LineSeries, { color: '#9c27b0', lineWidth: 1, priceLineVisible: false });
    }
    function ensureVwap() {
        if (vwapSeries) return;
        vwapSeries = chart.addSeries(LineSeries, { color: '#2962ff', lineWidth: 1, priceLineVisible: false });
    }
    function ensureBb() {
        if (bbUpper) return;
        bbUpper = chart.addSeries(LineSeries, { color: '#00e5ff', lineWidth: 1, priceLineVisible: false });
        bbLower = chart.addSeries(LineSeries, { color: '#00e5ff', lineWidth: 1, priceLineVisible: false });
    }
    function ensureSupertrend() {
        if (supertrendSeries) return;
        supertrendSeries = chart.addSeries(LineSeries, { color: '#26a69a', lineWidth: 1, priceLineVisible: false });
    }
    function ensureKeltner() {
        if (keltnerUpper) return;
        keltnerUpper = chart.addSeries(LineSeries, { color: '#78909c', lineWidth: 1, priceLineVisible: false });
        keltnerLower = chart.addSeries(LineSeries, { color: '#78909c', lineWidth: 1, priceLineVisible: false });
    }
    function ensureDonchian() {
        if (donchianUpper) return;
        donchianUpper = chart.addSeries(LineSeries, { color: '#ec407a', lineWidth: 1, priceLineVisible: false });
        donchianLower = chart.addSeries(LineSeries, { color: '#ec407a', lineWidth: 1, priceLineVisible: false });
    }
    function ensureIchimoku() {
        if (ichimokuTenkan) return;
        ichimokuTenkan = chart.addSeries(LineSeries, { color: '#f44336', lineWidth: 1, priceLineVisible: false });
        ichimokuKijun = chart.addSeries(LineSeries, { color: '#2196f3', lineWidth: 1, priceLineVisible: false });
        ichimokuSenkouA = chart.addSeries(LineSeries, { color: 'rgba(76,175,80,0.35)', lineWidth: 1, priceLineVisible: false });
        ichimokuSenkouB = chart.addSeries(LineSeries, { color: 'rgba(255,87,34,0.35)', lineWidth: 1, priceLineVisible: false });
    }
    function ensureHullMa() {
        if (hullMaSeries) return;
        hullMaSeries = chart.addSeries(LineSeries, { color: '#00bcd4', lineWidth: 2, priceLineVisible: false });
    }
    function ensurePsar() {
        if (psarSeries) return;
        psarSeries = chart.addSeries(LineSeries, { color: '#ffeb3b', lineWidth: 1, lineStyle: 2, priceLineVisible: false, lastValueVisible: false });
    }
    function ensureStddevChan() {
        if (stddevUpper) return;
        stddevUpper = chart.addSeries(LineSeries, { color: '#9e9e9e', lineWidth: 1, lineStyle: 2, priceLineVisible: false });
        stddevLower = chart.addSeries(LineSeries, { color: '#9e9e9e', lineWidth: 1, lineStyle: 2, priceLineVisible: false });
    }
    function drawFib(levels: { time: Time; value: number }[] | null) {
        fibLines.forEach(l => { try { chart.removePriceLine(l); } catch (_) {} });
        fibLines = [];
        if (!levels || !chart) return;
        for (const l of levels) {
            const priceLine = chart.createPriceLine({
                price: l.value,
                color: '#ffa726',
                lineWidth: 1,
                lineStyle: 2,
                axisLabelVisible: true,
                title: l.value.toFixed(2),
            });
            fibLines.push(priceLine);
        }
    }

    function destroyOptional(series: ISeriesApi<any> | undefined) {
        try { series ? chart.removeSeries(series) : void 0; } catch (_) {}
    }
    function hideSeries(series: ISeriesApi<any> | undefined) {
        if (!series) return;
        try { series.setData([]); } catch (_) {}
    }

    function toggleSeries(show: boolean, prev: boolean, factory: () => void, destroy: () => void) {
        if (show === prev) return;
        if (show) factory(); else destroy();
    }

    function persistHistory(history: any, isLineMode: boolean) {
        if (!history) return;

        const times = history.times || [];
        const len = times.length;
        if (len === 0) return;

        const candleData: { time: Time; open: number; high: number; low: number; close: number }[] = [];
        const lineData: { time: Time; value: number }[] = [];
        const volData: { time: Time; value: number; color: string }[] = [];

        for (let i = 0; i < len; i++) {
            const t = times[i] as Time;
            const open  = parseFloat(history.opens?.[i] ?? '0') || 0;
            const high  = parseFloat(history.highs?.[i] ?? '0') || 0;
            const low   = parseFloat(history.lows?.[i] ?? '0') || 0;
            const close = parseFloat(history.closes?.[i] ?? '0') || 0;
            const vol   = parseFloat(history.volumes?.[i] ?? '0') || 0;

            if (open && high && low && close) {
                candleData.push({ time: t, open, high, low, close });
                lineData.push({ time: t, value: close });
                const green = close >= open;
                volData.push({ time: t, value: vol, color: green ? 'rgba(34,197,94,0.3)' : 'rgba(239,68,68,0.3)' });
            }
        }

        if (candleData.length > 0) {
            candleSeries.setData(candleData);
            lineSeries.setData(lineData);
            candleSeries.applyOptions({ visible: !isLineMode });
            lineSeries.applyOptions({ visible: isLineMode });
        }
        if (volData.length > 0) {
            volumeSeries.setData(volData);
        }

        if (history.ema_fast && showEmaFast) pushHistoryLine(emaFastSeries, times, history.ema_fast);
        if (history.ema_medium && showEmaMedium) pushHistoryLine(emaMediumSeries, times, history.ema_medium);
        if (history.ema_slow && showEmaSlow) pushHistoryLine(emaSlowSeries, times, history.ema_slow);
        if (history.ema_long && showEmaLong) pushHistoryLine(emaLongSeries, times, history.ema_long);
        if (history.vwap && showVwap) pushHistoryLine(vwapSeries, times, history.vwap);
        if (history.bb_upper && showBb) pushHistoryLine(bbUpper, times, history.bb_upper);
        if (history.bb_lower && showBb) pushHistoryLine(bbLower, times, history.bb_lower);
        if (history.supertrend && showSupertrend) pushHistoryLine(supertrendSeries, times, history.supertrend);
        if (history.keltner_upper && showKeltner) pushHistoryLine(keltnerUpper, times, history.keltner_upper);
        if (history.keltner_lower && showKeltner) pushHistoryLine(keltnerLower, times, history.keltner_lower);
        if (history.donchian_upper && showDonchian) pushHistoryLine(donchianUpper, times, history.donchian_upper);
        if (history.donchian_lower && showDonchian) pushHistoryLine(donchianLower, times, history.donchian_lower);
    }

    function pushHistoryLine(s: ISeriesApi<'Line'> | undefined, times: number[], arr: string[]) {
        if (!s) return;
        const data: { time: Time; value: number }[] = [];
        for (let i = 0; i < times.length; i++) {
            const v = parseFloat(arr[i]);
            if (!isNaN(v)) data.push({ time: times[i] as Time, value: v });
        }
        if (data.length > 0) s.setData(data);
    }

    function updateOverlayLine(t: Time, series: ISeriesApi<'Line'> | undefined, val: number | null) {
        if (!series || val == null) return;
        series.update({ time: t, value: val });
    }

    onMount(() => {
        if (!pair) return;
        const refPrice = parseFloat(tf?.priceText ?? '0') || 0;
        const fmt = getPriceFormat(refPrice);

        chart = createChart(container, {
            autoSize: true,
            layout: { background: { color: '#131722' }, textColor: '#8f929d', fontSize: 10 },
            grid: { vertLines: { color: '#1a1d26' }, horzLines: { color: '#1a1d26' } },
            crosshair: { mode: CrosshairMode.Normal },
            rightPriceScale: { borderColor: '#2a2e39', scaleMargins: { top: 0.05, bottom: 0.25 } },
            timeScale: { borderColor: '#2a2e39', timeVisible: true, secondsVisible: true },
            localization: { priceFormatter: (p: number) => p.toFixed(fmt.precision) },
            handleScale: true, handleScroll: true,
        });

        candleSeries = chart.addSeries(CandlestickSeries, {
            priceFormat: { type: 'price', precision: fmt.precision, minMove: fmt.minMove },
        });

        lineSeries = chart.addSeries(LineSeries, {
            color: '#d1d5db',
            lineWidth: 2,
            priceLineVisible: false,
            lastValueVisible: false,
            priceFormat: { type: 'price', precision: fmt.precision, minMove: fmt.minMove },
            visible: false,
        });

        volumeSeries = chart.addSeries(HistogramSeries, {
            priceFormat: { type: 'volume' },
            priceScaleId: 'volume',
        });
        chart.priceScale('volume').applyOptions({
            scaleMargins: { top: 0.85, bottom: 0 },
            visible: false,
        });

        chart.timeScale().applyOptions({ rightOffset: 8, barSpacing: 8 });

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

        prevLineMode = priceLineMode;
        prevShowEmaFast = showEmaFast;
        prevShowEmaMedium = showEmaMedium;
        prevShowEmaSlow = showEmaSlow;
        prevShowEmaLong = showEmaLong;
        prevShowVwap = showVwap;
        prevShowBb = showBb;
        prevShowSupertrend = showSupertrend;
        prevShowKeltner = showKeltner;
        prevShowDonchian = showDonchian;

        if (showEmaFast) ensureEmaFast();
        if (showEmaMedium) ensureEmaMedium();
        if (showEmaSlow) ensureEmaSlow();
        if (showEmaLong) ensureEmaLong();
        if (showVwap) ensureVwap();
        if (showBb) ensureBb();
        if (showSupertrend) ensureSupertrend();
        if (showKeltner) ensureKeltner();
        if (showDonchian) ensureDonchian();

        (async () => {
            if (!pair) return;
            try {
                const res = await fetch(`/api/history?symbol=${encodeURIComponent(pairKey)}&timeframe_secs=${timeframe}`);
                const data = await res.json();
                if (data.prices && data.candles) {
                    const times: number[] = [];
                    const opens: string[] = [];
                    const highs: string[] = [];
                    const lows: string[] = [];
                    const closes: string[] = [];
                    const volumes: string[] = [];

                    for (const c of data.candles) {
                        const t = Math.floor(c.time / 1000);
                        times.push(t);
                        opens.push(String(c.open));
                        highs.push(String(c.high));
                        lows.push(String(c.low));
                        closes.push(String(c.close));
                        volumes.push(String(c.volume));
                    }

                    const indicatorHistory = data.indicator_history ? flattenHistory(data.indicator_history) : null;
                    persistHistory({
                        times, opens, highs, lows, closes, volumes,
                        ...(indicatorHistory ?? {}),
                    }, priceLineMode);
                    chart.timeScale().fitContent();
                }
            } catch (err) {
                console.error('Error bootstrapping PriceChart:', err);
            }
        })();

        ro = new ResizeObserver(() => {
            const w = container.clientWidth;
            const h = container.clientHeight;
            if (chart && w > 0 && h > 0) chart.resize(w, h);
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
        toggleSeries(priceLineMode, prevLineMode,
            () => {},
            () => {}
        );
        candleSeries?.applyOptions({ visible: !priceLineMode });
        lineSeries?.applyOptions({ visible: priceLineMode });
        prevLineMode = priceLineMode;
    });

    $effect(() => {
        toggleSeries(showEmaFast, prevShowEmaFast, ensureEmaFast, () => { hideSeries(emaFastSeries); destroyOptional(emaFastSeries); emaFastSeries = undefined; });
        prevShowEmaFast = showEmaFast;
    });
    $effect(() => {
        toggleSeries(showEmaMedium, prevShowEmaMedium, ensureEmaMedium, () => { hideSeries(emaMediumSeries); destroyOptional(emaMediumSeries); emaMediumSeries = undefined; });
        prevShowEmaMedium = showEmaMedium;
    });
    $effect(() => {
        toggleSeries(showEmaSlow, prevShowEmaSlow, ensureEmaSlow, () => { hideSeries(emaSlowSeries); destroyOptional(emaSlowSeries); emaSlowSeries = undefined; });
        prevShowEmaSlow = showEmaSlow;
    });
    $effect(() => {
        toggleSeries(showEmaLong, prevShowEmaLong, ensureEmaLong, () => { hideSeries(emaLongSeries); destroyOptional(emaLongSeries); emaLongSeries = undefined; });
        prevShowEmaLong = showEmaLong;
    });

    $effect(() => {
        toggleSeries(showVwap, prevShowVwap, ensureVwap, () => { hideSeries(vwapSeries); destroyOptional(vwapSeries); vwapSeries = undefined; });
        prevShowVwap = showVwap;
    });
    $effect(() => {
        toggleSeries(showBb, prevShowBb, ensureBb, () => { hideSeries(bbUpper); hideSeries(bbLower); destroyOptional(bbUpper); destroyOptional(bbLower); bbUpper = bbLower = undefined; });
        prevShowBb = showBb;
    });
    $effect(() => {
        toggleSeries(showSupertrend, prevShowSupertrend, ensureSupertrend, () => { hideSeries(supertrendSeries); destroyOptional(supertrendSeries); supertrendSeries = undefined; });
        prevShowSupertrend = showSupertrend;
    });
    $effect(() => {
        toggleSeries(showKeltner, prevShowKeltner, ensureKeltner, () => { hideSeries(keltnerUpper); hideSeries(keltnerLower); destroyOptional(keltnerUpper); destroyOptional(keltnerLower); keltnerUpper = keltnerLower = undefined; });
        prevShowKeltner = showKeltner;
    });
    $effect(() => {
        toggleSeries(showDonchian, prevShowDonchian, ensureDonchian, () => { hideSeries(donchianUpper); hideSeries(donchianLower); destroyOptional(donchianUpper); destroyOptional(donchianLower); donchianUpper = donchianLower = undefined; });
        prevShowDonchian = showDonchian;
    });
    $effect(() => {
        toggleSeries(showIchimoku, prevShowIchimoku, ensureIchimoku, () => {
            hideSeries(ichimokuTenkan); hideSeries(ichimokuKijun); hideSeries(ichimokuSenkouA); hideSeries(ichimokuSenkouB);
            destroyOptional(ichimokuTenkan); destroyOptional(ichimokuKijun); destroyOptional(ichimokuSenkouA); destroyOptional(ichimokuSenkouB);
            ichimokuTenkan = ichimokuKijun = ichimokuSenkouA = ichimokuSenkouB = undefined;
        });
        prevShowIchimoku = showIchimoku;
    });
    $effect(() => {
        toggleSeries(showHullMa, prevShowHullMa, ensureHullMa, () => { hideSeries(hullMaSeries); destroyOptional(hullMaSeries); hullMaSeries = undefined; });
        prevShowHullMa = showHullMa;
    });
    $effect(() => {
        toggleSeries(showPsar, prevShowPsar, ensurePsar, () => { hideSeries(psarSeries); destroyOptional(psarSeries); psarSeries = undefined; });
        prevShowPsar = showPsar;
    });
    $effect(() => {
        toggleSeries(showStddevChan, prevShowStddevChan, ensureStddevChan, () => { hideSeries(stddevUpper); hideSeries(stddevLower); destroyOptional(stddevUpper); destroyOptional(stddevLower); stddevUpper = stddevLower = undefined; });
        prevShowStddevChan = showStddevChan;
    });
    $effect(() => {
        if (!showFib || !tf) { drawFib(null); return; }
        const snap = tf.latestSnapshot;
        if (!snap) return;
        const indicators = (snap.indicators ?? {}) as IndicatorMap;
        const fib = indicators['fibonacci'] ?? indicators['fib'];
        if (!fib?.values) { drawFib(null); return; }
        const mid = (snap.mid_price ?? iRaw(indicators, 'mid_price')) as number;
        if (!mid) { drawFib(null); return; }
        const entries = Object.entries(fib.values).filter(([k]) => k.startsWith('level_'));
        const lines: { time: Time; value: number }[] = [];
        for (const [k, v] of entries) {
            const val = typeof v === 'number' ? v : Number(v);
            if (!isNaN(val) && val > 0) lines.push({ time: 0 as Time, value: val });
        }
        drawFib(lines.length > 0 ? lines : null);
        prevShowFib = showFib;
    });

    $effect(() => {
        if (!pair) return;
        const snap = tf?.latestSnapshot;
        if (!snap) return;
        const timeSec = snap.timestamp as number;
        const indicators = (snap.indicators ?? {}) as IndicatorMap;

        const open  = snap.open_px ?? iRaw(indicators, 'open');
        const high  = snap.high_px ?? iRaw(indicators, 'high');
        const low   = snap.low_px ?? iRaw(indicators, 'low');
        const close = snap.close_px ?? iRaw(indicators, 'mid_price') ?? iRaw(indicators, 'close');
        const vol   = iRaw(indicators, 'volume') ?? snap.volume;

        if (open != null && high != null && low != null && close != null) {
            if (snap.is_completed) {
                candleSeries.setData(candleSeries.data().slice(-200).map(d => ({ ...d } as any)));
            }
            candleSeries.update({ time: timeSec as Time, open, high, low, close } as any);
            lineSeries.update({ time: timeSec as Time, value: close });
        }

        if (vol != null) {
            const green = close != null && open != null ? close >= open : true;
            volumeSeries.update({
                time: timeSec as Time,
                value: vol,
                color: green ? 'rgba(34,197,94,0.3)' : 'rgba(239,68,68,0.3)',
            } as any);
        }

        updateOverlayLine(timeSec as Time, emaFastSeries, iSub(indicators, 'ema_stack', 'ema_fast'));
        updateOverlayLine(timeSec as Time, emaMediumSeries, iSub(indicators, 'ema_stack', 'ema_medium'));
        updateOverlayLine(timeSec as Time, emaSlowSeries, iSub(indicators, 'ema_stack', 'ema_slow'));
        updateOverlayLine(timeSec as Time, emaLongSeries, iSub(indicators, 'ema_stack', 'ema_long'));
        updateOverlayLine(timeSec as Time, vwapSeries, iSub(indicators, 'vwap', 'vwap'));
        updateOverlayLine(timeSec as Time, bbUpper, iSub(indicators, 'bollinger', 'bb_upper'));
        updateOverlayLine(timeSec as Time, bbLower, iSub(indicators, 'bollinger', 'bb_lower'));
        updateOverlayLine(timeSec as Time, supertrendSeries, iSub(indicators, 'supertrend', 'supertrend'));
        updateOverlayLine(timeSec as Time, keltnerUpper, iSub(indicators, 'keltner', 'keltner_upper'));
        updateOverlayLine(timeSec as Time, keltnerLower, iSub(indicators, 'keltner', 'keltner_lower'));
        updateOverlayLine(timeSec as Time, donchianUpper, iSub(indicators, 'donchian', 'donchian_upper'));
        updateOverlayLine(timeSec as Time, donchianLower, iSub(indicators, 'donchian', 'donchian_lower'));
        updateOverlayLine(timeSec as Time, ichimokuTenkan, iSub(indicators, 'ichimoku', 'tenkan_sen'));
        updateOverlayLine(timeSec as Time, ichimokuKijun, iSub(indicators, 'ichimoku', 'kijun_sen'));
        updateOverlayLine(timeSec as Time, ichimokuSenkouA, iSub(indicators, 'ichimoku', 'senkou_span_a'));
        updateOverlayLine(timeSec as Time, ichimokuSenkouB, iSub(indicators, 'ichimoku', 'senkou_span_b'));
        updateOverlayLine(timeSec as Time, hullMaSeries, iSub(indicators, 'hull_ma', 'hull_ma'));
        updateOverlayLine(timeSec as Time, psarSeries, iSub(indicators, 'psar', 'psar'));
        updateOverlayLine(timeSec as Time, stddevUpper, iSub(indicators, 'stddev_channel', 'stddev_upper'));
        updateOverlayLine(timeSec as Time, stddevLower, iSub(indicators, 'stddev_channel', 'stddev_lower'));
    });
</script>

<div class="chart-container" bind:this={container}></div>

<style>
    .chart-container { width: 100%; height: 100%; }
</style>
