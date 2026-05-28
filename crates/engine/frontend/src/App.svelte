<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { getState } from './state.svelte';
    import type { AssistantAnalysis } from './state.svelte';

    import Header from './components/Header.svelte';
    import PriceChart from './components/PriceChart.svelte';
    import VolumeChart from './components/VolumeChart.svelte';
    import AdxChart from './components/AdxChart.svelte';
    import AtrChart from './components/AtrChart.svelte';
    import RsiChart from './components/RsiChart.svelte';
    import MacdChart from './components/MacdChart.svelte';
    import SqueezeChart from './components/SqueezeChart.svelte';

    const app = getState();
    let ws: WebSocket | null = null;
    let analysisProgressStep = $state(0);
    let progressTimer: ReturnType<typeof setInterval> | null = null;

    // Descargar parámetros de configuración desde el backend
    async function fetchConfig() {
        try {
            const res = await fetch('/api/config');
            if (!res.ok) return;
            const config = await res.json();
            
            app.barDurationSec = config.candles.duration_seconds;
            app.emaFastVal = config.indicators.ema_fast;
            app.emaMediumVal = config.indicators.ema_medium;
            app.emaSlowVal = config.indicators.ema_slow;
            app.emaLongVal = config.indicators.ema_long;
            app.rsiPeriodVal = config.indicators.rsi_period;
            app.macdFastVal = config.indicators.macd_fast;
            app.macdSlowVal = config.indicators.macd_slow;
            app.macdSignalVal = config.indicators.macd_signal;
            app.adxPeriodVal = config.indicators.adx_period;
            app.atrPeriodVal = config.indicators.atr_period;
            app.squeezePeriodVal = config.indicators.squeeze_period;
        } catch (_) {
            // Silencioso en caso de error de red inicial
        }
    }

    async function fetchAssistantHistory() {
        try {
            const res = await fetch('/api/assistant-records');
            const data = await res.json();
            app.assistantHistory = data.records || [];
            app.historyLatestClose = data.latest_close || '0';
        } catch (_) {
            // Silencioso
        }
    }

    // Establecer conexión con el canal de telemetría en tiempo real
    function connectWebsocket() {
        const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        const wsUrl = `${protocol}//${window.location.host}/ws`;
        
        ws = new WebSocket(wsUrl);

        ws.onopen = () => {
            app.isConnected = true;
        };

        ws.onmessage = (event) => {
            try {
                const snapshot = JSON.parse(event.data);
                app.latestSnapshot = snapshot;

                // Actualizar textos reactivos en el estado global
                if (snapshot.mid_price) app.priceText = parseFloat(snapshot.mid_price).toFixed(2);
                if (snapshot.vwap) app.vwapText = parseFloat(snapshot.vwap).toFixed(2);
                if (snapshot.ema_fast) app.emaFastText = parseFloat(snapshot.ema_fast).toFixed(2);
                if (snapshot.ema_medium) app.emaMediumText = parseFloat(snapshot.ema_medium).toFixed(2);
                if (snapshot.ema_slow) app.emaSlowText = parseFloat(snapshot.ema_slow).toFixed(2);
                if (snapshot.ema_long) app.emaLongText = parseFloat(snapshot.ema_long).toFixed(2);
                
                if (snapshot.adx_14) app.adxText = parseFloat(snapshot.adx_14).toFixed(2);
                if (snapshot.adx_plus) app.adxPlusText = parseFloat(snapshot.adx_plus).toFixed(2);
                if (snapshot.adx_minus) app.adxMinusText = parseFloat(snapshot.adx_minus).toFixed(2);
                
                if (snapshot.atr_14) app.atrText = parseFloat(snapshot.atr_14).toFixed(2);
                if (snapshot.rsi_14) app.rsiText = parseFloat(snapshot.rsi_14).toFixed(2);
                
                if (snapshot.macd_line) app.macdLineText = parseFloat(snapshot.macd_line).toFixed(4);
                if (snapshot.macd_signal) app.macdSigText = parseFloat(snapshot.macd_signal).toFixed(4);
                if (snapshot.macd_hist) app.macdHistText = parseFloat(snapshot.macd_hist).toFixed(4);
                
                if (snapshot.squeeze_momentum) app.sqzValText = parseFloat(snapshot.squeeze_momentum).toFixed(4);
                app.isSqueezeOn = snapshot.squeeze_on ?? false;
                app.sqzStatusText = app.isSqueezeOn ? 'SQUEEZE ON' : 'SQUEEZE OFF';
                
                if (snapshot.volume) app.volText = parseFloat(snapshot.volume).toFixed(2);
            } catch (err) {
                console.error("Error parsing market snapshot JSON:", err);
            }
        };

        ws.onclose = () => {
            app.isConnected = false;
            // Intentar reconectar automáticamente cada 3 segundos
            setTimeout(connectWebsocket, 3000);
        };

        ws.onerror = () => {
            app.isConnected = false;
            ws?.close();
        };
    }

    onMount(() => {
        fetchConfig();
        fetchAssistantHistory();
        connectWebsocket();
    });

    onDestroy(() => {
        if (ws) {
            ws.close();
        }
        if (progressTimer) {
            clearInterval(progressTimer);
        }
    });

    async function requestAnalysis() {
        app.assistantLoading = true;
        app.assistantError = null;
        app.assistantResponse = null;
        analysisProgressStep = 0;

        progressTimer = setInterval(() => {
            if (analysisProgressStep < 2) {
                analysisProgressStep++;
            }
        }, 900);

        try {
            const historyRes = await fetch('/api/history');
            const historyData = await historyRes.json();
            const prices: number[] = (historyData.prices || []).map(Number);

            const snap = app.latestSnapshot || {};

            const body = {
                position: app.currentPosition,
                historical_prices: prices,
                indicators: {
                    rsi: snap.rsi_14 ? parseFloat(String(snap.rsi_14)) : null,
                    squeeze_on: snap.squeeze_on ?? null,
                    macd_histogram: snap.macd_hist ? parseFloat(String(snap.macd_hist)) : null,
                    adx: snap.adx_14 ? parseFloat(String(snap.adx_14)) : null,
                    ema_fast: snap.ema_fast ? parseFloat(String(snap.ema_fast)) : null,
                    ema_slow: snap.ema_slow ? parseFloat(String(snap.ema_slow)) : null,
                },
            };

            const res = await fetch('/api/analyze', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(body),
            });

            if (!res.ok) {
                throw new Error(`Server returned ${res.status}`);
            }

            const analysis: AssistantAnalysis = await res.json();
            analysisProgressStep = 2;
            app.assistantResponse = analysis;
            fetchAssistantHistory();
        } catch (e: any) {
            app.assistantError = e.message || 'Unknown error during analysis';
        } finally {
            if (progressTimer) clearInterval(progressTimer);
            progressTimer = null;
            app.assistantLoading = false;
        }
    }
