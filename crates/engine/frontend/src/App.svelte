<script lang="ts">
    //! # Svelte 5 Dynamic Trading Terminal Dashboard
    //! 
    //! Coordinates system components: handles WebSocket streams, synchronizes chart 
    //! legends, and drives 7 separate TradingView charts (including Volume and ATR).
    //! Features dynamic overlay toggling (EMAs, BB, and VWAP) via Svelte 5 $effect() runes.

    import { onMount, onDestroy } from 'svelte';
    import { 
        createChart, 
        CrosshairMode, 
        CandlestickSeries, 
        LineSeries, 
        HistogramSeries 
    } from 'lightweight-charts';
    import type { IChartApi, ISeriesApi } from 'lightweight-charts';

    // Direct DOM bindings
    let priceContainer: HTMLDivElement;
    let volumeContainer: HTMLDivElement;
    let adxContainer: HTMLDivElement;
    let atrContainer: HTMLDivElement;
    let rsiContainer: HTMLDivElement;
    let macdContainer: HTMLDivElement;
    let squeezeContainer: HTMLDivElement;

    // Squeeze 5 Runes state declarations
    let isConnected = $state(false);
    let priceText = $state('--');
    let emaFastText = $state('--');
    let emaMediumText = $state('--');
    let emaSlowText = $state('--');
    let emaLongText = $state('--');
    let adxText = $state('--');
    let atrText = $state('--'); // Standalone ATR
    let rsiText = $state('--');
    let macdLineText = $state('--');
    let macdSigText = $state('--');
    let macdHistText = $state('--');
    let sqzValText = $state('--');
    let sqzStatusText = $state('Calculating');
    let isSqueezeOn = $state(false);
    let volText = $state('--');
    let vwapText = $state('--'); // Standalone VWAP

    // Dynamic Top Bar Variables
    let activeSymbol = $state('ETH');
    let candleTimeframeLabel = $state('5s');

    // Dynamic Legend Labels synchronized from TOML config
    let emaFastLabel = $state('EMA Fast');
    let emaMediumLabel = $state('EMA Med');
    let emaSlowLabel = $state('EMA Slow');
    let emaLongLabel = $state('EMA Long');
    let rsiLabel = $state('RSI (14)');
    let adxLabel = $state('ADX (14)');
    let atrLabel = $state('ATR (14)');
    let macdLabel = $state('MACD (12, 26, 9)');

    // Overlay Visibility Toggles
    let showEmas = $state(true);
    let showBb = $state(true);
    let showVwap = $state(true);

    // Panel Visibility states (Svelte 5 reactive states)
    let showVolume = $state(true);
    let showAdx = $state(true);
    let showAtr = $state(true);
    let showRsi = $state(true);
    let showMacd = $state(true);
    let showSqueeze = $state(true);

    // System variables
    let charts: IChartApi[] = [];
    let ws: WebSocket | null = null;
    let lastCandle: any = null;
    let lastMacdHist = 0;
    let lastSqzMom = 0;
    let barDurationSec = 5;

    // Series APIs
    let candleSeries: ISeriesApi<'Candlestick'>;
    let ema10Series: ISeriesApi<'Line'>;
    let ema50Series: ISeriesApi<'Line'>;
    let ema100Series: ISeriesApi<'Line'>;
    let ema200Series: ISeriesApi<'Line'>;
    
    let bbUpperSeries: ISeriesApi<'Line'>;
    let bbMiddleSeries: ISeriesApi<'Line'>;
    let bbLowerSeries: ISeriesApi<'Line'>;
    let vwapSeries: ISeriesApi<'Line'>;

    let volumeSeries: ISeriesApi<'Histogram'>;
    let adxSeries: ISeriesApi<'Line'>;
    let atrSeries: ISeriesApi<'Line'>;
    let rsiSeries: ISeriesApi<'Line'>;
    let macdLineSeries: ISeriesApi<'Line'>;
    let macdSigSeries: ISeriesApi<'Line'>;
    let macdHistSeries: ISeriesApi<'Histogram'>;
    let squeezeMomSeries: ISeriesApi<'Histogram'>;
    let squeezeDotSeries: ISeriesApi<'Histogram'>;

    const chartBaseOptions = {
        autoSize: true,
        layout: {
            background: { color: '#131722' },
            textColor: '#8f929d',
            fontSize: 10,
        },
        grid: {
            vertLines: { color: '#1a1d26' },
            horzLines: { color: '#1a1d26' },
        },
        crosshair: {
            mode: CrosshairMode.Normal,
            vertLine: { color: '#4c525e', width: 1, style: 3 },
            horzLine: { color: '#4c525e', width: 1, style: 3 },
        },
        rightPriceScale: {
            borderColor: '#2a2e39',
            scaleMargins: { top: 0.15, bottom: 0.1 },
        },
        timeScale: {
            borderColor: '#2a2e39',
            visible: false,
            timeVisible: true,
            secondsVisible: true,
        },
        handleScale: false,
        handleScroll: false,
    };

    // Svelte 5 $effect() runes automatically handle visibility updates
    $effect(() => {
        if (ema10Series && ema50Series && ema100Series && ema200Series) {
            ema10Series.applyOptions({ visible: showEmas });
            ema50Series.applyOptions({ visible: showEmas });
            ema100Series.applyOptions({ visible: showEmas });
            ema200Series.applyOptions({ visible: showEmas });
        }
    });

    $effect(() => {
        if (bbUpperSeries && bbMiddleSeries && bbLowerSeries) {
            bbUpperSeries.applyOptions({ visible: showBb });
            bbMiddleSeries.applyOptions({ visible: showBb });
            bbLowerSeries.applyOptions({ visible: showBb });
        }
    });

    $effect(() => {
        if (vwapSeries) {
            vwapSeries.applyOptions({ visible: showVwap });
        }
    });

    onMount(async () => {
        try {
            const res = await fetch('/api/config');
            const config = await res.json();

            barDurationSec = config.candles.duration_seconds;
            candleTimeframeLabel = `${barDurationSec}s`;

            emaFastLabel = `EMA ${config.indicators.ema_fast}`;
            emaMediumLabel = `EMA ${config.indicators.ema_medium}`;
            emaSlowLabel = `EMA ${config.indicators.ema_slow}`;
            emaLongLabel = `EMA ${config.indicators.ema_long}`;
            rsiLabel = `RSI (${config.indicators.rsi_period})`;
            adxLabel = `ADX (${config.indicators.adx_period})`;
            atrLabel = `ATR (${config.indicators.atr_period})`;
            macdLabel = `MACD (${config.indicators.macd_fast}, ${config.indicators.macd_slow}, ${config.indicators.macd_signal})`;
        } catch (e) {
            console.error("⚠️ Failed to synchronize dynamic legends from config API, using defaults:", e);
        }

        const priceChart = createChart(priceContainer, chartBaseOptions);
        const volumeChart = createChart(volumeContainer, chartBaseOptions);
        const adxChart = createChart(adxContainer, chartBaseOptions);
        const atrChart = createChart(atrContainer, chartBaseOptions);
        const rsiChart = createChart(rsiContainer, chartBaseOptions);
        const macdChart = createChart(macdContainer, chartBaseOptions);
        const squeezeChart = createChart(squeezeContainer, {
            ...chartBaseOptions,
            timeScale: {
                borderColor: '#2a2e39',
                visible: true,
                timeVisible: true,
                secondsVisible: true,
            },
            handleScale: true,
            handleScroll: true,
        });

        // Track 7 synchronized charts in lockstep
        charts = [priceChart, volumeChart, adxChart, atrChart, rsiChart, macdChart, squeezeChart];

        // Register series
        candleSeries = priceChart.addSeries(CandlestickSeries, {
            upColor: '#26a69a', downColor: '#ef5350', borderVisible: false,
            wickUpColor: '#26a69a', wickDownColor: '#ef5350'
        });
        ema10Series = priceChart.addSeries(LineSeries, { color: '#2962ff', lineWidth: 1.5, priceLineVisible: false, crosshairMarkerVisible: false });
        ema50Series = priceChart.addSeries(LineSeries, { color: '#ff9800', lineWidth: 1.5, priceLineVisible: false, crosshairMarkerVisible: false });
        ema100Series = priceChart.addSeries(LineSeries, { color: '#e91e63', lineWidth: 1.5, priceLineVisible: false, crosshairMarkerVisible: false });
        ema200Series = priceChart.addSeries(LineSeries, { color: '#9c27b0', lineWidth: 1.5, priceLineVisible: false, crosshairMarkerVisible: false });

        // Bollinger Bands cyan-dashed overlays inside Price Box
        bbUpperSeries = priceChart.addSeries(LineSeries, { color: '#00bcd4', lineWidth: 1.0, lineStyle: 2, priceLineVisible: false, crosshairMarkerVisible: false });
        bbMiddleSeries = priceChart.addSeries(LineSeries, { color: '#00bcd4', lineWidth: 1.0, lineStyle: 2, priceLineVisible: false, crosshairMarkerVisible: false });
        bbLowerSeries = priceChart.addSeries(LineSeries, { color: '#00bcd4', lineWidth: 1.0, lineStyle: 2, priceLineVisible: false, crosshairMarkerVisible: false });
        
        // VWAP orange-dashed overlay inside Price Box
        vwapSeries = priceChart.addSeries(LineSeries, { color: '#e67e22', lineWidth: 1.5, lineStyle: 1, priceLineVisible: false, crosshairMarkerVisible: false });

        // Volume Panel (Bar Histogram)
        volumeSeries = volumeChart.addSeries(HistogramSeries, { base: 0, priceLineVisible: false });

        adxSeries = adxChart.addSeries(LineSeries, { color: '#f1c40f', lineWidth: 1.5, priceLineVisible: false });
        atrSeries = atrChart.addSeries(LineSeries, { color: '#9b59b6', lineWidth: 1.5, priceLineVisible: false }); // Standalone ATR
        rsiSeries = rsiChart.addSeries(LineSeries, { color: '#7e57c2', lineWidth: 1.5, priceLineVisible: false });

        macdLineSeries = macdChart.addSeries(LineSeries, { color: '#2962ff', lineWidth: 1.5, priceLineVisible: false });
        macdSigSeries = macdChart.addSeries(LineSeries, { color: '#ff9800', lineWidth: 1.5, priceLineVisible: false });
        macdHistSeries = macdChart.addSeries(HistogramSeries, { base: 0, priceLineVisible: false });

        squeezeMomSeries = squeezeChart.addSeries(HistogramSeries, { base: 0, priceLineVisible: false });
        squeezeDotSeries = squeezeChart.addSeries(HistogramSeries, { base: 0, priceLineVisible: false });

        charts.forEach(chart => {
            chart.priceScale('right').applyOptions({ alignLabels: true });
            chart.timeScale().applyOptions({ rightOffset: 12, barSpacing: 6 });
        });

        let isSyncing = false;
        charts.forEach((chart, index) => {
            chart.timeScale().subscribeVisibleLogicalRangeChange((range) => {
                if (isSyncing || !range) return;
                isSyncing = true;
                charts.forEach((otherChart, otherIndex) => {
                    if (index !== otherIndex) {
                        otherChart.timeScale().setVisibleLogicalRange(range);
                    }
                });
                isSyncing = false;
            });
        });

        connect();
    });

    onDestroy(() => {
        if (ws) ws.close();
        charts.forEach(chart => chart.remove());
    });

    function connect() {
        ws = new WebSocket(`ws://${window.location.host}/ws`);

        ws.onopen = () => {
            isConnected = true;
        };

        ws.onclose = () => {
            isConnected = false;
            setTimeout(connect, 3000);
        };

        ws.onmessage = (event) => {
            const data = JSON.parse(event.data);
            const timeSec = data.timestamp;

            const closePrice = data.close ? parseFloat(data.close) : parseFloat(data.mid_price);

            if (data.symbol) {
                activeSymbol = data.symbol;
            }

            // Update text labels
            priceText = `$${closePrice.toFixed(2)}`;
            emaFastText = data.ema_fast ? parseFloat(data.ema_fast).toFixed(2) : "--";
            emaMediumText = data.ema_medium ? parseFloat(data.ema_medium).toFixed(2) : "--";
            emaSlowText = data.ema_slow ? parseFloat(data.ema_slow).toFixed(2) : "--";
            emaLongText = data.ema_long ? parseFloat(data.ema_long).toFixed(2) : "--";
            rsiText = data.rsi_14 ? parseFloat(data.rsi_14).toFixed(2) : "--";
            adxText = data.adx_14 ? parseFloat(data.adx_14).toFixed(2) : "--";
            atrText = data.atr_14 ? parseFloat(data.atr_14).toFixed(2) : "--";
            volText = data.volume ? parseFloat(data.volume).toFixed(2) : "--";
            vwapText = data.vwap ? parseFloat(data.vwap).toFixed(2) : "--";

            // --- Direct Candlestick Plotting ---
            if (data.open && data.high && data.low && data.close) {
                candleSeries.update({
                    time: timeSec,
                    open: parseFloat(data.open),
                    high: parseFloat(data.high),
                    low: parseFloat(data.low),
                    close: parseFloat(data.close)
                });

                // Update Volume Bar (Colored Green/Red based on candle close direction)
                let volColor = parseFloat(data.close) >= parseFloat(data.open) ? '#26a69a' : '#ef5350';
                volumeSeries.update({
                    time: timeSec,
                    value: parseFloat(data.volume),
                    color: volColor
                });
            }

            // --- Update EMAs Overlays ---
            if (data.ema_fast) ema10Series.update({ time: timeSec, value: parseFloat(data.ema_fast) });
            if (data.ema_medium) ema50Series.update({ time: timeSec, value: parseFloat(data.ema_medium) });
            if (data.ema_slow) ema100Series.update({ time: timeSec, value: parseFloat(data.ema_slow) });
            if (data.ema_long) ema200Series.update({ time: timeSec, value: parseFloat(data.ema_long) });

            // --- Update Bollinger Bands ---
            if (data.bb_upper) bbUpperSeries.update({ time: timeSec, value: parseFloat(data.bb_upper) });
            if (data.bb_middle) bbMiddleSeries.update({ time: timeSec, value: parseFloat(data.bb_middle) });
            if (data.bb_lower) bbLowerSeries.update({ time: timeSec, value: parseFloat(data.bb_lower) });

            // --- Update VWAP ---
            if (data.vwap) vwapSeries.update({ time: timeSec, value: parseFloat(data.vwap) });

            // --- Update ADX ---
            if (data.adx_14) {
                adxSeries.update({ time: timeSec, value: parseFloat(data.adx_14) });
            }

            // --- Update Standalone ATR ---
            if (data.atr_14) {
                atrSeries.update({ time: timeSec, value: parseFloat(data.atr_14) });
            }

            // --- Update RSI ---
            if (data.rsi_14) {
                rsiSeries.update({ time: timeSec, value: parseFloat(data.rsi_14) });
            }

            // --- Update MACD ---
            if (data.macd_line) {
                const mLine = parseFloat(data.macd_line);
                const mSig = parseFloat(data.macd_signal);
                const mHist = parseFloat(data.macd_hist);

                macdLineText = mLine.toFixed(2);
                macdSigText = mSig.toFixed(2);
                macdHistText = mHist.toFixed(2);

                macdLineSeries.update({ time: timeSec, value: mLine });
                macdSigSeries.update({ time: timeSec, value: mSig });

                let histColor = mHist >= 0 
                    ? (mHist >= lastMacdHist ? "#26a69a" : "#b2dfdb")
                    : (mHist < lastMacdHist ? "#ef5350" : "#ffcdd2");

                macdHistSeries.update({ time: timeSec, value: mHist, color: histColor });
                lastMacdHist = mHist;
            }

            // --- Update Squeeze Momentum ---
            if (data.squeeze_momentum) {
                const momVal = parseFloat(data.squeeze_momentum);
                sqzValText = momVal.toFixed(4);

                let momColor = momVal >= 0
                    ? (momVal >= lastSqzMom ? "#4caf50" : "#086014")
                    : (momVal < lastSqzMom ? "#ff1744" : "#800b1d");

                squeezeMomSeries.update({ time: timeSec, value: momVal, color: momColor });
                lastSqzMom = momVal;

                isSqueezeOn = data.squeeze_on;
                sqzStatusText = isSqueezeOn ? 'SQUEEZE ON' : 'SQUEEZE OFF';
                
                let dotColor = isSqueezeOn ? "#ef5350" : "#4caf50";
                squeezeDotSeries.update({ time: timeSec, value: 0.1, color: dotColor });
            }
        };
    }
