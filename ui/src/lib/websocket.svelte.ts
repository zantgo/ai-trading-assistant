import type { AppStore } from '../state.svelte';
import type { IndicatorMap, TimeframeTelemetry, TimeframeSlotKind } from '../types';
import { getDecimalCount } from './telemetry';

export type WsKey = 'wsMicro' | 'wsFast' | 'wsSlow' | 'wsMacro';

/// Maps a slot key (`TimeframeSlotKind`) to the corresponding WS state key.
export const SLOT_TO_WS_KEY: Record<TimeframeSlotKind, WsKey> = {
    micro: 'wsMicro',
    fast: 'wsFast',
    slow: 'wsSlow',
    macro: 'wsMacro',
};

const WS_MAX_RETRIES = 30;

let _globalMsgCount = 0;
function logWsActivity(symbol: string, slot: string, msgCount: number): void {
    if (msgCount % 100 === 0) {
        console.log(`[WS-DIAG] ${symbol}/${slot}: message #${msgCount} at ${new Date().toISOString()}`);
    }
}
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

export function buildWsUrl(
    symbol: string,
    timeframeSecs: number,
    slot: TimeframeSlotKind,
): string {
    if (!symbol) return '';
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    // The backend now uses `slot` (`micro|fast|slow|macro`) as the
    // authoritative wire identifier, so it can never mis-route a snapshot
    // even if two slots happen to share the same `timeframe_secs`.
    return `${protocol}//${window.location.host}/ws?symbol=${encodeURIComponent(symbol)}&timeframe_secs=${timeframeSecs}&slot=${slot}`;
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
    _globalMsgCount++;
    logWsActivity(symbol, tf.slot, _globalMsgCount);
    const raw = JSON.parse(event.data);
    const snapshot = (raw.jsonrpc === '2.0' && raw.method === 'broadcast.market_snapshot')
        ? (raw.params?.snapshot || raw)
        : raw;
    if (!snapshot || typeof snapshot !== 'object') return;

    // Slot guard: the backend stamps `timeframe_slot` on every snapshot.
    // If a foreign slot's snapshot somehow arrives (corrupted dispatcher,
    // shared broadcast channel pre-fix, etc.) drop it instead of letting
    // it silently mutate this slot's telemetry.
    const wireSlot = (snapshot as Record<string, unknown>).timeframe_slot;
    if (wireSlot != null && wireSlot !== tf.slot) return;

    tf.indicators = (snapshot.indicators && typeof snapshot.indicators === 'object')
            ? (snapshot.indicators as IndicatorMap)
            : {};
        tf.latestSnapshot = snapshot;
        tf.isCompleted = snapshot.is_completed === true;

        // Capture the per-TF MarketContext synthesis block (L1 LOCAL
        // 5-dimension + regime + overall score/label). Previously this
        // lived only inside `latestSnapshot` as an opaque record and was
        // never surfaced. The MarketContextStrip in the redesigned Metrics
        // view reads this directly.
        if (snapshot.context && typeof snapshot.context === 'object') {
            tf.context = snapshot.context;
        }

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
        if (snapshot.volume_profile && typeof snapshot.volume_profile === 'object') {
            tf.volumeProfile = snapshot.volume_profile;
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
    tfSecs: number,
    symbol: string,
): void {
    const wsKey: WsKey = SLOT_TO_WS_KEY[tf.slot];
    closeWs(state[wsKey]);

    const url = buildWsUrl(symbol, tfSecs, tf.slot);
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
                    connectWebsocketForTimeframe(app, state, tf, tfSecs, symbol);
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

    // Each `TimeframeTelemetry` carries its own slot identity, so the WS
    // dispatcher can no longer mis-route by shared duration.
    connectWebsocketForTimeframe(app, state, pair.microTerm, pair.microTerm.barDurationSec, symbol);
    connectWebsocketForTimeframe(app, state, pair.fastTerm,  pair.fastTerm.barDurationSec,  symbol);
    connectWebsocketForTimeframe(app, state, pair.slowTerm,  pair.slowTerm.barDurationSec,  symbol);
    connectWebsocketForTimeframe(app, state, pair.macroTerm, pair.macroTerm.barDurationSec, symbol);
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
