// Unit tests for the confluent-level R:R helper — every possibility is
// locked: no levels, incomplete levels, one side, both sides, on-close
// (null-side) exclusions, market-distance fallback, degenerate geometry,
// and the 0.10 meaningfulness floor.

import { describe, expect, it } from 'vitest';
import type { ConfluentLevel, OpportunityMatrix } from '../types';
import { computeConfluentRr, fmtConfluentRr, fmtConfluentRrMagnitude, rrBarPct, riskBasisLabel } from './confluentRr';

function oppWith(levels: {
    entry?: ConfluentLevel[];
    target?: ConfluentLevel[];
    invalidation?: ConfluentLevel[];
}): OpportunityMatrix {
    return {
        confluent_entry_levels: levels.entry ?? [],
        confluent_target_levels: levels.target ?? [],
        confluent_invalidation_levels: levels.invalidation ?? [],
    } as unknown as OpportunityMatrix;
}

const lvl = (price: number, side: 'LONG' | 'SHORT' | null, strength = 50): ConfluentLevel => ({
    price,
    confluence_count: 1,
    sources: ['FIBONACCI'],
    strength,
    side,
});

describe('computeConfluentRr', () => {
    it('reports no confluent levels when entries and targets are empty', () => {
        const res = computeConfluentRr(oppWith({}), 64000);
        expect(res.sides).toEqual([]);
        expect(res.reason).toBe('no confluent levels');
    });

    it('reports no confluent levels for a null opportunity', () => {
        const res = computeConfluentRr(null, 64000);
        expect(res.sides).toEqual([]);
        expect(res.reason).toBe('no confluent levels');
    });

    it('reports incomplete levels when entries exist but targets do not', () => {
        const res = computeConfluentRr(
            oppWith({ entry: [lvl(63100, 'LONG')] }),
            64000,
        );
        expect(res.sides).toEqual([]);
        expect(res.reason).toBe('incomplete confluent levels');
    });

    it('builds a LONG R:R from averaged entries and targets using the invalidation average', () => {
        const res = computeConfluentRr(
            oppWith({
                entry: [lvl(63000, 'LONG'), lvl(63400, 'LONG')], // avg 63200
                target: [lvl(66000, 'LONG'), lvl(66500, 'LONG')], // avg 66250
                invalidation: [lvl(62400, 'LONG'), lvl(62800, 'LONG')], // avg 62600
            }),
            64000,
        );
        expect(res.reason).toBeNull();
        expect(res.sides).toHaveLength(1);
        const long = res.sides[0];
        expect(long.side).toBe('LONG');
        expect(long.entryAvg).toBe(63200);
        expect(long.targetAvg).toBe(66250);
        expect(long.invalidationAvg).toBe(62600);
        expect(long.riskBasis).toBe('invalidation');
        // reward = 3050, risk = 600 → 5.08
        expect(long.rr).toBe(5.08);
        expect(long.reason).toBeNull();
    });

    it('falls back to market distance when the side has no invalidation levels', () => {
        const res = computeConfluentRr(
            oppWith({
                entry: [lvl(63100, 'LONG')],
                target: [lvl(66100, 'LONG')],
            }),
            64000,
        );
        expect(res.reason).toBeNull();
        const long = res.sides[0];
        expect(long.invalidationAvg).toBeNull();
        expect(long.riskBasis).toBe('market_distance');
        // reward = 3000, risk = |63100 − 64000| = 900 → 3.33
        expect(long.rr).toBe(3.33);
    });

    it('renders BOTH sides with their own badges when both are present', () => {
        const res = computeConfluentRr(
            oppWith({
                entry: [lvl(63100, 'LONG'), lvl(64900, 'SHORT')],
                target: [lvl(66100, 'LONG'), lvl(61900, 'SHORT')],
                invalidation: [lvl(62400, 'LONG'), lvl(65600, 'SHORT')],
            }),
            64000,
        );
        expect(res.sides).toHaveLength(2);
        expect(res.sides.map((s) => s.side)).toEqual(['LONG', 'SHORT']);
        // LONG: reward 3000, risk 700 → 4.29
        expect(res.sides[0].rr).toBe(4.29);
        // SHORT: reward |61900 − 64900| = 3000, risk |64900 − 65600| = 700 → 4.29
        expect(res.sides[1].rr).toBe(4.29);
    });

    it('excludes on-close levels (side null) from every average', () => {
        const res = computeConfluentRr(
            oppWith({
                entry: [lvl(64000, null), lvl(63100, 'LONG')],
                target: [lvl(64000, null), lvl(66100, 'LONG')],
                invalidation: [lvl(64000, null), lvl(62400, 'LONG')],
            }),
            64000,
        );
        expect(res.sides).toHaveLength(1);
        expect(res.sides[0].entryAvg).toBe(63100);
        expect(res.sides[0].targetAvg).toBe(66100);
        expect(res.sides[0].invalidationAvg).toBe(62400);
        expect(res.sides[0].rr).not.toBeNull();
    });

    it('flags degenerate geometry when the target sits on the wrong side of the entry', () => {
        const res = computeConfluentRr(
            oppWith({
                entry: [lvl(66000, 'LONG')],
                target: [lvl(65000, 'LONG')],
                invalidation: [lvl(65500, 'LONG')],
            }),
            64000,
        );
        expect(res.sides).toHaveLength(1);
        expect(res.sides[0].rr).toBeNull();
        expect(res.sides[0].reason).toBe('degenerate geometry');
    });

    it('flags a ratio below the 0.10 meaningfulness floor', () => {
        const res = computeConfluentRr(
            oppWith({
                entry: [lvl(63100, 'LONG')],
                target: [lvl(63105, 'LONG')],
                invalidation: [lvl(62400, 'LONG')],
            }),
            64000,
        );
        expect(res.sides).toHaveLength(1);
        expect(res.sides[0].rr).toBeNull();
        expect(res.sides[0].reason).toBe('below the 0.10 meaningfulness floor');
    });

    it('skips a side missing targets and keeps the complete side', () => {
        const res = computeConfluentRr(
            oppWith({
                entry: [lvl(63100, 'LONG'), lvl(64900, 'SHORT')],
                target: [lvl(66100, 'LONG')],
                invalidation: [lvl(62400, 'LONG')],
            }),
            64000,
        );
        expect(res.sides).toHaveLength(1);
        expect(res.sides[0].side).toBe('LONG');
    });
});

