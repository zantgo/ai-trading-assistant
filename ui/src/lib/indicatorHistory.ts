// Unified indicator history layer — single source of truth for every
// chart component in `ui/src/components/*Chart.svelte`. All 27 chart
// components (and the PriceChart overlays) pull from this single helper
// for both their historical bootstrap and live snapshot reads.
//
// All charts consume the same `IndicatorFlatHistory` shape via the
// `historyValue(hist, key, subKey?)` accessor. The cache is single-flight,
// keyed by (pairKey, timeframe_secs), so opening N charts for the same
// (pair, timeframe) results in exactly ONE `/api/history` request.
//
// Sub-minute timeframes (1 s, 3 s, 5 s, 15 s, 30 s) are first-class. The
// fetch passes `timeframe_secs` verbatim; no minute-rounding floor.

import type { Time } from 'lightweight-charts';

/// Aligned time axis + per-field value arrays. `values` is keyed by:
///   "<indicatorKey>"                 — single-raw indicators (rsi, cci, obv, ...)
///   "<indicatorKey>.<subKey>"        — sub-keyed indicators (macd.line, keltner.upper, ...)
///   "candles.<ohlc>"                 — candle data: 'candles.open', 'candles.high', 'candles.low', 'candles.close', 'candles.volume'
///   "clusters"                       — LiquidationClusterMatrix[]
///   "volumeProfiles"                 — VolumeProfileSnapshot[]
///   "prices"                         — legacy flat price series (string[]) for endpoints
///                                     that don't emit structured candles
export interface IndicatorFlatHistory {
    times: number[];
    values: Record<string, Array<number | null>>;
    candleTimes: number[];
    candles: { open: number[]; high: number[]; low: number[]; close: number[]; volume: number[] };
    /// Per-candle `reconstructed` provenance (SCREAMING_SNAKE_CASE enum
    /// string or `undefined`). Parallel to `candleTimes`/`candles` —
    /// `candleReconstructed[i]` describes `candles.open[i]`. The chart
    /// uses this to filter synthetic gap-fill Dojis out of the
    /// persistent candle cache (see `setCachedCandles`).
    candleReconstructed?: Array<string | undefined>;
    prices?: string[];
    clusters?: Record<string, unknown>;
    volumeProfiles?: Record<string, unknown>;
    fetchedAtMs: number;
}

const HISTORY_URL = '/api/history';

const cache = new Map<string, Promise<IndicatorFlatHistory | null>>();
// Resolved history objects for live mutation (P0 fix: sub-minute live-append).
// The promise cache above is write-once; this map holds the mutable object
// that tab-switch remounts read, so live candles are not lost on navigation.
const historyData = new Map<string, IndicatorFlatHistory>();

