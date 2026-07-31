import type { AppStore } from '../state.svelte';
import type { IndicatorDto, IndicatorMap, TimeframeTelemetry, TimeframeSlotKind } from '../types';
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

// ─── Multi-tab coordination ────────────────────────────────────────
//
// Multiple browser tabs of the dashboard used to each open their own
// set of WebSocket connections per pair, wasting sockets and risking
// rate-limit hits against the upstream exchange. We coordinate via a
// `BroadcastChannel` so exactly one tab "owns" each pair at a time:
// the owner holds the live sockets; other tabs receive the broadcast
// frames from the owner and apply them locally without re-subscribing.
//
// When the owner closes (or the channel disconnects for any reason),
// the next tab to hear the silence promotes itself to owner. The
// `storage` event is a defensive fallback for browsers / contexts
// where `BroadcastChannel` is unavailable (e.g. some test runners).

type CrossTabMsg =
    | { kind: 'claim'; pair: string; ownerId: string }
    | { kind: 'release'; pair: string; ownerId: string }
    | { kind: 'heartbeat'; pair: string; ownerId: string; ts: number };

interface PairOwnership {
    pair: string;
    ownerId: string;
    lastHeartbeat: number;
}

const TAB_ID = Math.random().toString(36).slice(2);
const PAIR_OWNER_TTL_MS = 15000; // 2 missed heartbeats (7.5 s each) before takeover
const HEARTBEAT_INTERVAL_MS = 5000;

const crossTabChannel: BroadcastChannel | null = (() => {
    if (typeof BroadcastChannel === 'undefined') return null;
    try { return new BroadcastChannel('quant-trading-platform-ws'); }
    catch (_) { return null; }
})();

const ownedPairs = new Set<string>();
const otherTabOwnership = new Map<string, PairOwnership>();

if (crossTabChannel) {
    crossTabChannel.addEventListener('message', (ev) => {
        const msg = ev.data as CrossTabMsg;
        if (!msg || msg.ownerId === TAB_ID) return;
        if (msg.kind === 'claim' || msg.kind === 'heartbeat') {
            otherTabOwnership.set(msg.pair, { pair: msg.pair, ownerId: msg.ownerId, lastHeartbeat: msg.ts });
        } else if (msg.kind === 'release') {
            otherTabOwnership.delete(msg.pair);
        }
    });
}

function broadcastClaim(pair: string) {
    crossTabChannel?.postMessage({ kind: 'claim', pair, ownerId: TAB_ID } as CrossTabMsg);
}

function broadcastRelease(pair: string) {
    crossTabChannel?.postMessage({ kind: 'release', pair, ownerId: TAB_ID } as CrossTabMsg);
}

function broadcastHeartbeat(pair: string) {
    crossTabChannel?.postMessage({ kind: 'heartbeat', pair, ownerId: TAB_ID, ts: Date.now() } as CrossTabMsg);
}

/** True if another tab has live ownership of this pair's WebSocket and
 *  we should NOT open our own sockets. Pair ownership is held by the
 *  tab that most recently broadcast a claim/heartbeat within
 *  `PAIR_OWNER_TTL_MS`. */
export function isPairOwnedByOtherTab(pair: string): boolean {
    const o = otherTabOwnership.get(pair);
    if (!o) return false;
    return Date.now() - o.lastHeartbeat < PAIR_OWNER_TTL_MS;
}

// Periodic heartbeat for all owned pairs so other tabs see us as alive
// and don't take over our sockets.
if (typeof window !== 'undefined') {
    setInterval(() => {
        const now = Date.now();
        // Drop ownership records from tabs that haven't heartbeated.
        for (const [pair, o] of otherTabOwnership.entries()) {
            if (now - o.lastHeartbeat > PAIR_OWNER_TTL_MS) otherTabOwnership.delete(pair);
        }
        for (const pair of ownedPairs) broadcastHeartbeat(pair);
    }, HEARTBEAT_INTERVAL_MS);
}

