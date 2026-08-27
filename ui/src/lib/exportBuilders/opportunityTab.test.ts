// Regression tests for the v7.0-audit Opportunity tab export.

import { describe, it, expect } from 'vitest';
import { buildOpportunityTabExport } from './opportunityTab';
import { buildRecommendationTabExport } from './recommendationTab';
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
        direction_family: 'TREND_RIDING',
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
        trade_viability: 'ACTIONABLE',
      },
      {
        opportunity_type: 'NoClearOpportunity',
        score: 0,
        preconditions_met: 1,
        preconditions_total: 1,
        notes: 'NoClearOpportunity',
        direction_family: 'NEUTRAL',
        trade_viability: 'NO_CLEAR',
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
    direction_family: 'TREND_RIDING',
    long_geometry_consistent: false,
    short_geometry_consistent: true,
  } as unknown as OpportunityMatrix;
}

function makeAnalysis(overrides: Partial<AnalysisMatrix> = {}): AnalysisMatrix {
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
    ...overrides,
  } as unknown as AnalysisMatrix;
}

function makeDecisionContext(overrides: Partial<DecisionContext> = {}): DecisionContext {
  return {
    ...overrides,
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

  it('confluent sources render full names (FIBONACCI, VOLUME PROFILE, ATR)', () => {
    const p = JSON.parse(buildOpportunityTabExport({
      opportunity: makeOpportunity(),
      analysis: makeAnalysis(),
      decisionContext: makeDecisionContext(),
      symbol: 'BTC-USDC',
      markPrice: 63369,
      headerSpec,
    }));
    expect(p.confluent_entry_levels[0].sources).toEqual(['FIBONACCI']);
    expect(p.confluent_entry_levels[1].sources).toEqual(['VOLUME PROFILE']);
    expect(p.confluent_target_levels[0].sources).toEqual(['ATR']);
  });

  it('v6.15: confluent rows carry the qualitative strength_label pill band', () => {
    const p = JSON.parse(buildOpportunityTabExport({
      opportunity: makeOpportunity(),
      analysis: makeAnalysis(),
      decisionContext: makeDecisionContext(),
      symbol: 'BTC-USDC',
      markPrice: 63369,
      headerSpec,
    }));
    expect(p.confluent_entry_levels[0].strength_label).toBe('VERY STRONG');
    expect(p.confluent_entry_levels[1].strength_label).toBe('MODERATE');
    expect(p.confluent_target_levels[0].strength_label).toBe('MODERATE');
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
    expect(p.rr_internal).toBeDefined();
    expect(p.rr_internal.expected_rr_value).toBeTypeOf('number');
    // reason is `string | null` — null when the active-side R:R resolved.
    expect(
      p.rr_internal.expected_rr_reason == null ||
        typeof p.rr_internal.expected_rr_reason === 'string',
    ).toBe(true);
    expect(p.rr_internal.time_horizon).toBe('INTRADAY');
  });

  it('audit C2: confluent_rr mirrors the per-side Expected R:R section', () => {
    const p = JSON.parse(buildOpportunityTabExport({
      opportunity: makeOpportunity(),
      analysis: makeAnalysis(),
      decisionContext: makeDecisionContext(),
      symbol: 'BTC-USDC',
      markPrice: 63369,
      headerSpec,
    }));
    // The fixture's confluent levels have no side tags, so the global
    // no-row reason must be reported — the export must never fabricate a
    // side row.
    expect(p.confluent_rr.reason).toBe('incomplete confluent levels');
    expect(p.confluent_rr.sides).toEqual([]);

    // With side-tagged levels, the per-side rows surface with the exact
    // screen magnitude labels.
    const opp = makeOpportunity() as any;
    opp.confluent_entry_levels = [
      { price: 63552.71, sources: ['FIBONACCI'], strength: 100, side: 'LONG' },
      { price: 64384.0, sources: ['FIBONACCI'], strength: 100, side: 'SHORT' },
    ];
    opp.confluent_target_levels = [
      { price: 64117.07, sources: ['ATR_FALLBACK'], strength: 35, side: 'LONG' },
      { price: 63310.0, sources: ['ATR_FALLBACK'], strength: 35, side: 'SHORT' },
    ];
    const p2 = JSON.parse(buildOpportunityTabExport({
      opportunity: opp,
      analysis: makeAnalysis(),
      decisionContext: makeDecisionContext(),
      symbol: 'BTC-USDC',
      markPrice: 63369,
      headerSpec,
    }));
    expect(p2.confluent_rr.sides.length).toBe(2);
    const long = p2.confluent_rr.sides.find((s: { side: string }) => s.side === 'LONG');
    expect(long).toBeDefined();
    expect(long.risk_basis).toBe('market_distance');
    expect(long.rr).toBeTypeOf('number');
    expect(long.rr_display).toMatch(/R(\+)?$/);
    expect(long.reason).toBeNull();
  });

  it('v7.3: an incomplete side falls back to bracket geometry (bracket_geometry basis)', () => {
    // The user-observed NoClear shape: only SHORT confluent levels are
    // side-tagged while the LONG reference bracket's zones are valid —
    // the export must mirror the panel's bracket-geometry LONG row.
    const opp = makeOpportunity() as any;
    opp.confluent_entry_levels = [
      { price: 64384.0, sources: ['FIBONACCI'], strength: 100, side: 'SHORT' },
    ];
    opp.confluent_target_levels = [
      { price: 63310.0, sources: ['ATR_FALLBACK'], strength: 35, side: 'SHORT' },
    ];
    opp.confluent_invalidation_levels = [
      { price: 64400.0, sources: ['FIBONACCI'], strength: 50, side: 'SHORT' },
    ];
    // LONG zones already valid in the fixture: entry 63320-63340
    // (mid 63330), target 63681-64380 (mid 64030.5), invalidation 63327.
    const p = JSON.parse(buildOpportunityTabExport({
      opportunity: opp,
      analysis: makeAnalysis(),
      decisionContext: makeDecisionContext(),
      symbol: 'BTC-USDC',
      markPrice: 63369,
      headerSpec,
    }));
    expect(p.confluent_rr.reason).toBeNull();
    expect(p.confluent_rr.sides.length).toBe(2);
    const long = p.confluent_rr.sides.find((s: { side: string }) => s.side === 'LONG');
    expect(long).toBeDefined();
    expect(long.risk_basis).toBe('bracket_geometry');
    expect(long.entry_avg).toBe(63330);
    expect(long.target_avg).toBe(64030.5);
    expect(long.invalidation_avg).toBe(63327);
    expect(long.rr).toBeTypeOf('number');
    expect(long.rr_display).toMatch(/R(\+)?$/);
    expect(long.reason).toBeNull();
    // The SHORT side stays confluent-averaged.
    const short = p.confluent_rr.sides.find((s: { side: string }) => s.side === 'SHORT');
    expect(short.risk_basis).toBe('invalidation');
    expect(short.rr).toBeTypeOf('number');
  });

  it('v6.14: score_display prefers the backend display_score (drift guard)', () => {
    // A 2/3-precondition profile: the local legacy rule would compute
    // round(60.12 × 2/3) = 40, but the wire carries the authoritative 37
    // — the export must mirror the wire value (single source of truth).
    const opp = makeOpportunity();
    opp.profiles = [
      {
        ...opp.profiles[0],
        score: 60.12,
        preconditions_met: 2,
        preconditions_total: 3,
        display_score: 37,
      },
    ];
    const p = JSON.parse(buildOpportunityTabExport({
      opportunity: opp,
      analysis: makeAnalysis(),
      decisionContext: makeDecisionContext(),
      symbol: 'BTC-USDC',
      markPrice: 63369,
      headerSpec,
    }));
    expect(p.trade_setups[0].score_display).toBe(37);
    expect(p.trade_setups[0].score).toBe(60.12);
    expect(p.evaluated_setups[0].score_display).toBe(37);
    expect(p.evaluated_setups[0].score).toBe(60.12);
  });

  it('v6.14: legacy payloads without display_score fall back to the local rule', () => {
    // No `display_score` on the wire → the export reproduces the legacy
    // displayScore rule: round(60.12 × 2/3) = 40.
    const opp = makeOpportunity();
    opp.profiles = [
      {
        ...opp.profiles[0],
        score: 60.12,
        preconditions_met: 2,
        preconditions_total: 3,
        display_score: null,
      },
    ];
    const p = JSON.parse(buildOpportunityTabExport({
      opportunity: opp,
      analysis: makeAnalysis(),
      decisionContext: makeDecisionContext(),
      symbol: 'BTC-USDC',
      markPrice: 63369,
      headerSpec,
    }));
    expect(p.trade_setups[0].score_display).toBe(40);
  });

  it('FIX-O4: a DirectionalNeutral card reports no_directional_bias, not no_actionable_geometry', () => {
    // The user's live capture: a Pullback DirectionalNeutral card with
    // consistent geometry whose R:R reads N/A because the market bias is
    // Neutral. The card reason must match `rr_internal` and the
    // recommendation ("no directional bias"), never the hardcoded
    // geometry fallback.
    const neutralOpp: OpportunityMatrix = {
      ...makeOpportunity(),
      primary_opportunity: 'Pullback',
      // Top-level aggregated bracket (the capture's Pullback geometry) —
      // the NEUTRAL-side card resolves it via the net-bias fallback.
      long_entry_zone: { low: 62350.67, high: 62916.26 },
      long_target_zone: { low: 63217.72, high: 63688.67 },
      long_invalidation_level: 62343.41,
      long_expected_rr_internal: 0,
      long_geometry_consistent: true,
      profiles: [
        {
          opportunity_type: 'Pullback',
          score: 60.24,
          preconditions_met: 2,
          preconditions_total: 2,
          notes: 'Pullback',
          direction_family: 'NEUTRAL',
          trade_viability: 'DIRECTIONAL_NEUTRAL',
          long_entry_zone: null,
          long_target_zone: null,
          long_invalidation_level: null,
          long_expected_rr_internal: 0,
          long_geometry_consistent: true,
          short_entry_zone: null,
          short_target_zone: null,
          short_invalidation_level: null,
          short_expected_rr_internal: 0,
          short_geometry_consistent: true,
        },
        {
          opportunity_type: 'NoClearOpportunity',
          score: 0,
          preconditions_met: 1,
          preconditions_total: 1,
          notes: 'NoClearOpportunity',
          direction_family: 'NEUTRAL',
          trade_viability: 'NO_CLEAR',
        },
      ],
    } as unknown as OpportunityMatrix;
    const p = JSON.parse(buildOpportunityTabExport({
      opportunity: neutralOpp,
      analysis: makeAnalysis({ bias: 'Neutral' }),
      // Hold-dominant like the capture (21/2/77) so the L6 top is HOLD —
      // that is what makes rr_internal report no_directional_bias.
      decisionContext: {
        ...makeDecisionContext(),
        bias: 'Neutral',
        net_bias_pct: 0,
        long_probability: 21,
        short_probability: 2,
        hold_probability: 77,
      },
      symbol: 'BTC-USDC',
      markPrice: 63018,
      headerSpec,
    }));
    const card = p.trade_setups.find((s: { opportunity_type: string }) => s.opportunity_type === 'Pullback');
    expect(card).toBeTruthy();
    expect(card.viability).toBe('DirectionalNeutral');
    expect(card.geometry_consistent).toBe(true);
    expect(card.rr_available).toBe(false);
    // Resolver's human-readable reason — matches the recommendation's
    // top_setup ("no directional bias"); rr_internal keeps the wire
    // snake_case key ('no_directional_bias') per the key-vs-display
    // convention.
    expect(card.rr_reason).toBe('no directional bias');
    expect(p.rr_internal.expected_rr_reason).toBe('no_directional_bias');
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

  it('environment includes the display string for Timeframes considered', () => {
    const p = JSON.parse(buildOpportunityTabExport({
      opportunity: makeOpportunity(),
      analysis: makeAnalysis(),
      decisionContext: makeDecisionContext(),
      symbol: 'BTC-USDC',
      markPrice: 63369,
      headerSpec,
    }));
    expect(p.environment.timeframes_considered).toBe(4);
    expect(p.environment.timeframes_considered_display).toBe('4 Timeframes considered');
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
    // v6.10.18 (I-5): the top profile's bracket carries a sub-1 R:R in
    // this fixture — the badge demotes to QUALIFYING (a real bracket, no
    // edge to act on), never "TOP · ACTIONABLE".
    expect(p.trade_setups.length).toBeGreaterThan(0);
    const first = p.trade_setups[0];
    expect(['TOP · ACTIONABLE', 'QUALIFYING', 'ACTIONABLE', 'RANGE · NEUTRAL', 'GEOMETRY INVERTED', 'Actionable', 'Qualifying', 'DirectionalNeutral', 'GeometryInverted', 'NoClear'])
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
          direction_family: 'TREND_RIDING',
          trade_viability: 'ACTIONABLE',
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

  it('directional_bars always emit — L4 bracket conviction only (never the L6 split)', () => {
    // v7.1: the L4 bars read ONLY the opportunity matrix (score ×
    // active-side R:R). The L6 decision probabilities never shape them —
    // with no matrix the bars are 0/0/100 even when the decision context
    // carries a directional split (30/40/30 here).
    const p = JSON.parse(buildOpportunityTabExport({
      opportunity: null,
      analysis: makeAnalysis(),
      decisionContext: makeDecisionContext(),
      symbol: 'BTC-USDC',
      markPrice: 63369,
      headerSpec,
    }));
    expect(p.directional_bars).toEqual({ bullish_pct: 0, bearish_pct: 0, hold_pct: 100, sort: 'desc' });

    // Probability-less legacy payloads behave identically.
    const legacy = JSON.parse(buildOpportunityTabExport({
      opportunity: null,
      analysis: makeAnalysis(),
      decisionContext: { ...makeDecisionContext(), long_probability: undefined, short_probability: undefined, hold_probability: undefined } as any,
      symbol: 'BTC-USDC',
      markPrice: 63369,
      headerSpec,
    }));
    expect(legacy.directional_bars).toEqual({ bullish_pct: 0, bearish_pct: 0, hold_pct: 100, sort: 'desc' });
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

  it('empty states render "—" placeholders (horizon, market position)', () => {
    const p = JSON.parse(buildOpportunityTabExport({
      opportunity: null,
      analysis: null,
      decisionContext: makeDecisionContext(),
      symbol: 'BTC-USDC',
      markPrice: 63369,
      headerSpec,
    }));
    expect(p.rr_internal.time_horizon).toBe('—');
    expect(p.market_position.bias).toBe('—');
    expect(p.market_position.regime).toBe('—');
    expect(p.market_position.trend).toBe('—');
    expect(p.market_position.quality).toBe('—');
  });

  it('C2 (v6.10.19b): trade_setup_sections mirror the sectioned panel — three always-present sections with full per-setup values', () => {
    const p = JSON.parse(buildOpportunityTabExport({
      opportunity: makeOpportunity(),
      analysis: makeAnalysis(),
      decisionContext: makeDecisionContext(),
      symbol: 'BTC-USDC',
      markPrice: 63369,
      headerSpec,
    }));
    expect(p.trade_setup_sections.length).toBe(3);
    // v7.1: folders are RANKED by content (the populated folder first —
    // same relevance ordering as the conviction bars), then by the top
    // setup's score; the fixed RANGE → BULL → BEAR fallback only applies
    // to empty ties. Here BULL hosts the qualifying Breakout, BEAR hosts
    // its reference bracket, NEUTRAL is empty.
    expect(p.trade_setup_sections[0]).toMatchObject({ section: 'BULL', label: 'BULLISH' });
    expect(p.trade_setup_sections[1]).toMatchObject({ section: 'BEAR', label: 'BEARISH' });
    expect(p.trade_setup_sections[2]).toMatchObject({ section: 'NEUTRAL', label: 'RANGE' });
    // The qualifying LONG Breakout rides in BULL with EVERY value.
    const setup = p.trade_setup_sections[0].setups[0];
    expect(setup).toBeTruthy();
    expect(setup.opportunity_type).toBe('Breakout');
    expect(setup.entry_zone).toEqual({ low: 63320, high: 63340 });
    expect(setup.tp1).toBeGreaterThan(0);
    expect(setup.tp2).toBeGreaterThan(0);
    expect(setup.invalidation).toBe(63327);
    // The fixture's Breakout carries long_geometry_consistent: false —
    // the A2 fix respects the server flag (R:R N/A, never a leaked value).
    expect(setup.rr_value).toBeNull();
    expect(setup.rr_reason).toBe('geometry inverted');
    expect(setup.preconditions_met).toBe(2);
    expect(setup.preconditions_total).toBe(2);
    expect(setup.geometry_consistent).toBe(false);
    // v6.10.21: a card whose geometry is inconsistent always renders the
    // State D warning badge — never a Qualifying badge (panel parity).
    expect(setup.badge_text).toBe('GEOMETRY INVERTED');
    expect(setup.notes).toBe('Breakout');
    expect(setup.section).toBe('BULL');
    // The BEAR folder hosts the SHORT aggregated reference bracket
    // (informational — the folder's content count includes it).
    expect(p.trade_setup_sections[1].setups.length).toBe(1);
    expect(p.trade_setup_sections[1].setups[0].opportunity_type).toBe('Aggregated Bracket');
    expect(p.trade_setup_sections[1].setups[0].badge_text).toBe('INFORMATIONAL');
    // Empty sections stay present.
    expect(p.trade_setup_sections[2].setups).toEqual([]);
  });

  it('C1 (v6.10.19b, parity invariant): the SHORT-verdict reference bracket the Recommendation headlines rides in the BEAR section with the SAME zones', () => {
    // The 20:42 shape: SHORT verdict (54%) with only a LONG countertrend
    // MeanReversion qualifying. The Recommendation headlines the SHORT
    // aggregated reference bracket; the Opportunities export must carry
    // that exact bracket (BEAR section) AND the LONG qualifying profile
    // (BULL section) — nothing is lost.
    const opp = makeOpportunity();
    opp.primary_opportunity = 'MeanReversion';
    opp.opportunity_score = 52.7;
    opp.long_entry_zone = { low: 62558.5, high: 63023.9 };
    opp.long_target_zone = { low: 63134.4, high: 63416.2 };
    opp.long_invalidation_level = 62558.2;
    opp.long_expected_rr_internal = 1.14;
    opp.long_geometry_consistent = true;
    opp.short_entry_zone = { low: 63071, high: 63416.2 };
    opp.short_target_zone = { low: 62978.6, high: 63030 };
    opp.short_invalidation_level = 63416.4;
    opp.short_expected_rr_internal = 1.12;
    opp.short_geometry_consistent = true;
    opp.profiles = [{
      opportunity_type: 'MeanReversion',
      score: 52.7,
      preconditions_met: 2,
      preconditions_total: 2,
      notes: 'MeanReversion',
      direction_family: 'COUNTER_TREND',
      long_entry_zone: { low: 62558.5, high: 63023.9 },
      long_target_zone: { low: 63134.4, high: 63416.2 },
      long_invalidation_level: 62558.2,
      long_expected_rr_internal: 1.14,
      long_geometry_consistent: true,
      short_entry_zone: null,
      short_target_zone: null,
      short_invalidation_level: null,
      short_expected_rr_internal: null,
      short_geometry_consistent: false,
      trade_viability: 'ACTIONABLE',
    }];
    const dc = {
      ...makeDecisionContext(),
      bias: 'Bearish',
      long_probability: 2,
      short_probability: 54,
      hold_probability: 44,
      net_bias_pct: -52,
    } as unknown as DecisionContext;
    const analysis = makeAnalysis({ bias: 'Bearish' });
    const oppExport = JSON.parse(buildOpportunityTabExport({
      opportunity: opp, analysis, decisionContext: dc, symbol: 'BTC-USDC', markPrice: 63047, headerSpec,
    }));
    const recExport = JSON.parse(buildRecommendationTabExport({
      advisory: { directional_guidance: 'Short', market_stance: 'Cautious', strategy_environment: 'MeanReversion', opportunity_classification: 'MeanReversion', confidence_assessment: 18.6 } as any,
      decisionContext: dc, opportunity: opp, analysis, symbol: 'BTC-USDC', markPrice: 63047, headerSpec,
    }));
    // The Recommendation headlines the verdict-consistent SHORT bracket.
    expect(recExport.top_setup.direction_label).toBe('SHORT');
    expect(recExport.top_setup.opportunity_type).toBe('Aggregated Bracket');
    // The identical bracket rides in the BEAR section of Opportunities.
    // v7.1: sections are RANKED — BULL (the qualifying LONG MeanReversion)
    // first, then BEAR (its reference bracket), then empty NEUTRAL — so
    // find the BEAR section by key instead of by fixed index.
    const bearSection = oppExport.trade_setup_sections.find((s: any) => s.section === 'BEAR');
    const bearRef = bearSection.setups.find((r: any) => r.opportunity_type === 'Aggregated Bracket');
    expect(bearRef).toBeTruthy();
    expect(bearRef!.entry_zone.low).toBeCloseTo(recExport.top_setup.entry_zone.low, 1);
    expect(bearRef!.entry_zone.high).toBeCloseTo(recExport.top_setup.entry_zone.high, 1);
    expect(bearRef!.invalidation).toBeCloseTo(recExport.top_setup.invalidation, 1);
    expect(bearRef!.badge_text).toBe('INFORMATIONAL');
    // The LONG qualifying profile stays in the BULL section.
    const bullSection = oppExport.trade_setup_sections.find((s: any) => s.section === 'BULL');
    expect(bullSection.setups[0].opportunity_type).toBe('Mean Reversion');
    // BULL ranks first (content tie with BEAR, top score wins).
    expect(oppExport.trade_setup_sections[0].section).toBe('BULL');
    // And the Recommendation surfaces it as an alternate.
    expect(recExport.top_setup.alternate_qualifying_setups.length).toBe(1);
    expect(recExport.top_setup.alternate_qualifying_setups[0].side).toBe('LONG');
  });
});
describe('buildOpportunityTabExport — v7.0 summary block', () => {
  it('emits the SUMMARY paragraph + label (panel parity)', () => {
    const p = JSON.parse(buildOpportunityTabExport({
      opportunity: makeOpportunity(),
      analysis: makeAnalysis(),
      decisionContext: makeDecisionContext(),
      symbol: 'BTC-USDT',
      markPrice: 64000,
      headerSpec,
    }));
    expect(p.summary_label).toBe('SUMMARY');
    // Fixture: primary Breakout, opportunity_score 60.12 → moderate band.
    expect(p.summary).toContain('moderate-conviction breakout phase');
    expect(p.header.summary_label).toBeNull();
  });

  it('emits the highlighted summary_display while keeping summary raw (panel parity)', () => {
    const p = JSON.parse(buildOpportunityTabExport({
      opportunity: makeOpportunity(),
      analysis: makeAnalysis(),
      decisionContext: makeDecisionContext(),
      symbol: 'BTC-USDT',
      markPrice: 64000,
      headerSpec,
    }));
    // Raw string is untouched by the highlighter.
    expect(p.summary).not.toContain('<strong>');
    // Display variant mirrors the panel's @html rendering.
    expect(p.summary_display).toContain('<strong>moderate-conviction</strong>');
    expect(p.summary_display.replace(/<strong>|<\/strong>/g, '')).toBe(p.summary);
  });

  it('emits the awaiting fallback paragraph when the matrix is null', () => {
    const p = JSON.parse(buildOpportunityTabExport({
      opportunity: null,
      analysis: makeAnalysis(),
      decisionContext: makeDecisionContext(),
      symbol: 'BTC-USDT',
      markPrice: 64000,
      headerSpec,
    }));
    expect(p.summary).toContain('Awaiting opportunity data');
  });
});

describe('buildOpportunityTabExport — gross R:R side resolution (v2026-08)', () => {
  it('resolves the gross R:R to the active side under a bearish verdict (no 0.0 leak)', () => {
    // The wire fields are plain f64 (serde default 0.0), so a naive
    // `long ?? short` fallback exports 0.0 for a valid SHORT bracket.
    // The side-resolved chain (zone-presence-first) must pick the short
    // side: the qualifying profile carries SHORT zones only.
    const opp = {
      ...makeOpportunity(),
      profiles: [
        {
          opportunity_type: 'Breakout',
          score: 60.12,
          preconditions_met: 2,
          preconditions_total: 2,
          notes: 'Breakout',
          direction_family: 'TREND_RIDING',
          long_entry_zone: null,
          long_target_zone: null,
          long_invalidation_level: null,
          long_expected_rr_internal: 0,
          long_geometry_consistent: false,
          short_entry_zone: { low: 64363, high: 64384 },
          short_target_zone: { low: 63264, high: 63310 },
          short_invalidation_level: 64384,
          short_expected_rr_internal: 8.04,
          short_geometry_consistent: true,
          trade_viability: 'ACTIONABLE',
        },
      ],
      long_gross_rr_internal: 0.0,
      short_gross_rr_internal: 3.2,
    } as unknown as OpportunityMatrix;
    const p = JSON.parse(buildOpportunityTabExport({
      opportunity: opp,
      analysis: makeAnalysis({ bias: 'Bearish' }),
      decisionContext: makeDecisionContext({ bias: 'Bearish', trade_readiness: 'READY' }),
      symbol: 'BTC-USDC',
      markPrice: 63369,
      headerSpec,
    }));
    expect(p.rr_internal.gross_rr_value).toBe(3.2);
  });

  it('exports null gross R:R under a NEUTRAL verdict with no qualifying side', () => {
    // A neutral bias carries no actionable zones (02-08: "Neutral carries
    // no actionable setup") — the side chain resolves NEUTRAL → null.
    const opp = {
      ...makeOpportunity(),
      profiles: [
        {
          opportunity_type: 'NoClearOpportunity',
          score: 0,
          preconditions_met: 0,
          preconditions_total: 1,
          notes: 'No clear opportunity',
          direction_family: 'NEUTRAL',
          trade_viability: 'NO_CLEAR',
        },
      ],
      long_gross_rr_internal: 0.0,
      short_gross_rr_internal: 0.0,
    } as unknown as OpportunityMatrix;
    const p = JSON.parse(buildOpportunityTabExport({
      opportunity: opp,
      analysis: makeAnalysis({ bias: 'Neutral' }),
      decisionContext: makeDecisionContext({ bias: 'Neutral' }),
      symbol: 'BTC-USDC',
      markPrice: 63369,
      headerSpec,
    }));
    expect(p.rr_internal.gross_rr_value).toBeNull();
  });
});
