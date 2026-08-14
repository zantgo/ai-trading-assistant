// @vitest-environment jsdom
import { describe, it, expect } from 'vitest';
import type { Time } from 'lightweight-charts';
import {
    fetchIndicatorHistoryOnce,
    clearHistoryCache,
    clearCandleCache,
    setCachedCandles,
    getCachedCandles,
    buildPaintCandles,
    dedupSortByTime,
    type CandleOHLCV,
} from '../lib/indicatorHistory';

describe('indicatorHistory (unified)', () => {
    it('fetchIndicatorHistoryOnce_allows_sub_minute_timeframes', async () => {
        // Regression: sub-minute timeframes used to return `null`
        // immediately, which prevented every chart's `setData()` from
        // running on bootstrap. Now the helper always fetches and the
        // charts handle empty / sparse responses naturally.
        //
        // The inline mock does NOT provide a response body so the fetch
        // will fail with a JSON parse error, but the critical assertion
        // is that the function does NOT return `null` — it actually
        // attempts a network request.
        const originalFetch = globalThis.fetch;
        let reqUrl = '';
        globalThis.fetch = async (input: RequestInfo | URL, _init?: RequestInit) => {
            reqUrl = typeof input === 'string' ? input : String(input);
            return new Response('', { status: 404 });
        };
        try {
            const p = fetchIndicatorHistoryOnce('BTC-USDT', 1);
            const result = await p;
            // 404 returns null from the helper because !res.ok.
            expect(result).toBeNull();
            // But the URL was formed correctly and the request was made.
            expect(reqUrl).toContain('timeframe_secs=1');
            expect(reqUrl).toContain('BTC-USDT');
        } finally {
            clearHistoryCache();
            globalThis.fetch = originalFetch;
        }
    });

    it('fetchIndicatorHistoryOnce_returns_null_for_zero_timeframe', async () => {
        expect(await fetchIndicatorHistoryOnce('BTC-USDT', 0)).toBeNull();
    });

    it('fetchIndicatorHistoryOnce_returns_null_for_empty_pairKey', async () => {
        expect(await fetchIndicatorHistoryOnce('', 60)).toBeNull();
    });

    it('clearHistoryCache_is_noop_on_empty_cache', () => {
        expect(() => clearHistoryCache()).not.toThrow();
    });

    it('dedupSortByTime_sorts_and_removes_duplicates', () => {
        const input = [
            { time: 3 as import('lightweight-charts').Time, value: 30 },
            { time: 1 as import('lightweight-charts').Time, value: 10 },
            { time: 3 as import('lightweight-charts').Time, value: 31 }, // duplicate
            { time: 2 as import('lightweight-charts').Time, value: 20 },
        ];
        const result = dedupSortByTime(input);
        expect(result).toEqual([
            { time: 1 as import('lightweight-charts').Time, value: 10 },
            { time: 2 as import('lightweight-charts').Time, value: 20 },
            { time: 3 as import('lightweight-charts').Time, value: 30 },
        ]);
    });

    it('setCachedCandles_filters_synthetic_candles', () => {
        // Regression: the persistent candle cache must never hold
        // synthetic (SYNTHETIC) candles. They render as flat horizontal
        // lines that span the entire source interval, which on a
        // sub-minute chart looks like a "line of about 1 minute" — the
        // v6.9 "derived from minute-close" render artefact. The fix is
        // a defence-in-depth filter at the cache boundary: any candle
        // carrying a truthy `reconstructed` flag is dropped before
        // being cached, even if the caller forgot to filter.
        const pairKey = 'TEST-SYN-FILTER';
        const timeframe = 3;
        try {
            const realCandle: CandleOHLCV = { time: 1 as Time, open: 100, high: 101, low: 99, close: 100.5 };
            const syntheticCandle: CandleOHLCV = {
                time: 2 as Time, open: 100, high: 100, low: 100, close: 100,
                reconstructed: 'SYNTHETIC',
            };
            const anotherReal: CandleOHLCV = { time: 3 as Time, open: 101, high: 102, low: 100.5, close: 101.5 };
            setCachedCandles(pairKey, timeframe, [realCandle, syntheticCandle, anotherReal]);
            const cached = getCachedCandles(pairKey, timeframe);
            expect(cached).not.toBeNull();
            expect(cached!.length).toBe(2);
            expect(cached!.map((c) => c.time)).toEqual([1, 3]);
            for (const c of cached!) {
                expect(c.reconstructed).toBeUndefined();
            }
        } finally {
            clearCandleCache();
        }
    });

    it('setCachedCandles_filters_PRICE_FALLBACK_synthetic_marker', () => {
        // The chart's prices-only fallback path also tags its entries
        // so they don't pollute the cache across navigation.
        const pairKey = 'TEST-FALLBACK';
        const timeframe = 5;
        try {
            setCachedCandles(pairKey, timeframe, [
                { time: 10 as Time, open: 50, high: 50, low: 50, close: 50, reconstructed: 'PRICE_FALLBACK' },
                { time: 20 as Time, open: 51, high: 51, low: 51, close: 51, reconstructed: 'PRICE_FALLBACK' },
            ]);
            const cached = getCachedCandles(pairKey, timeframe);
            // All entries are synthetic — cache should be empty.
            expect(cached).toBeNull();
        } finally {
            clearCandleCache();
        }
    });

    it('setCachedCandles_preserves_real_candles_unchanged', () => {
        const pairKey = 'TEST-REAL';
        const timeframe = 60;
        try {
            const candles: CandleOHLCV[] = [
                { time: 1 as Time, open: 100, high: 101, low: 99, close: 100.5 },
                { time: 61 as Time, open: 100.5, high: 102, low: 100, close: 101.5 },
            ];
            setCachedCandles(pairKey, timeframe, candles);
            const cached = getCachedCandles(pairKey, timeframe);
            expect(cached).toEqual(candles);
        } finally {
            clearCandleCache();
        }
    });

    it('buildPaintCandles_preserves_synthetic_doji_candles_for_painting', () => {
        // Regression (F5 history wipe): the PriceChart paint path used to
        // filter out every candle the backend marked `reconstructed`.
        // On sparse sub-minute markets those SYNTHETIC doji-fill candles
        // are the majority of the in-memory history buffer, so after a
        // full page reload (empty candle cache → history comes only from
        // `/api/history`) the price chart lost ~90% of its history and
        // rendered long flat bridges instead of the continuous series the
        // live WebSocket coalescer draws. The paint helper must hand the
        // full gap-filled series to the chart; only the persistent cache
        // (`setCachedCandles`) may drop synthetic candles.
        const candles: CandleOHLCV[] = [
            { time: 100 as Time, open: 100, high: 101, low: 99, close: 100.5 },
            { time: 103 as Time, open: 100.5, high: 102, low: 100, close: 101.5, reconstructed: 'SYNTHETIC' },
        ];
        try {
            const painted = buildPaintCandles(candles, 1);
            // The backend-marked doji survives the paint path (and the
            // helper's own gap-fill bridges the 101/102 buckets).
            expect(painted.some((c) => c.reconstructed === 'SYNTHETIC')).toBe(true);
            expect(painted.length).toBe(4);
            // The cache path still filters the synthetic candles out.
            setCachedCandles('TEST-PAINT', 1, painted);
            const cached = getCachedCandles('TEST-PAINT', 1);
            expect(cached).not.toBeNull();
            for (const c of cached!) {
                expect(c.reconstructed).toBeUndefined();
            }
        } finally {
            clearCandleCache();
        }
    });
});