</script>

<div class="terminal-body">
    <!-- Header Panel (Top Bar) -->
    <header class="terminal-header">
        <div class="header-logo-group">
            <span class="text-xl">⚡</span>
            <h1 class="logo-title">{activeSymbol}USD</h1>
            
            <div class="time-badge">
                {candleTimeframeLabel}
            </div>
        </div>

        <!-- Dynamic Visibility Toggles -->
        <div class="visibility-selectors font-sans">
            <span class="selectors-label">Overlays:</span>
            <button class="selector-btn {showEmas ? 'active' : ''}" onclick={() => showEmas = !showEmas}>
                EMAs
            </button>
            <button class="selector-btn {showBb ? 'active' : ''}" onclick={() => showBb = !showBb}>
                BB
            </button>
            <button class="selector-btn {showVwap ? 'active' : ''}" onclick={() => showVwap = !showVwap}>
                VWAP
            </button>
            
            <span class="selectors-label ml-4">Panels:</span>
            <button class="selector-btn {showVolume ? 'active' : ''}" onclick={() => showVolume = !showVolume}>
                Volume
            </button>
            <button class="selector-btn {showAdx ? 'active' : ''}" onclick={() => showAdx = !showAdx}>
                ADX
            </button>
            <button class="selector-btn {showAtr ? 'active' : ''}" onclick={() => showAtr = !showAtr}>
                ATR
            </button>
            <button class="selector-btn {showRsi ? 'active' : ''}" onclick={() => showRsi = !showRsi}>
                RSI
            </button>
            <button class="selector-btn {showMacd ? 'active' : ''}" onclick={() => showMacd = !showMacd}>
                MACD
            </button>
            <button class="selector-btn {showSqueeze ? 'active' : ''}" onclick={() => showSqueeze = !showSqueeze}>
                Squeeze
            </button>
        </div>
        
        <div class="status-badge {isConnected ? 'status-online' : 'status-offline'}">
            <span class="status-pulse-dot {isConnected ? 'dot-online' : 'dot-offline'} animate-pulse"></span>
            <span>{isConnected ? 'LIVE STREAM ACTIVE' : 'OFFLINE'}</span>
        </div>
    </header>

    <!-- Main Chart Container Stack (Clean Vertical Gaps) -->
    <main class="dashboard-stack">

        <!-- Pane 1: Price and EMAs -->
        <div class="panel-box pane-price">
            <div class="absolute-label font-sans">
                <span class="price-header">Price: <span>{priceText}</span></span>
                {#if showVwap}
                    <span class="text-orange-400 font-medium">VWAP: <span>{vwapText}</span></span>
                {/if}
                {#if showEmas}
                    <span class="text-blue-400 font-medium">{emaFastLabel}: <span>{emaFastText}</span></span>
                    <span class="text-amber-500 font-medium">{emaMediumLabel}: <span>{emaMediumText}</span></span>
                    <span class="text-rose-500 font-medium">{emaSlowLabel}: <span>{emaSlowText}</span></span>
                    <span class="text-purple-400 font-medium">{emaLongLabel}: <span>{emaLongText}</span></span>
                {/if}
            </div>
            <div bind:this={priceContainer} class="chart-container"></div>
        </div>

        <!-- Pane 2: Volume Panel (Conditionally hidden) -->
        <div class="panel-box pane-vol" class:hidden-pane={!showVolume}>
            <div class="absolute-label font-sans label-text-xs">
                <span class="text-teal-400 font-bold">Volume: <span>{volText}</span></span>
            </div>
            <div bind:this={volumeContainer} class="chart-container"></div>
        </div>

        <!-- Pane 3: ADX Panel (Conditionally hidden) -->
        <div class="panel-box pane-adx" class:hidden-pane={!showAdx}>
            <div class="absolute-label font-sans label-text-xs">
                <span class="text-yellow-400">{adxLabel}: <span>{adxText}</span></span>
            </div>
            <div bind:this={adxContainer} class="chart-container"></div>
        </div>

        <!-- Pane 4: ATR Panel (Conditionally hidden) (New standalone panel positioned under ADX) -->
        <div class="panel-box pane-atr" class:hidden-pane={!showAtr}>
            <div class="absolute-label font-sans label-text-xs">
                <span class="text-purple-400 font-bold">{atrLabel}: <span>{atrText}</span></span>
            </div>
            <div bind:this={atrContainer} class="chart-container"></div>
        </div>

        <!-- Pane 5: RSI Panel (Conditionally hidden) -->
        <div class="panel-box pane-rsi" class:hidden-pane={!showRsi}>
            <div class="absolute-label font-sans label-text-xs">
                <span class="text-purple-400">{rsiLabel}: <span>{rsiText}</span></span>
            </div>
            <div bind:this={rsiContainer} class="chart-container"></div>
        </div>

        <!-- Pane 6: MACD Panel (Conditionally hidden) -->
        <div class="panel-box pane-macd" class:hidden-pane={!showMacd}>
            <div class="absolute-label font-sans label-text-xs">
                <span class="text-slate-300 font-bold">{macdLabel}</span>
                <span class="text-blue-400">Line: <span>{macdLineText}</span></span>
                <span class="text-amber-500">Signal: <span>{macdSigText}</span></span>
                <span class="text-teal-400">Hist: <span>{macdHistText}</span></span>
            </div>
            <div bind:this={macdContainer} class="chart-container"></div>
        </div>

        <!-- Pane 7: Squeeze Momentum Panel (Conditionally hidden) -->
        <div class="panel-box pane-squeeze" class:hidden-pane={!showSqueeze}>
            <div class="absolute-label font-sans label-text-xs">
                <span class="text-slate-300 font-bold">Squeeze Momentum (LazyBear)</span>
                <span class="text-emerald-400">Value: <span>{sqzValText}</span></span>
                <span class="{isSqueezeOn ? 'text-red-500' : 'text-emerald-500'} font-bold">Status: {sqzStatusText}</span>
            </div>
            <div bind:this={squeezeContainer} class="chart-container"></div>
        </div>
    </main>
</div>

<style>
    /* NATIVE SCOPED CSS STYLING */
    
    .terminal-body {
        background-color: #0b0e14;
        color: #f1f5f9;
        font-family: ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
        min-height: 100vh;
    }

    .terminal-header {
        border-bottom: 1px solid #1e293b;
        background-color: rgba(19, 23, 34, 0.9);
        padding: 12px 24px;
        display: flex;
        justify-content: space-between;
        align-items: center;
    }

    .header-logo-group {
        display: flex;
        align-items: center;
        gap: 16px;
    }

    .logo-title {
        font-size: 0.75rem;
        font-weight: 700;
        letter-spacing: 0.1em;
        color: #cbd5e1;
        text-transform: uppercase;
    }

    .time-badge {
        padding: 2px 8px;
        border-radius: 4px;
        font-size: 10px;
        font-weight: 900;
        background-color: rgba(59, 130, 246, 0.1);
        color: #60a5fa;
        border: 1px solid rgba(59, 130, 246, 0.2);
        text-transform: uppercase;
        letter-spacing: 0.1em;
    }

    /* Top Bar Selectors layout & design */
    .visibility-selectors {
        display: flex;
        align-items: center;
        gap: 8px;
    }

    .selectors-label {
        font-size: 9px;
        font-weight: 700;
        text-transform: uppercase;
        letter-spacing: 0.08em;
        color: #64748b;
        margin-right: 4px;
    }

    .selector-btn {
        background-color: #171b26;
        border: 1px solid #2a2e39;
        color: #8f929d;
        font-size: 9px;
        font-weight: 800;
        padding: 4px 10px;
        border-radius: 4px;
        cursor: pointer;
        transition: all 0.2s ease-in-out;
        text-transform: uppercase;
    }

    .selector-btn:hover {
        border-color: #4c526e;
        color: #cbd5e1;
    }

    /* Glowing active state matching standard TradingView themes */
    .selector-btn.active {
        background-color: rgba(59, 130, 246, 0.12);
        border-color: #3b82f6;
        color: #3b82f6;
        box-shadow: 0 0 8px rgba(59, 130, 246, 0.15);
    }

    /* Aligned stacked boxes with clear TradingView-style margins */
    .dashboard-stack {
        max-width: 1500px;
        margin: 0 auto;
        padding: 16px;
        display: flex;
        flex-direction: column;
        gap: 10px;
    }

    /* Distinct window container panels */
    .panel-box {
        position: relative;
        background-color: #131722;
        border: 1px solid #2a2e39;
        border-radius: 8px;
        box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -2px rgba(0, 0, 0, 0.1);
        overflow: hidden;
        transition: opacity 0.15s ease-in-out;
    }

    /* CSS Toggle Hider rule */
    .hidden-pane {
        display: none !important;
    }

    /* Strict Heights mapped to match reference layout */
    .pane-price { height: 320px; }
    .pane-vol { height: 110px; }
    .pane-adx { height: 110px; }
    .pane-atr { height: 110px; } /* Standalone ATR Height */
    .pane-rsi { height: 110px; }
    .pane-macd { height: 130px; }
    .pane-squeeze { height: 140px; }

    /* Force the canvas chart to take up full-bleed parent size */
    .chart-container {
        width: 100%;
        height: 100%;
    }

    /* Floating Labels Styling (overlays on left upper corners of the charts) */
    .absolute-label {
        position: absolute;
        top: 8px;
        left: 56px;
        z-index: 10;
        background-color: rgba(19, 23, 34, 0.9);
        border: 1px solid #2a2e39;
        border-radius: 4px;
        padding: 4px 8px;
        display: flex;
        gap: 16px;
    }

    .label-text-xs {
        font-size: 10px;
    }

    .price-header {
        font-weight: 700;
        color: #e2e8f0;
    }

    /* Connection status badge styling */
    .status-badge {
        padding: 4px 12px;
        border-radius: 4px;
        font-size: 0.75rem;
        font-weight: 600;
        border: 1px solid;
        display: flex;
        align-items: center;
        gap: 8px;
    }

    .status-online {
        background-color: rgba(16, 185, 129, 0.1);
        color: rgb(52, 211, 153);
        border-color: rgba(16, 185, 129, 0.2);
    }

    .status-offline {
        background-color: rgba(239, 68, 68, 0.1);
        color: rgb(248, 113, 113);
        border-color: rgba(239, 68, 68, 0.2);
    }

    .status-pulse-dot {
        height: 8px;
        width: 8px;
        border-radius: 9999px;
    }

    .dot-online { background-color: #10b981; }
    .dot-offline { background-color: #ef5350; }

    /* Indicator colors */
    .text-emerald-500 { color: #10b981; }
    .text-red-500 { color: #ef5350; }
    .text-teal-400 { color: #26a69a; }
    .text-yellow-400 { color: #f1c40f; }
    .text-purple-400 { color: #a78bfa; }
    .text-blue-400 { color: #60a5fa; }
    .text-amber-500 { color: #f59e0b; }
    .text-rose-500 { color: #f43f5e; }
    .text-slate-200 { color: #e2e8f0; }
    .text-slate-300 { color: #cbd5e1; }
    .text-orange-400 { color: #e67e22; } /* VWAP label color */
</style>
