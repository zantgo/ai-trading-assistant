import type { MarketBias, OpportunityMatrix } from '../types';
import { selectProfileSide, topQualifyingProfile } from './decisionRank';
// RR-001 (v6.10.12): the R:R floor is owned by `decisionRank` (shared with
// the R:R resolver + zones formula) and re-exported here for the bars.
import { RR_MEANINGFUL_FLOOR } from './decisionRank';

export { RR_MEANINGFUL_FLOOR };

export interface DirectionalBars {
  bullish: number;
  bearish: number;
  hold: number;
}

/**
 * Minimum directional conviction a valid active-side bracket always shows
 * (v6.10.12). The old hard cap (`conviction > score → conviction = score`)
 * collapsed a NO CLEAR SETUP matrix (score 0) with a real bracket to
 * 0/0/100 — the bars said "no lean" while the Recommendation gauge showed
 * a genuine directional distribution. A valid bracket now always carries
 * at least 30% directional conviction; scores ≥ 30 behave exactly as
 * before (still capped by the score).
 */
export const MIN_ACTIVE_FLOOR = 30;

export type EffectiveDirection = 'LONG' | 'SHORT' | 'NEUTRAL';

/**
 * The single effective direction of the L4 matrix — the same resolution
 * the L4 header, the profile cards, and the R:R displays use:
 *
 *   1. the top qualifying profile's resolved side (zone-presence aware
 *      `selectProfileSide` — deviation-driven for CounterTrend setups),
 *   2. the macro bias (Bullish → LONG, Bearish → SHORT).
 *
 * Under a Neutral (or absent) bias with no profile-side resolution the
 * direction is NEUTRAL (v6.10.15, FIX-1) — the legacy argmax of the two
 * per-side geometric R:R values lit the bars/badge directionally on a
 * directionally-neutral panel (57% "bearish" beside a DirectionalNeutral
 * card, `Lean: neutral`, and N/A R:R) and contradicted the L6 HOLD
 * verdict. Bracket geometry stays visible in the setup cards and
 * confluent levels; only the directional-conviction surfaces go neutral.
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
  return 'NEUTRAL';
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
 * Semantics (v6.10.6+; floor added v6.10.12):
 *   - The direction is the panel's own effective direction
 *     (`resolveEffectiveDirection`) — never the bare argmax of both
 *     sides' raw R:R, which previously lit the bars BULLISH under a
 *     bearish panel whenever the countertrend long bracket happened to
 *     have the larger ratio.
 *   - Conviction comes from the ACTIVE side's R:R only, exp-weighted
 *     (`exp(RR·3)`) against a hold floor (`exp(0.25)`), capped by
 *     `opportunity_score` — floored at `MIN_ACTIVE_FLOOR` so a valid
 *     active-side bracket always carries visible directional conviction,
 *     even when the primary is NO CLEAR SETUP (score 0).
 *   - When the active side has no valid bracket (geometry inverted /
 *     degenerate) but the bias is directional and a qualifying setup
 *     exists, a modest directional lean (`min(30, score·0.5)`) keeps
 *     the bars aligned with the panel's "Bullish/Bearish setups
 *     dominate" chip without overstating a non-actionable setup.
 *   - Everything else (no opportunity, neutral direction, nothing
 *     qualifying) renders pure Hold.
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
    // Conviction from the active-side R:R, floored so a real bracket
    // always shows visible directional conviction (even under NO CLEAR
    // SETUP with score 0) and capped by the setup score when the score
    // exceeds the floor — the remaining uncertainty stays visible as a
    // Hold buffer.
    // The wire R:R is unbounded; cap the exponent input so a pathological
    // ratio (≥ ~237) cannot overflow exp() to Infinity → NaN bars.
    const wDir = Math.exp(Math.min(activeRR, 200) * 3);
    const wHold = Math.exp(0.25);
    const rawConviction = (wDir / (wDir + wHold)) * 100;
    const conviction = Math.min(rawConviction, Math.max(score, MIN_ACTIVE_FLOOR));
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

// ── Trade Setups folder ranking ─────────────────────────────────────────

export interface RankedSectionInput<T> {
  key: 'NEUTRAL' | 'BULL' | 'BEAR';
  label: string;
  /** The section's setup rows (references NOT included — counted via `hasReference`). */
  setups: T[];
  /** `true` when a reference bracket occupies the folder instead of setups. */
  hasReference: boolean;
  /** Per-row score used for the top-row tie-break (references score 0). */
  scoreOf: (s: T) => number;
}

/**
 * Rank the three Trade Setups folders (RANGE / BULLISH / BEARISH) the way
 * the directional conviction bars are ordered — the most relevant folder
 * first, always populated folders above empty ones. Shared by the panel
 * and the export so the screen and the clipboard can never disagree.
 * Callers may pass extra fields (tone, empty label, …) — they ride along
 * on the returned sections.
 *
 *   1. Folder content count DESC — real setup cards plus one for a
 *      reference bracket that occupies the folder (mirrors the folder
 *      counter: `setups.length + (reference ? 1 : 0)`).
 *   2. Top row score DESC (references carry score 0).
 *   3. Fixed fallback order NEUTRAL → BULL → BEAR (the empty-state
 *      layout is unchanged).
 */
export function rankSectionsByCount<S extends RankedSectionInput<S['setups'][number]>>(sections: S[]): S[] {
  const fixed: Record<RankedSectionInput<S['setups'][number]>['key'], number> = { NEUTRAL: 0, BULL: 1, BEAR: 2 };
  return sections
    .map((s) => ({ ...s, rank: s.setups.length + (s.hasReference ? 1 : 0) }))
    .sort((a, b) => {
      if (b.rank !== a.rank) return b.rank - a.rank;
      const ta = a.setups[0] ? a.scoreOf(a.setups[0]) : 0;
      const tb = b.setups[0] ? b.scoreOf(b.setups[0]) : 0;
      if (tb !== ta) return tb - ta;
      return fixed[a.key] - fixed[b.key];
    });
}
