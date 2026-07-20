<script lang="ts">
    import { flattenHistory } from '../lib/historyAdapter';
    import { iSub, getPriceFormat, formatTimeframeLabel, resolveChartTimeframe } from '../lib/telemetry';
    import type { IndicatorMap } from '../types';
    import { onMount, onDestroy } from 'svelte';
    import { createChart, CrosshairMode, CandlestickSeries, LineSeries, LineStyle } from 'lightweight-charts';
    import type { IChartApi, ISeriesApi, Time } from 'lightweight-charts';
    import { useAppStore } from '../state.svelte';
    import { registerChart, unregisterChart } from '../chartRegistry.svelte';
    import { takeChartScreenshot } from '../lib/chartScreenshot';
    import { attachHeatmap, type LiquidationHeatmapPrimitive } from '../lib/liquidationHeatmap';
    import { attachZoneBands, type ZoneBandsPrimitive } from '../lib/zoneBands';
    import { attachStrategyLevels, buildLevelLines, type StrategyLevelsPrimitive } from '../lib/strategyLevels';

    const MAX_BARS = 1000;

    interface CandleBar {
        time: number;
        open: number;
        high: number;
        low: number;
        close: number;
        volume: string | null;
        vwap: number | null;
        ema_fast: number | null;
        ema_medium: number | null;
        ema_slow: number | null;
        ema_long: number | null;
    }

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

    const tf = $derived(resolveChartTimeframe(timeframe, pair));

    const priceLineMode = $derived((pair as any)?.priceLineMode ?? false);
    const showEmaFast    = $derived((pair as any)?.showEmaFast ?? false);
    const showEmaMedium  = $derived((pair as any)?.showEmaMedium ?? false);
    const showEmaSlow    = $derived((pair as any)?.showEmaSlow ?? false);
    const showEmaLong    = $derived((pair as any)?.showEmaLong ?? false);
    const showVwap       = $derived(tf?.showVwap ?? false);

    let container: HTMLDivElement;
    let chart: IChartApi = $state(null!);

    let candleSeries: ISeriesApi<'Candlestick'>;
    let lineSeries: ISeriesApi<'Line'>;

    let emaFastSeries: ISeriesApi<'Line'> | undefined;
    let emaMediumSeries: ISeriesApi<'Line'> | undefined;
    let emaSlowSeries: ISeriesApi<'Line'> | undefined;
    let emaLongSeries: ISeriesApi<'Line'> | undefined;
    let vwapSeries: ISeriesApi<'Line'> | undefined;
    let heatmap: LiquidationHeatmapPrimitive | undefined;
    let zoneBands: ZoneBandsPrimitive | undefined;
    let strategyLevels: StrategyLevelsPrimitive | undefined;

    let prevLineMode = $state(false);
    let prevShowEmaFast = $state(false);
    let prevShowEmaMedium = $state(false);
    let prevShowEmaSlow = $state(false);
    let prevShowEmaLong = $state(false);
    let prevShowVwap = $state(false);
    let isFullscreen = $state(false);

    // Single source of truth: the sliding window of up to MAX_BARS most recent
    // candle bars, each carrying its own OHLC + per-bar indicators. Time-
    // monotonic. Trimmed to MAX_BARS on every new bar. Rendered via renderWindow.
    // Plain `let` on purpose: re-renders are driven explicitly by renderWindow()
    // calls inside ingestHistorical/ingestLive, so wrapping in $state would only
    // turn these into spurious effect dependencies and re-trigger the timeframe
    // effect on every WS message (effect_update_depth_exceeded).
    let candleWindow: CandleBar[] = [];
    let lastBarTime: number | null = null;

    let showFibLevels = $state(false);
    let showVpLevels = $state(false);
    let showPivotLevels = $state(false);
    let showClusterLevels = $state(false);

    let historyLoading = $state(true);
    let historyError = $state<string | null>(null);

    let fetchSeq = 0;
    let abortCtrl: AbortController | null = null;
    let prevTimeframe: number | null = null;

    function toggleFullscreen() {
        isFullscreen = !isFullscreen;
        if (chart && container) {
            requestAnimationFrame(() => {
                chart.resize(container.clientWidth, container.clientHeight);
            });
        }
    }

    function screenshotChart() {
        if (chart) takeChartScreenshot(chart, `price-${pairKey}-${formatTimeframeLabel(timeframe)}`);
    }

    function handleKeydown(e: KeyboardEvent) {
        if (isFullscreen && e.key === 'Escape') toggleFullscreen();
    }

    function ensureEmaFast() {
        if (emaFastSeries) return;
        emaFastSeries = chart.addSeries(LineSeries, { color: '#fdd835', lineWidth: 2, priceLineVisible: false });
    }
    function ensureEmaMedium() {
        if (emaMediumSeries) return;
        emaMediumSeries = chart.addSeries(LineSeries, { color: '#ff9800', lineWidth: 2, priceLineVisible: false });
    }
    function ensureEmaSlow() {
        if (emaSlowSeries) return;
        emaSlowSeries = chart.addSeries(LineSeries, { color: '#e91e63', lineWidth: 2, priceLineVisible: false });
    }
    function ensureEmaLong() {
        if (emaLongSeries) return;
        emaLongSeries = chart.addSeries(LineSeries, { color: '#9c27b0', lineWidth: 2, priceLineVisible: false });
    }
    function ensureVwap() {
        if (vwapSeries) return;
        vwapSeries = chart.addSeries(LineSeries, { color: '#64ffda', lineWidth: 1, lineStyle: LineStyle.Dotted, priceLineVisible: false });
    }

    function safeFitContent() {
        if (!chart) return;
        try { chart.timeScale().fitContent(); } catch (e) { console.warn('fitContent skipped:', e); }
    }

    function safeSetData(s: ISeriesApi<any> | undefined, data: any[]) {
        if (!s || data.length === 0) return;
        try { s.setData(data); } catch (e) { console.warn('setData skipped:', e); }
    }

    function safeApplyOptions(s: ISeriesApi<any> | undefined, opts: any) {
        if (!s) return;
        try { s.applyOptions(opts); } catch (e) { console.warn('applyOptions skipped:', e); }
    }

    function numFromMaybe(s: string | null | undefined): number | null {
        if (s == null) return null;
        const v = parseFloat(s);
        return Number.isNaN(v) ? null : v;
    }

    // Build a single line series array, skipping bars whose indicator value is
    // null. Indicators that can't yet be computed (EMA before its lookback has
    // accumulated volume; VWAP until enough volume is present) simply do not
    // draw a point — no forward-fill, no leading artefact.
    function toLine(src: CandleBar[], pick: (b: CandleBar) => number | null): { time: Time; value: number }[] {
        const out: { time: Time; value: number }[] = [];
        for (const c of src) {
            const v = pick(c);
            if (v == null || Number.isNaN(v)) continue;
            out.push({ time: c.time as Time, value: v });
        }
        return out;
    }

    // Single render path: re-push every series from candleWindow.
    function renderWindow() {
        if (!chart) return;
        const candleData = candleWindow.map(c => ({
            time: c.time, open: c.open, high: c.high, low: c.low, close: c.close,
        }));
        const lineData = candleWindow.map(c => ({ time: c.time, value: c.close }));
        safeSetData(candleSeries, candleData);
        safeSetData(lineSeries, lineData);
        safeApplyOptions(candleSeries, { visible: !priceLineMode });
        safeApplyOptions(lineSeries, { visible: priceLineMode });
        if (showEmaFast && emaFastSeries)         safeSetData(emaFastSeries,   toLine(candleWindow, b => b.ema_fast));
        if (showEmaMedium && emaMediumSeries)     safeSetData(emaMediumSeries, toLine(candleWindow, b => b.ema_medium));
        if (showEmaSlow && emaSlowSeries)         safeSetData(emaSlowSeries,   toLine(candleWindow, b => b.ema_slow));
        if (showEmaLong && emaLongSeries)         safeSetData(emaLongSeries,   toLine(candleWindow, b => b.ema_long));
        if (showVwap && vwapSeries)               safeSetData(vwapSeries,      toLine(candleWindow, b => b.vwap));
        safeFitContent();
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

    // Ingest the historical API response into `candleWindow`, replacing any
    // prior contents. Bars whose OHLC are missing are skipped so the first
    // bar in the window is the first historical candle with real data. The
    // window is then capped to MAX_BARS trailing entries so the chart always
    // shows the most-recent slice and never grows unbounded.
    function ingestHistorical(history: any) {
        if (!history) return;
        const times: number[] = history.times || [];
        const built: CandleBar[] = [];
        for (let i = 0; i < times.length; i++) {
            const t = times[i];
            if (typeof t !== 'number') continue;
            const oRaw = history.opens?.[i];
            const hRaw = history.highs?.[i];
            const lRaw = history.lows?.[i];
            const cRaw = history.closes?.[i];
            if (oRaw == null || hRaw == null || lRaw == null || cRaw == null) continue;
            const open  = parseFloat(oRaw);
            const high  = parseFloat(hRaw);
            const low   = parseFloat(lRaw);
            const close = parseFloat(cRaw);
            if ([open, high, low, close].some(Number.isNaN)) continue;
            built.push({
                time: t,
                open, high, low, close,
                volume: history.volumes?.[i] ?? null,
                vwap:       numFromMaybe(history.vwap?.[i]),
                ema_fast:   numFromMaybe(history.ema_fast?.[i]),
                ema_medium: numFromMaybe(history.ema_medium?.[i]),
                ema_slow:   numFromMaybe(history.ema_slow?.[i]),
                ema_long:   numFromMaybe(history.ema_long?.[i]),
            });
        }
        candleWindow = built.length > MAX_BARS ? built.slice(-MAX_BARS) : built;
        lastBarTime = candleWindow.length > 0 ? candleWindow[candleWindow.length - 1].time : null;
        renderWindow();
    }

    // Ingest one live WS snapshot into `candleWindow`. Same-bar updates
    // overwrite the tail; new-bar updates append and evict the oldest when
    // the window exceeds MAX_BARS. Late out-of-order messages are dropped.
    function ingestLive(snap: any) {
        const t = typeof snap.timestamp === 'number' ? snap.timestamp : null;
        if (t == null) return;
        const open  = Number(snap.open  ?? snap.mid_price);
        const high  = Number(snap.high  ?? snap.mid_price);
        const low   = Number(snap.low   ?? snap.mid_price);
        const close = Number(snap.close ?? snap.mid_price);
        if (![open, high, low, close].every(Number.isFinite)) return;

        const indicators = (snap.indicators ?? {}) as IndicatorMap;
        const bar: CandleBar = {
            time: t, open, high, low, close,
            volume: snap.volume != null ? String(snap.volume) : null,
            vwap:       iSub(indicators, 'vwap',      'vwap'),
            ema_fast:   iSub(indicators, 'ema_stack', 'fast'),
            ema_medium: iSub(indicators, 'ema_stack', 'medium'),
            ema_slow:   iSub(indicators, 'ema_stack', 'slow'),
            ema_long:   iSub(indicators, 'ema_stack', 'long'),
        };

        if (lastBarTime == null) {
            candleWindow.push(bar);
        } else if (t > lastBarTime) {
            candleWindow.push(bar);
            if (candleWindow.length > MAX_BARS) candleWindow.shift();
        } else if (t === lastBarTime) {
            candleWindow[candleWindow.length - 1] = bar;
        } else {
            return; // late out-of-order message
        }
        lastBarTime = t;
        renderWindow();
    }

    async function fetchHistory(tfSecs: number) {
        if (!pair || !chart) return;
        if (abortCtrl) abortCtrl.abort();
        const ctrl = new AbortController();
        abortCtrl = ctrl;
        const seq = ++fetchSeq;

        historyLoading = true;
        historyError = null;
        try {
            const res = await fetch(
                `/api/history?symbol=${encodeURIComponent(pairKey)}&timeframe_secs=${tfSecs}&limit=1000`,
                { signal: ctrl.signal },
            );
            if (seq !== fetchSeq || !chart) return;
            if (!res.ok) throw new Error(`HTTP ${res.status}`);
            const data = await res.json();
            if (seq !== fetchSeq || !chart) return;

            if (data.prices && data.candles) {
                const times: number[] = [];
                const opens: string[] = [];
                const highs: string[] = [];
                const lows: string[] = [];
                const closes: string[] = [];
                const volumes: string[] = [];

                for (const c of data.candles) {
                    times.push(Math.floor(c.time / 1000));
                    opens.push(String(c.open));
                    highs.push(String(c.high));
                    lows.push(String(c.low));
                    closes.push(String(c.close));
                    volumes.push(c.volume != null ? String(c.volume) : '');
                }

                const indicatorHistory = data.indicator_history ? flattenHistory(data.indicator_history) : null;
                const historyData = {
                    times, opens, highs, lows, closes, volumes,
                    ...(indicatorHistory ?? {}),
                };
                if (seq !== fetchSeq || !chart) return;
                ingestHistorical(historyData);
                requestAnimationFrame(() => {
                    if (chart && container) {
                        chart.resize(container.clientWidth, container.clientHeight);
                    }
                });
            }
        } catch (err) {
            if (ctrl.signal.aborted) return;
            if (seq !== fetchSeq) return;
            historyError = err instanceof Error ? err.message : String(err);
            console.error('Error fetching PriceChart history:', err);
        } finally {
            if (seq === fetchSeq) historyLoading = false;
        }
    }

    onMount(() => {
        const refPrice = parseFloat(tf?.priceText ?? '0') || 0;
        const fmt = getPriceFormat(refPrice);

        chart = createChart(container, {
            autoSize: true,
            layout: { background: { color: '#131722' }, textColor: '#8f929d', fontSize: 10 },
            grid: { vertLines: { color: '#1a1d26' }, horzLines: { color: '#1a1d26' } },
            crosshair: { mode: CrosshairMode.Normal },
            rightPriceScale: { borderColor: '#2a2e39', scaleMargins: { top: 0.10, bottom: 0.10 } },
            timeScale: { borderColor: '#2a2e39', visible: true, timeVisible: true, secondsVisible: true },
            localization: { priceFormatter: (p: number) => p.toFixed(fmt.precision) },
            handleScale: true, handleScroll: true,
        });

        chart.priceScale('right').applyOptions({ alignLabels: true });

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

        chart.timeScale().applyOptions({ rightOffset: 8, barSpacing: 8 });

        registerChart(chart);

        heatmap = attachHeatmap(chart, candleSeries);
        zoneBands = attachZoneBands(chart, candleSeries);
        strategyLevels = attachStrategyLevels(candleSeries);

        if (onDoubleClick) chart.subscribeDblClick(onDoubleClick);

        if (onScreenshotReady) {
            onScreenshotReady(() => {
                if (!chart) return;
                const canvas = chart.takeScreenshot();
                const dataUrl = canvas.toDataURL('image/png');
                const link = document.createElement('a');
                link.download = `${pairKey}_${formatTimeframeLabel(timeframe)}_price.png`;
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

        if (showEmaFast) ensureEmaFast();
        if (showEmaMedium) ensureEmaMedium();
        if (showEmaSlow) ensureEmaSlow();
        if (showEmaLong) ensureEmaLong();
        if (showVwap) ensureVwap();

        fetchHistory(timeframe);
    });

    onDestroy(() => {
        try { if (chart) unregisterChart(chart); } catch (_) {}
        try { if (chart) chart.remove(); } catch (_) {}
        if (abortCtrl) abortCtrl.abort();
        abortCtrl = null;
        fetchSeq++;
    });

    $effect(() => {
        const tfSecs = timeframe;
        if (!chart) return;

        // Timeframe changed: drop the sliding window so we never paint stale
        // candles from the previous pipeline. The new fetch will repopulate it.
        if (prevTimeframe !== null && prevTimeframe !== tfSecs) {
            candleWindow = [];
            lastBarTime = null;
            renderWindow();
        }
        prevTimeframe = tfSecs;

        fetchHistory(tfSecs);
    });

    $effect(() => {
        if (priceLineMode !== prevLineMode) {
            safeApplyOptions(candleSeries, { visible: !priceLineMode });
            safeApplyOptions(lineSeries, { visible: priceLineMode });
        }
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
        if (!pair) return;
        const snap = tf?.latestSnapshot;
        if (!snap) return;
        // Single source of truth: ingestLive mutates candleWindow and renders
        // all series from it. zoneBands / strategyLevels stay independent.
        ingestLive(snap);

        if (zoneBands) {
            const opp = (snap as any)?.opportunity ?? null;
            zoneBands.updateData(opp);
        }

        if (strategyLevels) {
            const cluster = tf?.cluster;
            const indicators = (snap.indicators ?? {}) as IndicatorMap;
            const close = Number(snap.close ?? snap.mid_price);
            const anyShow = showFibLevels || showVpLevels || showPivotLevels || showClusterLevels;
            if (anyShow) {
                const lines = buildLevelLines(
                    indicators,
                    cluster,
                    showFibLevels,
                    showVpLevels,
                    showPivotLevels,
                    false,
                    showClusterLevels,
                    close,
                );
                strategyLevels.setLines(lines);
            } else {
                strategyLevels.setLines([]);
            }
        }
    });

    $effect(() => {
        if (!pair || !heatmap) return;
        const cluster = tf?.cluster;
        if (cluster) heatmap.updateData(cluster);
    });
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="chart-wrapper" class:fs-active={isFullscreen} ondblclick={toggleFullscreen} role="presentation">
    <div class="chart-container" bind:this={container}>
        {#if historyLoading}
            <div class="chart-loading">
                <div class="spinner"></div>
                <span class="loading-text">{!pair ? 'Waiting for data…' : `Loading ${formatTimeframeLabel(timeframe)} data…`}</span>
            </div>
        {:else if historyError}
            <div class="chart-loading">
                <span class="loading-text error">Error: {historyError}</span>
            </div>
        {/if}
    </div>
    <div class="level-toggles">
        <button class="lv-btn" class:lv-active={showFibLevels} onclick={() => showFibLevels = !showFibLevels} title="Fibonacci levels">Fib</button>
        <button class="lv-btn" class:lv-active={showVpLevels} onclick={() => showVpLevels = !showVpLevels} title="Volume Profile">VP</button>
        <button class="lv-btn" class:lv-active={showPivotLevels} onclick={() => showPivotLevels = !showPivotLevels} title="Pivot Points">Pivot</button>
        <button class="lv-btn" class:lv-active={showClusterLevels} onclick={() => showClusterLevels = !showClusterLevels} title="Liquidity Clusters">Liq</button>
    </div>
    {#if isFullscreen}
        <div class="fs-toolbar">
            <span class="fs-title">PRICE — {pairKey} · {formatTimeframeLabel(timeframe)}</span>
            <button class="fs-btn" onclick={screenshotChart}>Screenshot</button>
            <button class="fs-btn fs-close" onclick={toggleFullscreen}>✕</button>
        </div>
    {/if}
</div>

<style>
    .chart-container { width: 100%; height: 100%; position: relative; }
    .chart-wrapper { width: 100%; height: 100%; position: relative; }
    .chart-wrapper.fs-active {
        position: fixed;
        inset: 0;
        z-index: 990;
        background: #131722;
        padding: 0;
        box-sizing: border-box;
    }
    .chart-wrapper.fs-active .chart-container {
        width: 100%;
        height: 100%;
    }
    .chart-loading {
        position: absolute;
        inset: 0;
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        background: rgba(19, 23, 34, 0.92);
        z-index: 10;
        gap: 12px;
    }
    .spinner {
        width: 32px;
        height: 32px;
        border: 3px solid rgba(255,255,255,0.08);
        border-top-color: #3b82f6;
        border-radius: 50%;
        animation: spin 0.8s linear infinite;
    }
    @keyframes spin {
        to { transform: rotate(360deg); }
    }
    .loading-text {
        color: #94a3b8;
        font-size: 12px;
        font-family: var(--mono);
    }
    .loading-text.error {
        color: #ef4444;
    }
    .level-toggles {
        position: absolute;
        top: 4px;
        right: 50px;
        display: flex;
        gap: 4px;
        z-index: 5;
    }
    .lv-btn {
        padding: 2px 8px;
        border: 1px solid rgba(255,255,255,0.12);
        border-radius: 3px;
        background: rgba(10,12,18,0.85);
        color: #64748b;
        cursor: pointer;
        font-size: 10px;
        font-family: var(--mono);
        font-weight: 600;
        transition: background 0.15s, color 0.15s, border-color 0.15s;
    }
    .lv-btn:hover { color: #94a3b8; border-color: rgba(255,255,255,0.25); }
    .lv-active { color: #e2e8f0; border-color: rgba(255,255,255,0.35); background: rgba(30,30,40,0.9); }
    .fs-toolbar {
        position: absolute;
        top: 0;
        left: 0;
        right: 0;
        height: 36px;
        display: flex;
        align-items: center;
        gap: 8px;
        padding: 0 12px;
        background: rgba(10, 12, 18, 0.92);
        border-bottom: 1px solid #2a2e39;
        z-index: 992;
    }
    .fs-title {
        color: #f1f5f9;
        font-size: 12px;
        font-weight: 700;
        font-family: var(--mono);
        text-transform: uppercase;
        letter-spacing: 0.05em;
        flex: 1;
    }
    .fs-btn {
        padding: 4px 10px;
        border: 1px solid #2a2e39;
        border-radius: 4px;
        background: transparent;
        color: #888;
        cursor: pointer;
        font-size: 11px;
        font-family: var(--mono);
        transition: background 0.15s, color 0.15s;
    }
    .fs-btn:hover { background: #1a1d26; color: #fff; }
    .fs-close {
        background: none;
        border: none;
        color: #64748b;
        font-size: 16px;
        padding: 4px 8px;
        line-height: 1;
    }
    .fs-close:hover { color: #f1f5f9; }
</style>
