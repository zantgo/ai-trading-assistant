<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { getState } from './state.svelte';
    import type { AssistantAnalysis, ChatMessage, MultiAgentAnalysis, PairState } from './state.svelte';

    import PriceChart from './components/PriceChart.svelte';
    import VolumeChart from './components/VolumeChart.svelte';
    import AdxChart from './components/AdxChart.svelte';
    import AtrChart from './components/AtrChart.svelte';
    import RsiChart from './components/RsiChart.svelte';
    import MacdChart from './components/MacdChart.svelte';
    import SqueezeChart from './components/SqueezeChart.svelte';
    import PerformanceDashboard from './components/PerformanceDashboard.svelte';
    import TabHeader from './components/TabHeader.svelte';

    const app = getState();
    let ws: WebSocket | null = null;
    let configReady = false;
    let chatContainer: HTMLDivElement | null = null;

    // Config draft states for localized workspace settings editing
    let activeSettingsPairKey = $state('');
    let draftSymbol = $state('');
    let draftExchange = $state('Hyperliquid');
    let draftDurationValue = $state(60);
    let draftDurationUnit = $state<'seconds' | 'minutes' | 'hours'>('seconds');
    let draftEmaFast = $state(10);
    let draftEmaMedium = $state(50);
    let draftEmaSlow = $state(100);
    let draftEmaLong = $state(200);
    let draftRsiPeriod = $state(14);
    let draftMacdFast = $state(12);
    let draftMacdSlow = $state(26);
    let draftMacdSignal = $state(9);
    let draftAdxPeriod = $state(14);
    let draftAtrPeriod = $state(14);
    let draftSqueezePeriod = $state(20);

    let draftShowEmas = $state(true);
    let draftShowBb = $state(true);
    let draftShowVwap = $state(true);
    let draftShowVolume = $state(true);
    let draftShowAdx = $state(true);
    let draftShowAtr = $state(true);
    let draftShowRsi = $state(true);
    let draftShowMacd = $state(true);
    let draftShowSqueeze = $state(true);

    let draftAutomationEnabled = $state(false);
    let draftAutomationIntervalValue = $state(15);
    let draftAutomationIntervalUnit = $state<'seconds' | 'minutes' | 'hours'>('minutes');

    let draftApiKey = $state('');
    let apiKeyStatus = $state<'idle' | 'saving' | 'success' | 'error'>('idle');
    let apiKeyError = $state('');

    let draftRules = $state('');
    let rulesStatus = $state<'idle' | 'loading' | 'saving' | 'success' | 'error'>('idle');

    // Load active settings states when settings tab opens on any pair
    function syncSettingsDraft(pairKey: string, pair: PairState) {
        activeSettingsPairKey = pairKey;
        draftSymbol = pair.symbol;
        draftExchange = pair.exchange;

        const sec = pair.barDurationSec;
        if (sec % 3600 === 0) {
            draftDurationValue = sec / 3600;
            draftDurationUnit = 'hours';
        } else if (sec % 60 === 0) {
            draftDurationValue = sec / 60;
            draftDurationUnit = 'minutes';
        } else {
            draftDurationValue = sec;
            draftDurationUnit = 'seconds';
        }

        draftEmaFast = pair.emaFastVal;
        draftEmaMedium = pair.emaMediumVal;
        draftEmaSlow = pair.emaSlowVal;
        draftEmaLong = pair.emaLongVal;
        draftRsiPeriod = pair.rsiPeriodVal;
        draftMacdFast = pair.macdFastVal;
        draftMacdSlow = pair.macdSlowVal;
        draftMacdSignal = pair.macdSignalVal;
        draftAdxPeriod = pair.adxPeriodVal;
        draftAtrPeriod = pair.atrPeriodVal;
        draftSqueezePeriod = pair.squeezePeriodVal;

        draftShowEmas = pair.showEmas;
        draftShowBb = pair.showBb;
        draftShowVwap = pair.showVwap;
        draftShowVolume = pair.showVolume;
        draftShowAdx = pair.showAdx;
        draftShowAtr = pair.showAtr;
        draftShowRsi = pair.showRsi;
        draftShowMacd = pair.showMacd;
        draftShowSqueeze = pair.showSqueeze;

        draftAutomationEnabled = pair.automationEnabled;
        const autoSec = pair.automationIntervalValue;
        if (pair.automationIntervalUnit === 'hours') {
            draftAutomationIntervalValue = autoSec / 3600;
            draftAutomationIntervalUnit = 'hours';
        } else if (pair.automationIntervalUnit === 'minutes') {
            draftAutomationIntervalValue = autoSec / 60;
            draftAutomationIntervalUnit = 'minutes';
        } else {
            draftAutomationIntervalValue = autoSec;
            draftAutomationIntervalUnit = 'seconds';
        }
    }

    let calculatedDuration = $derived.by(() => {
        const val = Number(draftDurationValue) || 1;
        if (draftDurationUnit === 'hours') return val * 3600;
        if (draftDurationUnit === 'minutes') return val * 60;
        return val;
    });

    let calculatedAutomationInterval = $derived.by(() => {
        const val = Number(draftAutomationIntervalValue) || 1;
        if (draftAutomationIntervalUnit === 'hours') return val * 3600;
        if (draftAutomationIntervalUnit === 'minutes') return val * 60;
        return val;
    });

    function formatIntervalRemaining(totalSeconds: number): string {
        const h = Math.floor(totalSeconds / 3600);
        const m = Math.floor((totalSeconds % 3600) / 60);
        const s = totalSeconds % 60;
        if (h > 0) return `${h}h ${m.toString().padStart(2, '0')}m`;
        if (m > 0) return `${m}m ${s.toString().padStart(2, '0')}s`;
        return `${s}s`;
    }

    async function applySettings(pairKey: string, pair: PairState) {
        const cleanedSymbol = draftSymbol.trim().toUpperCase();
        if (!/^[A-Z0-9]{2,10}$/.test(cleanedSymbol)) {
            alert("Invalid Ticker. Must be 2-10 characters (alphanumeric).");
            return;
        }

        const body = {
            candles: { duration_seconds: Number(calculatedDuration) },
            indicators: {
                ema_fast: Number(draftEmaFast),
                ema_medium: Number(draftEmaMedium),
                ema_slow: Number(draftEmaSlow),
                ema_long: Number(draftEmaLong),
                rsi_period: Number(draftRsiPeriod),
                macd_fast: Number(draftMacdFast),
                macd_slow: Number(draftMacdSlow),
                macd_signal: Number(draftMacdSignal),
                adx_period: Number(draftAdxPeriod),
                atr_period: Number(draftAtrPeriod),
                squeeze_period: Number(draftSqueezePeriod)
            },
            automation: {
                enabled: draftAutomationEnabled,
                interval_seconds: Number(calculatedAutomationInterval)
            }
        };

        const isIdentityChanged = cleanedSymbol !== pair.symbol || draftExchange !== pair.exchange;

        try {
            if (isIdentityChanged) {
                const newPairKey = `${draftExchange}-${cleanedSymbol}`;

                await fetch(`/api/pairs`, {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ symbol: cleanedSymbol, exchange: draftExchange }),
                });

                await fetch(`/api/pairs/${encodeURIComponent(newPairKey)}/config`, {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify(body)
                });

                await fetch(`/api/pairs/${encodeURIComponent(pairKey)}`, { method: 'DELETE' });

                app.initPair(cleanedSymbol, draftExchange);

                const next = app.pairsMap[newPairKey];
                if (next) {
                    next.barDurationSec = calculatedDuration;
                    next.emaFastVal = draftEmaFast;
                    next.emaMediumVal = draftEmaMedium;
                    next.emaSlowVal = draftEmaSlow;
                    next.emaLongVal = draftEmaLong;
                    next.rsiPeriodVal = draftRsiPeriod;
                    next.macdFastVal = draftMacdFast;
                    next.macdSlowVal = draftMacdSlow;
                    next.macdSignalVal = draftMacdSignal;
                    next.adxPeriodVal = draftAdxPeriod;
                    next.atrPeriodVal = draftAtrPeriod;
                    next.squeezePeriodVal = draftSqueezePeriod;
                    next.automationEnabled = draftAutomationEnabled;
                    next.automationIntervalValue = draftAutomationIntervalValue;
                    next.automationIntervalUnit = draftAutomationIntervalUnit;
                    next.nextEvaluationIn = draftAutomationEnabled ? formatIntervalRemaining(calculatedAutomationInterval) : '--';
                }

                app.removePair(pairKey);
                app.activeTab = newPairKey;
            } else {
                await fetch(`/api/pairs/${encodeURIComponent(pairKey)}/config`, {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify(body)
                });

                pair.barDurationSec = calculatedDuration;
                pair.emaFastVal = draftEmaFast;
                pair.emaMediumVal = draftEmaMedium;
                pair.emaSlowVal = draftEmaSlow;
                pair.emaLongVal = draftEmaLong;
                pair.rsiPeriodVal = draftRsiPeriod;
                pair.macdFastVal = draftMacdFast;
                pair.macdSlowVal = draftMacdSlow;
                pair.macdSignalVal = draftMacdSignal;
                pair.adxPeriodVal = draftAdxPeriod;
                pair.atrPeriodVal = draftAtrPeriod;
                pair.squeezePeriodVal = draftSqueezePeriod;

                pair.latestSnapshot = null;
                pair.priceText = '--';
                pair.vwapText = '--';
            }

            pair.showEmas = draftShowEmas;
            pair.showBb = draftShowBb;
            pair.showVwap = draftShowVwap;
            pair.showVolume = draftShowVolume;
            pair.showAdx = draftShowAdx;
            pair.showAtr = draftShowAtr;
            pair.showRsi = draftShowRsi;
            pair.showMacd = draftShowMacd;
            pair.showSqueeze = draftShowSqueeze;

            pair.automationEnabled = draftAutomationEnabled;
            pair.automationIntervalValue = draftAutomationIntervalValue;
            pair.automationIntervalUnit = draftAutomationIntervalUnit;
            pair.nextEvaluationIn = draftAutomationEnabled ? formatIntervalRemaining(calculatedAutomationInterval) : '--';

            pair.currentView = 'terminal';
        } catch (e) {
            console.error("Save config exception:", e);
        }
    }

    async function fetchRules() {
        rulesStatus = 'loading';
        try {
            const res = await fetch('/api/rules');
            const data = await res.json();
            draftRules = data.content || '';
            app.rulesContent = draftRules;
            rulesStatus = 'idle';
        } catch (_) {
            rulesStatus = 'error';
        }
    }

    async function saveRules() {
        rulesStatus = 'saving';
        try {
            const res = await fetch('/api/rules', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ content: draftRules }),
            });
            if (res.ok) {
                app.rulesContent = draftRules;
                rulesStatus = 'success';
                setTimeout(() => { rulesStatus = 'idle'; }, 2000);
            } else {
                rulesStatus = 'error';
            }
        } catch (_) {
            rulesStatus = 'error';
        }
    }

    async function saveApiKey() {
        const key = draftApiKey.trim();
        if (!key) return;
        apiKeyStatus = 'saving';
        try {
            const res = await fetch('/api/config/key', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ api_key: key }),
            });
            if (res.ok) {
                app.apiKeyConfigured = true;
                draftApiKey = '';
                apiKeyStatus = 'success';
                setTimeout(() => { apiKeyStatus = 'idle'; }, 2000);
            } else {
                apiKeyError = 'Rejected by Server';
                apiKeyStatus = 'error';
            }
        } catch (e: any) {
            apiKeyError = e.message || 'Connection failed';
            apiKeyStatus = 'error';
        }
    }

    // Fetch configuration parameters from backend
    async function fetchConfig() {
        try {
            const res = await fetch(`/api/config?_=${Date.now()}`);
            if (!res.ok) return;
            const config = await res.json();

            app.apiKeyConfigured = config.api_key_configured ?? true;

            if (config.candles) app.globalCandlesConfig = config.candles;
            if (config.indicators) app.globalIndicatorsConfig = config.indicators;

            const pairConfigs = config.pairs || {};
            const symbols: string[] = config.symbols || ['Hyperliquid:BTC'];

            for (const item of symbols) {
                const parts = item.split(':');
                const exchange = parts[0] || 'Hyperliquid';
                const symbol = parts[1] || 'BTC';

                app.initPair(symbol, exchange);

                const pairKey = `${exchange}-${symbol}`;
                const specific = pairConfigs[pairKey];
                const targetState = app.pairsMap[pairKey];

                if (specific && targetState) {
                    targetState.barDurationSec = specific.candles.duration_seconds;
                    targetState.emaFastVal = specific.indicators.ema_fast;
                    targetState.emaMediumVal = specific.indicators.ema_medium;
                    targetState.emaSlowVal = specific.indicators.ema_slow;
                    targetState.emaLongVal = specific.indicators.ema_long;
                    targetState.rsiPeriodVal = specific.indicators.rsi_period;
                    targetState.macdFastVal = specific.indicators.macd_fast;
                    targetState.macdSlowVal = specific.indicators.macd_slow;
                    targetState.macdSignalVal = specific.indicators.macd_signal;
                    targetState.adxPeriodVal = specific.indicators.adx_period;
                    targetState.atrPeriodVal = specific.indicators.atr_period;
                    targetState.squeezePeriodVal = specific.indicators.squeeze_period;

                    if (specific.automation) {
                        targetState.automationEnabled = specific.automation.enabled ?? false;
                        const autoSec = specific.automation.interval_seconds ?? 900;
                        if (autoSec % 3600 === 0) {
                            targetState.automationIntervalValue = autoSec / 3600;
                            targetState.automationIntervalUnit = 'hours';
                        } else if (autoSec % 60 === 0) {
                            targetState.automationIntervalValue = autoSec / 60;
                            targetState.automationIntervalUnit = 'minutes';
                        } else {
                            targetState.automationIntervalValue = autoSec;
                            targetState.automationIntervalUnit = 'seconds';
                        }
                        targetState.nextEvaluationIn = targetState.automationEnabled ? formatIntervalRemaining(autoSec) : '--';
                    }
                }
            }
            if (symbols.length > 0) {
                const parts = symbols[0].split(':');
                app.activeTab = `${parts[0] || 'Hyperliquid'}-${parts[1] || 'BTC'}`;
            }
            configReady = true;
            connectWebsocket();
        } catch (e) {
            console.error('Failed to fetch config from server:', e);
            configReady = true;
        }
    }

    async function fetchAssistantHistory() {
        try {
            const res = await fetch('/api/assistant-records');
            const data = await res.json();
            app.assistantHistory = data.records || [];
            app.historyLatestClose = data.latest_close || '0';
        } catch (_) {}
    }

    let currentWsSymbol: string = '';

    // Establish real-time telemetry WebSocket connection
    function connectWebsocket() {
        if (ws) {
            if (ws.readyState !== WebSocket.CLOSED && ws.readyState !== WebSocket.CLOSING) {
                ws.onclose = null;
                ws.onerror = null;
                ws.close();
            }
            ws = null;
        }

        const symbol = app.activeTab;
        if (!symbol) return;

        const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        const wsUrl = `${protocol}//${window.location.host}/ws?symbol=${encodeURIComponent(symbol)}`;

        currentWsSymbol = symbol;
        ws = new WebSocket(wsUrl);

        ws.onopen = () => {
            app.isConnected = true;
        };

        ws.onmessage = (event) => {
            try {
                const snapshot = JSON.parse(event.data);

                const exchangeStr = snapshot.exchange || 'Hyperliquid';
                const symbolStr = snapshot.symbol || 'BTC';
                const tabKey = `${exchangeStr}-${symbolStr}`;

                if (app.pairsMap[tabKey]) {
                    const pair = app.pairsMap[tabKey];
                    pair.latestSnapshot = snapshot;
                    if (snapshot.mid_price) pair.priceText = parseFloat(snapshot.mid_price).toFixed(2);
                    if (snapshot.vwap) pair.vwapText = parseFloat(snapshot.vwap).toFixed(2);
                    if (snapshot.ema_fast) pair.emaFastText = parseFloat(snapshot.ema_fast).toFixed(2);
                    if (snapshot.ema_medium) pair.emaMediumText = parseFloat(snapshot.ema_medium).toFixed(2);
                    if (snapshot.ema_slow) pair.emaSlowText = parseFloat(snapshot.ema_slow).toFixed(2);
                    if (snapshot.ema_long) pair.emaLongText = parseFloat(snapshot.ema_long).toFixed(2);
                    if (snapshot.adx_14) pair.adxText = parseFloat(snapshot.adx_14).toFixed(2);
                    if (snapshot.adx_plus) pair.adxPlusText = parseFloat(snapshot.adx_plus).toFixed(2);
                    if (snapshot.adx_minus) pair.adxMinusText = parseFloat(snapshot.adx_minus).toFixed(2);
                    if (snapshot.atr_14) pair.atrText = parseFloat(snapshot.atr_14).toFixed(2);
                    if (snapshot.rsi_14) pair.rsiText = parseFloat(snapshot.rsi_14).toFixed(2);
                    if (snapshot.macd_line) pair.macdLineText = parseFloat(snapshot.macd_line).toFixed(4);
                    if (snapshot.macd_signal) pair.macdSigText = parseFloat(snapshot.macd_signal).toFixed(4);
                    if (snapshot.macd_hist) pair.macdHistText = parseFloat(snapshot.macd_hist).toFixed(4);
                    if (snapshot.squeeze_momentum) pair.sqzValText = parseFloat(snapshot.squeeze_momentum).toFixed(4);
                    pair.isSqueezeOn = snapshot.squeeze_on ?? false;
                    pair.sqzStatusText = pair.isSqueezeOn ? 'SQUEEZE ON' : 'SQUEEZE OFF';
                    if (snapshot.volume) pair.volText = parseFloat(snapshot.volume).toFixed(2);
                    if (snapshot.average_volume) pair.avgVolText = parseFloat(snapshot.average_volume).toFixed(2);

                    const pos = pair.activePaperPosition as any;
                    if (pos && snapshot.mid_price) {
                        const currentPrice = parseFloat(snapshot.mid_price);
                        const entryPrice = pos.entry_price ?? 0;
                        const size = pos.size ?? 0;
                        const allocated = pos.allocated_usd ?? 0;
                        if (pos.direction === 'LONG') {
                            pair.paperUnrealizedPnl = (currentPrice - entryPrice) * size;
                        } else {
                            pair.paperUnrealizedPnl = (entryPrice - currentPrice) * size;
                        }
                        pair.paperUnrealizedRoi = allocated > 0 ? (pair.paperUnrealizedPnl / allocated) * 100 : 0;
                        pair.paperTotalAccountValue = pair.paperCashBalance + allocated + pair.paperUnrealizedPnl;
                    }
                }
            } catch (err) {
                console.error("Error parsing market snapshot JSON:", err);
            }
        };

        ws.onclose = () => {
            app.isConnected = false;
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
    });

    onDestroy(() => {
        if (ws) {
            ws.onclose = null;
            ws.onerror = null;
            ws.close();
        }
    });

    $effect(() => {
        const tab = app.activeTab;
        if (configReady && tab && tab !== currentWsSymbol) {
            connectWebsocket();
        }
    });

    // Auto-sync setting draft state when moving to setting tab views
    $effect(() => {
        const key = app.activeTab;
        const pair = app.pairsMap[key];
        if (pair && pair.currentView === 'settings' && activeSettingsPairKey !== key) {
            syncSettingsDraft(key, pair);
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
            const historyRes = await fetch(`/api/history?symbol=${encodeURIComponent(app.activeTab)}`);
            const historyData = await historyRes.json();
            const prices: number[] = (historyData.prices || []).map(Number);

            const snap = app.latestSnapshot || {};

            const body = {
                symbol: app.activeTab,
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
    {#if !app.apiKeyConfigured}
        <div class="api-key-banner">
            ⚠️ DeepSeek AI API Key is not configured. Falling back to local heuristic mode.
        </div>
    {/if}

    <TabHeader />

    <div class="workspace-viewport">
        {#each Object.keys(app.pairsMap) as tabKey (tabKey)}
            {@const pair = app.pairsMap[tabKey]}
            <div class="workspace-window" class:hidden-pane={tabKey !== app.activeTab}>

                <!-- Secondary navigation bar within each pair's self-contained layout -->
                <div class="workspace-sub-header">
                    <div class="sub-tabs-container">
                        <button
                            class="sub-tab-btn"
                            class:sub-tab-active={pair.currentView === 'terminal'}
                            onclick={() => pair.currentView = 'terminal'}
                        >
                            📈 Live Terminal
                        </button>
                        <button
                            class="sub-tab-btn"
                            class:sub-tab-active={pair.currentView === 'positions'}
                            onclick={() => { pair.currentView = 'positions'; app.fetchPaperStatus(); }}
                        >
                            💰 Positions
                        </button>
                        <button
                            class="sub-tab-btn"
                            class:sub-tab-active={pair.currentView === 'performance'}
                            onclick={() => pair.currentView = 'performance'}
                        >
                            📊 Performance Metrics
                        </button>
                        <button
                            class="sub-tab-btn"
                            class:sub-tab-active={pair.currentView === 'settings'}
                            onclick={() => { pair.currentView = 'settings'; syncSettingsDraft(tabKey, pair); }}
                        >
                            ⚙️ Workspace Settings
                        </button>
                    </div>
                    <div class="time-badge">
                        {pair.symbol}USD — {pair.barDurationSec >= 3600 ? (pair.barDurationSec / 3600) + 'h' : pair.barDurationSec >= 60 ? (pair.barDurationSec / 60) + 'm' : pair.barDurationSec + 's'}
                    </div>
                </div>

                <!-- 1. Live Terminal Inner View -->
                {#if pair.currentView === 'terminal'}
                    <div class="main-layout animate-fade">
                        <div class="dashboard-stack">
                            <div class="panel-box pane-price">
                                <div class="absolute-label font-sans">
                                    <span class="price-header">Price: <span>{pair.priceText}</span></span>
                                    {#if pair.showVwap}
                                        <span class="text-orange-400 font-medium">VWAP: <span>{pair.vwapText}</span></span>
                                    {/if}
                                    {#if pair.showEmas}
                                        <span class="text-blue-400 font-medium">{app.emaFastLabel}: <span>{pair.emaFastText}</span></span>
                                        <span class="text-amber-500 font-medium">{app.emaMediumLabel}: <span>{pair.emaMediumText}</span></span>
                                        <span class="text-rose-500 font-medium">{app.emaSlowLabel}: <span>{pair.emaSlowText}</span></span>
                                        <span class="text-purple-400 font-medium">{app.emaLongLabel}: <span>{pair.emaLongText}</span></span>
                                    {/if}
                                </div>
                                {#key `${tabKey}-${pair.barDurationSec}-${pair.emaFastVal}-${pair.emaMediumVal}-${pair.emaSlowVal}-${pair.emaLongVal}`}
                                    <PriceChart pairKey={tabKey} />
                                {/key}
                            </div>

                            <div class="panel-box pane-vol" class:hidden-pane={!pair.showVolume}>
                                <div class="absolute-label font-sans label-text-xs">
                                    <span class="text-teal-400 font-bold">Volume: <span>{pair.volText}</span></span>
                                </div>
                                {#key `${tabKey}-${pair.barDurationSec}`}
                                    <VolumeChart pairKey={tabKey} />
                                {/key}
                            </div>

                            <div class="panel-box pane-adx" class:hidden-pane={!pair.showAdx}>
                                <div class="absolute-label font-sans label-text-xs">
                                    <span class="text-yellow-400 font-bold">ADX: <span>{pair.adxText}</span></span>
                                    <span class="text-emerald-400 font-medium">+DI: <span>{pair.adxPlusText}</span></span>
                                    <span class="text-red-500 font-medium">-DI: <span>{pair.adxMinusText}</span></span>
                                </div>
                                {#key `${tabKey}-${pair.barDurationSec}-${pair.adxPeriodVal}`}
                                    <AdxChart pairKey={tabKey} />
                                {/key}
                            </div>

                            <div class="panel-box pane-atr" class:hidden-pane={!pair.showAtr}>
                                <div class="absolute-label font-sans label-text-xs">
                                    <span class="text-purple-400 font-bold">{app.atrLabel}: <span>{pair.atrText}</span></span>
                                </div>
                                {#key `${tabKey}-${pair.barDurationSec}-${pair.atrPeriodVal}`}
                                    <AtrChart pairKey={tabKey} />
                                {/key}
                            </div>

                            <div class="panel-box pane-rsi" class:hidden-pane={!pair.showRsi}>
                                <div class="absolute-label font-sans label-text-xs">
                                    <span class="text-purple-400">{app.rsiLabel}: <span>{pair.rsiText}</span></span>
                                </div>
                                {#key `${tabKey}-${pair.barDurationSec}-${pair.rsiPeriodVal}`}
                                    <RsiChart pairKey={tabKey} />
                                {/key}
                            </div>

                            <div class="panel-box pane-macd" class:hidden-pane={!pair.showMacd}>
                                <div class="absolute-label font-sans label-text-xs">
                                    <span class="text-slate-300 font-bold">{app.macdLabel}</span>
                                    <span class="text-blue-400">Line: <span>{pair.macdLineText}</span></span>
                                    <span class="text-amber-500">Signal: <span>{pair.macdSigText}</span></span>
                                    <span class="text-teal-400">Hist: <span>{pair.macdHistText}</span></span>
                                </div>
                                {#key `${tabKey}-${pair.barDurationSec}-${pair.macdFastVal}-${pair.macdSlowVal}-${pair.macdSignalVal}`}
                                    <MacdChart pairKey={tabKey} />
                                {/key}
                            </div>

                            <div class="panel-box pane-squeeze" class:hidden-pane={!pair.showSqueeze}>
                                <div class="absolute-label font-sans label-text-xs">
                                    <span class="text-slate-300 font-bold">Squeeze Momentum (LazyBear)</span>
                                    <span class="text-emerald-400">Value: <span>{pair.sqzValText}</span></span>
                                    <span class={pair.isSqueezeOn ? 'text-red-500 font-bold' : 'text-emerald-500 font-bold'}>Status: {pair.sqzStatusText}</span>
                                </div>
                                {#key `${tabKey}-${pair.barDurationSec}-${pair.squeezePeriodVal}`}
                                    <SqueezeChart pairKey={tabKey} />
                                {/key}
                            </div>
                        </div>

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
                                        <div class="entry-price-input" style="margin-top: 8px;">
                                            <label for="stopLoss">Stop Loss ($):</label>
                                            <input id="stopLoss" type="number" step="any"
                                                   bind:value={app.stopLossVal} placeholder="0.00" />
                                            <small style="font-size: 9px; color: #64748b; margin-top: 2px; display: block;">
                                                Left blank? Defaults to 1% risk distance.
                                            </small>
                                        </div>
                                    {/if}

                                    <button class="analyze-btn" onclick={requestAnalysis} disabled={app.assistantLoading}>
                                        {app.assistantLoading ? 'Analyzing Market...' : 'Request AI Assistant Analysis'}
                                    </button>

                                    {#if app.assistantLoading}
                                        <div class="loading-indicator">
                                            <span class="dot pulse-blue"></span>
                                            <span class="status-text">
                                                {app.analysisPhase === 'phase1' ? 'Phase 1: Running indicator agents...' : 'Phase 2: Synthesizing master report...'}
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
                                                    <span class="ap-status">{agent.status === 'complete' ? '✓' : agent.status === 'failed' ? '✗' : '···'}</span>
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
                                                <span class="consensus-badge" class:badge-up={pt.general_trend === 'UPWARD'} class:badge-down={pt.general_trend === 'DOWNWARD'} class:badge-side={pt.general_trend === 'SIDEWAYS'}>
                                                    {pt.indicator_synthesis.summary_count}
                                                </span>
                                            </div>

                                            <div class="result-block reveal" style="animation-delay: 150ms">
                                                <h4 class="result-stage-title">Phase 2 — Trend & Structure</h4>
                                                <span class="result-badge" class:badge-up={pt.general_trend === 'UPWARD'} class:badge-down={pt.general_trend === 'DOWNWARD'} class:badge-side={pt.general_trend === 'SIDEWAYS'}>
                                                    {pt.general_trend}
                                                </span>
                                                <p class="result-reasoning">{pt.indicator_synthesis.evaluation.substring(0, 120)}...</p>
                                            </div>

                                            <div class="result-block result-action reveal" style="animation-delay: 300ms">
                                                <h4 class="result-stage-title">3. Position Recommendation</h4>
                                                <span class="action-call" class:action-green={pt.position_recommendation.action === 'Hold' || pt.position_recommendation.action === 'Open Long'} class:action-red={pt.position_recommendation.action === 'Close'} class:action-amber={pt.position_recommendation.action === 'Wait' || pt.position_recommendation.action === 'Open Short'}>
                                                    {pt.position_recommendation.action}
                                                </span>
                                                <p class="result-reasoning">{pt.position_recommendation.rationale.substring(0, 150)}...</p>
                                            </div>
                                            <div class="click-hint">Click for full analysis & chat</div>
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
                                        <p class="signals-placeholder">No history recorded yet.</p>
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
                                                            <td class="col-action" class:action-text-green={rec.recommended_action === 'Hold' || rec.recommended_action === 'Open Long'} class:action-text-red={rec.recommended_action === 'Close'} class:action-text-amber={rec.recommended_action === 'Wait' || rec.recommended_action === 'Open Short'}>
                                                                {rec.recommended_action.substring(0, 4)}
                                                            </td>
                                                            <td class="col-price">{rec.price_at_analysis.substring(0, 8)}</td>
                                                            <td class="col-delta" class:delta-positive={delta > 0} class:delta-negative={delta < 0}>{delta.toFixed(2)}%</td>
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

                <!-- 1.5 Positions Inner View -->
                {:else if pair.currentView === 'positions'}
                    <div class="workspace-inner-content animate-fade">
                        <div class="paper-layout">
                            <div class="paper-positions-col">
                                <h3 class="card-title" style="margin-top: 0;">Active Paper Position</h3>
                                {#if pair.activePaperPosition}
                                    {@const pos = pair.activePaperPosition as any}
                                    <div class="paper-position-card" class:direction-long={pos.direction === 'LONG'} class:direction-short={pos.direction === 'SHORT'}>
                                        <div class="pp-header">
                                            <span class="pp-direction">{pos.direction}</span>
                                            <span class="pp-symbol">{pos.symbol}</span>
                                        </div>
                                        <div class="pp-details">
                                            <div class="pp-row"><span>Entry Price:</span><span>${(pos.entry_price ?? 0).toFixed(2)}</span></div>
                                            <div class="pp-row"><span>Size:</span><span>{(pos.size ?? 0).toFixed(4)} units</span></div>
                                            <div class="pp-row"><span>Allocated:</span><span>${(pos.allocated_usd ?? 0).toFixed(2)}</span></div>
                                        </div>
                                        <div class="pp-pnl-section">
                                            <div class="pp-row"><span>Unrealized P&L:</span>
                                                <span class:pnl-positive={pair.paperUnrealizedPnl >= 0} class:pnl-negative={pair.paperUnrealizedPnl < 0}>
                                                    {pair.paperUnrealizedPnl >= 0 ? '+' : ''}${pair.paperUnrealizedPnl.toFixed(2)}
                                                </span>
                                            </div>
                                            <div class="pp-row"><span>ROI:</span>
                                                <span class:pnl-positive={pair.paperUnrealizedRoi >= 0} class:pnl-negative={pair.paperUnrealizedRoi < 0}>
                                                    {pair.paperUnrealizedRoi.toFixed(2)}%
                                                </span>
                                            </div>
                                        </div>
                                        <button class="paper-close-btn" onclick={() => app.closePaperPosition()}
                                                disabled={pair.paperLoading}>
                                            Close Position (Market)
                                        </button>
                                    </div>
                                {:else}
                                    <div class="paper-empty-state">
                                        <p>No active paper position.</p>
                                        <div class="paper-action-btns">
                                            <button class="paper-open-btn direction-long" onclick={() => app.openPaperPosition('LONG')}
                                                    disabled={pair.paperLoading}>
                                                Open Long
                                            </button>
                                            <button class="paper-open-btn direction-short" onclick={() => app.openPaperPosition('SHORT')}
                                                    disabled={pair.paperLoading}>
                                                Open Short
                                            </button>
                                        </div>
                                    </div>
                                {/if}
                            </div>

                            <div class="paper-ledger-col">
                                <h3 class="card-title" style="margin-top: 0;">Account Ledger</h3>
                                <div class="paper-ledger-card">
                                    <div class="ledger-row"><span>Total Balance:</span><span class="mono">${pair.paperTotalAccountValue.toFixed(2)}</span></div>
                                    <div class="ledger-row"><span>Available Cash:</span><span class="mono">${pair.paperCashBalance.toFixed(2)}</span></div>
                                    <div class="ledger-row"><span>Margin Used:</span><span class="mono">
                                        ${pair.paperMarginUsed.toFixed(2)} ({pair.paperAllocationPct}%)
                                    </span></div>
                                    <div class="ledger-divider"></div>
                                    <div class="ledger-row"><span>Trade Capacity:</span></div>
                                    <div class="capacity-bar-container">
                                        <div class="capacity-bar-track">
                                            <div class="capacity-bar-fill" style="width: {pair.paperMaxTrades > 0 ? (pair.paperActiveTrades / pair.paperMaxTrades * 100) : 0}%"></div>
                                        </div>
                                        <span class="capacity-text">{pair.paperActiveTrades} / {pair.paperMaxTrades} Active</span>
                                    </div>
                                    <div class="ledger-row" style="margin-top: 8px;">
                                        <span>Available Trades:</span><span class="mono">{pair.paperAvailableTrades}</span>
                                    </div>
                                </div>
                            </div>
                        </div>
                    </div>

                <!-- 2. Performance Metrics Inner View -->
                {:else if pair.currentView === 'performance'}
                    <div class="workspace-inner-content">
                        <PerformanceDashboard />
                    </div>

                <!-- 3. Local Workspace Settings Tab View -->
                {:else if pair.currentView === 'settings'}
                    <div class="settings-workspace-tab animate-fade">
                        <div class="settings-grid">

                            <!-- Visual Layout Column -->
                            <div class="settings-col">
                                <h3 class="card-title">Visual Overlays</h3>
                                <div class="setting-group-box">
                                    <span class="selectors-label">Chart Display Items</span>
                                    <div class="toggle-grid">
                                        <button class="selector-btn" class:active={draftShowEmas} onclick={() => draftShowEmas = !draftShowEmas}>EMAs</button>
                                        <button class="selector-btn" class:active={draftShowBb} onclick={() => draftShowBb = !draftShowBb}>Bollinger</button>
                                        <button class="selector-btn" class:active={draftShowVwap} onclick={() => draftShowVwap = !draftShowVwap}>VWAP</button>
                                    </div>
                                </div>

                                <div class="setting-group-box" style="margin-top: 12px;">
                                    <span class="selectors-label">Indicator Panels</span>
                                    <div class="toggle-grid">
                                        <button class="selector-btn" class:active={draftShowVolume} onclick={() => draftShowVolume = !draftShowVolume}>Volume</button>
                                        <button class="selector-btn" class:active={draftShowAdx} onclick={() => draftShowAdx = !draftShowAdx}>ADX</button>
                                        <button class="selector-btn" class:active={draftShowAtr} onclick={() => draftShowAtr = !draftShowAtr}>ATR</button>
                                        <button class="selector-btn" class:active={draftShowRsi} onclick={() => draftShowRsi = !draftShowRsi}>RSI</button>
                                        <button class="selector-btn" class:active={draftShowMacd} onclick={() => draftShowMacd = !draftShowMacd}>MACD</button>
                                        <button class="selector-btn" class:active={draftShowSqueeze} onclick={() => draftShowSqueeze = !draftShowSqueeze}>Squeeze</button>
                                    </div>
                                </div>

                                <div class="setting-group-box" style="margin-top: 12px;">
                                    <span class="selectors-label">Automated AI Evaluation</span>
                                    <div class="toggle-row">
                                        <span class="toggle-label">Status</span>
                                        <button class="selector-btn" class:active={draftAutomationEnabled}
                                                onclick={() => draftAutomationEnabled = !draftAutomationEnabled}>
                                            {draftAutomationEnabled ? 'ON' : 'OFF'}
                                        </button>
                                    </div>
                                    {#if draftAutomationEnabled}
                                        <div class="input-row" style="margin-top: 8px;">
                                            <label for="autoInterval">Interval:</label>
                                            <div class="tf-split-group">
                                                <input id="autoInterval" type="number" bind:value={draftAutomationIntervalValue} min="1" class="tf-number-input" />
                                                <select bind:value={draftAutomationIntervalUnit} class="tf-unit-select">
                                                    <option value="seconds">Seconds</option>
                                                    <option value="minutes">Minutes</option>
                                                    <option value="hours">Hours</option>
                                                </select>
                                            </div>
                                        </div>
                                        <div class="live-counter" style="margin-top: 8px; font-size: 10px; color: #3b82f6;">
                                            Next evaluation in: {pair.nextEvaluationIn}
                                        </div>
                                    {/if}
                                </div>
                            </div>

                            <!-- Indicator Parameters Column -->
                            <div class="settings-col">
                                <h3 class="card-title">Technical Parameters</h3>
                                <div class="parameter-inputs-scroll font-mono">
                                    <div class="input-row">
                                        <label for="exchange">Exchange Source:</label>
                                        <select id="exchange" bind:value={draftExchange} class="tf-unit-select">
                                            <option value="Hyperliquid">Hyperliquid</option>
                                        </select>
                                    </div>
                                    <div class="input-row">
                                        <label for="symbol">Market Pair:</label>
                                        <input id="symbol" type="text" bind:value={draftSymbol} />
                                    </div>
                                    <div class="input-row">
                                        <label for="tf">Timeframe:</label>
                                        <div class="tf-split-group">
                                            <input id="tf" type="number" bind:value={draftDurationValue} min="1" class="tf-number-input" />
                                            <select bind:value={draftDurationUnit} class="tf-unit-select">
                                                <option value="seconds">Seconds</option>
                                                <option value="minutes">Minutes</option>
                                                <option value="hours">Hours</option>
                                            </select>
                                        </div>
                                    </div>
                                    <hr class="section-divider" />
                                    <div class="input-row"><label for="emaf">EMA Fast:</label><input id="emaf" type="number" bind:value={draftEmaFast} /></div>
                                    <div class="input-row"><label for="emam">EMA Med:</label><input id="emam" type="number" bind:value={draftEmaMedium} /></div>
                                    <div class="input-row"><label for="emas">EMA Slow:</label><input id="emas" type="number" bind:value={draftEmaSlow} /></div>
                                    <div class="input-row"><label for="emal">EMA Long:</label><input id="emal" type="number" bind:value={draftEmaLong} /></div>
                                    <div class="input-row"><label for="rsi">RSI Window:</label><input id="rsi" type="number" bind:value={draftRsiPeriod} /></div>
                                    <div class="input-row"><label for="macdf">MACD Fast:</label><input id="macdf" type="number" bind:value={draftMacdFast} /></div>
                                    <div class="input-row"><label for="macds">MACD Slow:</label><input id="macds" type="number" bind:value={draftMacdSlow} /></div>
                                    <div class="input-row"><label for="macdsig">MACD Signal:</label><input id="macdsig" type="number" bind:value={draftMacdSignal} /></div>
                                    <div class="input-row"><label for="adx">ADX Period:</label><input id="adx" type="number" bind:value={draftAdxPeriod} /></div>
                                    <div class="input-row"><label for="atr">ATR Period:</label><input id="atr" type="number" bind:value={draftAtrPeriod} /></div>
                                    <div class="input-row"><label for="sqz">Squeeze Wave:</label><input id="sqz" type="number" bind:value={draftSqueezePeriod} /></div>
                                </div>

                                <div class="settings-footer-row" style="margin-top: 16px;">
                                    <button class="apply-workspace-btn" onclick={() => applySettings(tabKey, pair)}>
                                        Apply Workspace Configuration
                                    </button>
                                </div>
                            </div>

                            <!-- Backend Secrets & Prompts Guide Column -->
                            <div class="settings-col">
                                <h3 class="card-title">Backend & AI Prompts</h3>

                                <!-- API Key Config -->
                                <div class="setting-group-box">
                                    <span class="selectors-label">DeepSeek API Secret Key</span>
                                    <div class="key-input-row">
                                        <input type="password" class="key-field" placeholder="sk-..." bind:value={draftApiKey} />
                                        <button class="key-save-btn" disabled={apiKeyStatus === 'saving'} onclick={saveApiKey}>
                                            {apiKeyStatus === 'saving' ? '...' : 'Save'}
                                        </button>
                                    </div>
                                    {#if apiKeyStatus === 'success'}
                                        <div class="status-msg success-msg">Key saved.</div>
                                    {/if}
                                </div>

                                <!-- Rules Editor -->
                                <div class="setting-group-box" style="margin-top: 12px;">
                                    <span class="selectors-label">Technical rules guide handbook (Markdown)</span>
                                    <textarea class="rules-editor" rows="6" bind:value={draftRules}></textarea>
                                    <div style="display: flex; justify-content: space-between; align-items: center; margin-top: 4px;">
                                        <button class="key-save-btn" onclick={fetchRules}>Fetch</button>
                                        <button class="key-save-btn" disabled={rulesStatus === 'saving'} onclick={saveRules}>
                                            {rulesStatus === 'saving' ? '...' : 'Update Rules'}
                                        </button>
                                    </div>
                                </div>
                            </div>

                            <!-- Paper Trading Rules Column -->
                            <div class="settings-col">
                                <h3 class="card-title">Paper Trading Rules</h3>

                                <div class="setting-group-box">
                                    <span class="selectors-label">Account Configuration</span>
                                    <div class="input-row" style="margin-top: 4px;">
                                        <label for="paperUSD">Initial USD:</label>
                                        <input id="paperUSD" type="number" bind:value={pair.paperInitialUSD} min="100" step="100" />
                                    </div>
                                    <div class="input-row" style="margin-top: 8px;">
                                        <label for="paperAlloc">Allocation %:</label>
                                        <input id="paperAlloc" type="number" bind:value={pair.paperAllocationPct} min="1" max="100" step="1" />
                                    </div>
                                    <button class="key-save-btn" style="margin-top: 8px; width: 100%;"
                                            onclick={() => app.savePaperConfig(
                                                pair.paperInitialUSD,
                                                pair.paperAllocationPct,
                                                pair.paperAutoExecute
                                            )}>
                                        Save Paper Config
                                    </button>
                                </div>

                                <div class="setting-group-box" style="margin-top: 12px;">
                                    <span class="selectors-label">Auto-Execution</span>
                                    <div class="toggle-row">
                                        <span class="toggle-label">Auto-Place Orders</span>
                                        <button class="selector-btn" class:active={pair.paperAutoExecute}
                                                onclick={() => {
                                                    pair.paperAutoExecute = !pair.paperAutoExecute;
                                                    app.savePaperConfig(
                                                        pair.paperInitialUSD,
                                                        pair.paperAllocationPct,
                                                        pair.paperAutoExecute
                                                    );
                                                }}>
                                            {pair.paperAutoExecute ? 'ON' : 'OFF'}
                                        </button>
                                    </div>
                                    <p style="font-size: 9px; color: #64748b; margin: 6px 0 0 0;">
                                        When enabled, automated AI signals will automatically place paper orders.
                                    </p>
                                </div>

                                <div class="setting-group-box" style="margin-top: 12px;">
                                    <button class="paper-reset-btn" onclick={() => {
                                        if (confirm('Reset paper account? This will close any active position and restore initial balance.')) {
                                            app.resetPaperAccount();
                                        }
                                    }}>
                                        Reset Account Balance
                                    </button>
                                </div>
                            </div>

                        </div>
                    </div>
                {/if}

            </div>
        {/each}
    </div>

    <!-- Modals -->
    {#if app.isAssistantModalOpen && app.multiAgentResponse}
        {@const resp = app.multiAgentResponse!}
        {@const pt = resp.phase_two}
        {@const indicators = resp.phase_one}
        {@const snap = app.latestSnapshot || {}}
        {@const price = snap.mid_price ? parseFloat(String(snap.mid_price)) : null}

        <!-- svelte-ignore a11y_interactive_supports_focus a11y_click_events_have_key_events -->
        <div class="modal-backdrop" onclick={closeAssistantModal} role="dialog">
            <div class="modal-window" onclick={(e) => e.stopPropagation()}>
                <div class="modal-header">
                    <h2 class="modal-title">AI Copilot Intelligence Hub — {app.activeSymbol}</h2>
                    <button class="modal-close-btn" onclick={closeAssistantModal}>&#10005;</button>
                </div>

                <div class="modal-body">
                    <div class="modal-left">
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
                                        <span class="sr-level sr-none">None</span>
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
                                        <span class="sr-level sr-none">None</span>
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
                                    <span class="poc-signal">{ind.signal}</span>
                                    <p class="poc-reason">{ind.reason}</p>
                                </div>
                            {/each}
                        </div>
                    </div>

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
                            <input type="text" class="chat-input" placeholder="Ask details..." bind:value={app.chatInputText} disabled={app.isChatLoading} onkeydown={(e) => { if (e.key === 'Enter') sendChatMessage() }} />
                            <button class="chat-send-btn" onclick={sendChatMessage} disabled={app.isChatLoading || !app.chatInputText.trim()}>Send</button>
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
        font-family: ui-sans-serif, system-ui, sans-serif;
        min-height: 100vh;
        display: flex;
        flex-direction: column;
    }
    .api-key-banner {
        background: rgba(127, 29, 29, 0.5);
        border-bottom: 1px solid #ef4444;
        color: #fca5a5;
        font-size: 11px;
        padding: 6px 16px;
        text-align: center;
        font-family: 'Courier New', monospace;
    }
    .workspace-viewport {
        flex: 1;
        display: flex;
        flex-direction: column;
    }
    .workspace-window {
        flex: 1;
        display: flex;
        flex-direction: column;
    }
    .workspace-sub-header {
        background-color: #0f111a;
        border-bottom: 1px solid #1e293b;
        padding: 8px 16px;
        display: flex;
        align-items: center;
        justify-content: space-between;
    }
    .sub-tabs-container {
        display: flex;
        gap: 6px;
    }
    .sub-tab-btn {
        background: transparent;
        border: 1px solid transparent;
        color: #64748b;
        font-size: 11px;
        font-weight: 700;
        cursor: pointer;
        padding: 4px 10px;
        border-radius: 4px;
        transition: all 0.2s;
    }
    .sub-tab-btn:hover {
        color: #cbd5e1;
        background-color: rgba(255, 255, 255, 0.02);
    }
    .sub-tab-active {
        background: #1a2030;
        border-color: #2a3040;
        color: #f8fafc;
    }
    .time-badge {
        font-size: 10px;
        font-weight: 600;
        color: #64748b;
        background: #1a2030;
        border: 1px solid #2a3040;
        border-radius: 4px;
        padding: 3px 10px;
        font-family: 'Courier New', monospace;
    }
    .main-layout {
        display: flex;
        max-width: 1800px;
        margin: 0 auto;
        padding: 16px;
        gap: 16px;
        width: 100%;
        box-sizing: border-box;
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
        box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1);
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
        0% { transform: scale(0.95); box-shadow: 0 0 0 0 rgba(59, 130, 246, 0.7); }
        70% { transform: scale(1); box-shadow: 0 0 0 4px rgba(59, 130, 246, 0); }
        100% { transform: scale(0.95); box-shadow: 0 0 0 0 rgba(59, 130, 246, 0); }
    }

    @media (max-width: 1024px) {
        .main-layout { flex-direction: column; }
        .sidebar-panel { width: 100%; }
    }

    .panel-box {
        position: relative;
        background-color: #131722;
        border: 1px solid #2a2e39;
        border-radius: 8px;
        box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1);
        overflow: hidden;
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
        transition: opacity 0.2s;
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
    .result-block.reveal { animation: fadeInUp 0.4s ease forwards; }
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
    .history-table tbody tr:hover { background-color: #1a1f2e; }
    .col-time { color: #64748b; width: 54px; }
    .col-action { font-weight: 700; }
    .col-price { font-family: ui-monospace, monospace; text-align: right; }
    .col-delta { font-family: ui-monospace, monospace; text-align: right; font-weight: 700; }
    .action-text-green { color: #10b981; }
    .action-text-red { color: #ef4444; }
    .action-text-amber { color: #f59e0b; }
    .delta-positive { color: #10b981; }
    .delta-negative { color: #ef4444; }

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
    @keyframes fadeIn { from { opacity: 0; } to { opacity: 1; } }
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
    }
    .modal-close-btn:hover {
        background: #1e293b;
        color: #f1f5f9;
    }

    .modal-body { display: flex; flex: 1; overflow: hidden; }
    .modal-left { width: 50%; padding: 20px; overflow-y: auto; border-right: 1px solid #1e293b; }
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

    .assistant-summary { margin-top: 16px; }
    .summary-message {
        background: rgba(59, 130, 246, 0.05);
        border: 1px solid rgba(59, 130, 246, 0.15);
        border-radius: 8px;
        padding: 12px;
    }
    .summary-text { font-size: 11px; color: #cbd5e1; line-height: 1.6; margin: 0; white-space: pre-wrap; }

    .modal-right { width: 50%; display: flex; flex-direction: column; padding: 20px; overflow: hidden; }
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

    .chat-thread { flex: 1; overflow-y: auto; display: flex; flex-direction: column; gap: 8px; padding-right: 4px; }
    .chat-thread::-webkit-scrollbar { width: 4px; }
    .chat-thread::-webkit-scrollbar-track { background: transparent; }
    .chat-thread::-webkit-scrollbar-thumb { background: #2a2e39; border-radius: 2px; }

    .chat-bubble { max-width: 85%; padding: 10px 12px; border-radius: 8px; font-size: 11px; line-height: 1.5; }
    .user-bubble { align-self: flex-end; background: #1e40af; border: 1px solid #3b82f6; color: #e2e8f0; }
    .assistant-bubble { align-self: flex-start; background: #0f131c; border: 1px solid #1e293b; color: #cbd5e1; }
    .bubble-role { display: block; font-size: 8px; font-weight: 700; text-transform: uppercase; color: #64748b; }
    .bubble-content { white-space: pre-wrap; word-break: break-word; }

    .typing-bubble { opacity: 0.7; }
    .typing-dots { color: #94a3b8; font-style: italic; }
    .dot-anim { animation: dotPulse 1.4s infinite; }
    .dot-anim:nth-child(1) { animation-delay: 0s; }
    .dot-anim:nth-child(2) { animation-delay: 0.2s; }
    .dot-anim:nth-child(3) { animation-delay: 0.4s; }

    @keyframes dotPulse { 0%, 20% { opacity: 0; } 50% { opacity: 1; } 80%, 100% { opacity: 0; } }

    .chat-input-area { display: flex; gap: 8px; margin-top: 12px; flex-shrink: 0; }
    .chat-input {
        flex: 1; background: #0f131c; border: 1px solid #2a2e39; border-radius: 6px;
        padding: 8px 12px; color: #e2e8f0; font-size: 11px; outline: none;
    }
    .chat-input:focus { border-color: #3b82f6; }
    .chat-input:disabled { opacity: 0.5; }
    .chat-input::placeholder { color: #4c525e; }
    .chat-send-btn {
        background: linear-gradient(135deg, #1e40af, #3b82f6); color: #f1f5f9; border: 1px solid #3b82f6;
        border-radius: 6px; padding: 8px 16px; font-size: 11px; font-weight: 700; text-transform: uppercase;
        cursor: pointer;
    }
    .chat-send-btn:disabled { opacity: 0.5; }

    @media (max-width: 768px) {
        .modal-window { max-width: 100vw; max-height: 100vh; border-radius: 0; }
        .modal-body { flex-direction: column; }
        .modal-left { width: 100%; border-right: none; border-bottom: 1px solid #1e293b; }
        .modal-right { width: 100%; flex: 1; max-height: 50vh; }
    }

    .entry-price-input { display: flex; align-items: center; gap: 8px; margin-bottom: 10px; }
    .entry-price-input label { font-size: 10px; font-weight: 600; color: #64748b; text-transform: uppercase; white-space: nowrap; }
    .entry-price-input input {
        flex: 1; background: #0f131c; border: 1px solid #2a2e39; border-radius: 4px;
        padding: 5px 8px; color: #e2e8f0; font-size: 11px; outline: none; width: 100%;
    }
    .entry-price-input input:focus { border-color: #3b82f6; }

    .agent-progress-list { display: flex; flex-direction: column; gap: 2px; margin-bottom: 8px; }
    .agent-progress-item {
        display: flex; justify-content: space-between; align-items: center;
        padding: 3px 6px; border-radius: 3px; font-size: 9px; background: #0f131c;
    }
    .ap-name { color: #94a3b8; font-weight: 600; }
    .ap-status { font-size: 10px; }
    .ap-complete { background: rgba(16, 185, 129, 0.08); }
    .ap-complete .ap-status { color: #10b981; }
    .ap-failed { background: rgba(239, 68, 68, 0.08); }
    .ap-failed .ap-status { color: #ef4444; }
    .ap-running .ap-status { color: #3b82f6; animation: pulse 1.5s infinite; }

    .consensus-badge { display: inline-block; font-size: 10px; font-weight: 700; padding: 3px 10px; border-radius: 4px; background: rgba(59, 130, 246, 0.1); color: #3b82f6; }

    .master-synthesis { margin-bottom: 16px; }
    .sr-ribbon { display: flex; gap: 2px; margin-bottom: 8px; }
    .sr-block { flex: 1; padding: 8px; border-radius: 6px; text-align: center; }
    .sr-support { background: rgba(16, 185, 129, 0.08); border: 1px solid rgba(16, 185, 129, 0.2); }
    .sr-resistance { background: rgba(239, 68, 68, 0.08); border: 1px solid rgba(239, 68, 68, 0.2); }
    .sr-current { background: rgba(59, 130, 246, 0.08); border: 1px solid rgba(59, 130, 246, 0.25); }
    .sr-label { display: block; font-size: 8px; font-weight: 700; text-transform: uppercase; color: #64748b; margin-bottom: 3px; }
    .sr-level { display: block; font-size: 11px; font-weight: 600; color: #e2e8f0; }
    .sr-support .sr-level { color: #10b981; }
    .sr-resistance .sr-level { color: #ef4444; }
    .sr-current .sr-level { color: #3b82f6; }
    .sr-price-label { font-size: 13px; font-weight: 800; }
    .sr-none { font-size: 9px; color: #4c525e !important; font-style: italic; }
    .sr-structural { font-size: 10px; color: #94a3b8; line-height: 1.4; margin: 0 0 8px 0; }

    .decision-callout { padding: 12px; border-radius: 8px; margin-bottom: 10px; text-align: center; }
    .decision-green { background: rgba(16, 185, 129, 0.08); border: 1px solid rgba(16, 185, 129, 0.25); }
    .decision-red { background: rgba(239, 68, 68, 0.08); border: 1px solid rgba(239, 68, 68, 0.25); }
    .decision-amber { background: rgba(251, 191, 36, 0.08); border: 1px solid rgba(251, 191, 36, 0.25); }
    .decision-action { display: block; font-size: 16px; font-weight: 800; text-transform: uppercase; margin-bottom: 2px; }
    .decision-green .decision-action { color: #10b981; }
    .decision-red .decision-action { color: #ef4444; }
    .decision-amber .decision-action { color: #f59e0b; }
    .decision-trend { display: block; font-size: 10px; font-weight: 600; color: #64748b; text-transform: uppercase; margin-bottom: 6px; }
    .decision-rationale { font-size: 10px; color: #94a3b8; line-height: 1.4; margin: 0; }

    .synthesis-summary { background: #0f131c; border: 1px solid #1e293b; border-radius: 6px; padding: 10px; }
    .synth-count { display: block; font-size: 11px; font-weight: 700; color: #3b82f6; margin-bottom: 4px; }
    .synth-eval { font-size: 10px; color: #94a3b8; line-height: 1.4; margin: 0; }

    .indicator-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 6px; margin-bottom: 16px; }
    .phase-one-card { background: #0f131c; border: 1px solid #1e293b; border-radius: 5px; padding: 8px 10px; }
    .phase-one-card.poc-bullish { border-color: rgba(16, 185, 129, 0.3); background: rgba(16, 185, 129, 0.04); }
    .phase-one-card.poc-bearish { border-color: rgba(239, 68, 68, 0.3); background: rgba(239, 68, 68, 0.04); }
    .phase-one-card.poc-sideways { border-color: rgba(251, 191, 36, 0.3); background: rgba(251, 191, 36, 0.04); }
    .phase-one-card.poc-unavailable { border-color: rgba(100, 116, 139, 0.2); background: rgba(100, 116, 139, 0.03); opacity: 0.6; }
    .poc-name { display: block; font-size: 9px; font-weight: 700; color: #64748b; text-transform: uppercase; margin-bottom: 2px; }
    .poc-signal { display: block; font-size: 10px; font-weight: 700; margin-bottom: 3px; }
    .poc-bullish .poc-signal { color: #10b981; }
    .poc-bearish .poc-signal { color: #ef4444; }
    .poc-sideways .poc-signal { color: #f59e0b; }
    .poc-unavailable .poc-signal { color: #64748b; }
    .poc-reason { font-size: 9px; color: #94a3b8; line-height: 1.3; margin: 0; }

    /* Local Workspace Settings Layout */
    .settings-workspace-tab {
        max-width: 1400px;
        margin: 0 auto;
        padding: 16px;
        width: 100%;
        box-sizing: border-box;
    }
    .settings-grid {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
        gap: 16px;
    }
    .settings-col {
        background-color: #131722;
        border: 1px solid #2a2e39;
        border-radius: 8px;
        padding: 16px;
        display: flex;
        flex-direction: column;
    }
    .card-title {
        font-size: 13px;
        font-weight: 700;
        color: #f1f5f9;
        margin-top: 0;
        margin-bottom: 12px;
        text-transform: uppercase;
        letter-spacing: 0.05em;
        border-bottom: 1px solid #1e293b;
        padding-bottom: 6px;
    }
    .setting-group-box {
        background-color: #0e111a;
        border: 1px solid #1e293b;
        border-radius: 6px;
        padding: 12px;
        display: flex;
        flex-direction: column;
        gap: 8px;
    }
    .selectors-label {
        font-size: 9px;
        font-weight: 700;
        text-transform: uppercase;
        color: #64748b;
    }
    .toggle-grid {
        display: grid;
        grid-template-columns: repeat(3, 1fr);
        gap: 6px;
    }
    .selector-btn {
        background-color: #171b26;
        border: 1px solid #2a2e39;
        color: #8f929d;
        font-size: 9px;
        font-weight: 800;
        padding: 6px 0;
        border-radius: 4px;
        cursor: pointer;
        transition: all 0.2s ease;
        text-transform: uppercase;
        text-align: center;
    }
    .selector-btn:hover { border-color: #4c526e; color: #cbd5e1; }
    .selector-btn.active {
        background-color: rgba(59, 130, 246, 0.12);
        border-color: #3b82f6;
        color: #3b82f6;
        box-shadow: 0 0 8px rgba(59, 130, 246, 0.15);
    }
    .parameter-inputs-scroll {
        background-color: #0e111a;
        border: 1px solid #1e293b;
        border-radius: 6px;
        padding: 12px;
        display: flex;
        flex-direction: column;
        gap: 8px;
        max-height: 280px;
        overflow-y: auto;
    }
    .input-row { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
    .input-row label { font-size: 10px; color: #94a3b8; font-weight: 600; }
    .input-row input {
        background-color: #171b26; border: 1px solid #2a2e39; color: #cbd5e1;
        font-family: ui-monospace, monospace; font-size: 11px; padding: 4px 8px;
        border-radius: 4px; width: 80px; text-align: right; outline: none;
    }
    .input-row input:focus { border-color: #3b82f6; }
    .section-divider { border: 0; border-top: 1px solid #1e293b; margin: 6px 0; }
    .tf-split-group { display: flex; gap: 4px; width: 140px; }
    .tf-number-input { width: 50px !important; }
    .tf-unit-select {
        background-color: #171b26; border: 1px solid #2a2e39; color: #cbd5e1;
        font-size: 10px; font-weight: bold; padding: 2px 4px; border-radius: 4px;
        flex: 1; outline: none; cursor: pointer;
    }
    .apply-workspace-btn {
        width: 100%;
        background: linear-gradient(135deg, #1e40af, #3b82f6);
        border: 1px solid #3b82f6;
        color: #f1f5f9;
        font-size: 11px;
        font-weight: 700;
        text-transform: uppercase;
        padding: 10px 0;
        border-radius: 4px;
        cursor: pointer;
        transition: opacity 0.2s;
    }
    .apply-workspace-btn:hover { background: linear-gradient(135deg, #1e3a8a, #2563eb); }
    .key-input-row { display: flex; gap: 6px; }
    .key-field {
        flex: 1; background-color: #171b26; border: 1px solid #2a2e39;
        color: #f1f5f9; font-family: 'Courier New', monospace; font-size: 11px;
        padding: 6px 8px; border-radius: 4px; outline: none;
    }
    .key-save-btn {
        background-color: #1e40af; border: 1px solid #3b82f6; color: #f1f5f9;
        font-size: 10px; font-weight: 700; padding: 6px 12px; border-radius: 4px;
        cursor: pointer; text-transform: uppercase;
    }
    .status-msg { font-size: 10px; padding: 4px 0; }
    .success-msg { color: #10b981; }
    .rules-editor {
        width: 100%; background-color: #0a0d14; border: 1px solid #2a2e39;
        color: #cbd5e1; font-family: 'Courier New', monospace; font-size: 10px;
        line-height: 1.5; padding: 8px; border-radius: 4px; resize: vertical; outline: none;
    }
    .workspace-inner-content {
        flex: 1;
        overflow-y: auto;
    }
    .toggle-row {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 12px;
    }
    .toggle-label {
        font-size: 10px;
        font-weight: 600;
        color: #94a3b8;
    }
    .live-counter {
        font-family: 'Courier New', monospace;
        font-weight: 700;
    }
    .paper-layout {
        display: grid;
        grid-template-columns: 1fr 320px;
        gap: 16px;
        max-width: 1100px;
        margin: 0 auto;
        width: 100%;
    }
    .paper-positions-col, .paper-ledger-col {
        display: flex;
        flex-direction: column;
        gap: 12px;
    }
    .paper-position-card {
        background: #0f131c;
        border: 1px solid #2a2e39;
        border-radius: 8px;
        padding: 16px;
    }
    .direction-long { border-color: rgba(16, 185, 129, 0.4); }
    .direction-short { border-color: rgba(239, 68, 68, 0.4); }
    .pp-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-bottom: 12px;
    }
    .pp-direction {
        font-size: 13px;
        font-weight: 800;
        text-transform: uppercase;
    }
    .direction-long .pp-direction { color: #10b981; }
    .direction-short .pp-direction { color: #ef4444; }
    .pp-symbol {
        font-size: 11px;
        color: #64748b;
        font-weight: 600;
    }
    .pp-details, .pp-pnl-section {
        display: flex;
        flex-direction: column;
        gap: 6px;
    }
    .pp-pnl-section {
        margin-top: 10px;
        padding-top: 10px;
        border-top: 1px solid #1e293b;
    }
    .pp-row {
        display: flex;
        justify-content: space-between;
        font-size: 11px;
        color: #94a3b8;
    }
    .paper-close-btn {
        width: 100%;
        margin-top: 14px;
        padding: 10px;
        background: rgba(239, 68, 68, 0.15);
        border: 1px solid rgba(239, 68, 68, 0.35);
        color: #ef4444;
        font-size: 11px;
        font-weight: 700;
        border-radius: 6px;
        cursor: pointer;
        text-transform: uppercase;
    }
    .paper-close-btn:hover { background: rgba(239, 68, 68, 0.25); }
    .paper-close-btn:disabled { opacity: 0.5; cursor: not-allowed; }
    .paper-empty-state {
        background: #0f131c;
        border: 1px solid #2a2e39;
        border-radius: 8px;
        padding: 24px;
        text-align: center;
        color: #64748b;
        font-size: 12px;
    }
    .paper-action-btns {
        display: flex;
        gap: 10px;
        margin-top: 14px;
    }
    .paper-open-btn {
        flex: 1;
        padding: 10px;
        border: 1px solid;
        border-radius: 6px;
        font-size: 11px;
        font-weight: 700;
        text-transform: uppercase;
        cursor: pointer;
        background: transparent;
    }
    .paper-open-btn.direction-long {
        color: #10b981;
        border-color: rgba(16, 185, 129, 0.35);
    }
    .paper-open-btn.direction-long:hover { background: rgba(16, 185, 129, 0.1); }
    .paper-open-btn.direction-short {
        color: #ef4444;
        border-color: rgba(239, 68, 68, 0.35);
    }
    .paper-open-btn.direction-short:hover { background: rgba(239, 68, 68, 0.1); }
    .paper-open-btn:disabled { opacity: 0.5; cursor: not-allowed; }
    .paper-ledger-card {
        background: #0f131c;
        border: 1px solid #2a2e39;
        border-radius: 8px;
        padding: 16px;
        display: flex;
        flex-direction: column;
        gap: 8px;
    }
    .ledger-row {
        display: flex;
        justify-content: space-between;
        font-size: 11px;
        color: #94a3b8;
    }
    .ledger-divider {
        border-top: 1px solid #1e293b;
        margin: 2px 0;
    }
    .capacity-bar-container {
        display: flex;
        flex-direction: column;
        gap: 4px;
    }
    .capacity-bar-track {
        height: 6px;
        background: #1a1f2e;
        border-radius: 3px;
        overflow: hidden;
    }
    .capacity-bar-fill {
        height: 100%;
        background: linear-gradient(90deg, #3b82f6, #60a5fa);
        border-radius: 3px;
        transition: width 0.3s ease;
    }
    .capacity-text {
        font-size: 9px;
        color: #64748b;
        text-align: right;
    }
    .pnl-positive { color: #10b981; font-weight: 700; }
    .pnl-negative { color: #ef4444; font-weight: 700; }
    .paper-reset-btn {
        width: 100%;
        padding: 8px;
        background: rgba(239, 68, 68, 0.08);
        border: 1px solid rgba(239, 68, 68, 0.3);
        color: #ef4444;
        font-size: 10px;
        font-weight: 700;
        border-radius: 4px;
        cursor: pointer;
        text-transform: uppercase;
    }
    .paper-reset-btn:hover { background: rgba(239, 68, 68, 0.18); }
    @media (max-width: 768px) {
        .paper-layout { grid-template-columns: 1fr; }
    }
</style>
