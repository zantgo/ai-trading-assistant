import type { AppStore } from '../state.svelte';
import type { IndicatorMap, TimeframeTelemetry } from '../types';
import { getDecimalCount } from './telemetry';

export type WsKey = 'wsMicro' | 'wsFast' | 'wsSlow' | 'wsMacro';

const WS_MAX_RETRIES = 30;
const WS_INITIAL_DELAY_MS = 1000;
const WS_MAX_DELAY_MS = 30000;

interface WsBackoff {
    retries: number;
    delayMs: number;
}

export interface WsState {
    wsMicro: WebSocket | null;
    wsFast: WebSocket | null;
    wsSlow: WebSocket | null;
    wsMacro: WebSocket | null;
    currentWsSymbol: string;
    backoff: Record<WsKey, WsBackoff>;
}

function freshBackoff(): WsBackoff {
    return { retries: 0, delayMs: WS_INITIAL_DELAY_MS };
}

function nextBackoff(b: WsBackoff): WsBackoff {
    return {
        retries: b.retries + 1,
        delayMs: Math.min(b.delayMs * 2, WS_MAX_DELAY_MS),
    };
}

export function createWsState(): WsState {
    return {
        wsMicro: null,
        wsFast: null,
        wsSlow: null,
        wsMacro: null,
        currentWsSymbol: '',
        backoff: {
            wsMicro: freshBackoff(),
            wsFast: freshBackoff(),
            wsSlow: freshBackoff(),
            wsMacro: freshBackoff(),
        },
    };
}

export function buildWsUrl(symbol: string, timeframeSecs: number): string {
    if (!symbol) return '';
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    return `${protocol}//${window.location.host}/ws?symbol=${encodeURIComponent(symbol)}&timeframe_secs=${timeframeSecs}`;
}

export function closeWs(ws: WebSocket | null): void {
    if (ws && ws.readyState !== WebSocket.CLOSED && ws.readyState !== WebSocket.CLOSING) {
        try { ws.onclose = null; ws.onerror = null; ws.close(); } catch (_) {}
    }
}

export function disconnectAllWs(state: WsState): void {
    closeWs(state.wsMicro); state.wsMicro = null;
    closeWs(state.wsFast); state.wsFast = null;
    closeWs(state.wsSlow); state.wsSlow = null;
    closeWs(state.wsMacro); state.wsMacro = null;
}

function num(v: unknown): number | null {
    if (v === undefined || v === null) return null;
    const n = typeof v === 'number' ? v : parseFloat(String(v));
    return Number.isNaN(n) ? null : n;
}

/**
 * Parse a WebSocket `broadcast.market_snapshot` notification into the
 * timeframe telemetry. The nested `indicators` map is the sole source of
 * truth; only genuine non-indicator market data (price/volume) is stored as
 * flat text alongside it.
 */
export function applySnapshotToTimeframe(tf: TimeframeTelemetry, event: MessageEvent): void {
    try {
        const raw = JSON.parse(event.data);
        const snapshot = (raw.jsonrpc === '2.0' && raw.method === 'broadcast.market_snapshot')
            ? (raw.params?.snapshot || raw)
            : raw;
        if (!snapshot || typeof snapshot !== 'object') return;

        tf.indicators = (snapshot.indicators && typeof snapshot.indicators === 'object')
            ? (snapshot.indicators as IndicatorMap)
            : {};
        tf.latestSnapshot = snapshot;
        tf.isCompleted = snapshot.is_completed === true;

        const mid = num(snapshot.mid_price);
        if (mid != null) tf.priceText = mid.toFixed(getDecimalCount(mid));
        const vol = num(snapshot.volume);
        if (vol != null) tf.volText = vol.toFixed(2);
        const avgVol = num(snapshot.average_volume);
        if (avgVol != null) tf.avgVolText = avgVol.toFixed(2);
    } catch (_) {}
}

export function connectWebsocketForTimeframe(
    app: AppStore,
    state: WsState,
    tf: TimeframeTelemetry,
    wsKey: WsKey,
    tfSecs: number,
): void {
    closeWs(state[wsKey]);

    const url = buildWsUrl(app.activeTab, tfSecs);
    if (!url) return;

    const newWs = new WebSocket(url);
    state[wsKey] = newWs;

    newWs.onopen = () => {
        app.isConnected = true;
        state.backoff[wsKey] = freshBackoff();
    };
    newWs.onmessage = (event) => applySnapshotToTimeframe(tf, event);
    newWs.onclose = () => {
        app.isConnected = false;
        if (state[wsKey] === newWs) {
            state[wsKey] = null;
        }
        const bo = state.backoff[wsKey];
        state.backoff[wsKey] = nextBackoff(bo);
        if (bo.retries < WS_MAX_RETRIES) {
            setTimeout(() => {
                if (app.activeTab === state.currentWsSymbol) {
                    connectWebsocketForTimeframe(app, state, tf, wsKey, tfSecs);
                }
            }, bo.delayMs);
        }
    };
    newWs.onerror = () => { newWs.close(); };
}

export function connectWebsocket(app: AppStore, state: WsState): void {
    const symbol = app.activeTab;
    if (!symbol) return;
    state.currentWsSymbol = symbol;

    const pair = app.instancesMap[symbol];
    if (!pair) return;

    connectWebsocketForTimeframe(app, state, pair.microTerm, 'wsMicro', pair.microTerm.barDurationSec);
    connectWebsocketForTimeframe(app, state, pair.fastTerm, 'wsFast', pair.fastTerm.barDurationSec);
    connectWebsocketForTimeframe(app, state, pair.slowTerm, 'wsSlow', pair.slowTerm.barDurationSec);
    connectWebsocketForTimeframe(app, state, pair.macroTerm, 'wsMacro', pair.macroTerm.barDurationSec);
}

export function shouldReconnect(app: AppStore, state: WsState): boolean {
    const symbol = app.activeTab;
    if (!symbol) return false;
    if (symbol !== state.currentWsSymbol) return true;

    const pair = app.instancesMap[symbol];
    if (!pair) return false;

    const connectionsNeeded = 4;
    let activeConnections = 0;
    if (state.wsMicro && state.wsMicro.readyState === WebSocket.OPEN) activeConnections++;
    if (state.wsFast && state.wsFast.readyState === WebSocket.OPEN) activeConnections++;
    if (state.wsSlow && state.wsSlow.readyState === WebSocket.OPEN) activeConnections++;
    if (state.wsMacro && state.wsMacro.readyState === WebSocket.OPEN) activeConnections++;

    return activeConnections < connectionsNeeded;
}
