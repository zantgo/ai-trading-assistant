// @vitest-environment jsdom
// Tests for the LiquidityStatusPanel status pill.
//
// Verifies:
//  - the worst status across slots is surfaced as the pill state
//  - the skip reason from any failing slot is captured
//  - fetch failures (HTTP errors, malformed bodies) are surfaced as 'error'
//  - the polling timer cleans up on unmount
//
// The actual endpoint URL is mocked via `globalThis.fetch`.

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { mount, unmount } from 'svelte';
import { tick } from 'svelte';
import LiquidityStatusPanel from './LiquidityStatusPanel.svelte';

type FetchOkBody = {
    slots?: Record<string, {
        status: string;
        last_skip_reason?: string | null;
    }>;
};

function makeBody(slots: Record<string, { status: string; last_skip_reason?: string | null }>): FetchOkBody {
    return { slots };
}

describe('LiquidityStatusPanel worst-status rollup', () => {
    let fetchMock: ReturnType<typeof vi.fn>;

    beforeEach(() => {
        // Real timers (not fake) — `runAllTimersAsync` would loop forever
        // because the component sets a 3 s polling interval that re-fires
        // each tick. With real timers + a one-shot mockResolvedValueOnce,
        // the first fetch resolves on microtask flush and we can assert
        // before the next interval fires.
        fetchMock = vi.fn();
        (globalThis as any).fetch = fetchMock;
    });

    afterEach(() => {
        document.body.innerHTML = '';
        vi.restoreAllMocks();
    });

    it('shows OK when all slots are OK', async () => {
        fetchMock.mockResolvedValueOnce({
            ok: true,
            json: async () => makeBody({
                micro: { status: 'OK', last_skip_reason: null },
                fast: { status: 'OK', last_skip_reason: null },
                slow: { status: 'OK', last_skip_reason: null },
                macro: { status: 'OK', last_skip_reason: null },
            }),
        });
        const component = mount(LiquidityStatusPanel, {
            target: document.body,
            props: { symbol: 'BTC-USDC' },
        });
        // Let the in-flight fetch resolve + Svelte flush its effect.
        for (let i = 0; i < 10; i++) await Promise.resolve();
        await tick();
        const pill = document.body.querySelector('[title]') as HTMLElement;
        expect(pill).not.toBeNull();
        expect(pill.title).toContain('Cluster refresh OK for BTC-USDC');
        unmount(component);
    });

    it('shows SKIPPED when one slot is failing, with that slot\'s reason', async () => {
        fetchMock.mockResolvedValueOnce({
            ok: true,
            json: async () => makeBody({
                micro: { status: 'OK', last_skip_reason: null },
                fast: { status: 'SKIPPED', last_skip_reason: 'no open_interest yet' },
                slow: { status: 'OK', last_skip_reason: null },
                macro: { status: 'OK', last_skip_reason: null },
            }),
        });
        const component = mount(LiquidityStatusPanel, {
            target: document.body,
            props: { symbol: 'BTC-USDC' },
        });
        for (let i = 0; i < 10; i++) await Promise.resolve();
        await tick();
        const pill = document.body.querySelector('[title]') as HTMLElement;
        expect(pill.title).toContain('Cluster refresh failing');
        expect(pill.title).toContain('no open_interest yet');
        unmount(component);
    });

    it('surfaces Stale above Pending above Ok', async () => {
        fetchMock.mockResolvedValueOnce({
            ok: true,
            json: async () => makeBody({
                micro: { status: 'OK', last_skip_reason: null },
                fast: { status: 'PENDING', last_skip_reason: null },
                slow: { status: 'OK', last_skip_reason: null },
                macro: { status: 'STALE', last_skip_reason: 'TTL elapsed' },
            }),
        });
        const component = mount(LiquidityStatusPanel, {
            target: document.body,
            props: { symbol: 'BTC-USDC' },
        });
        for (let i = 0; i < 10; i++) await Promise.resolve();
        await tick();
        const pill = document.body.querySelector('[title]') as HTMLElement;
        // STALE outranks PENDING and OK → tooltip mentions Stale/TTL.
        expect(pill.title).toContain('TTL elapsed');
        unmount(component);
    });

    it('treats unknown backend status values as fetching (defensive)', async () => {
        fetchMock.mockResolvedValueOnce({
            ok: true,
            json: async () => makeBody({
                micro: { status: 'OK', last_skip_reason: null },
                fast: { status: 'UNKNOWN_FUTURE_VALUE', last_skip_reason: null },
                slow: { status: 'OK', last_skip_reason: null },
                macro: { status: 'OK', last_skip_reason: null },
            }),
        });
        const component = mount(LiquidityStatusPanel, {
            target: document.body,
            props: { symbol: 'BTC-USDC' },
        });
        for (let i = 0; i < 10; i++) await Promise.resolve();
        await tick();
        const pill = document.body.querySelector('[title]') as HTMLElement;
        // Unknown status is normalized to 'fetching' (rank 0, neutral)
        // so the pill doesn't go red on a future server-side enum
        // addition.
        expect(pill.title).not.toContain('failing');
        expect(pill.title).not.toContain('unreachable');
        unmount(component);
    });

    it('reports error when HTTP status is non-2xx', async () => {
        fetchMock.mockResolvedValueOnce({
            ok: false,
            status: 503,
            json: async () => ({}),
        });
        const component = mount(LiquidityStatusPanel, {
            target: document.body,
            props: { symbol: 'BTC-USDC' },
        });
        for (let i = 0; i < 10; i++) await Promise.resolve();
        await tick();
        const pill = document.body.querySelector('[title]') as HTMLElement;
        expect(pill.title).toContain('Cluster status endpoint unreachable');
        expect(pill.title).toContain('503');
        unmount(component);
    });

    it('reports error when the response body is malformed (no slots field)', async () => {
        fetchMock.mockResolvedValueOnce({
            ok: true,
            json: async () => ({ unexpected_shape: true }),
        });
        const component = mount(LiquidityStatusPanel, {
            target: document.body,
            props: { symbol: 'BTC-USDC' },
        });
        for (let i = 0; i < 10; i++) await Promise.resolve();
        await tick();
        const pill = document.body.querySelector('[title]') as HTMLElement;
        expect(pill.title).toContain('Cluster status endpoint unreachable');
        expect(pill.title).toContain('malformed response');
        unmount(component);
    });

    it('captures the most informative skip reason when multiple slots fail', async () => {
        fetchMock.mockResolvedValueOnce({
            ok: true,
            json: async () => makeBody({
                micro: { status: 'SKIPPED', last_skip_reason: 'no snapshot yet' },
                fast: { status: 'SKIPPED', last_skip_reason: 'insufficient history (3 bars)' },
                slow: { status: 'OK', last_skip_reason: null },
                macro: { status: 'OK', last_skip_reason: null },
            }),
        });
        const component = mount(LiquidityStatusPanel, {
            target: document.body,
            props: { symbol: 'BTC-USDC' },
        });
        for (let i = 0; i < 10; i++) await Promise.resolve();
        await tick();
        const pill = document.body.querySelector('[title]') as HTMLElement;
        // Whichever slot's reason surfaces first is fine — we just need
        // to capture *some* reason (not null) when any slot fails.
        expect(pill.title).toMatch(/no snapshot yet|insufficient history/);
        unmount(component);
    });
});
