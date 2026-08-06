import type { AppStore } from '../state.svelte';
import type { IndicatorMap, TimeframeTelemetry } from '../types';

export type ChartSlot = 'micro' | 'fast' | 'slow' | 'macro';

export interface ChartCoalescer {
    effect: () => void;
    destroy: () => void;
}

function deepUnwrap<T>(value: T): T {
    if (value == null || typeof value !== 'object') return value;
    try {
        return JSON.parse(JSON.stringify(value)) as T;
    } catch {
        return value;
    }
}

export function makeChartCoalescer(
    app: AppStore,
    pairKey: string | (() => string),
    slot: ChartSlot | (() => ChartSlot),
    tick: (
        snap: { timestamp: number; open?: unknown; high?: unknown; low?: unknown; close?: unknown; volume?: unknown; indicators?: IndicatorMap | null },
        tfPlain: TimeframeTelemetry,
    ) => void,
): ChartCoalescer {
    let pending = false;
    let destroyed = false;

    const getPairKey = (): string =>
        typeof pairKey === 'function' ? (pairKey as () => string)() : pairKey;
    const getSlot = (): ChartSlot =>
        typeof slot === 'function' ? (slot as () => ChartSlot)() : slot;

    function readTf(): { snap: { timestamp: number; open?: unknown; high?: unknown; low?: unknown; close?: unknown; volume?: unknown; indicators?: IndicatorMap | null }; tfPlain: TimeframeTelemetry } | null {
        const curSlot = getSlot();
        const pairVal = app.instancesMap[getPairKey()];
        if (!pairVal) return null;

        const tfVal =
            curSlot === 'micro' ? pairVal.microTerm :
            curSlot === 'fast'  ? pairVal.fastTerm  :
            curSlot === 'slow'  ? pairVal.slowTerm  :
                                  pairVal.macroTerm;

        const rawSnap = tfVal?.latestSnapshot;
        if (!rawSnap) return null;

        const snap = deepUnwrap(rawSnap) as { timestamp: number; open?: unknown; high?: unknown; low?: unknown; close?: unknown; volume?: unknown; indicators?: IndicatorMap | null };

        const ts = Number(snap.timestamp ?? 0);
        if (!Number.isFinite(ts) || ts <= 0) return null;
        snap.timestamp = ts;

        const plainIndicators = deepUnwrap(tfVal.indicators ?? null) as IndicatorMap | null;
        snap.indicators = plainIndicators;

        const tfPlain: TimeframeTelemetry = {
            ...deepUnwrap(tfVal),
            indicators: plainIndicators,
        } as TimeframeTelemetry;

        return { snap, tfPlain };
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
                tick(next.snap, next.tfPlain);
            });
        },
        destroy: () => { destroyed = true; },
    };
}