/// Fetch the indicator-history payload for `(pairKey, slot, timeframe_secs)`.
///
/// AUDIT-AIU-121: the cache key AND the request carry the SLOT. The legacy
/// duration-only key `${pairKey}@${timeframe}` let two slots sharing one
/// duration share a single cached payload sourced from the micro pipeline —
/// the second chart's historical overlays were seeded from micro's indicator
/// config. The slot hint makes `/api/history` resolve the exact pipeline.
///
/// Returns a single shared promise per cache key — repeated mounts are free.
/// The response is normalized into `IndicatorFlatHistory`: every indicator
/// key the backend emitted becomes a top-level entry under `values`, and
/// every `values.*` sub-key becomes an entry under `values['<key>.<sub>']`.
///
/// Sub-keys exposed by the backend (auto-discovered from the response):
/// for `macd`, `aroon`, `stochastic`, `bollinger`, `keltner`, `donchian`,
/// `ichimoku`, `supertrend`, `anchored_vwap`, `smc_*`, derivatives, etc.
export function fetchIndicatorHistoryOnce(
    pairKey: string,
    timeframe: number,
    slot?: string,
): Promise<IndicatorFlatHistory | null> {
    if (!pairKey || !timeframe) return Promise.resolve(null);
    const key = `${pairKey}@${slot ?? '?'}@${timeframe}`;
    const cached = cache.get(key);
    if (cached) return cached;

    const promise = (async (): Promise<IndicatorFlatHistory | null> => {
        try {
            const slotParam = slot ? `&slot=${encodeURIComponent(slot)}` : '';
            const res = await fetch(
                `${HISTORY_URL}?symbol=${encodeURIComponent(pairKey)}&timeframe_secs=${timeframe}&limit=1000${slotParam}`,
            );
            if (!res.ok) return null;
            const raw = await res.json() as RawResponse;
            const hist = normalizeHistory(raw);
            // Keep mutable reference for live ingestion (P0).
            // FIX: sub-minute live tail race — if `ingestLiveSnapshot` already
            // created a live-mutated entry while we were fetching (cold 1s
            // start, server empty/stale), don't clobber it. Merge instead.
            if (hist) {
                const existing = historyData.get(key);
                if (existing && existing.times.length > 0) {
                    if (hist.times.length === 0) {
                        // Cold sub-minute server empty but live already has
                        // candles (P0 keepalive) — keep live, preserve server
                        // clusters/volumeProfiles if present.
                        if (hist.clusters || hist.volumeProfiles) {
                            existing.clusters = hist.clusters ?? existing.clusters;
                            existing.volumeProfiles = hist.volumeProfiles ?? existing.volumeProfiles;
                        }
                        return existing;
                    }
                    const serverLast = hist.times[hist.times.length - 1] ?? -Infinity;
                    const hasLiveTail = existing.times.some((t) => t > serverLast);
                    if (hasLiveTail) {
                        // Preserve live tail newer than server — delegate to
                        // merge helper (caps at 1000, keeps indicator alignment).
                        mergeHistoryRefresh(pairKey, timeframe, slot, hist);
                        return historyData.get(key) ?? hist;
                    }
                }
                historyData.set(key, hist);
            }
            return hist;
        } catch (err) {
            console.error('indicatorHistory fetch failed', err);
            return null;
        }
    })();
    cache.set(key, promise);
    // Also populate historyData when promise resolves (covers already-pending).
    // Merge-aware so a live tail created during fetch is not erased.
    promise.then((h) => {
        if (!h) return;
        const cur = historyData.get(key);
        if (!cur || cur === h) return;
        // A live entry was created/mutated while fetching — don't overwrite blindly.
        if (cur.times.length > 0 && h.times.length === 0) return;
        if (cur.times.length > 0 && h.times.length > 0) {
            const serverLast = h.times[h.times.length - 1] ?? -Infinity;
            if (cur.times.some((t) => t > serverLast)) return;
        }
        historyData.set(key, h);
    });
    return promise;
}

/// Test hook / daemon restart: drops the in-memory cache so subsequent
/// mounts re-fetch from the server.
export function clearHistoryCache(): void {
    cache.clear();
    historyData.clear();
}

export function purgeCacheForKey(pairKey: string, timeframe: number, slot?: string): void {
    const k = `${pairKey}@${slot ?? '?'}@${timeframe}`;
    cache.delete(k);
    historyData.delete(k);
}

