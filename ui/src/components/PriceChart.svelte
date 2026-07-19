<script lang="ts">
    import { flattenHistory } from '../lib/historyAdapter';
    import { iSub, getPriceFormat } from '../lib/telemetry';
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
    let storedHistory: any = null;

    let showFibLevels = $state(false);
    let showVpLevels = $state(false);
    let showPivotLevels = $state(false);
    let showClusterLevels = $state(false);

    function toggleFullscreen() {
        isFullscreen = !isFullscreen;
        if (chart && container) {
            requestAnimationFrame(() => {
                chart.resize(container.clientWidth, container.clientHeight);
            });
        }
    }

    function screenshotChart() {
        if (chart) takeChartScreenshot(chart, `price-${pairKey}-${timeframe}s`);
    }

    function handleKeydown(e: KeyboardEvent) {
        if (isFullscreen && e.key === 'Escape') toggleFullscreen();
    }

    function ensureEmaFast() {
        if (emaFastSeries) return;
        emaFastSeries = chart.addSeries(LineSeries, { color: '#fdd835', lineWidth: 2, priceLineVisible: false });
        if (storedHistory?.ema_fast) pushHistoryLine(emaFastSeries, storedHistory.times, storedHistory.ema_fast);
    }
    function ensureEmaMedium() {
        if (emaMediumSeries) return;
        emaMediumSeries = chart.addSeries(LineSeries, { color: '#ff9800', lineWidth: 2, priceLineVisible: false });
        if (storedHistory?.ema_medium) pushHistoryLine(emaMediumSeries, storedHistory.times, storedHistory.ema_medium);
    }
    function ensureEmaSlow() {
        if (emaSlowSeries) return;
        emaSlowSeries = chart.addSeries(LineSeries, { color: '#e91e63', lineWidth: 2, priceLineVisible: false });
        if (storedHistory?.ema_slow) pushHistoryLine(emaSlowSeries, storedHistory.times, storedHistory.ema_slow);
    }
    function ensureEmaLong() {
        if (emaLongSeries) return;
        emaLongSeries = chart.addSeries(LineSeries, { color: '#9c27b0', lineWidth: 2, priceLineVisible: false });
        if (storedHistory?.ema_long) pushHistoryLine(emaLongSeries, storedHistory.times, storedHistory.ema_long);
    }
    function ensureVwap() {
        if (vwapSeries) return;
        vwapSeries = chart.addSeries(LineSeries, { color: '#64ffda', lineWidth: 1, lineStyle: LineStyle.Dotted, priceLineVisible: false });
        if (storedHistory?.vwap) pushHistoryLine(vwapSeries, storedHistory.times, storedHistory.vwap);
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

        for (let i = 0; i < len; i++) {
            const t = times[i] as Time;
            const open  = parseFloat(history.opens?.[i] ?? '0') || 0;
            const high  = parseFloat(history.highs?.[i] ?? '0') || 0;
            const low   = parseFloat(history.lows?.[i] ?? '0') || 0;
            const close = parseFloat(history.closes?.[i] ?? '0') || 0;

            if (!isNaN(open) && !isNaN(high) && !isNaN(low) && !isNaN(close)) {
                candleData.push({ time: t, open, high, low, close });
                lineData.push({ time: t, value: close });
            }
        }

        if (candleData.length > 0) {
            candleSeries.setData(candleData);
            lineSeries.setData(lineData);
            candleSeries.applyOptions({ visible: !isLineMode });
            lineSeries.applyOptions({ visible: isLineMode });
        }

        if (history.ema_fast && showEmaFast) pushHistoryLine(emaFastSeries, times, history.ema_fast);
        if (history.ema_medium && showEmaMedium) pushHistoryLine(emaMediumSeries, times, history.ema_medium);
        if (history.ema_slow && showEmaSlow) pushHistoryLine(emaSlowSeries, times, history.ema_slow);
        if (history.ema_long && showEmaLong) pushHistoryLine(emaLongSeries, times, history.ema_long);
        if (history.vwap && showVwap) pushHistoryLine(vwapSeries, times, history.vwap);
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

        if (showEmaFast) ensureEmaFast();
        if (showEmaMedium) ensureEmaMedium();
        if (showEmaSlow) ensureEmaSlow();
        if (showEmaLong) ensureEmaLong();
        if (showVwap) ensureVwap();

        (async () => {
            if (!pair) return;
            try {
                const res = await fetch(`/api/history?symbol=${encodeURIComponent(pairKey)}&timeframe_secs=${timeframe}&limit=1000`);
                const data = await res.json();
                if (data.prices && data.candles) {
                    const times: number[] = [];
                    const opens: string[] = [];
                    const highs: string[] = [];
                    const lows: string[] = [];
                    const closes: string[] = [];

                    for (const c of data.candles) {
                        const t = Math.floor(c.time / 1000);
                        times.push(t);
                        opens.push(String(c.open));
                        highs.push(String(c.high));
                        lows.push(String(c.low));
                        closes.push(String(c.close));
                    }

                    const indicatorHistory = data.indicator_history ? flattenHistory(data.indicator_history) : null;
                    const historyData = {
                        times, opens, highs, lows, closes,
                        ...(indicatorHistory ?? {}),
                    };
                    storedHistory = historyData;
                    persistHistory(historyData, priceLineMode);
                    chart.timeScale().fitContent();
                    requestAnimationFrame(() => {
                        if (chart && container) {
                            chart.resize(container.clientWidth, container.clientHeight);
                        }
                    });
                }
            } catch (err) {
                console.error('Error bootstrapping PriceChart:', err);
            }
        })();
    });

    onDestroy(() => {
        try { if (chart) unregisterChart(chart); } catch (_) {}
        try { if (chart) chart.remove(); } catch (_) {}
    });

    $effect(() => {
        if (priceLineMode !== prevLineMode) {
            candleSeries?.applyOptions({ visible: !priceLineMode });
            lineSeries?.applyOptions({ visible: priceLineMode });
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
        const timeSec = snap.timestamp as number;
        const indicators = (snap.indicators ?? {}) as IndicatorMap;

        const open  = Number(snap.open ?? snap.mid_price);
        const high  = Number(snap.high ?? snap.mid_price);
        const low   = Number(snap.low ?? snap.mid_price);
        const close = Number(snap.close ?? snap.mid_price);

        if (open != null && high != null && low != null && close != null) {
            candleSeries.update({ time: timeSec as Time, open, high, low, close } as any);
            lineSeries.update({ time: timeSec as Time, value: close } as any);
        }

        updateOverlayLine(timeSec as Time, emaFastSeries, iSub(indicators, 'ema_stack', 'ema_fast'));
        updateOverlayLine(timeSec as Time, emaMediumSeries, iSub(indicators, 'ema_stack', 'ema_medium'));
        updateOverlayLine(timeSec as Time, emaSlowSeries, iSub(indicators, 'ema_stack', 'ema_slow'));
        updateOverlayLine(timeSec as Time, emaLongSeries, iSub(indicators, 'ema_stack', 'ema_long'));
        updateOverlayLine(timeSec as Time, vwapSeries, iSub(indicators, 'vwap', 'vwap'));

        if (zoneBands) {
            const opp = (snap as any)?.opportunity ?? null;
            zoneBands.updateData(opp);
        }

        if (strategyLevels) {
            const cluster = tf?.cluster;
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
    <div class="chart-container" bind:this={container}></div>
    <div class="level-toggles">
        <button class="lv-btn" class:lv-active={showFibLevels} onclick={() => showFibLevels = !showFibLevels} title="Fibonacci levels">Fib</button>
        <button class="lv-btn" class:lv-active={showVpLevels} onclick={() => showVpLevels = !showVpLevels} title="Volume Profile">VP</button>
        <button class="lv-btn" class:lv-active={showPivotLevels} onclick={() => showPivotLevels = !showPivotLevels} title="Pivot Points">Pivot</button>
        <button class="lv-btn" class:lv-active={showClusterLevels} onclick={() => showClusterLevels = !showClusterLevels} title="Liquidity Clusters">Liq</button>
    </div>
    {#if isFullscreen}
        <div class="fs-toolbar">
            <span class="fs-title">PRICE — {pairKey} · {timeframe}s</span>
            <button class="fs-btn" onclick={screenshotChart}>Screenshot</button>
            <button class="fs-btn fs-close" onclick={toggleFullscreen}>✕</button>
        </div>
    {/if}
</div>

<style>
    .chart-container { width: 100%; height: 100%; }
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
        font-family: monospace;
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
        font-family: monospace;
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
        font-family: monospace;
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
