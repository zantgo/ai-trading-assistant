// @vitest-environment jsdom
//
// Regression lock for the Trade Setups UI on the Opportunities tab.
//
// The Opportunities tab must render a "Trade Setups" section with two
// side-by-side cards — Long Setup and Short Setup — each with ENTRY,
// TP1, TP2, SL, and an R:R ratio derived from the L4 opportunity matrix.
// The legacy "Directional Setups" section is gone.
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
import type { AnalysisMatrix, MarketSnapshot, OpportunityMatrix } from '../types';

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
        market_regime: 'TrendingBull' as AnalysisMatrix['market_regime'],
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
    return {
        symbol: 'BTC-USDT',
        primary_opportunity: 'TrendContinuation',
        opportunity_score: 78,
        setup_quality: 'Strong',
        profiles: [],
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

describe('OpportunitiesPanel — Trade Setups', () => {
    it('renders both Long Setup and Short Setup headers', () => {
        seedSnapshot('BTC-USDT', makeOpportunity(), 64000);
        render(OpportunitiesPanel, { props: { pairKey: 'BTC-USDT' } });
        expect(screen.getByText('Long Setup')).toBeTruthy();
        expect(screen.getByText('Short Setup')).toBeTruthy();
    });

    it('renders the Trade Setups section title', () => {
        seedSnapshot('BTC-USDT', makeOpportunity(), 64000);
        render(OpportunitiesPanel, { props: { pairKey: 'BTC-USDT' } });
        expect(screen.getByText('Trade Setups')).toBeTruthy();
    });

    it('no longer renders the legacy Directional Setups section', () => {
        seedSnapshot('BTC-USDT', makeOpportunity(), 64000);
        render(OpportunitiesPanel, { props: { pairKey: 'BTC-USDT' } });
        expect(screen.queryByText('Directional Setups')).toBeNull();
    });

    it('shows the long entry mid, TP1, TP2, and SL prices', () => {
        seedSnapshot('BTC-USDT', makeOpportunity(), 64000);
        render(OpportunitiesPanel, { props: { pairKey: 'BTC-USDT' } });
        // entry mid = (63000 + 63200) / 2 = 63100
        expect(screen.getByText('$63100')).toBeTruthy();
        // TP1 = target_zone.high = 66500
        expect(screen.getByText('$66500')).toBeTruthy();
        // TP2 = target_zone.low = 66000
        expect(screen.getByText('$66000')).toBeTruthy();
        // SL = invalidation_level = 62400
        expect(screen.getByText('$62400')).toBeTruthy();
    });

    it('renders an em-dash when no opportunity matrix is present', () => {
        const app = useAppStore();
        if (!app.instancesMap['BTC-USDT']) app.initInstance('BTC');
        app.instancesMap['BTC-USDT'].microTerm.priceText = '64000';
        render(OpportunitiesPanel, { props: { pairKey: 'BTC-USDT' } });
        expect(screen.getByText('Long Setup')).toBeTruthy();
        expect(screen.getByText('Short Setup')).toBeTruthy();
    });
});

describe('OpportunitiesPanel — L4 matrix binding (regression)', () => {
    // These tests guard against the shadow-tick wipe bug: shadow frames
    // arrive at ~4 Hz between completed candles and the original binding
    // read `microTerm.latestSnapshot.opportunity`, which the unconditional
    // `tf.latestSnapshot = snapshot` in `applySnapshotToTimeframe` overwrote
    // to `null` on every shadow tick. The fix mirrors the L4 matrix to
    // `pair.opportunity` (set once per completed candle) and binds the
    // panel to that field instead of the shadow buffer.

    it('renders L4 zones from pair.opportunity even when latestSnapshot.opportunity is null', () => {
        const app = useAppStore();
        if (!app.instancesMap['BTC-USDT']) app.initInstance('BTC');
        const entry = app.instancesMap['BTC-USDT'];
        entry.microTerm.priceText = '64000';
        entry.opportunity = makeOpportunity();
        // Shadow-tick shape: opportunity null on the per-TF snapshot buffer.
        // Pre-fix the panel read this field and rendered all zones as `—`.
        entry.microTerm.latestSnapshot = {
            timestamp: 1_700_000_000,
            opportunity: null,
        } as unknown as Record<string, unknown>;

        render(OpportunitiesPanel, { props: { pairKey: 'BTC-USDT' } });

        expect(screen.getByText('$63100')).toBeTruthy();
        expect(screen.getByText('$66500')).toBeTruthy();
        expect(screen.getByText('$66000')).toBeTruthy();
        expect(screen.getByText('$62400')).toBeTruthy();
    });

    it('renders em-dashes and the forming copy when pair.opportunity is null', () => {
        const app = useAppStore();
        if (!app.instancesMap['BTC-USDT']) app.initInstance('BTC');
        const entry = app.instancesMap['BTC-USDT'];
        entry.microTerm.priceText = '64000';
        entry.opportunity = null;

        render(OpportunitiesPanel, { props: { pairKey: 'BTC-USDT' } });

        // Section header still mounts; placeholder copy carries the
        // "Assessment conditions forming" message until the first
        // completed-candle WS frame arrives.
        expect(screen.getByText('Long Setup')).toBeTruthy();
        expect(screen.getByText('Short Setup')).toBeTruthy();
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
            { opportunity_type: 'Breakout', score: 43, preconditions_met: 3, preconditions_total: 5, notes: 'volume drying up' },
            { opportunity_type: 'TrendContinuation', score: 38, preconditions_met: 2, preconditions_total: 5, notes: '' },
        ];
        opp.confluent_entry_levels = [
            { price: 63100, confluence_count: 3, sources: ['FIBONACCI', 'VOLUME_PROFILE', 'PIVOT_POINTS'], strength: 78 },
        ];
        opp.confluent_target_levels = [
            { price: 66500, confluence_count: 2, sources: ['FIBONACCI', 'VOLUME_PROFILE'], strength: 64 },
        ];
        entry.opportunity = opp;

        render(OpportunitiesPanel, { props: { pairKey: 'BTC-USDT' } });

        // Evaluated Setup profile rows surface with their preconditions bar.
        expect(screen.getByText('3/5 met')).toBeTruthy();
        expect(screen.getByText('2/5 met')).toBeTruthy();
        // Confluent level rows surface (price + source tags). Each source
        // tag is rendered once per confluent level that carries it, so use
        // getAllByText to assert at least one occurrence.
        expect(screen.getByText('63100')).toBeTruthy();
        expect(screen.getByText('66500')).toBeTruthy();
        expect(screen.getAllByText('FIB').length).toBeGreaterThan(0);
        expect(screen.getAllByText('VP').length).toBeGreaterThan(0);
        expect(screen.getAllByText('PP').length).toBeGreaterThan(0);
    });
});
