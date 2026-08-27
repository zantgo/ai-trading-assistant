// @vitest-environment jsdom
//
// Launch Setup wizard tests. The wizard replaces the old WelcomeGate with a
// four-step installer:
//   1. Mode        — Observe / Simulate / Execute
//   2. Environment — exchange + settlement currency (+ capital for paper,
//                    credentials for live)
//   3. Instances   — staged drafts with per-TF duration dropdowns (same
//                    TIMEFRAME_OPTIONS tier list as the workspace Settings)
//   4. Review      — summary → Launch
//
// Currency contract per exchange (unchanged from WelcomeGate):
//   - Hyperliquid settles only in USDC.
//   - Bitget's dashboard exposes only USDT-M futures.

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { cleanup, render, fireEvent, screen, waitFor } from '@testing-library/svelte';
import { vi } from 'vitest';
import LaunchSetup from './LaunchSetup.svelte';
import { useAppStore } from './state.svelte';

beforeEach(() => {
    const app = useAppStore();
    // The wizard renders the session quote; pin the real singleton to the
    // Hyperliquid default so review rows read `BTC-USDC` deterministically.
    app.session.sessionCurrency = 'USDC';
    app.session.sessionExchange = 'Hyperliquid';
    app.session.sessionMode = 'observe';
    app.session.sessionActive = false;
    for (const key of Object.keys(app.instancesMap)) delete app.instancesMap[key];
});

afterEach(() => cleanup());

/** Read the text of every radio button label so we can check availability. */
function availableCurrencies(container: HTMLElement): string[] {
    const labels = container.querySelectorAll('label');
    const currencies: string[] = [];
    labels.forEach((l) => {
        const text = l.textContent?.trim() ?? '';
        if (text.includes('USDT')) currencies.push('USDT');
        if (text.includes('USDC')) currencies.push('USDC');
    });
    return Array.from(new Set(currencies));
}

/** Read whether the `Available` / `Not available` badge says a currency is enabled. */
function isCurrencyEnabled(container: HTMLElement, currency: string): boolean {
    const radios = container.querySelectorAll<HTMLInputElement>(
        `input[type="radio"][value="${currency}"]`,
    );
    if (radios.length === 0) return false;
    return !radios[0].disabled;
}

async function goToEnvironment(container: HTMLElement) {
    await fireEvent.click(screen.getByText('Continue'));
}

async function goToInstances(container: HTMLElement) {
    await goToEnvironment(container);
    await fireEvent.click(screen.getByText('Continue'));
}

/** Advance from the Instances step (3) to Review (4). */
async function goToReviewFromInstances() {
    await fireEvent.click(screen.getByText('Continue'));
}

/** Advance from the Environment step (2) to Review (4). */
async function goToReviewFromEnvironment() {
    await fireEvent.click(screen.getByText('Continue'));
    await goToReviewFromInstances();
}

/** Full path from the Mode step straight to Review. */
async function goToReview(container: HTMLElement) {
    await goToInstances(container);
    await goToReviewFromInstances();
}

