// Filtering helpers used by the redesigned Metrics view.
//
// The redesign uses filter pills (e.g. "Active signals only", "Confirmed+",
// "Hide gates", "Hide overlays") plus a free-text search bar. These pure
// functions are the canonical implementations shared by all six facet views
// so behavior is consistent.

import type {
    IndicatorMeta,
    IndicatorSignal,
    SignalKind,
} from '../types';

export interface FilterState {
    /** Free-text search: matches display_name, signal label, level name. */
    query: string;
    /** When true, rows with no active signals are hidden. */
    activeOnly: boolean;
    /** When true, Potential signals are hidden (only Confirmed / Active pass). */
    confirmedPlusOnly: boolean;
    /** When true, non-directional gate indicators are hidden. */
    hideGates: boolean;
    /** When true, price-overlay / price-levels / marker indicators are hidden
     *  (these belong on the chart and in the Structural Anchors Strip, not
     *  in the per-pane indicators table). */
    hideOverlays: boolean;
    /** Optional SignalKind whitelist — if non-empty, only these kinds pass. */
    kinds: SignalKind[];
}

/** Default filter state — everything visible. */
export function defaultFilters(): FilterState {
    return {
        query: '',
        activeOnly: false,
        confirmedPlusOnly: false,
        hideGates: false,
        hideOverlays: false,
        kinds: [],
    };
}

/** Case-insensitive substring match. Empty needle passes everything. */
export function matchesQuery(haystack: string | undefined | null, needle: string): boolean {
    if (!needle) return true;
    if (!haystack) return false;
    return haystack.toLowerCase().includes(needle.toLowerCase());
}

/**
 * Filter the registry against the global filter state.
 *
 *   - `hideGates: true`    → drops non-directional indicators (Volume, RVOL, ATR, BBWP, HV,
 *                            Choppiness, Funding, Spread, OI).
 *   - `hideOverlays: true` → drops rows whose `render` is anything other than
 *                            `Pane` (drops `PriceOverlay`, `PriceLevels`, and
 *                            `Marker` indicators — they have dedicated UI
 *                            surfaces on the chart and in the Structural
 *                            Anchors Strip).
 *   - `query`              → matches against `display_name` and `key`.
 *   - `activeOnly`         → drops indicators whose snapshot entry has no signals.
 *                            (Caller must provide `signalsFor(key)` — see
 *                            `filterRegistryWithSignals` below.)
 */
export function filterRegistry(
    registry: IndicatorMeta[],
    filters: FilterState,
    signalsFor?: (key: string) => IndicatorSignal[],
): IndicatorMeta[] {
    return registry.filter((m) => {
        if (!m.default_enabled) return false;
        if (filters.hideGates && !m.directional) return false;
        if (filters.hideOverlays && m.render !== 'Pane') return false;
        if (filters.query) {
            const hit = matchesQuery(m.display_name, filters.query)
                     || matchesQuery(m.key, filters.query);
            if (!hit) return false;
        }
        if (filters.activeOnly) {
            const sigs = signalsFor ? signalsFor(m.key) : [];
            if (sigs.length === 0) return false;
        }
        return true;
    });
}

/** Filter an indicator's signals against the filter state. */
export function filterSignals(
    signals: IndicatorSignal[],
    filters: FilterState,
): IndicatorSignal[] {
    if (!signals) return [];
    return signals.filter((s) => {
        if (filters.kinds.length > 0 && !filters.kinds.includes(s.kind)) return false;
        if (filters.confirmedPlusOnly && s.status === 'Potential') return false;
        if (filters.query && !matchesQuery(s.label, filters.query)) return false;
        return true;
    });
}