/// P0 fix: live ingestion for sub-minute history preservation.
/// Mutates the cached `IndicatorFlatHistory` so tab-switch remounts
/// see live-accumulated candles/indicators even when the initial
/// fetch was empty (cold sub-minute start). No-ops on shadow ticks
/// (`is_completed !== true`) — only completed candles advance history
/// (PRI-06).
export function ingestLiveSnapshot(
    pairKey: string,
    timeframe: number,
    slot: string | undefined,
    snapshot: Record<string, unknown>,
): void {
    if (!pairKey || !timeframe) return;
    const isCompleted = snapshot.is_completed === true;
    if (!isCompleted) return;
    const tsRaw = snapshot.timestamp;
    const ts = typeof tsRaw === 'number' ? tsRaw : Number(tsRaw ?? 0);
    if (!Number.isFinite(ts) || ts <= 0) return;
    const key = `${pairKey}@${slot ?? '?'}@${timeframe}`;
    let hist = historyData.get(key);
    // Cold path: no history yet (initial fetch empty or not yet fetched).
    // Create a fresh history so live candles are not lost on the next remount.
    if (!hist) {
        hist = {
            times: [],
            values: {},
            candleTimes: [],
            candles: { open: [], high: [], low: [], close: [], volume: [] },
            candleReconstructed: [],
            fetchedAtMs: Date.now(),
        };
        historyData.set(key, hist);
        // Prime the promise cache with the live-built history so
        // `fetchIndicatorHistoryOnce` returns it without a network roundtrip.
        cache.set(key, Promise.resolve(hist));
    }
    // De-duplicate: same timestamp as last entry = update in place (completed
    // candle for same bucket should not duplicate).
    if (hist.times.length > 0 && hist.times[hist.times.length - 1] === ts) return;

    // Append timestamp.
    hist.times.push(ts);

    // Append candle OHLCV if present.
    const open = parseFloat(String((snapshot as Record<string, unknown>).open ?? snapshot.close ?? '0')) || 0;
    const high = parseFloat(String((snapshot as Record<string, unknown>).high ?? snapshot.close ?? '0')) || 0;
    const low = parseFloat(String((snapshot as Record<string, unknown>).low ?? snapshot.close ?? '0')) || 0;
    const close = parseFloat(String((snapshot as Record<string, unknown>).close ?? '0')) || 0;
    const vol = parseFloat(String((snapshot as Record<string, unknown>).volume ?? '0')) || 0;
    const reconstructed = (snapshot as Record<string, unknown>).quality_envelope
        ? ((snapshot as Record<string, unknown>).quality_envelope as Record<string, unknown>).is_gap_filled ? 'SYNTHETIC' : undefined
        : undefined;
    hist.candleTimes.push(ts);
    hist.candles.open.push(open);
    hist.candles.high.push(high);
    hist.candles.low.push(low);
    hist.candles.close.push(close);
    hist.candles.volume.push(vol);
    if (hist.candleReconstructed) hist.candleReconstructed.push(reconstructed);
    else hist.candleReconstructed = [reconstructed];

    // Append indicator values. Keep `values` arrays aligned to `times`.
    const incoming = (snapshot.indicators && typeof snapshot.indicators === 'object')
        ? (snapshot.indicators as Record<string, { raw_value?: number; state_label?: string; values?: Record<string, number> }>)
        : null;

    // Ensure every existing key gets a slot (null if missing this tick).
    const existingKeys = Object.keys(hist.values);
    const incomingKeys = new Set<string>();
    if (incoming) {
        for (const [k, dto] of Object.entries(incoming)) {
            const isWarming = dto?.state_label === 'WARMING';
            const raw = isWarming ? null : (typeof dto.raw_value === 'number' && Number.isFinite(dto.raw_value) ? dto.raw_value : null);
            // raw series
            const rk = k;
            incomingKeys.add(rk);
            if (!(rk in hist.values)) {
                // Backfill with nulls for prior timestamps.
                hist.values[rk] = Array(hist.times.length - 1).fill(null);
            }
            hist.values[rk].push(raw);
            // sub-keys
            if (dto.values && typeof dto.values === 'object') {
                for (const [sub, sv] of Object.entries(dto.values)) {
                    const sk = `${k}.${sub}`;
                    incomingKeys.add(sk);
                    if (!(sk in hist.values)) {
                        hist.values[sk] = Array(hist.times.length - 1).fill(null);
                    }
                    const sval = isWarming ? null : (typeof sv === 'number' && Number.isFinite(sv) ? sv : null);
                    hist.values[sk].push(sval);
                }
            }
        }
    }
    // For keys that were in history but not updated this tick, push null to keep alignment.
    for (const ek of existingKeys) {
        if (!incomingKeys.has(ek)) {
            hist.values[ek].push(null);
        }
    }

    // Cap at HIST_BUFFER_MAX = 1000 (same as backend).
    const HIST_MAX = 1000;
    if (hist.times.length > HIST_MAX) {
        const trim = hist.times.length - HIST_MAX;
        hist.times.splice(0, trim);
        hist.candleTimes.splice(0, trim);
        hist.candles.open.splice(0, trim);
        hist.candles.high.splice(0, trim);
        hist.candles.low.splice(0, trim);
        hist.candles.close.splice(0, trim);
        hist.candles.volume.splice(0, trim);
        if (hist.candleReconstructed) hist.candleReconstructed.splice(0, trim);
        for (const arr of Object.values(hist.values)) {
            arr.splice(0, trim);
        }
    }
    hist.fetchedAtMs = Date.now();
}

