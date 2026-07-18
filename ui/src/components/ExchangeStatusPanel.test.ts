// @vitest-environment jsdom
import { cleanup, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { ExchangeStatusReport } from '../types';
import ExchangeStatusPanel from './ExchangeStatusPanel.svelte';

const report: ExchangeStatusReport = {
    exchanges: [
        {
            name: 'Hyperliquid',
            state: 'Connected',
            active_pairs: 3,
            last_heartbeat_ms: Date.now() - 5_000,
            total_reconnects: 1,
            ws_url: 'wss://api.hyperliquid.xyz/ws',
        },
        {
            name: 'Bitget',
            state: 'Connected',
            active_pairs: 2,
            last_heartbeat_ms: Date.now() - 8_000,
            total_reconnects: 0,
            ws_url: 'wss://ws.bitget.com/v2/ws/public',
        },
    ],
};

function mockResponse(data: ExchangeStatusReport): Response {
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

describe('ExchangeStatusPanel', () => {
    it('renders_loading_state_initially', () => {
        vi.stubGlobal('fetch', vi.fn(() => new Promise<Response>(() => {})));
        render(ExchangeStatusPanel);
        expect(screen.getByText('Loading...')).toBeTruthy();
    });

    it('renders_exchanges_after_fetch', async () => {
        vi.stubGlobal('fetch', vi.fn().mockResolvedValue(mockResponse(report)));
        render(ExchangeStatusPanel);
        expect(await screen.findByText('Hyperliquid')).toBeTruthy();
        expect(screen.getByText('Bitget')).toBeTruthy();
        expect(screen.getAllByText('● Connected').length).toBe(2);
        expect(screen.getByText('3')).toBeTruthy();
        expect(screen.getByText('0')).toBeTruthy();
    });

    it('renders_disconnected_state', async () => {
        const down: ExchangeStatusReport = {
            exchanges: [
                {
                    name: 'Hyperliquid',
                    state: 'Disconnected',
                    active_pairs: 0,
                    last_heartbeat_ms: 0,
                    total_reconnects: 5,
                    ws_url: 'wss://api.hyperliquid.xyz/ws',
                },
            ],
        };
        vi.stubGlobal('fetch', vi.fn().mockResolvedValue(mockResponse(down)));
        render(ExchangeStatusPanel);
        expect(await screen.findByText('● Disconnected')).toBeTruthy();
    });

    it('renders_error_state_on_fetch_failure', async () => {
        vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new Error('network down')));
        render(ExchangeStatusPanel);
        expect(await screen.findByText('Error: network down')).toBeTruthy();
    });

    it('renders_disabled_state', async () => {
        const disabled: ExchangeStatusReport = {
            exchanges: [
                {
                    name: 'Bitget',
                    state: 'Disabled',
                    active_pairs: 1,
                    last_heartbeat_ms: Date.now() - 60_000,
                    total_reconnects: 6,
                    ws_url: 'wss://ws.bitget.com/v2/ws/public',
                },
            ],
        };
        vi.stubGlobal('fetch', vi.fn().mockResolvedValue(mockResponse(disabled)));
        render(ExchangeStatusPanel);
        expect(await screen.findByText('✕ Disabled')).toBeTruthy();
        expect(screen.getByText('6')).toBeTruthy();
    });
});
