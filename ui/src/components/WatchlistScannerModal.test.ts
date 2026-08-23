// @vitest-environment jsdom
// Tests for the Watchlist Scanner modal — three phases (input, running,
// done) over a single dialog. The modal's three async dependencies
// (`createInstance`, `waitForAdvisory`, `connectWsForInstance`) are
// mocked at the module level so the test never hits the network.

import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { AdvisoryMatrix, DecisionContext } from '../types';
import { useAppStore } from '../state.svelte';

const mockCreateInstance = vi.fn();
const mockDeleteInstanceById = vi.fn();
const mockWaitForAdvisory = vi.fn();
const mockConnectWsForInstance = vi.fn();

vi.mock('../lib/api.svelte', () => ({
    createInstance: (...args: unknown[]) => mockCreateInstance(...args),
    deleteInstanceById: (...args: unknown[]) => mockDeleteInstanceById(...args),
    waitForAdvisory: (...args: unknown[]) => mockWaitForAdvisory(...args),
}));

vi.mock('../lib/websocket.svelte', () => ({
    connectWsForInstance: (...args: unknown[]) => mockConnectWsForInstance(...args),
}));

import WatchlistScannerModal from './WatchlistScannerModal.svelte';

function makeAdvisory(overrides: Partial<AdvisoryMatrix> = {}): AdvisoryMatrix {
    return {
        symbol: 'BTC-USDT',
        directional_guidance: 'Long',
        market_stance: 'Neutral',
        opportunity_classification: 'TrendContinuation',
        strategy_environment: 'TrendFollowing',
        entry_guidance: 'WaitForConfirmation',
        exit_guidance: 'NoWarning',
        protection_strategy: 'ATRBased',
        target_strategy: 'ResistanceBased',
        confidence_assessment: 31,
        final_recommendation: 'Long bias',
        ...overrides,
    } as AdvisoryMatrix;
}

function makeDecisionContext(overrides: Partial<DecisionContext> = {}): DecisionContext {
    return {
        score: 75,
        bias: 'Bullish',
        confidence: 0.7,
        score_confidence: 0.7,
        entry_danger: 25,
        expected_reward_risk_ratio: 2.0,
        trade_readiness: 'READY',
        contributing_indicators: [],
        ...overrides,
    } as DecisionContext;
}

beforeEach(() => {
    mockCreateInstance.mockReset();
    mockDeleteInstanceById.mockReset();
    mockWaitForAdvisory.mockReset();
    mockConnectWsForInstance.mockReset();
    mockDeleteInstanceById.mockResolvedValue(true);

    const app = useAppStore();
    for (const key of Object.keys(app.instancesMap)) delete app.instancesMap[key];
    app.sessionActive = true;
    app.sessionCurrency = 'USDT';
    app.sessionExchange = 'Hyperliquid';
});

afterEach(() => {
    cleanup();
    vi.useRealTimers();
});

describe('WatchlistScannerModal — input phase', () => {
    it('renders input textarea + Continue button when open and session active', () => {
        render(WatchlistScannerModal, {
            props: { isOpen: true, wssMap: {}, onclose: () => {} },
        });
        expect(screen.getByLabelText('Watchlist symbols')).toBeTruthy();
        expect(screen.getByText('Continue')).toBeTruthy();
        expect(screen.getByText('Cancel')).toBeTruthy();
    });

    it('does not render when isOpen is false', () => {
        const { container } = render(WatchlistScannerModal, {
            props: { isOpen: false, wssMap: {}, onclose: () => {} },
        });
        expect(container.querySelector('[role="dialog"]')).toBeNull();
    });

    it('disables Continue when no symbols are parsed', async () => {
        render(WatchlistScannerModal, {
            props: { isOpen: true, wssMap: {}, onclose: () => {} },
        });
        const continueBtn = screen.getByText('Continue') as HTMLButtonElement;
        expect(continueBtn.disabled).toBe(true);
    });

    it('renders the wait-window input defaulting to 5 minutes', () => {
        render(WatchlistScannerModal, {
            props: { isOpen: true, wssMap: {}, onclose: () => {} },
        });
        const waitInput = screen.getByLabelText('Wait window (minutes)') as HTMLInputElement;
        expect(waitInput).toBeTruthy();
        expect(waitInput.value).toBe('5');
        expect(waitInput.min).toBe('1');
        expect(waitInput.max).toBe('60');
    });

    it('passes the configured wait window (minutes → ms) to waitForAdvisory', async () => {
        mockCreateInstance.mockResolvedValue({ ok: true, instanceId: 'inst_btc' });
        mockWaitForAdvisory.mockResolvedValue({ status: 'TIMEOUT', waitedMs: 420_000 });

        render(WatchlistScannerModal, {
            props: { isOpen: true, wssMap: {}, onclose: () => {} },
        });
        const waitInput = screen.getByLabelText('Wait window (minutes)') as HTMLInputElement;
        await fireEvent.input(waitInput, { target: { value: '7' } });
        const textarea = screen.getByLabelText('Watchlist symbols') as HTMLTextAreaElement;
        await fireEvent.input(textarea, { target: { value: 'BTC' } });
        await fireEvent.click(screen.getByText('Continue'));

        await waitFor(() => expect(screen.getByText('Accept')).toBeTruthy());
        // 7 minutes → 420,000 ms window.
        expect(mockWaitForAdvisory).toHaveBeenCalledWith(
            expect.anything(),
            expect.any(String),
            420_000,
        );
    });

    it('shows session-not-ready banner when session is inactive', () => {
        const app = useAppStore();
        app.sessionActive = false;
        render(WatchlistScannerModal, {
            props: { isOpen: true, wssMap: {}, onclose: () => {} },
        });
        expect(screen.getByText(/Initialize a session/)).toBeTruthy();
    });

    it('disables Continue when session is inactive', () => {
        const app = useAppStore();
        app.sessionActive = false;
        render(WatchlistScannerModal, {
            props: { isOpen: true, wssMap: {}, onclose: () => {} },
        });
        const continueBtn = screen.getByText('Continue') as HTMLButtonElement;
        expect(continueBtn.disabled).toBe(true);
    });
});

