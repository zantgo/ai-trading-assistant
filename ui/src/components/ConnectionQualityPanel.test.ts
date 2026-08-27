// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { ConnectionQualityReport } from '../types';
import ConnectionQualityPanel from './ConnectionQualityPanel.svelte';

const report: ConnectionQualityReport = {
    window: 'one_hour',
    window_start_ms: 1_700_000_000_000,
    window_end_ms: 1_700_003_600_000,
    uptime_pct: 98.75,
    disconnect_count: 2,
    avg_reconnect_ms: 450,
    total_data_loss_secs: 3,
    reconstructed_candles: 1,
    score: 92,
};

function mockResponse(data: ConnectionQualityReport): Response {
    return {
        ok: true,
        status: 200,
        json: async () => data,
    } as Response;
}

afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
});

describe('ConnectionQualityPanel', () => {
    it('renders_loading_state_initially', () => {
        vi.stubGlobal('fetch', vi.fn(() => new Promise<Response>(() => {})));

        render(ConnectionQualityPanel);

        expect(screen.getByText('Loading…')).toBeTruthy();
    });

    it('renders_metrics_after_fetch', async () => {
        vi.stubGlobal('fetch', vi.fn().mockResolvedValue(mockResponse(report)));

        render(ConnectionQualityPanel);

        expect(await screen.findByText('98.75%')).toBeTruthy();
        expect(screen.getByText('92.0')).toBeTruthy();
        expect(screen.getByText('2')).toBeTruthy();
        expect(screen.getByText('450ms')).toBeTruthy();
        expect(screen.getByText('3s')).toBeTruthy();
        expect(screen.getByText('1')).toBeTruthy();
    });

    it('switches_window_on_tab_click', async () => {
        const fetchMock = vi.fn().mockResolvedValue(mockResponse(report));
        vi.stubGlobal('fetch', fetchMock);
        render(ConnectionQualityPanel);
        await waitFor(() => expect(fetchMock).toHaveBeenCalledWith('/api/connection-quality?window=one_hour'));

        await fireEvent.click(screen.getByRole('tab', { name: '6h' }));

        await waitFor(() => expect(fetchMock).toHaveBeenCalledWith('/api/connection-quality?window=six_hour'));
    });

    it('renders_error_state_on_fetch_failure', async () => {
        vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new Error('network down')));

        render(ConnectionQualityPanel);

        expect(await screen.findByText('Error: network down')).toBeTruthy();
    });

    it('score_color_applied_correctly', async () => {
        vi.stubGlobal('fetch', vi.fn().mockResolvedValue(mockResponse(report)));

        render(ConnectionQualityPanel);

        const score = await screen.findByText('92.0');
        // The shared KpiStrip colors the value via the `color` style token;
        // an excellent score (≥ 90) must render green (jsdom normalizes hex
        // to rgb).
        expect(score.getAttribute('style')).toContain('rgb(34, 197, 94)');
    });
});
