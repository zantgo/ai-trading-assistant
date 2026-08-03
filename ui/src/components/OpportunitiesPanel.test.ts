// @vitest-environment jsdom
//
// Regression lock for the Trade Setups UI on the Opportunities tab.
//
// The Opportunities tab must render a "Trade Setups" section listing one
// actionable card per qualifying `OpportunityMatrix.profiles` entry
// (`preconditions_met > 0`). Each card reads entry/target/SL/R:R from
// the per-profile zones carried on the wire (NOT the aggregated
// `OpportunityMatrix.entry_zone` mirror).
//
// Consistency contract with the Recommendation panel:
//   - Opportunities shows the full leaderboard (one card per profile,
//     sorted by score desc).
//   - Recommendation shows ONLY the top-scored profile from the same
//     `activeSetups` derivation. Both panels call
//     `selectProfileSide(profile, macroBias)` + `profileZones(profile,
//     side)` so their numbers always agree.
//
// Bind contract: the panel reads the L4 matrix from `pair.opportunity`
// (mirrored from the WS frame in `applySnapshotToTimeframe`), NOT from
// `microTerm.latestSnapshot.opportunity` — shadow ticks in the
// `broadcast_live_snapshot` path intentionally zero the latter and
// would otherwise wipe the completed-candle payload between bars.

import { cleanup, render, screen } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import OpportunitiesPanel from './OpportunitiesPanel.svelte';
import { useAppStore } from '../state.svelte';
import type { AnalysisMatrix, MarketSnapshot, OpportunityMatrix, OpportunityProfile } from '../types';

function seedSnapshot(pairKey: string, opp: OpportunityMatrix, markPrice: number) {
    const app = useAppStore();
    const [base] = pairKey.split('-');
    if (!app.instancesMap[pairKey]) app.initInstance(base);
    const entry = app.instancesMap[pairKey];
    entry.microTerm.priceText = String(markPrice);
    const analysis: AnalysisMatrix = {
        symbol: pairKey,
        bias: 'Bullish' as AnalysisMatrix['bias'],
        confidence: 0.6,
        state_confidence: 0.6,
        market_regime: 'TRENDING_BULL' as AnalysisMatrix['market_regime'],
        trend_assessment: 'Healthy' as AnalysisMatrix['trend_assessment'],
        momentum_assessment: 'Stable' as AnalysisMatrix['momentum_assessment'],
        structure_assessment: 'Healthy' as AnalysisMatrix['structure_assessment'],
        volatility_assessment: 'Normal' as AnalysisMatrix['volatility_assessment'],
        volume_assessment: 'Normal' as AnalysisMatrix['volume_assessment'],
        opportunity_analysis: 'TrendContinuation',
        market_quality: 'Good' as AnalysisMatrix['market_quality'],
        market_quality_score: 70,
        market_phase: 'Markup' as AnalysisMatrix['market_phase'],
        market_interpretation: 'Synthetic test analysis',
        rationale: '',
        supporting_signals: [],
        contradicting_signals: [],
        timeframes_considered: 4,
    };
    entry.analysis = analysis;
    // Seed a bullish DecisionContext so `rank.top = LONG` and the
    // top-scored profile surfaces the · TOP marker.
    entry.decisionContext = {
        score: 75,
        bias: 'BULLISH',
        confidence: 0.85,
        score_confidence: 0.85,
        entry_danger: { score: 20, level: 'Low', state: 'Stable', confidence: 80, evidence: [] },
        expected_reward_risk_ratio: 1.8,
        trade_readiness: 'READY',
        contributing_indicators: ['rsi', 'macd'],
    };
    entry.advisory = {
        symbol: pairKey,
        directional_guidance: 'Long',
        market_stance: 'Constructive',
        opportunity_classification: 'TrendContinuation',
        strategy_environment: 'TrendFollowing',
        entry_guidance: 'Immediate',
        exit_guidance: 'NoWarning',
        protection_strategy: 'ATRBased',
        target_strategy: 'ResistanceBased',
        stop_loss_distance_pct: 0.015,
        confidence_assessment: 80,
        cascade_risk_score: 20,
        environment_favorability: { score: 20, level: 'Low', state: 'Stable', confidence: 50, evidence: [] },
        final_recommendation: 'Long bias',
    };
    // Mirror the WS pair-level binding. The panel reads `entry.opportunity`,
    // not `entry.microTerm.latestSnapshot.opportunity`, because shadow
    // frames hard-code the snapshot field to `None` between candle closes.
    entry.opportunity = opp;
    const snap: MarketSnapshot = {
        timestamp: 1_700_000_000,
        opportunity: opp,
    } as unknown as MarketSnapshot;
    entry.microTerm.latestSnapshot = snap as unknown as Record<string, unknown>;
}

