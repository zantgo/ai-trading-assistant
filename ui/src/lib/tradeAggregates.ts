// Trade-opportunity aggregation shared between the Market Overview
// dashboard's hero strip, KPI cards, and Asset Rankings table.
//
// `OpportunityProfile.trade_viability` is the canonical wire-side flag
// the L4 producer sets to `Actionable` / `DirectionalNeutral` /
// `GeometryInverted` / `NoClear`. The overview's hero rule:
//
//   TRADE        — at least one profile with `viability === 'Actionable'`
//                   AND its instance's `DecisionContext.trade_readiness`
//                   is `READY`.
//   WAIT         — at least one NON-`NoClear` profile exists, but none
//                   meet the TRADE gate (regime still forming, R:R < 1,
//                   entry danger elevated, etc.).
//   STAND ASIDE  — no qualifying profile across all instances OR every
//                   instance's `trade_readiness === 'STAND_ASIDE'`.

import type {
    AdvisoryMatrix,
    DecisionContext,
    InstanceState,
    MarketBias,
    OpportunityMatrix,
    OpportunityProfile,
    TradeViability,
} from '../types';
import { normalizeViability } from './viability';
import { selectProfileSide } from './decisionRank';

export type HeroState = 'TRADE' | 'WAIT' | 'STAND_ASIDE';

export interface SetupSummary {
    symbol: string;
    profile: OpportunityProfile;
    viability: TradeViability;
    direction: 'LONG' | 'SHORT' | 'NEUTRAL';
    opportunityScore: number;
    rr: number;
    confidence: number;
    readiness: string;
}

/**
 * Resolve the trade direction for a profile. Delegates to the shared
 * `selectProfileSide` in `decisionRank.ts` (zone-presence aware —
 * CounterTrend profiles surface the deviation-driven side the L4
 * producer populated), so the dashboard and the Opportunity /
 * Recommendation panels can never disagree. Exported as a free helper
 * so dashboard components do not need to import the `decisionRank`
 * module (which is tightly coupled to the Decision Panel's internals).
 */
export function profileDirection(
    profile: OpportunityProfile,
    macroBias: MarketBias | null,
): 'LONG' | 'SHORT' | 'NEUTRAL' {
    return selectProfileSide(profile, macroBias);
}

/**
 * Read the per-profile R:R for the resolved direction. Falls back to
 * the aggregated `expected_rr_internal` when the per-side value is
 * null.
 */
export function profileRR(
    profile: OpportunityProfile,
    direction: 'LONG' | 'SHORT' | 'NEUTRAL',
    aggregated: number | null,
): number {
    if (direction === 'LONG') {
        const v = profile.long_expected_rr_internal;
        if (v && v > 0) return v;
    } else if (direction === 'SHORT') {
        const v = profile.short_expected_rr_internal;
        if (v && v > 0) return v;
    }
    return aggregated ?? 0;
}

/**
 * Walks every instance and collects the qualifying profile summary.
 * A profile qualifies when `preconditions_met > 0` and
 * `opportunity_type !== 'NoClearOpportunity'`. Each instance's
 * DecisionContext is read in parallel so the reader can emit the
 * `TRADE` / `WAIT` hero state.
 */
export function collectActiveSetups(
    instances: InstanceState[],
): SetupSummary[] {
    const out: SetupSummary[] = [];
    for (const inst of instances) {
        const opp = inst.opportunity;
        if (!opp) continue;
        const macroBias = inst.analysis?.bias ?? null;
        // The frontend's OpportunityMatrix carries per-side R:R values
        // (`long_expected_rr_internal` / `short_expected_rr_internal`).
        // Pick the side that matches the resolved direction so the
        // KPI strip and the per-asset "R:R" column display the
        // operator-relevant figure.
        const aggregatedLong = opp.long_expected_rr_internal ?? 0;
        const aggregatedShort = opp.short_expected_rr_internal ?? 0;
        const profiles = opp.profiles ?? [];
        for (const p of profiles) {
            if (p.preconditions_met <= 0) continue;
            if (p.opportunity_type === 'NoClearOpportunity') continue;
            const direction = profileDirection(p, macroBias);
            const aggregated = direction === 'SHORT' ? aggregatedShort : aggregatedLong;
            const readiness = inst.decisionContext?.trade_readiness ?? 'STAND_ASIDE';
            out.push({
                symbol: inst.symbol,
                profile: p,
                viability: normalizeViability(p.trade_viability ?? 'NoClear') as TradeViability,
                direction,
                opportunityScore: p.score ?? opp.opportunity_score ?? 0,
                rr: profileRR(p, direction, aggregated > 0 ? aggregated : null),
                confidence: inst.advisory?.confidence_assessment ?? 0,
                readiness,
            });
        }
    }
    return out;
}

