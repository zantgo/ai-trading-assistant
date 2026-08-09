// Tests for the Opportunity tab builder.

import { describe, it, expect } from 'vitest';
import { buildOpportunityTabExport, type OpportunityPayload } from './opportunityTab';
import type { OpportunityMatrix, AnalysisMatrix, DecisionContext } from '../../types';

function makeOpportunityMatrix(overrides: Partial<OpportunityMatrix> = {}): OpportunityMatrix {
  return {
    symbol: 'BTC-USDT',
    primary_opportunity: 'TrendContinuation',
    opportunity_score: 78,
    setup_quality: 'STRONG',
    forecast_confidence: 0.85,
    time_horizon: 'INTRADAY',
    entry_zone: { low: 64000, high: 64500 },
    target_zone: { low: 66000, high: 67000 },
    invalidation_level: 63000,
    long_entry_zone: { low: 64000, high: 64500 },
    long_target_zone: { low: 66000, high: 67000 },
    long_invalidation_level: 63000,
    short_entry_zone: { low: 65500, high: 66000 },
    short_target_zone: { low: 63000, high: 64000 },
    short_invalidation_level: 66500,
    long_expected_rr_internal: 2.5,
    short_expected_rr_internal: 0,
    contributing_signals: ['RSI cross up', 'VWAP support'],
    profiles: [
      {
        opportunity_type: 'TrendContinuation',
        score: 78,
        preconditions_met: 4,
        preconditions_total: 5,
        notes: 'Trend alignment strong',
        direction_family: 'TrendRiding',
        long_entry_zone: { low: 64000, high: 64500 },
        long_target_zone: { low: 66000, high: 67000 },
        long_invalidation_level: 63000,
        short_entry_zone: null,
        short_target_zone: null,
        short_invalidation_level: null,
        long_expected_rr_internal: 2.5,
        short_expected_rr_internal: null,
        trade_viability: 'Actionable',
      },
      {
        opportunity_type: 'Breakout',
        score: 65,
        preconditions_met: 3,
        preconditions_total: 5,
        notes: 'Watching for breakout above 65k',
        direction_family: 'TrendRiding',
        long_entry_zone: { low: 65000, high: 65100 },
        long_target_zone: { low: 66000, high: 66500 },
        long_invalidation_level: 64800,
        short_entry_zone: null,
        short_target_zone: null,
        short_invalidation_level: null,
        long_expected_rr_internal: 1.8,
        short_expected_rr_internal: null,
        trade_viability: 'Actionable',
      },
      {
        opportunity_type: 'NoClearOpportunity',
        score: 20,
        preconditions_met: 0,
        preconditions_total: 5,
        notes: '',
        direction_family: 'Neutral',
        long_entry_zone: null,
        long_target_zone: null,
        long_invalidation_level: null,
        short_entry_zone: null,
        short_target_zone: null,
        short_invalidation_level: null,
        long_expected_rr_internal: null,
        short_expected_rr_internal: null,
        trade_viability: 'NoClear',
      },
    ],
    confluent_entry_levels: [
      { price: 64000, confluence_count: 3, sources: ['FIBONACCI', 'VOLUME_PROFILE'], strength: 85 },
      { price: 63500, confluence_count: 2, sources: ['PIVOT_POINTS'], strength: 60 },
    ],
    confluent_target_levels: [
      { price: 67000, confluence_count: 2, sources: ['FIBONACCI'], strength: 70 },
    ],
    confluent_invalidation_levels: [],
    invalidation_note: 'Below 63000 invalidates the setup',
    ...overrides,
  };
}

function makeAnalysis(overrides: Partial<AnalysisMatrix> = {}): AnalysisMatrix {
  return {
    symbol: 'BTC-USDT',
    bias: 'Bullish',
    confidence: 0.7,
    state_confidence: 0.7,
    market_regime: 'TRENDING_BULL',
    trend_assessment: 'Healthy',
    momentum_assessment: 'Increasing',
    structure_assessment: 'Strong',
    volatility_assessment: 'Normal',
    volume_assessment: 'Strong',
    opportunity_analysis: 'TrendContinuation',
    market_quality: 'Good',
    market_quality_score: 75,
    market_phase: 'MARKUP',
    market_interpretation: 'Bullish trend healthy',
    rationale: 'L3 supporting signals aligned',
    supporting_signals: ['RSI cross up'],
    contradicting_signals: [],
    timeframes_considered: 4,
    ...overrides,
  };
}

function makeDecisionContext(overrides: Partial<DecisionContext> = {}): DecisionContext {
  return {
    score: 50,
    bias: 'Bullish',
    confidence: 0.7,
    score_confidence: 0.7,
    entry_danger: { score: 30, level: 'Low', state: 'Stable', confidence: 0.7, evidence: [] },
    expected_reward_risk_ratio: 2.5,
    trade_readiness: 'READY',
    contributing_indicators: ['RSI', 'MACD'],
    ...overrides,
  };
}