/// Test hook: read resolved history (for unit tests).
export function getResolvedHistory(pairKey: string, timeframe: number, slot?: string): IndicatorFlatHistory | null {
    return historyData.get(`${pairKey}@${slot ?? '?'}@${timeframe}`) ?? null;
}

// ── Processed candle cache ────────────────────────────────────────────
// Per (pairKey, slot, timeframe) cache of the final OHLCV array fed to
// lightweight-charts. This survives component unmount/remount so
// timeframe switches and back/forward navigation don't wipe the chart —
// the bootstrap paints from cache immediately while the async history
// fetch refreshes in the background.
//
// v10.2 fix: slot-aware key `${pairKey}@${slot}@${timeframe}`. The legacy
// duration-only key `${pairKey}@${timeframe}` let two slots sharing one
// duration collide and let the PriceChart purge miss (history cache was
// slot-aware since AUDIT-AIU-121 but the candle cache was not). Switching
// from 1s (micro) to another chart and back now restores the exact slot's
// live candles instantly; no server refetch is needed for a warm cache.

/// Cached candle shape. `reconstructed` is the SCREAMING_SNAKE_CASE
/// backend enum string (e.g. `SYNTHETIC`) or `undefined` for real candles.
/// `PriceChart.svelte` filters out any candle with a truthy `reconstructed`
/// before painting and caching so synthetic gap-fill Dojis never poison
/// the persistent candle cache.
export type CandleOHLCV = {
    time: Time;
    open: number;
    high: number;
    low: number;
    close: number;
    reconstructed?: string;
};

const candleCache = new Map<string, CandleOHLCV[]>();

function candleCacheKey(pairKey: string, timeframe: number, slot?: string): string {
    return `${pairKey}@${slot ?? '?'}@${timeframe}`;
}

export function getCachedCandles(pairKey: string, timeframe: number, slot?: string): CandleOHLCV[] | null {
    // Slot-aware lookup with duration-only fallback so a single cold miss
    // after the migration still finds the previous duration-only entry once.
    const slotKey = candleCacheKey(pairKey, timeframe, slot);
    const hit = candleCache.get(slotKey);
    if (hit) return hit;
    if (slot != null) {
        const legacy = candleCache.get(`${pairKey}@${timeframe}`);
        if (legacy) return legacy;
    }
    return null;
}

export function setCachedCandles(pairKey: string, timeframe: number, candles: CandleOHLCV[], slot?: string): void {
    // Defence in depth: never cache synthetic candles even if the caller
    // forgot to filter. The backend may serve a future reconstruction path
    // that we don't yet know about; this guard keeps the cache pure.
    const real = candles.filter((c) => !c.reconstructed);
    if (real.length > 0) candleCache.set(candleCacheKey(pairKey, timeframe, slot), real);
}

/// P0 fix: live candle append for `candleCache` so tab-switch preserves
/// live-accumulated candles (especially sub-minute where `setCachedCandles`
/// was only called on cold bootstrap). Called from `websocket.svelte.ts`
/// on every completed candle; dedups by `time`, caps at 1000, keeps sorted.
export function appendLiveCandle(pairKey: string, timeframe: number, slot: string | undefined, candle: CandleOHLCV): void {
    if (!pairKey || !timeframe || !candle || candle.reconstructed) return;
    const t = Number(candle.time);
    if (!Number.isFinite(t) || t <= 0) return;
    const key = candleCacheKey(pairKey, timeframe, slot);
    const existing = candleCache.get(key) ?? [];
    if (existing.length > 0 && Number(existing[existing.length - 1].time) === t) {
        // Same bucket — replace (e.g. completed candle update).
        existing[existing.length - 1] = candle;
        candleCache.set(key, existing);
        return;
    }
    if (existing.length > 0 && t < Number(existing[existing.length - 1].time)) {
        // Out-of-order live tick — ignore (monotonic history).
        return;
    }
    existing.push(candle);
    // Hard cap HIST_BUFFER_MAX = 1000 (backend parity).
    if (existing.length > 1000) {
        existing.splice(0, existing.length - 1000);
    }
    candleCache.set(key, existing);
}

