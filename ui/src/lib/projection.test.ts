// Unit tests for the Project Risk and Return helpers — locks the fee-leg
// math, the margin-ROI formula, the empty (unconfigured) projection, and
// the configured projection build from a backend RiskCalculation.

import { describe, expect, it } from 'vitest';
import type { RiskCalculation } from '../types';
import { buildProjection, computeFeeLegs, computeRoiPct, emptyProjection } from './projection';

function calc(overrides: Partial<RiskCalculation> = {}): RiskCalculation {
    return {
        risk_capital: '100',
        price_distance: '1000',
        position_size_units: '0.05',
        position_notional: '3000',
        leverage_required: '30.00',
        leverage_selected: 10,
        margin_required: '300',
        liquidation_price: '95000',
        risk_reward_ratio: '2.50',
        estimated_profit: '600',
        total_fees: '3.60',
        net_pnl: '596.40',
        ...overrides,
    };
}

const SETUP = { direction: 'LONG' as const, entry: 100000, stopLoss: 90000, takeProfit: 110000 };

describe('computeFeeLegs', () => {
    it('estimates entry + exit legs at commission% of notional', () => {
        const legs = computeFeeLegs(3000, 0.06);
        expect(legs.entryFee).toBeCloseTo(1.8, 5);
        expect(legs.exitFee).toBeCloseTo(1.8, 5);
    });

    it('returns zero legs for degenerate notional', () => {
        expect(computeFeeLegs(0, 0.06)).toEqual({ entryFee: 0, exitFee: 0 });
        expect(computeFeeLegs(NaN, 0.06)).toEqual({ entryFee: 0, exitFee: 0 });
    });
});

describe('computeRoiPct', () => {
    it('computes net_pnl ÷ margin_required × 100', () => {
        expect(computeRoiPct(596.4, 300)).toBeCloseTo(198.8, 5);
    });

    it('returns null when margin is missing or zero', () => {
        expect(computeRoiPct(100, 0)).toBeNull();
        expect(computeRoiPct(100, NaN)).toBeNull();
    });
});

describe('emptyProjection', () => {
    it('is unconfigured with null numerics (export parity)', () => {
        const p = emptyProjection();
        expect(p.configured).toBe(false);
        expect(p.capital).toBeNull();
        expect(p.roi_pct).toBeNull();
        expect(p.liquidation_price).toBeNull();
    });
});

describe('buildProjection', () => {
    it('derives the full configured state from a RiskCalculation', () => {
        const p = buildProjection(SETUP, 100, 10, 0.06, calc());
        expect(p.configured).toBe(true);
        expect(p.capital).toBe(100);
        expect(p.leverage).toBe(10);
        expect(p.direction).toBe('LONG');
        expect(p.position_size_units).toBe(0.05);
        expect(p.position_notional_usd).toBe(3000);
        expect(p.entry_fee_usd).toBeCloseTo(1.8, 5);
        expect(p.exit_fee_usd).toBeCloseTo(1.8, 5);
        expect(p.total_fees_usd).toBe(3.6);
        expect(p.liquidation_price).toBe(95000);
        expect(p.net_profit_usd).toBeCloseTo(596.4, 5);
        expect(p.roi_pct).toBeCloseTo(198.8, 5);
    });

    it('keeps nulls for absent numeric fields', () => {
        const p = buildProjection(SETUP, 100, 10, 0.06, calc({ net_pnl: '', position_notional: '' }));
        expect(p.net_profit_usd).toBeNull();
        expect(p.roi_pct).toBeNull();
    });
});