describe('buildOpportunityTabExport', () => {
  it('produces a valid payload with all expected top-level fields', () => {
    const json = buildOpportunityTabExport({
      opportunity: makeOpportunityMatrix(),
      analysis: makeAnalysis(),
      decisionContext: makeDecisionContext(),
      symbol: 'BTC-USDT',
      tfSecs: 60,
    });
    const p = JSON.parse(json) as OpportunityPayload;
    expect(p.source_tab).toBe('opportunity');
    expect(p.meta.symbol).toBe('BTC-USDT');
    expect(p.header).toBeDefined();
    expect(p.trade_setups).toBeDefined();
    expect(p.rr_internal).toBeDefined();
    expect(p.evaluated_setups).toBeDefined();
    expect(p.confluent_entry_levels).toBeDefined();
    expect(p.confluent_target_levels).toBeDefined();
    expect(p.market_position).toBeDefined();
    expect(p.environment).toBeDefined();
  });

  it('header captures setup_score and quality bucketing', () => {
    const p = JSON.parse(buildOpportunityTabExport({
      opportunity: makeOpportunityMatrix(),
      analysis: makeAnalysis(),
      decisionContext: makeDecisionContext(),
      symbol: 'BTC-USDT',
    })) as OpportunityPayload;
    expect(p.header.setup_score).toBe(78);
    expect(p.header.setup_quality).toBe('STRONG');
    expect(p.header.opportunity_class).toBe('TrendContinuation');
  });

  it('trade_setups only includes qualifying profiles with resolved side', () => {
    const p = JSON.parse(buildOpportunityTabExport({
      opportunity: makeOpportunityMatrix(),
      analysis: makeAnalysis(),
      decisionContext: makeDecisionContext(),
      symbol: 'BTC-USDT',
    })) as OpportunityPayload;
    expect(p.trade_setups.length).toBe(2); // NoClearOpportunity excluded
    expect(p.trade_setups[0].is_top).toBe(true);
    expect(p.trade_setups[0].opportunity_type).toBe('TrendContinuation');
    expect(p.trade_setups[0].side).toBe('LONG');
  });

  it('trade_setups captures entry/target/SL/R:R/geometry_consistent', () => {
    const p = JSON.parse(buildOpportunityTabExport({
      opportunity: makeOpportunityMatrix(),
      analysis: makeAnalysis(),
      decisionContext: makeDecisionContext(),
      symbol: 'BTC-USDT',
    })) as OpportunityPayload;
    const top = p.trade_setups[0];
    expect(top.entry_zone).toEqual({ low: 64000, high: 64500 });
    expect(top.entry_mid).toBeCloseTo(64250);
    expect(top.score).toBe(78);
    expect(top.preconditions_met).toBe(4);
    expect(top.preconditions_total).toBe(5);
  });

  it('evaluated_setups contains the full profile array (not sliced)', () => {
    const p = JSON.parse(buildOpportunityTabExport({
      opportunity: makeOpportunityMatrix(),
      analysis: makeAnalysis(),
      decisionContext: makeDecisionContext(),
      symbol: 'BTC-USDT',
    })) as OpportunityPayload;
    expect(p.evaluated_setups.length).toBe(3);
    expect(p.evaluated_setups[2].opportunity_type).toBe('NoClearOpportunity');
  });

  it('confluent_entry_levels captures full array (not sliced to 4)', () => {
    const p = JSON.parse(buildOpportunityTabExport({
      opportunity: makeOpportunityMatrix(),
      analysis: makeAnalysis(),
      decisionContext: makeDecisionContext(),
      symbol: 'BTC-USDT',
    })) as OpportunityPayload;
    expect(p.confluent_entry_levels.length).toBe(2);
    expect(p.confluent_entry_levels[0].sources).toContain('FIBONACCI');
  });

  it('market_position reads from analysis', () => {
    const p = JSON.parse(buildOpportunityTabExport({
      opportunity: makeOpportunityMatrix(),
      analysis: makeAnalysis(),
      decisionContext: makeDecisionContext(),
      symbol: 'BTC-USDT',
    })) as OpportunityPayload;
    expect(p.market_position.bias).toBe('Bullish');
    expect(p.market_position.regime).toBe('TRENDING_BULL');
    expect(p.market_position.trend).toBe('Healthy');
    expect(p.market_position.quality).toBe('Good');
  });

  it('environment reads from analysis', () => {
    const p = JSON.parse(buildOpportunityTabExport({
      opportunity: makeOpportunityMatrix(),
      analysis: makeAnalysis(),
      decisionContext: makeDecisionContext(),
      symbol: 'BTC-USDT',
    })) as OpportunityPayload;
    expect(p.environment.timeframes_considered).toBe(4);
    expect(p.environment.confidence_pct).toBe(70);
  });

  it('rr_internal captures expected_rr and time_horizon', () => {
    const p = JSON.parse(buildOpportunityTabExport({
      opportunity: makeOpportunityMatrix(),
      analysis: makeAnalysis(),
      decisionContext: makeDecisionContext(),
      symbol: 'BTC-USDT',
    })) as OpportunityPayload;
    expect(p.rr_internal.expected_rr).toBe(2.5);
    expect(p.rr_internal.time_horizon).toBe('INTRADAY');
  });

  it('produces a valid payload when opportunity is null', () => {
    const p = JSON.parse(buildOpportunityTabExport({
      opportunity: null,
      analysis: makeAnalysis(),
      decisionContext: makeDecisionContext(),
      symbol: 'BTC-USDT',
    })) as OpportunityPayload;
    expect(p.trade_setups).toEqual([]);
    expect(p.evaluated_setups).toEqual([]);
    expect(p.header.setup_score).toBe(0);
  });

  it('header lean reflects expected action', () => {
    const p = JSON.parse(buildOpportunityTabExport({
      opportunity: makeOpportunityMatrix(),
      analysis: makeAnalysis(),
      decisionContext: makeDecisionContext(),
      symbol: 'BTC-USDT',
    })) as OpportunityPayload;
    // The bias is Bullish and the top setup is LONG → bullish lean
    expect(p.header.lean).toBe('bullish_setups_dominate');
  });
});