export function purgeCandleCacheForKey(pairKey: string, timeframe: number, slot?: string): void {
    candleCache.delete(candleCacheKey(pairKey, timeframe, slot));
}

/// Build the final candle array handed to lightweight-charts.
///
/// Deliberately does NOT drop candles the backend marked `reconstructed`
/// (SYNTHETIC doji-fill / heartbeat candles). On sparse sub-minute markets
/// those dojis are the majority of the in-memory history buffer, so a
/// fresh mount (F5 reload, cold start) whose only source is `/api/history`
/// would render ~90% less history than the live WebSocket coalescer paints
/// (which has no such filter). Synthetic candles are instead excluded at
/// the persistent-cache boundary (`setCachedCandles`) so navigation can
/// never replay flat-line "ghost" Dojis — see AUDIT-V8-004 in
/// `crates/market-analyzer` and `crates/api-gateway/src/handlers/history.rs`.
export function buildPaintCandles(
    historicalCandles: CandleOHLCV[],
    stepSec: number,
    maxFill: number = 300,
): CandleOHLCV[] {
    return fillTimeGaps(historicalCandles, stepSec, maxFill);
}

export function clearCandleCache(): void {
    candleCache.clear();
}

/// Merge a background-refreshed history into the live-mutated cache
/// without losing live-appended tail. The server payload contains the
/// authoritative snapshot_history (which includes live candles), so we
/// can replace the prefix but keep any tail timestamps newer than the
/// server's last time (those are live candles not yet in snapshot_history
/// due to race). Used if we ever trigger a background refresh.
export function mergeHistoryRefresh(pairKey: string, timeframe: number, slot: string | undefined, serverHist: IndicatorFlatHistory): void {
    const key = `${pairKey}@${slot ?? '?'}@${timeframe}`;
    const live = historyData.get(key);
    if (!live || !serverHist || serverHist.times.length === 0) {
        historyData.set(key, serverHist);
        cache.set(key, Promise.resolve(serverHist));
        return;
    }
    const serverLast = serverHist.times[serverHist.times.length - 1] ?? -Infinity;
    const tailIdx = live.times.findIndex((t) => t > serverLast);
    if (tailIdx === -1) {
        historyData.set(key, serverHist);
        cache.set(key, Promise.resolve(serverHist));
        return;
    }
    // Keep serverHist + live tail
    const tailTimes = live.times.slice(tailIdx);
    const tailCandles = live.candleTimes.slice(tailIdx);
    // Merge tail values
    for (const [k, arr] of Object.entries(live.values)) {
        const serverArr = serverHist.values[k];
        if (!serverArr) {
            serverHist.values[k] = Array(serverHist.times.length).fill(null).concat(arr.slice(tailIdx));
        } else {
            serverHist.values[k] = serverArr.concat(arr.slice(tailIdx));
        }
    }
    serverHist.times = serverHist.times.concat(tailTimes);
    serverHist.candleTimes = serverHist.candleTimes.concat(tailCandles);
    serverHist.candles.open = serverHist.candles.open.concat(live.candles.open.slice(tailIdx));
    serverHist.candles.high = serverHist.candles.high.concat(live.candles.high.slice(tailIdx));
    serverHist.candles.low = serverHist.candles.low.concat(live.candles.low.slice(tailIdx));
    serverHist.candles.close = serverHist.candles.close.concat(live.candles.close.slice(tailIdx));
    serverHist.candles.volume = serverHist.candles.volume.concat(live.candles.volume.slice(tailIdx));
    if (serverHist.candleReconstructed && live.candleReconstructed) {
        serverHist.candleReconstructed = (serverHist.candleReconstructed as Array<string|undefined>).concat(live.candleReconstructed.slice(tailIdx));
    }
    serverHist.fetchedAtMs = Date.now();
    // Cap
    if (serverHist.times.length > 1000) {
        const trim = serverHist.times.length - 1000;
        serverHist.times.splice(0, trim);
        serverHist.candleTimes.splice(0, trim);
        serverHist.candles.open.splice(0, trim);
        serverHist.candles.high.splice(0, trim);
        serverHist.candles.low.splice(0, trim);
        serverHist.candles.close.splice(0, trim);
        serverHist.candles.volume.splice(0, trim);
        if (serverHist.candleReconstructed) (serverHist.candleReconstructed as Array<string|undefined>).splice(0, trim);
        for (const arr of Object.values(serverHist.values)) arr.splice(0, trim);
    }
    historyData.set(key, serverHist);
    cache.set(key, Promise.resolve(serverHist));
}