function makeOpportunity(): OpportunityMatrix {
    // Build three profiles so the Opportunities panel surfaces a real
    // leaderboard: TrendContinuation (top, LONG) and Breakout (2nd,
    // LONG) ride the prevailing bullish trend; Pullback (3rd, LONG)
    // rides the same bias but with weaker preconditions.
    const tc: OpportunityProfile = {
        opportunity_type: 'TrendContinuation',
        score: 78,
        preconditions_met: 3,
        preconditions_total: 3,
        notes: 'Trend + bias + momentum',
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
    };
    const bo: OpportunityProfile = {
        opportunity_type: 'Breakout',
        score: 60,
        preconditions_met: 2,
        preconditions_total: 2,
        notes: 'Vol + structure',
        direction_family: 'TrendRiding',
        long_entry_zone: { low: 63400, high: 63600 },
        long_target_zone: { low: 65000, high: 65500 },
        long_invalidation_level: 62800,
        long_expected_rr_internal: 1.5,
        short_entry_zone: null,
        short_target_zone: null,
        short_invalidation_level: null,
        short_expected_rr_internal: null,
        trade_viability: 'Actionable',
    };
    const pb: OpportunityProfile = {
        opportunity_type: 'Pullback',
        score: 50,
        preconditions_met: 2,
        preconditions_total: 2,
        notes: 'Trend weakening',
        direction_family: 'TrendRiding',
        long_entry_zone: { low: 62700, high: 62900 },
        long_target_zone: { low: 64500, high: 65000 },
        long_invalidation_level: 62200,
        long_expected_rr_internal: 1.0,
        short_entry_zone: null,
        short_target_zone: null,
        short_invalidation_level: null,
        short_expected_rr_internal: null,
        trade_viability: 'Actionable',
    };
    return {
        symbol: 'BTC-USDT',
        primary_opportunity: 'TrendContinuation',
        opportunity_score: 78,
        setup_quality: 'Strong',
        profiles: [tc, bo, pb],
        forecast_confidence: 0.72,
        contributing_signals: [],
        invalidation_note: 'synthetic',
        entry_zone: { low: 63000, high: 63200 },
        target_zone: { low: 66000, high: 66500 },
        invalidation_level: 62400,
        long_entry_zone: { low: 63000, high: 63200 },
        long_target_zone: { low: 66000, high: 66500 },
        long_invalidation_level: 62400,
        short_entry_zone: { low: 65000, high: 65200 },
        short_target_zone: { low: 62000, high: 62400 },
        short_invalidation_level: 66000,
        expected_rr_internal: 2.5,
        time_horizon: 'SWING',
        confluent_entry_levels: [],
        confluent_target_levels: [],
        confluent_invalidation_levels: [],
    };
}

beforeEach(() => {
    const app = useAppStore();
    for (const key of Object.keys(app.instancesMap)) delete app.instancesMap[key];
});

afterEach(() => {
    cleanup();
});

