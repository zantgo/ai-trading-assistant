// @vitest-environment jsdom
//
// Regression lock for the Trade Setups UI on the Opportunities tab.
//
// The Opportunities tab must render a "Trade Setups" section listing one
// actionable card per qualifying `OpportunityMatrix.profiles` entry
// (`preconditions_met > 0`). Each card reads entry/target/stop-loss/
// reward-to-risk from the per-profile zones carried on the wire (NOT the
// aggregated `OpportunityMatrix.entry_zone` mirror).
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
        bias: 'Bullish',
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
        long_expected_rr_internal: 1.0,
        short_expected_rr_internal: 0,
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

    it('renders directional conviction bars with a visible Range portion', () => {
        const opp = makeOpportunity() as any;
        opp.profiles = [
            {
                opportunity_type: 'TrendContinuation',
                score: 54,
                preconditions_met: 3,
                preconditions_total: 3,
                notes: 'Single bullish setup',
                direction_family: 'TrendRiding',
                long_entry_zone: { low: 63000, high: 63200 },
                long_target_zone: { low: 65000, high: 65500 },
                long_invalidation_level: 62400,
                long_expected_rr_internal: 1.5,
                short_entry_zone: null,
                short_target_zone: null,
                short_invalidation_level: null,
                short_expected_rr_internal: null,
                trade_viability: 'Actionable',
            },
        ];
        opp.long_entry_zone = { low: 63000, high: 63200 };
        opp.short_entry_zone = null;
        opp.short_target_zone = null;
        opp.short_invalidation_level = null;
        opp.short_expected_rr_internal = 0;
        opp.long_expected_rr_internal = 1.5;
        opp.opportunity_score = 78;

        seedSnapshot('BTC-USDT', opp, 64000);
        render(OpportunitiesPanel, { props: { pairKey: 'BTC-USDT' } });

        const bullishCell = screen.getAllByText('BULLISH')[0].closest('div');
        const bearishCell = screen.getAllByText('BEARISH')[0].closest('div');
        const bullishFill = bullishCell?.querySelector('div');
        const bearishFill = bearishCell?.querySelector('div');

        expect(screen.getAllByText('RANGE').length).toBeGreaterThanOrEqual(1);
        // v6.10.6: conviction comes from the ACTIVE side only (bullish),
        // capped by opportunity_score (78) → BULLISH 78% / BEARISH 0%.
        expect(bullishFill?.getAttribute('style')).toMatch(/width: 7[0-9]%/);
        expect(bearishFill?.getAttribute('style')).toContain('width: 0%');
    });

    it('renders 100% Range when no qualifying setups exist', () => {
        const opp = makeOpportunity() as any;
        opp.profiles = [];
        opp.primary_opportunity = 'NoClearOpportunity';
        opp.opportunity_score = 0;
        opp.setup_quality = 'None';
        opp.long_expected_rr_internal = 0;
        opp.short_expected_rr_internal = 0;

        seedSnapshot('BTC-USDT', opp, 64000);
        render(OpportunitiesPanel, { props: { pairKey: 'BTC-USDT' } });

        expect(screen.getAllByText('RANGE').length).toBeGreaterThanOrEqual(1);
        const rangeCell = screen.getAllByText('RANGE')[0].closest('div');
        const rangeFill = rangeCell?.querySelector('div');
        expect(rangeFill?.getAttribute('style')).toContain('width: 100%');
    });

    /// Regression: all three directional bars (BULLISH / BEARISH / RANGE)
    /// are ALWAYS rendered, even when one or more are at 0%. Previously
    /// zero-value bars were filtered out, which hid the dominant-RANGE
    /// case behind a single RANGE=100% bar. The user couldn't see that
    /// bullish and bearish were also genuinely zero. Now all three
    /// render explicitly with their actual split.
    it('renders all three directional bars even when bullish/bearish are zero', () => {
        const opp = makeOpportunity() as any;
        opp.profiles = [];
        opp.primary_opportunity = 'NoClearOpportunity';
        opp.opportunity_score = 0;
        opp.setup_quality = 'None';
        opp.long_expected_rr_internal = 0;
        opp.short_expected_rr_internal = 0;

        seedSnapshot('BTC-USDT', opp, 64000);
        render(OpportunitiesPanel, { props: { pairKey: 'BTC-USDT' } });

        // All three labels must be present regardless of value — the
        // v6.10.19b section headers also carry these words, so scope to
        // the directional bars (first occurrence).
        expect(screen.getAllByText('BULLISH').length).toBeGreaterThanOrEqual(1);
        expect(screen.getAllByText('BEARISH').length).toBeGreaterThanOrEqual(1);
        expect(screen.getAllByText('RANGE').length).toBeGreaterThanOrEqual(1);

        // BULLISH bar is rendered with width 0% (the browser normalises
        // `0.0%` to `0%`).
        const bullishCell = screen.getAllByText('BULLISH')[0].closest('div');
        const bullishFill = bullishCell?.querySelector('div');
        expect(bullishFill?.getAttribute('style')).toContain('width: 0%');
        // BEARISH bar likewise.
        const bearishCell = screen.getAllByText('BEARISH')[0].closest('div');
        const bearishFill = bearishCell?.querySelector('div');
        expect(bearishFill?.getAttribute('style')).toContain('width: 0%');
        // RANGE is full width.
        const rangeCell = screen.getAllByText('RANGE')[0].closest('div');
        const rangeFill = rangeCell?.querySelector('div');
        expect(rangeFill?.getAttribute('style')).toContain('width: 100%');
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
        // The current Trade Setup card shows ENTRY/TAKE-PROFIT 1/
        // STOP-LOSS/REWARD-TO-RISK RATIO only (4-row compact layout);
        // 66000 (TP1) is rendered, 66500 is implicit via the ratio.
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
            bias: 'Bullish',
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
        expect(screen.getAllByText('FIBONACCI').length).toBeGreaterThan(0);
        expect(screen.getAllByText('VOLUME PROFILE').length).toBeGreaterThan(0);
        expect(screen.getAllByText('PIVOT POINTS').length).toBeGreaterThan(0);
    });
});

