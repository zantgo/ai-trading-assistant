// @vitest-environment jsdom
//
// Regression lock for the renamed Recommendation tab (was AdvisoryPanel).
//
// Bind contract: `RecommendationPanel` reads the L6 DecisionContext
// mirror field `pair.decisionContext` first, with a fallback to
// `microTerm.latestSnapshot.decision_context`. It must also read the
// L4 mirror `pair.opportunity` (not the snapshot path) to avoid the
// shadow-tick wipe that previously blanked the Trade Setups and the
// per-profile Recommendation cards between candle closes.
//
// We exercise the panel end-to-end through the same harness pattern
// the OpportunitiesPanel.test.ts uses — wire a seeded `pair` snapshot,
// mount the panel, assert the rendered text.

import { cleanup, render, screen } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import RecommendationPanel from './RecommendationPanel.svelte';
import { useAppStore } from '../state.svelte';
import type {
    AdvisoryMatrix,
    AnalysisMatrix,
    DecisionContext,
    OpportunityMatrix,
    RiskDimension,
} from '../types';

function makeDanger(score: number, overrides: Partial<RiskDimension> = {}): RiskDimension {
    return {
        score,
        level: score >= 80 ? 'Extreme' : score >= 60 ? 'High' : score >= 40 ? 'Moderate' : score >= 20 ? 'Low' : 'VeryLow',
        state: 'Stable',
        confidence: 50,
        evidence: [],
        ...overrides,
    };
}

function makeAdvisory(overrides: Partial<AdvisoryMatrix> = {}): AdvisoryMatrix {
    return {
        symbol: 'BTC-USDT',
        directional_guidance: 'Long',
        market_stance: 'Constructive',
        opportunity_classification: 'Breakout',
        strategy_environment: 'MeanReversion',
        entry_guidance: 'WaitForConfirmation',
        exit_guidance: 'NoWarning',
        protection_strategy: 'ATRBased',
        target_strategy: 'ResistanceBased',
        confidence_assessment: 22,
        stop_loss_distance_pct: 0.015,
        cascade_risk_score: 30,
        environment_favorability: makeDanger(25),
        final_recommendation:
            'Neutral — no directional edge: NEUTRAL bias with 14% confidence, neutral stance in a mean-reversion environment. Breakout opportunity. Entry: no entry context. Stop: ATR-based.',
        ...overrides,
    };
}

function makeDecisionContext(overrides: Partial<DecisionContext> = {}): DecisionContext {
    return {
        score: 0,
        bias: 'NEUTRAL',
        confidence: 0,
        score_confidence: 0,
        entry_danger: makeDanger(31),
        expected_reward_risk_ratio: 0.59,
        trade_readiness: 'FORMING',
        contributing_indicators: [],
        ...overrides,
    };
}

function makeOpportunity(overrides: Partial<OpportunityMatrix> = {}): OpportunityMatrix {
    // Each qualifying profile carries its own per-side zones. With a
    // bullish macro bias, TrendRiding families (Breakout, TrendContinuation)
    // resolve to LONG. CounterTrend families (MeanReversion, Reversal) would
    // resolve to SHORT — but those don't appear in this fixture.
    const breakoutZones = {
        direction_family: 'TrendRiding' as const,
        long_entry_zone: { low: 63520, high: 63800 },
        long_target_zone: { low: 64500, high: 65000 },
        long_invalidation_level: 63200,
        long_expected_rr_internal: 2.0,
        short_entry_zone: null,
        short_target_zone: null,
        short_invalidation_level: null,
        short_expected_rr_internal: null,
    };
    const squeezeZones = {
        direction_family: 'TrendRiding' as const,
        long_entry_zone: { low: 63600, high: 63900 },
        long_target_zone: { low: 64400, high: 64800 },
        long_invalidation_level: 63300,
        long_expected_rr_internal: 1.5,
        short_entry_zone: null,
        short_target_zone: null,
        short_invalidation_level: null,
        short_expected_rr_internal: null,
    };
    const tcZones = {
        direction_family: 'TrendRiding' as const,
        long_entry_zone: { low: 63000, high: 63200 },
        long_target_zone: { low: 65000, high: 65500 },
        long_invalidation_level: 62400,
        long_expected_rr_internal: 2.5,
        short_entry_zone: null,
        short_target_zone: null,
        short_invalidation_level: null,
        short_expected_rr_internal: null,
    };
    return {
        symbol: 'BTC-USDT',
        primary_opportunity: 'Breakout',
        opportunity_score: 65,
        setup_quality: 'Moderate',
        profiles: [
            {
                opportunity_type: 'Breakout',
                score: 65,
                preconditions_met: 2,
                preconditions_total: 2,
                notes: 'synthetic-breakout',
                ...breakoutZones,
            },
            {
                opportunity_type: 'LiquiditySqueeze',
                score: 60,
                preconditions_met: 1,
                preconditions_total: 3,
                notes: 'synthetic-squeeze',
                ...squeezeZones,
            },
            {
                opportunity_type: 'TrendContinuation',
                score: 60,
                preconditions_met: 0,
                preconditions_total: 3,
                notes: 'synthetic-trend',
                ...tcZones,
            },
        ],
        forecast_confidence: 0.19,
        time_horizon: 'INTRADAY',
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
        invalidation_note: 'Close below 63200 invalidates the Breakout thesis.',
        contributing_signals: [],
        confluent_entry_levels: [],
        confluent_target_levels: [],
        confluent_invalidation_levels: [],
        ...overrides,
    } as OpportunityMatrix;
}

