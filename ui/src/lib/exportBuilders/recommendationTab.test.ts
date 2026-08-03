// Tests for the Recommendation tab builder.

import { describe, it, expect } from 'vitest';
import { buildRecommendationTabExport, type RecommendationPayload } from './recommendationTab';
import type { AdvisoryMatrix, AnalysisMatrix, DecisionContext, OpportunityMatrix, OpportunityProfile, RiskDimension } from '../../types';

function makeRiskDim(overrides: Partial<RiskDimension> = {}): RiskDimension {
  return {
    score: 50,
    level: 'Moderate',
    state: 'Stable',
    confidence: 0.7,
    evidence: [],
    ...overrides,
  };
}

function makeAdvisory(overrides: Partial<AdvisoryMatrix> = {}): AdvisoryMatrix {
  return {
    symbol: 'BTC-USDT',
    directional_guidance: 'Long',
    market_stance: 'Constructive',
    opportunity_classification: 'TrendContinuation',
    strategy_environment: 'TrendFollowing',
    entry_guidance: 'Pullback',
    exit_guidance: 'TrendWeakening',
    protection_strategy: 'StructureBased',
    target_strategy: 'RRBased',
    confidence_assessment: 78,
    stop_loss_distance_pct: 0.015,
    cascade_risk_score: 30,
    environment_favorability: makeRiskDim({ score: 30, level: 'Low' }),
    final_recommendation: 'Long bias — structure-based entry with R:R 2.5',
    ...overrides,
  };
}

function makeDecisionContext(overrides: Partial<DecisionContext> = {}): DecisionContext {
  return {
    score: 60,
    bias: 'BULLISH',
    confidence: 0.75,
    score_confidence: 0.75,
    entry_danger: makeRiskDim({ score: 30, level: 'Low', state: 'Stable' }),
    expected_reward_risk_ratio: 2.5,
    trade_readiness: 'READY',
    contributing_indicators: ['RSI', 'MACD', 'VWAP'],
    ...overrides,
  };
}

function makeOpportunity(): OpportunityMatrix {
  const profile: OpportunityProfile = {
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
  };
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
    expected_rr_internal: 2.5,
    contributing_signals: ['RSI cross up'],
    profiles: [profile],
    confluent_entry_levels: [],
    confluent_target_levels: [],
    confluent_invalidation_levels: [],
    invalidation_note: 'Below 63000 invalidates the setup',
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
    rationale: 'Multi-timeframe alignment supports the bullish bias',
    supporting_signals: ['RSI cross up'],
    contradicting_signals: [],
    timeframes_considered: 4,
    ...overrides,
  };
}

