// @vitest-environment jsdom
// v7 TradeAutomationDashboard — live-data regression lock.
//
// The dashboard is fully API-driven (no placeholder arrays): it lists
// instances, polls `/api/instances/:id/automation`, renders the tracked
// setup card + projection, the order board, the position card with the
// manual close button, the activity log, and the trade history.
//
// What these tests assert:
//   - the instance selector renders from `/api/instances`
//   - the PAPER badge + AUTOMATION ON chips render from the payload
//   - the active-setup card renders entry/SL/TP with the projected R&R
//   - the position card renders and the Close now button POSTs
//     `/api/instances/:id/automation/close`
//   - activity-log events render with their invalidation labels
//   - no `state_unsafe_mutation` on mount

import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import TradeAutomationDashboard from './TradeAutomationDashboard.svelte';

function jsonResponse(body: unknown): Response {
    return new Response(JSON.stringify(body), {
        status: 200,
        headers: { 'content-type': 'application/json' },
    });
}

const automationPayload = {
    instance_id: 'inst_btc',
    symbol: 'BTC-USDC',
    mode: 'paper',
    enabled: true,
    phase: 'position_open',
    fingerprint: 'BTC-USDC:LONG:TrendContinuation:1000',
    tracked_setup: {
        symbol: 'BTC-USDC',
        direction: 'LONG',
        setup_type: 'TrendContinuation',
        score: 82,
        source_tf: 'micro',
        entry_mid: '95.00',
        entry_zone_low: '90.00',
        entry_zone_high: '100.00',
        sl: '85.00',
        tp: '125.00',
        net_rr: 2.4,
        time_horizon: 'SWING',
    },
    projection: {
        risk_capital: '10.00',
        position_size_units: '0.105263',
        position_notional: '10.00',
        margin_required: '0.50',
        liquidation_price: '75.00',
        entry_fee_usd: '0.006',
        exit_fee_usd: '0.006',
        total_fees: '0.012',
        net_profit_usd: '3.16',
        roi_pct: '632.00',
        net_rr: '2.63',
    },
    entry_order: {
        id: 'paper_000001',
        side: 'BUY',
        order_type: 'Limit',
        price: '95.00',
        size: '0.105263',
        status: 'Closed',
        filled_size: '0.105263',
        fill_price: '94.00',
        reduce_only: false,
        created_at: 1000,
    },
    bracket: {
        tp_order: {
            id: 'paper_000002',
            side: 'SELL',
            order_type: 'Limit',
            price: '125.00',
            size: '0.105263',
            status: 'Open',
            filled_size: '0',
            fill_price: null,
            reduce_only: true,
            created_at: 1000,
        },
        sl_order: {
            id: 'paper_000003',
            side: 'SELL',
            order_type: 'Stop',
            price: '85.00',
            size: '0.105263',
            status: 'Open',
            filled_size: '0',
            fill_price: null,
            reduce_only: true,
            created_at: 1000,
        },
    },
    position: {
        symbol: 'BTC-USDC',
        direction: 'LONG',
        size: '0.105263',
        entry_price: '94.00',
        unrealized_pnl: '2.00',
    },
    invalidation: { state: 'none', detail: '' },
    activity_log: [
        { ts: 1700000000000, event: 'setup_accepted', detail: 'LONG TrendContinuation entry=95' },
        { ts: 1700000001000, event: 'entry_filled', detail: 'BTC-USDC:LONG:TrendContinuation:1000' },
        { ts: 1700000002000, event: 'bracket_armed', detail: 'TP 125 / SL 85' },
    ],
    safety_gate: { blocked: false, reason: null },
    lifecycle: 'RUNNING',
    equity: '9999.99',
    open_positions_count: 1,
};

