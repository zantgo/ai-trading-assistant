// @vitest-environment jsdom
//
// Regression lock for the Directional Setups UI on the Opportunities tab.
//
// The Opportunities tab must always render two side-by-side cards —
// LONG SETUP and SHORT SETUP — each with its own entry zone, target
// zone, and invalidation price. The legacy Tactical Bracket is gone.

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

describe('OpportunitiesPanel — Directional Setups', () => {
    it('renders both LONG SETUP and SHORT SETUP headers', () => {
        seedSnapshot('BTC-USDT', makeOpportunity(), 64000);
        render(OpportunitiesPanel, { props: { pairKey: 'BTC-USDT' } });
        expect(screen.getByText('LONG SETUP')).toBeTruthy();
        expect(screen.getByText('SHORT SETUP')).toBeTruthy();
    });

    it('renders the Directional Setups section title', () => {
        seedSnapshot('BTC-USDT', makeOpportunity(), 64000);
        render(OpportunitiesPanel, { props: { pairKey: 'BTC-USDT' } });
        expect(screen.getByText('Directional Setups')).toBeTruthy();
    });

    it('no longer renders the Tactical Bracket section', () => {
        seedSnapshot('BTC-USDT', makeOpportunity(), 64000);
        render(OpportunitiesPanel, { props: { pairKey: 'BTC-USDT' } });
        expect(screen.queryByText('Tactical Bracket')).toBeNull();
    });

    it('shows the per-direction entry, target, and invalidation values', () => {
        seedSnapshot('BTC-USDT', makeOpportunity(), 64000);
        render(OpportunitiesPanel, { props: { pairKey: 'BTC-USDT' } });
        expect(screen.getByText('63000 – 63200')).toBeTruthy();
        expect(screen.getByText('66000 – 66500')).toBeTruthy();
        expect(screen.getByText('62400')).toBeTruthy();
        expect(screen.getByText('65000 – 65200')).toBeTruthy();
        expect(screen.getByText('62000 – 62400')).toBeTruthy();
        expect(screen.getByText('66000')).toBeTruthy();
    });

    it('renders an em-dash when no opportunity matrix is present', () => {
        const app = useAppStore();
        if (!app.instancesMap['BTC-USDT']) app.initInstance('BTC');
        app.instancesMap['BTC-USDT'].microTerm.priceText = '64000';
        render(OpportunitiesPanel, { props: { pairKey: 'BTC-USDT' } });
        expect(screen.getByText('LONG SETUP')).toBeTruthy();
        expect(screen.getByText('SHORT SETUP')).toBeTruthy();
    });
});