describe('OpportunitiesPanel — per-profile Trade Setups', () => {
    it('renders one card per qualifying profile sorted by score', () => {
        seedSnapshot('BTC-USDT', makeOpportunity(), 64000);
        render(OpportunitiesPanel, { props: { pairKey: 'BTC-USDT' } });
        // Three qualifying profiles → three card headers with their type label.
        // (The opportunity badge in the header ALSO prints the primary type,
        // so use getAllByText to count occurrences across both surfaces.)
        expect(screen.getAllByText(/Trend Continuation/i).length).toBeGreaterThanOrEqual(1);
        expect(screen.getAllByText(/Breakout/i).length).toBeGreaterThanOrEqual(1);
        expect(screen.getAllByText(/Pullback/i).length).toBeGreaterThanOrEqual(1);
        // All three are LONG (TrendRiding + bullish bias).
        expect(screen.getAllByText(/LONG/i).length).toBeGreaterThanOrEqual(3);
    });

    it('top-scored profile is marked TOP · ACTIONABLE', () => {
        seedSnapshot('BTC-USDT', makeOpportunity(), 64000);
        render(OpportunitiesPanel, { props: { pairKey: 'BTC-USDT' } });
        // The TrendContinuation profile (score 78) is the top; a
        // dedicated badge surfaces `TOP · ACTIONABLE` next to it.
        expect(screen.getByText('TOP · ACTIONABLE')).toBeTruthy();
    });

    it('reads ENTRY mid from the per-profile long_entry_zone (not aggregated)', () => {
        seedSnapshot('BTC-USDT', makeOpportunity(), 64000);
        render(OpportunitiesPanel, { props: { pairKey: 'BTC-USDT' } });
        // TrendContinuation entry mid = (63000+63200)/2 = 63100
        expect(screen.getAllByText(/63100/).length).toBeGreaterThan(0);
        // Breakout entry mid = (63400+63600)/2 = 63500
        expect(screen.getAllByText(/63500/).length).toBeGreaterThan(0);
        // Pullback entry mid = (62700+62900)/2 = 62800
        expect(screen.getAllByText(/62800/).length).toBeGreaterThan(0);
    });

    it('shows TP1 as the nearest profitable target (TP ordering fix)', () => {
        seedSnapshot('BTC-USDT', makeOpportunity(), 64000);
        render(OpportunitiesPanel, { props: { pairKey: 'BTC-USDT' } });
        // TrendContinuation target {66000, 66500}; entry mid 63100.
        // TP1 (nearest) = 66000, TP2 (farther) = 66500.
        // The current Trade Setup card shows ENTRY/TP1/SL/R:R only
        // (4-row compact layout); 66000 (TP1) is rendered, 66500 is
        // implicit via R:R.
        expect(screen.getAllByText(/66000/).length).toBeGreaterThan(0);
    });

    it('shows the per-profile invalidation as the SL row', () => {
        seedSnapshot('BTC-USDT', makeOpportunity(), 64000);
        render(OpportunitiesPanel, { props: { pairKey: 'BTC-USDT' } });
        expect(screen.getAllByText(/62400/).length).toBeGreaterThan(0); // TC
        expect(screen.getAllByText(/62800/).length).toBeGreaterThan(0); // BO
        expect(screen.getAllByText(/62200/).length).toBeGreaterThan(0); // PB
    });

    it('no longer renders the legacy Directional Setups section', () => {
        seedSnapshot('BTC-USDT', makeOpportunity(), 64000);
        render(OpportunitiesPanel, { props: { pairKey: 'BTC-USDT' } });
        expect(screen.queryByText('Directional Setups')).toBeNull();
    });

    it('renders awaiting-message when no opportunity matrix is present', () => {
        const app = useAppStore();
        if (!app.instancesMap['BTC-USDT']) app.initInstance('BTC');
        app.instancesMap['BTC-USDT'].microTerm.priceText = '64000';
        render(OpportunitiesPanel, { props: { pairKey: 'BTC-USDT' } });
        expect(screen.getByText(/Trade Setups/i)).toBeTruthy();
        expect(screen.getByText(/Assessment conditions forming/)).toBeTruthy();
    });
});