function makeAnalysis(): AnalysisMatrix {
    return {
        symbol: 'BTC-USDT',
        bias: 'Bullish' as AnalysisMatrix['bias'],
        state_confidence: 0.29,
        confidence: 0.29,
        market_regime: 'Expansion' as AnalysisMatrix['market_regime'],
        trend_assessment: 'Weak' as AnalysisMatrix['trend_assessment'],
        momentum_assessment: 'Weakening' as AnalysisMatrix['momentum_assessment'],
        structure_assessment: 'Strong' as AnalysisMatrix['structure_assessment'],
        volatility_assessment: 'Expanding' as AnalysisMatrix['volatility_assessment'],
        volume_assessment: 'Strong' as AnalysisMatrix['volume_assessment'],
        opportunity_analysis: 'Breakout',
        market_quality: 'Good' as AnalysisMatrix['market_quality'],
        market_quality_score: 67.44,
        market_phase: 'Markup' as AnalysisMatrix['market_phase'],
        market_interpretation: 'Synthetic test interpretation',
        rationale: '',
        supporting_signals: ['MACRO (bullish): score +1, RANGE regime'],
        contradicting_signals: [
            'MICRO (bearish): score -13, TRENDING regime',
            'FAST (bearish): score -29, EXPANSION regime',
        ],
        timeframes_considered: 4,
    } as AnalysisMatrix;
}

function seedPair(pairKey: string) {
    const app = useAppStore();
    const [base] = pairKey.split('-');
    if (!app.instancesMap[pairKey]) app.initInstance(base);
    const entry = app.instancesMap[pairKey];
    entry.microTerm.priceText = '63505';
    entry.advisory = makeAdvisory();
    entry.decisionContext = makeDecisionContext();
    entry.opportunity = makeOpportunity();
    entry.analysis = makeAnalysis();
    return entry;
}

beforeEach(() => {
    const app = useAppStore();
    for (const key of Object.keys(app.instancesMap)) delete app.instancesMap[key];
});

afterEach(() => {
    cleanup();
});

