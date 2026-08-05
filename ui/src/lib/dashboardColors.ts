// Shared color helpers for the Market Overview dashboard.
//
// All sub-components import from here so the visual vocabulary stays
// DRY across the redesign. The trade-off between Risk-as-danger
// (RiskMatrix.score: high = dangerous) and the dashboard's Quality
// framing (high = good) is reconciled by the `invert` flag on
// `riskBarColor` — risk *scores* are coloured red, but the dashboard
// renders them as Quality bars (e.g. Liquidity bar = 100 - risk.score)
// with the colour derived from the *quality* value, not the risk score.

export const DASHBOARD_COLORS = {
    // Semantic
    bullish: '#4ade80',
    bearish: '#f87171',
    neutral: '#f59e0b',
    info: '#22d3ee',
    inactive: 'rgba(255,255,255,0.25)',

    // Surfaces
    surface: 'rgba(255,255,255,0.02)',
    surfaceStrong: 'rgba(255,255,255,0.04)',
    border: 'rgba(255,255,255,0.06)',
    borderStrong: 'rgba(255,255,255,0.12)',
    text: 'rgba(255,255,255,0.85)',
    textMuted: 'rgba(255,255,255,0.55)',
    textDim: 'rgba(255,255,255,0.35)',
} as const;

/**
 * Colour for a Wire `MarketBias` value (StrongBullish / Bullish /
 * Neutral / Bearish / StrongBearish). The neutral band is intentionally
 * amber — operators read neutral as "wait, not strong / weak" rather
 * than as "no data".
 */
export function biasColor(bias: string | null | undefined): string {
    if (!bias) return DASHBOARD_COLORS.neutral;
    // Normalize for both CamelCase (AnalysisMatrix.bias — 'StrongBullish')
    // and SCREAMING_SNAKE_CASE (OverviewMatrix.global_market_bias —
    // 'STRONG_BULLISH'). Strip underscores so a single suffix check
    // covers both wire formats.
    const b = bias.toUpperCase().replaceAll('_', '');
    const isStrong = b.startsWith('STRONG');
    const isBull = b.includes('BULL') || b === 'LONG';
    const isBear = b.includes('BEAR') || b === 'SHORT';
    if (isBull && isStrong) return '#22c55e';
    if (isBear && isStrong) return '#dc2626';
    if (isBull) return DASHBOARD_COLORS.bullish;
    if (isBear) return DASHBOARD_COLORS.bearish;
    return DASHBOARD_COLORS.neutral;
}

/**
 * Colour for a Risk-as-danger score (0..100). High score = dangerous.
 */
export function riskDangerColor(score: number | null | undefined): string {
    if (score == null || !isFinite(score)) return DASHBOARD_COLORS.inactive;
    if (score >= 70) return DASHBOARD_COLORS.bearish;
    if (score >= 50) return DASHBOARD_COLORS.neutral;
    if (score >= 30) return DASHBOARD_COLORS.bullish;
    return '#22c55e';
}

/**
 * Colour for a Quality/value score (0..100). High score = good. Used
 * for the Dashboard's quality bars (Trend / Liquidity / Signal Stability
 * where the L5 risk score is inverted to a quality value).
 */
export function qualityColor(value: number | null | undefined): string {
    if (value == null || !isFinite(value)) return DASHBOARD_COLORS.inactive;
    if (value >= 70) return '#22c55e';
    if (value >= 50) return DASHBOARD_COLORS.bullish;
    if (value >= 30) return DASHBOARD_COLORS.neutral;
    return DASHBOARD_COLORS.bearish;
}

/**
 * Colour for a "BUY / SELL / WAIT" Signal token. The dashboard's
 * per-asset Signal column collapses `directional_guidance` to this
 * 3-token vocabulary.
 */
export function directionColor(direction: 'LONG' | 'SHORT' | 'NEUTRAL' | null | undefined): string {
    if (direction === 'LONG') return DASHBOARD_COLORS.bullish;
    if (direction === 'SHORT') return DASHBOARD_COLORS.bearish;
    return DASHBOARD_COLORS.neutral;
}

/**
 * Colour for an R:R value (1:N). Operator rule of thumb: ≥ 2.0 is
 * green (good trade), ≥ 1.0 is amber (acceptable), < 1.0 is red
 * (risk > reward).
 */
export function rrColor(rr: number | null | undefined): string {
    if (rr == null || !isFinite(rr) || rr <= 0) return DASHBOARD_COLORS.textMuted;
    if (rr >= 2.0) return '#22c55e';
    if (rr >= 1.0) return DASHBOARD_COLORS.neutral;
    return DASHBOARD_COLORS.bearish;
}

/**
 * Colour for a confidence / opportunity score (0..100).
 * Single canonical mapping used by Score columns and KPI tiles.
 */
export function scoreColor(score: number | null | undefined): string {
    if (score == null || !isFinite(score)) return DASHBOARD_COLORS.inactive;
    if (score >= 85) return '#22c55e';
    if (score >= 70) return DASHBOARD_COLORS.bullish;
    if (score >= 50) return DASHBOARD_COLORS.neutral;
    if (score >= 30) return '#fbbf24';
    return DASHBOARD_COLORS.bearish;
}

/**
 * Map a directional_guidance string to the 3-token dashboard
 * vocabulary. Returns the bare LONG/SHORT/NEUTRAL axis (so the
 * caller can decide whether to surface "BUY"/"SELL" or "LONG"/"SHORT").
 */
export function directionLabel(guidance: string | null | undefined): 'LONG' | 'SHORT' | 'NEUTRAL' {
    if (!guidance) return 'NEUTRAL';
    const g = guidance.toUpperCase();
    if (g.includes('LONG')) return 'LONG';
    if (g.includes('SHORT')) return 'SHORT';
    return 'NEUTRAL';
}

/**
 * Map a directional_guidance to the operator-facing Signal token:
 * "BUY" for LONG, "SELL" for SHORT, "WAIT" for Neutral.
 */
export function signalLabel(guidance: string | null | undefined): 'BUY' | 'SELL' | 'WAIT' {
    const dir = directionLabel(guidance);
    if (dir === 'LONG') return 'BUY';
    if (dir === 'SHORT') return 'SELL';
    return 'WAIT';
}

/**
 * Bucket an advisory `confidence_assessment` (0..100) into the
 * dashboard's 3-band signal-quality vocabulary.
 */
export function signalQualityBucket(confidence: number | null | undefined): 'STRONG' | 'MODERATE' | 'WEAK' {
    if (confidence == null || !isFinite(confidence)) return 'WEAK';
    if (confidence >= 70) return 'STRONG';
    if (confidence >= 40) return 'MODERATE';
    return 'WEAK';
}

/**
 * Format an R:R value for the asset table: `1 : 2.43` or `—` when no
 * meaningful R:R is available. Matches the operator's mental model.
 */
export function formatRR(rr: number | null | undefined): string {
    if (rr == null || !isFinite(rr) || rr <= 0) return '—';
    return `1 : ${rr.toFixed(2)}`;
}

/**
 * ASCII bar glyph (10 chars) for the regime distribution. The
 * dashboard uses a fixed-width bar so columns align across rows.
 */
export function asciiBar(pct: number, width = 10): string {
    const clamped = Math.max(0, Math.min(100, pct));
    const filled = Math.round((clamped / 100) * width);
    return '█'.repeat(filled) + '░'.repeat(width - filled);
}