describe('OpportunitiesPanel — L4 matrix binding (regression)', () => {
    it('renders L4 zones from pair.opportunity even when latestSnapshot.opportunity is null', () => {
        const app = useAppStore();
        if (!app.instancesMap['BTC-USDT']) app.initInstance('BTC');
        const entry = app.instancesMap['BTC-USDT'];
        entry.microTerm.priceText = '64000';
        // Seed a bullish analysis + advisory + decisionContext so the
        // rank resolves to LONG and the per-profile cards surface zones.
        entry.analysis = {
            ...entry.analysis!,
            bias: 'Bullish',
            market_regime: 'TRENDING_BULL',
            opportunity_analysis: 'TrendContinuation',
        } as unknown as AnalysisMatrix;
        entry.decisionContext = {
            score: 75,
            bias: 'BULLISH',
            confidence: 0.85,
            score_confidence: 0.85,
            entry_danger: { score: 20, level: 'Low', state: 'Stable', confidence: 80, evidence: [] },
            expected_reward_risk_ratio: 1.8,
            trade_readiness: 'READY',
            contributing_indicators: [],
        };
        entry.advisory = {
            symbol: 'BTC-USDT',
            directional_guidance: 'Long',
            market_stance: 'Constructive',
            opportunity_classification: 'TrendContinuation',
            strategy_environment: 'TrendFollowing',
            entry_guidance: 'Immediate',
            exit_guidance: 'NoWarning',
            protection_strategy: 'ATRBased',
            target_strategy: 'ResistanceBased',
            stop_loss_distance_pct: 0.015,
            confidence_assessment: 80,
            cascade_risk_score: 20,
            environment_favorability: { score: 20, level: 'Low', state: 'Stable', confidence: 50, evidence: [] },
            final_recommendation: 'Long bias',
        };
        entry.opportunity = makeOpportunity();
        entry.microTerm.latestSnapshot = {
            timestamp: 1_700_000_000,
            opportunity: null,
        } as unknown as Record<string, unknown>;

        render(OpportunitiesPanel, { props: { pairKey: 'BTC-USDT' } });

        // Per-profile zones surface even when the snapshot path is null.
        expect(screen.getAllByText(/63100/).length).toBeGreaterThan(0);
        expect(screen.getAllByText(/66000/).length).toBeGreaterThan(0);
        expect(screen.getAllByText(/62400/).length).toBeGreaterThan(0);
    });

    it('renders em-dashes and the forming copy when pair.opportunity is null', () => {
        const app = useAppStore();
        if (!app.instancesMap['BTC-USDT']) app.initInstance('BTC');
        const entry = app.instancesMap['BTC-USDT'];
        entry.microTerm.priceText = '64000';
        entry.opportunity = null;

        render(OpportunitiesPanel, { props: { pairKey: 'BTC-USDT' } });

        expect(screen.getByText(/Trade Setups/i)).toBeTruthy();
        expect(screen.getByText(/Assessment conditions forming/)).toBeTruthy();
    });

    it('renders the evaluated setup profiles when pair.opportunity.profiles is non-empty', () => {
        const app = useAppStore();
        if (!app.instancesMap['BTC-USDT']) app.initInstance('BTC');
        const entry = app.instancesMap['BTC-USDT'];
        entry.microTerm.priceText = '64000';
        const opp = makeOpportunity();
        opp.primary_opportunity = 'Breakout';
        opp.opportunity_score = 43;
        opp.profiles = [
            {
                opportunity_type: 'Breakout',
                score: 43,
                preconditions_met: 3,
                preconditions_total: 5,
                notes: 'volume drying up',
                direction_family: 'TrendRiding',
                long_entry_zone: { low: 63100, high: 63200 },
                long_target_zone: { low: 66000, high: 66500 },
                long_invalidation_level: 62400,
                long_expected_rr_internal: 1.0,
                short_entry_zone: null,
                short_target_zone: null,
                short_invalidation_level: null,
                short_expected_rr_internal: null,
                trade_viability: 'Actionable',
            },
            {
                opportunity_type: 'TrendContinuation',
                score: 38,
                preconditions_met: 2,
                preconditions_total: 5,
                notes: '',
                direction_family: 'TrendRiding',
                long_entry_zone: { low: 63000, high: 63200 },
                long_target_zone: { low: 66000, high: 66500 },
                long_invalidation_level: 62400,
                long_expected_rr_internal: 1.5,
                short_entry_zone: null,
                short_target_zone: null,
                short_invalidation_level: null,
                short_expected_rr_internal: null,
                trade_viability: 'Actionable',
            },
        ];
        opp.confluent_entry_levels = [
            { price: 63100, confluence_count: 3, sources: ['FIBONACCI', 'VOLUME_PROFILE', 'PIVOT_POINTS'], strength: 78 },
        ];
        opp.confluent_target_levels = [
            { price: 66500, confluence_count: 2, sources: ['FIBONACCI', 'VOLUME_PROFILE'], strength: 64 },
        ];
        entry.opportunity = opp;

        render(OpportunitiesPanel, { props: { pairKey: 'BTC-USDT' } });

        expect(screen.getByText('3/5 met')).toBeTruthy();
        expect(screen.getByText('2/5 met')).toBeTruthy();
        expect(screen.getByText('63100')).toBeTruthy();
        expect(screen.getAllByText(/66500/).length).toBeGreaterThan(0);
        expect(screen.getAllByText('FIB').length).toBeGreaterThan(0);
        expect(screen.getAllByText('VP').length).toBeGreaterThan(0);
        expect(screen.getAllByText('PP').length).toBeGreaterThan(0);
    });
});

