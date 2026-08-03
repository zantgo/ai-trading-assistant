import type { AppStore } from '../state.svelte';
import type { IndicatorMap, TimeframeTelemetry } from '../types';

export type ChartSlot = 'micro' | 'fast' | 'slow' | 'macro';

export interface ChartCoalescer {
    /// Body for the chart's `$effect`. When `tf.latestSnapshot` mutates,
    /// Svelte calls this. It schedules a single `requestAnimationFrame`
    /// callback per frame, collapsing multiple broadcasts in one frame
    /// into one batched chart redraw. At sub-60s timeframes the analyzer
    /// can broadcast 50+ Hz; without coalescing every chart redraws on
    /// every broadcast, freezing the UI.
    effect: () => void;
    /// Cancel any pending rAF and stop further callbacks. Call from
    /// `onDestroy` so a teardown-during-pending-frame can't fire
    /// `series.update` on a destroyed chart.
    destroy: () => void;
}

/// Wrap a per-snapshot chart update so that at most one redraw runs per
/// browser frame, regardless of how often `tf.latestSnapshot` mutates.
///
/// `tick` receives the **latest** snapshot visible at the moment the rAF
/// fires (not the snapshot that originally triggered the effect), so
/// rapid bursts collapse to a single redraw using the freshest data.
///
/// `pairKey` and `slot` are accepted as **getter functions** rather
/// than raw values so callers can pass `() => pairKey` / `() => slot`
/// when those are `$props` / `$state` in a Svelte 5 component — this
/// avoids the `state_referenced_locally` warning (the captured value
/// is the getter closure, not the reactive value itself; the getter
/// is invoked at every `readTf()`).
export function makeChartCoalescer(
    app: AppStore,
    pairKey: string | (() => string),
    slot: ChartSlot | (() => ChartSlot),
    tick: (snap: { timestamp: number; open?: unknown; high?: unknown; low?: unknown; close?: unknown; volume?: unknown; indicators?: IndicatorMap | null }, tfVal: TimeframeTelemetry) => void,
): ChartCoalescer {
    let pending = false;
    let destroyed = false;

    const getPairKey = (): string =>
        typeof pairKey === 'function' ? (pairKey as () => string)() : pairKey;
    const getSlot = (): ChartSlot =>
        typeof slot === 'function' ? (slot as () => ChartSlot)() : slot;

    function readTf(): { snap: any; tfVal: TimeframeTelemetry } | null {
        const curSlot = getSlot();
        const pairVal = app.instancesMap[getPairKey()];
        if (!pairVal) return null;
        const tfVal =
            curSlot === 'micro' ? pairVal.microTerm :
            curSlot === 'fast'  ? pairVal.fastTerm  :
            curSlot === 'slow'  ? pairVal.slowTerm  :
                                  pairVal.macroTerm;
        const snap = tfVal?.latestSnapshot;
        if (!snap) return null;
        return { snap, tfVal };
    }

    return {
        effect: () => {
            if (destroyed || pending) return;
            const first = readTf();
            if (!first) return;
            pending = true;
            requestAnimationFrame(() => {
                pending = false;
                if (destroyed) return;
                const next = readTf();
                if (!next) return;
                tick(next.snap, next.tfVal);
            });
        },
        destroy: () => { destroyed = true; },
    };
}