describe('formatting helpers', () => {
    it('formats the R:R as a bare R-multiple — no 1: prefix', () => {
        expect(fmtConfluentRr(0.95)).toBe('0.95');
        expect(fmtConfluentRr(3.333)).toBe('3.33');
    });

    it('renders trader-vernacular magnitude with an R suffix and a 10x+ cap', () => {
        expect(fmtConfluentRrMagnitude(3.32)).toBe('3.32R');
        expect(fmtConfluentRrMagnitude(0.95)).toBe('0.95R');
        expect(fmtConfluentRrMagnitude(10)).toBe('10x+');
        expect(fmtConfluentRrMagnitude(12.5)).toBe('10x+');
    });

    it('maps R:R to the 0→10x bar fill (0% = 0R, 100% = 10x) with clamping', () => {
        expect(rrBarPct(0)).toBe(0);
        expect(rrBarPct(3.32)).toBeCloseTo(33.2);
        expect(rrBarPct(4.29)).toBeCloseTo(42.9);
        expect(rrBarPct(10)).toBe(100);
        expect(rrBarPct(25)).toBe(100);
        expect(rrBarPct(NaN)).toBe(0);
    });

    it('labels both risk bases', () => {
        expect(riskBasisLabel('invalidation')).toContain('invalidation');
        expect(riskBasisLabel('market_distance')).toContain('market');
    });
});
