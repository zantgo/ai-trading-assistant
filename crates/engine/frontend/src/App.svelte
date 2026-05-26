<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { getState } from './state.svelte';

    import Header from './components/Header.svelte';
    import PriceChart from './components/PriceChart.svelte';
    import VolumeChart from './components/VolumeChart.svelte';
    import AdxChart from './components/AdxChart.svelte';
    import AtrChart from './components/AtrChart.svelte';
    import RsiChart from './components/RsiChart.svelte';
    import MacdChart from './components/MacdChart.svelte';
    import SqueezeChart from './components/SqueezeChart.svelte';

    const state = getState();
    let ws: WebSocket | null = null;

    onMount(async () => {
        try {
            const res = await fetch('/api/config');
            const config = await res.json();

            state.barDurationSec = config.candles.duration_seconds;
            state.candleTimeframeLabel = `${config.candles.duration_seconds}s`;

            state.emaFastLabel = `EMA ${config.indicators.ema_fast}`;
            state.emaMediumLabel = `EMA ${config.indicators.ema_medium}`;
            state.emaSlowLabel = `EMA ${config.indicators.ema_slow}`;
            state.emaLongLabel = `EMA ${config.indicators.ema_long}`;
            state.rsiLabel = `RSI (${config.indicators.rsi_period})`;
            state.adxLabel = `ADX (${config.indicators.adx_period})`;
            state.atrLabel = `ATR (${config.indicators.atr_period})`;
            state.macdLabel = `MACD (${config.indicators.macd_fast}, ${config.indicators.macd_slow}, ${config.indicators.macd_signal})`;

            // Populate settings parameters inside global state
            state.emaFastVal = config.indicators.ema_fast;
            state.emaMediumVal = config.indicators.ema_medium;
            state.emaSlowVal = config.indicators.ema_slow;
            state.emaLongVal = config.indicators.ema_long;
            state.rsiPeriodVal = config.indicators.rsi_period;
            state.macdFastVal = config.indicators.macd_fast;
            state.macdSlowVal = config.indicators.macd_slow;
            state.macdSignalVal = config.indicators.macd_signal;
            state.adxPeriodVal = config.indicators.adx_period;
            state.atrPeriodVal = config.indicators.atr_period;
            state.squeezePeriodVal = config.indicators.squeeze_period;
        } catch (e) {
            console.error("⚠️ Failed to synchronize dynamic legends from config API, using defaults:", e);
        }

        connect();
    });

    onDestroy(() => {
        if (ws) ws.close();
    });

    function connect() {
        ws = new WebSocket(`ws://${window.location.host}/ws`);

        ws.onopen = () => {
            state.isConnected = true;
        };

        ws.onclose = () => {
            state.isConnected = false;
            setTimeout(connect, 3000);
        };

        ws.onmessage = (event) => {
            const data = JSON.parse(event.data);

            const closePrice = data.close ? parseFloat(data.close) : parseFloat(data.mid_price);

            if (data.symbol) state.activeSymbol = data.symbol;

            state.priceText = `$${closePrice.toFixed(2)}`;
            state.emaFastText = data.ema_fast ? parseFloat(data.ema_fast).toFixed(2) : '--';
            state.emaMediumText = data.ema_medium ? parseFloat(data.ema_medium).toFixed(2) : '--';
            state.emaSlowText = data.ema_slow ? parseFloat(data.ema_slow).toFixed(2) : '--';
            state.emaLongText = data.ema_long ? parseFloat(data.ema_long).toFixed(2) : '--';
            state.rsiText = data.rsi_14 ? parseFloat(data.rsi_14).toFixed(2) : '--';
            state.atrText = data.atr_14 ? parseFloat(data.atr_14).toFixed(2) : '--';
            state.volText = data.volume ? parseFloat(data.volume).toFixed(2) : '--';
            state.vwapText = data.vwap ? parseFloat(data.vwap).toFixed(2) : '--';

            if (data.adx_14) {
                state.adxText = parseFloat(data.adx_14).toFixed(2);
                state.adxPlusText = data.adx_plus ? parseFloat(data.adx_plus).toFixed(2) : '--';
                state.adxMinusText = data.adx_minus ? parseFloat(data.adx_minus).toFixed(2) : '--';
            }

            if (data.macd_line) {
                state.macdLineText = parseFloat(data.macd_line).toFixed(2);
                state.macdSigText = parseFloat(data.macd_signal).toFixed(2);
                state.macdHistText = parseFloat(data.macd_hist).toFixed(2);
            }

            if (data.squeeze_momentum) {
                state.sqzValText = parseFloat(data.squeeze_momentum).toFixed(4);
                state.isSqueezeOn = data.squeeze_on;
                state.sqzStatusText = data.squeeze_on ? 'SQUEEZE ON' : 'SQUEEZE OFF';
            }

            state.latestSnapshot = data;
        };
    }