describe('WatchlistScannerModal — run phase', () => {
    it('advances to running phase on Continue', async () => {
        mockCreateInstance.mockResolvedValue({ ok: false, error: 'simulated' });

        render(WatchlistScannerModal, {
            props: { isOpen: true, wssMap: {}, onclose: () => {} },
        });
        const textarea = screen.getByLabelText('Watchlist symbols') as HTMLTextAreaElement;
        await fireEvent.input(textarea, { target: { value: 'BTC ETH' } });
        const continueBtn = screen.getByText('Continue');
        await fireEvent.click(continueBtn);

        await waitFor(() => expect(screen.getByText('Watchlist scan')).toBeTruthy());
        expect(mockCreateInstance).toHaveBeenCalled();
    });

    it('keeps pair with READY + Long; removes pair with STAND_ASIDE', async () => {
        mockCreateInstance
            .mockResolvedValueOnce({ ok: true, instanceId: 'inst_btc' })
            .mockResolvedValueOnce({ ok: true, instanceId: 'inst_eth' });
        mockWaitForAdvisory
            .mockResolvedValueOnce({
                status: 'READY',
                decisionContext: makeDecisionContext({ trade_readiness: 'READY' }),
                advisory: makeAdvisory({ directional_guidance: 'Long' }),
            })
            .mockResolvedValueOnce({
                status: 'READY',
                decisionContext: makeDecisionContext({ trade_readiness: 'STAND_ASIDE' }),
                advisory: makeAdvisory({ directional_guidance: 'StrongLong' }),
            });

        const onclose = vi.fn();
        render(WatchlistScannerModal, {
            props: { isOpen: true, wssMap: {}, onclose },
        });
        const textarea = screen.getByLabelText('Watchlist symbols') as HTMLTextAreaElement;
        await fireEvent.input(textarea, { target: { value: 'BTC ETH' } });
        await fireEvent.click(screen.getByText('Continue'));

        await waitFor(() => expect(screen.getByText('Accept')).toBeTruthy());
        // The "Kept" label appears in both the summary stats block and the
        // summary groups block; use getAllByText since both must render.
        expect(screen.getAllByText('Kept').length).toBeGreaterThanOrEqual(1);
        expect(mockDeleteInstanceById).toHaveBeenCalledWith('inst_eth');
        expect(mockDeleteInstanceById).not.toHaveBeenCalledWith('inst_btc');
    });

    it('removes pair on timeout (TIMEOUT branch)', async () => {
        mockCreateInstance.mockResolvedValue({ ok: true, instanceId: 'inst_btc' });
        mockWaitForAdvisory.mockResolvedValue({ status: 'TIMEOUT' });

        render(WatchlistScannerModal, {
            props: { isOpen: true, wssMap: {}, onclose: () => {} },
        });
        const textarea = screen.getByLabelText('Watchlist symbols') as HTMLTextAreaElement;
        await fireEvent.input(textarea, { target: { value: 'BTC' } });
        await fireEvent.click(screen.getByText('Continue'));

        await waitFor(() => expect(screen.getByText('Accept')).toBeTruthy());
        expect(mockDeleteInstanceById).toHaveBeenCalledWith('inst_btc');
    });

    it('keeps pair with READY + StrongShort', async () => {
        mockCreateInstance.mockResolvedValue({ ok: true, instanceId: 'inst_sol' });
        mockWaitForAdvisory.mockResolvedValue({
            status: 'READY',
            decisionContext: makeDecisionContext({ trade_readiness: 'READY' }),
            advisory: makeAdvisory({ directional_guidance: 'StrongShort' }),
        });

        render(WatchlistScannerModal, {
            props: { isOpen: true, wssMap: {}, onclose: () => {} },
        });
        const textarea = screen.getByLabelText('Watchlist symbols') as HTMLTextAreaElement;
        await fireEvent.input(textarea, { target: { value: 'SOL' } });
        await fireEvent.click(screen.getByText('Continue'));

        await waitFor(() => expect(screen.getByText('Accept')).toBeTruthy());
        expect(mockDeleteInstanceById).not.toHaveBeenCalled();
    });

    it('removes pair with READY + Neutral', async () => {
        mockCreateInstance.mockResolvedValue({ ok: true, instanceId: 'inst_avax' });
        mockWaitForAdvisory.mockResolvedValue({
            status: 'READY',
            decisionContext: makeDecisionContext({ trade_readiness: 'READY' }),
            advisory: makeAdvisory({ directional_guidance: 'Neutral' }),
        });

        render(WatchlistScannerModal, {
            props: { isOpen: true, wssMap: {}, onclose: () => {} },
        });
        const textarea = screen.getByLabelText('Watchlist symbols') as HTMLTextAreaElement;
        await fireEvent.input(textarea, { target: { value: 'AVAX' } });
        await fireEvent.click(screen.getByText('Continue'));

        await waitFor(() => expect(screen.getByText('Accept')).toBeTruthy());
        expect(mockDeleteInstanceById).toHaveBeenCalledWith('inst_avax');
    });

    it('skips duplicate pair without deleting anything', async () => {
        mockCreateInstance.mockResolvedValue({
            ok: false,
            error: 'Instance for pair BTC-USDT already exists',
        });

        render(WatchlistScannerModal, {
            props: { isOpen: true, wssMap: {}, onclose: () => {} },
        });
        const textarea = screen.getByLabelText('Watchlist symbols') as HTMLTextAreaElement;
        await fireEvent.input(textarea, { target: { value: 'BTC' } });
        await fireEvent.click(screen.getByText('Continue'));

        await waitFor(() => expect(screen.getByText('Accept')).toBeTruthy());
        expect(mockDeleteInstanceById).not.toHaveBeenCalled();
        expect(mockWaitForAdvisory).not.toHaveBeenCalled();
    });
});

