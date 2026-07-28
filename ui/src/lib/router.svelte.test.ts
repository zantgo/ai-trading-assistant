// @vitest-environment jsdom
// Router contract tests for `lib/router.svelte.ts`.
//
// These exercise the round-trip between URLs and `RouteParams` and the
// subtle back-button path that broke the dashboard in Bug 3. The
// runtime consumers (`App.svelte::applyRoute`) layer additional
// semantics on top — `middleTab` defaults to `overview` when the URL
// omits it, and `currentView` defaults to `terminal` — but those are
// covered by the integration scenarios in `deleteInstance.test.ts`
// and the manual regression checklist.

import { describe, expect, it } from 'vitest';
import { buildEngineHash, parseEngineHash, hashEquals } from './router.svelte';

describe('buildEngineHash / parseEngineHash round-trip', () => {
    it('round-trips a bare engine', () => {
        const hash = buildEngineHash('market_monitor');
        expect(hash).toBe('#/engine/market_monitor');
        const parsed = parseEngineHash(hash);
        expect(parsed).toEqual({ engine: 'market_monitor' });
    });

    it('round-trips engine + middleTab', () => {
        const hash = buildEngineHash('market_monitor', 'workspace');
        expect(hash).toBe('#/engine/market_monitor/workspace');
        const parsed = parseEngineHash(hash);
        expect(parsed).toEqual({ engine: 'market_monitor', middleTab: 'workspace' });
    });

    it('round-trips engine + middleTab + instance + view', () => {
        const hash = buildEngineHash('market_monitor', 'workspace', 'BTC-USDT', 'monitor');
        expect(hash).toBe('#/engine/market_monitor/workspace/instance/BTC-USDT/view/monitor');
        const parsed = parseEngineHash(hash);
        expect(parsed).toEqual({
            engine: 'market_monitor',
            middleTab: 'workspace',
            instance: 'BTC-USDT',
            view: 'monitor',
        });
    });

    it('omits middleTab when engine is exchange_settings (canonical URL)', () => {
        const hash = buildEngineHash('exchange_settings', undefined, undefined, undefined);
        expect(hash).toBe('#/engine/exchange_settings');
        const parsed = parseEngineHash(hash);
        expect(parsed).toEqual({ engine: 'exchange_settings' });
    });

    it('handles a URL with engine + view but no middleTab (legacy / direct-link)', () => {
        // The user can hand-edit a URL like `#/engine/market_monitor/instance/BTC-USDT/view/monitor`
        // (skipping middleTab) — the parser must still surface the
        // instance and view, and the round-trip must remain stable.
        const hash = '#/engine/market_monitor/instance/BTC-USDT/view/monitor';
        const parsed = parseEngineHash(hash);
        expect(parsed).toEqual({
            engine: 'market_monitor',
            instance: 'BTC-USDT',
            view: 'monitor',
        });
        // Re-build from the parsed params and compare — should match the
        // canonical form once middleTab is supplied.
        const rebuilt = buildEngineHash(parsed!.engine, parsed!.middleTab, parsed!.instance, parsed!.view);
        expect(rebuilt).toBe('#/engine/market_monitor/instance/BTC-USDT/view/monitor');
    });

    it('returns null for an empty hash', () => {
        expect(parseEngineHash('')).toBeNull();
        expect(parseEngineHash('#')).toBeNull();
        expect(parseEngineHash('#/')).toBeNull();
    });

    it('returns null for a non-engine hash', () => {
        expect(parseEngineHash('#/random/path')).toBeNull();
        expect(parseEngineHash('#something')).toBeNull();
    });
});

describe('hashEquals', () => {
    it('compares hashes ignoring the leading "#" vs "#/"', () => {
        expect(hashEquals('#/engine/market_monitor', '#engine/market_monitor')).toBe(true);
        expect(hashEquals('#/engine/market_monitor', '#/engine/data_infra')).toBe(false);
        expect(hashEquals('#/engine/market_monitor', '')).toBe(false);
    });
});

describe('back/forward sequences preserve engine, instance, and view', () => {
    // Bug 3 regression: when the user presses Back from a deep
    // workspace URL to a shallow engine URL, the URL parser must
    // surface every piece the runtime needs to rebuild the right
    // state. The full handler lives in `App.svelte`; this test only
    // locks the URL semantics.

    const sequence = [
        '#/engine/market_monitor',
        '#/engine/market_monitor/workspace',
        '#/engine/market_monitor/workspace/instance/BTC-USDT',
        '#/engine/market_monitor/workspace/instance/BTC-USDT/view/monitor',
        '#/engine/market_monitor/workspace/instance/BTC-USDT/view/risk',
    ];

    it('each step parses losslessly', () => {
        for (const hash of sequence) {
            const parsed = parseEngineHash(hash);
            expect(parsed, `failed to parse ${hash}`).not.toBeNull();
            // Round-trip after buildEngineHash must produce the same
            // canonical hash (modulo the optional middleTab absent in
            // the first entry).
            if (parsed!.middleTab) {
                expect(
                    buildEngineHash(parsed!.engine, parsed!.middleTab, parsed!.instance, parsed!.view),
                ).toBe(hash);
            }
        }
    });

    it('back from view/risk to view/monitor keeps the instance', () => {
        const before = parseEngineHash(sequence[3])!;
        const after = parseEngineHash(sequence[2])!;
        expect(after.engine).toBe(before.engine);
        expect(after.middleTab).toBe(before.middleTab);
        expect(after.instance).toBe(before.instance);
        expect(after.view).toBeUndefined();
    });

    it('back to a URL with no middleTab still surfaces engine', () => {
        // sequence[0] is `#/engine/market_monitor` — engine-only, no
        // middleTab, no instance. The runtime defaults middleTab to
        // 'overview' for market_monitor (see `App.svelte::applyRoute`),
        // so back-navigation to this URL must leave the dashboard on
        // the overview tab even when state thinks it's on `workspace`.
        const parsed = parseEngineHash(sequence[0])!;
        expect(parsed.engine).toBe('market_monitor');
        expect(parsed.middleTab).toBeUndefined();
        expect(parsed.instance).toBeUndefined();
        expect(parsed.view).toBeUndefined();
    });
});