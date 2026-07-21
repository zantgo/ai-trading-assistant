// Declarative chart spec — let each chart describe what it needs, then
// pull both historical bootstrap and live updates through the SAME
// resolver. This guarantees that the last historical bar and the first
// WS live frame reconcile cleanly (live wins on a timestamp collision).

import type { Time } from 'lightweight-charts';
import type { IndicatorMap } from '../types';
import type { IndicatorFlatHistory } from './indicatorHistory';
import {
    historyValue,
    lastHistoricalTime,
    pairsFromHistory,
} from './indicatorHistory';

/// One series descriptor inside a chart.
///
/// `kind` — discriminator:
///   'raw'          → `IndicatorMap[key]?.raw_value`
///   'sub'          → `IndicatorMap[key]?.values?.[subKey]`
///   'candles'      → candle close (used by PriceChart for the line-mode toggle)
///   'state_label'  → `IndicatorMap[key]?.state_label` (string[]) (rare; for mode-based charts)
export type SeriesKind = 'raw' | 'sub' | 'candles' | 'state_label';

export interface SeriesSpec {
    id: string;             // local id used by the consumer (e.g. 'k_line', 'upper', 'line')
    kind: SeriesKind;
    key: string;            // backend indicator key (or 'candles' for OHLC)
    subKey?: string;        // backend values sub-key (only when kind === 'sub')
    /// Optional value-side filter. `'nonzero'` drops 0.0 entries from the
    /// historical series (used by OFI / OI-Δ to avoid drawing false bars
    /// at the empty-book baseline). Defaults to 'all'.
    filter?: 'all' | 'nonzero';
}

export interface ChartSpec {
    /// Multi-series descriptor. Use one entry per line / histogram series
    /// in the chart. Empty means "no historical bootstrap needed for this
    /// chart; subscribe to live only" — currently used by SMC primitives
    /// which only consume the latest snapshot.
    series: SeriesSpec[];
}

/// Read every series declared in `spec` from the unified history payload.
/// Returns an array of `(id, points[])` pairs aligned to `times`. Series
/// without historical data yield `[]` — the consumer renders an empty
/// chart and lets live frames fill it.
export function readHistory(
    hist: IndicatorFlatHistory | null | undefined,
    spec: ChartSpec,
): Array<{ id: string; points: Array<{ time: Time; value: number }> }> {
    const out: Array<{ id: string; points: Array<{ time: Time; value: number }> }> = [];
    for (const s of spec.series) {
        if (s.kind === 'candles') {
            // Candle close series (used by PriceChart only).
            if (!hist) {
                out.push({ id: s.id, points: [] });
                continue;
            }
            const arr: Array<{ time: Time; value: number }> = [];
            for (let i = 0; i < hist.candles.close.length && i < hist.candleTimes.length; i++) {
                const t = hist.candleTimes[i];
                const v = hist.candles.close[i];
                if (t == null || v == null) continue;
                arr.push({ time: t as Time, value: v });
            }
            out.push({ id: s.id, points: arr });
            continue;
        }
        const subKey = s.kind === 'sub' ? s.subKey : undefined;
        let points = pairsFromHistory(hist, s.key, subKey);
        if (s.filter === 'nonzero') {
            points = points.filter((p) => p.value !== 0 && Number.isFinite(p.value));
        }
        out.push({ id: s.id, points });
    }
    return out;
}

/// Read the latest value of each declared series from a live snapshot.
/// Returns `{ id → value | null }`. `null` is a legitimate "indicator
/// hasn't emitted yet" and must not be confused with 0 (which is a
/// real reading).
export function readLive(
    snap: { indicators?: IndicatorMap | null } | null | undefined,
    spec: ChartSpec,
): Record<string, number | null> {
    const out: Record<string, number | null> = {};
    if (!snap) {
        for (const s of spec.series) out[s.id] = null;
        return out;
    }
    const m = (snap.indicators ?? {}) as IndicatorMap;
    for (const s of spec.series) {
        if (s.kind === 'candles') {
            out[s.id] = null;
            continue;
        }
        if (s.kind === 'sub') {
            const v = m[s.key]?.values?.[s.subKey ?? ''];
            out[s.id] = typeof v === 'number' && Number.isFinite(v) ? v : null;
        } else if (s.kind === 'state_label') {
            // String-encoded; consumers usually want a number; skip if not numeric.
            const lbl = m[s.key]?.state_label;
            // Some series expose a numeric concentration under a heuristic
            // sub-key (e.g. squeeze "COMPRESSION_COILING" maps to 1/0 via
            // squeeze_on). For now callers should derive their own.
            out[s.id] = typeof lbl === 'number' ? lbl : null;
        } else {
            const v = m[s.key]?.raw_value;
            out[s.id] = typeof v === 'number' && Number.isFinite(v) ? v : null;
        }
    }
    return out;
}

/// Merge historical + live data such that, on a timestamp collision, the
/// LIVE value wins. Used to drop a phantom marker at the bootstrap
/// boundary when both history and live have populated the same bar.
///
/// Pass `snap.timestamp` (seconds) as `liveTime`; pass `lastHistorical`
/// (the value from `lastHistoricalTime(hist)`).
///
/// Returns `null` if the snapshot is not relevant (i.e. `liveTime !== lastHistorical`)
/// — i.e. it appends to history rather than replacing it.
export function resolveBootstrapBoundary(
    history: Array<{ time: Time; value: number }>,
    liveTime: number | null | undefined,
    liveValue: number | null,
): Array<{ time: Time; value: number }> | null {
    if (liveTime == null || liveValue == null) return null;
    if (history.length === 0) return null;
    const lastHist = history[history.length - 1];
    const lastHistTime = lastHist.time as unknown as number;
    if (lastHistTime !== liveTime) return null;
    // Replace the last entry with the live value.
    return [...history.slice(0, -1), { time: liveTime as Time, value: liveValue }];
}

/// Build the canonical descriptive label for a chart from a spec. Used in
/// snapshot filenames and screen-reader regions.
export function describeSpec(spec: ChartSpec): string {
    return spec.series.map((s) => {
        if (s.kind === 'candles') return 'candles';
        if (s.kind === 'sub') return `${s.key}.${s.subKey ?? ''}`;
        return s.key;
    }).join('+');
}

/// Re-export the boundary helper consumers reach for.
export { lastHistoricalTime };
