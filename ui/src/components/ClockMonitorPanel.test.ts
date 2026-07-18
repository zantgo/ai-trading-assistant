// @vitest-environment jsdom
import { cleanup, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { ClockStatusResponse } from '../types';
import ClockMonitorPanel from './ClockMonitorPanel.svelte';

const report: ClockStatusResponse = {
    within_threshold: true,
    drift_us: 15,
    jitter_rms_us: 8.3,
    last_poll_ms: 1_700_000_000_000,
    breach_count: 0,
    breach_action: 'Warn',
    ntp_servers: ['pool.ntp.org', 'time.aws.com'],
    sample_count: 42,
    threshold_micros: 50,
};

function mockResponse(data: ClockStatusResponse): Response {
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

describe('ClockMonitorPanel', () => {
    it('renders_loading_state_initially', () => {
        vi.stubGlobal('fetch', vi.fn(() => new Promise<Response>(() => {})));
        render(ClockMonitorPanel);
        expect(screen.getByText('Loading...')).toBeTruthy();
    });

    it('renders_metrics_after_fetch', async () => {
        vi.stubGlobal('fetch', vi.fn().mockResolvedValue(mockResponse(report)));
        render(ClockMonitorPanel);
        expect(await screen.findByText('Within Threshold')).toBeTruthy();
        expect(screen.getByText('15µs')).toBeTruthy();
        expect(screen.getByText('50µs')).toBeTruthy();
        expect(screen.getByText('8.30µs')).toBeTruthy();
        expect(screen.getByText('42')).toBeTruthy();
        expect(screen.getByText('pool.ntp.org')).toBeTruthy();
    });

    it('renders_breach_state', async () => {
        const breached: ClockStatusResponse = {
            ...report,
            within_threshold: false,
            drift_us: 120,
            breach_action: 'Warn',
        };
        vi.stubGlobal('fetch', vi.fn().mockResolvedValue(mockResponse(breached)));
        render(ClockMonitorPanel);
        expect(await screen.findByText('BREACH')).toBeTruthy();
    });

    it('renders_panic_breach_state', async () => {
        const panicked: ClockStatusResponse = {
            ...report,
            within_threshold: false,
            drift_us: 200,
            breach_action: 'Panic',
        };
        vi.stubGlobal('fetch', vi.fn().mockResolvedValue(mockResponse(panicked)));
        render(ClockMonitorPanel);
        expect(await screen.findByText('BREACH (PANIC)')).toBeTruthy();
    });

    it('renders_error_state_on_fetch_failure', async () => {
        vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new Error('network down')));
        render(ClockMonitorPanel);
        expect(await screen.findByText('Error: network down')).toBeTruthy();
    });
});
