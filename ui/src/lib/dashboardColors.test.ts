import { describe, it, expect } from 'vitest';
import {
    biasColor,
    riskDangerColor,
    qualityColor,
    directionColor,
    directionLabel,
    signalLabel,
    signalQualityBucket,
    rrColor,
    scoreColor,
    formatRR,
    asciiBar,
} from './dashboardColors';

describe('biasColor', () => {
    it('StrongBullish -> good green', () => {
        expect(biasColor('StrongBullish')).toBe('#22c55e');
    });
    it('Bullish -> bull green', () => {
        expect(biasColor('Bullish')).toBe('#4ade80');
    });
    it('Neutral -> amber', () => {
        expect(biasColor('Neutral')).toBe('#f59e0b');
    });
    it('Bearish -> red', () => {
        expect(biasColor('Bearish')).toBe('#f87171');
    });
    it('StrongBearish -> dark red', () => {
        expect(biasColor('StrongBearish')).toBe('#dc2626');
    });
    it('null -> amber', () => {
        expect(biasColor(null)).toBe('#f59e0b');
    });
    it('SCREAMING_SNAKE wire shape derives correctly', () => {
        expect(biasColor('STRONG_BULLISH')).toBe('#22c55e');
        expect(biasColor('STRONG_BEARISH')).toBe('#dc2626');
        expect(biasColor('BULLISH')).toBe('#4ade80');
        expect(biasColor('BEARISH')).toBe('#f87171');
    });
});

describe('riskDangerColor', () => {
    it('80+ -> extreme red', () => {
        expect(riskDangerColor(85)).toBe('#f87171');
    });
    it('50-69 -> amber', () => {
        expect(riskDangerColor(60)).toBe('#f59e0b');
    });
    it('30-49 -> bull green', () => {
        expect(riskDangerColor(40)).toBe('#4ade80');
    });
    it('<30 -> solid green', () => {
        expect(riskDangerColor(20)).toBe('#22c55e');
    });
    it('null -> inactive', () => {
        expect(riskDangerColor(null)).toBe('rgba(255,255,255,0.25)');
    });
});

describe('qualityColor', () => {
    it('mirror of riskDangerColor on the 0-100 axis', () => {
        expect(qualityColor(85)).toBe('#22c55e');
        expect(qualityColor(60)).toBe('#4ade80');
        expect(qualityColor(40)).toBe('#f59e0b');
        expect(qualityColor(20)).toBe('#f87171');
    });
});

describe('directionColor', () => {
    it('LONG -> bull green', () => {
        expect(directionColor('LONG')).toBe('#4ade80');
    });
    it('SHORT -> red', () => {
        expect(directionColor('SHORT')).toBe('#f87171');
    });
    it('NEUTRAL -> amber', () => {
        expect(directionColor('NEUTRAL')).toBe('#f59e0b');
    });
    it('null -> amber', () => {
        expect(directionColor(null)).toBe('#f59e0b');
    });
});

describe('directionLabel', () => {
    it('long guidance variants collapse to LONG', () => {
        expect(directionLabel('StrongLong')).toBe('LONG');
        expect(directionLabel('Long')).toBe('LONG');
        expect(directionLabel('LONG')).toBe('LONG');
    });
    it('short guidance variants collapse to SHORT', () => {
        expect(directionLabel('StrongShort')).toBe('SHORT');
        expect(directionLabel('Short')).toBe('SHORT');
    });
    it('Neutral -> NEUTRAL', () => {
        expect(directionLabel('Neutral')).toBe('NEUTRAL');
    });
    it('null -> NEUTRAL', () => {
        expect(directionLabel(null)).toBe('NEUTRAL');
    });
});

describe('signalLabel', () => {
    it('BUY for LONG', () => {
        expect(signalLabel('Long')).toBe('BUY');
        expect(signalLabel('StrongLong')).toBe('BUY');
    });
    it('SELL for SHORT', () => {
        expect(signalLabel('Short')).toBe('SELL');
        expect(signalLabel('StrongShort')).toBe('SELL');
    });
    it('WAIT for Neutral', () => {
        expect(signalLabel('Neutral')).toBe('WAIT');
        expect(signalLabel(null)).toBe('WAIT');
    });
});

describe('signalQualityBucket', () => {
    it('STRONG >= 70', () => {
        expect(signalQualityBucket(70)).toBe('STRONG');
        expect(signalQualityBucket(95)).toBe('STRONG');
    });
    it('MODERATE 40-69', () => {
        expect(signalQualityBucket(40)).toBe('MODERATE');
        expect(signalQualityBucket(69)).toBe('MODERATE');
    });
    it('WEAK < 40', () => {
        expect(signalQualityBucket(39)).toBe('WEAK');
        expect(signalQualityBucket(0)).toBe('WEAK');
        expect(signalQualityBucket(null)).toBe('WEAK');
    });
});

describe('rrColor', () => {
    it('>= 2.0 -> good green', () => {
        expect(rrColor(2.5)).toBe('#22c55e');
    });
    it('1.0-1.99 -> amber', () => {
        expect(rrColor(1.5)).toBe('#f59e0b');
    });
    it('< 1.0 -> red', () => {
        expect(rrColor(0.5)).toBe('#f87171');
    });
    it('null -> muted', () => {
        expect(rrColor(null)).toBe('rgba(255,255,255,0.55)');
    });
    it('0 -> muted', () => {
        expect(rrColor(0)).toBe('rgba(255,255,255,0.55)');
    });
});

describe('scoreColor', () => {
    it('85+ -> good green', () => {
        expect(scoreColor(95)).toBe('#22c55e');
    });
    it('70-84 -> bull green', () => {
        expect(scoreColor(75)).toBe('#4ade80');
    });
    it('50-69 -> amber', () => {
        expect(scoreColor(60)).toBe('#f59e0b');
    });
    it('30-49 -> amber', () => {
        expect(scoreColor(40)).toBe('#fbbf24');
    });
    it('<30 -> red', () => {
        expect(scoreColor(20)).toBe('#f87171');
    });
});

describe('formatRR', () => {
    it('formats 1:2.50', () => {
        expect(formatRR(2.5)).toBe('1 : 2.50');
    });
    it('formats 1:1.00', () => {
        expect(formatRR(1.0)).toBe('1 : 1.00');
    });
    it('null -> —', () => {
        expect(formatRR(null)).toBe('—');
    });
    it('0 -> —', () => {
        expect(formatRR(0)).toBe('—');
    });
    it('negative -> —', () => {
        expect(formatRR(-1)).toBe('—');
    });
});

describe('asciiBar', () => {
    it('0% -> empty bar', () => {
        expect(asciiBar(0)).toBe('░░░░░░░░░░');
    });
    it('100% -> full bar', () => {
        expect(asciiBar(100)).toBe('██████████');
    });
    it('50% -> half bar', () => {
        expect(asciiBar(50)).toBe('█████░░░░░');
    });
    it('clamps > 100', () => {
        expect(asciiBar(150)).toBe('██████████');
    });
    it('clamps < 0', () => {
        expect(asciiBar(-10)).toBe('░░░░░░░░░░');
    });
    it('respects custom width', () => {
        expect(asciiBar(50, 4)).toBe('██░░');
    });
});