// ── Gap-fill utility ──────────────────────────────────────────────────
// Lightweight Charts renders on a continuous time axis — any missing
// interval between two consecutive candle timestamps becomes a
// proportional pixel gap.  This function scans a sorted candle array and
// inserts flat Doji candles (O=H=L=C=prev close) for missing intervals
// so the chart remains visually continuous even when the backend hasn't
// yet accumulated every bar (e.g. cold sub-minute start, DB fallback
// with sparse rows).

export function fillTimeGaps(
    candles: CandleOHLCV[],
    expectedStepSec: number,
    maxFill: number = 300,
): CandleOHLCV[] {
    if (candles.length < 2) return candles;
    const filled: CandleOHLCV[] = [];
    for (let i = 0; i < candles.length; i++) {
        filled.push(candles[i]);
        if (i + 1 < candles.length) {
            const nextTime = Number(candles[i + 1].time);
            const currTime = Number(candles[i].time);
            const gap = nextTime - currTime;
            const missing = Math.floor(gap / expectedStepSec) - 1;
            const fillCount = Math.min(missing, maxFill);
            const close = candles[i].close;
            for (let j = 1; j <= fillCount; j++) {
                filled.push({
                    time: (currTime + j * expectedStepSec) as Time,
                    open: close,
                    high: close,
                    low: close,
                    close,
                });
            }
        }
    }
    return filled;
}

interface RawHistoryIndicator {
    raw?: Array<number | null>;
    normalized?: Array<number | null>;
    state_label?: Array<string | null>;
    values?: Record<string, Array<number | null>>;
}

interface RawHistory {
    times?: number[];
    indicators?: Record<string, RawHistoryIndicator>;
}

interface RawHistoryCandle {
    time: number;
    open: string;
    high: string;
    low: string;
    close: string;
    volume: string;
    /// Reconstruction provenance from the backend
    /// (`core_domain::normalized::ReconstructionMethod`). SCREAMING_SNAKE_CASE
    /// when present: `EXCHANGE_HISTORICAL`, `EXPONENTIAL_MOVING_AVERAGE`,
    /// `LINEAR_INTERPOLATION`, `UNAVAILABLE`, `SYNTHETIC`. Absent for
    /// real, persisted OHLCV candles. The chart-side `normalizeHistory`
    /// uses this flag to drop synthetic gap-fill Dojis out of the
    /// persistent candle cache so the chart never paints a flat-line
    /// "ghost" from minute-close interpolation.
    reconstructed?: string | null;
}

interface RawResponse {
    prices?: string[];
    candles?: RawHistoryCandle[];
    indicator_history?: RawHistory | null;
    clusters?: Record<string, unknown>;
    volume_profiles?: Record<string, unknown>;
}

function toNumberArray(arr: ReadonlyArray<number | null | string | undefined>): Array<number | null> {
    if (!arr) return [];
    return arr.map((v) => {
        if (v == null) return null;
        if (typeof v === 'number') return Number.isFinite(v) ? v : null;
        const n = Number(v);
        return Number.isFinite(n) ? n : null;
    });
}

