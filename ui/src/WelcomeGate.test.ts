// @vitest-environment jsdom
//
// Tests for the WelcomeGate currency-selector contract. The supported
// currencies per exchange are derived from the operator's preferred
// product set per exchange:
//   - Hyperliquid settles only in USDC (HL is USDC-only).
//   - Bitget's dashboard exposes only USDT-M futures. (The backend's
//     `session::ExchangeChoice::supports_currency` still returns true
//     for USDC, but the welcome modal does not surface it.)

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { cleanup, render } from '@testing-library/svelte';
import WelcomeGate from './WelcomeGate.svelte';

beforeEach(() => {
    (globalThis as any).__appStore = {
        instancesMap: {},
        // Minimal stub for initSession — these tests never call handleEnter().
        initSession: async () => ({ success: true }),
    };
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

describe('WelcomeGate currency selector', () => {
    it('Hyperliquid exposes only USDC', async () => {
        // Hyperliquid is the default. The component reads `exchange` as a
        // local `$state`; the default value in the source is 'Hyperliquid'
        // — that's the path under test.
        const { container } = await render(WelcomeGate);
        const enabled = availableCurrencies(container);
        expect(enabled).toContain('USDC');
        expect(isCurrencyEnabled(container, 'USDC')).toBe(true);
        // USDT must NOT be enabled on Hyperliquid.
        expect(isCurrencyEnabled(container, 'USDT')).toBe(false);
    });

    it('Bitget exposes only USDT', async () => {
        // The component's local `$state` defaults to `exchange =
        // 'Hyperliquid'` and the source-of-truth `supportedCurrencies`
        // derives a list at render time. Without a way to mutate
        // `$state` from the outside in this test, we exercise the
        // contract statically by reading the source file and asserting
        // the Bitget branch contains ONLY USDT (not USDC). This guards
        // against the dashboard re-introducing the Bitget+USDC option.
        const fs = await import('node:fs/promises');
        const path = await import('node:path');
        const here = path.dirname(new URL(import.meta.url).pathname);
        const src = await fs.readFile(path.join(here, 'WelcomeGate.svelte'), 'utf-8');

        // Positive: the Bitget branch is exactly `['USDT']`.
        expect(src).toMatch(
            /exchange\s*===\s*['"]Hyperliquid['"]\s*\?\s*\[['"]USDC['"]\]\s*:\s*\[['"]USDT['"]\]/,
        );

        // Negative: the old `['USDT', 'USDC']` Bitget branch must not
        // come back. This is the regression guard — the test would
        // fail loudly if a future change re-broadens the Bitget bucket.
        expect(src).not.toMatch(/\[\s*['"]USDT['"]\s*,\s*['"]USDC['"]\s*\]/);
    });
});
// ── v7.1 follow-up: mode + paper capital flow ─────────────────────────
// The Welcome screen lets the operator choose Paper Trading or Live
// Trading; paper mode shows a USD capital field that is submitted with
// the session init request. `useAppStore()` returns the real singleton,
// so the session-init POST is mocked via `globalThis.fetch`.

import { fireEvent, screen, waitFor } from '@testing-library/svelte';
import { vi } from 'vitest';

function mockSessionInit(handler: (body: unknown) => Response) {
    const fetchMock = vi.fn((url: string, opts?: RequestInit) => {
        if (typeof url === 'string' && url.includes('/api/session/init')) {
            let body: unknown = null;
            try { body = JSON.parse(String(opts?.body ?? '{}')); } catch { /* noop */ }
            return Promise.resolve(handler(body));
        }
        return Promise.resolve(new Response('{}', { status: 200, headers: { 'content-type': 'application/json' } }));
    });
    vi.stubGlobal('fetch', fetchMock);
    return fetchMock;
}

describe('WelcomeGate mode + paper capital (v7.1)', () => {
    afterEach(() => {
        vi.unstubAllGlobals();
    });

    it('shows the paper capital field in paper mode and submits it', async () => {
        let captured: unknown = null;
        mockSessionInit((body) => {
            captured = body;
            return new Response(JSON.stringify({ success: true }), {
                status: 200,
                headers: { 'content-type': 'application/json' },
            });
        });

        const { container } = await render(WelcomeGate);
        const capitalInput = container.querySelector<HTMLInputElement>('#paper-capital');
        expect(capitalInput).toBeTruthy();

        capitalInput!.value = '2500';
        await fireEvent.input(capitalInput!);
        await fireEvent.click(screen.getByText('Enter System'));

        await waitFor(() => expect(captured).toBeTruthy());
        expect(captured).toMatchObject({
            mode: 'paper',
            initial_capital_usd: 2500,
            exchange: 'Hyperliquid',
        });
    });

    it('live mode hides the capital field and shows the key hint', async () => {
        const { container } = await render(WelcomeGate);
        const liveRadio = container.querySelector<HTMLInputElement>('input[type="radio"][value="live"]');
        await fireEvent.click(liveRadio!);

        expect(container.querySelector('#paper-capital')).toBeFalsy();
        expect(container.textContent).toContain('Exchange API Keys');
    });

    it('surfaces a backend error (e.g. live without a key)', async () => {
        mockSessionInit(() =>
            new Response(
                JSON.stringify({ success: false, error: 'Live session requires an active Hyperliquid API key' }),
                { status: 400, headers: { 'content-type': 'application/json' } },
            ),
        );

        const { container } = await render(WelcomeGate);
        await fireEvent.click(screen.getByText('Enter System'));
        await waitFor(() =>
            expect(container.textContent).toContain('Live session requires an active Hyperliquid API key'),
        );
    });
});
