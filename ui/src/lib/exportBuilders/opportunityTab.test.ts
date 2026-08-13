// Regression tests for the v7.0-audit Opportunity tab export.

import { describe, it, expect } from 'vitest';
import { buildOpportunityTabExport } from './opportunityTab';
import type { LayerHeaderSpec } from '../layerHeader';
import type { OpportunityMatrix, AnalysisMatrix, DecisionContext } from '../../types';

const headerSpec: LayerHeaderSpec = {
  layerNumber: 4,
  layerName: 'Opportunity',
  badge: { label: 'Breakout', color: '#f59e0b', background: 'rgba(245,158,11,0.08)', state: 'valid' },
  meta: [],
  status: 'live',
};

function makeOpportunity(): OpportunityMatrix {
  return {
    symbol: 'BTC-USDC',
    primary_opportunity: 'Breakout',
    opportunity_score: 60.12,
    setup_quality: 'Moderate',
    profiles: [
      {
        opportunity_type: 'Breakout',
        score: 60.12,
        preconditions_met: 2,
        preconditions_total: 2,
        notes: 'Breakout',
        direction_family: 'TrendRiding',
        long_entry_zone: { low: 63320, high: 63340 },
        long_target_zone: { low: 63681, high: 64380 },
        long_invalidation_level: 63327,
        long_expected_rr_internal: 0,
        long_geometry_consistent: false,
        short_entry_zone: null,
        short_target_zone: null,
        short_invalidation_level: null,
        short_expected_rr_internal: 0,
        short_geometry_consistent: false,
        trade_viability: 'Actionable',
      },
      {
        opportunity_type: 'NoClearOpportunity',
        score: 0,
        preconditions_met: 1,
        preconditions_total: 1,
        notes: 'NoClearOpportunity',
        direction_family: 'Neutral',
        trade_viability: 'NoClear',
      },
    ],
    forecast_confidence: 0.28,
    contributing_signals: [],
    invalidation_note: 'Close below 64384.6 invalidates the Breakout thesis.',
    entry_zone: { low: 63320, high: 63340 },
    target_zone: { low: 63681, high: 64380 },
    invalidation_level: 63327,
    long_entry_zone: { low: 63320, high: 63340 },
    long_target_zone: { low: 63681, high: 64380 },
    long_invalidation_level: 63327,
    long_expected_rr_internal: 0,
    short_entry_zone: { low: 64363, high: 64384 },
    short_target_zone: { low: 63264, high: 63310 },
    short_invalidation_level: 64384,
    short_expected_rr_internal: 8.04,
    time_horizon: 'INTRADAY',
    confluent_entry_levels: [
      { price: 63552.71, sources: ['FIBONACCI'], strength: 100 },
      { price: 64117.07, sources: ['VOLUME_PROFILE'], strength: 30 },
    ],
    confluent_target_levels: [
      { price: 63264.33, sources: ['ATR_FALLBACK'], strength: 35 },
    ],
    confluent_invalidation_levels: [],
    direction_family: 'TrendRiding',
    long_geometry_consistent: false,
    short_geometry_consistent: true,
  } as unknown as OpportunityMatrix;
}

function makeAnalysis(): AnalysisMatrix {
  return {
    bias: 'Bullish',
    confidence: 0.28,
    state_confidence: 0.28,
    market_regime: 'ACCUMULATION',
    market_quality: 'Average',
    market_phase: 'ACCUMULATION',
    timeframes_considered: 4,
    supporting_signals: [],
    contradicting_signals: [],
    trend_assessment: 'Weak',
    momentum_assessment: 'Neutral',
    structure_assessment: 'Neutral',
    volatility_assessment: 'Neutral',
    volume_assessment: 'Neutral',
    market_interpretation: '',
    rationale: '',
    opportunity_analysis: 'Breakout',
  } as unknown as AnalysisMatrix;
}

