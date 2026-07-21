// Single-flight history fetch shared across all 36 mounted chart instances
// (4 timeframes × 9 charts). Coalesces concurrent requests into one HTTP call
// per (pairKey, timeframe) and serves cached data on subsequent mounts.
//
// Every timeframe (including sub-minute) fetches what the backend provides.
// Chart components handle empty / sparse responses gracefully — they render
// only what the bootstrap returned and let the live $effect accumulate the
// rest as candles close. This design is intentional: sub-minute timeframes
// have limited historical data but the WS stream fills them organically,
// and indicator values appear as each indicator chain warms from its
// lookback window.
//
// v6.5.0: the response now also carries per-TF `clusters` and
// `volumeProfiles` maps so the chart overlays (LIQ HEATMAP, VOL PROFILE)
// render on first-mount — before the WS stream has delivered a snapshot.

import type { Time } from 'lightweight-charts';
import type {
    LiquidationClusterMatrix,
    VolumeProfileSnapshot,
} from '../types';

export interface HistoryCandle {
    time: number;
    open: string;
    high: string;
    low: string;
    close: string;
    volume: string;
}

export interface FlatIndicatorHistory {
    times: number[];
    rsi_14: Array<string | null>;
    macd_line: Array<string | null>;
    macd_signal: Array<string | null>;
    macd_hist: Array<string | null>;
    adx_14: Array<string | null>;
    adx_plus: Array<string | null>;
    adx_minus: Array<string | null>;
    atr_14: Array<string | null>;
    bbwp: Array<string | null>;
    rvol: Array<string | null>;
    squeeze_momentum: Array<string | null>;
    squeeze_on: Array<boolean>;
    ema_fast: Array<string | null>;
    ema_medium: Array<string | null>;
    ema_slow: Array<string | null>;
    ema_long: Array<string | null>;
    bb_upper: Array<string | null>;
    bb_middle: Array<string | null>;
    bb_lower: Array<string | null>;
    vwap: Array<string | null>;
}

export interface HistoryResponse {
    prices: string[];
    candles: HistoryCandle[];
    indicatorHistory: FlatIndicatorHistory | null;
    /// v6.5: per-TF cluster matrices keyed by slot (`micro`/`fast`/`slow`/`macro`).
    /// `PriceChart` reads `clusters?.[slot]` if `tf.cluster` is not yet populated
    /// by the WS stream.
    clusters?: Partial<Record<string, LiquidationClusterMatrix>>;
    /// v6.5: per-TF volume profile snapshots keyed by slot.
    volumeProfiles?: Partial<Record<string, VolumeProfileSnapshot>>;
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

interface RawResponse {
    prices?: string[];
    candles?: HistoryCandle[];
    indicator_history?: RawHistory | null;
    clusters?: Partial<Record<string, LiquidationClusterMatrix>>;
    volume_profiles?: Partial<Record<string, VolumeProfileSnapshot>>;
}

function toStr(arr: Array<number | null>): Array<string | null> {
    return arr.map((v) => (v == null ? null : String(v)));
}

function flattenRaw(ih: RawHistory | undefined | null): FlatIndicatorHistory {
    const map = ih?.indicators ?? {};
    const raw = (k: string): Array<string | null> => toStr(map[k]?.raw ?? []);
    const val = (k: string, s: string): Array<string | null> => toStr(map[k]?.values?.[s] ?? []);
    const label = (k: string): Array<string | null> => map[k]?.state_label ?? [];
    const adxMain = val('adx', 'adx');

    return {
        times: ih?.times ?? [],
        rsi_14: raw('rsi'),
        macd_line: val('macd', 'line'),
        macd_signal: val('macd', 'signal'),
        macd_hist: val('macd', 'histogram'),
        adx_14: adxMain.length ? adxMain : raw('adx'),
        adx_plus: val('adx', 'plus_di'),
        adx_minus: val('adx', 'minus_di'),
        atr_14: raw('atr'),
        bbwp: raw('bbwp'),
        rvol: raw('rvol'),
        squeeze_momentum: raw('squeeze'),
        squeeze_on: label('squeeze').map((l) => l === 'COMPRESSION_COILING'),
        ema_fast: val('ema_stack', 'fast'),
        ema_medium: val('ema_stack', 'medium'),
        ema_slow: val('ema_stack', 'slow'),
        ema_long: val('ema_stack', 'long'),
        bb_upper: val('bollinger', 'upper'),
        bb_middle: val('bollinger', 'middle'),
        bb_lower: val('bollinger', 'lower'),
        vwap: val('vwap', 'vwap'),
    };
}

const cache = new Map<string, Promise<HistoryResponse | null>>();
const HISTORY_URL = '/api/history';

export function fetchChartHistoryOnce(
    pairKey: string,
    timeframe: number,
): Promise<HistoryResponse | null> {
    if (!pairKey || !timeframe) return Promise.resolve(null);

    const key = `${pairKey}@${timeframe}`;
    const cached = cache.get(key);
    if (cached) return cached;

    const p = (async () => {
        try {
            const res = await fetch(
                `${HISTORY_URL}?symbol=${encodeURIComponent(pairKey)}&timeframe_secs=${timeframe}&limit=1000`,
            );
            if (!res.ok) return null;
            const data: RawResponse = await res.json();
            return {
                prices: data.prices ?? [],
                candles: data.candles ?? [],
                indicatorHistory: data.indicator_history ? flattenRaw(data.indicator_history) : null,
                clusters: data.clusters ?? {},
                volumeProfiles: data.volume_profiles ?? {},
            };
        } catch (err) {
            console.error('chartHistory fetch failed', err);
            return null;
        }
    })();
    cache.set(key, p);
    return p;
}

export interface TimeKeyed {
    time: Time;
}

// De-duplicate + sort an array of `{time, ...}` entries by ascending time.
// Strips entries whose time is missing and ignores repeats so light-weight
// charts never sees out-of-order or zero-second duplicates.
export function dedupSortByTime<T extends TimeKeyed>(items: T[]): T[] {
    const seen = new Set<number>();
    const out: T[] = [];
    for (const it of items) {
        const t = typeof it.time === 'number' ? Number(it.time) : Number(it.time);
        if (!Number.isFinite(t) || t === 0 || seen.has(t)) continue;
        seen.add(t);
        out.push(it);
    }
    out.sort((a, b) => (a.time as number) - (b.time as number));
    return out;
}

export function clearHistoryCache(): void {
    cache.clear();
}
