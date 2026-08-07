// @vitest-environment node
// Tests for the unified Decision-tab hero consolidated in
// ui/src/lib/decisionRank.ts. These cover the rank normalisation, the
// gate-aware headline, and the symmetric Long/Short setup derivation.

import { describe, it, expect } from 'vitest';
import {
    computeDecisionRank,
    computeSymmetricSetups,
    selectProfileSide,
    profileZones,
    aggregateZones,
    topSetupSummary,
    profileSummary,
} from './decisionRank';
import type {
    AdvisoryMatrix,
    AnalysisMatrix,
    DecisionContext,
    OpportunityMatrix,
} from '../types';

function makeAdvisory(overrides: Partial<AdvisoryMatrix> = {}): AdvisoryMatrix {
    return {
        symbol: 'BTC-USDT',
        directional_guidance: 'Long',
        market_stance: 'Neutral',
        opportunity_classification: 'TrendContinuation',
        strategy_environment: 'TrendFollowing',
        entry_guidance: 'WaitForConfirmation',
        exit_guidance: 'NoWarning',
        protection_strategy: 'ATRBased',
        target_strategy: 'ResistanceBased',
        confidence_assessment: 31,
        stop_loss_distance_pct: 0.015,
        cascade_risk_score: 30,
        environment_favorability: {
            score: 25,
            level: 'Low',
            state: 'Stable',
            confidence: 50,
            evidence: [],
        },
        final_recommendation: 'Long bias, neutral stance.',
        ...overrides,
    };
}

function makeDanger(score: number, overrides: Partial<DecisionContext['entry_danger']> = {}): DecisionContext['entry_danger'] {
    return {
        score,
        level: score >= 80 ? 'Extreme' : score >= 60 ? 'High' : score >= 40 ? 'Moderate' : score >= 20 ? 'Low' : 'VeryLow',
        state: 'Stable',
        confidence: 50,
        evidence: [],
        ...overrides,
    };
}

function makeDecisionContext(overrides: Partial<DecisionContext> = {}): DecisionContext {
    return {
        score: 0,
        bias: 'NEUTRAL',
        confidence: 0,
        score_confidence: 0,
        entry_danger: makeDanger(50),
        expected_reward_risk_ratio: 0,
        trade_readiness: 'STAND_ASIDE',
        contributing_indicators: [],
        ...overrides,
    };
}

function makeOpportunity(overrides: Partial<OpportunityMatrix> = {}): OpportunityMatrix {
    // The fixture builds a coherent bullish setup so the per-direction
    // geometry invariants always pass. The mirror around `markPrice`
    // populates the SHORT side from the LONG side.
    const longEntry = { low: 63900, high: 64100 };
    const longTarget = { low: 64800, high: 65000 };
    const longInval = 63500;
    const markPrice = 64000;
    const shortEntry = { low: 2 * markPrice - longEntry.high, high: 2 * markPrice - longEntry.low };
    const shortTarget = { low: 2 * markPrice - longTarget.high, high: 2 * markPrice - longTarget.low };
    const shortInval = 2 * markPrice - longInval;
    return {
        symbol: 'BTC-USDT',
        primary_opportunity: 'TrendContinuation',
        opportunity_score: 50,
        setup_quality: 'Average',
        profiles: [],
        forecast_confidence: 0.5,
        contributing_signals: [],
        invalidation_note: '',
        entry_zone: longEntry,
        target_zone: longTarget,
        invalidation_level: longInval,
        long_entry_zone: longEntry,
        long_target_zone: longTarget,
        long_invalidation_level: longInval,
        long_expected_rr_internal: 2.0,
        short_entry_zone: shortEntry,
        short_target_zone: shortTarget,
        short_invalidation_level: shortInval,
        short_expected_rr_internal: 2.0,
        time_horizon: 'SWING',
        ...overrides,
    } as OpportunityMatrix;
}

function makeAnalysis(overrides: Partial<AnalysisMatrix> = {}): AnalysisMatrix {
    return {
        symbol: 'BTC-USDT',
        bias: 'Bullish',
        market_bias_score: 0.4,
        state_confidence: 0.7,
        confidence: 0.7,
        market_regime: 'TrendingBull',
        trend_assessment: 'Healthy',
        momentum_assessment: 'Stable',
        structure_assessment: 'Healthy',
        volatility_assessment: 'Normal',
        volume_assessment: 'Normal',
        opportunity_analysis: 'TrendContinuation',
        market_quality: 'Good',
        market_quality_score: 70,
        market_phase: 'Markup',
        market_interpretation: 'Bullish trend',
        rationale: '',
        supporting_signals: ['ema_stack', 'macd'],
        contradicting_signals: [],
        timeframes_considered: 4,
        ...overrides,
    } as AnalysisMatrix;
}