function makeDecisionContext(): DecisionContext {
  return {
    score: 0,
    bias: 'Neutral',
    score_confidence: 0,
    entry_danger: { score: 59, level: 'Moderate', state: 'Stable', confidence: 27, evidence: [] },
    expected_reward_risk_ratio: 0,
    trade_readiness: 'STAND_ASIDE',
    contributing_indicators: [],
    long_probability: 30,
    short_probability: 40,
    hold_probability: 30,
    net_bias_pct: -10,
  } as unknown as DecisionContext;
}

describe('buildOpportunityTabExport', () => {
  it('includes directional_bars and structured header chrome', () => {
    const p = JSON.parse(buildOpportunityTabExport({
      opportunity: makeOpportunity(),
      analysis: makeAnalysis(),
      decisionContext: makeDecisionContext(),
      symbol: 'BTC-USDC',
      markPrice: 63369,
      headerSpec,
    }));
    expect(p.header.layer_name).toBe('Opportunity');
    expect(p.directional_bars).not.toBeNull();
    expect(typeof p.directional_bars.bullish_pct).toBe('number');
    expect(p.directional_bars.sort).toBe('desc');
  });

  it('confluent sources are abbreviated (FIB, VP, ATR)', () => {
    const p = JSON.parse(buildOpportunityTabExport({
      opportunity: makeOpportunity(),
      analysis: makeAnalysis(),
      decisionContext: makeDecisionContext(),
      symbol: 'BTC-USDC',
      markPrice: 63369,
      headerSpec,
    }));
    expect(p.confluent_entry_levels[0].sources).toEqual(['FIB']);
    expect(p.confluent_entry_levels[1].sources).toEqual(['VP']);
    expect(p.confluent_target_levels[0].sources).toEqual(['ATR']);
  });

  it('no_clear_strip surfaces from the NoClearOpportunity profile', () => {
    const p = JSON.parse(buildOpportunityTabExport({
      opportunity: makeOpportunity(),
      analysis: makeAnalysis(),
      decisionContext: makeDecisionContext(),
      symbol: 'BTC-USDC',
      markPrice: 63369,
      headerSpec,
    }));
    expect(p.no_clear_strip).not.toBeNull();
    expect(p.no_clear_strip.badge).toBe('NO CLEAR OPPORTUNITY');
    expect(p.no_clear_strip.preconditions_met).toBe(1);
  });

  it('rr_internal uses available/value/reason triple', () => {
    const p = JSON.parse(buildOpportunityTabExport({
      opportunity: makeOpportunity(),
      analysis: makeAnalysis(),
      decisionContext: makeDecisionContext(),
      symbol: 'BTC-USDC',
      markPrice: 63369,
      headerSpec,
    }));
    expect(typeof p.rr_internal.expected_rr_available).toBe('boolean');
    expect('expected_rr_value' in p.rr_internal).toBe(true);
  });

  it('evaluated_setups carry viability and display-formatted types', () => {
    const p = JSON.parse(buildOpportunityTabExport({
      opportunity: makeOpportunity(),
      analysis: makeAnalysis(),
      decisionContext: makeDecisionContext(),
      symbol: 'BTC-USDC',
      markPrice: 63369,
      headerSpec,
    }));
    // The screen's Evaluated Setups list excludes the NoClearOpportunity
    // profile (it has its own placeholder strip) — the export mirrors that.
    expect(p.evaluated_setups).toHaveLength(1);
    expect(p.evaluated_setups[0].opportunity_type).toBe('Breakout');
    expect(p.evaluated_setups[0].viability).toBe('Actionable');
  });

  it('environment includes the display string for TFs considered', () => {
    const p = JSON.parse(buildOpportunityTabExport({
      opportunity: makeOpportunity(),
      analysis: makeAnalysis(),
      decisionContext: makeDecisionContext(),
      symbol: 'BTC-USDC',
      markPrice: 63369,
      headerSpec,
    }));
    expect(p.environment.timeframes_considered).toBe(4);
    expect(p.environment.timeframes_considered_display).toBe('4/4 TFs considered');
  });

  it('trade_setups carry badge_text mirroring screen badges', () => {
    const p = JSON.parse(buildOpportunityTabExport({
      opportunity: makeOpportunity(),
      analysis: makeAnalysis(),
      decisionContext: makeDecisionContext(),
      symbol: 'BTC-USDC',
      markPrice: 63369,
      headerSpec,
    }));
    // First setup is the top one with viability='Actionable' + topAction='STAND_ASIDE' (HOLD)
    // → `is_top=false` so badge text falls through to just 'ACTIONABLE' or 'GEOMETRY INVERTED'
    expect(p.trade_setups.length).toBeGreaterThan(0);
    const first = p.trade_setups[0];
    expect(['TOP · ACTIONABLE', 'ACTIONABLE', 'NEUTRAL · HOLD', 'GEOMETRY INVERTED', 'Actionable', 'DirectionalNeutral', 'GeometryInverted', 'NoClear'])
      .toContain(first.badge_text);
    expect(first.viability).toBeDefined();
  });

  it('evaluated_setups notes are raw wire strings (screen parity)', () => {
    const opp = {
      ...makeOpportunity(),
      profiles: [
        {
          opportunity_type: 'TrendContinuation',
          score: 78,
          preconditions_met: 3,
          preconditions_total: 3,
          notes: 'pullback_to_EMA20',
          direction_family: 'TrendRiding',
          trade_viability: 'Actionable',
        },
      ],
    } as unknown as OpportunityMatrix;
    const p = JSON.parse(buildOpportunityTabExport({
      opportunity: opp,
      analysis: makeAnalysis(),
      decisionContext: makeDecisionContext(),
      symbol: 'BTC-USDC',
      markPrice: 63369,
      headerSpec,
    }));
    expect(p.evaluated_setups[0].notes).toBe('pullback_to_EMA20');
  });

  it('directional_bars always emit — 0/0/100 when the matrix is absent', () => {
    const p = JSON.parse(buildOpportunityTabExport({
      opportunity: null,
      analysis: makeAnalysis(),
      decisionContext: makeDecisionContext(),
      symbol: 'BTC-USDC',
      markPrice: 63369,
      headerSpec,
    }));
    expect(p.directional_bars).toEqual({ bullish_pct: 0, bearish_pct: 0, hold_pct: 100, sort: 'desc' });
  });

  it('expected R:R of 0 with a non-HOLD top renders available:true value:0 ("0.00" on screen)', () => {
    // Bias Bullish + long rr 0 + SHORT-leaning decision → top != HOLD →
    // the screen shows "0.00" (never N/A).
    const p = JSON.parse(buildOpportunityTabExport({
      opportunity: makeOpportunity(),
      analysis: makeAnalysis(), // bias Bullish, long_expected_rr_internal = 0
      decisionContext: makeDecisionContext(),
      symbol: 'BTC-USDC',
      markPrice: 63369,
      headerSpec,
    }));
    expect(p.rr_internal.expected_rr_available).toBe(true);
    expect(p.rr_internal.expected_rr_value).toBe(0);
    expect(p.rr_internal.expected_rr_reason).toBeNull();
  });

  it('empty states render "—" placeholders (header class, horizon, market position)', () => {
    const p = JSON.parse(buildOpportunityTabExport({
      opportunity: null,
      analysis: null,
      decisionContext: makeDecisionContext(),
      symbol: 'BTC-USDC',
      markPrice: 63369,
      headerSpec,
    }));
    expect(p.header_block.opportunity_class).toBe('—');
    expect(p.rr_internal.time_horizon).toBe('—');
    expect(p.market_position.bias).toBe('—');
    expect(p.market_position.regime).toBe('—');
    expect(p.market_position.trend).toBe('—');
    expect(p.market_position.quality).toBe('—');
  });
});