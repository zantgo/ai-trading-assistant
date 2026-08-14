import type { MarketBias, OpportunityMatrix } from '../types';
import { selectProfileSide, topQualifyingProfile } from './decisionRank';

export interface DirectionalBars {
  bullish: number;
  bearish: number;
  hold: number;
}

/**
 * Minimum economically meaningful R:R. Ratios below this floor (e.g.
 * `0.0117` — reward ≈ 1% of risk) are degenerate noise; the backend
 * rejects them with `NoValueReason::RatioBelowFloor`, and this frontend
 * floor is the second layer of defence for stale snapshots from older
 * engine builds.
 */
export const RR_MEANINGFUL_FLOOR = 0.1;

export type EffectiveDirection = 'LONG' | 'SHORT' | 'NEUTRAL';

/**
 * The single effective direction of the L4 matrix — the same resolution
 * the L4 header, the profile cards, and the R:R displays use:
 *
 *   1. the top qualifying profile's resolved side (zone-presence aware
 *      `selectProfileSide` — deviation-driven for CounterTrend setups),
 *   2. the macro bias (Bullish → LONG, Bearish → SHORT),
 *   3. the argmax of the per-side geometric R:R.
 *
 * One resolution, shared everywhere, so the bull/bear/hold bars can
 * never contradict the panel that renders them.
 */
export function resolveEffectiveDirection(
  opp: OpportunityMatrix | null | undefined,
  bias: MarketBias | null | undefined,
): EffectiveDirection {
  if (!opp) return 'NEUTRAL';
  const top = topQualifyingProfile(opp);
  if (top) {
    const side = selectProfileSide(top, bias ?? null);
    if (side !== 'NEUTRAL') return side;
  }
  if (bias === 'Bullish' || bias === 'StrongBullish') return 'LONG';
  if (bias === 'Bearish' || bias === 'StrongBearish') return 'SHORT';
  const longRR = Math.max(0, opp.long_expected_rr_internal ?? 0);
  const shortRR = Math.max(0, opp.short_expected_rr_internal ?? 0);
  if (longRR === shortRR) return 'NEUTRAL';
  return longRR > shortRR ? 'LONG' : 'SHORT';
}

function clamp(x: number, lo: number, hi: number): number {
  return Math.min(hi, Math.max(lo, x));
}

function roundBars(bullish: number, bearish: number): DirectionalBars {
  const rounded = {
    bullish: Math.round(bullish),
    bearish: Math.round(bearish),
    hold: Math.round(100 - bullish - bearish),
  };
  const sum = rounded.bullish + rounded.bearish + rounded.hold;
  if (sum !== 100) {
    rounded.hold += 100 - sum;
  }
  return rounded;
}

/**
 * Directional conviction bars (bullish / bearish / hold) for the
 * Opportunities panel.
 *
 * Semantics (v6.10.6+):
 *   - The direction is the panel's own effective direction
 *     (`resolveEffectiveDirection`) — never the bare argmax of both
 *     sides' raw R:R, which previously lit the bars BULLISH under a
 *     bearish panel whenever the countertrend long bracket happened to
 *     have the larger ratio.
 *   - Conviction comes from the ACTIVE side's R:R only, exp-weighted
 *     (`exp(RR·3)`) against a hold floor (`exp(0.25)`), and capped by
 *     `opportunity_score` so the remaining uncertainty stays visible as
 *     a Hold buffer.
 *   - When the active side has no valid bracket (geometry inverted /
 *     degenerate) but the bias is directional and a qualifying setup
 *     exists, a modest directional lean (`min(30, score·0.5)`) keeps
 *     the bars aligned with the panel's "Bullish/Bearish setups
 *     dominate" chip without overstating a non-actionable setup.
 *   - Everything else (no opportunity, neutral direction, nothing
 *     qualifying) renders pure Hold — including the NO CLEAR SETUP case
 *     where `opportunity_score = 0` caps any residual conviction to 0.
 *
 * All three bars are ALWAYS rendered, even at 0% — the previous
 * behaviour filtered out zero-value bars which hid the dominant-HOLD
 * case.
 */
export function computeOpportunityBars(
  opp: OpportunityMatrix | null,
  bias?: MarketBias | null,
): DirectionalBars {
  if (!opp) return { bullish: 0, bearish: 0, hold: 100 };

  const score = clamp(opp.opportunity_score ?? 0, 0, 100);
  const dir = resolveEffectiveDirection(opp, bias ?? null);
  if (dir === 'NEUTRAL') return { bullish: 0, bearish: 0, hold: 100 };

  const activeRR = Math.max(
    0,
    dir === 'LONG'
      ? (opp.long_expected_rr_internal ?? 0)
      : (opp.short_expected_rr_internal ?? 0),
  );

  if (activeRR >= RR_MEANINGFUL_FLOOR) {
    const wDir = Math.exp(activeRR * 3);
    const wHold = Math.exp(0.25);
    let conviction = (wDir / (wDir + wHold)) * 100;
    if (conviction > score) conviction = score;
    return dir === 'LONG' ? roundBars(conviction, 0) : roundBars(0, conviction);
  }

  const biasDirectional =
    bias === 'Bullish' || bias === 'StrongBullish' || bias === 'Bearish' || bias === 'StrongBearish';
  if (biasDirectional && topQualifyingProfile(opp)) {
    const lean = Math.min(30, score * 0.5);
    return dir === 'LONG' ? roundBars(lean, 0) : roundBars(0, lean);
  }
  return { bullish: 0, bearish: 0, hold: 100 };
}
