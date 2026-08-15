// Regression tests for the v7.0-audit Recommendation tab export.

import { describe, it, expect } from 'vitest';
import { buildRecommendationTabExport } from './recommendationTab';
import type { LayerHeaderSpec } from '../layerHeader';
import type { DecisionContext, OpportunityMatrix, AnalysisMatrix, AdvisoryMatrix } from '../../types';

const headerSpec: LayerHeaderSpec = {
  layerNumber: 6,
  layerName: 'Recommendation',
  badge: { label: 'STAND ASIDE', color: '#f59e0b', background: 'rgba(245,158,11,0.08)', state: 'valid' },
  meta: [],
  status: 'live',
};

function makeDecisionContext(overrides: Partial<DecisionContext> = {}): DecisionContext {
  return {
    score: 0,
    bias: 'Neutral',
    confidence: 0.1,
    score_confidence: 0,
    entry_danger: { score: 59.25, level: 'Moderate', state: 'Stable', confidence: 27.66, evidence: [] },
    expected_reward_risk_ratio: 0,
    trade_readiness: 'STAND_ASIDE',
    contributing_indicators: [],
    long_probability: 12,
    short_probability: 2,
    hold_probability: 86,
    net_bias_pct: 10,
    ...overrides,
  };
}

function makeOpportunity(): OpportunityMatrix {
  return {
    symbol: 'SOL-USDC',
    primary_opportunity: 'NoClearOpportunity',
    opportunity_score: 41.3,
    setup_quality: 'Moderate',
    profiles: [],
    forecast_confidence: 0.1,
    contributing_signals: [],
    invalidation_note: 'Close below 75.4 invalidates the MeanReversion thesis.',
    entry_zone: { low: 75.509, high: 75.523 },
    target_zone: { low: 75.564, high: 75.591 },
    invalidation_level: 75.4957,
    long_entry_zone: { low: 75.509, high: 75.523 },
    long_target_zone: { low: 75.564, high: 75.591 },
    long_invalidation_level: 75.4957,
    long_expected_rr_internal: 0,
    short_entry_zone: { low: 75.523, high: 75.537 },
    short_target_zone: { low: 75.455, high: 75.482 },
    short_invalidation_level: 75.5503,
    short_expected_rr_internal: 0,
    time_horizon: 'SWING',
    confluent_entry_levels: [],
    confluent_target_levels: [],
    confluent_invalidation_levels: [],
    direction_family: 'Neutral',
    long_geometry_consistent: true,
    short_geometry_consistent: true,
  } as unknown as OpportunityMatrix;
}

function makeAnalysis(): AnalysisMatrix {
  return {
    bias: 'Neutral',
    confidence: 0.13,
    state_confidence: 0.1,
    market_regime: 'RANGING',
    market_quality: 'Average',
    market_phase: 'ACCUMULATION',
    timeframes_considered: 4,
    supporting_signals: [],
    contradicting_signals: [],
    trend_assessment: 'Neutral',
    momentum_assessment: 'Neutral',
    structure_assessment: 'Neutral',
    volatility_assessment: 'Neutral',
    volume_assessment: 'Neutral',
    market_interpretation: '',
    rationale: '',
  } as unknown as AnalysisMatrix;
}

function makeAdvisory(overrides: Partial<AdvisoryMatrix> = {}): AdvisoryMatrix {
  return {
    directional_guidance: 'Neutral',
    market_stance: 'Cautious',
    strategy_environment: 'LowActivity',
    opportunity_classification: 'Pullback',
    confidence_assessment: 13.15,
    entry_guidance: 'Breakout',
    exit_guidance: 'StructureBreakdown',
    protection_strategy: 'ATRBased',
    target_strategy: 'TrailingMethod',
    final_recommendation: 'Neutral — no directional edge.',
    ...overrides,
  } as unknown as AdvisoryMatrix;
}

