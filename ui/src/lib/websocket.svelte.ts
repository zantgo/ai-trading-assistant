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
export function applySnapshotToTimeframe(app: AppStore, tf: TimeframeTelemetry, event: MessageEvent, symbol: string): void {
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

        if (snapshot.liquidity && typeof snapshot.liquidity === 'object') {
            tf.liquidity = snapshot.liquidity;
        }
        if (snapshot.cluster && typeof snapshot.cluster === 'object') {
            tf.cluster = snapshot.cluster;
        }
        if (Array.isArray(snapshot.liquidity_signals)) {
            tf.liquiditySignals = snapshot.liquidity_signals;
        }
        const pair = app.instancesMap[symbol];
        if (pair) {
            if (snapshot.alignment && typeof snapshot.alignment === 'object') {
                pair.alignment = snapshot.alignment;
            }
            if (snapshot.analysis && typeof snapshot.analysis === 'object') {
                pair.analysis = snapshot.analysis;
            }
            if (snapshot.risk && typeof snapshot.risk === 'object') {
                pair.risk = snapshot.risk;
            }
            if (snapshot.advisory && typeof snapshot.advisory === 'object') {
                pair.advisory = snapshot.advisory;
            }
        }
    } catch (_) {}
}

export function connectWebsocketForTimeframe(
    app: AppStore,
    state: WsState,
    tf: TimeframeTelemetry,
    wsKey: WsKey,
    tfSecs: number,
    symbol: string,
): void {
    closeWs(state[wsKey]);

    const url = buildWsUrl(symbol, tfSecs);
    if (!url) return;

    const newWs = new WebSocket(url);
    state[wsKey] = newWs;

    newWs.onopen = () => {
        const pair = app.instancesMap[symbol];
        if (pair) pair.isConnected = true;
        state.backoff[wsKey] = freshBackoff();
    };
    newWs.onmessage = (event) => applySnapshotToTimeframe(app, tf, event, symbol);
    newWs.onclose = () => {
        const pairAfter = app.instancesMap[symbol];
        if (pairAfter) pairAfter.isConnected = false;
        if (state[wsKey] === newWs) {
            state[wsKey] = null;
        }
        const bo = state.backoff[wsKey];
        state.backoff[wsKey] = nextBackoff(bo);
        if (bo.retries < WS_MAX_RETRIES) {
            setTimeout(() => {
                if (app.instancesMap[symbol]) {
                    connectWebsocketForTimeframe(app, state, tf, wsKey, tfSecs, symbol);
                }
            }, bo.delayMs);
        }
    };
    newWs.onerror = () => { newWs.close(); };
}

export function connectWebsocket(app: AppStore, state: WsState, symbol: string): void {
    if (!symbol) return;
    state.currentWsSymbol = symbol;

    const pair = app.instancesMap[symbol];
    if (!pair) return;

    connectWebsocketForTimeframe(app, state, pair.microTerm, 'wsMicro', pair.microTerm.barDurationSec, symbol);
    connectWebsocketForTimeframe(app, state, pair.fastTerm,  'wsFast',  pair.fastTerm.barDurationSec,  symbol);
    connectWebsocketForTimeframe(app, state, pair.slowTerm,  'wsSlow',  pair.slowTerm.barDurationSec,  symbol);
    connectWebsocketForTimeframe(app, state, pair.macroTerm, 'wsMacro', pair.macroTerm.barDurationSec, symbol);
}

export function connectWsForInstance(
    app: AppStore,
    wssMap: Record<string, WsState>,
    symbol: string,
): void {
    if (!symbol) return;
    const existing = wssMap[symbol];
    if (existing) disconnectAllWs(existing);
    const state = createWsState();
    wssMap[symbol] = state;
    connectWebsocket(app, state, symbol);
}

export function disconnectWsForInstance(wssMap: Record<string, WsState>, symbol: string): void {
    const state = wssMap[symbol];
    if (!state) return;
    disconnectAllWs(state);
    delete wssMap[symbol];
}

export function shouldReconnect(app: AppStore, state: WsState, symbol: string): boolean {
    if (!symbol) return false;

    const pair = app.instancesMap[symbol];
    if (!pair) return false;

    const connectionsNeeded = 4;
    let activeConnections = 0;
    if (state.wsMicro && state.wsMicro.readyState === WebSocket.OPEN) activeConnections++;
    if (state.wsFast  && state.wsFast.readyState  === WebSocket.OPEN) activeConnections++;
    if (state.wsSlow  && state.wsSlow.readyState  === WebSocket.OPEN) activeConnections++;
    if (state.wsMacro && state.wsMacro.readyState === WebSocket.OPEN) activeConnections++;

    return activeConnections < connectionsNeeded;
}
