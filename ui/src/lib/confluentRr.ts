// Confluent-level Reward-to-Risk — the Opportunities panel's
// "Expected Reward-to-Risk Ratio" section.
//
// The operator rule: average the entry levels and the target levels of
// the confluent level sets and build an R:R from those averages. The
// risk denominator is the per-side confluent invalidation average when
// the backend emitted one for that side, falling back to the distance
// from the averaged entry to the market (mark price) when invalidation
// levels are absent — a bracket R:R can still be read from geometry
// alone, and the fallback is flagged on the result so the UI can label
// it.
//
// Every case is handled consistently:
//   - no confluent levels at all            → `reason: 'no confluent levels'`
//   - entries without targets (or reverse)  → `reason: 'incomplete confluent levels'`
//   - one side only                         → a single `ConfluentRrSide`
//   - both sides                            → two `ConfluentRrSide` rows, each
//                                              labelled with its own side badge
//   - levels pinned exactly on the close    → excluded (they carry `side: null`
//                                              — no directional meaning)
//   - degenerate reward/risk (≤ 0, non-finite) → per-side `rr: null`,
//                                              reason 'degenerate geometry'
//   - below the shared 0.10 meaningfulness floor → per-side `rr: null`,
//                                              reason 'below the 0.10
//                                              meaningfulness floor'
//
// The floor is shared with the rest of the panel (`RR_MEANINGFUL_FLOOR`)
// so the section can never disagree with the header or the setup cards.
//
// Display vocabulary (v6.15): the section shows the bare R-multiple
// (`3.32R`) — no `1:` prefix — on a 0→10x magnitude bar where the fill
// is `rr / 10 × 100`% (100% = 10x = 1000% return). Ratios at or above
// 10x render as `10x+` with the fill clamped at 100%.

import type { ConfluentLevel, OpportunityMatrix } from '../types';
import { RR_MEANINGFUL_FLOOR } from './decisionRank';

export type ConfluentSide = 'LONG' | 'SHORT';

export interface ConfluentRrSide {
    side: ConfluentSide;
    /** Mean of the side's confluent entry level prices. */
    entryAvg: number;
    /** Mean of the side's confluent target level prices. */
    targetAvg: number;
    /** Mean of the side's confluent invalidation level prices, `null` when
     *  the backend emitted none for this side (risk falls back to market
     *  distance). */
    invalidationAvg: number | null;
    /** How the risk denominator was derived. */
    riskBasis: 'invalidation' | 'market_distance';
    /** The R:R (reward/risk) rounded to 2 decimals, `null` when degenerate. */
    rr: number | null;
    /** Human-readable N/A reason when `rr` is null. */
    reason: string | null;
}

export interface ConfluentRrResult {
    /** One row per side with a complete entry+target level set. */
    sides: ConfluentRrSide[];
    /** Global N/A reason when NO side produced a row. */
    reason: string | null;
}

function avg(prices: number[]): number {
    if (prices.length === 0) return 0;
    return prices.reduce((a, b) => a + b, 0) / prices.length;
}

function sidePrices(levels: ConfluentLevel[], side: ConfluentSide): number[] {
    return levels
        .filter((l) => l.side === side && l.price > 0 && isFinite(l.price))
        .map((l) => l.price);
}

export function computeConfluentRr(
    opportunity: OpportunityMatrix | null | undefined,
    markPrice: number,
): ConfluentRrResult {
    const entries = opportunity?.confluent_entry_levels ?? [];
    const targets = opportunity?.confluent_target_levels ?? [];
    const invalidations = opportunity?.confluent_invalidation_levels ?? [];
    if (entries.length === 0 && targets.length === 0) {
        return { sides: [], reason: 'no confluent levels' };
    }
    if (entries.length === 0 || targets.length === 0) {
        return { sides: [], reason: 'incomplete confluent levels' };
    }

    const out: ConfluentRrSide[] = [];
    for (const side of ['LONG', 'SHORT'] as const) {
        const entryPrices = sidePrices(entries, side);
        const targetPrices = sidePrices(targets, side);
        if (entryPrices.length === 0 || targetPrices.length === 0) continue;

        const entryAvg = avg(entryPrices);
        const targetAvg = avg(targetPrices);
        const invalPrices = sidePrices(invalidations, side);
        const invalidationAvg = invalPrices.length > 0 ? avg(invalPrices) : null;
        const riskBasis: ConfluentRrSide['riskBasis'] =
            invalidationAvg != null ? 'invalidation' : 'market_distance';

        // Direction-aware reward (mirrors `geometricRrFromZones`): a LONG
        // target must sit above its entry, a SHORT target below — a
        // LONG-tagged target under its own entry average is degenerate.
        const reward =
            side === 'LONG'
                ? targetAvg - entryAvg
                : entryAvg - targetAvg;
        const risk =
            invalidationAvg != null
                ? Math.abs(entryAvg - invalidationAvg)
                : Math.abs(entryAvg - markPrice);

        if (!isFinite(reward) || !isFinite(risk) || reward <= 0 || risk <= 0) {
            out.push({
                side,
                entryAvg,
                targetAvg,
                invalidationAvg,
                riskBasis,
                rr: null,
                reason: 'degenerate geometry',
            });
            continue;
        }
        const ratio = reward / risk;
        if (ratio < RR_MEANINGFUL_FLOOR) {
            out.push({
                side,
                entryAvg,
                targetAvg,
                invalidationAvg,
                riskBasis,
                rr: null,
                reason: 'below the 0.10 meaningfulness floor',
            });
            continue;
        }
        out.push({
            side,
            entryAvg,
            targetAvg,
            invalidationAvg,
            riskBasis,
            rr: Math.round(ratio * 100) / 100,
            reason: null,
        });
    }

    if (out.length === 0) {
        return { sides: [], reason: 'incomplete confluent levels' };
    }
    return { sides: out, reason: null };
}

/** Bare R-multiple format (`3.32`) — the `1:` prefix is gone (v6.15). */
export function fmtConfluentRr(rr: number): string {
    return rr.toFixed(2);
}

/** Trader-vernacular magnitude: `3.32R`, capped at `10x+` for rr ≥ 10. */
export function fmtConfluentRrMagnitude(rr: number): string {
    return rr >= 10 ? '10x+' : `${rr.toFixed(2)}R`;
}

/**
 * Fill percentage for the 0→10x magnitude bar: 0% = 0R, 100% = 10x
 * (1000% return). Anything at or above 10x clamps to 100%.
 */
export function rrBarPct(rr: number): number {
    if (!isFinite(rr) || rr <= 0) return 0;
    return Math.min(100, Math.max(0, (rr / 10) * 100));
}

/** Human-readable risk-basis label for the per-side card sub-line. */
export function riskBasisLabel(basis: ConfluentRrSide['riskBasis']): string {
    return basis === 'invalidation'
        ? 'risk = confluent invalidation average'
        : 'risk = distance to market — no confluent invalidation levels';
}
