import { describe, expect, it } from 'vitest';
import { computeOpportunityBars, resolveEffectiveDirection } from './opportunityBars';
import type { OpportunityMatrix, OpportunityProfile } from '../types';

function profile(partial: Partial<OpportunityProfile> & { opportunity_type: string }): OpportunityProfile {
  return {
    score: 50,
    preconditions_met: 0,
    preconditions_total: 3,
    notes: '',
    direction_family: 'TrendRiding',
    long_entry_zone: null,
    long_target_zone: null,
    long_invalidation_level: null,
    short_entry_zone: null,
    short_target_zone: null,
    short_invalidation_level: null,
    long_expected_rr_internal: null,
    short_expected_rr_internal: null,
    trade_viability: null,
    ...partial,
  };
}

function opp(partial: Partial<OpportunityMatrix> & { opportunity_score?: number }): OpportunityMatrix {
  return {
    symbol: 'BTC-USDC',
    primary_opportunity: 'TrendContinuation',
    opportunity_score: 50,
    setup_quality: 'MODERATE',
    profiles: [],
    forecast_confidence: 0.5,
    contributing_signals: [],
    invalidation_note: '',
    entry_zone: { low: 0, high: 0 },
    target_zone: { low: 0, high: 0 },
    invalidation_level: 0,
    long_entry_zone: { low: 0, high: 0 },
    long_target_zone: { low: 0, high: 0 },
    long_invalidation_level: 0,
    short_entry_zone: { low: 0, high: 0 },
    short_target_zone: { low: 0, high: 0 },
    short_invalidation_level: 0,
    long_expected_rr_internal: 0,
    short_expected_rr_internal: 0,
    time_horizon: 'SWING',
    confluent_entry_levels: [],
    confluent_target_levels: [],
    confluent_invalidation_levels: [],
    ...partial,
  };
}

describe('resolveEffectiveDirection', () => {
  it('resolves NEUTRAL when the matrix is missing', () => {
    expect(resolveEffectiveDirection(null, 'Bullish')).toBe('NEUTRAL');
  });

  it('prefers the top qualifying profile side over the bias', () => {
    // CounterTrend MeanReversion under a Bearish bias that resolved
    // SHORT via its populated zones (deviation-driven, 4b).
    const o = opp({
      long_expected_rr_internal: 2.51,
      short_expected_rr_internal: 1.21,
      profiles: [
        profile({
          opportunity_type: 'MeanReversion',
          direction_family: 'CounterTrend',
          preconditions_met: 2,
          preconditions_total: 2,
          long_entry_zone: null,
          short_entry_zone: { low: 100.5, high: 101 },
          short_target_zone: { low: 98, high: 99 },
          short_invalidation_level: 102,
        }),
      ],
    });
    expect(resolveEffectiveDirection(o, 'Bearish')).toBe('SHORT');
  });

  it('falls back to the macro bias when no profile qualifies', () => {
    const o = opp({
      long_expected_rr_internal: 2.51,
      short_expected_rr_internal: 1.21,
    });
    expect(resolveEffectiveDirection(o, 'Bearish')).toBe('SHORT');
    expect(resolveEffectiveDirection(o, 'StrongBullish')).toBe('LONG');
  });

  it('FIX-1: Neutral bias resolves NEUTRAL even when one bracket has a larger R:R', () => {
    // The legacy argmax fallback lit the bars/badge directionally on a
    // directionally-neutral panel (57% "bearish" beside a DirectionalNeutral
    // card, `Lean: neutral`, and N/A R:R). Under Neutral bias the L4
    // directional surfaces are neutral by design.
    const o = opp({
      long_expected_rr_internal: 1.0,
      short_expected_rr_internal: 2.5,
    });
    expect(resolveEffectiveDirection(o, 'Neutral')).toBe('NEUTRAL');
    const bars = computeOpportunityBars(o, 'Neutral');
    expect(bars).toEqual({ bullish: 0, bearish: 0, hold: 100 });
  });
});

