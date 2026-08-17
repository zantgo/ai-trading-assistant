/**
 * Qualitative band for a confluent level's `strength` (0..100 additive
 * confidence weight of the sources aligned at that price — NOT a
 * probability). The panel renders the band instead of the raw number so
 * the meaning is self-evident; the raw weight stays available as a
 * tooltip and in the export payload.
 *
 * Bands are tuned to the backend weight formula (single-source levels
 * land at 5–30, two-source up to ~55, three-source up to ~70, four+
 * toward 100).
 */
export type ConfluenceTier = 'WEAK' | 'MODERATE' | 'STRONG' | 'VERY STRONG';

export function confluenceStrengthLabel(strength: number): ConfluenceTier {
    if (strength >= 80) return 'VERY STRONG';
    if (strength >= 55) return 'STRONG';
    if (strength >= 30) return 'MODERATE';
    return 'WEAK';
}