describe('Launch Setup — mode selection', () => {
    it('shows the three mode cards Observe / Simulate / Execute', async () => {
        const { container } = await render(LaunchSetup);
        expect(container.textContent).toContain('Observe');
        expect(container.textContent).toContain('Simulate');
        expect(container.textContent).toContain('Execute');
        // Progression hint verbs.
        expect(container.textContent).toContain('Monitor');
        expect(container.textContent).toContain('Paper');
        expect(container.textContent).toContain('Real orders');
    });

    it('simulate mode shows the capital field', async () => {
        const { container } = await render(LaunchSetup);
        await fireEvent.click(screen.getByText('Simulate'));
        await goToEnvironment(container);
        expect(container.querySelector('#launch-capital')).toBeTruthy();
        expect(container.querySelector('#launch-wallet')).toBeFalsy();
        expect(container.querySelector('#launch-api-key')).toBeFalsy();
    });

    it('observe mode shows neither capital nor credentials', async () => {
        const { container } = await render(LaunchSetup);
        // Observe is the default mode.
        await goToEnvironment(container);
        expect(container.querySelector('#launch-capital')).toBeFalsy();
        expect(container.querySelector('#launch-wallet')).toBeFalsy();
        expect(container.querySelector('#launch-api-key')).toBeFalsy();
        expect(container.textContent).toContain('no capital and no credentials');
    });

    it('execute mode on Hyperliquid shows wallet credentials', async () => {
        const { container } = await render(LaunchSetup);
        await fireEvent.click(screen.getByText('Execute'));
        await goToEnvironment(container);
        expect(container.querySelector('#launch-wallet')).toBeTruthy();
        expect(container.querySelector('#launch-private-key')).toBeTruthy();
        expect(container.querySelector('#launch-capital')).toBeFalsy();
    });

    it('execute mode on Bitget shows API key credentials', async () => {
        const { container } = await render(LaunchSetup);
        await fireEvent.click(screen.getByText('Execute'));
        await goToEnvironment(container);
        const exchange = container.querySelector<HTMLSelectElement>('#launch-exchange');
        await fireEvent.change(exchange!, { target: { value: 'Bitget' } });
        expect(container.querySelector('#launch-api-key')).toBeTruthy();
        expect(container.querySelector('#launch-api-secret')).toBeTruthy();
        expect(container.querySelector('#launch-passphrase')).toBeTruthy();
    });
});

describe('Launch Setup — currency contract', () => {
    it('Hyperliquid exposes only USDC', async () => {
        const { container } = await render(LaunchSetup);
        await goToEnvironment(container);
        const enabled = availableCurrencies(container);
        expect(enabled).toContain('USDC');
        expect(isCurrencyEnabled(container, 'USDC')).toBe(true);
        expect(isCurrencyEnabled(container, 'USDT')).toBe(false);
    });

    it('Bitget exposes only USDT', async () => {
        const { container } = await render(LaunchSetup);
        await goToEnvironment(container);
        const exchange = container.querySelector<HTMLSelectElement>('#launch-exchange');
        await fireEvent.change(exchange!, { target: { value: 'Bitget' } });
        const enabled = availableCurrencies(container);
        expect(enabled).toContain('USDT');
        expect(isCurrencyEnabled(container, 'USDT')).toBe(true);
        expect(isCurrencyEnabled(container, 'USDC')).toBe(false);
    });
});

describe('Launch Setup — instances step', () => {
    it('renders per-slot timeframe dropdowns preseeded from the workspace ladder', async () => {
        const { container } = await render(LaunchSetup);
        await goToInstances(container);

        // The four TF slots are the only selects on this step.
        const selects = container.querySelectorAll<HTMLSelectElement>('select');
        expect(selects.length).toBe(4);

        // Same tier list as the Workspace Settings timeframe selector, with
        // the preset ladder (60/180/workspace-slow/workspace-macro) selected.
        const expected = [
            { seconds: 60, label: '1 min' },
            { seconds: 180, label: '3 min' },
            { seconds: 300, label: '5 min' },
            { seconds: 900, label: '15 min' },
        ];
        expected.forEach((exp, i) => {
            const opts = Array.from(selects[i].options);
            const tier = opts.find((o) => o.value === String(exp.seconds));
            expect(tier?.textContent).toBe(exp.label);
            expect(selects[i].value).toBe(String(exp.seconds));
        });

        // Full option parity with the workspace selector: 14 tiers + the
        // disabled "Custom:" fallback.
        expect(selects[0].options.length).toBe(15);
    });

    it('adds and removes staged instances with per-TF durations', async () => {
        const { container } = await render(LaunchSetup);
        await goToInstances(container);

        const baseInput = container.querySelector<HTMLInputElement>('#launch-base');
        await fireEvent.input(baseInput!, { target: { value: 'btc' } });
        await fireEvent.click(screen.getByText('+ Add'));

        // Normalized to uppercase and shown with the quote + TF ladder
        // (v7.2: the registry ladder — 60/180/workspace-slow/workspace-macro).
        expect(container.textContent).toContain('BTC');
        expect(container.textContent).toContain('1m / 3m / 5m / 15m');

        // Duplicate rejection.
        await fireEvent.input(baseInput!, { target: { value: 'BTC' } });
        await fireEvent.click(screen.getByText('+ Add'));
        expect(container.textContent).toContain('already in the instance list');

        // Invalid ticker rejection.
        await fireEvent.input(baseInput!, { target: { value: '!!' } });
        await fireEvent.click(screen.getByText('+ Add'));
        expect(container.textContent).toContain('Invalid ticker');

        // Remove.
        await fireEvent.click(container.querySelector<HTMLButtonElement>('button[aria-label="Remove BTC"]')!);
        expect(container.textContent).toContain('No instances configured yet.');
    });

    it('review step lists the staged instances', async () => {
        const { container } = await render(LaunchSetup);
        await goToInstances(container);
        const baseInput = container.querySelector<HTMLInputElement>('#launch-base');
        await fireEvent.input(baseInput!, { target: { value: 'ETH' } });
        await fireEvent.click(screen.getByText('+ Add'));

        await goToReviewFromInstances();
        expect(container.textContent).toContain('Review');
        expect(container.textContent).toContain('1 configured');
        expect(container.textContent).toContain('ETH-USDC');
    });
});