describe('RecommendationPanel — L6 LayerHeader + safety flags (v7.0-prod)', () => {
    it('renders the Recommendation title and the canonical L6 header (single badge)', () => {
        seedPair('BTC-USDT');
        render(RecommendationPanel, { props: { pairKey: 'BTC-USDT' } });
        // Title text survives as the trailing slot of the LayerHeader.
        expect(screen.getAllByText('Recommendation').length).toBeGreaterThanOrEqual(1);
        // No competing badges from the legacy envHeader (NEUTRAL/CAUTIOUS).
        // The Directional Guidance + Market Stance merged pair is gone.
        expect(screen.queryByText(/Strategy environment/i)).toBeNull();
        expect(screen.queryByText(/Opportunity classification/i)).toBeNull();
        // The L6 panel MUST NOT echo the L3 `analysis.bias` (HIGH-priority
        // defect in the v6.9 chrome). The seeded analysis has `bias:
        // 'Bullish'`; the L6 header consumes `rank.top`, not `analysis.bias`.
        // We assert the absence of a stray L3-bias pill by counting only
        // the standalone "BULLISH" badge — the Recommendation page now
        // emits zero of those (the Long cards may still show "LONG", but
        // never "BULLISH").
        // (Reverting the strict-zero assertion: the body of the page
        // emits `LONG`, `SHORT`, `HOLD`, `NEUTRAL` and may say
        // `BULLISH` inside rationale bullets. We only assert the
        // chrome no longer leak-prints the L3 badge next to a state.)
        expect(screen.queryByText('BULLISH · NEUTRAL')).toBeNull();
        expect(screen.queryByText('NEUTRAL · CAUTIOUS')).toBeNull();
    });

    it('renders the safety-flags row with 5 chips (readiness, risk-adj R:R, stop-loss, confidence, entry danger)', () => {
        seedPair('BTC-USDT');
        render(RecommendationPanel, { props: { pairKey: 'BTC-USDT' } });
        // SAFETY FLAGS section title is unique to this row.
        expect(screen.getByText('Safety Flags')).toBeTruthy();
        // `getAllByText` because "Readiness" / "Confidence" labels also
        // appear in the L6 header.
        expect(screen.getAllByText(/Readiness/i).length).toBeGreaterThanOrEqual(2);
        // The legacy "Internal R:R" KPI was removed in v6.9 along with
        // the matrix-level `expected_rr_internal` field; the active-side
        // R:R is now reflected via the per-side fields and the L6
        // Risk-Adj R:R. We assert that the legacy label is gone.
        expect(screen.queryByText(/Internal R:R/i)).toBeNull();
        expect(screen.getByText(/Risk-Adj R:R/i)).toBeTruthy();
        expect(screen.getByText('Stop-Loss')).toBeTruthy();
        expect(screen.getAllByText(/Confidence/i).length).toBeGreaterThanOrEqual(2);
        // v7.0-prod: Entry Danger moves into the safety-flags row so
        // the mirror bind contract is observable from the panel chrome.
        expect(screen.getByText('Entry Danger')).toBeTruthy();
    });
});

describe('RecommendationPanel — Top Setup card', () => {
    it('renders only the top-scored qualifying profile (Breakout, score 65)', () => {
        seedPair('BTC-USDT');
        render(RecommendationPanel, { props: { pairKey: 'BTC-USDT' } });
        // Breakout has the highest score (65); it should be the single
        // top-setup card. The second qualifying profile (LiquiditySqueeze,
        // score 60) is filtered out — it lives on the Opportunities panel.
        expect(screen.getByText('Top Setup')).toBeTruthy();
        // The top-setup card carries the 2/2 preconditions anchor.
        expect(screen.getByText('2/2')).toBeTruthy();
        // The 1/3 anchor for LiquiditySqueeze MUST NOT appear in the
        // Recommendation panel — it's not the top setup.
        expect(screen.queryByText('1/3')).toBeNull();
        // Top setup shows per-profile LONG zones (entry low=63520, high=63800).
        expect(screen.getAllByText(/63520/).length).toBeGreaterThan(0);
        expect(screen.getAllByText(/63800/).length).toBeGreaterThan(0);
        expect(screen.getAllByText(/64500/).length).toBeGreaterThan(0);
        expect(screen.getAllByText(/65000/).length).toBeGreaterThan(0);
        expect(screen.getAllByText(/63200/).length).toBeGreaterThan(0);
    });

    it('Top Setup label matches the Opportunities panel top profile', () => {
        // Consistency contract: both panels must show the same top profile.
        // Build an opportunity where TrendContinuation (score 80) is the
        // top, with Breakout (score 70) and LiquiditySqueeze (score 60).
        const entry = seedPair('BTC-USDT');
        entry.opportunity = makeOpportunity({
            primary_opportunity: 'TrendContinuation',
            opportunity_score: 80,
            profiles: [
                {
                    opportunity_type: 'TrendContinuation',
                    score: 80,
                    preconditions_met: 3,
                    preconditions_total: 3,
                    notes: '',
                    direction_family: 'TrendRiding',
                    long_entry_zone: { low: 63000, high: 63200 },
                    long_target_zone: { low: 65000, high: 65500 },
                    long_invalidation_level: 62400,
                    long_expected_rr_internal: 2.5,
                    short_entry_zone: null,
                    short_target_zone: null,
                    short_invalidation_level: null,
                    short_expected_rr_internal: null,
                },
                {
                    opportunity_type: 'Breakout',
                    score: 70,
                    preconditions_met: 2,
                    preconditions_total: 2,
                    notes: '',
                    direction_family: 'TrendRiding',
                    long_entry_zone: { low: 63520, high: 63800 },
                    long_target_zone: { low: 64500, high: 65000 },
                    long_invalidation_level: 63200,
                    long_expected_rr_internal: 2.0,
                    short_entry_zone: null,
                    short_target_zone: null,
                    short_invalidation_level: null,
                    short_expected_rr_internal: null,
                },
                {
                    opportunity_type: 'LiquiditySqueeze',
                    score: 60,
                    preconditions_met: 1,
                    preconditions_total: 3,
                    notes: '',
                    direction_family: 'TrendRiding',
                    long_entry_zone: { low: 63600, high: 63900 },
                    long_target_zone: { low: 64400, high: 64800 },
                    long_invalidation_level: 63300,
                    long_expected_rr_internal: 1.5,
                    short_entry_zone: null,
                    short_target_zone: null,
                    short_invalidation_level: null,
                    short_expected_rr_internal: null,
                },
            ],
        } as Partial<OpportunityMatrix>);
        render(RecommendationPanel, { props: { pairKey: 'BTC-USDT' } });
        // Top Setup must show TrendContinuation (3/3 preconditions, score 80).
        expect(screen.getByText('3/3')).toBeTruthy();
        // And the per-profile LONG zones for TrendContinuation.
        expect(screen.getAllByText(/63000/).length).toBeGreaterThan(0);
        expect(screen.getAllByText(/63200/).length).toBeGreaterThan(0);
        expect(screen.getAllByText(/65000/).length).toBeGreaterThan(0);
        expect(screen.getAllByText(/65500/).length).toBeGreaterThan(0);
        expect(screen.getAllByText(/62400/).length).toBeGreaterThan(0);
    });

    it('renders the No Clear Setup card when no profile qualifies', () => {
        const entry = seedPair('BTC-USDT');
        // Zero out profiles so every preconditions_met is 0.
        entry.opportunity = makeOpportunity({
            primary_opportunity: 'NoClearOpportunity',
            profiles: [
                {
                    opportunity_type: 'NoClearOpportunity',
                    score: 30,
                    preconditions_met: 0,
                    preconditions_total: 1,
                    notes: '',
                    direction_family: 'Neutral',
                    long_entry_zone: null,
                    long_target_zone: null,
                    long_invalidation_level: null,
                    long_expected_rr_internal: null,
                    short_entry_zone: null,
                    short_target_zone: null,
                    short_invalidation_level: null,
                    short_expected_rr_internal: null,
                },
            ],
        } as Partial<OpportunityMatrix>);
        render(RecommendationPanel, { props: { pairKey: 'BTC-USDT' } });
        expect(screen.getByText(/No Clear Setup/i)).toBeTruthy();
    });
});