describe('buildRecommendationTabExport', () => {
  it('produces a valid payload with all expected top-level fields', () => {
    const json = buildRecommendationTabExport({
      advisory: makeAdvisory(),
      decisionContext: makeDecisionContext(),
      opportunity: makeOpportunity(),
      analysis: makeAnalysis(),
      symbol: 'BTC-USDT',
      tfSecs: 60,
    });
    const p = JSON.parse(json) as RecommendationPayload;
    expect(p.source_tab).toBe('recommendation');
    expect(p.meta.symbol).toBe('BTC-USDT');
    expect(p.environment).toBeDefined();
    expect(p.verdict).toBeDefined();
    expect(p.runner_ups).toBeDefined();
    expect(p.top_setup).toBeDefined();
    expect(p.safety_flags).toBeDefined();
    expect(p.why).toBeDefined();
    expect(p.price_levels).toBeDefined();
    expect(p.strategy).toBeDefined();
    expect(p.final_verdict).toBeDefined();
  });

  it('environment captures directional_guidance, market_stance, strategy_environment', () => {
    const p = JSON.parse(buildRecommendationTabExport({
      advisory: makeAdvisory(),
      decisionContext: makeDecisionContext(),
      opportunity: makeOpportunity(),
      analysis: makeAnalysis(),
      symbol: 'BTC-USDT',
    })) as RecommendationPayload;
    expect(p.environment.directional_guidance).toBe('Long');
    expect(p.environment.market_stance).toBe('Constructive');
    expect(p.environment.strategy_environment).toBe('TrendFollowing');
    expect(p.environment.opportunity_classification).toBe('TrendContinuation');
    expect(p.environment.confidence_pct).toBe(78);
  });

  it('environment captures entry_danger as a structured shape', () => {
    const p = JSON.parse(buildRecommendationTabExport({
      advisory: makeAdvisory(),
      decisionContext: makeDecisionContext(),
      opportunity: makeOpportunity(),
      analysis: makeAnalysis(),
      symbol: 'BTC-USDT',
    })) as RecommendationPayload;
    expect(p.environment.entry_danger.score).toBe(30);
    expect(p.environment.entry_danger.level).toBe('Low');
    expect(p.environment.entry_danger.state).toBe('Stable');
  });

  it('verdict captures all 3 probabilities + top + headline', () => {
    const p = JSON.parse(buildRecommendationTabExport({
      advisory: makeAdvisory(),
      decisionContext: makeDecisionContext(),
      opportunity: makeOpportunity(),
      analysis: makeAnalysis(),
      symbol: 'BTC-USDT',
    })) as RecommendationPayload;
    expect(p.verdict.top).toBeDefined();
    expect(['LONG', 'SHORT', 'HOLD']).toContain(p.verdict.top);
    expect(p.verdict.long_probability).toBeDefined();
    expect(p.verdict.short_probability).toBeDefined();
    expect(p.verdict.hold_probability).toBeDefined();
  });

  it('runner_ups excludes the winner', () => {
    const p = JSON.parse(buildRecommendationTabExport({
      advisory: makeAdvisory(),
      decisionContext: makeDecisionContext(),
      opportunity: makeOpportunity(),
      analysis: makeAnalysis(),
      symbol: 'BTC-USDT',
    })) as RecommendationPayload;
    expect(p.runner_ups.length).toBe(2);
    expect(p.runner_ups.every((r) => r.action !== p.verdict.top)).toBe(true);
  });

  it('top_setup captures entry/target/SL/R:R', () => {
    const p = JSON.parse(buildRecommendationTabExport({
      advisory: makeAdvisory(),
      decisionContext: makeDecisionContext(),
      opportunity: makeOpportunity(),
      analysis: makeAnalysis(),
      symbol: 'BTC-USDT',
    })) as RecommendationPayload;
    expect(p.top_setup).not.toBeNull();
    expect(p.top_setup?.opportunity_type).toBe('TrendContinuation');
    expect(p.top_setup?.entry_zone).toEqual({ low: 64000, high: 64500 });
    expect(p.top_setup?.target_zone).toEqual({ low: 66000, high: 67000 });
    expect(p.top_setup?.invalidation).toBe(63000);
  });

  it('safety_flags captures all 5 fields', () => {
    const p = JSON.parse(buildRecommendationTabExport({
      advisory: makeAdvisory(),
      decisionContext: makeDecisionContext(),
      opportunity: makeOpportunity(),
      analysis: makeAnalysis(),
      symbol: 'BTC-USDT',
    })) as RecommendationPayload;
    expect(p.safety_flags.internal_rr).toBe(2.5);
    expect(p.safety_flags.risk_adj_rr).toBe(2.5);
    expect(p.safety_flags.stop_loss_pct).toBe(0.015);
    expect(p.safety_flags.confidence_pct).toBe(78);
  });

  it('why captures top-3 rationale bullets', () => {
    const p = JSON.parse(buildRecommendationTabExport({
      advisory: makeAdvisory(),
      decisionContext: makeDecisionContext(),
      opportunity: makeOpportunity(),
      analysis: makeAnalysis(),
      symbol: 'BTC-USDT',
    })) as RecommendationPayload;
    expect(p.why.length).toBeLessThanOrEqual(3);
    expect(p.why.length).toBeGreaterThan(0);
  });

  it('price_levels reflects verdict side', () => {
    const p = JSON.parse(buildRecommendationTabExport({
      advisory: makeAdvisory(),
      decisionContext: makeDecisionContext(),
      opportunity: makeOpportunity(),
      analysis: makeAnalysis(),
      symbol: 'BTC-USDT',
    })) as RecommendationPayload;
    const side = p.verdict.top;
    if (side === 'LONG' || side === 'SHORT') {
      expect(p.price_levels.side).toBe(side === 'LONG' ? 'long' : 'short');
      expect(p.price_levels.scenarios).toBeNull();
    } else {
      expect(p.price_levels.side).toBe('hold');
      expect(p.price_levels.scenarios).not.toBeNull();
      expect(p.price_levels.scenarios?.long).toBeDefined();
      expect(p.price_levels.scenarios?.short).toBeDefined();
    }
  });

  it('strategy captures all 4 advisory fields', () => {
    const p = JSON.parse(buildRecommendationTabExport({
      advisory: makeAdvisory(),
      decisionContext: makeDecisionContext(),
      opportunity: makeOpportunity(),
      analysis: makeAnalysis(),
      symbol: 'BTC-USDT',
    })) as RecommendationPayload;
    expect(p.strategy.entry).toBe('Pullback');
    expect(p.strategy.exit).toBe('TrendWeakening');
    expect(p.strategy.protection).toBe('StructureBased');
    expect(p.strategy.target).toBe('RRBased');
  });

  it('final_verdict captures the advisory text', () => {
    const p = JSON.parse(buildRecommendationTabExport({
      advisory: makeAdvisory(),
      decisionContext: makeDecisionContext(),
      opportunity: makeOpportunity(),
      analysis: makeAnalysis(),
      symbol: 'BTC-USDT',
    })) as RecommendationPayload;
    expect(p.final_verdict).toBe('Long bias — structure-based entry with R:R 2.5');
  });

  it('produces a valid payload when advisory is null', () => {
    const p = JSON.parse(buildRecommendationTabExport({
      advisory: null,
      decisionContext: makeDecisionContext(),
      opportunity: makeOpportunity(),
      analysis: makeAnalysis(),
      symbol: 'BTC-USDT',
    })) as RecommendationPayload;
    expect(p.environment.directional_guidance).toBe('—');
    expect(p.final_verdict).toBe('');
  });

  it('top_setup is null when no qualifying profile exists', () => {
    const noQualifying: OpportunityMatrix = {
      ...makeOpportunity(),
      profiles: [],
    };
    const p = JSON.parse(buildRecommendationTabExport({
      advisory: makeAdvisory(),
      decisionContext: makeDecisionContext(),
      opportunity: noQualifying,
      analysis: makeAnalysis(),
      symbol: 'BTC-USDT',
    })) as RecommendationPayload;
    expect(p.top_setup).toBeNull();
  });
});
