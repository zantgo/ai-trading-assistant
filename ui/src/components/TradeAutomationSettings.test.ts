// TradeAutomationSettings (v10) — lifecycle-hardening dials: the posture
// segmented control and the entry/exit policy cards edit the bound
// strategy JSON via GET/PUT /api/strategies/:name.
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor, cleanup } from '@testing-library/svelte';
import TradeAutomationSettings from './TradeAutomationSettings.svelte';

const strategyJson = {
    name: 'default',
    schema_version: 1,
    tae: {
        intake: { max_setup_age_bars: null },
        lifecycle: {
            pending_entry_expiry_bars: null,
            replace_policy: 'cancel_and_adopt',
            min_reprice_delta_atr: 0.25,
        },
        execution: {
            entry_mode: 'zone_midpoint',
            chase_max_atr: 0.5,
            chase_score_floor: 75,
            instant_fill_policy: 'take_better',
            spread_gate_bps: null,
            tp_placement: 'zone_midpoint',
        },
        risk: {
            setup_gone_policy: 'balanced',
            tp_refresh_min_rr_delta: 0.3,
            sl_mode: 'invalidation',
            sl_padding_atr: 0,
            atr_anchor_mult: 1.5,
            min_sl_atr: null,
            confidence_drop_pct: null,
        },
    },
};

function jsonResponse(body: unknown): Response {
    return { ok: true, status: 200, json: async () => body, text: async () => '' } as Response;
}

function mockFetch() {
    const fetchMock = vi.fn((url: string, opts?: RequestInit) => {
        if (url === '/api/config') {
            return Promise.resolve(
                jsonResponse({
                    minimal_tae: {
                        enabled: true,
                        allocation_pct: 10,
                        min_net_rr: 1,
                        max_position_size_pct_of_equity: null,
                        max_open_positions: 1,
                        entry_mode: 'zone_midpoint',
                        invalidate_on: 'direction_flip',
                    },
                    execution: { slippage_ceiling_pct: 0.5 },
                    instances: [{ id: 'inst_btc', strategy: 'default' }],
                }),
            );
        }
        if (url === '/api/strategies') {
            return Promise.resolve(jsonResponse({ strategies: [{ name: 'default' }] }));
        }
        if (url === '/api/strategies/default') {
            if (opts?.method === 'PUT') return Promise.resolve(jsonResponse({ ok: true }));
            return Promise.resolve(jsonResponse(strategyJson));
        }
        return Promise.resolve(jsonResponse({}));
    });
    vi.stubGlobal('fetch', fetchMock);
    return fetchMock;
}

beforeEach(() => {
    cleanup();
    mockFetch();
});

afterEach(() => {
    vi.unstubAllGlobals();
    cleanup();
});

describe('TradeAutomationSettings — v10 lifecycle-hardening dials', () => {
    it('renders the posture segmented control and policy cards from the bound strategy', async () => {
        render(TradeAutomationSettings);

        await waitFor(() => expect(screen.getByText('Lifecycle Posture')).toBeTruthy());
        expect(screen.getByText('BALANCED')).toBeTruthy();
        expect(screen.getByText('STRICT')).toBeTruthy();
        expect(screen.getByText('RISKY')).toBeTruthy();
        expect(screen.getByText('Entry Policy')).toBeTruthy();
        expect(screen.getByText('Exit Policy')).toBeTruthy();
        expect(screen.getByText('zone_midpoint — zone center limit')).toBeTruthy();
        expect(screen.getByText('invalidation — exact level (strict)')).toBeTruthy();
    });

    it('saves the edited posture to the bound strategy via PUT', async () => {
        render(TradeAutomationSettings);

        await waitFor(() => expect(screen.getByText('STRICT')).toBeTruthy());
        const strictRadio = screen.getByRole('radio', { name: /STRICT/ });
        expect(strictRadio).toBeTruthy();
        await fireEvent.click(strictRadio);

        const saveBtn = screen.getByText('Save dials') as HTMLButtonElement;
        await waitFor(() => expect(saveBtn.disabled).toBe(false));
        await fireEvent.click(saveBtn);

        const fetchMock = globalThis.fetch as unknown as ReturnType<typeof vi.fn>;
        await waitFor(() => {
            const put = fetchMock.mock.calls.find(
                (c: unknown[]) => String(c[0]) === '/api/strategies/default' && (c[1] as RequestInit | undefined)?.method === 'PUT',
            );
            expect(put).toBeTruthy();
            const opts = put[1] as { body?: string } | undefined;
            const body = JSON.parse(opts?.body ?? '{}') as {
                strategy: { tae: { risk: { setup_gone_policy: string } } };
            };
            expect(body.strategy.tae.risk.setup_gone_policy).toBe('strict');
        });
    });

    it('shows the bind hint when no strategy is bound', async () => {
        const fetchMock = globalThis.fetch as unknown as ReturnType<typeof vi.fn>;
        fetchMock.mockClear();
        fetchMock.mockImplementation((url: string) => {
            if (url === '/api/config') {
                return Promise.resolve(
                    jsonResponse({
                        minimal_tae: {
                            enabled: true, allocation_pct: 10, min_net_rr: 1,
                            max_position_size_pct_of_equity: null, max_open_positions: 1,
                            entry_mode: 'zone_midpoint', invalidate_on: 'direction_flip',
                        },
                        execution: { slippage_ceiling_pct: 0.5 },
                        instances: [{ id: 'inst_btc', strategy: null }],
                    }),
                );
            }
            if (url === '/api/strategies') return Promise.resolve(jsonResponse({ strategies: [] }));
            return Promise.resolve(jsonResponse({}));
        });

        render(TradeAutomationSettings);

        await waitFor(() => expect(screen.getByText('Lifecycle Hardening (v10)')).toBeTruthy());
        expect(screen.getByText(/Bind a strategy below/)).toBeTruthy();
    });
});