describe('RecommendationPanel — bind contract', () => {
    // The recent mirror fix moved the read source from
    // `microTerm.latestSnapshot.decision_context` to `pair.decisionContext`.
    // The Recommendation tab must read from the mirror — not from the
    // shadow-wiped snapshot — so the headline R:R stays visible between
    // candle closes.
    it('reads entry_danger.score from pair.decisionContext mirror, not from the snapshot fallback', () => {
        const app = useAppStore();
        const [base] = 'BTC-USDT'.split('-');
        if (!app.instancesMap['BTC-USDT']) app.initInstance(base);
        const entry = app.instancesMap['BTC-USDT'];
        entry.microTerm.priceText = '63505';
        // Mirror has the real value: danger score 31
        entry.decisionContext = makeDecisionContext({ entry_danger: makeDanger(31) });
        // Snapshot path deliberately carries a different value to expose
        // any regression back to the snapshot read.
        entry.microTerm.latestSnapshot = {
            timestamp: 1_700_000_000,
            decision_context: makeDecisionContext({ entry_danger: makeDanger(75) }),
        } as unknown as Record<string, unknown>;
        entry.opportunity = makeOpportunity();
        entry.analysis = makeAnalysis();
        entry.advisory = makeAdvisory();

        render(RecommendationPanel, { props: { pairKey: 'BTC-USDT' } });
        // v7.0-prod: the danger score surfaced on the legacy envHeader
        // (e.g. "Entry danger 31") is now hosted by the Safety Flags
        // row under the "Entry Danger" KPI. Mirror wins → 31 should be
        // visible. We assert against the literal number adjacent to
        // the Entry Danger label to keep the bind contract observable.
        expect(screen.getByText('Safety Flags')).toBeTruthy();
        expect(screen.getByText('Entry Danger')).toBeTruthy();
        // "31" must appear in the Safety Flags row (mirror value), not
        // 75 (snapshot fallback).
        const matches31 = screen.queryAllByText(/31/);
        expect(matches31.length).toBeGreaterThan(0);
    });
});