describe('OpportunitiesPanel — HOLD scenario', () => {
    it('renders the NoClearOpportunity placeholder strip + HOLD banner when no qualifying profiles exist', () => {
        const opp = makeOpportunity();
        opp.primary_opportunity = 'NoClearOpportunity';
        opp.opportunity_score = 0;
        opp.setup_quality = 'None';
        opp.expected_rr_internal = 0;
        opp.profiles = [
            {
                opportunity_type: 'NoClearOpportunity',
                score: 0,
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
                trade_viability: 'NoClear',
            },
        ];
        opp.entry_zone = { low: 64000, high: 64000 };
        opp.target_zone = { low: 64000, high: 64000 };
        opp.invalidation_level = 64000;
        opp.long_entry_zone = { low: 64000, high: 64000 };
        opp.long_target_zone = { low: 64000, high: 64000 };
        opp.long_invalidation_level = 64000;
        opp.short_entry_zone = { low: 64000, high: 64000 };
        opp.short_target_zone = { low: 64000, high: 64000 };
        opp.short_invalidation_level = 64000;
        seedSnapshot('BTC-USDT', opp, 64000);

        const app = useAppStore();
        const entry = app.instancesMap['BTC-USDT'];
        entry.analysis = { ...entry.analysis!, bias: 'Neutral', market_regime: 'RANGE' };
        entry.advisory = { ...entry.advisory!, directional_guidance: 'Neutral' };
        entry.decisionContext = {
            ...entry.decisionContext!,
            score: 0,
            bias: 'NEUTRAL',
            score_confidence: 0,
            expected_reward_risk_ratio: 0,
            trade_readiness: 'WATCH',
        };

        render(OpportunitiesPanel, { props: { pairKey: 'BTC-USDT' } });
        // HOLD banner visible because rank.top === 'HOLD'.
        expect(screen.getByText('HOLD / NO CLEAR')).toBeTruthy();
        // NoClearOpportunity strip is rendered.
        expect(screen.getByText('NO CLEAR OPPORTUNITY')).toBeTruthy();
    });

    it('shows N/A — no directional bias in R:R (Internal) when verdict is HOLD', () => {
        const opp = makeOpportunity();
        opp.primary_opportunity = 'NoClearOpportunity';
        opp.opportunity_score = 0;
        opp.setup_quality = 'None';
        opp.expected_rr_internal = 0;
        opp.profiles = [
            {
                opportunity_type: 'NoClearOpportunity',
                score: 0,
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
                trade_viability: 'NoClear',
            },
        ];
        opp.entry_zone = { low: 64000, high: 64000 };
        opp.target_zone = { low: 64000, high: 64000 };
        opp.invalidation_level = 64000;
        opp.long_entry_zone = { low: 64000, high: 64000 };
        opp.long_target_zone = { low: 64000, high: 64000 };
        opp.long_invalidation_level = 64000;
        opp.short_entry_zone = { low: 64000, high: 64000 };
        opp.short_target_zone = { low: 64000, high: 64000 };
        opp.short_invalidation_level = 64000;
        seedSnapshot('BTC-USDT', opp, 64000);

        const app = useAppStore();
        const entry = app.instancesMap['BTC-USDT'];
        entry.analysis = { ...entry.analysis!, bias: 'Neutral', market_regime: 'RANGE' };
        entry.advisory = { ...entry.advisory!, directional_guidance: 'Neutral' };
        entry.decisionContext = {
            ...entry.decisionContext!,
            score: 0,
            bias: 'NEUTRAL',
            score_confidence: 0,
            expected_reward_risk_ratio: 0,
            trade_readiness: 'WATCH',
        };

        render(OpportunitiesPanel, { props: { pairKey: 'BTC-USDT' } });
        // R:R (Internal) shows N/A instead of a misleading 0.00.
        expect(screen.getAllByText(/N\/A/).length).toBeGreaterThan(0);
    });
});