describe('buildRecommendationTabExport', () => {
  it('has the mandatory meta identity fields and no filter_state', () => {
    const p = JSON.parse(buildRecommendationTabExport({
      advisory: makeAdvisory(),
      decisionContext: makeDecisionContext(),
      opportunity: makeOpportunity(),
      analysis: makeAnalysis(),
      symbol: 'SOL-USDC',
      markPrice: 75.5465,
      headerSpec,
    }));
    expect(p.source_tab).toBe('recommendation');
    expect(p.meta.datetime_utc).toBeTruthy();
    expect(p.meta.exchange).toBe('Hyperliquid');
    expect(p.meta.pair).toBe('SOL-USDC');
    expect(p.meta.timeframe_secs).toBe(0);
    expect(p.meta.current_price).toBeCloseTo(75.55, 1);
    expect(p.meta.price_change_direction).toBe('unknown');
    expect('filter_state' in p.meta).toBe(false);
  });

  it('emits the gauge block with net_bias_pct as a raw number', () => {
    const p = JSON.parse(buildRecommendationTabExport({
      advisory: makeAdvisory(),
      decisionContext: makeDecisionContext({ long_probability: 60, short_probability: 10, hold_probability: 30, net_bias_pct: 50 }),
      opportunity: makeOpportunity(),
      analysis: makeAnalysis(),
      symbol: 'SOL-USDC',
      markPrice: 75.55,
      headerSpec,
    }));
    expect(p.gauge.net_bias_pct).toBe(50);
    expect(p.gauge.bias_direction).toBe('LONG');
    expect(p.gauge.long_pct).toBe(60);
    expect(p.gauge.short_pct).toBe(10);
    expect(p.gauge.hold_pct).toBe(30);
    // No mixed-string net_label — raw numbers only
    expect('net_label' in p.gauge).toBe(false);
  });

  it('renders rr as {available, value, reason} not null', () => {
    const p = JSON.parse(buildRecommendationTabExport({
      advisory: makeAdvisory(),
      decisionContext: makeDecisionContext(),
      opportunity: makeOpportunity(),
      analysis: makeAnalysis(),
      symbol: 'SOL-USDC',
      markPrice: 75.55,
      headerSpec,
    }));
    expect(p.safety_flags.rr_available).toBe(false);
    expect(p.safety_flags.rr_value).toBeNull();
    expect(p.safety_flags.rr_reason).toBe('no_directional_bias');
  });

  it('entry_danger is split into score + level', () => {
    const p = JSON.parse(buildRecommendationTabExport({
      advisory: makeAdvisory(),
      decisionContext: makeDecisionContext(),
      opportunity: makeOpportunity(),
      analysis: makeAnalysis(),
      symbol: 'SOL-USDC',
      markPrice: 75.55,
      headerSpec,
    }));
    expect(p.environment.entry_danger_score).toBeCloseTo(59.25, 1);
    expect(p.environment.entry_danger_level).toBe('MODERATE');
    expect(p.safety_flags.entry_danger_score).toBeCloseTo(59.25, 1);
    expect(p.safety_flags.entry_danger_level).toBe('MODERATE');
  });

  it('no_clear_card surfaces when the primary opportunity is NoClearOpportunity', () => {
    const p = JSON.parse(buildRecommendationTabExport({
      advisory: makeAdvisory(),
      decisionContext: makeDecisionContext(),
      opportunity: makeOpportunity(),
      analysis: makeAnalysis(),
      symbol: 'SOL-USDC',
      markPrice: 75.55,
      headerSpec,
    }));
    expect(p.no_clear_card).not.toBeNull();
    expect(p.no_clear_card.title).toBe('No Clear Setup');
    expect(p.why_note).toContain('No directional edge');
  });

  it('strategy fields are display-formatted (no raw enums)', () => {
    const p = JSON.parse(buildRecommendationTabExport({
      advisory: makeAdvisory(),
      // A directional verdict is required for the playbook values to
      // render — under HOLD/STAND_ASIDE they are "—" (FIX-O5 v6.10.16).
      decisionContext: makeDecisionContext({
        trade_readiness: 'FORMING',
        long_probability: 60,
        short_probability: 10,
        hold_probability: 30,
        net_bias_pct: 50,
      }),
      opportunity: makeOpportunity(),
      analysis: makeAnalysis(),
      symbol: 'SOL-USDC',
      markPrice: 75.55,
      headerSpec,
    }));
    expect(p.strategy.protection).toBe('ATR-Based');
    expect(p.strategy.entry).toBe('Breakout');
    expect(p.strategy.hold_caption).toBeNull();
  });

  it('FIX-O5: strategy values are "—" under a HOLD verdict (never "Entry: Immediate")', () => {
    const p = JSON.parse(buildRecommendationTabExport({
      advisory: makeAdvisory(),
      decisionContext: makeDecisionContext(), // hold-dominant (12/2/86) → HOLD
      opportunity: makeOpportunity(),
      analysis: makeAnalysis(),
      symbol: 'SOL-USDC',
      markPrice: 75.55,
      headerSpec,
    }));
    expect(p.strategy.entry).toBe('—');
    expect(p.strategy.exit).toBe('—');
    expect(p.strategy.protection).toBe('—');
    expect(p.strategy.target).toBe('—');
    expect(p.strategy.hold_caption).toContain('no active directional call');
    // The advisory text still survives as environment guidance.
    expect(p.final_verdict_guidance).toContain('Environment guidance:');
  });

  it('top_setup carries badge_text mirroring screen badge', () => {
    const oppWithProfile: OpportunityMatrix = {
      ...makeOpportunity(),
      primary_opportunity: 'Breakout',
      profiles: [
        {
          opportunity_type: 'Breakout',
          score: 60,
          preconditions_met: 2,
          preconditions_total: 2,
          notes: 'Breakout',
          direction_family: 'TrendRiding',
          long_entry_zone: { low: 75.0, high: 75.5 },
          long_target_zone: { low: 76.0, high: 77.0 },
          long_invalidation_level: 74.0,
          long_expected_rr_internal: 2.5,
          long_geometry_consistent: true,
          short_entry_zone: null,
          short_target_zone: null,
          short_invalidation_level: null,
          short_expected_rr_internal: 0,
          short_geometry_consistent: false,
          trade_viability: 'Actionable',
        } as any,
      ],
    };
    const analysisBullish: AnalysisMatrix = { ...makeAnalysis(), bias: 'Bullish' };
    const dc: DecisionContext = { ...makeDecisionContext(), bias: 'Bullish' };
    const p = JSON.parse(buildRecommendationTabExport({
      advisory: makeAdvisory(),
      decisionContext: dc,
      opportunity: oppWithProfile,
      analysis: analysisBullish,
      symbol: 'SOL-USDC',
      markPrice: 75.55,
      headerSpec,
    }));
    expect(p.top_setup).not.toBeNull();
    // Either a real badge text (one of the 4 tokens) or empty string
    // when the screen shows nothing (Actionable + HOLD).
    expect(['', 'ACTIONABLE', 'HOLD · NO DIRECTIONAL EDGE', 'GEOMETRY INVERTED', 'NO CLEAR SETUP'])
      .toContain(p.top_setup!.badge_text);
    expect(p.top_setup!.viability).toBeDefined();
  });

  it('no qualifying setup → the aggregated bracket is published (v6.10.17 A3)', () => {
    // v6.10.17: with zero qualifying profiles the top setup block now
    // carries the aggregated bracket on the bias side (marked NoClear,
    // informational) — never a bare null. The "no qualifying setup yet"
    // caption only appears when the opportunity matrix is absent.
    const p = JSON.parse(buildRecommendationTabExport({
      advisory: makeAdvisory(),
      decisionContext: makeDecisionContext(),
      opportunity: makeOpportunity(), // NoClearOpportunity, no profiles
      analysis: makeAnalysis(),
      symbol: 'SOL-USDC',
      markPrice: 75.55,
      headerSpec,
    }));
    expect(p.top_setup).not.toBeNull();
    expect(p.top_setup!.viability).toBe('NoClear');
    expect(p.top_setup!.badge_text).toBe('NO CLEAR SETUP');
    expect(p.top_setup_empty_text).toBeNull();
    // The No Clear explanation card coexists with the informational bracket.
    expect(p.no_clear_card).not.toBeNull();
    expect(p.no_clear_card!.title).toBe('No Clear Setup');
  });

  it('strategy fields render "—" when the advisory is absent (screen parity)', () => {
    const p = JSON.parse(buildRecommendationTabExport({
      advisory: null,
      decisionContext: makeDecisionContext(),
      opportunity: null,
      analysis: null,
      symbol: 'SOL-USDC',
      markPrice: 75.55,
      headerSpec,
    }));
    expect(p.strategy.entry).toBe('—');
    expect(p.strategy.exit).toBe('—');
    expect(p.strategy.protection).toBe('—');
    expect(p.strategy.target).toBe('—');
  });

  it('R6: final_verdict is verdict-consistent under HOLD (advisory demoted to guidance)', () => {
    const p = JSON.parse(buildRecommendationTabExport({
      advisory: makeAdvisory({ final_recommendation: 'Long bias: BULLISH bias with 34% confidence. Entry: immediate.' }),
      // Hold-dominant (12/2/86) → HOLD; readiness FORMING (NOT STAND_ASIDE)
      // so the headline state is HOLD, not STAND ASIDE.
      decisionContext: makeDecisionContext({ trade_readiness: 'FORMING' }),
      opportunity: makeOpportunity(),
      analysis: makeAnalysis(),
      symbol: 'SOL-USDC',
      markPrice: 75.55,
      headerSpec,
    }));
    expect(p.verdict.top).toBe('HOLD');
    // The final verdict is the VERDICT, never the advisory's directional
    // sentence under a HOLD badge.
    expect(p.final_verdict).toContain('HOLD — no directional call');
    expect(p.final_verdict).not.toContain('immediate');
    // v6.10.19 (T5): the advisory text survives as VERDICT-AWARE
    // environment guidance — no "Entry: immediate" execution instruction
    // under a HOLD, and the claim reads "no actionable directional edge".
    expect(p.final_verdict_guidance).toContain('Environment guidance:');
    expect(p.final_verdict_guidance).toContain('no actionable directional edge');
    expect(p.final_verdict_guidance).not.toContain('immediate');
  });

  it('R6 (v6.10.17): final_verdict carries the graded sentence under a directional verdict', () => {
    const p = JSON.parse(buildRecommendationTabExport({
      advisory: makeAdvisory({ final_recommendation: 'Strong long bias: BULLISH bias with 72% confidence.' }),
      decisionContext: makeDecisionContext({ trade_readiness: 'FORMING', long_probability: 60, short_probability: 10, hold_probability: 30, net_bias_pct: 50 }),
      opportunity: makeOpportunity(),
      analysis: makeAnalysis(),
      symbol: 'SOL-USDC',
      markPrice: 75.55,
      headerSpec,
    }));
    expect(p.verdict.top).toBe('LONG');
    // v6.10.17: the verdict sentence is the graded call, never the raw
    // advisory sentence; the advisory text survives as environment guidance.
    expect(p.final_verdict).toContain('LONG lean 60% — awaiting confirmation (readiness: FORMING)');
    expect(p.final_verdict).not.toContain('Strong long bias');
    expect(p.final_verdict_guidance).toContain('Environment guidance:');
    expect(p.strategy.hold_caption).toBeNull();
  });

  it('FIX-4 (v6.10.17): directional verdict gated by STAND ASIDE reports lean + gate', () => {
    const p = JSON.parse(buildRecommendationTabExport({
      advisory: makeAdvisory({ final_recommendation: 'Long bias: BULLISH bias with 20% confidence. Entry: immediate. Stop: ATR-based.' }),
      // The user's real capture: long 62 / short 2 / hold 36 with readiness
      // STAND_ASIDE — the verdict is LONG (62%) and the gate is STAND ASIDE.
      decisionContext: makeDecisionContext({ trade_readiness: 'STAND_ASIDE', long_probability: 62, short_probability: 2, hold_probability: 36, net_bias_pct: 60 }),
      opportunity: makeOpportunity(),
      analysis: makeAnalysis(),
      symbol: 'SOL-USDC',
      markPrice: 75.55,
      headerSpec,
    }));
    expect(p.verdict.top).toBe('LONG');
    expect(p.final_verdict).toContain('LONG lean 62% — STAND ASIDE (readiness: STAND_ASIDE, entry_danger MODERATE)');
    expect(p.final_verdict).not.toContain('no directional call');
    expect(p.final_verdict_guidance).toContain('Environment guidance:');
    expect(p.final_verdict_guidance).toContain('immediate');
    // v6.10.17: a directional lean carries a REAL playbook — no caption.
    expect(p.strategy.hold_caption).toBeNull();
    expect(p.strategy.entry).not.toBe('—');
  });

  it('v6.10.19a (D2a): HOLD guidance has no orphaned ":," / trailing ":" after the confidence strip', () => {
    // The live Neutral-claim sentence ("…no directional edge: NEUTRAL bias
    // with 13% confidence, cautious…") loses the bias fragment to the
    // strip — the leftover ":," must collapse, never surface as "edge:,".
    const p = JSON.parse(buildRecommendationTabExport({
      advisory: makeAdvisory({ final_recommendation: 'Neutral — no directional edge: NEUTRAL bias with 13% confidence, cautious stance in a high-volatility environment. No clear opportunity.' }),
      decisionContext: makeDecisionContext({ trade_readiness: 'STAND_ASIDE', long_probability: 2, short_probability: 2, hold_probability: 96, net_bias_pct: 0, bias: 'Neutral' }),
      opportunity: makeOpportunity(),
      analysis: makeAnalysis(),
      symbol: 'SOL-USDC',
      markPrice: 75.55,
      headerSpec,
    }));
    expect(p.verdict.top).toBe('HOLD');
    // The "Environment guidance:" prefix colon is legitimate — the artifact
    // to ban is the orphaned ":," / dangling "edge:" after the claim.
    expect(p.final_verdict_guidance).not.toContain('edge:');
    expect(p.final_verdict_guidance).not.toContain(',,');
    expect(p.final_verdict_guidance).toContain('no directional edge');
    expect(p.final_verdict_guidance).not.toContain('NEUTRAL bias with 13% confidence');
    expect(p.final_verdict_guidance).toContain('cautious stance');
  });

  it('P6 (v6.10.19): lean_floor_applied rides the gauge when the floors boosted the split', () => {    const p = JSON.parse(buildRecommendationTabExport({
      advisory: makeAdvisory(),
      decisionContext: makeDecisionContext({ bias: 'Bullish', lean_floor_applied: true }),
      opportunity: makeOpportunity(),
      analysis: makeAnalysis(),
      symbol: 'SOL-USDC',
      markPrice: 75.55,
      headerSpec,
    }));
    expect(p.gauge.lean_floor_applied).toBe(true);
  });

  it('T3 (v6.10.19): a sub-1.0 aggregated bracket exports as R:R BELOW ACTIONABLE FLOOR with levels intact', () => {
    const p = JSON.parse(buildRecommendationTabExport({
      advisory: makeAdvisory(),
      decisionContext: makeDecisionContext({ bias: 'Bullish' }),
      opportunity: {
        ...makeOpportunity(),
        profiles: [],
        long_expected_rr_internal: 0.4,
        short_expected_rr_internal: 0.0,
      } as any,
      analysis: makeAnalysis(),
      symbol: 'SOL-USDC',
      markPrice: 75.55,
      headerSpec,
    }));
    expect(p.top_setup).not.toBeNull();
    expect(p.top_setup!.badge_text).toBe('R:R BELOW ACTIONABLE FLOOR');
    expect(p.top_setup!.rr_display).toBe('R:R 1 : 0.40');
    // Levels stay visible for manual analysis.
    expect(p.top_setup!.entry_zone).not.toBeNull();
    expect(p.top_setup!.invalidation).not.toBeNull();
  });

  it('R5: hold placeholder describes the aggregated bracket (not the close-pinned sentinel)', () => {
    const p = JSON.parse(buildRecommendationTabExport({
      advisory: makeAdvisory(),
      decisionContext: makeDecisionContext(), // HOLD verdict
      opportunity: makeOpportunity(),
      analysis: makeAnalysis(),
      symbol: 'SOL-USDC',
      markPrice: 75.55,
      headerSpec,
    }));
    expect(p.price_levels.side).toBe('hold');
    expect(p.price_levels.hold_placeholder).toContain('aggregated bracket on the net-bias side');
    expect(p.price_levels.hold_placeholder).not.toContain('entry = target = invalidation = close');
    expect(p.strategy.hold_caption).toContain('For reference — no active directional call');
  });

  it('RR-008: risk_adj_rr_explanation mirrors the header tooltip sentence', () => {
    const opp = {
      ...makeOpportunity(),
      primary_opportunity: 'TrendContinuation',
      long_expected_rr_internal: 2.0,
    } as OpportunityMatrix;
    const p = JSON.parse(buildRecommendationTabExport({
      advisory: makeAdvisory(),
      decisionContext: makeDecisionContext({
        bias: 'Bullish',
        expected_reward_risk_ratio: 0.6,
        long_probability: 60,
        short_probability: 10,
        hold_probability: 30,
      }),
      opportunity: opp,
      analysis: { ...makeAnalysis(), bias: 'Bullish' },
      symbol: 'SOL-USDC',
      markPrice: 75.55,
      headerSpec,
    }));
    expect(p.safety_flags.risk_adj_rr_explanation).toBe('Risk-adjusted: net R:R 2.00 × risk factor 0.30 = 0.60');
  });

  it('RR-008: explanation is null when there is no risk-adjusted R:R', () => {
    const p = JSON.parse(buildRecommendationTabExport({
      advisory: makeAdvisory(),
      decisionContext: makeDecisionContext(),
      opportunity: makeOpportunity(),
      analysis: makeAnalysis(),
      symbol: 'SOL-USDC',
      markPrice: 75.55,
      headerSpec,
    }));
    expect(p.safety_flags.risk_adj_rr_explanation).toBeNull();
  });
});
