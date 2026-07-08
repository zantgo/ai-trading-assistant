<script lang="ts">
    import { flattenHistory } from '../lib/historyAdapter';
    import { emaStackState, vwapBias, divStatus, iSub, iRaw, getPriceFormat, getDecimalCount } from '../lib/telemetry';
    import type { IndicatorMap } from '../types';
    import { onMount, onDestroy } from 'svelte';
    import { createChart, CrosshairMode, CandlestickSeries, LineSeries, AreaSeries, LineStyle, createSeriesMarkers } from 'lightweight-charts';
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
    let avwapWeeklySeries: ISeriesApi<'Line'>;
    let avwapMonthlySeries: ISeriesApi<'Line'>;
    let avwapSwingSeries: ISeriesApi<'Line'>;
    let priceLineSeries: ISeriesApi<'Line'>;
    let supertrendSeries: ISeriesApi<'Line'>;
    let keltnerUpperSeries: ISeriesApi<'Line'>;
    let keltnerMiddleSeries: ISeriesApi<'Line'>;
    let keltnerLowerSeries: ISeriesApi<'Line'>;
    let donchianUpperSeries: ISeriesApi<'Line'>;
    let donchianMiddleSeries: ISeriesApi<'Line'>;
    let donchianLowerSeries: ISeriesApi<'Line'>;
    let ichimokuTenkanSeries: ISeriesApi<'Line'>;
    let ichimokuKijunSeries: ISeriesApi<'Line'>;
    let ichimokuSenkouASeries: ISeriesApi<'Line'>;
    let ichimokuSenkouBSeries: ISeriesApi<'Line'>;
    let ichimokuChikouSeries: ISeriesApi<'Line'>;
    let ichimokuCloudASeries: ISeriesApi<'Area'>;
    let ichimokuCloudBSeries: ISeriesApi<'Area'>;
    let psarSeries: ISeriesApi<'Line'>;
    let hmaSeries: ISeriesApi<'Line'>;
    let sdUpperSeries: ISeriesApi<'Line'>;
    let sdCenterSeries: ISeriesApi<'Line'>;
    let sdLowerSeries: ISeriesApi<'Line'>;

    let supportLines: IPriceLine[] = [];
    let resistanceLines: IPriceLine[] = [];
    let divergenceLines: IPriceLine[] = [];
    let fibGpTopLine: IPriceLine | null = null;
    let fibGpBottomLine: IPriceLine | null = null;
    let fibExt1618Line: IPriceLine | null = null;
    let fibExt2618Line: IPriceLine | null = null;
    let fibExt1272Line: IPriceLine | null = null;
    let fibRetLines: IPriceLine[] = [];
    let entryLine: IPriceLine | null = null;
    let stopLossLine: IPriceLine | null = null;
    let pivotLines: IPriceLine[] = [];
    let volumeProfileLines: IPriceLine[] = [];
    let markersApi: any = null;
    // Accumulated candlestick pattern markers, keyed by candle time.
    let patternMarkers = new Map<number, any>();
    // SMC marker maps (one per sub-indicator).
    let smcStructureMarkers = new Map<number, any>();
    let smcLiquidityMarkers = new Map<number, any>();
    let smcFvgMarkers = new Map<number, any>();
    let smcOrderBlockMarkers = new Map<number, any>();

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
            avwapWeeklySeries, avwapMonthlySeries, avwapSwingSeries,
            supertrendSeries,
            keltnerUpperSeries, keltnerMiddleSeries, keltnerLowerSeries,
            donchianUpperSeries, donchianMiddleSeries, donchianLowerSeries,
            ichimokuTenkanSeries, ichimokuKijunSeries, ichimokuSenkouASeries,
            ichimokuSenkouBSeries, ichimokuChikouSeries,
            ichimokuCloudASeries, ichimokuCloudBSeries,
            psarSeries,
            hmaSeries, sdUpperSeries, sdCenterSeries, sdLowerSeries,
        ]) {
            s?.applyOptions({ priceFormat });
        }
    }

    async function updateMarkers() {
        if (!candleSeries || !markersApi || !pair) return;
        const trades = await buildTradeMarkers(pairKey, pair.symbol, tf?.barDurationSec || 60);
        const patterns = tf?.showCandlestick ? Array.from(patternMarkers.values()) : [];
        const smc = tf?.showSmcStructure ? Array.from(smcStructureMarkers.values()) : [];
        const smcLiq = tf?.showSmcLiquidity ? Array.from(smcLiquidityMarkers.values()) : [];
        const smcFvg = tf?.showSmcFvg ? Array.from(smcFvgMarkers.values()) : [];
        const smcOb = tf?.showSmcOrderBlocks ? Array.from(smcOrderBlockMarkers.values()) : [];
        const combined = [...trades, ...patterns, ...smc, ...smcLiq, ...smcFvg, ...smcOb].sort(
            (a, b) => (a.time as number) - (b.time as number)
        );
        markersApi.setMarkers(combined);
    }

    /// Record a candlestick pattern marker from a completed-candle snapshot.
    function recordPatternMarker(timeSec: number, m: IndicatorMap) {
        const cs = m['candlestick'];
        const sigs = cs?.signals ?? [];
        const patSig = sigs.find(s => s.kind === 'PatternForming');
        if (!patSig) return;
        const bullish = patSig.direction === 'Bullish';
        const confirmed = patSig.status === 'Confirmed';
        // Short label: strip the _FORMED/_CONFIRMED suffix for the badge.
        const short = patSig.label.replace(/_(FORMED|CONFIRMED)$/, '');
        patternMarkers.set(timeSec, {
            time: timeSec as Time,
            position: bullish ? 'belowBar' : 'aboveBar',
            color: bullish ? '#26a69a' : '#ef5350',
            shape: confirmed ? (bullish ? 'arrowUp' : 'arrowDown') : 'circle',
            text: short,
        });
        // Cap the marker trail to the most recent 60 patterns.
        if (patternMarkers.size > 60) {
            const oldest = Math.min(...patternMarkers.keys());
            patternMarkers.delete(oldest);
        }
    }

    /// Record SMC Structure markers (BOS/CHoCH) on completed candles.
    function recordSmcStructureMarker(timeSec: number, m: IndicatorMap) {
        const entry = m['smc_structure'];
        if (!entry) return;
        const values = entry.values ?? {};
        const bosBull = values.bos_bullish === 1;
        const bosBear = values.bos_bearish === 1;
        const chochBull = values.choch_bullish === 1;
        const chochBear = values.choch_bearish === 1;
        if (bosBull) {
            smcStructureMarkers.set(timeSec, { time: timeSec as Time, position: 'belowBar', color: '#00e5ff', shape: 'arrowUp', text: 'BOS ↑' });
        } else if (bosBear) {
            smcStructureMarkers.set(timeSec, { time: timeSec as Time, position: 'aboveBar', color: '#ff5252', shape: 'arrowDown', text: 'BOS ↓' });
        } else if (chochBull) {
            smcStructureMarkers.set(timeSec, { time: timeSec as Time, position: 'belowBar', color: '#ffab40', shape: 'circle', text: 'CHoCH ↑' });
        } else if (chochBear) {
            smcStructureMarkers.set(timeSec, { time: timeSec as Time, position: 'aboveBar', color: '#ffab40', shape: 'circle', text: 'CHoCH ↓' });
        }
        if (smcStructureMarkers.size > 60) { const o = Math.min(...smcStructureMarkers.keys()); smcStructureMarkers.delete(o); }
    }

    /// Record SMC Liquidity sweep markers.
    function recordSmcLiquidityMarker(timeSec: number, m: IndicatorMap) {
        const entry = m['smc_liquidity'];
        if (!entry) return;
        const values = entry.values ?? {};
        if (values.sweep_buy === 1) {
            smcLiquidityMarkers.set(timeSec, { time: timeSec as Time, position: 'belowBar', color: '#64ffda', shape: 'square', text: 'Liq Buy' });
        } else if (values.sweep_sell === 1) {
            smcLiquidityMarkers.set(timeSec, { time: timeSec as Time, position: 'aboveBar', color: '#ff5252', shape: 'square', text: 'Liq Sell' });
        }
        if (smcLiquidityMarkers.size > 30) { const o = Math.min(...smcLiquidityMarkers.keys()); smcLiquidityMarkers.delete(o); }
    }

    /// Record SMC Fair Value Gap markers.
    function recordSmcFvgMarker(timeSec: number, m: IndicatorMap) {
        const entry = m['smc_fvg'];
        if (!entry) return;
        const values = entry.values ?? {};
        if (values.fvg_top != null && values.fvg_bottom != null) {
            const bullish = values.fvg_bullish === 1;
            smcFvgMarkers.set(timeSec, { time: timeSec as Time, position: bullish ? 'belowBar' : 'aboveBar', color: '#ffca28', shape: 'circle', text: bullish ? 'FVG ↑' : 'FVG ↓' });
        }
        if (smcFvgMarkers.size > 30) { const o = Math.min(...smcFvgMarkers.keys()); smcFvgMarkers.delete(o); }
    }

    /// Record SMC Order Block markers.
    function recordSmcOrderBlockMarker(timeSec: number, m: IndicatorMap) {
        const entry = m['smc_order_blocks'];
        if (!entry) return;
        const label = entry.state_label ?? '';
        if (label.includes('BULLISH_TEST')) {
            smcOrderBlockMarkers.set(timeSec, { time: timeSec as Time, position: 'belowBar', color: '#26a69a', shape: 'circle', text: 'OB ↑' });
        } else if (label.includes('BEARISH_TEST')) {
            smcOrderBlockMarkers.set(timeSec, { time: timeSec as Time, position: 'aboveBar', color: '#ef5350', shape: 'circle', text: 'OB ↓' });
        }
        if (smcOrderBlockMarkers.size > 30) { const o = Math.min(...smcOrderBlockMarkers.keys()); smcOrderBlockMarkers.delete(o); }
    }

    onMount(() => {
        chart = createChart(container, {
            autoSize: true,
            layout: { background: { color: '#0b0c10' }, textColor: '#94a3b8', fontSize: 10 },
            grid: { vertLines: { color: '#1c212e' }, horzLines: { color: '#1c212e' } },
            crosshair: { mode: CrosshairMode.Normal, vertLine: { color: '#334155', width: 1, style: 3 }, horzLine: { color: '#334155', width: 1, style: 3 } },
            rightPriceScale: { borderColor: '#2d3448', scaleMargins: { top: 0.15, bottom: 0.1 } },
            timeScale: { borderColor: '#2d3448', visible: true, timeVisible: true, secondsVisible: true },
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
        avwapWeeklySeries = chart.addSeries(LineSeries, { color: '#ffab40', lineWidth: 1, lineStyle: LineStyle.Dashed, priceLineVisible: false, crosshairMarkerVisible: false });
        avwapMonthlySeries = chart.addSeries(LineSeries, { color: '#ff6d00', lineWidth: 1, lineStyle: LineStyle.Dashed, priceLineVisible: false, crosshairMarkerVisible: false });
        avwapSwingSeries = chart.addSeries(LineSeries, { color: '#ff8a65', lineWidth: 1, lineStyle: LineStyle.Solid, priceLineVisible: false, crosshairMarkerVisible: false });
        supertrendSeries = chart.addSeries(LineSeries, { color: '#26a69a', lineWidth: 2, lineStyle: LineStyle.Solid, priceLineVisible: false, crosshairMarkerVisible: false });
        keltnerUpperSeries = chart.addSeries(LineSeries, { color: '#78909c', lineWidth: 1, lineStyle: LineStyle.Dashed, priceLineVisible: false, crosshairMarkerVisible: false });
        keltnerMiddleSeries = chart.addSeries(LineSeries, { color: '#78909c', lineWidth: 1, lineStyle: LineStyle.Dashed, priceLineVisible: false, crosshairMarkerVisible: false });
        keltnerLowerSeries = chart.addSeries(LineSeries, { color: '#78909c', lineWidth: 1, lineStyle: LineStyle.Dashed, priceLineVisible: false, crosshairMarkerVisible: false });
        donchianUpperSeries = chart.addSeries(LineSeries, { color: '#ec407a', lineWidth: 1, lineStyle: LineStyle.Solid, priceLineVisible: false, crosshairMarkerVisible: false });
        donchianMiddleSeries = chart.addSeries(LineSeries, { color: '#ec407a', lineWidth: 1, lineStyle: LineStyle.Dotted, priceLineVisible: false, crosshairMarkerVisible: false });
        donchianLowerSeries = chart.addSeries(LineSeries, { color: '#ec407a', lineWidth: 1, lineStyle: LineStyle.Solid, priceLineVisible: false, crosshairMarkerVisible: false });
        // Ichimoku Cloud — 5-line overlay (Tenkan, Kijun, Senkou A/B, Chikou).
        // Senkou lines are rendered with time-shifted data to achieve the +26
        // forward projection; Chikou is shifted −26 backward via data points.
        ichimokuTenkanSeries = chart.addSeries(LineSeries, { color: '#ea80fc', lineWidth: 1, lineStyle: LineStyle.Solid, priceLineVisible: false, crosshairMarkerVisible: false });
        ichimokuKijunSeries = chart.addSeries(LineSeries, { color: '#82b1ff', lineWidth: 1, lineStyle: LineStyle.Solid, priceLineVisible: false, crosshairMarkerVisible: false });
        ichimokuSenkouASeries = chart.addSeries(LineSeries, { color: '#69f0ae', lineWidth: 1, lineStyle: LineStyle.Dashed, priceLineVisible: false, crosshairMarkerVisible: false });
        ichimokuSenkouBSeries = chart.addSeries(LineSeries, { color: '#ff8a80', lineWidth: 1, lineStyle: LineStyle.Dashed, priceLineVisible: false, crosshairMarkerVisible: false });
        ichimokuChikouSeries = chart.addSeries(LineSeries, { color: '#b388ff', lineWidth: 1, lineStyle: LineStyle.Dotted, priceLineVisible: false, crosshairMarkerVisible: false });
        // Semi-transparent AreaSeries behind the Senkou lines for the cloud fill.
        ichimokuCloudASeries = chart.addSeries(AreaSeries, { lineColor: 'rgba(105,240,174,0)', topColor: 'rgba(105,240,174,0.08)', bottomColor: 'rgba(105,240,174,0.02)', priceLineVisible: false, crosshairMarkerVisible: false });
        ichimokuCloudBSeries = chart.addSeries(AreaSeries, { lineColor: 'rgba(255,138,128,0)', topColor: 'rgba(255,138,128,0.08)', bottomColor: 'rgba(255,138,128,0.02)', priceLineVisible: false, crosshairMarkerVisible: false });

        psarSeries = chart.addSeries(LineSeries, { color: '#ffab40', lineWidth: 2, lineStyle: LineStyle.Dotted, priceLineVisible: false, crosshairMarkerVisible: false });

        hmaSeries = chart.addSeries(LineSeries, { color: '#ff8a65', lineWidth: 2, lineStyle: LineStyle.Solid, priceLineVisible: false, crosshairMarkerVisible: false });
        sdUpperSeries = chart.addSeries(LineSeries, { color: '#a1887f', lineWidth: 1, lineStyle: LineStyle.Dashed, priceLineVisible: false, crosshairMarkerVisible: false });
        sdCenterSeries = chart.addSeries(LineSeries, { color: '#a1887f', lineWidth: 1, lineStyle: LineStyle.Dashed, priceLineVisible: false, crosshairMarkerVisible: false });
        sdLowerSeries = chart.addSeries(LineSeries, { color: '#a1887f', lineWidth: 1, lineStyle: LineStyle.Dashed, priceLineVisible: false, crosshairMarkerVisible: false });

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
                        avwapWeeklySeries.setData(mapIndicator(ind.avwap_weekly));
                        avwapMonthlySeries.setData(mapIndicator(ind.avwap_monthly));
                        avwapSwingSeries.setData(mapIndicator(ind.avwap_swing));
                        supertrendSeries.setData(mapIndicator(ind.supertrend));
                        keltnerUpperSeries.setData(mapIndicator(ind.keltner_upper));
                        keltnerMiddleSeries.setData(mapIndicator(ind.keltner_middle));
                        keltnerLowerSeries.setData(mapIndicator(ind.keltner_lower));
                        donchianUpperSeries.setData(mapIndicator(ind.donchian_upper));
                        donchianMiddleSeries.setData(mapIndicator(ind.donchian_middle));
                        donchianLowerSeries.setData(mapIndicator(ind.donchian_lower));
                        ichimokuTenkanSeries.setData(mapIndicator(ind.ichimoku_tenkan));
                        ichimokuKijunSeries.setData(mapIndicator(ind.ichimoku_kijun));
                        ichimokuSenkouASeries.setData(mapIndicator(ind.ichimoku_senkou_a));
                        ichimokuSenkouBSeries.setData(mapIndicator(ind.ichimoku_senkou_b));
                        ichimokuChikouSeries.setData(mapIndicator(ind.ichimoku_chikou));
                        ichimokuCloudASeries.setData(mapIndicator(ind.ichimoku_senkou_a));
                        ichimokuCloudBSeries.setData(mapIndicator(ind.ichimoku_senkou_b));
                        psarSeries.setData(mapIndicator(ind.psar_sar));
                        hmaSeries.setData(mapIndicator(ind.hull_ma));
                        sdUpperSeries.setData(mapIndicator(ind.stddev_upper));
                        sdCenterSeries.setData(mapIndicator(ind.stddev_center));
                        sdLowerSeries.setData(mapIndicator(ind.stddev_lower));
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
        if (!avwapWeeklySeries || !avwapMonthlySeries || !avwapSwingSeries || !pair || !tf) return;
        avwapWeeklySeries.applyOptions({ visible: tf.showAvwap });
        avwapMonthlySeries.applyOptions({ visible: tf.showAvwap });
        avwapSwingSeries.applyOptions({ visible: tf.showAvwap });
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
        ichimokuTenkanSeries.applyOptions({ visible: tf.showIchimoku });
        ichimokuKijunSeries.applyOptions({ visible: tf.showIchimoku });
        ichimokuSenkouASeries.applyOptions({ visible: tf.showIchimoku });
        ichimokuSenkouBSeries.applyOptions({ visible: tf.showIchimoku });
        ichimokuChikouSeries.applyOptions({ visible: tf.showChikou });
        ichimokuCloudASeries.applyOptions({ visible: tf.showIchimokuCloud });
        ichimokuCloudBSeries.applyOptions({ visible: tf.showIchimokuCloud });
    });

    $effect(() => {
        if (!psarSeries || !pair || !tf) return;
        psarSeries.applyOptions({ visible: tf.showPsar });
    });

    $effect(() => {
        if (!hmaSeries || !pair || !tf) return;
        hmaSeries.applyOptions({ visible: tf.showHullMa });
    });

    $effect(() => {
        if (!sdUpperSeries || !sdCenterSeries || !sdLowerSeries || !pair || !tf) return;
        sdUpperSeries.applyOptions({ visible: tf.showStdDevChnl });
        sdCenterSeries.applyOptions({ visible: tf.showStdDevChnl });
        sdLowerSeries.applyOptions({ visible: tf.showStdDevChnl });
    });

    $effect(() => {
        if (!pair) return;
        const snap = tf.latestSnapshot;
        if (!snap) return;
        const timeSec = snap.timestamp as number;
        const m = (snap.indicators ?? {}) as IndicatorMap;

        // Completed-candle candlestick pattern → accumulate a chart marker.
        if (snap.is_completed) {
            recordPatternMarker(timeSec, m);
            recordSmcStructureMarker(timeSec, m);
            recordSmcLiquidityMarker(timeSec, m);
            recordSmcFvgMarker(timeSec, m);
            recordSmcOrderBlockMarker(timeSec, m);
        }

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
        const avwapWeekly = iSub(m, 'anchored_vwap', 'weekly');
        const avwapMonthly = iSub(m, 'anchored_vwap', 'monthly');
        const avwapSwing = iSub(m, 'anchored_vwap', 'swing');

        if (emaFast != null) ema10Series.update({ time: timeSec as Time, value: emaFast });
        if (emaMedium != null) ema50Series.update({ time: timeSec as Time, value: emaMedium });
        if (emaSlow != null) ema100Series.update({ time: timeSec as Time, value: emaSlow });
        if (emaLong != null) ema200Series.update({ time: timeSec as Time, value: emaLong });
        if (bbUpper != null) bbUpperSeries.update({ time: timeSec as Time, value: bbUpper });
        if (bbMiddle != null) bbMiddleSeries.update({ time: timeSec as Time, value: bbMiddle });
        if (bbLower != null) bbLowerSeries.update({ time: timeSec as Time, value: bbLower });
        if (vwapVal != null) vwapSeries.update({ time: timeSec as Time, value: vwapVal });
        if (avwapWeekly != null) avwapWeeklySeries.update({ time: timeSec as Time, value: avwapWeekly });
        if (avwapMonthly != null) avwapMonthlySeries.update({ time: timeSec as Time, value: avwapMonthly });
        if (avwapSwing != null) avwapSwingSeries.update({ time: timeSec as Time, value: avwapSwing });

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

        // Ichimoku Cloud: five lines from the nested indicator map.
        const iTenkan = iSub(m, 'ichimoku', 'tenkan');
        const iKijun = iSub(m, 'ichimoku', 'kijun');
        const iSenkouA = iSub(m, 'ichimoku', 'senkou_a');
        const iSenkouB = iSub(m, 'ichimoku', 'senkou_b');
        const iChikou = iSub(m, 'ichimoku', 'chikou');
        if (iTenkan != null) ichimokuTenkanSeries.update({ time: timeSec as Time, value: iTenkan });
        if (iKijun != null) ichimokuKijunSeries.update({ time: timeSec as Time, value: iKijun });
        if (iSenkouA != null) ichimokuSenkouASeries.update({ time: timeSec as Time, value: iSenkouA });
        if (iSenkouB != null) ichimokuSenkouBSeries.update({ time: timeSec as Time, value: iSenkouB });
        if (iChikou != null) ichimokuChikouSeries.update({ time: timeSec as Time, value: iChikou });
        if (iSenkouA != null) ichimokuCloudASeries.update({ time: timeSec as Time, value: iSenkouA });
        if (iSenkouB != null) ichimokuCloudBSeries.update({ time: timeSec as Time, value: iSenkouB });

        const psarVal = iSub(m, 'psar', 'sar');
        if (psarVal != null) psarSeries.update({ time: timeSec as Time, value: psarVal });

        const hmaVal = iRaw(m, 'hull_ma');
        if (hmaVal != null) hmaSeries.update({ time: timeSec as Time, value: hmaVal });
        const sdU = iSub(m, 'stddev_channel', 'upper');
        const sdC = iSub(m, 'stddev_channel', 'center');
        const sdL = iSub(m, 'stddev_channel', 'lower');
        if (sdU != null) sdUpperSeries.update({ time: timeSec as Time, value: sdU });
        if (sdC != null) sdCenterSeries.update({ time: timeSec as Time, value: sdC });
        if (sdL != null) sdLowerSeries.update({ time: timeSec as Time, value: sdL });
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
                    color: '#94a3b8',
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
                color: '#94a3b8',
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
            const lineColor = isConfirmed ? '#22c55e' : '#94a3b8';
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
        if (fibExt1272Line) { candleSeries.removePriceLine(fibExt1272Line); fibExt1272Line = null; }
        fibRetLines.forEach(l => candleSeries.removePriceLine(l));
        fibRetLines = [];
        if (!pair || !tf.showFib) return;

        const snap = tf.latestSnapshot;
        if (!snap) return;

        const fm = (snap.indicators ?? {}) as IndicatorMap;
        const gpLow = iSub(fm, 'fibonacci', 'gp_bottom');
        const gpHigh = iSub(fm, 'fibonacci', 'gp_top');
        const ext1618 = iSub(fm, 'fibonacci', 'ext_1618');
        const ext2618 = iSub(fm, 'fibonacci', 'ext_2618');
        const ext1272 = iSub(fm, 'fibonacci', 'ext_1272');

        const retLevels: [string, string, string][] = [
            ['ret_0236', '23.6%', '#64748b'],
            ['ret_0382', '38.2%', '#64748b'],
            ['ret_0500', '50.0%', '#94a3b8'],
            ['ret_0786', '78.6%', '#f59e0b'],
        ];
        for (const [subKey, title, color] of retLevels) {
            const val = iSub(fm, 'fibonacci', subKey);
            if (val != null && val > 0) {
                fibRetLines.push(candleSeries.createPriceLine({
                    price: val, color,
                    lineWidth: 1, lineStyle: 3,
                    axisLabelVisible: false, title,
                }));
            }
        }

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
        if (ext1272 != null && ext1272 > 0) {
            fibExt1272Line = candleSeries.createPriceLine({
                price: ext1272,
                color: '#81c784',
                lineWidth: 1,
                lineStyle: 2,
                axisLabelVisible: true,
                title: '1.272 Ext',
            });
        }
    });

    // Session Pivot Points — seven static horizontal levels (P/R1-3/S1-3).
    $effect(() => {
        if (!candleSeries) return;
        pivotLines.forEach(l => candleSeries.removePriceLine(l));
        pivotLines = [];
        if (!pair || !tf.showPivots) return;

        const snap = tf.latestSnapshot;
        if (!snap) return;
        const pm = (snap.indicators ?? {}) as IndicatorMap;

        const levels: Array<[string, string, string]> = [
            ['r3', 'R3', '#ef5350'],
            ['r2', 'R2', '#ef5350'],
            ['r1', 'R1', '#ef5350'],
            ['pivot', 'P', '#8d6e63'],
            ['s1', 'S1', '#26a69a'],
            ['s2', 'S2', '#26a69a'],
            ['s3', 'S3', '#26a69a'],
        ];
        for (const [key, title, color] of levels) {
            const price = iSub(pm, 'pivot_points', key);
            if (price != null && price > 0) {
                pivotLines.push(candleSeries.createPriceLine({
                    price,
                    color,
                    lineWidth: 1,
                    lineStyle: key === 'pivot' ? 0 : 3,
                    axisLabelVisible: true,
                    title,
                }));
            }
        }
    });

    // Volume Profile — POC/VAH/VAL horizontal price levels.
    $effect(() => {
        if (!candleSeries) return;
        volumeProfileLines.forEach(l => candleSeries.removePriceLine(l));
        volumeProfileLines = [];
        if (!pair || !tf.showVolumeProfile) return;

        const snap = tf.latestSnapshot;
        if (!snap) return;
        const vm = (snap.indicators ?? {}) as IndicatorMap;
        const poc = iSub(vm, 'volume_profile', 'poc');
        const vah = iSub(vm, 'volume_profile', 'vah');
        const val = iSub(vm, 'volume_profile', 'val');
        if (poc != null && poc > 0) {
            volumeProfileLines.push(candleSeries.createPriceLine({
                price: poc,
                color: '#bcaaa4',
                lineWidth: 2,
                lineStyle: 0,
                axisLabelVisible: true,
                title: 'POC',
            }));
        }
        if (vah != null && vah > 0) {
            volumeProfileLines.push(candleSeries.createPriceLine({
                price: vah,
                color: '#ef5350',
                lineWidth: 1,
                lineStyle: 3,
                axisLabelVisible: true,
                title: 'VAH',
            }));
        }
        if (val != null && val > 0) {
            volumeProfileLines.push(candleSeries.createPriceLine({
                price: val,
                color: '#26a69a',
                lineWidth: 1,
                lineStyle: 3,
                axisLabelVisible: true,
                title: 'VAL',
            }));
        }
        const hvn = iSub(vm, 'volume_profile', 'hvn');
        if (hvn != null && hvn > 0) {
            volumeProfileLines.push(candleSeries.createPriceLine({
                price: hvn, color: '#bcaaa4', lineWidth: 1, lineStyle: 1,
                axisLabelVisible: false, title: 'HVN',
            }));
        }
        const lvn = iSub(vm, 'volume_profile', 'lvn');
        if (lvn != null && lvn > 0) {
            volumeProfileLines.push(candleSeries.createPriceLine({
                price: lvn, color: '#78909c', lineWidth: 1, lineStyle: 3,
                axisLabelVisible: false, title: 'LVN',
            }));
        }
    });

    // SMC Zone Overlays — OB zones and FVGs as price-line bands
    let smcZoneLines: IPriceLine[] = [];
    $effect(() => {
        smcZoneLines.forEach(l => candleSeries?.removePriceLine(l));
        smcZoneLines = [];
        if (!pair || !candleSeries) return;

        const snap = tf.latestSnapshot;
        if (!snap) return;
        const sm = (snap.indicators ?? {}) as IndicatorMap;

        // Order Block zones
        if (tf.showSmcOrderBlocks) {
            const obBh = iSub(sm, 'smc_order_blocks', 'ob_bullish_high');
            const obBl = iSub(sm, 'smc_order_blocks', 'ob_bullish_low');
            if (obBh != null && obBl != null && obBh > 0) {
                smcZoneLines.push(candleSeries.createPriceLine({ price: obBh, color: 'rgba(16,185,129,0.4)', lineWidth: 3, lineStyle: 0, axisLabelVisible: true, title: 'OB Bull' }));
                smcZoneLines.push(candleSeries.createPriceLine({ price: obBl, color: 'rgba(16,185,129,0.4)', lineWidth: 3, lineStyle: 0, axisLabelVisible: false, title: '' }));
            }
            const obSh = iSub(sm, 'smc_order_blocks', 'ob_bearish_high');
            const obSl = iSub(sm, 'smc_order_blocks', 'ob_bearish_low');
            if (obSh != null && obSl != null && obSh > 0) {
                smcZoneLines.push(candleSeries.createPriceLine({ price: obSh, color: 'rgba(239,68,68,0.4)', lineWidth: 3, lineStyle: 0, axisLabelVisible: true, title: 'OB Bear' }));
                smcZoneLines.push(candleSeries.createPriceLine({ price: obSl, color: 'rgba(239,68,68,0.4)', lineWidth: 3, lineStyle: 0, axisLabelVisible: false, title: '' }));
            }
        }

        // FVG gap zones
        if (tf.showSmcFvg) {
            const fvgTop = iSub(sm, 'smc_fvg', 'fvg_top');
            const fvgBot = iSub(sm, 'smc_fvg', 'fvg_bottom');
            if (fvgTop != null && fvgBot != null && fvgTop > 0) {
                smcZoneLines.push(candleSeries.createPriceLine({ price: fvgTop, color: 'rgba(255,202,40,0.5)', lineWidth: 2, lineStyle: 0, axisLabelVisible: true, title: 'FVG Top' }));
                smcZoneLines.push(candleSeries.createPriceLine({ price: fvgBot, color: 'rgba(255,202,40,0.5)', lineWidth: 2, lineStyle: 0, axisLabelVisible: false, title: 'FVG Bot' }));
            }
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

    // Chart Pattern Trendline Overlays
    let patternLines: IPriceLine[] = [];
    $effect(() => {
        patternLines.forEach(l => candleSeries?.removePriceLine(l));
        patternLines = [];
        if (!pair || !candleSeries || !tf.showPatterns) return;
        const snap = tf.latestSnapshot;
        if (!snap) return;
        const pm = (snap.indicators ?? {}) as IndicatorMap;
        const pat = pm['patterns'];
        if (!pat || pat.state_label === 'NO_PATTERN') return;

        const upperSlope = iSub(pm, 'patterns', 'upper_slope');
        const upperIntercept = iSub(pm, 'patterns', 'upper_intercept');
        const lowerSlope = iSub(pm, 'patterns', 'lower_slope');
        const lowerIntercept = iSub(pm, 'patterns', 'lower_intercept');
        const currentBar = (snap.timestamp as number) || Date.now() / 1000;
        const barIdx = currentBar / (tf.barDurationSec || 60);

        const color = pat.state_label.includes('BULLISH') ? '#26a69a' : '#ef5350';

        if (upperSlope != null && upperIntercept != null) {
            const price = upperSlope * barIdx + upperIntercept;
            if (price > 0) {
                patternLines.push(candleSeries.createPriceLine({
                    price, color, lineWidth: 1, lineStyle: 2,
                    axisLabelVisible: true, title: 'PAT ↑',
                }));
            }
        }
        if (lowerSlope != null && lowerIntercept != null) {
            const price = lowerSlope * barIdx + lowerIntercept;
            if (price > 0) {
                patternLines.push(candleSeries.createPriceLine({
                    price, color, lineWidth: 1, lineStyle: 2,
                    axisLabelVisible: false, title: 'PAT ↓',
                }));
            }
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
