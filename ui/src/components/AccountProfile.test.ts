// AccountProfile.test — v9 mode matrix: observe is backtest-only (no
// trading capital), paper is editable with Reset, live is read-only.
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import AccountProfile from './AccountProfile.svelte';
import { useAppStore } from '../state.svelte';

vi.mock('../state.svelte', async (importOriginal) => {
    const actual = await importOriginal<typeof import('../state.svelte')>();
    return { ...actual };
});

function mockApi(mode: string) {
    const summary = {
        mode,
        portfolio_capital_source: mode === 'live' ? 'exchange' : mode === 'observe' ? 'none' : 'paper_config',
        portfolio_capital_usd: mode === 'observe' ? null : 2500,
        equity: 2510,
        daily_pnl: 10,
        drawdown_pct: 0.4,
        safety_state: 'NORMAL',
        instance_count: 2,
        open_positions_count: 0,
    };
    vi.stubGlobal(
        'fetch',
        vi.fn(async (input: RequestInfo | URL) => {
            const url = String(input);
            if (url.includes('/api/account/summary')) {
                return { ok: true, json: async () => summary } as Response;
            }
            if (url.includes('/api/strategies')) {
                return {
                    ok: true,
                    json: async () => ({ strategies: [{ name: 'default', base: null, description: 'baseline', schema_version: 1 }] }),
                } as Response;
            }
            if (url.includes('/api/account/capital')) {
                return { ok: true, json: async () => ({ success: true }) } as Response;
            }
            if (url.includes('/api/account/reset')) {
                return { ok: true, json: async () => ({ success: true }) } as Response;
            }
            return { ok: true, json: async () => ({}) } as Response;
        }),
    );
}

beforeEach(() => {
    cleanup();
    vi.unstubAllGlobals();
});

describe('AccountProfile — mode matrix', () => {
    it('observe shows the Backtest Studio hero and no trading capital', async () => {
        mockApi('observe');
        useAppStore().sessionMode = 'observe';
        const { container } = await render(AccountProfile);
        await waitFor(() => expect(screen.getByText('Backtest Studio')).toBeTruthy());
        expect(container.textContent).toContain('Observe mode has no trading capital');
        expect(container.querySelector('#acc-capital')).toBeFalsy();
        expect(container.textContent).toContain('none');
    });

    it('paper shows an editable capital card with Reset Paper Portfolio', async () => {
        mockApi('paper');
        useAppStore().sessionMode = 'paper';
        const { container } = await render(AccountProfile);
        await waitFor(() => expect(container.querySelector('#acc-capital')).toBeTruthy());
        expect(screen.getByText('Reset Paper Portfolio')).toBeTruthy();
        const input = container.querySelector<HTMLInputElement>('#acc-capital')!;
        expect(input.disabled).toBe(false);
        await waitFor(() => expect(container.textContent).toContain('Equity'));
        // paper: the capital input is editable with the configured value;
        // equity renders from the ledger.
        await waitFor(() => expect(container.querySelector<HTMLInputElement>('#acc-capital')?.value).toBe('2500'));
        await waitFor(() => expect(container.textContent).toContain('$2,510'));
    });

    it('live shows the same template with a read-only capital card', async () => {
        mockApi('live');
        useAppStore().sessionMode = 'live';
        const { container } = await render(AccountProfile);
        await waitFor(() => expect(container.querySelector('#acc-capital')).toBeTruthy());
        const input = container.querySelector<HTMLInputElement>('#acc-capital')!;
        expect(input.disabled).toBe(true);
        expect(container.textContent).toContain('The exchange balance IS your portfolio capital');
        const buttons = Array.from(container.querySelectorAll('button'));
        expect(buttons.some((b) => b.textContent?.includes('Reset Paper Portfolio'))).toBe(false);
    });

    it('reset requires confirmation then posts', async () => {
        mockApi('paper');
        useAppStore().sessionMode = 'paper';
        await render(AccountProfile);
        await waitFor(() => expect(screen.getByText('Reset Paper Portfolio')).toBeTruthy());
        await fireEvent.click(screen.getByText('Reset Paper Portfolio'));
        expect(await screen.findByText('Confirm Reset')).toBeTruthy();
        vi.stubGlobal('fetch', vi.fn(async (input: RequestInfo | URL) => {
            if (String(input).includes('/api/account/reset')) {
                return { ok: true, json: async () => ({ success: true }) } as Response;
            }
            if (String(input).includes('/api/account/summary')) {
                return { ok: true, json: async () => ({
                    mode: 'paper', portfolio_capital_source: 'paper_config',
                    portfolio_capital_usd: 2500, equity: 2500, daily_pnl: 0,
                    drawdown_pct: 0, safety_state: 'NORMAL', instance_count: 2,
                    open_positions_count: 0,
                }) } as Response;
            }
            return { ok: true, json: async () => ({ strategies: [] }) } as Response;
        }));
        await fireEvent.click(screen.getByText('Confirm Reset'));
        await waitFor(() => expect(screen.getByText(/reseeded/i)).toBeTruthy());
    });
});