</script>

<div class="terminal-body">
    <Header />

    <div class="main-layout">
        <!-- Center column showing active visual panels -->
        <main class="dashboard-stack">
            <div class="panel-box pane-price">
                <div class="absolute-label font-sans">
                    <span class="price-header">Price: <span>{app.priceText}</span></span>
                    {#if app.showVwap}
                        <span class="text-orange-400 font-medium">VWAP: <span>{app.vwapText}</span></span>
                    {/if}
                    {#if app.showEmas}
                        <span class="text-blue-400 font-medium">{app.emaFastLabel}: <span>{app.emaFastText}</span></span>
                        <span class="text-amber-500 font-medium">{app.emaMediumLabel}: <span>{app.emaMediumText}</span></span>
                        <span class="text-rose-500 font-medium">{app.emaSlowLabel}: <span>{app.emaSlowText}</span></span>
                        <span class="text-purple-400 font-medium">{app.emaLongLabel}: <span>{app.emaLongText}</span></span>
                    {/if}
                </div>
                <PriceChart />
            </div>

            <div class="panel-box pane-vol" class:hidden-pane={!app.showVolume}>
                <div class="absolute-label font-sans label-text-xs">
                    <span class="text-teal-400 font-bold">Volume: <span>{app.volText}</span></span>
                </div>
                <VolumeChart />
            </div>

            <div class="panel-box pane-adx" class:hidden-pane={!app.showAdx}>
                <div class="absolute-label font-sans label-text-xs">
                    <span class="text-yellow-400 font-bold">ADX: <span>{app.adxText}</span></span>
                    <span class="text-emerald-400 font-medium">+DI: <span>{app.adxPlusText}</span></span>
                    <span class="text-red-500 font-medium">-DI: <span>{app.adxMinusText}</span></span>
                </div>
                <AdxChart />
            </div>

            <div class="panel-box pane-atr" class:hidden-pane={!app.showAtr}>
                <div class="absolute-label font-sans label-text-xs">
                    <span class="text-purple-400 font-bold">{app.atrLabel}: <span>{app.atrText}</span></span>
                </div>
                <AtrChart />
            </div>

            <div class="panel-box pane-rsi" class:hidden-pane={!app.showRsi}>
                <div class="absolute-label font-sans label-text-xs">
                    <span class="text-purple-400">{app.rsiLabel}: <span>{app.rsiText}</span></span>
                </div>
                <RsiChart />
            </div>

            <div class="panel-box pane-macd" class:hidden-pane={!app.showMacd}>
                <div class="absolute-label font-sans label-text-xs">
                    <span class="text-slate-300 font-bold">{app.macdLabel}</span>
                    <span class="text-blue-400">Line: <span>{app.macdLineText}</span></span>
                    <span class="text-amber-500">Signal: <span>{app.macdSigText}</span></span>
                    <span class="text-teal-400">Hist: <span>{app.macdHistText}</span></span>
                </div>
                <MacdChart />
            </div>

            <div class="panel-box pane-squeeze" class:hidden-pane={!app.showSqueeze}>
                <div class="absolute-label font-sans label-text-xs">
                    <span class="text-slate-300 font-bold">Squeeze Momentum (LazyBear)</span>
                    <span class="text-emerald-400">Value: <span>{app.sqzValText}</span></span>
                    <span class={app.isSqueezeOn ? 'text-red-500 font-bold' : 'text-emerald-500 font-bold'}>Status: {app.sqzStatusText}</span>
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
                        <span class="setting-value text-blue-400">{app.activeSymbol}USD</span>
                    </div>
                    <div class="setting-row">
                        <span class="setting-label">Timeframe:</span>
                        <span class="setting-value text-blue-400">{app.candleTimeframeLabel} ({app.barDurationSec}s)</span>
                    </div>
                    
                    <hr class="divider"/>
                    
                    <h4 class="sub-title">Indicators Parameter Limits</h4>
                    <div class="setting-row">
                        <span class="setting-label">EMA Fast Period:</span>
                        <span class="setting-value">{app.emaFastVal}</span>
                    </div>
                    <div class="setting-row">
                        <span class="setting-label">EMA Medium Period:</span>
                        <span class="setting-value">{app.emaMediumVal}</span>
                    </div>
                    <div class="setting-row">
                        <span class="setting-label">EMA Slow Period:</span>
                        <span class="setting-value">{app.emaSlowVal}</span>
                    </div>
                    <div class="setting-row">
                        <span class="setting-label">EMA Long Period:</span>
                        <span class="setting-value">{app.emaLongVal}</span>
                    </div>
                    <div class="setting-row">
                        <span class="setting-label">RSI Lookback Period:</span>
                        <span class="setting-value">{app.rsiPeriodVal}</span>
                    </div>
                    <div class="setting-row">
                        <span class="setting-label">MACD Fast Lookback:</span>
                        <span class="setting-value">{app.macdFastVal}</span>
                    </div>
                    <div class="setting-row">
                        <span class="setting-label">MACD Slow Lookback:</span>
                        <span class="setting-value">{app.macdSlowVal}</span>
                    </div>
                    <div class="setting-row">
                        <span class="setting-label">MACD Signal Line:</span>
                        <span class="setting-value">{app.macdSignalVal}</span>
                    </div>
                    <div class="setting-row">
                        <span class="setting-label">ADX Calculation Period:</span>
                        <span class="setting-value">{app.adxPeriodVal}</span>
                    </div>
                    <div class="setting-row">
                        <span class="setting-label">ATR Volatility Window:</span>
                        <span class="setting-value">{app.atrPeriodVal}</span>
                    </div>
                    <div class="setting-row">
                        <span class="setting-label">Squeeze Wave Period:</span>
                        <span class="setting-value">{app.squeezePeriodVal}</span>
                    </div>
                </div>
            </div>

            <div class="sidebar-section signals-box">
                <h3 class="section-title">AI ASSISTANT</h3>
                <div class="signals-content">
                    <div class="position-selector">
                        <span class="sub-title">Current Position:</span>
                        <label>
                            <input type="radio" bind:group={app.currentPosition} value="None" /> None
                        </label>
                        <label>
                            <input type="radio" bind:group={app.currentPosition} value="Long" /> Long
                        </label>
                        <label>
                            <input type="radio" bind:group={app.currentPosition} value="Short" /> Short
                        </label>
                    </div>

                    <button
                        class="analyze-btn"
                        onclick={requestAnalysis}
                        disabled={app.assistantLoading}
                    >
                        {#if app.assistantLoading}
                            Analyzing Market...
                        {:else}
                            Request AI Assistant Analysis
                        {/if}
                    </button>

                    {#if app.assistantLoading}
                        <div class="loading-indicator">
                            <span class="dot pulse-blue"></span>
                            <span class="status-text">Running sequential analysis...</span>
                        </div>
                        <div class="analysis-progress">
                            <span class="step" class:active={analysisProgressStep >= 0}>Trend Check</span>
                            <span class="step-arrow" class:active-arrow={analysisProgressStep >= 1}>→</span>
                            <span class="step" class:active={analysisProgressStep >= 1}>Indicators</span>
                            <span class="step-arrow" class:active-arrow={analysisProgressStep >= 2}>→</span>
                            <span class="step" class:active={analysisProgressStep >= 2}>Recommendation</span>
                        </div>
                    {/if}

                    {#if app.assistantError}
                        <div class="error-box">
                            <span>Failed: {app.assistantError}</span>
                        </div>
                    {/if}

                    {#if app.assistantResponse && !app.assistantLoading}
                        {@const resp = app.assistantResponse}
                        <div class="analysis-result">
                            <div class="result-block reveal" style="animation-delay: 0ms">
                                <h4 class="result-stage-title">1. Price Action Trend</h4>
                                <span class="result-badge"
                                    class:badge-up={resp.trend_analysis.classification === 'trending upwards'}
                                    class:badge-down={resp.trend_analysis.classification === 'trending downwards'}
                                    class:badge-side={resp.trend_analysis.classification === 'sideways'}
                                >
                                    {resp.trend_analysis.classification}
                                </span>
                                <p class="result-reasoning">{resp.trend_analysis.structural_reasoning}</p>
                            </div>

                            <div class="result-block reveal" style="animation-delay: 200ms">
                                <h4 class="result-stage-title">2. Indicator Alignment</h4>
                                <span class="result-badge"
                                    class:badge-supportive={resp.indicator_alignment.classification === 'supportive'}
                                    class:badge-conflicting={resp.indicator_alignment.classification === 'conflicting'}
                                    class:badge-neutral={resp.indicator_alignment.classification === 'neutral'}
                                >
                                    {resp.indicator_alignment.classification}
                                </span>
                                <p class="result-reasoning">{resp.indicator_alignment.observation}</p>
                            </div>

                            <div class="result-block result-action reveal" style="animation-delay: 400ms">
                                <h4 class="result-stage-title">3. Position Recommendation</h4>
                                <span
                                    class="action-call"
                                    class:action-green={resp.position_recommendation.action === 'Hold' || resp.position_recommendation.action === 'Open Long'}
                                    class:action-red={resp.position_recommendation.action === 'Close'}
                                    class:action-amber={resp.position_recommendation.action === 'Wait' || resp.position_recommendation.action === 'Open Short'}
                                >
                                    {resp.position_recommendation.action}
                                </span>
                                <p class="result-reasoning">{resp.position_recommendation.rationale}</p>
                            </div>
                        </div>
                    {:else if !app.assistantLoading && !app.assistantError}
                        <p class="signals-placeholder">
                            Select your current position and request an AI market analysis.
                        </p>
                    {/if}
                </div>
            </div>

            <div class="sidebar-section history-box">
                <h3 class="section-title">ANALYSIS HISTORY</h3>
                <div class="history-content">
                    {#if app.assistantHistory.length === 0}
                        <p class="signals-placeholder">No analysis history recorded yet.</p>
                    {:else}
                        <div class="history-table-wrap">
                            <table class="history-table">
                                <thead>
                                    <tr>
                                        <th>Time</th>
                                        <th>Pos</th>
                                        <th>Action</th>
                                        <th>Entry $</th>
                                        <th>Δ%</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {#each app.assistantHistory as rec}
                                        {@const recPrice = parseFloat(rec.close_price) || 0}
                                        {@const latestPrice = parseFloat(app.historyLatestClose) || 0}
                                        {@const delta = recPrice > 0 ? ((latestPrice - recPrice) / recPrice * 100) : 0}
                                        <tr>
                                            <td class="col-time">{rec.created_at.substring(11, 19)}</td>
                                            <td>{rec.position}</td>
                                            <td class="col-action"
                                                class:action-text-green={rec.recommended_action === 'Hold' || rec.recommended_action === 'Open Long'}
                                                class:action-text-red={rec.recommended_action === 'Close'}
                                                class:action-text-amber={rec.recommended_action === 'Wait' || rec.recommended_action === 'Open Short'}
                                            >
                                                {rec.recommended_action.substring(0, 4)}
                                            </td>
                                            <td class="col-price">{rec.close_price.substring(0, 8)}</td>
                                            <td class="col-delta"
                                                class:delta-positive={delta > 0}
                                                class:delta-negative={delta < 0}
                                            >
                                                {delta.toFixed(2)}%
                                            </td>
                                        </tr>
                                    {/each}
                                </tbody>
                            </table>
                        </div>
                    {/if}
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

    /* AI Assistant styles */
    .position-selector {
        display: flex;
        align-items: center;
        gap: 8px;
        flex-wrap: wrap;
        margin-bottom: 10px;
    }
    .position-selector label {
        display: flex;
        align-items: center;
        gap: 3px;
        font-size: 10px;
        color: #94a3b8;
        cursor: pointer;
    }
    .position-selector input[type="radio"] {
        accent-color: #3b82f6;
        cursor: pointer;
    }

    .analyze-btn {
        width: 100%;
        padding: 8px 12px;
        background: linear-gradient(135deg, #1e40af, #3b82f6);
        color: #f1f5f9;
        border: 1px solid #3b82f6;
        border-radius: 6px;
        font-size: 11px;
        font-weight: 700;
        text-transform: uppercase;
        letter-spacing: 0.05em;
        cursor: pointer;
        transition: opacity 0.2s, background 0.2s;
        margin-bottom: 10px;
    }
    .analyze-btn:hover:not(:disabled) {
        background: linear-gradient(135deg, #1e3a8a, #2563eb);
    }
    .analyze-btn:disabled {
        opacity: 0.5;
        cursor: not-allowed;
    }

    .loading-indicator {
        display: flex;
        align-items: center;
        gap: 8px;
        background-color: rgba(59, 130, 246, 0.05);
        border: 1px solid rgba(59, 130, 246, 0.15);
        padding: 8px 12px;
        border-radius: 6px;
        margin-bottom: 8px;
    }

    .analysis-progress {
        display: flex;
        align-items: center;
        justify-content: center;
        gap: 6px;
        margin-bottom: 10px;
    }
    .step {
        font-size: 9px;
        font-weight: 600;
        color: #4c525e;
        text-transform: uppercase;
        letter-spacing: 0.05em;
    }
    .step.active {
        color: #3b82f6;
    }
    .step-arrow {
        color: #4c525e;
        font-size: 9px;
        transition: color 0.3s;
    }
    .step-arrow.active-arrow {
        color: #3b82f6;
    }

    .error-box {
        background-color: rgba(239, 68, 68, 0.08);
        border: 1px solid rgba(239, 68, 68, 0.3);
        border-radius: 6px;
        padding: 8px 12px;
        font-size: 10px;
        color: #ef4444;
    }

    .analysis-result {
        display: flex;
        flex-direction: column;
        gap: 8px;
        margin-top: 4px;
    }
    .result-block {
        background-color: #0f131c;
        border: 1px solid #1e293b;
        border-radius: 6px;
        padding: 10px;
        opacity: 0;
    }
    .result-block.reveal {
        animation: fadeInUp 0.4s ease forwards;
    }
    .result-block.result-action {
        border-color: #3b82f6;
        background-color: rgba(59, 130, 246, 0.05);
    }
    .result-stage-title {
        font-size: 9px;
        font-weight: 700;
        color: #64748b;
        margin: 0 0 4px 0;
        text-transform: uppercase;
        letter-spacing: 0.05em;
    }
    .result-badge {
        display: inline-block;
        font-size: 10px;
        font-weight: 700;
        padding: 2px 8px;
        border-radius: 3px;
        margin-bottom: 4px;
    }
    .badge-up { background: rgba(16, 185, 129, 0.15); color: #10b981; }
    .badge-down { background: rgba(239, 68, 68, 0.15); color: #ef4444; }
    .badge-side { background: rgba(251, 191, 36, 0.15); color: #f59e0b; }
    .badge-supportive { background: rgba(16, 185, 129, 0.15); color: #10b981; }
    .badge-conflicting { background: rgba(239, 68, 68, 0.15); color: #ef4444; }
    .badge-neutral { background: rgba(148, 163, 184, 0.15); color: #94a3b8; }
    .result-reasoning {
        font-size: 10px;
        color: #94a3b8;
        line-height: 1.4;
        margin: 0;
    }
    .action-call {
        display: block;
        font-size: 12px;
        font-weight: 800;
        color: #3b82f6;
        text-transform: uppercase;
        letter-spacing: 0.05em;
        margin-bottom: 4px;
    }
    .action-green { color: #10b981; }
    .action-red { color: #ef4444; }
    .action-amber { color: #f59e0b; }

    /* Analysis History table */
    .history-box {
        flex: 0 0 auto;
        max-height: 240px;
    }
    .history-content {
        display: flex;
        flex-direction: column;
        overflow: hidden;
    }
    .history-table-wrap {
        overflow-y: auto;
        max-height: 190px;
    }
    .history-table {
        width: 100%;
        border-collapse: collapse;
        font-size: 9px;
        color: #94a3b8;
    }
    .history-table thead {
        position: sticky;
        top: 0;
        background-color: #131722;
    }
    .history-table th {
        text-align: left;
        padding: 3px 2px;
        font-weight: 700;
        color: #64748b;
        text-transform: uppercase;
        letter-spacing: 0.05em;
        border-bottom: 1px solid #1e293b;
    }
    .history-table td {
        padding: 3px 2px;
        border-bottom: 1px solid #0f131c;
        white-space: nowrap;
    }
    .history-table tbody tr:hover {
        background-color: #1a1f2e;
    }
    .col-time { color: #64748b; width: 54px; }
    .col-action { font-weight: 700; }
    .col-price { font-family: ui-monospace, monospace; text-align: right; }
    .col-delta { font-family: ui-monospace, monospace; text-align: right; font-weight: 700; }
    .action-text-green { color: #10b981; }
    .action-text-red { color: #ef4444; }
    .action-text-amber { color: #f59e0b; }
    .delta-positive { color: #10b981; }
    .delta-negative { color: #ef4444; }

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
