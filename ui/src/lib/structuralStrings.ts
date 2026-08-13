// Shared canonical string builders for the Structural Anchors strip,
// the Levels facet, and the metrics/MTF export builders.
//
// Every surface (panel strip, facet view, JSON export) must render the
// exact same human-readable sentence for the same underlying numbers —
// these helpers are the single source of truth for those strings.

export interface FibGpLike {
  gp_top: number | null;
  gp_bottom: number | null;
}

/**
 * Human-readable position sentence of `markPrice` vs the Fibonacci Golden
 * Pocket zone. Mirrors the strip's formula (relative-to-level distances):
 *   - inside:   "INSIDE GP (-X.XX% from center)"  (magnitude, half-range basis)
 *   - above:    "+X.XX% ABOVE GP"
 *   - below:    "X.XX% BELOW GP"
 *   - unknown:  "NO DATA"
 */
export function fibStatusString(
  gpTop: number | null,
  gpBottom: number | null,
  markPrice: number | null,
): string {
  if (gpTop == null || gpBottom == null || !isFinite(markPrice ?? NaN) || (markPrice ?? 0) <= 0) {
    return 'NO DATA';
  }
  const lo = Math.min(gpTop, gpBottom);
  const hi = Math.max(gpTop, gpBottom);
  if (markPrice! >= lo && markPrice! <= hi) {
    const halfRange = (hi - lo) / 2;
    const dist = Math.abs(markPrice! - (lo + halfRange)) / Math.max(halfRange, 1e-9);
    return `INSIDE GP (-${(dist * 100).toFixed(2)}% from center)`;
  }
  const ref = markPrice! > hi ? hi : lo;
  const pct = ((markPrice! - ref) / ref) * 100;
  const sign = pct >= 0 ? '+' : '';
  return markPrice! > hi
    ? `${sign}${pct.toFixed(2)}% ABOVE GP`
    : `${pct.toFixed(2)}% BELOW GP`;
}

/** Swing direction sentence derived from the fibonacci normalized value. */
export function fibSwingLabel(norm: number | null): string {
  if (norm == null) return 'NEUTRAL SWING';
  if (norm > 0.05) return 'BULL SWING';
  if (norm < -0.05) return 'BEAR SWING';
  return 'NEUTRAL SWING';
}

export interface VolumeProfileLike {
  value_area_high: number;
  value_area_low: number;
  range_high: number;
  range_low: number;
  poc_price: number;
}

/**
 * Position sentence of `markPrice` vs the value area (the canonical label
 * shown in the anchors strip badge, the Levels facet, and the JSON export):
 *   - inside:   "INSIDE VALUE AREA"
 *   - above:    "+X.XX% ABOVE VAH"
 *   - below:    "X.XX% BELOW VAL"
 */
export function vpPositionLabel(vp: VolumeProfileLike | null, markPrice: number | null): string {
  if (!vp) return '';
  const price = markPrice ?? vp.poc_price;
  if (!isFinite(price) || price <= 0) return '';
  if (price > vp.value_area_high) {
    return `+${(((price - vp.value_area_high) / vp.value_area_high) * 100).toFixed(2)}% ABOVE VAH`;
  }
  if (price < vp.value_area_low) {
    return `${(((vp.value_area_low - price) / price) * 100).toFixed(2)}% BELOW VAL`;
  }
  return 'INSIDE VALUE AREA';
}