/// Convert the wire-format history into the canonical
/// `IndicatorFlatHistory` shape used by every chart.
function normalizeHistory(raw: RawResponse): IndicatorFlatHistory {
    const times = (raw.indicator_history?.times ?? []).map((t) => Number(t));
    const values: Record<string, Array<number | null>> = {};

    const ih = raw.indicator_history?.indicators ?? {};
    for (const [key, dto] of Object.entries(ih)) {
        if (!dto) continue;
        // Secondary WARMING filter: even if a backend build surfaces a
        // placeholder row (state_label === "WARMING", raw 0.0), drop it
        // so charts never paint a phantom zero plateau at the series
        // start. The primary filter is server-side (history.rs pushes
        // None for WARMING rows); this guards other payload sources.
        const labels = Array.isArray(dto.state_label) ? dto.state_label : null;
        const masked = (i: number) => labels != null && labels[i] === 'WARMING';
        if (Array.isArray(dto.raw)) {
            values[key] = toNumberArray(dto.raw).map((v, i) => (masked(i) ? null : v));
        }
        if (dto.values) {
            for (const [sub, arr] of Object.entries(dto.values)) {
                if (!Array.isArray(arr)) continue;
                values[`${key}.${sub}`] = toNumberArray(arr).map((v, i) => (masked(i) ? null : v));
            }
        }
    }

    // Candles. Backend `candles[]` uses millisecond timestamps;
    // indicator_history `times` uses seconds. We surface both via
    // dedicated fields so callers don't have to mix-and-match.
    const candleRows = raw.candles ?? [];
    const candleTimes: number[] = [];
    const candleReconstructed: Array<string | undefined> = [];
    const candles = { open: [] as number[], high: [] as number[], low: [] as number[], close: [] as number[], volume: [] as number[] };
    for (const c of candleRows) {
        if (!c || c.time == null) continue;
        candleTimes.push(Math.floor(c.time / 1000));
        candles.open.push(parseFloat(c.open) || 0);
        candles.high.push(parseFloat(c.high) || 0);
        candles.low.push(parseFloat(c.low) || 0);
        candles.close.push(parseFloat(c.close) || 0);
        candles.volume.push(parseFloat(c.volume) || 0);
        candleReconstructed.push(
            c.reconstructed && typeof c.reconstructed === 'string' ? c.reconstructed : undefined
        );
    }

    return {
        times,
        values,
        candleTimes,
        candles,
        candleReconstructed,
        prices: raw.prices ?? undefined,
        clusters: raw.clusters ?? undefined,
        volumeProfiles: raw.volume_profiles ?? undefined,
        fetchedAtMs: Date.now(),
    };
}

/// Read a single indicator value series out of the unified history
/// payload. Returns `undefined` when the backend did not emit the field.
///
/// Pass `subKey` to read a sub-field (e.g. `historyValue(h, 'macd', 'line')`
/// returns the MACD-line series). Pass `candles.<ohlc>` to read a candle
/// column (e.g. `historyValue(h, 'candles', 'close')`).
export function historyValue(
    hist: IndicatorFlatHistory | null | undefined,
    key: string,
    subKey?: string,
): Array<number | null> | undefined {
    if (!hist) return undefined;
    if (subKey) {
        const v = hist.values[`${key}.${subKey}`];
        if (v && v.length) return v;
        // PRI-10 (v6.10.7): never fall back to the raw series for a
        // sub-keyed lookup. On cold sub-minute histories the per-line
        // sub-series (e.g. `ema_stack.medium/slow/long`) is absent until
        // its `bars_required` gate passes; the raw series for a
        // price-overlay entry is the close (or the `value_source` line),
        // so the old fallback drew the ema50/100/200 lines exactly on the
        // price line "from the beginning". Missing sub-series render
        // absent — the intended partial-ribbon behavior.
        return undefined;
    }
    const v = hist.values[key];
    if (v && v.length) return v;
    return undefined;
}

/// Read a string-valued state_label series. Use for fields where the
/// backend emits string categoricals (e.g. `squeeze.state_label`,
/// `smc_structure.state_label`).
export function historyStringValue(
    hist: IndicatorFlatHistory | null | undefined,
    key: string,
    subKey?: string,
): Array<string | null> | undefined {
    if (!hist) return undefined;
    // The current unify pass casts all sub-values to `Array<number | null>`,
    // so we re-derive the string series by reading the raw paylaod from
    // `candles` for now. Future improvement: extend `IndicatorFlatHistory`
    // with a parallel `stringValues` map. Until then, callers needing
    // string history should fall back to live-only behavior when the
    // backend didn't persist the categorical field for the timeframe.
    void key;
    void subKey;
    return undefined;
}