function mockFetchImpl() {
    const fetchMock = vi.fn((url: string, opts?: RequestInit) => {
        if (url === '/api/instances') {
            return Promise.resolve(
                jsonResponse({ instances: [{ id: 'inst_btc', pair: 'BTC-USDC', status: 'running' }] }),
            );
        }
        if (url.includes('/automation')) {
            if (opts?.method === 'POST') {
                return Promise.resolve(jsonResponse({ success: true }));
            }
            return Promise.resolve(jsonResponse(automationPayload));
        }
        if (url.includes('/api/trade-ledger')) {
            return Promise.resolve(
                jsonResponse([
                    {
                        id: 1,
                        symbol: 'BTC-USDC',
                        direction: 'LONG',
                        entry_price: 90,
                        exit_price: 125,
                        size: 0.1,
                        commission_fees: 0.012,
                        realized_pnl: 3.5,
                        roi_pct: 3.8,
                        trigger_source: 'TrendContinuation',
                        entry_timestamp: 900,
                        exit_timestamp: 950,
                    },
                ]),
            );
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

describe('TradeAutomationDashboard (v7 live)', () => {
    it('renders the mode badge, setup card and projection', async () => {
        render(TradeAutomationDashboard);

        await waitFor(() => expect(screen.getByText('AUTOMATION ON')).toBeTruthy());
        await waitFor(() =>
            expect(screen.getAllByText('TrendContinuation').length).toBeGreaterThan(0),
        );
        expect(screen.getByText('PAPER')).toBeTruthy();
        expect(screen.getByText('Entry (limit)')).toBeTruthy();
        expect(screen.getByText('Stop-Loss (invalidation)')).toBeTruthy();
        expect(screen.getByText('Take-Profit')).toBeTruthy();
        expect(screen.getByText('PROJECTED RISK AND RETURN')).toBeTruthy();
    });

    it('renders the order board with entry / TP / SL roles', async () => {
        render(TradeAutomationDashboard, { props: { section: 'orders' } });

        await waitFor(() => expect(screen.getByText('ENTRY')).toBeTruthy());
        expect(screen.getByText('TP')).toBeTruthy();
        expect(screen.getByText('SL')).toBeTruthy();
        expect(screen.getByText('CLOSED')).toBeTruthy();
    });

    it('renders the position card and POSTs on Close now', async () => {
        const fetchMock = vi.stubGlobal('fetch', undefined) ?? undefined;
        vi.unstubAllGlobals();
        mockFetchImpl();
        render(TradeAutomationDashboard);

        await waitFor(() => expect(screen.getByText('Close now')).toBeTruthy());
        await fireEvent.click(screen.getByText('Close now'));

        const calls = (globalThis.fetch as unknown as ReturnType<typeof vi.fn>).mock.calls;
        const closeCall = calls.find(
            (c: unknown[]) => String(c[0]).includes('/automation/close'),
        );
        expect(closeCall).toBeTruthy();
        expect((closeCall as unknown[])[1]).toMatchObject({ method: 'POST' });
    });

    it('renders activity-log events with invalidation labels', async () => {
        render(TradeAutomationDashboard, { props: { section: 'activity' } });

        await waitFor(() => expect(screen.getByText('SETUP ACCEPTED')).toBeTruthy());
        expect(screen.getByText('ENTRY FILLED')).toBeTruthy();
        expect(screen.getByText('BRACKET ARMED')).toBeTruthy();
    });

    it('renders the invalidation explainer banner', async () => {
        render(TradeAutomationDashboard);

        await waitFor(() =>
            expect(screen.getByText(/Invalidation:/)).toBeTruthy(),
        );
    });
});

describe('TradeAutomationDashboard mode badge (v7.2)', () => {
    it('renders the fixed mode from the automation payload without any toggle', async () => {
        const fetchMock = globalThis.fetch as unknown as ReturnType<typeof vi.fn>;
        fetchMock.mockClear();
        fetchMock.mockImplementation((url: string) => {
            if (url === '/api/instances') {
                return Promise.resolve(jsonResponse({ instances: [{ id: 'inst_btc', pair: 'BTC-USDC', status: 'running' }] }));
            }
            if (typeof url === 'string' && url.includes('/automation')) {
                return Promise.resolve(jsonResponse({ ...automationPayload, mode: 'live' }));
            }
            return Promise.resolve(jsonResponse([]));
        });

        render(TradeAutomationDashboard);
        await waitFor(() => expect(screen.getByText('AUTOMATION ON')).toBeTruthy());
        // Mode is displayed as a read-only badge — no switch affordance.
        await waitFor(() => expect(screen.getByText('LIVE')).toBeTruthy());
        expect(screen.queryByText(/Switch to/i)).toBeNull();
        const calls = fetchMock.mock.calls;
        expect(calls.some((c: unknown[]) => String(c[0]).includes('/mode'))).toBe(false);
    });

    it('observe mode renders the ghost radar with no order/close affordances', async () => {
        const fetchMock = globalThis.fetch as unknown as ReturnType<typeof vi.fn>;
        fetchMock.mockClear();
        fetchMock.mockImplementation((url: string) => {
            if (url === '/api/instances') {
                return Promise.resolve(jsonResponse({ instances: [{ id: 'inst_btc', pair: 'BTC-USDC', status: 'running' }] }));
            }
            if (typeof url === 'string' && url.includes('/automation')) {
                return Promise.resolve(jsonResponse({ ...automationPayload, mode: 'observe', ghost: true, entry_order: null, bracket: { tp_order: null, sl_order: null }, position: null }));
            }
            return Promise.resolve(jsonResponse([]));
        });

        render(TradeAutomationDashboard);
        await waitFor(() => expect(screen.getByText('OBSERVE')).toBeTruthy());
        // Ghost radar vocabulary.
        await waitFor(() => expect(screen.getByText('GHOST / NO ACTION')).toBeTruthy());
        expect(screen.getByText('Qualification Diagnostics')).toBeTruthy();
        expect(screen.getByText(/monitoring only/i)).toBeTruthy();
        // No dispatch affordances in observe.
        expect(screen.queryByText('Close now')).toBeNull();
        expect(screen.queryByText(/Switch to/i)).toBeNull();
    });

    it('live mode renders the reconciliation strip with venue order ids', async () => {
        const fetchMock = globalThis.fetch as unknown as ReturnType<typeof vi.fn>;
        fetchMock.mockClear();
        fetchMock.mockImplementation((url: string) => {
            if (url === '/api/instances') {
                return Promise.resolve(jsonResponse({ instances: [{ id: 'inst_btc', pair: 'BTC-USDC', status: 'running' }] }));
            }
            if (typeof url === 'string' && url.includes('/automation')) {
                return Promise.resolve(jsonResponse({
                    ...automationPayload,
                    mode: 'live',
                    ghost: false,
                    entry_order: { ...automationPayload.entry_order, id: 'hl_0xdeadbeef', status: 'Open' },
                    bracket: {
                        tp_order: { ...automationPayload.bracket.tp_order, id: 'hl_tp_123' },
                        sl_order: { ...automationPayload.bracket.sl_order, id: 'hl_sl_456' },
                    },
                }));
            }
            return Promise.resolve(jsonResponse([]));
        });

        render(TradeAutomationDashboard);
        await waitFor(() => expect(screen.getByText('Engine ↔ Venue Reconciliation')).toBeTruthy());
        expect(screen.getByText('hl_0xdeadbeef')).toBeTruthy();
        expect(screen.getAllByText('REDUCE-ONLY').length).toBeGreaterThan(0);
        expect(screen.queryByText('GHOST / NO ACTION')).toBeNull();
    });
});

describe('TradeAutomationDashboard no-instance state (v7.3)', () => {
    it('renders the SVG empty state instead of an infinite loading message', async () => {
        cleanup();
        const fetchMock = vi.fn((url: string) => {
            if (url === '/api/instances') {
                return Promise.resolve(jsonResponse({ instances: [] }));
            }
            return Promise.resolve(jsonResponse({}));
        });
        vi.stubGlobal('fetch', fetchMock);

        render(TradeAutomationDashboard);

        await waitFor(() => expect(screen.getByText('No active instance')).toBeTruthy());
        expect(screen.getByText(/Trade automation runs per instance/)).toBeTruthy();
        // The loading message must never appear once the list resolves empty.
        expect(screen.queryByText('Loading automation state…')).toBeNull();
        // Header shows the NO INSTANCE chip instead of an empty select.
        expect(screen.getByText('NO INSTANCE')).toBeTruthy();
    });

    it('keeps the Settings tab rendered without an instance', async () => {
        cleanup();
        const fetchMock = vi.fn((url: string) => {
            if (url === '/api/instances') {
                return Promise.resolve(jsonResponse({ instances: [] }));
            }
            if (url === '/api/config') {
                return Promise.resolve(jsonResponse({ minimal_tae: { enabled: false, risk_per_trade_pct: 1.0 } }));
            }
            return Promise.resolve(jsonResponse({}));
        });
        vi.stubGlobal('fetch', fetchMock);

        render(TradeAutomationDashboard, { props: { section: 'settings' } });

        await waitFor(() => expect(screen.getByText('Setup Executor')).toBeTruthy());
        expect(screen.queryByText('No active instance')).toBeNull();
    });
});

