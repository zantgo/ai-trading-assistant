/**
 * Canonical cascade-asymmetry display classification (v2026-08).
 *
 * Shared vocabulary + thresholds with metricsTab.ts and riskTab.ts:
 * positive = short squeeze risk, negative = long squeeze risk, with a
 * ±0.3 dead-band (the L4/L5 threshold documented in 03-02-11). The
 * LiquidityPanel cluster tab renders these tokens; the metrics export
 * mirrors them lowercase for payload consumers.
 */

export function cascadeAsymmetryLabel(asym: number | null | undefined): 'SHORT_SQUEEZE_RISK' | 'LONG_SQUEEZE_RISK' | 'NEUTRAL' | null {
    if (asym == null || !Number.isFinite(asym)) return null;
    if (asym > 0.3) return 'SHORT_SQUEEZE_RISK';
    if (asym < -0.3) return 'LONG_SQUEEZE_RISK';
    return 'NEUTRAL';
}

export function cascadeAsymmetryIsBullish(asym: number | null | undefined): boolean {
    return cascadeAsymmetryLabel(asym) === 'SHORT_SQUEEZE_RISK';
}

export function cascadeAsymmetryIsBearish(asym: number | null | undefined): boolean {
    return cascadeAsymmetryLabel(asym) === 'LONG_SQUEEZE_RISK';
}
