import type { AppStore } from '../state.svelte';
import type { WizardAnalysisResponse } from '../types';
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
    app.wizardResponse = null;
    app.analysisPhase = 'running';

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
                fast_term: buildIndicators(app.fastTerm.latestSnapshot || {}),
                slow_term: buildIndicators(app.slowTerm.latestSnapshot || {}),
                macro_term: buildIndicators(app.macroTerm.latestSnapshot || {}),
            },
        };

        const res = await fetch('/api/analyze', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(body),
        });

        if (!res.ok) throw new Error(`Server returned ${res.status}`);

        const analysis: WizardAnalysisResponse = await res.json();
        app.wizardResponse = analysis;
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
    if (!app.wizardResponse) return;
    const resp = app.wizardResponse;
    const analyst = resp.analyst_document;
    const trader = resp.trader_decision;

    const snap = app.latestSnapshot || {};
    const contextLines: string[] = [];
    contextLines.push(`Current position: ${app.currentPosition}`);
    contextLines.push(`Symbol: ${app.activeSymbol}`);
    if (app.currentPosition !== 'None' && app.entryPriceVal) {
        contextLines.push(`Entry price: $${app.entryPriceVal}`);
    }

    const rsi = snap.rsi_14 ? parseFloat(String(snap.rsi_14)) : null;
    const price = snap.mid_price ? parseFloat(String(snap.mid_price)) : null;

    if (price !== null) contextLines.push(`Current price: $${price.toFixed(4)}`);
    if (rsi !== null) {
        const rsiDesc = rsi > 70 ? 'overbought' : rsi < 30 ? 'oversold' : 'neutral';
        contextLines.push(`RSI(14): ${rsi.toFixed(2)} (${rsiDesc})`);
    }
    contextLines.push(`Decision: ${trader.action} (confidence: ${trader.confidence})`);
    contextLines.push(`Analyst Summary: ${analyst.market_summary}`);

    const systemContext = contextLines.join('\n');

    const assistantGreeting = [
        `Hello! Based on the two-agent technical analysis, I recommend **${trader.action}**.`,
        ``,
        `**Market Summary:** ${analyst.market_summary}`,
        ``,
        `**Trend:** ${analyst.trend_indicators}`,
        ``,
        `**Confluence:** ${analyst.confluence_summary}`,
        ``,
        `**Decision Rationale:** ${trader.rationale}`,
        ``,
        `**Risk Notes:** ${trader.risk_notes}`,
        ``,
        `Feel free to ask me about any specific indicator or market condition.`,
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