describe('OpportunitiesPanel — HOLD scenario', () => {
    it('renders the NoClearOpportunity placeholder strip + HOLD banner when no qualifying profiles exist', () => {
        const opp = makeOpportunity();
        opp.primary_opportunity = 'NoClearOpportunity';
        opp.opportunity_score = 0;
        opp.setup_quality = 'None';
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
            bias: 'Neutral',
            score_confidence: 0,
            expected_reward_risk_ratio: 0,
            trade_readiness: 'WATCH',
        };

        render(OpportunitiesPanel, { props: { pairKey: 'BTC-USDT' } });
        // v6.10.19c (A): the HOLD banner and the NO CLEAR strip were
        // erased — the RANGE/BULLISH/BEARISH sections are the container
        // for the empty state.
        expect(screen.queryByText('HOLD / NO CLEAR')).toBeNull();
        expect(screen.queryByText('NO CLEAR OPPORTUNITY')).toBeNull();
        expect(screen.getAllByText('RANGE').length).toBeGreaterThanOrEqual(1);
        expect(screen.getAllByText('BULLISH').length).toBeGreaterThanOrEqual(1);
        expect(screen.getAllByText('BEARISH').length).toBeGreaterThanOrEqual(1);
        expect(screen.getByText('no range setups')).toBeTruthy();
    });

    it('shows N/A — no directional bias in R:R (Internal) when verdict is HOLD', () => {
        const opp = makeOpportunity();
        opp.primary_opportunity = 'NoClearOpportunity';
        opp.opportunity_score = 0;
        opp.setup_quality = 'None';
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
            bias: 'Neutral',
            score_confidence: 0,
            expected_reward_risk_ratio: 0,
            trade_readiness: 'WATCH',
        };

        render(OpportunitiesPanel, { props: { pairKey: 'BTC-USDT' } });
        // R:R (Internal) shows N/A instead of a misleading 0.00.
        expect(screen.getAllByText(/N\/A/).length).toBeGreaterThan(0);
    });
});