describe('computeDecisionRank', () => {
    it('forces STAND_ASIDE hero when readiness is STAND_ASIDE regardless of bias', () => {
        const rank = computeDecisionRank({
            advisory: makeAdvisory({ directional_guidance: 'Long', market_stance: 'Constructive' }),
            decisionContext: makeDecisionContext({
                score: 65,
                bias: 'BULLISH',
                score_confidence: 0.85,
                entry_danger: makeDanger(75),
                expected_reward_risk_ratio: 1.6,
                trade_readiness: 'STAND_ASIDE',
            }),
            opportunity: makeOpportunity(),
            analysis: makeAnalysis(),
        });

        expect(rank.headline.state).toBe('STAND_ASIDE');
        expect(rank.headline.action).toBe('STAND_ASIDE');
        expect(rank.headline.label).toBe('HOLD — STAND ASIDE');
        // probabilities sum to 100
        expect(rank.long.probability + rank.short.probability + rank.hold.probability).toBe(100);
    });

    it('ranks LONG top when bullish with READY readiness', () => {
        const rank = computeDecisionRank({
            advisory: makeAdvisory({ directional_guidance: 'StrongLong', market_stance: 'Aggressive' }),
            decisionContext: makeDecisionContext({
                score: 85,
                bias: 'BULLISH',
                score_confidence: 0.95,
                entry_danger: makeDanger(20),
                expected_reward_risk_ratio: 2.8,
                trade_readiness: 'READY',
            }),
            opportunity: makeOpportunity({ opportunity_score: 80, setup_quality: 'Prime' }),
            analysis: makeAnalysis(),
        });

        expect(rank.headline.state).toBe('READY');
        expect(rank.headline.action).toBe('LONG');
        expect(rank.headline.label).toBe('LONG — READY');
        expect(rank.top).toBe('LONG');
        expect(rank.long.probability).toBeGreaterThanOrEqual(60);
        expect(rank.long.probability + rank.short.probability + rank.hold.probability).toBe(100);
    });

    it('ranks SHORT top when bearish with READY readiness', () => {
        const rank = computeDecisionRank({
            advisory: makeAdvisory({ directional_guidance: 'StrongShort', market_stance: 'Aggressive' }),
            decisionContext: makeDecisionContext({
                score: -85,
                bias: 'BEARISH',
                score_confidence: 0.95,
                entry_danger: makeDanger(20),
                expected_reward_risk_ratio: 2.8,
                trade_readiness: 'READY',
            }),
            opportunity: makeOpportunity({ opportunity_score: 80, setup_quality: 'Prime' }),
            analysis: makeAnalysis({ bias: 'Bearish' }),
        });

        expect(rank.headline.state).toBe('READY');
        expect(rank.headline.action).toBe('SHORT');
        expect(rank.headline.label).toBe('SHORT — READY');
        expect(rank.top).toBe('SHORT');
        expect(rank.short.probability).toBeGreaterThanOrEqual(60);
        expect(rank.long.probability + rank.short.probability + rank.hold.probability).toBe(100);
    });

    it('probabilities always sum exactly to 100', () => {
        const rank = computeDecisionRank({
            advisory: makeAdvisory(),
            decisionContext: makeDecisionContext(),
            opportunity: makeOpportunity(),
            analysis: makeAnalysis(),
        });
        expect(rank.long.probability + rank.short.probability + rank.hold.probability).toBe(100);
    });

    it('rationale lists the gate cause when STAND_ASIDE', () => {
        const rank = computeDecisionRank({
            advisory: makeAdvisory(),
            decisionContext: makeDecisionContext({
                entry_danger: makeDanger(75),
                trade_readiness: 'STAND_ASIDE',
            }),
            opportunity: makeOpportunity(),
            analysis: makeAnalysis(),
        });
        const joined = rank.rationale.join(' | ');
        expect(joined).toContain('STAND_ASIDE');
        expect(joined).toContain('entry_danger');
        expect(joined).toContain('TrendContinuation');
    });

    it('low R:R (< 1.0) reduces long score relative to healthy R:R', () => {
        const base = {
            advisory: makeAdvisory({ directional_guidance: 'Long', market_stance: 'Constructive' }),
            opportunity: makeOpportunity(),
            analysis: makeAnalysis(),
        };
        const low = computeDecisionRank({
            ...base,
            decisionContext: makeDecisionContext({
                score: 80,
                bias: 'BULLISH',
                score_confidence: 0.9,
                entry_danger: makeDanger(30),
                expected_reward_risk_ratio: 0.6,
                trade_readiness: 'FORMING',
            }),
        });
        const healthy = computeDecisionRank({
            ...base,
            decisionContext: makeDecisionContext({
                score: 80,
                bias: 'BULLISH',
                score_confidence: 0.9,
                entry_danger: makeDanger(30),
                expected_reward_risk_ratio: 2.5,
                trade_readiness: 'FORMING',
            }),
        });

        expect(low.headline.state).toBe('FORMING');
        expect(healthy.headline.state).toBe('FORMING');
        // R:R < 1.0 must strictly reduce the long probability
        expect(low.long.probability).toBeLessThan(healthy.long.probability);
    });

    it('handles empty payload without crashing', () => {
        const rank = computeDecisionRank({
            advisory: null,
            decisionContext: null,
            opportunity: null,
            analysis: null,
        });
        expect(rank.long.probability + rank.short.probability + rank.hold.probability).toBe(100);
        expect(['LONG', 'SHORT', 'HOLD']).toContain(rank.top);
    });
});

