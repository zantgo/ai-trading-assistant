// @vitest-environment jsdom
// v7 PerformanceDashboard — backtest tab regression lock.
//
// The backtest tab is live: the form POSTs /api/backtest/run and renders
// stat cards, the NHST edge verdict (α, p-values, Monte Carlo runs), the
// trade log, and the equity curve. No setTimeout mock.

import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import PerformanceDashboard from './PerformanceDashboard.svelte';
import { useAppStore } from '../state.svelte';

function jsonResponse(body: unknown): Response {
    return new Response(JSON.stringify(body), {
        status: 200,
        headers: { 'content-type': 'application/json' },
    });
}

const backtestResult = {
    backtest_id: 7,
    summary: {
        total_trades: 42,
        win_count: 26,
        loss_count: 16,
        win_rate: 61.9,
        gross_profit: 480.0,
        gross_loss: 210.0,
        profit_factor: 2.2857,
        expectancy: 6.43,
        max_drawdown_pct: 8.4,
    },
    stats: {
        setup_type: 'BACKTEST',
        alpha: 0.05,
        total_trades: 42,
        win_count: 26,
        loss_count: 16,
        win_rate: 61.9,
        gross_profit: 480.0,
        gross_loss: 210.0,
        profit_factor: 2.2857,
        average_win: 18.46,
        average_loss: -13.12,
        avg_win_loss_ratio: 1.41,
        expectancy: 6.43,
        slippage_overhead: 0.0,
        t_statistic: 2.91,
        p_value: 0.003,
        p_mc: 0.002,
        monte_carlo_runs: 10000,
        is_significant: true,
        classification: 'ModerateEdge',
    },
    trades: [
        { timestamp: 1700000000000, direction: 'LONG', entry_price: 30000, exit_price: 33000, size: 0.01, pnl: 30.0, exit_reason: 'tp' },
        { timestamp: 1700000100000, direction: 'LONG', entry_price: 30000, exit_price: 29500, size: 0.01, pnl: -5.0, exit_reason: 'sl' },
    ],
    equity_curve: [[1700000000000, 1000], [1700000100000, 1025], [1700000200000, 1020]],
};

function mockFetchImpl() {
    const fetchMock = vi.fn((url: string, opts?: RequestInit) => {
        if (typeof url === 'string' && url.includes('/api/backtest/run')) {
            return Promise.resolve(jsonResponse(backtestResult));
        }
        if (typeof url === 'string' && url.includes('/api/analytics/strategy')) {
            return Promise.resolve(jsonResponse([]));
        }
        if (typeof url === 'string' && url.includes('/api/analytics/risk')) {
            return Promise.resolve(jsonResponse({ maximum_drawdown_pct: 5.0 }));
        }
        if (typeof url === 'string' && url.includes('/api/analytics/performance')) {
            return Promise.resolve(jsonResponse([]));
        }
        if (typeof url === 'string' && url.includes('/api/analytics/optimization')) {
            return Promise.resolve(jsonResponse(null));
        }
        if (typeof url === 'string' && url.includes('/api/analytics/trades')) {
            return Promise.resolve(jsonResponse([]));
        }
        if (typeof url === 'string' && url.includes('/api/dashboard/stats')) {
            return Promise.resolve(jsonResponse({}));
        }
        return Promise.resolve(jsonResponse({}));
    });
    vi.stubGlobal('fetch', fetchMock);
    return fetchMock;
}

beforeEach(() => {
    cleanup();
    mockFetchImpl();
    // v7.3: no symbol fallback — the backtest form needs a real instance.
    useAppStore().initInstance('BTC');
});

afterEach(() => {
    vi.unstubAllGlobals();
    cleanup();
});

describe('PerformanceDashboard mode-aware framing (v7.3, v8)', () => {
    it('observe mode keeps the edge-validator overview + data-bearing tabs', async () => {
        const app = useAppStore();
        app.session.sessionMode = 'observe';
        render(PerformanceDashboard, { props: { section: 'overview' } });

        // Observe keeps a real Overview: the edge-validation strip plus
        // data coverage / latest verdict / significance summary.
        await waitFor(() => expect(screen.getByText('OBSERVE')).toBeTruthy());
        await waitFor(() => expect(screen.getByText('Edge Validation')).toBeTruthy());
        expect(screen.getByText('Data Coverage — Recorded Decisions')).toBeTruthy();
        expect(screen.getByText('Significance Summary')).toBeTruthy();
        // The drift / recorded-trade surfaces are not reachable in observe.
        expect(screen.queryByText('Forward Test')).toBeNull();
        expect(screen.queryByText('Trade Analytics')).toBeNull();
    });

    // v8: the Backtesting tab moved to the Backtesting Engine
    // (`ui/src/components/backtesting/`) — a stale `#/engine/performance/
    // backtesting` URL clamps to Overview instead of rendering a form.
    it('v8: a stale backtesting section clamps to Overview', async () => {
        const app = useAppStore();
        app.session.sessionMode = 'observe';
        render(PerformanceDashboard, { props: { section: 'backtesting' } });

        await waitFor(() => expect(screen.getByText('OBSERVE')).toBeTruthy());
        await waitFor(() => expect(screen.getByText('Edge Validation')).toBeTruthy());
        expect(document.querySelector('#bt-symbol')).toBeNull();
    });

    it('paper mode shows the forward-test drift card and full tabs', async () => {
        const app = useAppStore();
        app.session.sessionMode = 'paper';
        render(PerformanceDashboard, { props: { section: 'overview' } });

        await waitFor(() => expect(screen.getByText('PAPER')).toBeTruthy());
        await waitFor(() =>
            expect(screen.getByText(/Forward Test/)).toBeTruthy(),
        );
        expect(screen.getAllByText('Performance Overview').length).toBeGreaterThan(0);
    });
});

describe('PerformanceDashboard no-instance state (v7.3)', () => {
    it('renders the SVG empty state (no symbol fallback) when no instance is active', async () => {
        const app = useAppStore();
        for (const k of Object.keys(app.instancesMap)) app.removeInstance(k);
        cleanup();
        mockFetchImpl();
        app.session.sessionMode = 'observe';
        // v8: 'backtesting' is no longer a PAE tab — the clamp lands on
        // Overview which renders the same no-instance empty state.
        render(PerformanceDashboard, { props: { section: 'backtesting' } });

        await waitFor(() => expect(screen.getByText('No active instance')).toBeTruthy());
        expect(screen.getByText(/Performance analytics evaluate recorded decisions/)).toBeTruthy();
    });

    it('keeps the Settings tab rendered without an instance', async () => {
        const app = useAppStore();
        for (const k of Object.keys(app.instancesMap)) app.removeInstance(k);
        cleanup();
        mockFetchImpl();
        render(PerformanceDashboard, { props: { section: 'settings' } });

        await waitFor(() => expect(screen.getByText('Significance Treatment')).toBeTruthy());
        expect(screen.queryByText('No active instance')).toBeNull();
    });
});

