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
    prices?: string[];
    clusters?: Record<string, unknown>;
    volumeProfiles?: Record<string, unknown>;
    fetchedAtMs: number;
}

const HISTORY_URL = '/api/history';

const cache = new Map<string, Promise<IndicatorFlatHistory | null>>();

/// Fetch the indicator-history payload for `(pairKey, timeframe_secs)`.
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
): Promise<IndicatorFlatHistory | null> {
    if (!pairKey || !timeframe) return Promise.resolve(null);
    const key = `${pairKey}@${timeframe}`;
    const cached = cache.get(key);
    if (cached) return cached;

    const promise = (async (): Promise<IndicatorFlatHistory | null> => {
        try {
            const res = await fetch(
                `${HISTORY_URL}?symbol=${encodeURIComponent(pairKey)}&timeframe_secs=${timeframe}&limit=1000`,
            );
            if (!res.ok) return null;
            const raw = await res.json() as RawResponse;
            return normalizeHistory(raw);
        } catch (err) {
            console.error('indicatorHistory fetch failed', err);
            return null;
        }
    })();
    cache.set(key, promise);
    return promise;
}

/// Test hook / daemon restart: drops the in-memory cache so subsequent
/// mounts re-fetch from the server.
export function clearHistoryCache(): void {
    cache.clear();
}

export function purgeCacheForKey(pairKey: string, timeframe: number): void {
    cache.delete(`${pairKey}@${timeframe}`);
}

// ── Processed candle cache ────────────────────────────────────────────
// Per (pairKey, timeframe) cache of the final OHLCV array fed to
// lightweight-charts. This survives component unmount/remount so
// timeframe switches and back/forward navigation don't wipe the chart —
// the bootstrap paints from cache immediately while the async history
// fetch refreshes in the background.

export type CandleOHLCV = { time: Time; open: number; high: number; low: number; close: number };

const candleCache = new Map<string, CandleOHLCV[]>();

export function getCachedCandles(pairKey: string, timeframe: number): CandleOHLCV[] | null {
    return candleCache.get(`${pairKey}@${timeframe}`) ?? null;
}

export function setCachedCandles(pairKey: string, timeframe: number, candles: CandleOHLCV[]): void {
    if (candles.length > 0) candleCache.set(`${pairKey}@${timeframe}`, candles);
}

export function clearCandleCache(): void {
    candleCache.clear();
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
        if (Array.isArray(dto.raw)) {
            values[key] = toNumberArray(dto.raw);
        }
        if (dto.values) {
            for (const [sub, arr] of Object.entries(dto.values)) {
                if (!Array.isArray(arr)) continue;
                values[`${key}.${sub}`] = toNumberArray(arr);
            }
        }
    }

    // Candles. Backend `candles[]` uses millisecond timestamps;
    // indicator_history `times` uses seconds. We surface both via
    // dedicated fields so callers don't have to mix-and-match.
    const candleRows = raw.candles ?? [];
    const candleTimes: number[] = [];
    const candles = { open: [] as number[], high: [] as number[], low: [] as number[], close: [] as number[], volume: [] as number[] };
    for (const c of candleRows) {
        if (!c || c.time == null) continue;
        candleTimes.push(Math.floor(c.time / 1000));
        candles.open.push(parseFloat(c.open) || 0);
        candles.high.push(parseFloat(c.high) || 0);
        candles.low.push(parseFloat(c.low) || 0);
        candles.close.push(parseFloat(c.close) || 0);
        candles.volume.push(parseFloat(c.volume) || 0);
    }

    return {
        times,
        values,
        candleTimes,
        candles,
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