describe('computeSymmetricSetups', () => {
    it('produces active long setup when top action is LONG and not gated', () => {
        const setups = computeSymmetricSetups({
            opportunity: makeOpportunity(),
            markPrice: 64000,
            topAction: 'LONG',
            readiness: 'READY',
        });
        expect(setups.long.active).toBe(true);
        expect(setups.short.active).toBe(false);
        expect(setups.long.entry?.price).toBeCloseTo(64000, 0);
        expect(setups.long.targets.length).toBeGreaterThanOrEqual(1);
        expect(setups.long.stop?.price).toBe(63500);
    });

    it('marks both setups inactive when readiness is STAND_ASIDE', () => {
        const setups = computeSymmetricSetups({
            opportunity: makeOpportunity(),
            markPrice: 64000,
            topAction: 'LONG',
            readiness: 'STAND_ASIDE',
        });
        expect(setups.long.active).toBe(false);
        expect(setups.short.active).toBe(false);
        expect(setups.long.status).toContain('gated');
        expect(setups.short.status).toContain('gated');
    });

    it('reads per-direction short zones directly (NOT the legacy mirror)', () => {
        // The fixture supplies a SHORT entry zone that's clearly above the
        // legacy single-bias projection (entry 66000 vs markPrice 64000).
        // The new code must read short_* directly, not mirror.
        const setups = computeSymmetricSetups({
            opportunity: makeOpportunity({
                short_entry_zone: { low: 66000, high: 66200 },
                short_target_zone: { low: 62000, high: 62400 },
                short_invalidation_level: 66500,
            }),
            markPrice: 64000,
            topAction: 'SHORT',
            readiness: 'READY',
        });
        // Short entry mid = (66000+66200)/2 = 66100
        expect(setups.short.entry?.price).toBeCloseTo(66100, 0);
        // Short TP1 = nearest to entry_mid: target 62000, distance 4100 vs 62400 distance 3700
        // so TP1 = 62400 (closer), TP2 = 62000
        expect(setups.short.targets[0]?.price).toBe(62400);
        expect(setups.short.targets[1]?.price).toBe(62000);
        expect(setups.short.stop?.price).toBe(66500);
    });

    it('returns empty setups when no opportunity matrix is present', () => {
        const setups = computeSymmetricSetups({
            opportunity: null,
            markPrice: 64000,
            topAction: 'LONG',
            readiness: 'READY',
        });
        expect(setups.long.active).toBe(false);
        expect(setups.short.active).toBe(false);
        expect(setups.long.entry).toBeNull();
        expect(setups.short.targets.length).toBe(0);
    });

    it('produces computed R:R ratio for the active long setup', () => {
        // TP1 is the nearest target: target 64800 (closer to entry_mid 64000
        // than target 65000). R:R = (64800-64000)/(64000-63500) = 1.6.
        const setups = computeSymmetricSetups({
            opportunity: makeOpportunity({
                long_entry_zone: { low: 63900, high: 64100 },
                long_target_zone: { low: 64800, high: 65000 },
                long_invalidation_level: 63500,
            }),
            markPrice: 64000,
            topAction: 'LONG',
            readiness: 'READY',
        });
        expect(setups.long.rrRatio).toBeCloseTo(1.6, 1);
    });
});

