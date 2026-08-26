// @vitest-environment jsdom
import { describe, expect, it, beforeEach, vi } from 'vitest';
import { clearHistoryCache, fetchIndicatorHistoryOnce, getResolvedHistory, ingestLiveSnapshot } from './indicatorHistory';

function makeSnap(ts: number, close: number): Record<string, unknown> {
    return {
        timestamp: ts,
        close,
        open: close,
        high: close,
        low: close,
        volume: 10,
        is_completed: true,
        indicators: { rsi: { raw_value: 50, normalized: 0, state_label: 'Live', values: null } },
    };
}

describe('indicatorHistory race — fetch vs live tail (sub-minute)', () => {
    beforeEach(() => {
        clearHistoryCache();
        vi.restoreAllMocks();
    });

    it('live tail survives when server fetch returns empty after live was primed (cold 1s)', async () => {
        const pair = 'BTC-USDT';
        const slot = 'micro';
        const tf = 1;
        // Mock fetch to delay 30ms and return empty history (cold sub-minute server has 0 candles)
        const fetchMock = vi.fn(async () => {
            await new Promise((r) => setTimeout(r, 30));
            return {
                ok: true,
                json: async () => ({
                    candles: [],
                    indicator_history: { times: [], indicators: {} },
                    clusters: {},
                    volume_profiles: {},
                }),
            } as unknown as Response;
        });
        vi.stubGlobal('fetch', fetchMock as unknown as typeof fetch);

        // Start fetch (cold) — will be pending
        const p = fetchIndicatorHistoryOnce(pair, tf, slot);

        // While fetch pending, live WS delivers 3 completed 1s candles
        ingestLiveSnapshot(pair, tf, slot, makeSnap(1_700_000_000, 50000));
        ingestLiveSnapshot(pair, tf, slot, makeSnap(1_700_000_001, 50001));
        ingestLiveSnapshot(pair, tf, slot, makeSnap(1_700_000_002, 50002));

        const liveBefore = getResolvedHistory(pair, tf, slot);
        expect(liveBefore?.times.length).toBe(3);

        // Now fetch resolves (empty server)
        const serverHist = await p;

        // With the fix, live tail must NOT be clobbered by empty server
        const after = getResolvedHistory(pair, tf, slot);
        expect(after?.times.length).toBe(3);
        expect(after?.candleTimes.length).toBe(3);
        expect(after?.times[0]).toBe(1_700_000_000);
        // With fix, fetch returns the preserved live (3) instead of empty (0)
        // — without fix, after would be 0 (live clobbered)
        expect(serverHist?.times.length ?? 0).toBe(3);
        expect(after?.times.length).toBe(3);
    });

    it('live tail newer than server is merged, not overwritten', async () => {
        const pair = 'ETH-USDT';
        const slot = 'micro';
        const tf = 60;
        // Pre-seed live with 5 candles at t=1000..1004
        for (let i = 0; i < 5; i++) ingestLiveSnapshot(pair, tf, slot, makeSnap(1000 + i, 50000 + i));
        expect(getResolvedHistory(pair, tf, slot)?.times.length).toBe(5);

        // Mock server fetch that returns only first 3 (stale, behind live)
        const fetchMock = vi.fn(async () => ({
            ok: true,
            json: async () => ({
                candles: [1000,1001,1002].map((t) => ({ time: t*1000, open:'50000', high:'50000', low:'50000', close:'50000', volume:'1' })),
                indicator_history: { times: [1000,1001,1002], indicators: { rsi: { raw:[50,51,52], state_label:['Live','Live','Live'] } } },
            }),
        } as unknown as Response));
        vi.stubGlobal('fetch', fetchMock as unknown as typeof fetch);

        // Need to clear cache to force new fetch? But historyData already has live and cache has promise resolving to live.
        // To simulate stale fetch racing live, we need to purge then re-fetch.
        // Purge and then start fetch racy with new live tail? Instead test mergeHistoryRefresh directly via fetch path:
        // Clear then re-create scenario where fetch starts before new live tail
        clearHistoryCache();
        // Start with live 3
        for (let i=0;i<3;i++) ingestLiveSnapshot(pair, tf, slot, makeSnap(1000+i, 50000+i));
        // Now start fetch that will return 2 (even more stale) while we add 2 more live before it resolves
        const slowFetch = vi.fn(async () => {
            await new Promise((r)=>setTimeout(r,20));
            return {
                ok:true,
                json: async()=>({
                    candles: [1000,1001].map((t)=>({time:t*1000, open:'50000', high:'50000', low:'50000', close:'50000', volume:'1'})),
                    indicator_history:{times:[1000,1001], indicators:{rsi:{raw:[50,51], state_label:['Live','Live']}}},
                }),
            } as unknown as Response;
        });
        vi.stubGlobal('fetch', slowFetch as unknown as typeof fetch);
        const p2 = fetchIndicatorHistoryOnce(pair, tf, slot);
        // Add live tail newer than server while fetch pending
        ingestLiveSnapshot(pair, tf, slot, makeSnap(1003, 50003));
        ingestLiveSnapshot(pair, tf, slot, makeSnap(1004, 50004));
        await p2;
        const after = getResolvedHistory(pair, tf, slot);
        // Live tail 1003,1004 plus server 1000,1001 => merged should have 1000,1001,1002? Actually live already had 1000,1001,1002, plus new 1003,1004 = 5
        // Server stale 1000,1001 should be merged with tail 1002,1003,1004 preserved
        expect(after!.times.length).toBeGreaterThanOrEqual(4);
        expect(after!.times).toContain(1003);
        expect(after!.times).toContain(1004);
    });
});
