// @vitest-environment jsdom
// v7 PortfolioDashboard — live-data regression lock.
//
// The dashboard is fully API-driven (no placeholder arrays): it lists
// instances, polls `/api/instances/:id/portfolio` + `/api/instances/:id/safety`,
// and renders the five panels (Overview / Positions / Exposure / Capital /
// Safety) with informational resets only.

import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import PortfolioDashboard from './PortfolioDashboard.svelte';

function jsonResponse(body: unknown): Response {
    return new Response(JSON.stringify(body), {
        status: 200,
        headers: { 'content-type': 'application/json' },
    });
}

const portfolioPayload = {
    instance_id: 'inst_btc',
    symbol: 'BTC-USDC',
    initial_capital: 1000,
    current_equity: '1012.40',
    peak_equity: '1015.00',
    max_drawdown_pct: '1.20',
    realized_pnl: '12.40',
    unrealized_pnl: '3.10',
    daily_pnl: '8.75',
    starting_session_equity: '1003.65',
    safety_state: 'NORMAL',
    safety_context: 'Normal risk mode.',
    consecutive_losses: { 'BTC-USDC': 0 },
    systemic_risk_score: 21.4,
    lifecycle: 'RUNNING',
    exposure: {
        gross_exposure: '310.00',
        net_exposure: '310.00',
        net_exposure_pct: '30.6',
        long_exposure: '310.00',
        short_exposure: '0.00',
        symbol_concentration: { 'BTC-USDC': '0.31' },
        max_single_pair_pct: '0.20',
    },
    capital: {
        available_margin: '1006.80',
        committed_margin: '15.50',
        margin_usage_ratio: '0.015',
        leverage_ratio: '0.31',
        margin_alert: null,
    },
    position_count: 1,
    positions: [
        {
            symbol: 'BTC-USDC',
            direction: 'LONG',
            size: '0.0100',
            entry_price: '30000.00',
            mark_price: '31000.00',
            unrealized_pnl: '10.00',
            roi_pct: '3.33',
            stop_loss_price: '29500.00',
            take_profit_price: '33000.00',
        },
    ],
};

const safetyPayload = {
    instance_id: 'inst_btc',
    safety_state: 'NORMAL',
    consecutive_losses: { 'BTC-USDC': 0 },
    peak_equity: '1015.00',
    current_equity: 1012.4,
    initial_capital: 1000,
    context: 'Normal risk mode.',
    daily_pnl: '8.75',
    max_drawdown_pct: '1.20',
    margin_usage_ratio: '0.015',
};

function mockFetchImpl() {
    const fetchMock = vi.fn((url: string, opts?: RequestInit) => {
        if (url === '/api/instances') {
            return Promise.resolve(
                jsonResponse({ instances: [{ id: 'inst_btc', pair: 'BTC-USDC', status: 'running' }] }),
            );
        }
        if (url.includes('/portfolio')) {
            return Promise.resolve(jsonResponse(portfolioPayload));
        }
        if (url.includes('/safety/session-reset')) {
            return Promise.resolve(jsonResponse({ success: true }));
        }
        if (url.includes('/safety')) {
            return Promise.resolve(jsonResponse(safetyPayload));
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

describe('PortfolioDashboard (v7 live)', () => {
    it('renders the overview stats from the API', async () => {
        render(PortfolioDashboard);

        await waitFor(() => expect(screen.getByText('PORTFOLIO')).toBeTruthy());
        await waitFor(() => expect(screen.getByText('Account Overview')).toBeTruthy());
        expect(screen.getByText('NORMAL')).toBeTruthy();
        expect(screen.getByText('Peak Equity')).toBeTruthy();
        expect(screen.getByText('Daily PnL')).toBeTruthy();
        expect(screen.getByText('Systemic Risk')).toBeTruthy();
    });

    it('renders the positions panel', async () => {
        render(PortfolioDashboard, { props: { section: 'positions' } });

        await waitFor(() => expect(screen.getByText('Positions')).toBeTruthy());

        expect(screen.getByText('BTC-USDC')).toBeTruthy();
        expect(screen.getByText('LONG')).toBeTruthy();
    });

    it('renders the exposure panel with concentration', async () => {
        render(PortfolioDashboard, { props: { section: 'exposure' } });

        await waitFor(() => expect(screen.getByText('Gross Exposure')).toBeTruthy());
        expect(screen.getByText('Symbol Concentration')).toBeTruthy();
    });

    it('renders the capital panel', async () => {
        render(PortfolioDashboard, { props: { section: 'capital' } });

        await waitFor(() => expect(screen.getByText('Available Margin')).toBeTruthy());
        expect(screen.getByText('Margin Usage')).toBeTruthy();
    });

    it('renders the safety panel and POSTs session-reset', async () => {
        render(PortfolioDashboard, { props: { section: 'safety' } });

        await waitFor(() => expect(screen.getByText('Normal risk mode.')).toBeTruthy());
        expect(screen.getByText('Consecutive Losses')).toBeTruthy();

        await fireEvent.click(screen.getByText(/Reset session/));
        const calls = (globalThis.fetch as unknown as ReturnType<typeof vi.fn>).mock.calls;
        const resetCall = calls.find((c: unknown[]) =>
            String(c[0]).includes('/safety/session-reset'),
        );
        expect(resetCall).toBeTruthy();
        expect((resetCall as unknown[])[1]).toMatchObject({ method: 'POST' });
    });

    it('shows the informational PME explainer', async () => {
        render(PortfolioDashboard);

        await waitFor(() =>
            expect(screen.getByText(/It never executes/)).toBeTruthy(),
        );
    });
});