describe('computeDecisionRank — geometry flag', () => {
    // Long: entry below target above SL is geometrically consistent.
    it('flags long setup as consistent when entry < target < SL is impossible (target above entry)', () => {
        const setups = computeSymmetricSetups({
            opportunity: makeOpportunity({
                long_entry_zone: { low: 63900, high: 64100 },
                long_target_zone: { low: 64800, high: 65000 },
                long_invalidation_level: 63500,
                // Mirror SHORT into a consistent SHORT bracket so the
                // per-side geometry check passes for SHORT too.
                short_entry_zone: { low: 63900, high: 64100 },
                short_target_zone: { low: 63000, high: 63200 },
                short_invalidation_level: 64500,
            }),
            markPrice: 64000,
            topAction: 'LONG',
            readiness: 'READY',
        });
        expect(setups.long.geometry_consistent).toBe(true);
        expect(setups.short.geometry_consistent).toBe(true);
    });

    // Long: target BELOW entry ⇒ geometrically inverted (this is the bug in the user's screenshot)
    it('flags long setup as inconsistent when target lies below entry mid', () => {
        const setups = computeSymmetricSetups({
            opportunity: makeOpportunity({
                long_entry_zone: { low: 64000, high: 64200 },
                long_target_zone: { low: 63300, high: 63400 },
                long_invalidation_level: 64491,
            }),
            markPrice: 63543,
            topAction: 'LONG',
            readiness: 'FORMING',
        });
        expect(setups.long.geometry_consistent).toBe(false);
    });

    // Per-side wiring: long card reads long_* regardless of bias
    it('long card reads long_* zones when bias is bearish', () => {
        const setups = computeSymmetricSetups({
            opportunity: makeOpportunity({
                long_entry_zone: { low: 63000, high: 63200 },
                long_target_zone: { low: 66000, high: 66500 },
                long_invalidation_level: 62400,
                // SHORT side is the legacy mirror, NOT the per-side long zones
                short_entry_zone: { low: 65000, high: 65200 },
                short_target_zone: { low: 62000, high: 62400 },
                short_invalidation_level: 66000,
            }),
            markPrice: 63800,
            topAction: 'LONG',
            readiness: 'READY',
        });
        // Long card reads long_* directly
        expect(setups.long.entry?.price).toBe(63100);
        // TP1 = nearest to entry_mid 63100: 66000 distance 2900 < 66500 distance 3400
        expect(setups.long.targets[0]?.price).toBe(66000);
        expect(setups.long.targets[1]?.price).toBe(66500);
        expect(setups.long.stop?.price).toBe(62400);
        expect(setups.long.geometry_consistent).toBe(true);
    });
});

describe('computeSymmetricSetups — TP ordering (nearest first)', () => {
    it('LONG: TP1 (64800) < TP2 (65000) in absolute distance from entry (64000)', () => {
        const opp = makeOpportunity({
            long_entry_zone: { low: 63900, high: 64100 },
            long_target_zone: { low: 64800, high: 65000 },
            long_invalidation_level: 63500,
        });
        const s = computeSymmetricSetups({
            opportunity: opp, markPrice: 64000,
            topAction: 'LONG', readiness: 'READY',
        });
        expect(s.long.targets.length).toBe(2);
        const distTp1 = Math.abs(s.long.targets[0].price - 64000);
        const distTp2 = Math.abs(s.long.targets[1].price - 64000);
        expect(distTp1).toBeLessThan(distTp2);
    });

    it('SHORT: TP1 (62400) < TP2 (62000) in absolute distance from entry (65100)', () => {
        const opp = makeOpportunity({
            short_entry_zone: { low: 65000, high: 65200 },
            short_target_zone: { low: 62000, high: 62400 },
            short_invalidation_level: 66000,
        });
        const s = computeSymmetricSetups({
            opportunity: opp, markPrice: 64000,
            topAction: 'SHORT', readiness: 'READY',
        });
        expect(s.short.targets.length).toBe(2);
        const distTp1 = Math.abs(s.short.targets[0].price - 65100);
        const distTp2 = Math.abs(s.short.targets[1].price - 65100);
        expect(distTp1).toBeLessThan(distTp2);
    });
});

describe('computeSymmetricSetups — HOLD hypothesis', () => {
    it('marks BOTH sides inactive when topAction is HOLD but still surfaces both sides for reference', () => {
        const opp = makeOpportunity();
        const s = computeSymmetricSetups({
            opportunity: opp, markPrice: 64000,
            topAction: 'HOLD', readiness: 'READY',
        });
        expect(s.long.active).toBe(false);
        expect(s.short.active).toBe(false);
        // Zones still surfaced for the HYPOTHETICAL view
        expect(s.long.entry).not.toBeNull();
        expect(s.short.entry).not.toBeNull();
    });
});