describe('OpportunitiesPanel — v6.10.21 state-driven cards, folder references, quality pills', () => {
    it('renders quality level pills banded on the displayed score next to every card', () => {
        seedSnapshot('BTC-USDT', makeOpportunity(), 64000);
        render(OpportunitiesPanel, { props: { pairKey: 'BTC-USDT' } });
        // TC 78/3 → STRONG (the old hero quality badge was removed in
        // v7.1 — the card pills are the only quality surface now);
        // BO 60/2 and PB 50/2 → MODERATE pills on the two remaining cards.
        expect(screen.getAllByText('STRONG').length).toBeGreaterThanOrEqual(1);
        expect(screen.getAllByText('MODERATE').length).toBeGreaterThanOrEqual(2);
        expect(screen.queryByText('PRIME')).toBeNull();
    });

    it('v6.14: the backend display_score wins over the local scaling rule (drift guard)', () => {
        // A 2/3-precondition profile: the local legacy rule would compute
        // round(78 × 2/3) = 52, but the wire carries the authoritative
        // value 41 — the panel must render the wire value (single source
        // of truth, no frontend drift).
        const opp = makeOpportunity() as any;
        opp.profiles = [
            {
                ...opp.profiles[0],
                score: 78,
                preconditions_met: 2,
                preconditions_total: 3,
                display_score: 41,
            },
        ];
        seedSnapshot('BTC-USDT', opp, 64000);
        render(OpportunitiesPanel, { props: { pairKey: 'BTC-USDT' } });
        expect(screen.getAllByText('41').length).toBeGreaterThanOrEqual(1);
        // The locally-computed value (52) must never appear.
        expect(screen.queryByText('52')).toBeNull();
    });

    it('v6.14: legacy payloads without display_score fall back to the local rule', () => {
        // A 2/3-precondition profile WITHOUT the wire field: the panel
        // must reproduce the legacy displayScore rule (round(78 × 2/3) = 52).
        const opp = makeOpportunity() as any;
        opp.profiles = [
            {
                ...opp.profiles[0],
                score: 78,
                preconditions_met: 2,
                preconditions_total: 3,
                display_score: null,
            },
        ];
        seedSnapshot('BTC-USDT', opp, 64000);
        render(OpportunitiesPanel, { props: { pairKey: 'BTC-USDT' } });
        expect(screen.getAllByText('52').length).toBeGreaterThanOrEqual(1);
    });

    it('bands the pill at the 85/70/50/30 quality boundaries', () => {        const opp = makeOpportunity() as any;
        const mk = (type: string, score: number) => ({
            opportunity_type: type,
            score,
            preconditions_met: 3,
            preconditions_total: 3,
            notes: '',
            direction_family: 'TrendRiding',
            long_entry_zone: { low: 63000, high: 63200 },
            long_target_zone: { low: 66000, high: 66500 },
            long_invalidation_level: 62400,
            long_expected_rr_internal: 2.0,
            short_entry_zone: null,
            short_target_zone: null,
            short_invalidation_level: null,
            short_expected_rr_internal: null,
            trade_viability: 'Actionable',
        });
        opp.profiles = [
            mk('TrendContinuation', 85),
            mk('Breakout', 70),
            mk('Pullback', 50),
            mk('MeanReversion', 30),
            mk('Reversal', 29),
        ];
        opp.opportunity_score = 78;
        seedSnapshot('BTC-USDT', opp, 64000);
        render(OpportunitiesPanel, { props: { pairKey: 'BTC-USDT' } });
        expect(screen.getAllByText('PRIME').length).toBeGreaterThanOrEqual(1);
        expect(screen.getAllByText('STRONG').length).toBeGreaterThanOrEqual(1);
        expect(screen.getAllByText('MODERATE').length).toBeGreaterThanOrEqual(1);
        expect(screen.getAllByText('MARGINAL').length).toBeGreaterThanOrEqual(1);
        expect(screen.getAllByText('NONE').length).toBeGreaterThanOrEqual(1);
    });

    it('badges EVERY actionable card — TOP for the top-ranked, ACTIONABLE for the rest', () => {
        seedSnapshot('BTC-USDT', makeOpportunity(), 64000);
        render(OpportunitiesPanel, { props: { pairKey: 'BTC-USDT' } });
        expect(screen.getByText('TOP · ACTIONABLE')).toBeTruthy();
        // BO + PB render plain ACTIONABLE badges (exact text match).
        expect(screen.getAllByText('ACTIONABLE').length).toBeGreaterThanOrEqual(2);
    });

    it('keeps the actionable badge under a HOLD verdict (verdict gate removed)', () => {
        seedSnapshot('BTC-USDT', makeOpportunity(), 64000);
        const app = useAppStore();
        const entry = app.instancesMap['BTC-USDT'];
        // Bias stays Bullish (cards resolve LONG + R:R) but the
        // probability split makes the L6 verdict HOLD.
        entry.decisionContext = {
            ...entry.decisionContext!,
            long_probability: 20,
            short_probability: 20,
            hold_probability: 60,
        };
        render(OpportunitiesPanel, { props: { pairKey: 'BTC-USDT' } });
        expect(screen.getByText('TOP · ACTIONABLE')).toBeTruthy();
        expect(screen.getAllByText('ACTIONABLE').length).toBeGreaterThanOrEqual(2);
    });

    it('mounts per-direction reference brackets inside their folders when the folder hosts no setups', () => {
        const opp = makeOpportunity() as any;
        opp.profiles = [];
        opp.primary_opportunity = 'NoClearOpportunity';
        opp.opportunity_score = 0;
        opp.setup_quality = 'None';
        seedSnapshot('BTC-USDT', opp, 64000);
        render(OpportunitiesPanel, { props: { pairKey: 'BTC-USDT' } });
        expect(screen.getByText('Reference Bracket · LONG')).toBeTruthy();
        expect(screen.getByText('Reference Bracket · SHORT')).toBeTruthy();
        expect(screen.getAllByText('INFORMATIONAL').length).toBeGreaterThanOrEqual(2);
        // Empty-state placeholders are suppressed while a reference card
        // occupies the folder; the RANGE folder (no neutral bracket on
        // the wire) keeps its placeholder.
        expect(screen.queryByText('no bullish setups')).toBeNull();
        expect(screen.queryByText('no bearish setups')).toBeNull();
        expect(screen.getByText('no range setups')).toBeTruthy();
    });

    it('renders the backend neutral range bracket inside the RANGE folder', () => {
        const opp = makeOpportunity() as any;
        opp.profiles = [];
        opp.primary_opportunity = 'NoClearOpportunity';
        opp.opportunity_score = 0;
        opp.setup_quality = 'None';
        opp.neutral_reference_bracket = {
            entry_zone: { low: 63900, high: 64100 },
            target_zone: { low: 64900, high: 65100 },
            invalidation_level: 62000,
            expected_rr_internal: 1.5,
            geometry_consistent: true,
            rationale: 'range reference — no directional setup',
        };
        seedSnapshot('BTC-USDT', opp, 64000);
        render(OpportunitiesPanel, { props: { pairKey: 'BTC-USDT' } });
        expect(screen.getByText('Reference Bracket · RANGE')).toBeTruthy();
        expect(screen.queryByText('no range setups')).toBeNull();
        expect(screen.getByText('range reference — no directional setup')).toBeTruthy();
    });

    it('demotes a below-floor reference bracket to State D (BELOW ACTIONABLE FLOOR)', () => {
        const opp = makeOpportunity() as any;
        opp.profiles = [];
        opp.primary_opportunity = 'NoClearOpportunity';
        opp.opportunity_score = 0;
        opp.setup_quality = 'None';
        // LONG wire R:R drops under the 1.0 actionable floor; SHORT
        // keeps its valid geometric ratio → stays informational.
        opp.long_expected_rr_internal = 0.4;
        seedSnapshot('BTC-USDT', opp, 64000);
        render(OpportunitiesPanel, { props: { pairKey: 'BTC-USDT' } });
        expect(screen.getAllByText('BELOW ACTIONABLE FLOOR').length).toBeGreaterThanOrEqual(1);
        expect(screen.getAllByText('INFORMATIONAL').length).toBeGreaterThanOrEqual(1);
    });

    it('renders a geometry-inverted card with the warning badge and an N/A R:R', () => {
        const opp = makeOpportunity() as any;
        opp.profiles = [
            {
                opportunity_type: 'Breakout',
                score: 60,
                preconditions_met: 2,
                preconditions_total: 2,
                notes: '',
                direction_family: 'TrendRiding',
                long_entry_zone: { low: 63000, high: 63200 },
                // Target BELOW the entry zone → inverted geometry.
                long_target_zone: { low: 62000, high: 62500 },
                long_invalidation_level: 62400,
                long_expected_rr_internal: null,
                short_entry_zone: null,
                short_target_zone: null,
                short_invalidation_level: null,
                short_expected_rr_internal: null,
                trade_viability: 'Actionable',
            },
        ];
        opp.long_expected_rr_internal = 0;
        seedSnapshot('BTC-USDT', opp, 64000);
        render(OpportunitiesPanel, { props: { pairKey: 'BTC-USDT' } });
        expect(screen.getByText('GEOMETRY INVERTED')).toBeTruthy();
        // The R:R row renders "—" (no leaked geometric value).
        expect(screen.getAllByText('—').length).toBeGreaterThan(0);
    });

    it('renders DirectionalNeutral cards with the RANGE · NEUTRAL amber badge', () => {
        const opp = makeOpportunity() as any;
        opp.profiles = [
            {
                opportunity_type: 'MeanReversion',
                score: 42,
                preconditions_met: 2,
                preconditions_total: 3,
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
                trade_viability: 'DirectionalNeutral',
            },
        ];
        opp.primary_opportunity = 'MeanReversion';
        seedSnapshot('BTC-USDT', opp, 64000);
        render(OpportunitiesPanel, { props: { pairKey: 'BTC-USDT' } });
        expect(screen.getByText('RANGE · NEUTRAL')).toBeTruthy();
    });

    it('no longer renders the TOP SETUP hero (the Recommendation panel owns the top setup)', () => {
        seedSnapshot('BTC-USDT', makeOpportunity(), 64000);
        render(OpportunitiesPanel, { props: { pairKey: 'BTC-USDT' } });
        // v7.1: the duplicated hero (badge + lean chip + Setup Score bar +
        // quality badge) was removed — the LayerHeader badge/chip rail and
        // the Recommendation SETUP card carry that information.
        expect(screen.queryByText('TOP SETUP')).toBeNull();
        expect(screen.queryByText('Setup Score')).toBeNull();
        // The primary opportunity class still surfaces via the header badge
        // and the setup cards, and the score via the header chip rail.
        expect(screen.getAllByText(/Trend Continuation/i).length).toBeGreaterThanOrEqual(1);
    });

    it('orders the Trade Setups folders by ranking — the populated folder first (bar order)', () => {
        // The user's rule: one BEARISH setup with none elsewhere → the
        // BEARISH folder renders FIRST, exactly like the conviction bars
        // at the top rank their highest-value bar first.
        const opp = makeOpportunity() as any;
        opp.profiles = [
            {
                opportunity_type: 'Breakout',
                score: 60,
                preconditions_met: 2,
                preconditions_total: 2,
                notes: 'Single bearish setup',
                direction_family: 'TrendRiding',
                long_entry_zone: null,
                long_target_zone: null,
                long_invalidation_level: null,
                long_expected_rr_internal: null,
                short_entry_zone: { low: 65000, high: 65200 },
                short_target_zone: { low: 63000, high: 63500 },
                short_invalidation_level: 66000,
                short_expected_rr_internal: 2.0,
                trade_viability: 'Actionable',
            },
        ];
        opp.long_entry_zone = null;
        opp.long_target_zone = null;
        opp.long_invalidation_level = null;
        opp.long_expected_rr_internal = 0;
        opp.short_entry_zone = { low: 65000, high: 65200 };
        opp.short_target_zone = { low: 63000, high: 63500 };
        opp.short_invalidation_level = 66000;
        opp.short_expected_rr_internal = 2.0;

        seedSnapshot('BTC-USDT', opp, 64000);
        const app = useAppStore();
        app.instancesMap['BTC-USDT'].analysis = {
            ...app.instancesMap['BTC-USDT'].analysis!,
            bias: 'Bearish',
        };

        render(OpportunitiesPanel, { props: { pairKey: 'BTC-USDT' } });

        const headers = Array.from(document.querySelectorAll('[class*="setupSectionHeader"]'))
            .map((el) => el.textContent ?? '');
        expect(headers.length).toBe(3);
        expect(headers[0]).toContain('BEARISH');
        expect(headers[1]).toContain('RANGE');
        expect(headers[2]).toContain('BULLISH');
        // The lone bearish card lives in the first (BEARISH) folder.
        expect(screen.getAllByText(/Breakout · SHORT/).length).toBeGreaterThanOrEqual(1);
    });
});
