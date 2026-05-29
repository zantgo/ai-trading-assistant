<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { getState } from './state.svelte';
    import type { AssistantAnalysis, ChatMessage, MultiAgentAnalysis } from './state.svelte';

    import Header from './components/Header.svelte';
    import SettingsPanel from './components/SettingsPanel.svelte';
    import PriceChart from './components/PriceChart.svelte';
    import VolumeChart from './components/VolumeChart.svelte';
    import AdxChart from './components/AdxChart.svelte';
    import AtrChart from './components/AtrChart.svelte';
    import RsiChart from './components/RsiChart.svelte';
    import MacdChart from './components/MacdChart.svelte';
    import SqueezeChart from './components/SqueezeChart.svelte';

    const app = getState();
    let ws: WebSocket | null = null;
    // svelte-ignore non_reactive_update
    let chatContainer: HTMLDivElement | null = null;

    // Descargar parámetros de configuración desde el backend
    async function fetchConfig() {
        try {
            const res = await fetch(`/api/config?_=${Date.now()}`);
            if (!res.ok) return;
            const config = await res.json();
            
            app.activeSymbol = config.symbol || 'ETH';
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
        } catch (e) {
            console.error('Failed to fetch config from server:', e);
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
                if (snapshot.average_volume) app.avgVolText = parseFloat(snapshot.average_volume).toFixed(2);
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
    });

    async function requestAnalysis() {
        app.assistantLoading = true;
        app.assistantError = null;
        app.assistantResponse = null;
        app.multiAgentResponse = null;
        app.individualResults = [];
        app.analysisPhase = 'phase1';
        app.agentProgress = [
            { name: 'RSI',         status: 'pending' },
            { name: 'MACD',        status: 'pending' },
            { name: 'SQUEEZE',     status: 'pending' },
            { name: 'ADX',         status: 'pending' },
            { name: 'BOLLINGER_ATR', status: 'pending' },
            { name: 'VOLUME_EMA',  status: 'pending' },
            { name: 'VWAP',        status: 'pending' },
        ];

        try {
            const historyRes = await fetch('/api/history');
            const historyData = await historyRes.json();
            const prices: number[] = (historyData.prices || []).map(Number);

            const snap = app.latestSnapshot || {};

            const body = {
                position: app.currentPosition,
                entry_price: app.currentPosition !== 'None' ? (parseFloat(app.entryPriceVal) || 0).toString() : '',
                historical_prices: prices,
                indicators: {
                    rsi: snap.rsi_14 ? parseFloat(String(snap.rsi_14)) : null,
                    squeeze_on: snap.squeeze_on ?? null,
                    squeeze_momentum: snap.squeeze_momentum ? parseFloat(String(snap.squeeze_momentum)) : null,
                    macd_line: snap.macd_line ? parseFloat(String(snap.macd_line)) : null,
                    macd_signal: snap.macd_signal ? parseFloat(String(snap.macd_signal)) : null,
                    macd_histogram: snap.macd_hist ? parseFloat(String(snap.macd_hist)) : null,
                    macd_histogram_trend: snap.macd_hist ? (parseFloat(String(snap.macd_hist)) > 0 ? 'increasing' : 'decreasing') : null,
                    adx: snap.adx_14 ? parseFloat(String(snap.adx_14)) : null,
                    adx_plus: snap.adx_plus ? parseFloat(String(snap.adx_plus)) : null,
                    adx_minus: snap.adx_minus ? parseFloat(String(snap.adx_minus)) : null,
                    bb_upper: snap.bb_upper ? parseFloat(String(snap.bb_upper)) : null,
                    bb_middle: snap.bb_middle ? parseFloat(String(snap.bb_middle)) : null,
                    bb_lower: snap.bb_lower ? parseFloat(String(snap.bb_lower)) : null,
                    atr: snap.atr_14 ? parseFloat(String(snap.atr_14)) : null,
                    current_price: snap.mid_price ? parseFloat(String(snap.mid_price)) : null,
                    volume: snap.volume ? parseFloat(String(snap.volume)) : null,
                    average_volume: snap.average_volume ? parseFloat(String(snap.average_volume)) : null,
                    ema_fast: snap.ema_fast ? parseFloat(String(snap.ema_fast)) : null,
                    ema_medium: snap.ema_medium ? parseFloat(String(snap.ema_medium)) : null,
                    ema_slow: snap.ema_slow ? parseFloat(String(snap.ema_slow)) : null,
                    ema_long: snap.ema_long ? parseFloat(String(snap.ema_long)) : null,
                    vwap: snap.vwap ? parseFloat(String(snap.vwap)) : null,
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

            const analysis: MultiAgentAnalysis = await res.json();
            app.multiAgentResponse = analysis;
            app.individualResults = analysis.phase_one;

            for (let i = 0; i < app.agentProgress.length; i++) {
                const result = analysis.phase_one.find(r => r.indicator_name === app.agentProgress[i].name);
                if (result) {
                    app.agentProgress[i].status = result.signal === 'UNAVAILABLE' ? 'failed' : 'complete';
                } else {
                    app.agentProgress[i].status = 'failed';
                }
            }
            app.analysisPhase = 'complete';
            fetchAssistantHistory();
        } catch (e: any) {
            app.assistantError = e.message || 'Unknown error during analysis';
            app.analysisPhase = 'idle';
        } finally {
            app.assistantLoading = false;
        }
    }

    function scrollChatToBottom() {
        requestAnimationFrame(() => {
            if (chatContainer) {
                chatContainer.scrollTop = chatContainer.scrollHeight;
            }
        });
    }

    function openAssistantModal() {
        if (!app.multiAgentResponse) return;
        const resp = app.multiAgentResponse!;
        const phaseTwo = resp.phase_two;
        const snap = app.latestSnapshot || {};

        const contextLines: string[] = [];
        contextLines.push(`Current position: ${app.currentPosition}`);
        contextLines.push(`Symbol: ${app.activeSymbol}`);
        if (app.currentPosition !== 'None' && app.entryPriceVal) {
            contextLines.push(`Entry price: $${app.entryPriceVal}`);
        }

        const rsi = snap.rsi_14 ? parseFloat(String(snap.rsi_14)) : null;
        const squeezeOn = snap.squeeze_on ?? null;
        const squeezeMom = snap.squeeze_momentum ? parseFloat(String(snap.squeeze_momentum)) : null;
        const macdHist = snap.macd_hist ? parseFloat(String(snap.macd_hist)) : null;
        const adx = snap.adx_14 ? parseFloat(String(snap.adx_14)) : null;
        const atr = snap.atr_14 ? parseFloat(String(snap.atr_14)) : null;
        const emaFast = snap.ema_fast ? parseFloat(String(snap.ema_fast)) : null;
        const emaSlow = snap.ema_slow ? parseFloat(String(snap.ema_slow)) : null;
        const vwap = snap.vwap ? parseFloat(String(snap.vwap)) : null;
        const bbUpper = snap.bb_upper ? parseFloat(String(snap.bb_upper)) : null;
        const bbLower = snap.bb_lower ? parseFloat(String(snap.bb_lower)) : null;
        const price = snap.mid_price ? parseFloat(String(snap.mid_price)) : null;

        if (price !== null) contextLines.push(`Current price: $${price.toFixed(4)}`);
        if (rsi !== null) {
            const rsiDesc = rsi > 70 ? 'overbought' : rsi < 30 ? 'oversold' : 'neutral';
            contextLines.push(`RSI(14): ${rsi.toFixed(2)} (${rsiDesc})`);
        }
        if (squeezeOn !== null) contextLines.push(`Squeeze: ${squeezeOn ? 'ON (potential breakout)' : 'OFF'}`);
        if (macdHist !== null) contextLines.push(`MACD Histogram: ${macdHist.toFixed(4)}`);
        if (adx !== null) contextLines.push(`ADX(14): ${adx.toFixed(2)}`);
        if (atr !== null) contextLines.push(`ATR(14): ${atr.toFixed(4)}`);
        if (emaFast !== null && emaSlow !== null) {
            contextLines.push(`EMA Fast: ${emaFast.toFixed(4)}, EMA Slow: ${emaSlow.toFixed(4)}`);
        }
        if (vwap !== null) contextLines.push(`VWAP: ${vwap.toFixed(4)}`);
        if (bbUpper !== null && bbLower !== null) {
            contextLines.push(`BB Upper: ${bbUpper.toFixed(4)}, BB Lower: ${bbLower.toFixed(4)}`);
        }
        contextLines.push(`Phase 1 Signals: ${phaseTwo.indicator_synthesis.summary_count}`);
        contextLines.push(`Trend: ${phaseTwo.general_trend}`);
        contextLines.push(`Support: ${phaseTwo.support_and_resistance.detected_support_levels.join(', ')}`);
        contextLines.push(`Resistance: ${phaseTwo.support_and_resistance.detected_resistance_levels.join(', ')}`);
        contextLines.push(`Recommendation: ${phaseTwo.position_recommendation.action} — ${phaseTwo.position_recommendation.rationale}`);

        const systemContext = contextLines.join('\n');

        const assistantGreeting = [
            `Hello! Based on my multi-agent technical analysis, I recommend **${phaseTwo.position_recommendation.action}**.`,
            ``,
            `**Market Trend:** ${phaseTwo.general_trend}`,
            ``,
            `**Indicator Consensus:** ${phaseTwo.indicator_synthesis.summary_count}`,
            `${phaseTwo.indicator_synthesis.evaluation}`,
            ``,
            `**Support/Resistance Analysis:** ${phaseTwo.support_and_resistance.structural_analysis}`,
            ``,
            `**Rationale:** ${phaseTwo.position_recommendation.rationale}`,
            ``,
            `Feel free to ask me about any specific indicator or market condition — I'm here to help you understand the data.`,
        ].join('\n');

        app.chatHistory = [
            { role: 'system', content: systemContext },
            { role: 'assistant', content: assistantGreeting },
        ];
        app.isAssistantModalOpen = true;
        scrollChatToBottom();
    }

    function closeAssistantModal() {
        app.isAssistantModalOpen = false;
    }

    async function sendChatMessage() {
        const text = app.chatInputText.trim();
        if (!text || app.isChatLoading) return;

        app.chatHistory.push({ role: 'user', content: text });
        app.chatInputText = '';
        app.isChatLoading = true;
        scrollChatToBottom();

        try {
            const res = await fetch('/api/chat', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ history: app.chatHistory }),
            });

            if (!res.ok) {
                throw new Error(`Server returned ${res.status}`);
            }

            const data = await res.json();
            app.chatHistory.push({ role: 'assistant', content: data.reply });
            scrollChatToBottom();
        } catch (e: any) {
            app.chatHistory.push({
                role: 'assistant',
                content: `Sorry, I couldn't process that request: ${e.message || 'Unknown error'}`,
            });
            scrollChatToBottom();
        } finally {
            app.isChatLoading = false;
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

                    {#if app.currentPosition !== 'None'}
                        <div class="entry-price-input">
                            <label for="entryPrice">Entry Price ($):</label>
                            <input id="entryPrice" type="number" step="any"
                                   bind:value={app.entryPriceVal} placeholder="0.00" />
                        </div>
                    {/if}

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
                            <span class="status-text">
                                {app.analysisPhase === 'phase1' ? 'Phase 1: Running 7 indicator agents...' : 'Phase 2: Synthesizing master report...'}
                            </span>
                        </div>
                        <div class="agent-progress-list">
                            {#each app.agentProgress as agent (agent.name)}
                                <div class="agent-progress-item"
                                    class:ap-complete={agent.status === 'complete'}
                                    class:ap-failed={agent.status === 'failed'}
                                    class:ap-running={agent.status === 'pending' && app.analysisPhase === 'phase1'}
                                >
                                    <span class="ap-name">{agent.name}</span>
                                    <span class="ap-status">
                                        {#if agent.status === 'complete'}
                                            ✓
                                        {:else if agent.status === 'failed'}
                                            ✗
                                        {:else}
                                            ···
                                        {/if}
                                    </span>
                                </div>
                            {/each}
                        </div>
                    {/if}

                    {#if app.assistantError}
                        <div class="error-box">
                            <span>Failed: {app.assistantError}</span>
                        </div>
                    {/if}

                    {#if app.multiAgentResponse && !app.assistantLoading}
                        {@const resp = app.multiAgentResponse}
                        {@const pt = resp.phase_two}
                        <div class="analysis-result clickable-result" onclick={openAssistantModal} role="button" tabindex="0" onkeydown={(e) => { if (e.key === 'Enter') openAssistantModal() }}>
                            <div class="result-block reveal" style="animation-delay: 0ms">
                                <h4 class="result-stage-title">Phase 1 — Indicator Consensus</h4>
                                <span class="consensus-badge"
                                    class:badge-up={pt.general_trend === 'UPWARD'}
                                    class:badge-down={pt.general_trend === 'DOWNWARD'}
                                    class:badge-side={pt.general_trend === 'SIDEWAYS'}
                                >
                                    {pt.indicator_synthesis.summary_count}
                                </span>
                            </div>

                            <div class="result-block reveal" style="animation-delay: 150ms">
                                <h4 class="result-stage-title">Phase 2 — Trend & Structure</h4>
                                <span class="result-badge"
                                    class:badge-up={pt.general_trend === 'UPWARD'}
                                    class:badge-down={pt.general_trend === 'DOWNWARD'}
                                    class:badge-side={pt.general_trend === 'SIDEWAYS'}
                                >
                                    {pt.general_trend}
                                </span>
                                <p class="result-reasoning">{pt.indicator_synthesis.evaluation.substring(0, 120)}...</p>
                            </div>

                            <div class="result-block result-action reveal" style="animation-delay: 300ms">
                                <h4 class="result-stage-title">3. Position Recommendation</h4>
                                <span
                                    class="action-call"
                                    class:action-green={pt.position_recommendation.action === 'Hold' || pt.position_recommendation.action === 'Open Long'}
                                    class:action-red={pt.position_recommendation.action === 'Close'}
                                    class:action-amber={pt.position_recommendation.action === 'Wait' || pt.position_recommendation.action === 'Open Short'}
                                >
                                    {pt.position_recommendation.action}
                                </span>
                                <p class="result-reasoning">{pt.position_recommendation.rationale.substring(0, 150)}...</p>
                            </div>
                            <div class="click-hint">Click for full multi-agent analysis & chat</div>
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
                                        {@const recPrice = parseFloat(rec.price_at_analysis) || 0}
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
                                            <td class="col-price">{rec.price_at_analysis.substring(0, 8)}</td>
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

    <SettingsPanel />

    {#if app.isAssistantModalOpen && app.multiAgentResponse}
        {@const resp = app.multiAgentResponse!}
        {@const pt = resp.phase_two}
        {@const indicators = resp.phase_one}
        {@const snap = app.latestSnapshot || {}}
        {@const rsi = snap.rsi_14 ? parseFloat(String(snap.rsi_14)) : null}
        {@const squeezeOn = snap.squeeze_on ?? null}
        {@const squeezeMom = snap.squeeze_momentum ? parseFloat(String(snap.squeeze_momentum)) : null}
        {@const macdHist = snap.macd_hist ? parseFloat(String(snap.macd_hist)) : null}
        {@const macdLine = snap.macd_line ? parseFloat(String(snap.macd_line)) : null}
        {@const macdSig = snap.macd_signal ? parseFloat(String(snap.macd_signal)) : null}
        {@const adx = snap.adx_14 ? parseFloat(String(snap.adx_14)) : null}
        {@const adxPlus = snap.adx_plus ? parseFloat(String(snap.adx_plus)) : null}
        {@const adxMinus = snap.adx_minus ? parseFloat(String(snap.adx_minus)) : null}
        {@const atr = snap.atr_14 ? parseFloat(String(snap.atr_14)) : null}
        {@const emaFast = snap.ema_fast ? parseFloat(String(snap.ema_fast)) : null}
        {@const emaSlow = snap.ema_slow ? parseFloat(String(snap.ema_slow)) : null}
        {@const vwap = snap.vwap ? parseFloat(String(snap.vwap)) : null}
        {@const bbUpper = snap.bb_upper ? parseFloat(String(snap.bb_upper)) : null}
        {@const bbMiddle = snap.bb_middle ? parseFloat(String(snap.bb_middle)) : null}
        {@const bbLower = snap.bb_lower ? parseFloat(String(snap.bb_lower)) : null}
        {@const price = snap.mid_price ? parseFloat(String(snap.mid_price)) : null}
        <!-- svelte-ignore a11y_interactive_supports_focus a11y_click_events_have_key_events -->
        <div class="modal-backdrop" onclick={closeAssistantModal} role="dialog">
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div class="modal-window" onclick={(e) => e.stopPropagation()}>
                <div class="modal-header">
                    <h2 class="modal-title">AI Copilot Intelligence Hub — {app.activeSymbol}</h2>
                    <button class="modal-close-btn" onclick={closeAssistantModal}>&#10005;</button>
                </div>

                <div class="modal-body">
                    <!-- LEFT: Multi-Agent Analysis -->
                    <div class="modal-left">
                        <!-- Phase 2: Master Synthesis -->
                        <div class="master-synthesis">
                            <h3 class="section-heading">Phase 2 — Master Synthesis</h3>

                            <div class="sr-ribbon">
                                <div class="sr-block sr-support">
                                    <span class="sr-label">SUPPORT</span>
                                    {#if pt.support_and_resistance.detected_support_levels.length > 0}
                                        {#each pt.support_and_resistance.detected_support_levels as lvl}
                                            <span class="sr-level">{lvl}</span>
                                        {/each}
                                    {:else}
                                        <span class="sr-level sr-none">None detected</span>
                                    {/if}
                                </div>
                                <div class="sr-block sr-current">
                                    <span class="sr-label">PRICE</span>
                                    <span class="sr-level sr-price-label">{price !== null ? price.toFixed(4) : '--'}</span>
                                </div>
                                <div class="sr-block sr-resistance">
                                    <span class="sr-label">RESISTANCE</span>
                                    {#if pt.support_and_resistance.detected_resistance_levels.length > 0}
                                        {#each pt.support_and_resistance.detected_resistance_levels as lvl}
                                            <span class="sr-level">{lvl}</span>
                                        {/each}
                                    {:else}
                                        <span class="sr-level sr-none">None detected</span>
                                    {/if}
                                </div>
                            </div>
                            <p class="sr-structural">{pt.support_and_resistance.structural_analysis}</p>

                            <div class="decision-callout"
                                class:decision-green={pt.position_recommendation.action === 'Hold' || pt.position_recommendation.action === 'Open Long'}
                                class:decision-red={pt.position_recommendation.action === 'Close'}
                                class:decision-amber={pt.position_recommendation.action === 'Wait' || pt.position_recommendation.action === 'Open Short'}
                            >
                                <span class="decision-action">{pt.position_recommendation.action}</span>
                                <span class="decision-trend">{pt.general_trend}</span>
                                <p class="decision-rationale">{pt.position_recommendation.rationale}</p>
                            </div>

                            <div class="synthesis-summary">
                                <span class="synth-count">{pt.indicator_synthesis.summary_count}</span>
                                <p class="synth-eval">{pt.indicator_synthesis.evaluation}</p>
                            </div>
                        </div>

                        <!-- Phase 1: Individual Indicator Grid -->
                        <h3 class="section-heading">Phase 1 — Individual Indicator Agents</h3>
                        <div class="indicator-grid">
                            {#each indicators as ind}
                                <div class="phase-one-card"
                                    class:poc-bullish={ind.signal === 'BULLISH'}
                                    class:poc-bearish={ind.signal === 'BEARISH'}
                                    class:poc-sideways={ind.signal === 'SIDEWAYS'}
                                    class:poc-unavailable={ind.signal === 'UNAVAILABLE'}
                                >
                                    <span class="poc-name">{ind.indicator_name}</span>
                                    <span class="poc-signal">
                                        {#if ind.signal === 'BULLISH'}
                                            ▲ BULLISH
                                        {:else if ind.signal === 'BEARISH'}
                                            ▼ BEARISH
                                        {:else if ind.signal === 'SIDEWAYS'}
                                            ◆ SIDEWAYS
                                        {:else}
                                            ✕ UNAVAILABLE
                                        {/if}
                                    </span>
                                    <p class="poc-reason">{ind.reason}</p>
                                </div>
                            {/each}
                        </div>

                        <!-- Assistant Summary -->
                        <div class="assistant-summary">
                            <h3 class="section-heading">Assistant Summary</h3>
                            <div class="summary-message">
                                {#each app.chatHistory.filter(m => m.role === 'assistant') as msg}
                                    <p class="summary-text">{msg.content}</p>
                                {/each}
                            </div>
                        </div>
                    </div>

                    <!-- RIGHT: Interactive Chat -->
                    <div class="modal-right">
                        <h3 class="section-heading">Real-time Chat</h3>

                        <div class="chat-thread" bind:this={chatContainer}>
                            {#each app.chatHistory.filter(m => m.role !== 'system') as msg, i (i)}
                                <div class="chat-bubble" class:user-bubble={msg.role === 'user'} class:assistant-bubble={msg.role === 'assistant'}>
                                    <span class="bubble-role">{msg.role === 'user' ? 'You' : 'Assistant'}</span>
                                    <span class="bubble-content">{msg.content}</span>
                                </div>
                            {/each}
                            {#if app.isChatLoading}
                                <div class="chat-bubble assistant-bubble typing-bubble">
                                    <span class="bubble-role">Assistant</span>
                                    <span class="bubble-content"><span class="typing-dots">Thinking<span class="dot-anim">.</span><span class="dot-anim">.</span><span class="dot-anim">.</span></span></span>
                                </div>
                            {/if}
                        </div>

                        <div class="chat-input-area">
                            <input
                                type="text"
                                class="chat-input"
                                placeholder="Ask about indicators, market conditions..."
                                bind:value={app.chatInputText}
                                disabled={app.isChatLoading}
                                onkeydown={(e) => { if (e.key === 'Enter') sendChatMessage() }}
                            />
                            <button
                                class="chat-send-btn"
                                onclick={sendChatMessage}
                                disabled={app.isChatLoading || !app.chatInputText.trim()}
                            >
                                Send
                            </button>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    {/if}
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
    .signals-content {
        font-size: 11px;
        color: #94a3b8;
        display: flex;
        flex-direction: column;
        gap: 2px;
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

    /* Clickable result area */
    .clickable-result {
        cursor: pointer;
        border-radius: 6px;
        padding: 2px;
        transition: box-shadow 0.2s, border-color 0.2s;
        border: 1px solid transparent;
    }
    .clickable-result:hover {
        border-color: #3b82f6;
        box-shadow: 0 0 12px rgba(59, 130, 246, 0.15);
    }
    .click-hint {
        text-align: center;
        font-size: 9px;
        color: #64748b;
        margin-top: 6px;
        text-transform: uppercase;
        letter-spacing: 0.05em;
    }

    /* Modal overlay */
    .modal-backdrop {
        position: fixed;
        inset: 0;
        z-index: 1000;
        background: rgba(0, 0, 0, 0.75);
        display: flex;
        align-items: center;
        justify-content: center;
        animation: fadeIn 0.2s ease;
    }
    @keyframes fadeIn {
        from { opacity: 0; }
        to { opacity: 1; }
    }
    @keyframes fadeInUp {
        from { opacity: 0; transform: translateY(6px); }
        to { opacity: 1; transform: translateY(0); }
    }

    .modal-window {
        background: #131722;
        border: 1px solid #2a2e39;
        border-radius: 12px;
        width: 95vw;
        max-width: 1100px;
        max-height: 85vh;
        display: flex;
        flex-direction: column;
        box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
        animation: fadeInUp 0.25s ease;
    }

    .modal-header {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 16px 24px;
        border-bottom: 1px solid #1e293b;
    }
    .modal-title {
        font-size: 14px;
        font-weight: 700;
        color: #e2e8f0;
        margin: 0;
        letter-spacing: 0.03em;
    }
    .modal-close-btn {
        background: none;
        border: 1px solid #2a2e39;
        color: #94a3b8;
        font-size: 16px;
        cursor: pointer;
        width: 32px;
        height: 32px;
        border-radius: 6px;
        display: flex;
        align-items: center;
        justify-content: center;
        transition: background 0.15s, color 0.15s;
    }
    .modal-close-btn:hover {
        background: #1e293b;
        color: #f1f5f9;
    }

    .modal-body {
        display: flex;
        flex: 1;
        overflow: hidden;
    }

    /* Left column: Indicator breakdown */
    .modal-left {
        width: 50%;
        padding: 20px;
        overflow-y: auto;
        border-right: 1px solid #1e293b;
    }
    .modal-left .section-heading {
        font-size: 11px;
        font-weight: 700;
        color: #64748b;
        text-transform: uppercase;
        letter-spacing: 0.08em;
        margin: 0 0 12px 0;
        padding-bottom: 6px;
        border-bottom: 1px solid #1e293b;
    }

    /* Assistant summary */
    .assistant-summary {
        margin-top: 16px;
    }
    .summary-message {
        background: rgba(59, 130, 246, 0.05);
        border: 1px solid rgba(59, 130, 246, 0.15);
        border-radius: 8px;
        padding: 12px;
    }
    .summary-text {
        font-size: 11px;
        color: #cbd5e1;
        line-height: 1.6;
        margin: 0 0 4px 0;
        white-space: pre-wrap;
    }

    /* Right column: Chat */
    .modal-right {
        width: 50%;
        display: flex;
        flex-direction: column;
        padding: 20px;
        overflow: hidden;
    }
    .modal-right .section-heading {
        font-size: 11px;
        font-weight: 700;
        color: #64748b;
        text-transform: uppercase;
        letter-spacing: 0.08em;
        margin: 0 0 12px 0;
        padding-bottom: 6px;
        border-bottom: 1px solid #1e293b;
        flex-shrink: 0;
    }

    .chat-thread {
        flex: 1;
        overflow-y: auto;
        display: flex;
        flex-direction: column;
        gap: 8px;
        padding-right: 4px;
    }
    .chat-thread::-webkit-scrollbar {
        width: 4px;
    }
    .chat-thread::-webkit-scrollbar-track {
        background: transparent;
    }
    .chat-thread::-webkit-scrollbar-thumb {
        background: #2a2e39;
        border-radius: 2px;
    }

    .chat-bubble {
        max-width: 85%;
        padding: 10px 12px;
        border-radius: 8px;
        font-size: 11px;
        line-height: 1.5;
        animation: fadeInUp 0.2s ease;
    }
    .user-bubble {
        align-self: flex-end;
        background: #1e40af;
        border: 1px solid #3b82f6;
        color: #e2e8f0;
    }
    .assistant-bubble {
        align-self: flex-start;
        background: #0f131c;
        border: 1px solid #1e293b;
        color: #cbd5e1;
    }
    .bubble-role {
        display: block;
        font-size: 8px;
        font-weight: 700;
        text-transform: uppercase;
        letter-spacing: 0.05em;
        margin-bottom: 3px;
        color: #64748b;
    }
    .bubble-content {
        white-space: pre-wrap;
        word-break: break-word;
    }

    .typing-bubble {
        opacity: 0.7;
    }
    .typing-dots {
        color: #94a3b8;
        font-style: italic;
    }
    .dot-anim {
        animation: dotPulse 1.4s infinite;
    }
    .dot-anim:nth-child(1) { animation-delay: 0s; }
    .dot-anim:nth-child(2) { animation-delay: 0.2s; }
    .dot-anim:nth-child(3) { animation-delay: 0.4s; }

    @keyframes dotPulse {
        0%, 20% { opacity: 0; }
        50% { opacity: 1; }
        80%, 100% { opacity: 0; }
    }

    .chat-input-area {
        display: flex;
        gap: 8px;
        margin-top: 12px;
        flex-shrink: 0;
    }
    .chat-input {
        flex: 1;
        background: #0f131c;
        border: 1px solid #2a2e39;
        border-radius: 6px;
        padding: 8px 12px;
        color: #e2e8f0;
        font-size: 11px;
        font-family: ui-sans-serif, system-ui, sans-serif;
        outline: none;
        transition: border-color 0.15s;
    }
    .chat-input:focus {
        border-color: #3b82f6;
    }
    .chat-input:disabled {
        opacity: 0.5;
        cursor: not-allowed;
    }
    .chat-input::placeholder {
        color: #4c525e;
    }
    .chat-send-btn {
        background: linear-gradient(135deg, #1e40af, #3b82f6);
        color: #f1f5f9;
        border: 1px solid #3b82f6;
        border-radius: 6px;
        padding: 8px 16px;
        font-size: 11px;
        font-weight: 700;
        text-transform: uppercase;
        letter-spacing: 0.05em;
        cursor: pointer;
        transition: opacity 0.2s;
        white-space: nowrap;
    }
    .chat-send-btn:hover:not(:disabled) {
        background: linear-gradient(135deg, #1e3a8a, #2563eb);
    }
    .chat-send-btn:disabled {
        opacity: 0.5;
        cursor: not-allowed;
    }

    @media (max-width: 768px) {
        .modal-window {
            max-width: 100vw;
            max-height: 100vh;
            border-radius: 0;
        }
        .modal-body {
            flex-direction: column;
        }
        .modal-left {
            width: 100%;
            max-height: none;
            border-right: none;
            border-bottom: 1px solid #1e293b;
        }
        .modal-right {
            width: 100%;
            flex: 1;
            max-height: 50vh;
        }
        .indicator-grid {
            grid-template-columns: 1fr;
        }
        .sr-ribbon {
            flex-direction: column;
            gap: 6px;
        }
    }

    /* Entry price input */
    .entry-price-input {
        display: flex;
        align-items: center;
        gap: 8px;
        margin-bottom: 10px;
    }
    .entry-price-input label {
        font-size: 10px;
        font-weight: 600;
        color: #64748b;
        text-transform: uppercase;
        letter-spacing: 0.05em;
        white-space: nowrap;
    }
    .entry-price-input input {
        flex: 1;
        background: #0f131c;
        border: 1px solid #2a2e39;
        border-radius: 4px;
        padding: 5px 8px;
        color: #e2e8f0;
        font-size: 11px;
        font-family: ui-monospace, monospace;
        outline: none;
        width: 100%;
    }
    .entry-price-input input:focus {
        border-color: #3b82f6;
    }

    /* Agent progress list */
    .agent-progress-list {
        display: flex;
        flex-direction: column;
        gap: 2px;
        margin-bottom: 8px;
    }
    .agent-progress-item {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: 3px 6px;
        border-radius: 3px;
        font-size: 9px;
        background: #0f131c;
    }
    .ap-name {
        color: #94a3b8;
        font-weight: 600;
        letter-spacing: 0.03em;
    }
    .ap-status {
        font-size: 10px;
    }
    .ap-complete { background: rgba(16, 185, 129, 0.08); }
    .ap-complete .ap-status { color: #10b981; }
    .ap-failed { background: rgba(239, 68, 68, 0.08); }
    .ap-failed .ap-status { color: #ef4444; }
    .ap-running .ap-status { color: #3b82f6; animation: pulse 1.5s infinite; }

    /* Consensus badge */
    .consensus-badge {
        display: inline-block;
        font-size: 10px;
        font-weight: 700;
        padding: 3px 10px;
        border-radius: 4px;
        background: rgba(59, 130, 246, 0.1);
        color: #3b82f6;
    }

    /* Master synthesis */
    .master-synthesis {
        margin-bottom: 16px;
    }

    /* Support / Resistance ribbon */
    .sr-ribbon {
        display: flex;
        gap: 2px;
        margin-bottom: 8px;
    }
    .sr-block {
        flex: 1;
        padding: 8px;
        border-radius: 6px;
        text-align: center;
    }
    .sr-support {
        background: rgba(16, 185, 129, 0.08);
        border: 1px solid rgba(16, 185, 129, 0.2);
    }
    .sr-resistance {
        background: rgba(239, 68, 68, 0.08);
        border: 1px solid rgba(239, 68, 68, 0.2);
    }
    .sr-current {
        background: rgba(59, 130, 246, 0.08);
        border: 1px solid rgba(59, 130, 246, 0.25);
    }
    .sr-label {
        display: block;
        font-size: 8px;
        font-weight: 700;
        text-transform: uppercase;
        letter-spacing: 0.08em;
        margin-bottom: 3px;
        color: #64748b;
    }
    .sr-level {
        display: block;
        font-size: 11px;
        font-weight: 600;
        font-family: ui-monospace, monospace;
        color: #e2e8f0;
    }
    .sr-support .sr-level { color: #10b981; }
    .sr-resistance .sr-level { color: #ef4444; }
    .sr-current .sr-level { color: #3b82f6; }
    .sr-price-label { font-size: 13px; font-weight: 800; }
    .sr-none { font-size: 9px; color: #4c525e !important; font-style: italic; }
    .sr-structural {
        font-size: 10px;
        color: #94a3b8;
        line-height: 1.4;
        margin: 0 0 8px 0;
    }

    /* Decision callout */
    .decision-callout {
        padding: 12px;
        border-radius: 8px;
        margin-bottom: 10px;
        text-align: center;
    }
    .decision-green {
        background: rgba(16, 185, 129, 0.08);
        border: 1px solid rgba(16, 185, 129, 0.25);
    }
    .decision-red {
        background: rgba(239, 68, 68, 0.08);
        border: 1px solid rgba(239, 68, 68, 0.25);
    }
    .decision-amber {
        background: rgba(251, 191, 36, 0.08);
        border: 1px solid rgba(251, 191, 36, 0.25);
    }
    .decision-action {
        display: block;
        font-size: 16px;
        font-weight: 800;
        text-transform: uppercase;
        letter-spacing: 0.08em;
        margin-bottom: 2px;
    }
    .decision-green .decision-action { color: #10b981; }
    .decision-red .decision-action { color: #ef4444; }
    .decision-amber .decision-action { color: #f59e0b; }
    .decision-trend {
        display: block;
        font-size: 10px;
        font-weight: 600;
        color: #64748b;
        text-transform: uppercase;
        letter-spacing: 0.05em;
        margin-bottom: 6px;
    }
    .decision-rationale {
        font-size: 10px;
        color: #94a3b8;
        line-height: 1.4;
        margin: 0;
    }

    /* Synthesis summary */
    .synthesis-summary {
        background: #0f131c;
        border: 1px solid #1e293b;
        border-radius: 6px;
        padding: 10px;
    }
    .synth-count {
        display: block;
        font-size: 11px;
        font-weight: 700;
        color: #3b82f6;
        margin-bottom: 4px;
    }
    .synth-eval {
        font-size: 10px;
        color: #94a3b8;
        line-height: 1.4;
        margin: 0;
    }

    /* Phase 1 indicator grid */
    .indicator-grid {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 6px;
        margin-bottom: 16px;
    }
    .phase-one-card {
        background: #0f131c;
        border: 1px solid #1e293b;
        border-radius: 5px;
        padding: 8px 10px;
    }
    .phase-one-card.poc-bullish {
        border-color: rgba(16, 185, 129, 0.3);
        background: rgba(16, 185, 129, 0.04);
    }
    .phase-one-card.poc-bearish {
        border-color: rgba(239, 68, 68, 0.3);
        background: rgba(239, 68, 68, 0.04);
    }
    .phase-one-card.poc-sideways {
        border-color: rgba(251, 191, 36, 0.3);
        background: rgba(251, 191, 36, 0.04);
    }
    .phase-one-card.poc-unavailable {
        border-color: rgba(100, 116, 139, 0.2);
        background: rgba(100, 116, 139, 0.03);
        opacity: 0.6;
    }
    .poc-name {
        display: block;
        font-size: 9px;
        font-weight: 700;
        color: #64748b;
        text-transform: uppercase;
        letter-spacing: 0.05em;
        margin-bottom: 2px;
    }
    .poc-signal {
        display: block;
        font-size: 10px;
        font-weight: 700;
        margin-bottom: 3px;
    }
    .poc-bullish .poc-signal { color: #10b981; }
    .poc-bearish .poc-signal { color: #ef4444; }
    .poc-sideways .poc-signal { color: #f59e0b; }
    .poc-unavailable .poc-signal { color: #64748b; }
    .poc-reason {
        font-size: 9px;
        color: #94a3b8;
        line-height: 1.3;
        margin: 0;
    }
</style>