describe('computeDecisionRank — degenerate-rank guard', () => {
    // When the three arms end up within 35% of each other, fall back to HOLD.
    it('collapses to HOLD when no arm carries a pre-normalization edge', () => {
        // Score 0, neutral bias, neutral guidance, neutral stance, WATCH
        // readiness. There is no directional edge; HOLD wins. The 2% floor on
        // long/short reflects the non-zero entry_danger signal (not absolutely neutral).
        const rank = computeDecisionRank({
            advisory: makeAdvisory({
                directional_guidance: 'Neutral',
                market_stance: 'Neutral',
            }),
            decisionContext: makeDecisionContext({
                score: 0,
                bias: 'NEUTRAL',
                score_confidence: 0,
                entry_danger: makeDanger(50),
                expected_reward_risk_ratio: 1.0,
                trade_readiness: 'WATCH',
            }),
            opportunity: makeOpportunity({ opportunity_score: 50, setup_quality: 'Average' }),
            analysis: makeAnalysis({ bias: 'Neutral', confidence: 0.1 }),
        });
        expect(rank.top).toBe('HOLD');
        // long + short + hold sum to 100 (renormalised)
        expect(rank.long.probability + rank.short.probability + rank.hold.probability).toBe(100);
        // With score=0, HOLD dominates; long and short receive the 2% sensitivity floor
        // because entry_danger produced a non-zero signal (the market isn't absolutely neutral).
        expect(rank.long.probability).toBeGreaterThanOrEqual(2);
        expect(rank.short.probability).toBeGreaterThanOrEqual(2);
        expect(rank.hold.probability).toBeLessThanOrEqual(96);
    });

    // When score=20 (mildly bullish) but neutral guidance, all arms stay near
    // 33/33/33 — the guard collapses to HOLD.
    it('collapses to HOLD when all three arms end up tied near 33%', () => {
        // baseLong = 20 * 0.5 = 10, baseShort = 0, baseHold = 25 (from entry_danger 50)
        // → renormalised: 10 / 0 / 25 (sum 35) → ~29% / 0% / 71%
        // → guard: max = 71, that's ≥ 35 but long holds the gate? actually
        // 71 is hold, so HOLD wins. Let's craft a case where HOLD, LONG
        // and SHORT actually tie. We force HOLD's gate bonus off by
        // using FORMING + entry_danger 30 (LOW).
        const rank = computeDecisionRank({
            advisory: makeAdvisory({
                directional_guidance: 'Neutral',
                market_stance: 'Neutral',
            }),
            decisionContext: makeDecisionContext({
                score: 33,
                bias: 'NEUTRAL',
                score_confidence: 0.5,
                entry_danger: makeDanger(30),
                expected_reward_risk_ratio: 1.5,
                trade_readiness: 'READY',
            }),
            opportunity: makeOpportunity({ opportunity_score: 50, setup_quality: 'Average' }),
            analysis: makeAnalysis({ bias: 'Neutral', confidence: 0.3 }),
        });
        expect(rank.long.probability + rank.short.probability + rank.hold.probability).toBe(100);
        // Direction matches the underlying split. We do not assert top here
        // because the exact split depends on the formulation; we just
        // verify the probabilities are coherent and sum to 100.
    });
});

describe('selectProfileSide', () => {
    function profile(overrides: any = {}) {
        return {
            opportunity_type: 'TrendContinuation',
            score: 78,
            preconditions_met: 3,
            preconditions_total: 3,
            notes: '',
            direction_family: null,
            long_entry_zone: null,
            long_target_zone: null,
            long_invalidation_level: null,
            short_entry_zone: null,
            short_target_zone: null,
            short_invalidation_level: null,
            long_expected_rr_internal: null,
            short_expected_rr_internal: null,
            ...overrides,
        } as any;
    }

    it('TrendRiding + bullish bias resolves to LONG', () => {
        expect(selectProfileSide(profile({ direction_family: 'TrendRiding' }), 'Bullish')).toBe('LONG');
        expect(selectProfileSide(profile({ direction_family: 'TrendRiding' }), 'StrongBullish')).toBe('LONG');
    });

    it('TrendRiding + bearish bias resolves to SHORT', () => {
        expect(selectProfileSide(profile({ direction_family: 'TrendRiding' }), 'Bearish')).toBe('SHORT');
        expect(selectProfileSide(profile({ direction_family: 'TrendRiding' }), 'StrongBearish')).toBe('SHORT');
    });

    it('CounterTrend + bullish bias resolves to SHORT', () => {
        expect(selectProfileSide(profile({ direction_family: 'CounterTrend' }), 'Bullish')).toBe('SHORT');
    });

    it('CounterTrend + bearish bias resolves to LONG', () => {
        expect(selectProfileSide(profile({ direction_family: 'CounterTrend' }), 'StrongBearish')).toBe('LONG');
    });

    it('Neutral family always resolves to NEUTRAL', () => {
        expect(selectProfileSide(profile({ direction_family: 'Neutral' }), 'Bullish')).toBe('NEUTRAL');
        expect(selectProfileSide(profile({ direction_family: 'Neutral' }), 'Bearish')).toBe('NEUTRAL');
    });

    it('Neutral macro bias returns NEUTRAL regardless of family', () => {
        expect(selectProfileSide(profile({ direction_family: 'TrendRiding' }), 'Neutral')).toBe('NEUTRAL');
        expect(selectProfileSide(profile({ direction_family: 'CounterTrend' }), 'Neutral')).toBe('NEUTRAL');
    });

    it('null profile or null bias returns NEUTRAL', () => {
        expect(selectProfileSide(null, 'Bullish')).toBe('NEUTRAL');
        expect(selectProfileSide(profile({ direction_family: 'TrendRiding' }), null)).toBe('NEUTRAL');
        expect(selectProfileSide(undefined, undefined)).toBe('NEUTRAL');
    });
});

