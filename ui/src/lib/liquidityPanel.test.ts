import { describe, it, expect } from 'vitest';
import { cascadeAsymmetryLabel, cascadeAsymmetryIsBullish, cascadeAsymmetryIsBearish } from './liquidityPanel';

describe('cascadeAsymmetryLabel (canonical sign convention, v2026-08)', () => {
    it('maps positive beyond the ±0.3 dead-band to SHORT_SQUEEZE_RISK (bullish)', () => {
        expect(cascadeAsymmetryLabel(0.35)).toBe('SHORT_SQUEEZE_RISK');
        expect(cascadeAsymmetryIsBullish(0.35)).toBe(true);
        expect(cascadeAsymmetryIsBearish(0.35)).toBe(false);
    });

    it('maps negative beyond the ±0.3 dead-band to LONG_SQUEEZE_RISK (bearish)', () => {
        expect(cascadeAsymmetryLabel(-0.4)).toBe('LONG_SQUEEZE_RISK');
        expect(cascadeAsymmetryIsBearish(-0.4)).toBe(true);
        expect(cascadeAsymmetryIsBullish(-0.4)).toBe(false);
    });

    it('treats the ±0.3 dead-band as NEUTRAL (same scalar, one classification)', () => {
        expect(cascadeAsymmetryLabel(0.2)).toBe('NEUTRAL');
        expect(cascadeAsymmetryLabel(-0.2)).toBe('NEUTRAL');
        expect(cascadeAsymmetryLabel(0.3)).toBe('NEUTRAL');
        expect(cascadeAsymmetryLabel(-0.3)).toBe('NEUTRAL');
    });

    it('returns null for missing/non-finite input', () => {
        expect(cascadeAsymmetryLabel(null)).toBeNull();
        expect(cascadeAsymmetryLabel(undefined)).toBeNull();
        expect(cascadeAsymmetryLabel(NaN)).toBeNull();
        expect(cascadeAsymmetryLabel(Infinity)).toBeNull();
    });
});