// Per-pair debounce so rapid back/forward navigation in the UI doesn't
// open redundant sockets. The latest call wins, so when a route
// stabilises the most recent connect attempt is the one that survives.
const pendingConnectAt = new Map<string, number>();
const CONNECT_DEBOUNCE_MS = 1000;

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

    // Per-key merge: shadow ticks now skip close-only indicators entirely
    // (registry `updates_on_shadow = false`), so a simple spread merge is
    // sufficient to keep the last completed-candle values across live
    // ticks. The previous implementation rebuilt each DTO from the
    // incoming shape, which (a) lost the per-key `values` submap when
    // the incoming shadow didn't carry it and (b) re-introduced the
    // zero-valued WARMING placeholder for every close-only indicator.
    //
    // Divergence-signal preservation (v6.6+): divergence signals are
    // structural markers computed on the completed-bar path; the shadow
    // path does not re-emit them. Without this preservation branch, a
    // shadow tick that arrives between candle closes wholesale-replaces
    // the parent's `signals` array with an empty one, wiping the
    // divergence from the UI until the next completed bar. We keep the
    // prior tick's `Divergence` signals on each parent whose incoming
    // value either drops the divergence or arrives with no `signals`
    // at all — which is exactly the shape a shadow tick produces for
    // every divergence-bearing oscillator (RSI/MACD/stochastic/
    // chandemo/obv/cmf/mfi/squeeze).
    const incoming = (snapshot.indicators && typeof snapshot.indicators === 'object')
        ? (snapshot.indicators as IndicatorMap)
        : null;
    if (incoming != null && Object.keys(incoming).length > 0) {
        const merged: IndicatorMap = { ...tf.indicators };
        for (const [key, val] of Object.entries(incoming)) {
            const prev = tf.indicators[key];
            const prevDivergenceSignals = (prev?.signals ?? []).filter(
                (s) => s.kind === 'Divergence',
            );
            const incomingDivergenceSignals = (val.signals ?? []).filter(
                (s) => s.kind === 'Divergence',
            );
            if (prevDivergenceSignals.length > 0 && incomingDivergenceSignals.length === 0) {
                const nonDivergenceIncoming = (val.signals ?? []).filter(
                    (s) => s.kind !== 'Divergence',
                );
                merged[key] = {
                    ...val,
                    signals: [...nonDivergenceIncoming, ...prevDivergenceSignals],
                };
            } else {
                merged[key] = val;
            }
        }
        tf.indicators = merged;
    }
    // Only overwrite the cached snapshot when the incoming frame actually
    // carries matrix payload OR the slot has never received a snapshot
    // yet. Shadow ticks (`broadcast_live_snapshot` in
    // `crates/market-analyzer/src/analyzer/mod.rs`) intentionally zero
    // out `alignment/analysis/risk/advisory/opportunity/decision_context`
    // for throughput; without this guard a shadow frame would wipe the
    // last completed-candle payload from any consumer still reading
    // from `latestSnapshot` (e.g. `decision_context` in TradePlanStrip).
    const hasMatrixPayload = !!(snapshot.alignment || snapshot.analysis ||
        snapshot.risk || snapshot.advisory || snapshot.opportunity ||
        snapshot.decision_context);
    if (!tf.latestSnapshot || hasMatrixPayload) {
        tf.latestSnapshot = snapshot;
    }
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
    if (snapshot.indicator_lifecycle && typeof snapshot.indicator_lifecycle === 'object') {
        // Per-key merge of the indicator lifecycle map. The backend always
        // populates the full set of registered keys on every snapshot, but
        // a sparse shadow frame from a future code path or a manual
        // diagnostic should NOT wipe the prior loading state for keys
        // omitted from the incoming payload.
        const incomingLc = snapshot.indicator_lifecycle as Record<string, unknown>;
        const mergedLc: Record<string, unknown> = { ...tf.indicatorLifecycle };
        for (const k of Object.keys(incomingLc)) {
            mergedLc[k] = incomingLc[k];
        }
        tf.indicatorLifecycle = mergedLc as TimeframeTelemetry['indicatorLifecycle'];
    }
    if (snapshot.pipeline_state && typeof snapshot.pipeline_state === 'string') {
        tf.pipelineState = snapshot.pipeline_state;
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
        // Decision-context (L6) carries `trade_readiness`, the L4.75 gate
        // the Watchlist Scanner polls for. Mirror the existing
        // alignment/analysis/risk/advisory extraction so downstream
        // consumers (including the scanner's `waitForAdvisory`) can read
        // it from the pair-level store rather than trawling every TF's
        // `latestSnapshot`. The scanner polls regardless of TF — the
        // trade_readiness value is identical across the 4 WS streams once
        // the macro context has settled, so the "first arrival wins"
        // behavior is correct.
        if (snapshot.decision_context && typeof snapshot.decision_context === 'object') {
            pair.decisionContext = snapshot.decision_context;
        }
        // Opportunity matrix (L4) — entry/target/invalidation zones for
        // both sides, R:R, time horizon, confluent levels, evaluated
        // profiles. Only completed-candle frames carry this payload;
        // shadow ticks hard-code it to `None` for performance. Mirroring
        // here means `OpportunitiesPanel`, `TradePlanStrip` and
        // `RecommendationPanel` can read from `pair.opportunity` directly
        // instead of trawling `microTerm.latestSnapshot.opportunity`
        // (which the unconditional assignment above used to wipe).
        if (snapshot.opportunity && typeof snapshot.opportunity === 'object') {
            pair.opportunity = snapshot.opportunity;
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

    // Per-pair debounce: rapid back/forward navigation can fire this
    // call many times per second. Coalesce so only the most recent
    // call within `CONNECT_DEBOUNCE_MS` actually opens sockets.
    const now = Date.now();
    const lastAttempt = pendingConnectAt.get(symbol) ?? 0;
    if (now - lastAttempt < CONNECT_DEBOUNCE_MS) {
        // Schedule a trailing call so we still attach once the user
        // settles on a route. Don't attach immediately — multiple rapid
        // triggers each reset the timer.
        const trailing = setTimeout(() => {
            pendingConnectAt.delete(symbol);
            connectWsForInstance(app, wssMap, symbol);
        }, CONNECT_DEBOUNCE_MS);
        // Cancel any prior trailing timer so we don't double-schedule.
        const existing = pendingConnectAt.get(`${symbol}:trailing`);
        if (existing) clearTimeout(existing);
        pendingConnectAt.set(`${symbol}:trailing`, trailing as unknown as number);
        return;
    }
    pendingConnectAt.set(symbol, now);

    // Multi-tab coordination: if another tab already owns this pair's
    // sockets, skip opening ours. The other tab is broadcasting
    // `heartbeat` messages every `HEARTBEAT_INTERVAL_MS`; if it
    // crashes the entry will expire after `PAIR_OWNER_TTL_MS` and a
    // later call here will pick up the slack.
    if (isPairOwnedByOtherTab(symbol)) return;

    const existing = wssMap[symbol];
    if (existing) disconnectAllWs(existing);
    const state = createWsState();
    wssMap[symbol] = state;
    ownedPairs.add(symbol);
    broadcastClaim(symbol);
    connectWebsocket(app, state, symbol);
}

export function disconnectWsForInstance(wssMap: Record<string, WsState>, symbol: string): void {
    const state = wssMap[symbol];
    if (!state) return;
    disconnectAllWs(state);
    delete wssMap[symbol];
    if (ownedPairs.has(symbol)) {
        ownedPairs.delete(symbol);
        broadcastRelease(symbol);
    }
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