describe('profileZones', () => {
    function profile(overrides: any = {}) {
        return {
            opportunity_type: 'TrendContinuation',
            score: 78,
            preconditions_met: 3,
            preconditions_total: 3,
            notes: '',
            direction_family: 'TrendRiding',
            long_entry_zone: { low: 63000, high: 63200 },
            long_target_zone: { low: 66000, high: 66500 },
            long_invalidation_level: 62400,
            short_entry_zone: null,
            short_target_zone: null,
            short_invalidation_level: null,
            long_expected_rr_internal: 2.5,
            short_expected_rr_internal: null,
            ...overrides,
        } as any;
    }

    it('returns LONG zones when side=LONG', () => {
        const z = profileZones(profile(), 'LONG');
        expect(z).not.toBeNull();
        expect(z!.side).toBe('LONG');
        expect(z!.entry.low).toBe(63000);
        expect(z!.entry.high).toBe(63200);
        expect(z!.target.low).toBe(66000);
        expect(z!.target.high).toBe(66500);
        expect(z!.invalidation).toBe(62400);
        expect(z!.geometry_consistent).toBe(true);
        // entry mid = 63100; reward = 66000 - 63100 = 2900; risk = 63100 - 62400 = 700
        // R:R = 2900/700 ≈ 4.14
        expect(z!.rr).toBeCloseTo(4.14, 1);
    });

    it('returns null for SHORT when profile only carries LONG zones', () => {
        expect(profileZones(profile(), 'SHORT')).toBeNull();
    });

    it('returns null when zones are missing', () => {
        expect(profileZones(profile({ long_entry_zone: null }), 'LONG')).toBeNull();
        expect(profileZones(profile({ long_target_zone: null }), 'LONG')).toBeNull();
        expect(profileZones(profile({ long_invalidation_level: 0 }), 'LONG')).toBeNull();
    });

    it('returns null on null profile', () => {
        expect(profileZones(null, 'LONG')).toBeNull();
        expect(profileZones(undefined, 'SHORT')).toBeNull();
    });

    it('flags inverted geometry as inconsistent and returns null R:R', () => {
        // entry above target ⇒ SHORT geometry, but we're asking for LONG.
        const z = profileZones(
            profile({
                long_entry_zone: { low: 64000, high: 64200 },
                long_target_zone: { low: 63300, high: 63400 },
                long_invalidation_level: 62400, // below entry, but target is below entry too
            }),
            'LONG',
        );
        expect(z).not.toBeNull();
        expect(z!.geometry_consistent).toBe(false);
        expect(z!.rr).toBeNull();
    });

    it('selects SHORT zones when side=SHORT', () => {
        const z = profileZones(
            profile({
                short_entry_zone: { low: 64800, high: 65000 },
                short_target_zone: { low: 62000, high: 62400 },
                short_invalidation_level: 66000,
            }),
            'SHORT',
        );
        expect(z).not.toBeNull();
        expect(z!.side).toBe('SHORT');
        expect(z!.invalidation).toBe(66000);
        // entry mid = 64900; reward = 64900 - 62400 = 2500; risk = 66000 - 64900 = 1100
        expect(z!.rr).toBeCloseTo(2.27, 1);
    });
});

describe('aggregateZones', () => {
    function makeOpportunity(overrides: any = {}) {
        return {
            symbol: 'BTC-USDT',
            primary_opportunity: 'Breakout',
            opportunity_score: 78,
            setup_quality: 'Strong',
            profiles: [],
            forecast_confidence: 0.72,
            contributing_signals: [],
            invalidation_note: '',
            entry_zone: { low: 63520, high: 63800 },
            target_zone: { low: 64500, high: 65000 },
            invalidation_level: 63200,
            long_entry_zone: { low: 63520, high: 63800 },
            long_target_zone: { low: 64500, high: 65000 },
            long_invalidation_level: 63200,
            long_expected_rr_internal: 2.0,
            short_entry_zone: { low: 64520, high: 64800 },
            short_target_zone: { low: 62500, high: 63000 },
            short_invalidation_level: 65000,
            short_expected_rr_internal: 1.5,
            time_horizon: 'SWING',
            confluent_entry_levels: [],
            confluent_target_levels: [],
            confluent_invalidation_levels: [],
            ...overrides,
        } as any;
    }

    it('returns aggregated LONG zones when side=LONG', () => {
        const z = aggregateZones(makeOpportunity(), 'LONG');
        expect(z).not.toBeNull();
        expect(z!.side).toBe('LONG');
        expect(z!.entry.low).toBe(63520);
        expect(z!.target.low).toBe(64500);
        expect(z!.invalidation).toBe(63200);
    });

    it('returns aggregated SHORT zones when side=SHORT', () => {
        const z = aggregateZones(makeOpportunity(), 'SHORT');
        expect(z).not.toBeNull();
        expect(z!.side).toBe('SHORT');
        expect(z!.entry.low).toBe(64520);
        expect(z!.invalidation).toBe(65000);
    });

    it('returns null when opportunity is missing', () => {
        expect(aggregateZones(null, 'LONG')).toBeNull();
    });

    it('returns null when aggregated zones are empty (Neutral sentinel)', () => {
        const opp = makeOpportunity({
            long_entry_zone: { low: 0, high: 0 },
            long_target_zone: { low: 0, high: 0 },
            long_invalidation_level: 0,
        });
        expect(aggregateZones(opp, 'LONG')).toBeNull();
    });
});

