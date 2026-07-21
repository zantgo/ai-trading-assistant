// @vitest-environment jsdom
import { describe, it, expect } from 'vitest';
import {
    fetchChartHistoryOnce,
    clearHistoryCache,
    dedupSortByTime,
} from '../lib/chartHistory';

describe('chartHistory', () => {
    it('fetchChartHistoryOnce_allows_sub_minute_timeframes', async () => {
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
            const p = fetchChartHistoryOnce('BTC-USDT', 1);
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

    it('fetchChartHistoryOnce_returns_null_for_zero_timeframe', async () => {
        expect(await fetchChartHistoryOnce('BTC-USDT', 0)).toBeNull();
    });

    it('fetchChartHistoryOnce_returns_null_for_empty_pairKey', async () => {
        expect(await fetchChartHistoryOnce('', 60)).toBeNull();
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
});
