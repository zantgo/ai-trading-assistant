// Pure accessors for the nested normalized indicator map (v2.0). These replace
// the removed flat `TimeframeTelemetry` indicator fields — the map is the sole
// source of truth. Callers pass either `tf.indicators` or `snapshot.indicators`.

import type { IndicatorMap, IndicatorDto } from '../types';

function entry(m: IndicatorMap | undefined | null, key: string): IndicatorDto | undefined {
    return m ? m[key] : undefined;
}

export function iRaw(m: IndicatorMap | undefined | null, key: string): number | null {
    return entry(m, key)?.raw_value ?? null;
}
export function iNorm(m: IndicatorMap | undefined | null, key: string): number {
    return entry(m, key)?.normalized ?? 0;
}
export function iLabel(m: IndicatorMap | undefined | null, key: string): string {
    return entry(m, key)?.state_label ?? 'UNKNOWN';
}
export function iSub(m: IndicatorMap | undefined | null, key: string, sub: string): number | null {
    return entry(m, key)?.values?.[sub] ?? null;
}

export function fmt(v: number | null, digits: number): string {
    return v == null ? '--' : v.toFixed(digits);
}

// ── Price-adaptive decimal formatting ──
// Resolve the target decimal count for a given reference price based on the
// unified 6-tier scale. Higher-value assets render coarser; sub-dollar assets
// retain micro-scale definition.
export function getDecimalCount(price: number | null | undefined): number {
    const p = Math.abs(price ?? 0);
    if (p >= 10000) return 1;
    if (p >= 1000) return 2;
    if (p >= 100) return 3;
    if (p >= 10) return 4;
    if (p >= 1) return 6;
    return 8;
}

// Format a price-scaled value using the decimal resolution of a reference
// price. Nullish/non-finite values collapse to a placeholder dash.
export function fmtPrice(value: number | null | undefined, refPrice: number | null | undefined): string {
    if (value == null || !isFinite(value)) return '--';
    return value.toFixed(getDecimalCount(refPrice));
}

// Build a Lightweight-Charts `priceFormat` object. `precision` alone is not
// enough — `minMove` must match (10^-precision) or sub-cent digits won't render.
export function getPriceFormat(refPrice: number | null | undefined): {
    type: 'price';
    precision: number;
    minMove: number;
} {
    const d = getDecimalCount(refPrice);
    return { type: 'price', precision: d, minMove: Math.pow(10, -d) };
}

// ── Derived categorical states (from backend state_label / normalized) ──
export type StackState = 'bullish' | 'bearish' | 'tangled';
export function emaStackState(m: IndicatorMap | undefined | null): StackState {
    const l = iLabel(m, 'ema_stack');
    if (l.includes('BULLISH')) return 'bullish';
    if (l.includes('BEARISH')) return 'bearish';
    return 'tangled';
}

export type VwapBias = 'premium' | 'discount' | 'equilibrium';
export function vwapBias(m: IndicatorMap | undefined | null): VwapBias {
    const l = iLabel(m, 'vwap');
    if (l.includes('PREMIUM')) return 'premium';
    if (l.includes('DISCOUNT')) return 'discount';
    return 'equilibrium';
}

export type AdxRegime = 'congestion' | 'emerging' | 'strong' | 'extreme';
export function adxRegime(m: IndicatorMap | undefined | null): AdxRegime {
    const l = iLabel(m, 'adx');
    if (l.includes('CONGESTION')) return 'congestion';
    if (l.includes('EMERGING')) return 'emerging';
    if (l.includes('CLIMACTIC')) return 'extreme';
    if (l.includes('STRONG')) return 'strong';
    return 'congestion';
}
export function adxSlope(m: IndicatorMap | undefined | null): number {
    return iSub(m, 'adx', 'adx_slope') ?? 0;
}
export function adxExhaustionReached(m: IndicatorMap | undefined | null): boolean {
    return (iSub(m, 'adx', 'adx') ?? iRaw(m, 'adx') ?? 0) > 40;
}

export function isSqueezeOn(m: IndicatorMap | undefined | null): boolean {
    return iLabel(m, 'squeeze') === 'COMPRESSION_COILING';
}

// ── Stochastic / ChandeMO derived states ──
export function isStochOverbought(m: IndicatorMap | undefined | null): boolean {
    return (iSub(m, 'stochastic', 'k_line') ?? 50) >= 80;
}
export function isStochOversold(m: IndicatorMap | undefined | null): boolean {
    return (iSub(m, 'stochastic', 'k_line') ?? 50) <= 20;
}
export function isChandeMoExtreme(m: IndicatorMap | undefined | null): boolean {
    return Math.abs(iRaw(m, 'chandemo') ?? 0) >= 50;
}

export function squeezeReleaseTrigger(m: IndicatorMap | undefined | null): boolean {
    return iLabel(m, 'squeeze').endsWith('VOLATILITY_RELEASE');
}
export type SqueezeDirection =
    | 'BullishAcceleration'
    | 'BullishDeceleration'
    | 'BearishAcceleration'
    | 'BearishDeceleration'
    | 'Flat';
export function squeezeDirection(m: IndicatorMap | undefined | null): SqueezeDirection {
    const e = entry(m, 'squeeze');
    if (!e) return 'Flat';
    if (e.state_label.includes('BULLISH')) return e.normalized >= 0.5 ? 'BullishAcceleration' : 'BullishDeceleration';
    if (e.state_label.includes('BEARISH')) return e.normalized <= -0.5 ? 'BearishAcceleration' : 'BearishDeceleration';
    return 'Flat';
}

export type CrossDir = 'BULLISH' | 'BEARISH' | 'NONE';
export function macdCrossoverDetected(m: IndicatorMap | undefined | null): boolean {
    return iLabel(m, 'macd').includes('CROSSOVER');
}
export function macdCrossoverDirection(m: IndicatorMap | undefined | null): CrossDir {
    const e = entry(m, 'macd');
    if (!e || !e.state_label.includes('CROSSOVER')) return 'NONE';
    return e.normalized >= 0 ? 'BULLISH' : 'BEARISH';
}
export function macdContractionTriggered(m: IndicatorMap | undefined | null): boolean {
    return iLabel(m, 'macd').includes('EXHAUSTION');
}

export type AtrRegime = 'expanding' | 'contracting' | 'stable';
export function atrVolatilityRegime(m: IndicatorMap | undefined | null): AtrRegime {
    const s = iSub(m, 'atr', 'atr_slope');
    if (s == null) return 'stable';
    return s > 0 ? 'expanding' : s < 0 ? 'contracting' : 'stable';
}

export type DivStatus = 'none' | 'potential' | 'confirmed';
export function divStatus(m: IndicatorMap | undefined | null, key: string): DivStatus {
    const l = iLabel(m, key);
    if (l.includes('CONFIRMED')) return 'confirmed';
    if (l.includes('POTENTIAL')) return 'potential';
    return 'none';
}