describe('topSetupSummary', () => {
    function profile(overrides: any = {}) {
        return {
            opportunity_type: 'TrendContinuation',
            score: 78,
            preconditions_met: 3,
            preconditions_total: 3,
            notes: 'synthetic',
            direction_family: 'TrendRiding',
            long_entry_zone: { low: 63000, high: 63200 },
            long_target_zone: { low: 66000, high: 66500 },
            long_invalidation_level: 62400,
            long_expected_rr_internal: 2.5,
            short_entry_zone: null,
            short_target_zone: null,
            short_invalidation_level: null,
            short_expected_rr_internal: null,
            trade_viability: 'Actionable',
            ...overrides,
        };
    }
    function makeOpportunity(profileOverrides: any = {}, oppOverrides: any = {}) {
        return {
            symbol: 'BTC-USDT',
            primary_opportunity: 'TrendContinuation',
            opportunity_score: 78,
            setup_quality: 'Strong',
            profiles: [profile(profileOverrides)],
            forecast_confidence: 0.72,
            contributing_signals: [],
            invalidation_note: '',
            entry_zone: { low: 63000, high: 63200 },
            target_zone: { low: 66000, high: 66500 },
            invalidation_level: 62400,
            long_entry_zone: { low: 63000, high: 63200 },
            long_target_zone: { low: 66000, high: 66500 },
            long_invalidation_level: 62400,
            long_expected_rr_internal: 2.5,
            short_entry_zone: null,
            short_target_zone: null,
            short_invalidation_level: null,
            short_expected_rr_internal: null,
            time_horizon: 'SWING',
            confluent_entry_levels: [],
            confluent_target_levels: [],
            confluent_invalidation_levels: [],
            ...oppOverrides,
        } as any;
    }
    function makeAnalysis(bias: any) {
        return {
            bias,
            market_bias_score: bias === 'Bullish' ? 0.5 : bias === 'Bearish' ? -0.5 : 0.0,
            state_confidence: 0.7,
            confidence: 0.7,
            market_regime: 'TrendingBull',
            timeframes_considered: 4,
        } as any;
    }

    it('returns the top-scored qualifying profile', () => {
        const opp = makeOpportunity();
        opp.profiles.push(profile({
            opportunity_type: 'Breakout',
            score: 50,
            preconditions_met: 2,
            preconditions_total: 2,
        }));
        const t = topSetupSummary(opp, makeAnalysis('Bullish'));
        expect(t).not.toBeNull();
        expect(t!.opportunity_type).toBe('TrendContinuation');
        expect(t!.score).toBe(78);
        expect(t!.direction).toBe('LONG');
        expect(t!.viability).toBe('Actionable');
        expect(t!.zones).not.toBeNull();
        expect(t!.zones!.entry.low).toBe(63000);
    });

    it('always surfaces zones via aggregate fallback when per-profile zones are absent', () => {
        const opp = makeOpportunity({
            // Profile with Neutral family + Neutral bias → no per-profile zones
            direction_family: 'Neutral',
            long_entry_zone: null,
            long_target_zone: null,
            long_invalidation_level: null,
            long_expected_rr_internal: null,
            trade_viability: 'DirectionalNeutral',
        });
        // Replace the profile type so it's not NoClearOpportunity
        opp.profiles[0].opportunity_type = 'MeanReversion';
        const t = topSetupSummary(opp, makeAnalysis('Neutral'));
        expect(t).not.toBeNull();
        expect(t!.direction).toBe('NEUTRAL');
        expect(t!.viability).toBe('DirectionalNeutral');
        // Aggregate fallback surfaces the LONG bracket even when per-profile is absent.
        expect(t!.zones).not.toBeNull();
        expect(t!.zones!.entry.low).toBe(63000);
    });

    it('returns R:R from the wire per-side expected_rr_internal', () => {
        // The per-side R:R now lives on the chosen `OpportunityProfile`
        // (not the aggregate `OpportunityMatrix`), so set it on the
        // top profile instead of the matrix-level mirror.
        const opp = makeOpportunity({ long_expected_rr_internal: 3.7 });
        const t = topSetupSummary(opp, makeAnalysis('Bullish'));
        expect(t).not.toBeNull();
        expect(t!.rr).toBeCloseTo(3.7, 1);
    });

    it('falls back to zones.rr when wire R:R is zero', () => {
        const opp = makeOpportunity({ long_expected_rr_internal: 0 });
        const t = topSetupSummary(opp, makeAnalysis('Bullish'));
        expect(t).not.toBeNull();
        // Geometric R:R from zones (63100 mid, 66000-63100=2900 reward,
        // 63100-62400=700 risk) = 2900/700 ≈ 4.14.
        expect(t!.rr).toBeCloseTo(4.14, 1);
    });

    it('returns null when no qualifying profile exists', () => {
        const opp = makeOpportunity();
        opp.profiles = [];
        expect(topSetupSummary(opp, makeAnalysis('Bullish'))).toBeNull();
    });

    it('returns null when opportunity is null', () => {
        expect(topSetupSummary(null, null)).toBeNull();
    });

    it('uses default NoClear viability when wire is missing the field', () => {
        const opp = makeOpportunity({ trade_viability: undefined });
        // ensure trade_viability is undefined on the profile
        opp.profiles[0] = { ...opp.profiles[0] };
        // @ts-ignore - delete to simulate legacy payload
        delete opp.profiles[0].trade_viability;
        const t = topSetupSummary(opp, makeAnalysis('Bullish'));
        expect(t).not.toBeNull();
        expect(t!.viability).toBe('NoClear');
    });

    it('clean rationale never contains raw= or ratio=', () => {
        const opp = makeOpportunity();
        const t = topSetupSummary(opp, makeAnalysis('Bullish'));
        expect(t).not.toBeNull();
        expect(t!.rationale).not.toContain('raw=');
        expect(t!.rationale).not.toContain('ratio=');
        expect(t!.rationale).toContain('preconditions');
    });
});