/**
 * Top-level hero state. See module docstring for the rule.
 */
export function computeHeroState(instances: InstanceState[]): HeroState {
    if (instances.length === 0) return 'STAND_ASIDE';
    const setups = collectActiveSetups(instances);
    if (setups.length === 0) return 'STAND_ASIDE';
    const actionable = setups.filter(
        (s) => s.viability === 'Actionable' && s.readiness === 'READY',
    );
    if (actionable.length > 0) return 'TRADE';
    return 'WAIT';
}

/**
 * Picker for the dashboard's "best opportunity" tile. Prefers
 * Actionable + READY, then highest `opportunityScore`, then highest RR.
 */
export function pickBestOpportunity(instances: InstanceState[]): SetupSummary | null {
    const setups = collectActiveSetups(instances);
    if (setups.length === 0) return null;
    const actionable = setups.filter(
        (s) => s.viability === 'Actionable' && s.readiness === 'READY',
    );
    const pool = actionable.length > 0 ? actionable : setups;
    return pool.slice().sort((a, b) => {
        if (b.opportunityScore !== a.opportunityScore) {
            return b.opportunityScore - a.opportunityScore;
        }
        return b.rr - a.rr;
    })[0] ?? null;
}

/**
 * Aggregate R:R across instances for the KPI strip. Returns the
 * mean of the per-side `long_expected_rr_internal` (or SHORT side if
 * the macro bias is bearish) from every instance with an
 * OpportunityMatrix. The KPI reflects the system's general R:R, not
 * just the actionable subset.
 */
export function aggregateRR(instances: InstanceState[]): { avg: number; best: number; count: number } {
    let sum = 0;
    let best = 0;
    let count = 0;
    for (const inst of instances) {
        const opp = inst.opportunity;
        if (!opp) continue;
        const bias = inst.analysis?.bias ?? null;
        const isBearish = bias === 'Bearish' || bias === 'StrongBearish';
        const r = isBearish
            ? (opp.short_expected_rr_internal ?? 0)
            : (opp.long_expected_rr_internal ?? 0);
        if (r > 0) {
            sum += r;
            count += 1;
            if (r > best) best = r;
        }
    }
    return { avg: count > 0 ? sum / count : 0, best, count };
}

/**
 * Aggregate confidence across instances (L6 `confidence_assessment`).
 */
export function aggregateConfidence(instances: InstanceState[]): { avg: number; best: number; count: number } {
    let sum = 0;
    let best = 0;
    let count = 0;
    for (const inst of instances) {
        const c = inst.advisory?.confidence_assessment ?? 0;
        if (c > 0) {
            sum += c;
            count += 1;
            if (c > best) best = c;
        }
    }
    return { avg: count > 0 ? sum / count : 0, best, count };
}

/**
 * Aggregate risk across instances (L5 `overall_risk.score`).
 */
export function aggregateRisk(instances: InstanceState[]): { avg: number; count: number } {
    let sum = 0;
    let count = 0;
    for (const inst of instances) {
        const r = inst.risk?.overall_risk?.score;
        if (typeof r === 'number' && isFinite(r)) {
            sum += r;
            count += 1;
        }
    }
    return { avg: count > 0 ? sum / count : 0, count };
}

/**
 * Direction counts across instances (L6 `directional_guidance`).
 */
export function aggregateDirections(instances: InstanceState[]): {
    long: number;
    short: number;
    neutral: number;
} {
    let long = 0;
    let short = 0;
    let neutral = 0;
    for (const inst of instances) {
        const g = inst.advisory?.directional_guidance ?? null;
        if (!g) {
            neutral += 1;
            continue;
        }
        const upper = g.toUpperCase();
        if (upper.includes('LONG')) long += 1;
        else if (upper.includes('SHORT')) short += 1;
        else neutral += 1;
    }
    return { long, short, neutral };
}

/**
 * Signal-quality buckets from L6 `confidence_assessment` across instances.
 */
export function aggregateSignalQuality(instances: InstanceState[]): {
    strong: number;
    moderate: number;
    weak: number;
} {
    let strong = 0;
    let moderate = 0;
    let weak = 0;
    for (const inst of instances) {
        const c = inst.advisory?.confidence_assessment ?? 0;
        if (c >= 70) strong += 1;
        else if (c >= 40) moderate += 1;
        else weak += 1;
    }
    return { strong, moderate, weak };
}

/**
 * Re-export the type signatures other modules may want.
 */
export type { AdvisoryMatrix, DecisionContext, OpportunityMatrix };