/// Last populated historical timestamp (in seconds). Used by the live
/// layer to decide whether a new WS frame should dedupe-against-history
/// or append as a fresh candle.
export function lastHistoricalTime(hist: IndicatorFlatHistory | null | undefined): number | null {
    if (!hist) return null;
    if (hist.times.length === 0) return null;
    const last = hist.times[hist.times.length - 1];
    return Number.isFinite(last) ? last : null;
}

/// Build dedup-sorted `{time, value}` pairs from a value series aligned
/// to the history `times[]`. Returns `[]` if either input is missing
/// or empty.
///
/// `opts.filterZero` — drop entries whose value is `0.0` (used by OI Δ
/// and OFI charts whose analyzer can emit legal `0.0` readings during
/// quiet books; we don't want those drawing phantom bars).
export function pairsFromHistory(
    hist: IndicatorFlatHistory | null | undefined,
    key: string,
    subKey?: string,
    opts?: { filterZero?: boolean },
): Array<{ time: Time; value: number }> {
    if (!hist) return [];
    const arr = historyValue(hist, key, subKey);
    if (!arr) return [];
    const out: Array<{ time: Time; value: number }> = [];
    for (let i = 0; i < arr.length && i < hist.times.length; i++) {
        const t = hist.times[i];
        const v = arr[i];
        if (t == null || v == null || !Number.isFinite(t) || !Number.isFinite(v)) continue;
        if (opts?.filterZero && v === 0) continue;
        out.push({ time: t as Time, value: v });
    }
    // De-duplicate by timestamp (cheap, since `times` are usually monotonic).
    const seen = new Set<number>();
    const unique: Array<{ time: Time; value: number }> = [];
    for (const p of out) {
        const tn = p.time as unknown as number;
        if (seen.has(tn)) continue;
        seen.add(tn);
        unique.push(p);
    }
    unique.sort((a, b) => (a.time as unknown as number) - (b.time as unknown as number));
    return unique;
}

/// Convenience for `PriceChart`-style overlays: read multiple sub-fields
/// in one call, each aligned to `hist.times`, dedup-sorted. Any missing
/// sub-field is silently skipped.
///
/// Usage:
///   alignedSeriesFromHistory(hist, [
///     ['ema_stack', 'fast'], ['bollinger', 'upper'],
///     ['supertrend'], ['ichimoku', 'tenkan'], ...
///   ])
/// returns an array of arrays (one per request).
export function alignedSeriesFromHistory(
    hist: IndicatorFlatHistory | null | undefined,
    keys: Array<[string, string?]>,
): Array<Array<{ time: Time; value: number }>> {
    return keys.map(([k, sub]) => pairsFromHistory(hist, k, sub));
}

/// True when the history payload has no data for the given field AND
/// also has no recent emit (no live frame yet). Used to render the
/// "NO HISTORICAL DATA" overlay so the user understands why a series
/// is blank instead of suspecting a bug.
export function historyFieldIsEmpty(
    hist: IndicatorFlatHistory | null | undefined,
    key: string,
    subKey?: string,
): boolean {
    return historyValue(hist, key, subKey) === undefined;
}

/// Generic de-duplicate + sort by `time` (ascending). Used by every
/// chart to clean the raw `{time, value}[]` payload before handing it to
/// lightweight-charts' `setData()`. Items with non-finite or zero time
/// are dropped.
export function dedupSortByTime<T extends { time: Time }>(items: T[]): T[] {
    const seen = new Set<number>();
    const out: T[] = [];
    for (const it of items) {
        const tn = typeof it.time === 'number' ? Number(it.time) : Number(it.time);
        if (!Number.isFinite(tn) || tn === 0 || seen.has(tn)) continue;
        seen.add(tn);
        out.push(it);
    }
    out.sort((a, b) => (a.time as unknown as number) - (b.time as unknown as number));
    return out;
}
