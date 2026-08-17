import { describe, expect, it } from 'vitest';
import { confluenceStrengthLabel } from './confluenceStrength';

describe('confluenceStrengthLabel — qualitative bands for confluent level strength', () => {
    it('maps ≥ 80 to VERY STRONG', () => {
        expect(confluenceStrengthLabel(80)).toBe('VERY STRONG');
        expect(confluenceStrengthLabel(100)).toBe('VERY STRONG');
    });

    it('maps 55–79 to STRONG', () => {
        expect(confluenceStrengthLabel(55)).toBe('STRONG');
        expect(confluenceStrengthLabel(64)).toBe('STRONG');
        expect(confluenceStrengthLabel(79)).toBe('STRONG');
    });

    it('maps 30–54 to MODERATE', () => {
        expect(confluenceStrengthLabel(30)).toBe('MODERATE');
        expect(confluenceStrengthLabel(45)).toBe('MODERATE');
        expect(confluenceStrengthLabel(54)).toBe('MODERATE');
    });

    it('maps < 30 to WEAK', () => {
        expect(confluenceStrengthLabel(0)).toBe('WEAK');
        expect(confluenceStrengthLabel(15)).toBe('WEAK');
        expect(confluenceStrengthLabel(29)).toBe('WEAK');
    });
});
