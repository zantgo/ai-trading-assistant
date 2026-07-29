// @vitest-environment node
// Tests for the unified Decision-tab hero consolidated in
// ui/src/lib/decisionRank.ts. These cover the rank normalisation, the
// gate-aware headline, and the symmetric Long/Short setup derivation.

import { describe, it, expect } from 'vitest';
import {
    computeDecisionRank,
    computeSymmetricSetups,
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
        final_recommendation: 'Long bias, neutral stance.',
        ...overrides,
    };
}

function makeDecisionContext(overrides: Partial<DecisionContext> = {}): DecisionContext {
    return {
        score: 0,
        bias: 'NEUTRAL',
        confidence: 0,
        score_confidence: 0,
        entry_danger: 50,
        expected_reward_risk_ratio: 0,
        trade_readiness: 'STAND_ASIDE',
        contributing_indicators: [],
        ...overrides,
    };
}

function makeOpportunity(overrides: Partial<OpportunityMatrix> = {}): OpportunityMatrix {
    return {
        symbol: 'BTC-USDT',
        primary_opportunity: 'TrendContinuation',
        opportunity_score: 50,
        setup_quality: 'Average',
        profiles: [],
        forecast_confidence: 0.5,
        contributing_signals: [],
        invalidation_note: '',
        entry_zone: { low: 63900, high: 64100 },
        target_zone: { low: 64800, high: 65000 },
        invalidation_level: 63500,
        expected_rr_internal: 2.0,
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
                entry_danger: 75,
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
                entry_danger: 20,
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
                entry_danger: 20,
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
                entry_danger: 75,
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
                entry_danger: 30,
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
                entry_danger: 30,
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

    it('mirrors the short setup around markPrice', () => {
        const setups = computeSymmetricSetups({
            opportunity: makeOpportunity({
                entry_zone: { low: 63900, high: 64100 },
                target_zone: { low: 64800, high: 65000 },
                invalidation_level: 63500,
            }),
            markPrice: 64000,
            topAction: 'HOLD',
            readiness: 'WATCH',
        });
        // Short entry should be the mirror of long entry around markPrice
        // long entry mid = 64000; mirror = 2*64000 - 64000 = 64000
        expect(setups.short.entry?.price).toBeCloseTo(64000, 0);
        // Short TP1 mirrors long TP1
        // long TP1 = 65000; mirror = 2*64000 - 65000 = 63000
        expect(setups.short.targets[0]?.price).toBeCloseTo(63000, 0);
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
        const setups = computeSymmetricSetups({
            opportunity: makeOpportunity({
                entry_zone: { low: 63900, high: 64100 },
                target_zone: { low: 64800, high: 65000 },
                invalidation_level: 63500,
            }),
            markPrice: 64000,
            topAction: 'LONG',
            readiness: 'READY',
        });
        // entry mid = 64000, TP1 = 65000, SL = 63500
        // risk = 500, reward = 1000, R:R = 2.0
        expect(setups.long.rrRatio).toBeCloseTo(2.0, 1);
    });
});
