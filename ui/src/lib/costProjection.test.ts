// costProjection — fee + funding drag projection math (v7.4).
import { describe, expect, it } from 'vitest';
import { costProjection } from './costProjection';

describe('costProjection', () => {
    it('computes notional, round-trip fees and min profit %', () => {
        const r = costProjection({
            capital: 1000,
            leverage: 10,
            takerFeePct: 0.06,
            fundingRatePct: 0,
            holdPeriods: 1,
        });
        expect(r.notional).toBe(10000);
        expect(r.roundTripFees).toBeCloseTo(12.0); // 0.06% × 10k × 2
        expect(r.fundingDrag).toBe(0);
        expect(r.totalCost).toBeCloseTo(12.0);
        expect(r.minProfitPct).toBeCloseTo(1.2);
    });

    it('adds funding drag per 8h hold period', () => {
        const r = costProjection({
            capital: 1000,
            leverage: 10,
            takerFeePct: 0.06,
            fundingRatePct: 0.01,
            holdPeriods: 3,
        });
        expect(r.fundingDrag).toBeCloseTo(3.0); // 0.01% × 10k × 3
        expect(r.totalCost).toBeCloseTo(15.0);
        expect(r.minProfitPct).toBeCloseTo(1.5);
    });

    it('zero capital yields zero min profit without division blow-up', () => {
        const r = costProjection({ capital: 0, leverage: 10, takerFeePct: 0.06, fundingRatePct: 0.01, holdPeriods: 1 });
        expect(r.notional).toBe(0);
        expect(r.minProfitPct).toBe(0);
    });

    it('longer holds scale funding drag linearly', () => {
        const one = costProjection({ capital: 2000, leverage: 5, takerFeePct: 0.05, fundingRatePct: 0.01, holdPeriods: 1 });
        const ten = costProjection({ capital: 2000, leverage: 5, takerFeePct: 0.05, fundingRatePct: 0.01, holdPeriods: 10 });
        expect(ten.fundingDrag).toBeCloseTo(one.fundingDrag * 10);
        expect(ten.roundTripFees).toBe(one.roundTripFees); // fees do not scale with hold
    });
});
