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

describe('BacktestingDashboard instance binding (v8.2)', () => {
    it('no instance selected → the launcher wizard renders (standalone)', async () => {
        const app = useAppStore();
        for (const k of Object.keys(app.instancesMap)) app.removeInstance(k);
        app.selectedInstance = null;

        render(BacktestingDashboard, { props: { section: 'overview' } });
        // The installer-style launcher: step 1 = Environment.
        await waitFor(() => expect(screen.getAllByText('Environment').length).toBeGreaterThanOrEqual(1));
        expect(screen.getByText('Continue')).toBeTruthy();
        expect(screen.getByText('STANDALONE')).toBeTruthy();
    });

    it('running instance selected → launcher preseeded with the bound chip', async () => {
        const app = useAppStore();
        app.initInstance('BTC');
        app.selectedInstance = 'BTC-USDC';

        render(BacktestingDashboard, { props: { section: 'overview' } });
        await waitFor(() => expect(screen.getAllByText(/inst_bte/).length).toBeGreaterThanOrEqual(1));
        // The wizard renders; no date pickers exist (depth-only window).
        expect(screen.getAllByText(/Environment/).length).toBeGreaterThanOrEqual(1);
        expect(document.querySelector('#bte-start')).toBeNull();
        expect(document.querySelector('#bte-end')).toBeNull();
    });

    it('stale selection → launcher still renders (standalone backtests)', async () => {
        const app = useAppStore();
        app.initInstance('BTC');
        app.selectedInstance = 'ETH-USDC';

        render(BacktestingDashboard, { props: { section: 'die' } });
        // The DIE tab is outside the no-instance set → clamps to Overview,
        // which renders the launcher regardless of the stale selection.
        await waitFor(() => expect(screen.getAllByText('Environment').length).toBeGreaterThanOrEqual(1));
    });
});