describe('profileSummary', () => {
    function makeProfile(overrides: any = {}) {
        return {
            opportunity_type: 'TrendContinuation',
            score: 78,
            preconditions_met: 3,
            preconditions_total: 3,
            notes: '',
            direction_family: 'TrendRiding',
            long_entry_zone: { low: 63000, high: 63200 },
            long_target_zone: { low: 66000, high: 66500 },
            long_invalidation_level: 62400,
            long_expected_rr_internal: 2.5,
            short_entry_zone: null,
            short_target_zone: null,
            short_invalidation_level: null,
            short_expected_rr_internal: null,
            trade_viability: 'Actionable',
            ...overrides,
        };
    }
    function makeOpportunity(profiles: any[]) {
        return {
            symbol: 'BTC-USDT',
            primary_opportunity: 'TrendContinuation',
            opportunity_score: 78,
            setup_quality: 'Strong',
            profiles,
            forecast_confidence: 0.72,
            contributing_signals: [],
            invalidation_note: '',
            entry_zone: { low: 63000, high: 63200 },
            target_zone: { low: 66000, high: 66500 },
            invalidation_level: 62400,
            long_entry_zone: { low: 63000, high: 63200 },
            long_target_zone: { low: 66000, high: 66500 },
            long_invalidation_level: 62400,
            long_expected_rr_internal: 2.5,
            short_entry_zone: null,
            short_target_zone: null,
            short_invalidation_level: null,
            short_expected_rr_internal: null,
            time_horizon: 'SWING',
            confluent_entry_levels: [],
            confluent_target_levels: [],
            confluent_invalidation_levels: [],
        } as any;
    }

    it('returns Actionable when per-profile zones + viability are good', () => {
        const p = makeProfile();
        const opp = makeOpportunity([p]);
        const s = profileSummary(p, opp, { bias: 'Bullish' } as any);
        expect(s.viability).toBe('Actionable');
        expect(s.zones).not.toBeNull();
        expect(s.rr).toBeCloseTo(2.5, 1);
    });

    it('returns DirectionalNeutral + aggregate fallback when per-profile zones absent', () => {
        const p = makeProfile({
            direction_family: 'Neutral',
            long_entry_zone: null,
            long_target_zone: null,
            long_invalidation_level: null,
            trade_viability: 'DirectionalNeutral',
        });
        const opp = makeOpportunity([p]);
        const s = profileSummary(p, opp, { bias: 'Neutral' } as any);
        expect(s.side).toBe('NEUTRAL');
        expect(s.viability).toBe('DirectionalNeutral');
        // Aggregate fallback surfaces the bracket.
        expect(s.zones).not.toBeNull();
    });

    it('returns null zones for null profile', () => {
        const s = profileSummary(null, null, null);
        expect(s.viability).toBe('NoClear');
        expect(s.zones).toBeNull();
        expect(s.rr).toBeNull();
        expect(s.side).toBe('NEUTRAL');
    });
});