describe('computeOpportunityBars', () => {
  it('returns 100% hold when opportunity matrix is missing', () => {
    expect(computeOpportunityBars(null)).toEqual({ bullish: 0, bearish: 0, hold: 100 });
  });

  it('returns 100% hold when both directional R:R values are zero and nothing qualifies', () => {
    const bars = computeOpportunityBars(
      opp({ long_expected_rr_internal: 0, short_expected_rr_internal: 0, opportunity_score: 50 }),
      'Neutral',
    );
    expect(bars).toEqual({ bullish: 0, bearish: 0, hold: 100 });
  });

  it('REGRESSION (real 60s sample): bearish panel with a larger countertrend LONG R:R must NOT light up bullish', () => {
    // The user's 60s BTC sample: long_expected_rr_internal ≈ 2.51
    // (countertrend buy-dip bracket), short ≈ 1.21, score 59.17,
    // bearish bias, no qualifying profile. The old implementation
    // emitted 58/1/41 BULLISH-dominant bars that contradicted the
    // panel. The active side is the macro bias side (SHORT) and only
    // the active side's R:R counts.
    const bars = computeOpportunityBars(
      opp({
        long_expected_rr_internal: 2.51,
        short_expected_rr_internal: 1.21,
        opportunity_score: 59.17,
      }),
      'Bearish',
    );
    expect(bars.bullish).toBe(0);
    expect(bars.bearish).toBeGreaterThan(50);
    expect(bars.bearish + bars.bullish + bars.hold).toBe(100);
  });

  it('NO CLEAR SETUP (score 0) with a valid bracket shows the MIN_ACTIVE_FLOOR conviction (v6.10.12)', () => {
    // After the primary-selection fix the same 60s market headlines
    // NoClearOpportunity with score 0. The old hard cap floored any
    // conviction to 0 — 0/0/100 beside a Recommendation gauge showing a
    // real directional distribution. A valid active-side bracket now
    // always carries at least MIN_ACTIVE_FLOOR (30) directional
    // conviction; the remainder stays Hold.
    const bars = computeOpportunityBars(
      opp({
        primary_opportunity: 'NoClearOpportunity',
        long_expected_rr_internal: 2.51,
        short_expected_rr_internal: 1.21,
        opportunity_score: 0,
      }),
      'Bearish',
    );
    expect(bars.bearish).toBe(30);
    expect(bars.bullish).toBe(0);
    expect(bars.hold).toBe(70);
  });

  it('REGRESSION (real 1s sample): inverted geometry + StrongBullish + qualifying setup shows a modest bullish lean', () => {
    // The user's 1s BTC sample: TrendContinuation 3/3 preconditions,
    // geometry inverted (both R:R = 0), score 52.2, StrongBullish bias,
    // lean chip "Bullish setups dominate". The old implementation
    // emitted 0/0/100 — pure hold despite the panel's own bullish lean.
    const o = opp({
      primary_opportunity: 'TrendContinuation',
      opportunity_score: 52.22,
      long_expected_rr_internal: 0,
      short_expected_rr_internal: 0,
      profiles: [
        profile({
          opportunity_type: 'TrendContinuation',
          direction_family: 'TrendRiding',
          preconditions_met: 3,
          preconditions_total: 3,
          long_entry_zone: { low: 10, high: 11 },
          long_target_zone: { low: 12, high: 13 },
          long_invalidation_level: 9,
        }),
      ],
    });
    const bars = computeOpportunityBars(o, 'StrongBullish');
    // min(30, 52.22 × 0.5) = 26.1
    expect(bars.bullish).toBeGreaterThanOrEqual(25);
    expect(bars.bullish).toBeLessThanOrEqual(27);
    expect(bars.bearish).toBe(0);
    expect(bars.hold).toBe(100 - bars.bullish);
  });

  it('bias override beats the raw argmax R:R', () => {
    // long RR < short RR but the top qualifying profile + bullish bias
    // resolve LONG → bullish conviction only.
    const o = opp({
      opportunity_score: 78,
      long_expected_rr_internal: 1.0,
      short_expected_rr_internal: 2.5,
      profiles: [
        profile({
          opportunity_type: 'Breakout',
          direction_family: 'TrendRiding',
          preconditions_met: 2,
          preconditions_total: 2,
          long_entry_zone: { low: 10, high: 11 },
          long_target_zone: { low: 12, high: 13 },
          long_invalidation_level: 9,
        }),
      ],
    });
    const bars = computeOpportunityBars(o, 'Bullish');
    expect(bars.bullish).toBeGreaterThan(50);
    expect(bars.bearish).toBe(0);
    expect(bars.bullish + bars.bearish + bars.hold).toBe(100);
  });

  it('CounterTrend profile resolves its own deviation side (4b)', () => {
    // Bearish bias but the MeanReversion profile carries SHORT zones
    // (price stretched above its mean → sell the rip). The bars follow
    // the profile side, not the family × bias fallback (which would be
    // LONG under bearish).
    const o = opp({
      opportunity_score: 70,
      long_expected_rr_internal: 2.0,
      short_expected_rr_internal: 2.0,
      profiles: [
        profile({
          opportunity_type: 'MeanReversion',
          direction_family: 'CounterTrend',
          preconditions_met: 2,
          preconditions_total: 2,
          short_entry_zone: { low: 100.5, high: 101 },
          short_target_zone: { low: 98, high: 99 },
          short_invalidation_level: 102,
        }),
      ],
    });
    const bars = computeOpportunityBars(o, 'Bearish');
    expect(bars.bearish).toBeGreaterThan(50);
    expect(bars.bullish).toBe(0);
  });

  it('degenerate near-zero active R:R is treated as no bracket (B3)', () => {
    // The user's NO CLEAR SETUP sample: short_expected_rr_internal
    // = 0.0117. Below the floor → the geometry-inverted fallback rules
    // (no qualifying profile → pure hold).
    const o = opp({
      primary_opportunity: 'NoClearOpportunity',
      opportunity_score: 0,
      long_expected_rr_internal: 0,
      short_expected_rr_internal: 0.0117,
    });
    expect(computeOpportunityBars(o, 'StrongBearish')).toEqual({
      bullish: 0,
      bearish: 0,
      hold: 100,
    });
  });

  it('conviction is capped by opportunity_score', () => {
    const o = opp({
      opportunity_score: 40,
      long_expected_rr_internal: 3.0,
      short_expected_rr_internal: 0,
    });
    const bars = computeOpportunityBars(o, 'Bullish');
    expect(bars.bullish).toBeLessThanOrEqual(40);
    expect(bars.hold).toBeGreaterThanOrEqual(60);
    expect(bars.bullish + bars.bearish + bars.hold).toBe(100);
  });
});
