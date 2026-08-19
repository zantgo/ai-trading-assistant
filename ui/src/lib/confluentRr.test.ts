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
    zones?: {
        long?: { entry: [number, number]; target: [number, number]; invalidation: number };
        short?: { entry: [number, number]; target: [number, number]; invalidation: number };
    };
}): OpportunityMatrix {
    return {
        confluent_entry_levels: levels.entry ?? [],
        confluent_target_levels: levels.target ?? [],
        confluent_invalidation_levels: levels.invalidation ?? [],
        ...(levels.zones?.long
            ? {
                long_entry_zone: { low: levels.zones.long.entry[0], high: levels.zones.long.entry[1] },
                long_target_zone: { low: levels.zones.long.target[0], high: levels.zones.long.target[1] },
                long_invalidation_level: levels.zones.long.invalidation,
            }
            : {}),
        ...(levels.zones?.short
            ? {
                short_entry_zone: { low: levels.zones.short.entry[0], high: levels.zones.short.entry[1] },
                short_target_zone: { low: levels.zones.short.target[0], high: levels.zones.short.target[1] },
                short_invalidation_level: levels.zones.short.invalidation,
            }
            : {}),
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

    it('v7.3: falls back to bracket geometry for a side whose confluent set lacks targets', () => {
        // The user-observed NoClear shape: LONG entries + invalidation
        // levels exist but the LONG target set is empty (its target zone
        // was ATR-derived), while SHORT is confluent-complete.
        const res = computeConfluentRr(
            oppWith({
                entry: [lvl(63100, 'LONG'), lvl(64900, 'SHORT')],
                target: [lvl(61900, 'SHORT')],
                invalidation: [lvl(62400, 'LONG'), lvl(65600, 'SHORT')],
                zones: {
                    long: {
                        entry: [63000, 63400],
                        target: [66000, 67000],
                        invalidation: 62600,
                    },
                },
            }),
            64000,
        );
        expect(res.reason).toBeNull();
        expect(res.sides).toHaveLength(2);
        const long = res.sides.find((s) => s.side === 'LONG')!;
        // Entry/target averages come from the zone MIDPOINTS, risk from
        // the zone invalidation — flagged so the operator can tell the
        // row was not confluent-averaged.
        expect(long.entryAvg).toBe(63200);
        expect(long.targetAvg).toBe(66500);
        expect(long.invalidationAvg).toBe(62600);
        expect(long.riskBasis).toBe('bracket_geometry');
        // reward = 3300, risk = 600 → 5.5
        expect(long.rr).toBe(5.5);
        expect(long.reason).toBeNull();
        // The SHORT side stays confluent-averaged (unchanged behavior).
        const short = res.sides.find((s) => s.side === 'SHORT')!;
        expect(short.riskBasis).toBe('invalidation');
        expect(short.rr).toBe(4.29);
    });

    it('v7.3: falls back to bracket geometry when a side has NO confluent levels at all', () => {
        const res = computeConfluentRr(
            oppWith({
                entry: [lvl(64900, 'SHORT')],
                target: [lvl(61900, 'SHORT')],
                invalidation: [lvl(65600, 'SHORT')],
                zones: {
                    long: {
                        entry: [63000, 63400],
                        target: [66000, 67000],
                        invalidation: 62600,
                    },
                },
            }),
            64000,
        );
        expect(res.sides).toHaveLength(2);
        const long = res.sides.find((s) => s.side === 'LONG')!;
        expect(long.riskBasis).toBe('bracket_geometry');
        expect(long.rr).toBe(5.5);
    });

    it('v7.3: does NOT fabricate rows when every level is pinned on close (side null)', () => {
        // The activity gate: untagged levels carry no directional meaning,
        // so valid bracket zones must NOT synthesize rows on their own.
        const res = computeConfluentRr(
            oppWith({
                entry: [lvl(64000, null)],
                target: [lvl(64000, null)],
                zones: {
                    long: {
                        entry: [63000, 63400],
                        target: [66000, 67000],
                        invalidation: 62600,
                    },
                },
            }),
            64000,
        );
        expect(res.sides).toEqual([]);
        expect(res.reason).toBe('incomplete confluent levels');
    });

    it('v7.3: an incomplete side with invalid zones is skipped, keeping the complete side', () => {
        const res = computeConfluentRr(
            oppWith({
                entry: [lvl(63100, 'LONG'), lvl(64900, 'SHORT')],
                target: [lvl(61900, 'SHORT')],
                invalidation: [lvl(65600, 'SHORT')],
                zones: {
                    // LONG is incomplete (no target level) but its zones
                    // are the zeroed sentinel — the fallback must skip it.
                    long: {
                        entry: [0, 0],
                        target: [0, 0],
                        invalidation: 0,
                    },
                },
            }),
            64000,
        );
        expect(res.sides).toHaveLength(1);
        expect(res.sides[0].side).toBe('SHORT');
        expect(res.sides[0].riskBasis).toBe('invalidation');
        expect(res.sides[0].rr).toBe(4.29);
    });
});

describe('formatting helpers', () => {
    it('formats the R:R as a bare R-multiple — no 1: prefix', () => {
        expect(fmtConfluentRr(0.95)).toBe('0.95');
        expect(fmtConfluentRr(3.333)).toBe('3.33');
    });

    it('renders trader-vernacular magnitude with an R suffix and a 10R+ cap (v7.0 unified scale)', () => {
        expect(fmtConfluentRrMagnitude(3.32)).toBe('3.32R');
        expect(fmtConfluentRrMagnitude(0.95)).toBe('0.95R');
        expect(fmtConfluentRrMagnitude(10)).toBe('10R+');
        expect(fmtConfluentRrMagnitude(12.5)).toBe('10R+');
    });

    it('maps R:R to the 0→10x bar fill (0% = 0R, 100% = 10x) with clamping', () => {
        expect(rrBarPct(0)).toBe(0);
        expect(rrBarPct(3.32)).toBeCloseTo(33.2);
        expect(rrBarPct(4.29)).toBeCloseTo(42.9);
        expect(rrBarPct(10)).toBe(100);
        expect(rrBarPct(25)).toBe(100);
        expect(rrBarPct(NaN)).toBe(0);
    });

    it('labels all three risk bases', () => {
        expect(riskBasisLabel('invalidation')).toContain('invalidation');
        expect(riskBasisLabel('market_distance')).toContain('market');
        expect(riskBasisLabel('bracket_geometry')).toContain('bracket');
    });
});
