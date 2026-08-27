// StrategyListPanel.test — v9 CRUD surface: lists cards, creates via
// clone, deletes with confirm, exports via download link.
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import StrategyListPanel from './StrategyListPanel.svelte';

function mockApi(strategies: { name: string; base: string | null; description: string; schema_version: number }[]) {
    vi.stubGlobal(
        'fetch',
        vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
            const url = String(input);
            if (url.includes('/api/strategies') && (!init || init.method === 'GET')) {
                return { ok: true, json: async () => ({ strategies }) } as Response;
            }
            if (url.includes('/clone')) {
                return { ok: true, json: async () => ({ success: true, name: 'x' }) } as Response;
            }
            if (init?.method === 'DELETE') {
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

describe('StrategyListPanel', () => {
    it('renders strategy cards with the default locked', async () => {
        mockApi([
            { name: 'default', base: null, description: 'baseline', schema_version: 1 },
            { name: 'trend-following', base: 'default', description: 'trends only', schema_version: 1 },
        ]);
        await render(StrategyListPanel);
        await waitFor(() => expect(screen.getByText('trend-following')).toBeTruthy());
        expect(screen.getByText('DEFAULT')).toBeTruthy();
        // default has no Delete button; the custom one does
        expect(screen.getAllByText('Delete').length).toBe(1);
    });

    it('delete asks for confirmation', async () => {
        mockApi([{ name: 'custom', base: null, description: '', schema_version: 1 }]);
        const confirm = vi.spyOn(window, 'confirm').mockReturnValue(false);
        await render(StrategyListPanel);
        await waitFor(() => expect(screen.getByText('custom')).toBeTruthy());
        await fireEvent.click(screen.getByText('Delete'));
        expect(confirm).toHaveBeenCalled();
        confirm.mockRestore();
    });
});
