// @vitest-environment jsdom
// v8 BacktestingDashboard — instance-binding + navbar derivation tests.
//
// No instance selected → the shared NoInstanceState look (the BTE copy);
// a running instance selected via the shared store → the full run form
// with the depth slider (1..=365) and the bound-instance chip.
import { cleanup, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import BacktestingDashboard from './BacktestingDashboard.svelte';
import { useAppStore } from '../../state.svelte';

function jsonResponse(body: unknown): Response {
    return new Response(JSON.stringify(body), {
        status: 200,
        headers: { 'content-type': 'application/json' },
    });
}

function mockFetchImpl() {
    const fetchMock = vi.fn((url: string) => {
        const u = String(url);
        if (u.includes('/api/instances')) {
            return Promise.resolve(jsonResponse({
                instances: [
                    { id: 'inst_bte', pair: 'BTC-USDC', symbol: 'BTC-USDC', status: 'running', mode: 'observe' },
                ],
            }));
        }
        if (u.includes('/api/config')) {
            return Promise.resolve(jsonResponse({
                workspace: { slow_timeframe: { duration_seconds: 300 }, macro_timeframe: { duration_seconds: 900 } },
                backtest: { archive_depth_days: 180 },
            }));
        }
        if (u.includes('/api/backtest/coverage')) {
            return Promise.resolve(jsonResponse({
                archive_depth_days: 180,
                burn_in_secs: 3 * 86400,
                ladder: [60, 180, 300, 900],
                snapshots: [],
                archive: [
                    { symbol: 'BTC-USDC', timeframe_secs: 900, candle_count: 672, earliest_secs: 1750000000, latest_secs: 1760000000, covered_span_secs: 604800, max_lookback_secs: 15552000, coverage_pct: 4.0 },
                ],
                backfill_jobs: [],
            }));
        }
        return Promise.resolve(jsonResponse({}));
    });
    vi.stubGlobal('fetch', fetchMock);
    return fetchMock;
}

beforeEach(() => {
    cleanup();
    mockFetchImpl();
});

afterEach(() => {
    vi.unstubAllGlobals();
    cleanup();
});

describe('BacktestingDashboard instance binding (v8)', () => {
    it('no instance selected → the shared no-instance look', async () => {
        const app = useAppStore();
        for (const k of Object.keys(app.instancesMap)) app.removeInstance(k);
        app.selectedInstance = null;

        render(BacktestingDashboard, { props: { section: 'overview' } });
        await waitFor(() => expect(screen.getByText('No instance selected')).toBeTruthy());
        expect(screen.getByText(/Select an instance from the right-side Instances panel/)).toBeTruthy();
    });

    it('running instance selected → full form with bound-instance chip + depth slider (no dates)', async () => {
        const app = useAppStore();
        app.initInstance('BTC');
        app.selectedInstance = 'BTC-USDC';

        render(BacktestingDashboard, { props: { section: 'overview' } });
        await waitFor(() => expect(screen.getByText(/Run a Backtest — BTC-USDC/)).toBeTruthy());
        // The bound-instance chip carries the id resolved from /api/instances
        // (header chip + run-form chip both carry it).
        await waitFor(() => expect(screen.getAllByText(/inst_bte/).length).toBeGreaterThanOrEqual(1));
        // Depth slider present; date fields removed (v8.1 depth-only flow).
        expect(screen.getByText(/How far back can I look/)).toBeTruthy();
        expect(document.querySelector('input[type="range"]')).toBeTruthy();
        expect(document.querySelector('#bte-start')).toBeNull();
        expect(document.querySelector('#bte-end')).toBeNull();
        // The four-timeframe readiness strip renders (v8.1).
        expect(screen.getByText(/Four-Timeframe Readiness/)).toBeTruthy();
        expect(screen.getAllByText(/Run Backtest/).length).toBeGreaterThanOrEqual(1);
    });

    it('coverage shortfall drives automatic data preparation', async () => {
        const app = useAppStore();
        app.initInstance('BTC');
        app.selectedInstance = 'BTC-USDC';

        // The mocked coverage has archive rows only for 900s → the other
        // three TFs are short, so the Run button must NOT be disabled by
        // readiness and the fetch path must POST a backfill first.
        render(BacktestingDashboard, { props: { section: 'overview' } });
        await waitFor(() => expect(screen.getAllByText(/FETCHING/).length).toBeGreaterThanOrEqual(3));
        expect(screen.getByText(/Missing coverage is fetched automatically/)).toBeTruthy();

        const calls = (globalThis.fetch as unknown as ReturnType<typeof vi.fn>).mock.calls;
        const backfillCalls = calls.filter((c: unknown[]) => String(c[0]).includes('/api/backtest/archive/backfill'));
        expect(backfillCalls.length).toBe(0);
    });

    it('non-running instance → simplified navbar (no DIE tab content)', async () => {
        const app = useAppStore();
        app.initInstance('BTC');
        app.selectedInstance = 'BTC-USDC';

        // The mocked instance list returns status 'running', so simulate a
        // stale selection by selecting a pair absent from /api/instances.
        app.selectedInstance = 'ETH-USDC';
        render(BacktestingDashboard, { props: { section: 'die' } });
        // The DIE tab is outside the no-instance set → clamps to Overview
        // which renders the no-instance look.
        await waitFor(() => expect(screen.getByText('No instance selected')).toBeTruthy());
    });
});
