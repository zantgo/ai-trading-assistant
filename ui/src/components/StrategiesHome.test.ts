// StrategiesHome.test — list ↔ editor switching.
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, render, screen, waitFor } from '@testing-library/svelte';
import StrategiesHome from './StrategiesHome.svelte';

vi.stubGlobal('fetch', vi.fn(async (input: RequestInfo | URL) => {
    const url = String(input);
    if (url.includes('/api/strategies')) {
        return { ok: true, json: async () => ({ strategies: [{ name: 'default', base: null, description: 'baseline', schema_version: 1 }] }) } as Response;
    }
    return { ok: true, json: async () => ({}) } as Response;
}));

beforeEach(() => cleanup());

describe('StrategiesHome', () => {
    it('shows the list and opens the editor on Edit', async () => {
        await render(StrategiesHome);
        await waitFor(() => expect(screen.getByText('default')).toBeTruthy());
        const edit = screen.getAllByText('Edit')[0];
        await edit.click();
        await waitFor(() => expect(screen.getByText('STRATEGY EDITOR')).toBeTruthy());
    });
});
