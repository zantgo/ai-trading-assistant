// @vitest-environment jsdom
// v7 PerformanceDashboard — backtest tab regression lock.
//
// The backtest tab is live: the form POSTs /api/backtest/run and renders
// stat cards, the NHST edge verdict (α, p-values, Monte Carlo runs), the
// trade log, and the equity curve. No setTimeout mock.

import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import PerformanceDashboard from './PerformanceDashboard.svelte';

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
});

afterEach(() => {
    vi.unstubAllGlobals();
    cleanup();
});

describe('PerformanceDashboard backtest tab (v7 live)', () => {
    it('runs a backtest and renders stats + edge verdict + trades + equity curve', async () => {
        render(PerformanceDashboard);

        await waitFor(() => expect(screen.getByText('Backtesting')).toBeTruthy());
        await fireEvent.click(screen.getByText('Backtesting'));

        // Real form fields.
        await waitFor(() => expect(document.querySelector('#bt-symbol')).toBeTruthy());
        expect(document.querySelector('#bt-tf')).toBeTruthy();
        expect(document.querySelector('#bt-start')).toBeTruthy();
        expect(document.querySelector('#bt-end')).toBeTruthy();
        expect(document.querySelector('#bt-capital')).toBeTruthy();

        // Submit the backtest.
        await waitFor(() => expect(screen.getByText('Run Backtest')).toBeTruthy());
        await fireEvent.click(screen.getByText('Run Backtest'));

        // Results: summary stat cards + edge verdict + trade log + curve.
        await waitFor(() => expect(screen.getByText('EDGE VERDICT')).toBeTruthy());
        expect(screen.getByText('42')).toBeTruthy();
        expect(screen.getByText(/Moderate Edge/)).toBeTruthy();
        expect(screen.getByText(/significant at α = 0.05/)).toBeTruthy();
        expect(screen.getByText(/[0-9,]+ runs/)).toBeTruthy();
        expect(screen.getByText('Trade Log')).toBeTruthy();
        expect(screen.getByText('Equity Curve')).toBeTruthy();

        // POST was issued with the right body shape.
        const calls = (globalThis.fetch as unknown as ReturnType<typeof vi.fn>).mock.calls;
        const runCall = calls.find((c: unknown[]) => String(c[0]).includes('/api/backtest/run'));
        expect(runCall).toBeTruthy();
        const opts = runCall?.[1] as RequestInit | undefined;
        const body = JSON.parse(String(opts?.body ?? '{}'));
        expect(body.symbol).toBeTruthy();
        expect(body.timeframe_secs).toBeTruthy();
        expect(body.initial_capital).toBeTruthy();
    });
});
