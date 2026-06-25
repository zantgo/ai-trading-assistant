import type { AppStore } from '../state.svelte';
import type { MultiAgentAnalysis } from '../types';
import {
    fetchAssistantHistoryFromServer,
} from './api.svelte';

function buildIndicators(snap: Record<string, unknown>) {
    return {
        rsi: snap.rsi_14 ? parseFloat(String(snap.rsi_14)) : null,
        squeeze_on: snap.squeeze_on ?? null,
        squeeze_momentum: snap.squeeze_momentum ? parseFloat(String(snap.squeeze_momentum)) : null,
        macd_line: snap.macd_line ? parseFloat(String(snap.macd_line)) : null,
        macd_signal: snap.macd_signal ? parseFloat(String(snap.macd_signal)) : null,
        macd_histogram: snap.macd_hist ? parseFloat(String(snap.macd_hist)) : null,
        macd_histogram_trend: null,
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
    };
}

export async function requestAssistantAnalysis(app: AppStore): Promise<void> {
    app.assistantLoading = true;
    app.assistantError = null;
    app.assistantResponse = null;
    app.multiAgentResponse = null;
    app.individualResults = [];
    app.analysisPhase = 'phase1';
    app.agentProgress = [
        { name: 'micro-RSI', status: 'pending' }, { name: 'micro-MACD', status: 'pending' },
        { name: 'micro-SQUEEZE', status: 'pending' }, { name: 'micro-ADX', status: 'pending' },
        { name: 'micro-BOLLINGER_ATR', status: 'pending' }, { name: 'micro-VOLUME_EMA', status: 'pending' },
        { name: 'micro-VWAP', status: 'pending' },
        { name: 'small-RSI', status: 'pending' }, { name: 'small-MACD', status: 'pending' },
        { name: 'small-SQUEEZE', status: 'pending' }, { name: 'small-ADX', status: 'pending' },
        { name: 'small-BOLLINGER_ATR', status: 'pending' }, { name: 'small-VOLUME_EMA', status: 'pending' },
        { name: 'small-VWAP', status: 'pending' },
        { name: 'medium-RSI', status: 'pending' }, { name: 'medium-MACD', status: 'pending' },
        { name: 'medium-SQUEEZE', status: 'pending' }, { name: 'medium-ADX', status: 'pending' },
        { name: 'medium-BOLLINGER_ATR', status: 'pending' }, { name: 'medium-VOLUME_EMA', status: 'pending' },
        { name: 'medium-VWAP', status: 'pending' },
        { name: 'large-RSI', status: 'pending' }, { name: 'large-MACD', status: 'pending' },
        { name: 'large-SQUEEZE', status: 'pending' }, { name: 'large-ADX', status: 'pending' },
        { name: 'large-BOLLINGER_ATR', status: 'pending' }, { name: 'large-VOLUME_EMA', status: 'pending' },
        { name: 'large-VWAP', status: 'pending' },
    ];

    try {
        const historyRes = await fetch(`/api/history?symbol=${encodeURIComponent(app.activeTab)}&timeframe_secs=60`);
        const historyData = await historyRes.json();
        const prices: number[] = (historyData.prices || []).map(Number);

        const snap = app.microTerm.latestSnapshot || {};

        const body = {
            symbol: app.activeTab,
            position: app.currentPosition,
            entry_price: app.currentPosition !== 'None' ? (parseFloat(app.entryPriceVal) || 0).toString() : '',
            historical_prices: prices,
            indicators: buildIndicators(snap),
            timeframes: {
                micro_term: buildIndicators(app.microTerm.latestSnapshot || {}),
                short_term: buildIndicators(app.smallTerm.latestSnapshot || {}),
                medium_term: buildIndicators(app.mediumTerm.latestSnapshot || {}),
                large_term: buildIndicators(app.largeTerm.latestSnapshot || {}),
            },
        };

        const res = await fetch('/api/analyze', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(body),
        });

        if (!res.ok) throw new Error(`Server returned ${res.status}`);

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
        await fetchAssistantHistory(app);
    } catch (e: any) {
        app.assistantError = e.message || 'Unknown error during analysis';
        app.analysisPhase = 'idle';
    } finally {
        app.assistantLoading = false;
    }
}

export function scrollChatToBottom(container: HTMLElement | null): void {
    requestAnimationFrame(() => {
        if (container) {
            container.scrollTop = container.scrollHeight;
        }
    });
}

export function openAssistantChat(app: AppStore, getContainer: () => HTMLElement | null): void {
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
    scrollChatToBottom(getContainer());
}

export function closeAssistantChat(app: AppStore): void {
    app.isAssistantModalOpen = false;
}

export async function sendChatMessage(app: AppStore, getContainer: () => HTMLElement | null): Promise<void> {
    const text = app.chatInputText.trim();
    if (!text || app.isChatLoading) return;

    app.chatHistory.push({ role: 'user', content: text });
    app.chatInputText = '';
    app.isChatLoading = true;
    scrollChatToBottom(getContainer());

    try {
        const res = await fetch('/api/chat', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ history: app.chatHistory }),
        });

        if (!res.ok) throw new Error(`Server returned ${res.status}`);

        const data = await res.json();
        app.chatHistory.push({ role: 'assistant', content: data.reply });
        scrollChatToBottom(getContainer());
    } catch (e: any) {
        app.chatHistory.push({
            role: 'assistant',
            content: `Sorry, I couldn't process that request: ${e.message || 'Unknown error'}`,
        });
        scrollChatToBottom(getContainer());
    } finally {
        app.isChatLoading = false;
    }
}

export async function fetchAssistantHistory(app: AppStore): Promise<void> {
    try {
        const data = await fetchAssistantHistoryFromServer();
        app.assistantHistory = data.records as any[];
        app.historyLatestClose = data.latest_close;
    } catch (_) {}
}
