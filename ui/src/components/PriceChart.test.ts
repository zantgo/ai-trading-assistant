// @vitest-environment jsdom
//
// Regression tests for the early-return reactivity trap in
// `PriceChart.svelte` overlay `$effect`s.
//
// # Why these tests exist
//
// Svelte 5's `$effect` rune only registers dependencies for values it
// reads *during its execution*. If an `$effect` returns early before
// reading a reactive value, that value is never tracked and subsequent
// mutations of it never re-trigger the effect.
//
// `PriceChart.svelte` had six `$effect` blocks (heatmap, volume profile,
// EMA visibility, BB visibility, VWAP visibility, candle-vs-line mode)
// where the early return `if (!volumeProfilePrim) return;` (and similar
// guards for the other primitives) fired before the effect read
// `tf?.showVolumeProfile` (and similar toggle flags). On first mount,
// those primitives are `null` — they're only assigned in `onMount` —
// so the early return triggered, the dependencies went untracked, and
// the overlay toggles never actually drove `updateData()`.
//
// The fix was to read the toggle flag and data into local variables
// *before* the early-return guard. `OverlayEffectHarness.svelte` pins
// down the exact pattern (primitive = null initially → onMount assigns
// it → effect reads dependencies before the guard) so a future
// regression that re-introduces the early-return trap is caught
// immediately.
//
// We don't render the real `PriceChart.svelte` here because the
// lightweight-charts canvas + the `$state` runes inside `state.svelte.ts`
// require a Svelte-compiled module to function; testing the full
// component requires `vite-plugin-svelte` to compile it, which the
// project's vitest config currently does not for arbitrary `.ts` test
// files. The harness exercises the *exact pattern* with a `.svelte`
// component so it does get compiled.

import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { mount, unmount } from 'svelte';
import { flushSync } from 'svelte';

import OverlayEffectHarness from './OverlayEffectHarness.svelte';

const cell = () => (globalThis as any).__overlayCell;

beforeEach(() => {
    (globalThis as any).__overlayCalls = [];
});

afterEach(() => {
    document.body.innerHTML = '';
});

function calls(): unknown[][] {
    return (globalThis as any).__overlayCalls;
}

describe('Overlay $effect early-return trap (regression: PriceChart toggle wiring)', () => {
    it('feeds the primitive updateData with the snapshot when the toggle is on', () => {
        mount(OverlayEffectHarness, { target: document.body });
        flushSync();

        // Initial render: the effect runs synchronously while primitive
        // is still null (early return). onMount then assigns the
        // primitive and the effect re-runs. showOverlay=false → null.
        expect(calls().at(-1)).toEqual([null]);

        // Toggle on. The effect must re-run and feed the data in.
        // WITHOUT the dependency-before-guard fix, the effect never
        // re-runs and `calls.at(-1)` stays at [null].
        cell().data = { value: 'alpha' };
        cell().showOverlay = true;
        flushSync();
        expect(calls().at(-1)).toEqual([{ value: 'alpha' }]);

        // Toggle off: passes null again.
        cell().showOverlay = false;
        flushSync();
        expect(calls().at(-1)).toEqual([null]);

        unmount(OverlayEffectHarness);
    });

    it('reacts to data updates while the toggle stays on', () => {
        cell().showOverlay = true;
        cell().data = { value: 'one' };
        mount(OverlayEffectHarness, { target: document.body });
        flushSync();

        expect(calls().at(-1)).toEqual([{ value: 'one' }]);

        cell().data = { value: 'two' };
        flushSync();
        expect(calls().at(-1)).toEqual([{ value: 'two' }]);

        unmount(OverlayEffectHarness);
    });

    it('updates the primitive when both toggle and data change in the same tick', () => {
        cell().showOverlay = false;
        cell().data = { value: 'a' };
        mount(OverlayEffectHarness, { target: document.body });
        flushSync();

        expect(calls().at(-1)).toEqual([null]);

        // Toggle + data change together. Before the fix the early return
        // would swallow the change and the primitive would stay frozen
        // on null.
        cell().showOverlay = true;
        cell().data = { value: 'b' };
        flushSync();
        expect(calls().at(-1)).toEqual([{ value: 'b' }]);

        unmount(OverlayEffectHarness);
    });
});