describe('Launch Setup — launch orchestration', () => {
    afterEach(() => {
        vi.unstubAllGlobals();
    });

    function mockBackend() {
        const calls: { url: string; body?: unknown }[] = [];
        const fetchMock = vi.fn((url: string, opts?: RequestInit) => {
            let body: unknown = null;
            try { body = opts?.body ? JSON.parse(String(opts.body)) : null; } catch { /* noop */ }
            calls.push({ url, body });
            if (typeof url === 'string' && url.includes('/api/session/init')) {
                return Promise.resolve(new Response(JSON.stringify({ success: true }), {
                    status: 200,
                    headers: { 'content-type': 'application/json' },
                }));
            }
            if (typeof url === 'string' && url.includes('/api/instances') && !url.includes('/config')) {
                return Promise.resolve(new Response(JSON.stringify({ id: 'inst_abc' }), {
                    status: 200,
                    headers: { 'content-type': 'application/json' },
                }));
            }
            return Promise.resolve(new Response('{}', {
                status: 200,
                headers: { 'content-type': 'application/json' },
            }));
        });
        vi.stubGlobal('fetch', fetchMock);
        return { calls, fetchMock };
    }

    it('launches an observe session with staged instances', async () => {
        const { calls } = mockBackend();
        const { container } = await render(LaunchSetup);
        await goToInstances(container);
        const baseInput = container.querySelector<HTMLInputElement>('#launch-base');
        await fireEvent.input(baseInput!, { target: { value: 'BTC' } });
        await fireEvent.click(screen.getByText('+ Add'));
        await goToReviewFromInstances();
        await fireEvent.click(screen.getByText('Launch'));

        await waitFor(() => expect(calls.length).toBeGreaterThanOrEqual(2));

        const initCall = calls.find((c) => String(c.url).includes('/api/session/init'));
        expect(initCall?.body).toMatchObject({
            mode: 'observe',
            exchange: 'Hyperliquid',
            currency: 'USDC',
        });
        // No capital submitted in observe mode.
        expect((initCall?.body as any)?.initial_capital_usd).toBeUndefined();

        const instCall = calls.find((c) => String(c.url).endsWith('/api/instances'));
        expect(instCall?.body).toMatchObject({ base: 'BTC', quote: 'USDC' });

        const configCall = calls.find((c) => String(c.url).includes('/config'));
        expect(configCall?.body).toMatchObject({
            micro_term: { candles: { duration_seconds: 60 } },
            fast_term: { candles: { duration_seconds: 180 } },
            slow_term: { candles: { duration_seconds: 300 } },
            macro_term: { candles: { duration_seconds: 900 } },
        });
    });

    it('launches with dropdown-selected durations in the config payload', async () => {
        const { calls } = mockBackend();
        const { container } = await render(LaunchSetup);
        await goToInstances(container);

        const selects = container.querySelectorAll<HTMLSelectElement>('select');
        // Micro → 15 min, slow → 1 hrs; fast/macro keep the ladder presets.
        await fireEvent.change(selects[0], { target: { value: '900' } });
        await fireEvent.change(selects[2], { target: { value: '3600' } });

        const baseInput = container.querySelector<HTMLInputElement>('#launch-base');
        await fireEvent.input(baseInput!, { target: { value: 'BTC' } });
        await fireEvent.click(screen.getByText('+ Add'));
        expect(container.textContent).toContain('15m / 3m / 1h / 15m');

        await goToReviewFromInstances();
        await fireEvent.click(screen.getByText('Launch'));

        await waitFor(() => expect(calls.length).toBeGreaterThanOrEqual(2));
        const configCall = calls.find((c) => String(c.url).includes('/config'));
        expect(configCall?.body).toMatchObject({
            micro_term: { candles: { duration_seconds: 900 } },
            fast_term: { candles: { duration_seconds: 180 } },
            slow_term: { candles: { duration_seconds: 3600 } },
            macro_term: { candles: { duration_seconds: 900 } },
        });
    });

    it('launches a simulate session with capital', async () => {
        const { calls } = mockBackend();
        const { container } = await render(LaunchSetup);
        await fireEvent.click(screen.getByText('Simulate'));
        await goToEnvironment(container);
        const capitalInput = container.querySelector<HTMLInputElement>('#launch-capital');
        await fireEvent.input(capitalInput!, { target: { value: '2500' } });
        await goToReviewFromEnvironment();
        await fireEvent.click(screen.getByText('Launch'));

        await waitFor(() => expect(calls.length).toBeGreaterThanOrEqual(1));
        const initCall = calls.find((c) => String(c.url).includes('/api/session/init'));
        expect(initCall?.body).toMatchObject({ mode: 'paper', portfolio_capital_usd: 2500 });
    });

    it('launches an execute session and saves credentials first', async () => {
        const { calls } = mockBackend();
        const { container } = await render(LaunchSetup);
        await fireEvent.click(screen.getByText('Execute'));
        await goToEnvironment(container);
        await fireEvent.input(container.querySelector<HTMLInputElement>('#launch-wallet')!, { target: { value: '0xabc' } });
        await fireEvent.input(container.querySelector<HTMLInputElement>('#launch-private-key')!, { target: { value: 'secret-key' } });
        await goToReviewFromEnvironment();
        await fireEvent.click(screen.getByText('Launch'));

        await waitFor(() => expect(calls.length).toBeGreaterThanOrEqual(2));
        const keyCall = calls.find((c) => String(c.url).includes('/api/keys'));
        expect(keyCall?.body).toMatchObject({
            exchange: 'Hyperliquid',
            api_key: '0xabc',
            api_secret: 'secret-key',
            is_active: true,
        });
        const initCall = calls.find((c) => String(c.url).includes('/api/session/init'));
        expect(initCall?.body).toMatchObject({ mode: 'live' });
    });

    it('surfaces a backend error from session init', async () => {
        const fetchMock = vi.fn((url: string) => {
            if (typeof url === 'string' && url.includes('/api/session/init')) {
                return Promise.resolve(new Response(
                    JSON.stringify({ success: false, error: 'Live session requires an active Hyperliquid API key' }),
                    { status: 400, headers: { 'content-type': 'application/json' } },
                ));
            }
            return Promise.resolve(new Response('{}', { status: 200, headers: { 'content-type': 'application/json' } }));
        });
        vi.stubGlobal('fetch', fetchMock);
        const { container } = await render(LaunchSetup);
        await goToReview(container);
        await fireEvent.click(screen.getByText('Launch'));
        await waitFor(() =>
            expect(container.textContent).toContain('Live session requires an active Hyperliquid API key'),
        );
    });
});
