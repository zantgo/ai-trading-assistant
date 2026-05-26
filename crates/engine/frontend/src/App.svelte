<script lang="ts">
    //! # Svelte 5 Trading Terminal Dashboard
    //! 
    //! Coordinates system components: handles WebSocket streams, aggregates price
    //! ticks into 5-second candles, and synchronizes 5 TradingView Lightweight Charts.
    //! Formatted with Svelte 5 $state() runes and Lightweight Charts 5.0 generic .addSeries().

    import { onMount, onDestroy } from 'svelte';
    import { 
        createChart, 
        CrosshairMode, 
        CandlestickSeries, 
        LineSeries, 
        HistogramSeries 
    } from 'lightweight-charts';
    import type { IChartApi, ISeriesApi, CandlestickData } from 'lightweight-charts';

    // Time-aggregation constant (5s candles)
    const BAR_DURATION_SEC = 5;

    // Direct DOM bindings
    let priceContainer: HTMLDivElement;
    let adxContainer: HTMLDivElement;
    let rsiContainer: HTMLDivElement;
    let macdContainer: HTMLDivElement;
    let squeezeContainer: HTMLDivElement;

    // Squeeze 5 Runes state declarations (forces modern UI reactivity)
    let isConnected = $state(false);
    let priceText = $state('--');
    let ema10Text = $state('--');
    let ema50Text = $state('--');
    let ema100Text = $state('--');
    let ema200Text = $state('--');
    let adxText = $state('--');
    let rsiText = $state('--');
    let macdLineText = $state('--');
    let macdSigText = $state('--');
    let macdHistText = $state('--');
    let sqzValText = $state('--');
    let sqzStatusText = $state('Calculating');
    let isSqueezeOn = $state(false);

    // Chart and Series instances
    let charts: IChartApi[] = [];
    let ws: WebSocket | null = null;
    let lastCandle: CandlestickData<number> | null = null;
    let lastMacdHist = 0;
    let lastSqzMom = 0;

    // Series APIs
    let candleSeries: ISeriesApi<'Candlestick'>;
    let ema10Series: ISeriesApi<'Line'>;
    let ema50Series: ISeriesApi<'Line'>;
    let ema100Series: ISeriesApi<'Line'>;
    let ema200Series: ISeriesApi<'Line'>;
    let adxSeries: ISeriesApi<'Line'>;
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

    onMount(() => {
        const priceChart = createChart(priceContainer, chartBaseOptions);
        const adxChart = createChart(adxContainer, chartBaseOptions);
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

        charts = [priceChart, adxChart, rsiChart, macdChart, squeezeChart];

        // Version 5.x compatible series initialization using .addSeries()
        candleSeries = priceChart.addSeries(CandlestickSeries, {
            upColor: '#26a69a', downColor: '#ef5350', borderVisible: false,
            wickUpColor: '#26a69a', wickDownColor: '#ef5350'
        });
        ema10Series = priceChart.addSeries(LineSeries, { color: '#2962ff', lineWidth: 1.5, priceLineVisible: false, crosshairMarkerVisible: false });
        ema50Series = priceChart.addSeries(LineSeries, { color: '#ff9800', lineWidth: 1.5, priceLineVisible: false, crosshairMarkerVisible: false });
        ema100Series = priceChart.addSeries(LineSeries, { color: '#e91e63', lineWidth: 1.5, priceLineVisible: false, crosshairMarkerVisible: false });
        ema200Series = priceChart.addSeries(LineSeries, { color: '#9c27b0', lineWidth: 1.5, priceLineVisible: false, crosshairMarkerVisible: false });

        adxSeries = adxChart.addSeries(LineSeries, { color: '#f1c40f', lineWidth: 1.5, priceLineVisible: false });
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
            const price = parseFloat(data.mid_price);

            priceText = `$${price.toFixed(2)}`;
            ema10Text = data.ema_10 ? parseFloat(data.ema_10).toFixed(2) : "--";
            ema50Text = data.ema_50 ? parseFloat(data.ema_50).toFixed(2) : "--";
            ema100Text = data.ema_100 ? parseFloat(data.ema_100).toFixed(2) : "--";
            ema200Text = data.ema_200 ? parseFloat(data.ema_200).toFixed(2) : "--";
            rsiText = data.rsi_14 ? parseFloat(data.rsi_14).toFixed(2) : "--";
            adxText = data.adx_14 ? parseFloat(data.adx_14).toFixed(2) : "--";

            const roundedTimeSec = Math.floor(timeSec / BAR_DURATION_SEC) * BAR_DURATION_SEC;

            if (lastCandle && lastCandle.time === roundedTimeSec) {
                lastCandle.high = Math.max(lastCandle.high, price);
                lastCandle.low = Math.min(lastCandle.low, price);
                lastCandle.close = price;
                candleSeries.update(lastCandle);
            } else {
                lastCandle = {
                    time: roundedTimeSec,
                    open: price, high: price, low: price, close: price
                };
                candleSeries.update(lastCandle);
            }

            if (data.ema_10) ema10Series.update({ time: timeSec, value: parseFloat(data.ema_10) });
            if (data.ema_50) ema50Series.update({ time: timeSec, value: parseFloat(data.ema_50) });
            if (data.ema_100) ema100Series.update({ time: timeSec, value: parseFloat(data.ema_100) });
            if (data.ema_200) ema200Series.update({ time: timeSec, value: parseFloat(data.ema_200) });

            if (data.adx_14) adxSeries.update({ time: timeSec, value: parseFloat(data.adx_14) });
            if (data.rsi_14) rsiSeries.update({ time: timeSec, value: parseFloat(data.rsi_14) });

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
            <h1 class="logo-title">ETHUSD Live Execution Interface</h1>
            
            <div class="time-badge">
                5s
            </div>
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
                <span class="text-blue-400 font-medium">EMA10: <span>{ema10Text}</span></span>
                <span class="text-amber-500 font-medium">EMA50: <span>{ema50Text}</span></span>
                <span class="text-rose-500 font-medium">EMA100: <span>{ema100Text}</span></span>
                <span class="text-purple-400 font-medium">EMA200: <span>{ema200Text}</span></span>
            </div>
            <div bind:this={priceContainer} class="chart-container"></div>
        </div>

        <!-- Pane 2: ADX Panel -->
        <div class="panel-box pane-adx">
            <div class="absolute-label font-sans label-text-xs">
                <span class="text-yellow-400">ADX (14): <span>{adxText}</span></span>
            </div>
            <div bind:this={adxContainer} class="chart-container"></div>
        </div>

        <!-- Pane 3: RSI Panel -->
        <div class="panel-box pane-rsi">
            <div class="absolute-label font-sans label-text-xs">
                <span class="text-purple-400">RSI (14): <span>{rsiText}</span></span>
            </div>
            <div bind:this={rsiContainer} class="chart-container"></div>
        </div>

        <!-- Pane 4: MACD Panel -->
        <div class="panel-box pane-macd">
            <div class="absolute-label font-sans label-text-xs">
                <span class="text-slate-300 font-bold">MACD (12, 26, 9)</span>
                <span class="text-blue-400">Line: <span>{macdLineText}</span></span>
                <span class="text-amber-500">Signal: <span>{macdSigText}</span></span>
                <span class="text-teal-400">Hist: <span>{macdHistText}</span></span>
            </div>
            <div bind:this={macdContainer} class="chart-container"></div>
        </div>

        <!-- Pane 5: Squeeze Momentum Panel -->
        <div class="panel-box pane-squeeze">
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
    }

    /* Strict Heights mapped to match reference layout */
    .pane-price { height: 320px; }
    .pane-adx { height: 110px; }
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
</style>