</script>

<div class="terminal-body">
    <Header />

    <div class="main-layout">
        <!-- Center column showing active visual panels -->
        <main class="dashboard-stack">
            <div class="panel-box pane-price">
                <div class="absolute-label font-sans">
                    <span class="price-header">Price: <span>{state.priceText}</span></span>
                    {#if state.showVwap}
                        <span class="text-orange-400 font-medium">VWAP: <span>{state.vwapText}</span></span>
                    {/if}
                    {#if state.showEmas}
                        <span class="text-blue-400 font-medium">{state.emaFastLabel}: <span>{state.emaFastText}</span></span>
                        <span class="text-amber-500 font-medium">{state.emaMediumLabel}: <span>{state.emaMediumText}</span></span>
                        <span class="text-rose-500 font-medium">{state.emaSlowLabel}: <span>{state.emaSlowText}</span></span>
                        <span class="text-purple-400 font-medium">{state.emaLongLabel}: <span>{state.emaLongText}</span></span>
                    {/if}
                </div>
                <PriceChart />
            </div>

            <div class="panel-box pane-vol" class:hidden-pane={!state.showVolume}>
                <div class="absolute-label font-sans label-text-xs">
                    <span class="text-teal-400 font-bold">Volume: <span>{state.volText}</span></span>
                </div>
                <VolumeChart />
            </div>

            <div class="panel-box pane-adx" class:hidden-pane={!state.showAdx}>
                <div class="absolute-label font-sans label-text-xs">
                    <span class="text-yellow-400 font-bold">ADX: <span>{state.adxText}</span></span>
                    <span class="text-emerald-400 font-medium">+DI: <span>{state.adxPlusText}</span></span>
                    <span class="text-red-500 font-medium">-DI: <span>{state.adxMinusText}</span></span>
                </div>
                <AdxChart />
            </div>

            <div class="panel-box pane-atr" class:hidden-pane={!state.showAtr}>
                <div class="absolute-label font-sans label-text-xs">
                    <span class="text-purple-400 font-bold">{state.atrLabel}: <span>{state.atrText}</span></span>
                </div>
                <AtrChart />
            </div>

            <div class="panel-box pane-rsi" class:hidden-pane={!state.showRsi}>
                <div class="absolute-label font-sans label-text-xs">
                    <span class="text-purple-400">{state.rsiLabel}: <span>{state.rsiText}</span></span>
                </div>
                <RsiChart />
            </div>

            <div class="panel-box pane-macd" class:hidden-pane={!state.showMacd}>
                <div class="absolute-label font-sans label-text-xs">
                    <span class="text-slate-300 font-bold">{state.macdLabel}</span>
                    <span class="text-blue-400">Line: <span>{state.macdLineText}</span></span>
                    <span class="text-amber-500">Signal: <span>{state.macdSigText}</span></span>
                    <span class="text-teal-400">Hist: <span>{state.macdHistText}</span></span>
                </div>
                <MacdChart />
            </div>

            <div class="panel-box pane-squeeze" class:hidden-pane={!state.showSqueeze}>
                <div class="absolute-label font-sans label-text-xs">
                    <span class="text-slate-300 font-bold">Squeeze Momentum (LazyBear)</span>
                    <span class="text-emerald-400">Value: <span>{state.sqzValText}</span></span>
                    <span class={state.isSqueezeOn ? 'text-red-500 font-bold' : 'text-emerald-500 font-bold'}>Status: {state.sqzStatusText}</span>
                </div>
                <SqueezeChart />
            </div>
        </main>

        <!-- Right Side Panel containing settings variables & signal pipeline states -->
        <aside class="sidebar-panel font-sans">
            <div class="sidebar-section settings-box">
                <h3 class="section-title">SETTINGS</h3>
                <div class="settings-content">
                    <div class="setting-row">
                        <span class="setting-label">Pair:</span>
                        <span class="setting-value text-blue-400">{state.activeSymbol}USD</span>
                    </div>
                    <div class="setting-row">
                        <span class="setting-label">Timeframe:</span>
                        <span class="setting-value text-blue-400">{state.candleTimeframeLabel} ({state.barDurationSec}s)</span>
                    </div>
                    
                    <hr class="divider"/>
                    
                    <h4 class="sub-title">Indicators Parameter Limits</h4>
                    <div class="setting-row">
                        <span class="setting-label">EMA Fast Period:</span>
                        <span class="setting-value">{state.emaFastVal}</span>
                    </div>
                    <div class="setting-row">
                        <span class="setting-label">EMA Medium Period:</span>
                        <span class="setting-value">{state.emaMediumVal}</span>
                    </div>
                    <div class="setting-row">
                        <span class="setting-label">EMA Slow Period:</span>
                        <span class="setting-value">{state.emaSlowVal}</span>
                    </div>
                    <div class="setting-row">
                        <span class="setting-label">EMA Long Period:</span>
                        <span class="setting-value">{state.emaLongVal}</span>
                    </div>
                    <div class="setting-row">
                        <span class="setting-label">RSI Lookback Period:</span>
                        <span class="setting-value">{state.rsiPeriodVal}</span>
                    </div>
                    <div class="setting-row">
                        <span class="setting-label">MACD Fast Lookback:</span>
                        <span class="setting-value">{state.macdFastVal}</span>
                    </div>
                    <div class="setting-row">
                        <span class="setting-label">MACD Slow Lookback:</span>
                        <span class="setting-value">{state.macdSlowVal}</span>
                    </div>
                    <div class="setting-row">
                        <span class="setting-label">MACD Signal Line:</span>
                        <span class="setting-value">{state.macdSignalVal}</span>
                    </div>
                    <div class="setting-row">
                        <span class="setting-label">ADX Calculation Period:</span>
                        <span class="setting-value">{state.adxPeriodVal}</span>
                    </div>
                    <div class="setting-row">
                        <span class="setting-label">ATR Volatility Window:</span>
                        <span class="setting-value">{state.atrPeriodVal}</span>
                    </div>
                    <div class="setting-row">
                        <span class="setting-label">Squeeze Wave Period:</span>
                        <span class="setting-value">{state.squeezePeriodVal}</span>
                    </div>
                </div>
            </div>

            <div class="sidebar-section signals-box">
                <h3 class="section-title">SIGNALS</h3>
                <div class="signals-content">
                    <p class="signals-placeholder">
                        (Here in the future we will put the result of a decision based on indicators that will tell us bearish/bullish)
                    </p>
                    <div class="signal-indicator">
                        <span class="dot pulse-blue"></span>
                        <span class="status-text">Awaiting Agent Decision...</span>
                    </div>
                </div>
            </div>
        </aside>
    </div>
</div>

<style>
    .terminal-body {
        background-color: #0b0e14;
        color: #f1f5f9;
        font-family: ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
        min-height: 100vh;
    }
    .main-layout {
        display: flex;
        max-width: 1800px;
        margin: 0 auto;
        padding: 16px;
        gap: 16px;
    }
    .dashboard-stack {
        flex: 1;
        display: flex;
        flex-direction: column;
        gap: 10px;
    }
    .sidebar-panel {
        width: 320px;
        display: flex;
        flex-direction: column;
        gap: 16px;
    }
    .sidebar-section {
        background-color: #131722;
        border: 1px solid #2a2e39;
        border-radius: 8px;
        padding: 16px;
        box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -2px rgba(0, 0, 0, 0.1);
        display: flex;
        flex-direction: column;
    }
    .settings-box {
        flex: 0 0 auto;
    }
    .signals-box {
        flex: 1 1 auto;
    }
    .section-title {
        font-size: 11px;
        font-weight: 700;
        letter-spacing: 0.1em;
        color: #cbd5e1;
        margin-top: 0;
        margin-bottom: 12px;
        border-bottom: 1px solid #1e293b;
        padding-bottom: 6px;
        text-transform: uppercase;
    }
    .sub-title {
        font-size: 10px;
        font-weight: 600;
        letter-spacing: 0.05em;
        color: #64748b;
        margin-top: 10px;
        margin-bottom: 6px;
        text-transform: uppercase;
    }
    .settings-content, .signals-content {
        font-size: 11px;
        color: #94a3b8;
        display: flex;
        flex-direction: column;
        gap: 2px;
    }
    .setting-row {
        display: flex;
        justify-content: space-between;
        padding: 3px 0;
    }
    .setting-label {
        color: #64748b;
    }
    .setting-value {
        font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
        color: #3b82f6;
        font-weight: 600;
    }
    .divider {
        border: 0;
        border-top: 1px solid #1e293b;
        margin: 10px 0;
    }
    .signals-placeholder {
        font-style: italic;
        color: #4c525e;
        line-height: 1.4;
        margin-bottom: 12px;
    }
    .signal-indicator {
        display: flex;
        align-items: center;
        gap: 8px;
        background-color: rgba(59, 130, 246, 0.05);
        border: 1px solid rgba(59, 130, 246, 0.15);
        padding: 8px 12px;
        border-radius: 6px;
        margin-top: auto;
    }
    .pulse-blue {
        background-color: #3b82f6;
        box-shadow: 0 0 0 0 rgba(59, 130, 246, 0.7);
        animation: pulse 2s infinite;
    }
    .dot {
        height: 6px;
        width: 6px;
        border-radius: 50%;
    }
    .status-text {
        font-weight: 600;
        color: #3b82f6;
        font-size: 10px;
        text-transform: uppercase;
        letter-spacing: 0.05em;
    }

    @keyframes pulse {
        0% {
            transform: scale(0.95);
            box-shadow: 0 0 0 0 rgba(59, 130, 246, 0.7);
        }
        70% {
            transform: scale(1);
            box-shadow: 0 0 0 4px rgba(59, 130, 246, 0);
        }
        100% {
            transform: scale(0.95);
            box-shadow: 0 0 0 0 rgba(59, 130, 246, 0);
        }
    }

    /* Fallback layout adjustment for compact screen sizes */
    @media (max-width: 1024px) {
        .main-layout {
            flex-direction: column;
        }
        .sidebar-panel {
            width: 100%;
        }
    }

    .panel-box {
        position: relative;
        background-color: #131722;
        border: 1px solid #2a2e39;
        border-radius: 8px;
        box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -2px rgba(0, 0, 0, 0.1);
        overflow: hidden;
        transition: opacity 0.15s ease-in-out;
        resize: vertical;
        min-height: 80px;
        max-height: 800px;
    }
    .hidden-pane { display: none !important; }
    .pane-price { height: 320px; }
    .pane-vol { height: 110px; }
    .pane-adx { height: 110px; }
    .pane-atr { height: 110px; }
    .pane-rsi { height: 110px; }
    .pane-macd { height: 130px; }
    .pane-squeeze { height: 140px; }

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
    .label-text-xs { font-size: 10px; }
    .price-header { font-weight: 700; color: #e2e8f0; }

    /* Indicator colors */
    .text-emerald-500 { color: #10b981; }
    .text-red-500 { color: #ef5350; }
    .text-teal-400 { color: #26a69a; }
    .text-yellow-400 { color: #f1c40f; }
    .text-purple-400 { color: #a78bfa; }
    .text-blue-400 { color: #60a5fa; }
    .text-amber-500 { color: #f59e0b; }
    .text-rose-500 { color: #f43f5e; }
    .text-slate-300 { color: #cbd5e1; }
    .text-orange-400 { color: #f1c40f; }
</style>
