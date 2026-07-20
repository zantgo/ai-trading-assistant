// Shared color tokens + small style helpers used by every Metrics facet.
// Keeps the visual vocabulary consistent: bullish / bearish / neutral /
// extreme / confirmed / active across the redesigned page.

export const COLORS = {
    bullish: '#4ade80',
    bearish: '#f87171',
    neutral: '#f59e0b',
    extreme: '#c084fc',
    confirmed: '#22d3ee',
    inactive: 'rgba(255,255,255,0.25)',
    muted: 'rgba(255,255,255,0.4)',
    text: 'rgba(255,255,255,0.85)',
    textMuted: 'rgba(255,255,255,0.55)',
    border: 'rgba(255,255,255,0.08)',
    surface: 'rgba(255,255,255,0.02)',
    surfaceHover: 'rgba(255,255,255,0.04)',
} as const;

/** Color for a normalized value in [-1, +1]. */
export function normColor(n: number | undefined | null): string {
    if (n == null || isNaN(n)) return COLORS.inactive;
    const mag = Math.min(Math.abs(n), 1);
    if (mag >= 0.9) return COLORS.extreme;
    if (n > 0.1) return COLORS.bullish;
    if (n < -0.1) return COLORS.bearish;
    return COLORS.neutral;
}

/** Color for a direction enum. */
export function dirColor(d: 'Bullish' | 'Bearish' | 'Neutral' | undefined | null): string {
    if (d === 'Bullish') return COLORS.bullish;
    if (d === 'Bearish') return COLORS.bearish;
    return COLORS.neutral;
}

/** CSS class token (matches the .bull / .bear / .neutral CSS rules). */
export function dirClass(d: 'Bullish' | 'Bearish' | 'Neutral' | undefined | null): string {
    if (d === 'Bullish') return 'bull';
    if (d === 'Bearish') return 'bear';
    return 'neutral';
}

/** Pretty-format a confidence value (0..1) as a 0–100 integer percent. */
export function confPct(c: number | undefined | null): number {
    if (c == null || isNaN(c)) return 0;
    return Math.round(c * 100);
}

/** Age label for signals (0 → "now", n → "Nb"). */
export function ageLabel(age_bars: number | undefined | null): string {
    const a = age_bars ?? 0;
    return a === 0 ? 'now' : `${a}b`;
}

/** Truncate text to max length with ellipsis. */
export function truncate(s: string, max: number): string {
    if (!s) return '';
    return s.length <= max ? s : s.slice(0, max - 1) + '…';
}