describe('WatchlistScannerModal — done phase', () => {
    it('Accept button closes the modal', async () => {
        mockCreateInstance.mockResolvedValue({ ok: false, error: 'simulated' });

        const onclose = vi.fn();
        render(WatchlistScannerModal, {
            props: { isOpen: true, wssMap: {}, onclose },
        });
        const textarea = screen.getByLabelText('Watchlist symbols') as HTMLTextAreaElement;
        await fireEvent.input(textarea, { target: { value: 'BTC' } });
        await fireEvent.click(screen.getByText('Continue'));

        await waitFor(() => expect(screen.getByText('Accept')).toBeTruthy());
        await fireEvent.click(screen.getByText('Accept'));
        expect(onclose).toHaveBeenCalled();
    });

    it('renders summary stats with kept/removed counts', async () => {
        mockCreateInstance
            .mockResolvedValueOnce({ ok: true, instanceId: 'inst_btc' })
            .mockResolvedValueOnce({ ok: true, instanceId: 'inst_eth' });
        mockWaitForAdvisory
            .mockResolvedValueOnce({
                status: 'READY',
                decisionContext: makeDecisionContext({ trade_readiness: 'READY' }),
                advisory: makeAdvisory({ directional_guidance: 'Long' }),
            })
            .mockResolvedValueOnce({ status: 'TIMEOUT' });

        render(WatchlistScannerModal, {
            props: { isOpen: true, wssMap: {}, onclose: () => {} },
        });
        const textarea = screen.getByLabelText('Watchlist symbols') as HTMLTextAreaElement;
        await fireEvent.input(textarea, { target: { value: 'BTC ETH' } });
        await fireEvent.click(screen.getByText('Continue'));

        await waitFor(() => expect(screen.getByText('Accept')).toBeTruthy());
        // Kept: 1 (BTC), Removed: 1 (ETH — timeout). Both labels appear in
        // multiple sections (stats + groups), so use getAllByText.
        expect(screen.getAllByText('Kept').length).toBeGreaterThanOrEqual(1);
        expect(screen.getAllByText('Removed').length).toBeGreaterThanOrEqual(1);
    });
});
