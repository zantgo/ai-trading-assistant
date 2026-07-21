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
export function makeChartCoalescer(
    app: AppStore,
    pairKey: string,
    slot: ChartSlot,
    tick: (snap: { timestamp: number; open?: unknown; high?: unknown; low?: unknown; close?: unknown; volume?: unknown; indicators?: IndicatorMap | null }, tfVal: TimeframeTelemetry) => void,
): ChartCoalescer {
    let pending = false;
    let destroyed = false;

    function readTf(): { snap: any; tfVal: TimeframeTelemetry } | null {
        const pairVal = app.instancesMap[pairKey];
        if (!pairVal) return null;
        const tfVal =
            slot === 'micro' ? pairVal.microTerm :
            slot === 'fast'  ? pairVal.fastTerm  :
            slot === 'slow'  ? pairVal.slowTerm  :
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
