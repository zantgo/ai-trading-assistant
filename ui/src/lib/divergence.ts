// Divergence classification — derives one of the 4 canonical divergence
// sub-types (Regular Bull / Regular Bear / Hidden Bull / Hidden Bear) from
// the `points[]` field of an `IndicatorSignal`. The Rust analyzer also tags
// the label with the sub-type name, so we have a primary path (label parse)
// and a fallback (slope-based derivation from points).
//
// The 4 sub-types come from the sign of the (price-slope, oscillator-slope)
// pair across the two pivots in `points[]`:
//   - Regular Bull:  price LL, osc HL   → exhaustion → reversal up
//   - Regular Bear:  price HH, osc LH   → exhaustion → reversal down
//   - Hidden Bull:   price HL, osc LL   → continuation up
//   - Hidden Bear:   price LH, osc HH   → continuation down

import type { SignalPoint } from '../types';

export type DivergenceSubKind =
    | 'RegularBull'
    | 'RegularBear'
    | 'HiddenBull'
    | 'HiddenBear'
    | 'Unknown';

/**
 * Parse a divergence sub-type out of an `IndicatorSignal.label`.
 *
 * Backend labels for divergence are typically of the form
 * `BULLISH_DIVERGENCE`, `BEARISH_DIVERGENCE`, `HIDDEN_BULLISH_DIVERGENCE`,
 * `HIDDEN_BEARISH_DIVERGENCE`, `REGULAR_BULLISH_DIVERGENCE`,
 * `REGULAR_BEARISH_DIVERGENCE`, `BULLISH_DIVERGENCE_CONFIRMED`, etc.
 *
 * Anything that doesn't contain `DIVERGENCE` returns `'Unknown'` so callers
 * can decide whether to fall back to slope derivation.
 */
export function parseDivergenceLabel(label: string | undefined | null): DivergenceSubKind {
    if (!label) return 'Unknown';
    const l = label.toUpperCase();

    if (!l.includes('DIVERGENCE')) return 'Unknown';

    const isHidden = l.includes('HIDDEN');
    const isBull = l.includes('BULL');
    const isBear = l.includes('BEAR');

    if (isBull && isHidden) return 'HiddenBull';
    if (isBear && isHidden) return 'HiddenBear';
    if (isBull) return 'RegularBull';
    if (isBear) return 'RegularBear';
    return 'Unknown';
}

/**
 * Derive divergence sub-type from `SignalPoint[]` (fallback when label is
 * ambiguous). The points array carries two pivot coordinates:
 *   - point[0] → previous pivot (older bar)
 *   - point[1] → current pivot (newer bar)
 *
 * For divergence-bearing indicators each point's `value` is the oscillator
 * reading at that pivot; the time axis alone tells us the price pivot price
 * (we don't have price in points, so we approximate using the time axis
 * ordering alone — the slope sign comes from `value` deltas, and the price
 * slope sign is implied by the time axis inversion used by the analyzer:
 * the most recent pivot is always point[1]).
 *
 * Note: the analyzer labels carry the authoritative sub-type; this is just a
 * defensive fallback when `points` exists but the label is missing.
 */
export function deriveDivergenceFromPoints(
    points: SignalPoint[] | null | undefined,
    direction: 'Bullish' | 'Bearish' | 'Neutral',
): DivergenceSubKind {
    if (!points || points.length < 2) return 'Unknown';
    const [a, b] = points;
    const oscUp = b.value > a.value;
    // The "price slope" is implicit in which pivot is the newer one; the
    // analyzer orders points oldest-first so a HH setup means the newer
    // pivot is higher on the time axis — we approximate that with the
    // direction field already on the signal: if `direction === 'Bullish'`
    // the sub-type is on the bullish side of the taxonomy.
    if (direction === 'Bullish') return oscUp ? 'RegularBull' : 'HiddenBull';
    if (direction === 'Bearish') return oscUp ? 'HiddenBear' : 'RegularBear';
    return 'Unknown';
}

/** Human-readable sub-type label for UI rendering. */
export function divergenceLabel(sub: DivergenceSubKind): string {
    switch (sub) {
        case 'RegularBull': return 'Regular Bull';
        case 'RegularBear': return 'Regular Bear';
        case 'HiddenBull':  return 'Hidden Bull';
        case 'HiddenBear':  return 'Hidden Bear';
        case 'Unknown':     return 'Unknown';
    }
}

/** Color hint for divergence sub-types — green for bull, red for bear, gray for hidden. */
export function divergenceAccent(sub: DivergenceSubKind): string {
    switch (sub) {
        case 'RegularBull': return '#4ade80';
        case 'RegularBear': return '#f87171';
        case 'HiddenBull':  return '#86efac';
        case 'HiddenBear':  return '#fca5a5';
        case 'Unknown':     return 'rgba(255,255,255,0.4)';
    }
}

/**
 * Classify a signal end-to-end: try the label first, then fall back to the
 * points[] slope derivation.
 */
export function classifyDivergence(
    label: string | undefined | null,
    points: SignalPoint[] | null | undefined,
    direction: 'Bullish' | 'Bearish' | 'Neutral',
): DivergenceSubKind {
    const fromLabel = parseDivergenceLabel(label);
    if (fromLabel !== 'Unknown') return fromLabel;
    return deriveDivergenceFromPoints(points, direction);
}
