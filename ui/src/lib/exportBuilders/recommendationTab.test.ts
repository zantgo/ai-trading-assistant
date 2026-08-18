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
    direction_family: 'NEUTRAL',
    long_geometry_consistent: true,
    short_geometry_consistent: true,
  } as unknown as OpportunityMatrix;
}

function makeAnalysis(): AnalysisMatrix {
  return {
    bias: 'Neutral',
    confidence: 0.13,
    state_confidence: 0.1,
    market_regime: 'Range',
    market_quality: 'Average',
    market_phase: 'Accumulation',
    timeframes_considered: 4,
    supporting_signals: [],
    contradicting_signals: [],
    trend_assessment: 'Weak',
    momentum_assessment: 'Stable',
    structure_assessment: 'Weak',
    volatility_assessment: 'Normal',
    volume_assessment: 'Normal',
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
    // v6.10.19c (D4): the empty container carries no bracket → N/A.
    expect(p.safety_flags.rr_reason).toBe('no_wire_rr');
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

  it('v6.11: quality_to_risk_ratio flows into environment and safety_flags blocks', () => {
    const p = JSON.parse(buildRecommendationTabExport({
      advisory: makeAdvisory({ quality_to_risk_ratio: 3.2 }),
      decisionContext: makeDecisionContext(),
      opportunity: makeOpportunity(),
      analysis: makeAnalysis(),
      symbol: 'SOL-USDC',
      markPrice: 75.55,
      headerSpec,
    }));
    expect(p.environment.quality_to_risk_ratio).toBeCloseTo(3.2, 1);
    expect(p.environment.quality_to_risk_ratio_display).toBe('3.20');
    expect(p.safety_flags.quality_to_risk_ratio).toBeCloseTo(3.2, 1);
    expect(p.safety_flags.quality_to_risk_ratio_display).toBe('3.20');
  });

  it('v6.11: missing quality_to_risk_ratio renders em-dash displays', () => {
    const p = JSON.parse(buildRecommendationTabExport({
      advisory: makeAdvisory({ quality_to_risk_ratio: null }),
      decisionContext: makeDecisionContext(),
      opportunity: makeOpportunity(),
      analysis: makeAnalysis(),
      symbol: 'SOL-USDC',
      markPrice: 75.55,
      headerSpec,
    }));
    expect(p.environment.quality_to_risk_ratio).toBeNull();
    expect(p.environment.quality_to_risk_ratio_display).toBe('\u2014');
    expect(p.safety_flags.quality_to_risk_ratio).toBeNull();
    expect(p.safety_flags.quality_to_risk_ratio_display).toBe('\u2014');
  });

  it('v6.14: top_setup.score_display mirrors the backend display_score (raw score preserved)', () => {
    // A 1/3-precondition top profile: the wire carries display_score 33 —
    // the export's `score_display` must prefer it (screen parity), while
    // the raw `score` stays 65 for data-science consumers.
    const opp = makeOpportunity();
    opp.opportunity_score = 65;
    opp.profiles = [
      {
        opportunity_type: 'Breakout',
        score: 65,
        preconditions_met: 1,
        preconditions_total: 3,
        display_score: 33,
        notes: 'Breakout',
        direction_family: 'TREND_RIDING',
        long_entry_zone: { low: 63320, high: 63340 },
        long_target_zone: { low: 63681, high: 64380 },
        long_invalidation_level: 63327,
        long_expected_rr_internal: 2.5,
        long_geometry_consistent: true,
        short_entry_zone: null,
        short_target_zone: null,
        short_invalidation_level: null,
        short_expected_rr_internal: null,
        short_geometry_consistent: false,
        trade_viability: 'ACTIONABLE',
      },
    ] as any;
    const p = JSON.parse(buildRecommendationTabExport({
      advisory: makeAdvisory(),
      decisionContext: makeDecisionContext(),
      opportunity: opp,
      analysis: makeAnalysis(),
      symbol: 'SOL-USDC',
      markPrice: 75.55,
      headerSpec,
    }));
    expect(p.top_setup.score).toBe(65);
    expect(p.top_setup.score_display).toBe(33);
  });

  it('v6.14: legacy top_setup without display_score falls back to the raw score', () => {
    const p = JSON.parse(buildRecommendationTabExport({
      advisory: makeAdvisory(),
      decisionContext: makeDecisionContext(),
      opportunity: makeOpportunity(),
      analysis: makeAnalysis(),
      symbol: 'SOL-USDC',
      markPrice: 75.55,
      headerSpec,
    }));
    expect(p.top_setup.score_display).toBe(p.top_setup.score);
  });

  it('D4 (v6.10.19c): Risk-Adj R:R is bracket-aware — a HOLD + qualifying setup shows the discounted ratio, not N/A', () => {
    // The 20:46 shape: hold-dominant with a qualifying NEUTRAL-side
    // MeanReversion whose LONG-populated zones carry an R:R. The chip
    // must compute bracket R:R × (1 − overall_risk/100) instead of N/A.
    const opp = makeOpportunity();
    opp.primary_opportunity = 'MeanReversion';
    opp.opportunity_score = 55;
    opp.profiles = [
      {
        opportunity_type: 'MeanReversion',
        score: 55,
        preconditions_met: 2,
        preconditions_total: 2,
        notes: 'MeanReversion',
        direction_family: 'COUNTER_TREND',
        long_entry_zone: { low: 63058.7, high: 63059.6 },
        long_target_zone: { low: 63104.07, high: 63207.26 },
        long_invalidation_level: 63055.67,
        long_expected_rr_internal: 4.5,
        long_geometry_consistent: true,
        short_entry_zone: null,
        short_target_zone: null,
        short_invalidation_level: null,
        short_expected_rr_internal: null,
        short_geometry_consistent: false,
        trade_viability: 'DIRECTIONAL_NEUTRAL',
      },
    ] as any;
    const p = JSON.parse(buildRecommendationTabExport({
      advisory: makeAdvisory(),
      decisionContext: makeDecisionContext({ bias: 'Neutral', long_probability: 12, short_probability: 2, hold_probability: 86, net_bias_pct: 10, expected_reward_risk_ratio: 0 }),
      opportunity: opp,
      analysis: makeAnalysis(),
      symbol: 'SOL-USDC',
      markPrice: 75.55,
      overallRisk: 40,
      headerSpec,
    }));
    // The headline setup rides in the container with its bracket R:R.
    expect(p.top_setup!.opportunity_type).toBe('Mean Reversion');
    expect(p.top_setup!.rr_value).toBeCloseTo(4.5, 1);
    // The Risk-Adj chip computes 4.5 × (1 − 0.40) = 2.7 — not N/A.
    expect(p.safety_flags.rr_available).toBe(true);
    expect(p.safety_flags.rr_value).toBeCloseTo(2.7, 1);
    expect(p.safety_flags.risk_adj_rr_explanation).toContain('2.70');
  });

  it('D3 (v6.10.19c): no qualifying setup → the clean "No Active Setup" container (no badges, no no-clear card)', () => {
    const p = JSON.parse(buildRecommendationTabExport({
      advisory: makeAdvisory(),
      decisionContext: makeDecisionContext(),
      opportunity: makeOpportunity(),
      analysis: makeAnalysis(),
      symbol: 'SOL-USDC',
      markPrice: 75.55,
      headerSpec,
    }));
    expect(p.top_setup).not.toBeNull();
    expect(p.top_setup!.opportunity_type).toBe('No Active Setup');
    expect(p.top_setup!.viability).toBe('NoClear');
    expect(p.top_setup!.badge_text).toBe('');
    expect(p.top_setup!.entry_zone).toBeNull();
    expect(p.top_setup!.target_zone).toBeNull();
    expect(p.top_setup!.invalidation).toBeNull();
    expect(p.top_setup!.rr_value).toBeNull();
    expect(p.top_setup!.hold_placeholder).toBe('No active setup.');
    expect(p.no_clear_card).toBeUndefined();
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
    expect(p.strategy).not.toHaveProperty('hold_caption');
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
    expect(p.strategy).not.toHaveProperty('hold_caption');
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
          direction_family: 'TREND_RIDING',
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
          trade_viability: 'ACTIONABLE',
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

  it('D3 (v6.10.19c): no qualifying setup → "No Active Setup" under HOLD; the verdict-side reference bracket under a directional verdict', () => {
    // HOLD + no qualifying → the clean empty container (never a bare
    // null; never a NO CLEAR SETUP badge).
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
    expect(p.top_setup!.opportunity_type).toBe('No Active Setup');
    expect(p.top_setup!.viability).toBe('NoClear');
    expect(p.top_setup!.badge_text).toBe('');
    expect(p.top_setup_empty_text).toBeNull();
    // Directional verdict + no qualifying on that side → the verdict-side
    // aggregated reference bracket is still published (T3/B1).
    const pd = JSON.parse(buildRecommendationTabExport({
      advisory: makeAdvisory(),
      decisionContext: makeDecisionContext({ bias: 'Bullish', long_probability: 60, short_probability: 5, hold_probability: 35, net_bias_pct: 55 }),
      opportunity: makeOpportunity(),
      analysis: makeAnalysis(),
      symbol: 'SOL-USDC',
      markPrice: 75.55,
      headerSpec,
    }));
    expect(pd.top_setup!.opportunity_type).toBe('Aggregated Bracket');
    expect(pd.top_setup!.badge_text).toBe('NO CLEAR SETUP');
    expect(pd.no_clear_card).toBeUndefined();
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
    expect(p.final_verdict).toContain('LONG lean 60% — awaiting confirmation (readiness: Forming)');
    expect(p.final_verdict).not.toContain('Strong long bias');
    expect(p.final_verdict_guidance).toContain('Environment guidance:');
    expect(p.strategy).not.toHaveProperty('hold_caption');
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
    expect(p.final_verdict).toContain('LONG lean 62% — Stand Aside (readiness: Stand Aside, Entry Danger Moderate)');
    expect(p.final_verdict).not.toContain('no directional call');
    expect(p.final_verdict_guidance).toContain('Environment guidance:');
    // v6.17: execution clauses are stripped under every verdict and the
    // guidance leads with the verdict's own read (never the stale claim).
    expect(p.final_verdict_guidance).not.toContain('immediate');
    expect(p.final_verdict_guidance).toContain('Bullish market bias with 62% confidence');
    expect(p.final_verdict_guidance).not.toContain('BULLISH bias with 20% confidence');
    // v6.10.17: a directional lean carries a REAL playbook — no caption.
    expect(p.strategy).not.toHaveProperty('hold_caption');
    expect(p.strategy.entry).not.toBe('—');
  });

  it('v6.17: directional guidance is verdict-consistent — the neutral claim is rewritten, the tail survives', () => {
    // The user's real capture shape: a LONG verdict headline with a stale
    // neutral advisory sentence — the guidance must now lead with the
    // verdict's own direction + probability, never "Neutral — no
    // directional edge".
    const p = JSON.parse(buildRecommendationTabExport({
      advisory: makeAdvisory({ final_recommendation: 'Neutral — no directional edge: 28% confidence, cautious stance in a trend-following environment. Breakout opportunity.' }),
      decisionContext: makeDecisionContext({ trade_readiness: 'WATCH', long_probability: 71, short_probability: 10, hold_probability: 19, net_bias_pct: 61 }),
      opportunity: makeOpportunity(),
      analysis: makeAnalysis(),
      symbol: 'SOL-USDC',
      markPrice: 75.55,
      headerSpec,
    }));
    expect(p.verdict.top).toBe('LONG');
    expect(p.final_verdict).toContain('LONG lean 71% — awaiting confirmation (readiness: Watch)');
    expect(p.final_verdict_guidance).toBe('Environment guidance: Bullish market bias with 71% confidence, cautious stance in a trend-following environment. Breakout opportunity.');
    expect(p.final_verdict_guidance).not.toContain('Neutral');
    expect(p.final_verdict_guidance).not.toContain('no directional edge');

    // SHORT verdict mirrors the bearish lead.
    const pShort = JSON.parse(buildRecommendationTabExport({
      advisory: makeAdvisory({ final_recommendation: 'Long bias: BULLISH bias with 72% confidence.' }),
      decisionContext: makeDecisionContext({ trade_readiness: 'WATCH', long_probability: 20, short_probability: 60, hold_probability: 20, net_bias_pct: -40 }),
      opportunity: makeOpportunity(),
      analysis: makeAnalysis(),
      symbol: 'SOL-USDC',
      markPrice: 75.55,
      headerSpec,
    }));
    expect(pShort.verdict.top).toBe('SHORT');
    expect(pShort.final_verdict_guidance).toBe('Environment guidance: Bearish market bias with 60% confidence.');
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
      // v6.10.19b (B1): the demotion only renders under a DIRECTIONAL
      // verdict with no qualifying profile on that side — force LONG.
      decisionContext: makeDecisionContext({ bias: 'Bullish', long_probability: 60, short_probability: 5, hold_probability: 35, net_bias_pct: 55 }),
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
    expect(p.top_setup!.rr_display).toBe('0.40');
    // Levels stay visible for manual analysis.
    expect(p.top_setup!.entry_zone).not.toBeNull();
    expect(p.top_setup!.invalidation).not.toBeNull();
  });

  it('R5: hold placeholder describes the unified HOLD state', () => {
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
    // v6.10.19d (D): the "fields are placeholders" / "Qualifying setups…"
    // copy is gone — the placeholder is the single clean sentence.
    expect(p.price_levels.hold_placeholder).toBe('No active setup.');
    expect(p.strategy).not.toHaveProperty('hold_caption');
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

describe('buildRecommendationTabExport — v7.0 projection block', () => {
  it('exports the empty (configured: false) projection when the drawer was never used', () => {
    const p = JSON.parse(buildRecommendationTabExport({
      advisory: makeAdvisory(),
      decisionContext: makeDecisionContext(),
      opportunity: makeOpportunity(),
      analysis: makeAnalysis(),
      symbol: 'SOL-USDC',
      markPrice: 75.55,
      headerSpec,
    }));
    expect(p.projection.configured).toBe(false);
    expect(p.projection.capital).toBeNull();
    expect(p.projection.roi_pct).toBeNull();
    expect(p.projection.position_size_units).toBeNull();
  });

  it('populates the projection block verbatim once configured', () => {
    const p = JSON.parse(buildRecommendationTabExport({
      advisory: makeAdvisory(),
      decisionContext: makeDecisionContext(),
      opportunity: makeOpportunity(),
      analysis: makeAnalysis(),
      symbol: 'SOL-USDC',
      markPrice: 75.55,
      headerSpec,
      projection: {
        configured: true,
        capital: 100,
        leverage: 10,
        direction: 'LONG',
        entry_price: 75.55,
        stop_loss: 74.0,
        take_profit: 80.0,
        position_size_units: 0.12,
        position_notional_usd: 1000,
        entry_fee_usd: 0.6,
        exit_fee_usd: 0.6,
        total_fees_usd: 1.2,
        liquidation_price: 72.5,
        net_profit_usd: 55.0,
        roi_pct: 55.0,
      },
    }));
    expect(p.projection.configured).toBe(true);
    expect(p.projection.capital).toBe(100);
    expect(p.projection.leverage).toBe(10);
    expect(p.projection.direction).toBe('LONG');
    expect(p.projection.roi_pct).toBe(55.0);
  });
});
