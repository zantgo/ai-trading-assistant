// Opportunity tab builder — scoped export payload mirroring the panel.
//
// v7.0-audit: rewrites the payload to use the new shared envelope (no
// filter_state, single current_price, structured header chrome) and the
// new R:R availability helper. Adds the missing visual blocks
// (directional bars, no-clear strip, hold scenario note, viability).

import type {
  OpportunityMatrix,
  OpportunityProfile,
  AnalysisMatrix,
  AdvisoryMatrix,
  DecisionContext,
  MarketBias,
} from '../../types';
import {
  computeDecisionRank,
  profileSummary,
  resolveActiveRr,
} from '../../lib/decisionRank';
import {
  buildPriceBlock,
  buildHeaderBlock,
  buildRrBlock,
  type MetaEnvelope,
  type HeaderBlock,
  type InstanceTermsLike,
} from './shared';
import type { LayerHeaderSpec } from '../layerHeader';
import { computeOpportunityBars } from '../../lib/opportunityBars';
import { LEVEL_SOURCE_ABBREV } from '../levelSourceAbbrev';

// ── Payload types ────────────────────────────────────────────────────────

export interface OpportunityHeaderBlock {
  opportunity_class: string;
  lean: string;
  setup_score: number;
  setup_quality: string;
}

export interface TradeSetupRow {
  opportunity_type: string;
  viability: string;
  badge_text: string;
  side: 'LONG' | 'SHORT' | 'NEUTRAL';
  rank_idx: number;
  is_top: boolean;
  geometry_consistent: boolean;
  entry_mid: number | null;
  entry_zone: { low: number; high: number } | null;
  tp1: number;
  tp2: number;
  invalidation: number | null;
  rr_available: boolean;
  rr_value: number | null;
  rr_reason: string | null;
  score: number;
  preconditions_met: number;
  preconditions_total: number;
  notes: string;
}

export interface RrInternalBlock {
  expected_rr_available: boolean;
  expected_rr_value: number | null;
  expected_rr_reason: string | null;
  time_horizon: string;
}

export interface EvaluatedSetupRow {
  opportunity_type: string;
  viability: string;
  score: number;
  preconditions_met: number;
  preconditions_total: number;
  trade_viability: string | null;
  notes: string;
}

export interface ConfluentLevelRow {
  price: number;
  sources: string[];
  strength: number;
}

export interface MarketPositionBlock {
  bias: string;
  regime: string;
  trend: string;
  quality: string;
}

export interface EnvironmentBlock {
  timeframes_considered: number;
  timeframes_considered_display: string;
  confidence_pct: number;
  /** Panel-format string mirroring `Confidence: N%`. */
  confidence_display: string;
}

export interface DirectionalBarsBlock {
  bullish_pct: number;
  bearish_pct: number;
  hold_pct: number;
  sort: 'desc';
}

export interface NoClearStripBlock {
  badge: string;
  preconditions_met: number;
  preconditions_total: number;
  meta: string;
}

export interface OpportunityPayload {
  source_tab: 'opportunity';
  meta: MetaEnvelope;
  header: HeaderBlock;
  directional_bars: DirectionalBarsBlock;
  header_block: OpportunityHeaderBlock;
  trade_setups: TradeSetupRow[];
  no_clear_strip: NoClearStripBlock | null;
  hold_scenario_note: string | null;
  rr_internal: RrInternalBlock;
  invalidation_note: string;
  evaluated_setups: EvaluatedSetupRow[];
  confluent_entry_levels: ConfluentLevelRow[];
  confluent_target_levels: ConfluentLevelRow[];
  market_position: MarketPositionBlock;
  environment: EnvironmentBlock;
}

// ── Helpers ──────────────────────────────────────────────────────────────

function setupQuality(s: number): string {
  if (s >= 85) return 'PRIME';
  if (s >= 70) return 'STRONG';
  if (s >= 50) return 'MODERATE';
  if (s >= 30) return 'MARGINAL';
  return 'NONE';
}

function prettifyOpportunityType(raw: string): string {
  // "TrendContinuation" → "Trend Continuation"
  return raw
    .replace(/([a-z])([A-Z])/g, '$1 $2')
    .replace(/_/g, ' ');
}

function deriveLean(topAction: 'LONG' | 'SHORT' | 'HOLD'): string {
  if (topAction === 'LONG') return 'Bullish setups dominate';
  if (topAction === 'SHORT') return 'Bearish setups dominate';
  return 'Lean: neutral';
}

function buildOpportunityHeaderBlock(
  opportunity: OpportunityMatrix | null,
  lean: string,
): OpportunityHeaderBlock {
  return {
    // Screen badge renders the L4 primary opportunity class
    // ("Trend Continuation"); "—" when absent. The L3 legacy
    // `analysis.opportunity_analysis` label is intentionally NOT used —
    // it could contradict the L4 verdict (e.g. "Trend Continuation"
    // under a NO CLEAR SETUP badge).
    opportunity_class: opportunity?.primary_opportunity
      ? prettifyOpportunityType(opportunity.primary_opportunity)
      : '\u2014',
    lean,
    setup_score: opportunity?.opportunity_score ?? 0,
    setup_quality: setupQuality(opportunity?.opportunity_score ?? 0),
  };
}

function setupBadgeText(viability: string, isTop: boolean): string {
  // Mirrors `OpportunitiesPanel.svelte` badge branches — the screen shows
  // NO badge for plain Actionable cards (only TOP · ACTIONABLE,
  // NEUTRAL · HOLD and GEOMETRY INVERTED are rendered).
  if (viability === 'Actionable' && isTop) return 'TOP · ACTIONABLE';
  if (viability === 'DirectionalNeutral') return 'NEUTRAL · HOLD';
  if (viability === 'GeometryInverted') return 'GEOMETRY INVERTED';
  return '';
}

const SETUP_VIABILITY_RANK: Record<string, number> = {
  Actionable: 0,
  DirectionalNeutral: 1,
  GeometryInverted: 2,
  NoClear: 3,
};

function buildTradeSetups(
  opportunity: OpportunityMatrix | null,
  analysis: AnalysisMatrix | null,
  decisionContext: DecisionContext | null,
  topAction: 'LONG' | 'SHORT' | 'HOLD',
): TradeSetupRow[] {
  if (!opportunity) return [];
  const profiles = opportunity.profiles ?? [];
  const qualifying = profiles
    .filter((p) => p.preconditions_met > 0 && p.opportunity_type !== 'NoClearOpportunity')
    .slice()
    .sort((a, b) => b.score - a.score);
  const out: TradeSetupRow[] = [];
  qualifying.forEach((p, idx) => {
    // Mirrors the panel's `activeSetups` derivation exactly: profileSummary
    // (aggregate-bracket fallback + wire R:R preference), NEUTRAL-side
    // cards included, geometry from the resolved zones.
    const s = profileSummary(p, opportunity, analysis, decisionContext);
    const z = s.zones;
    const tpCandidates = z ? [z.target.low, z.target.high].filter((v) => v > 0) : [];
    const sortedTp = z
      ? [...tpCandidates].sort(
          (a, b) =>
            Math.abs(a - z.entry.low - ((z.entry.high - z.entry.low) / 2)) -
            Math.abs(b - z.entry.low - ((z.entry.high - z.entry.low) / 2)),
        )
      : [];
    const rr = buildRrBlock(s.rr, 'no_actionable_geometry');
    const isTop = idx === 0 && topAction !== 'HOLD';
    out.push({
      opportunity_type: prettifyOpportunityType(p.opportunity_type),
      viability: s.viability,
      badge_text: setupBadgeText(s.viability, isTop),
      side: s.side,
      rank_idx: idx,
      is_top: isTop,
      geometry_consistent: z?.geometry_consistent ?? false,
      entry_mid: z ? (z.entry.low + z.entry.high) / 2 : null,
      entry_zone: z ? { low: z.entry.low, high: z.entry.high } : null,
      tp1: sortedTp[0] ?? 0,
      tp2: sortedTp.length > 1 ? sortedTp[1] : sortedTp[0] ?? 0,
      invalidation: z?.invalidation ?? null,
      rr_available: rr.available,
      rr_value: rr.value,
      rr_reason: rr.reason,
      score: p.score,
      preconditions_met: p.preconditions_met,
      preconditions_total: p.preconditions_total,
      // Panel stores the raw wire notes and renders them verbatim.
      notes: p.notes,
    });
  });
  // Viability ordering (Actionable first), then score desc — panel order.
  return out.sort((a, b) => {
    const va = SETUP_VIABILITY_RANK[a.viability] ?? 3;
    const vb = SETUP_VIABILITY_RANK[b.viability] ?? 3;
    if (va !== vb) return va - vb;
    return b.score - a.score;
  });
}

function buildNoClearStrip(opportunity: OpportunityMatrix | null): NoClearStripBlock | null {
  if (!opportunity?.profiles) return null;
  const noClear = opportunity.profiles.find((p) => p.opportunity_type === 'NoClearOpportunity');
  if (!noClear) return null;
  return {
    badge: 'NO CLEAR OPPORTUNITY',
    preconditions_met: noClear.preconditions_met,
    preconditions_total: noClear.preconditions_total,
    meta: `${noClear.preconditions_met}/${noClear.preconditions_total} preconditions met · informational only`,
  };
}

function buildHoldScenarioNote(topAction: 'LONG' | 'SHORT' | 'HOLD'): string | null {
  if (topAction !== 'HOLD') return null;
  // Mirrors the screen scenario note (badge + body) verbatim.
  return `HOLD / NO CLEAR — No directional call. The cards below show each qualifying profile's aggregated bracket — when geometry is inverted (entry/target/SL on the wrong side of close, or zero-bound contamination), R:R reads N/A and the bracket is non-actionable. None are active.`;
}

function buildEvaluatedSetups(opportunity: OpportunityMatrix | null): EvaluatedSetupRow[] {
  if (!opportunity?.profiles) return [];
  // The screen's Evaluated Setups list excludes the NoClearOpportunity
  // profile — it has its own placeholder strip.
  return opportunity.profiles
    .filter((p) => p.opportunity_type !== 'NoClearOpportunity')
    .map((p) => ({
      opportunity_type: prettifyOpportunityType(p.opportunity_type),
      viability: p.trade_viability ?? 'NoClear',
      score: p.score,
      preconditions_met: p.preconditions_met,
      preconditions_total: p.preconditions_total,
      // Keep the raw wire value so consumers can round-trip the enum;
      // panel renders verbatim.
      trade_viability: p.trade_viability ?? null,
      // Panel renders the raw wire notes verbatim.
      notes: p.notes,
    }));
}

function buildConfluentLevels(
  levels: OpportunityMatrix['confluent_entry_levels'] | undefined,
): ConfluentLevelRow[] {
  if (!levels) return [];
  return levels.map((l) => ({
    price: l.price,
    // Screen `fmtSource` defaults unknown tokens to "ATR" — mirror it.
    sources: l.sources.map((s) => LEVEL_SOURCE_ABBREV[s] ?? 'ATR'),
    strength: l.strength,
  }));
}

function buildMarketPosition(analysis: AnalysisMatrix | null): MarketPositionBlock {
  // Screen renders "—" for missing fields.
  return {
    bias: analysis?.bias ?? '\u2014',
    regime: analysis?.market_regime ?? '\u2014',
    trend: analysis?.trend_assessment ?? '\u2014',
    quality: analysis?.market_quality ?? '\u2014',
  };
}

function buildEnvironment(analysis: AnalysisMatrix | null): EnvironmentBlock {
  const tf = analysis?.timeframes_considered ?? 0;
  const confidencePct = analysis ? Math.round(analysis.confidence * 100) : 0;
  return {
    timeframes_considered: tf,
    timeframes_considered_display: `${tf}/4 TFs considered`,
    confidence_pct: confidencePct,
    confidence_display: analysis ? `${confidencePct}%` : '\u2014',
  };
}

function buildDirectionalBars(
  opportunity: OpportunityMatrix | null,
  bias: MarketBias | null,
): DirectionalBarsBlock {
  // The panel ALWAYS renders all three bars — even when the matrix is
  // absent `computeOpportunityBars` yields the 0/0/100 HOLD-dominant
  // split. Direction resolution (top profile side → bias → argmax R:R)
  // mirrors the panel exactly so the export can never disagree with
  // the screen.
  const bars = computeOpportunityBars(opportunity, bias);
  return {
    bullish_pct: bars.bullish,
    bearish_pct: bars.bearish,
    hold_pct: bars.hold,
    sort: 'desc',
  };
}

// ── Public builder ───────────────────────────────────────────────────────

export interface OpportunityTabInputs {
  opportunity: OpportunityMatrix | null;
  analysis: AnalysisMatrix | null;
  decisionContext: DecisionContext | null;
  advisory?: AdvisoryMatrix | null;
  symbol: string;
  exchange?: string;
  tfSecs?: number | null;
  timestamp?: number | null;
  markPrice?: number | null;
  isCompleted?: boolean;
  terms?: InstanceTermsLike;
  headerSpec: LayerHeaderSpec;
}

/**
 * Build the Opportunity tab export payload. Mirrors
 * `OpportunitiesPanel.svelte` 1:1.
 */
export function buildOpportunityTabExport(args: OpportunityTabInputs): string {
  const { meta } = buildPriceBlock({
    symbol: args.symbol,
    exchange: args.exchange,
    terms: args.terms,
    fallbackMarkPrice: args.markPrice,
    tfSecs: args.tfSecs,
    timestamp: args.timestamp,
    isCompleted: args.isCompleted,
  });
  // The panel computes its rank with the real advisory — the export must
  // pass the same inputs or lean/top/badges would drift from the screen.
  const rank = computeDecisionRank({
    advisory: args.advisory ?? null,
    decisionContext: args.decisionContext,
    opportunity: args.opportunity,
    analysis: args.analysis,
  });
  const lean = deriveLean(rank.top);
  const opp = args.opportunity;
  const activeSideRr = (() => {
    if (!opp) return null;
    // RR-002 (v6.10.12): mirror the panel's R:R (Internal) block through
    // the shared resolver — the same chain (profile wire → matrix wire →
    // aligned zones fallback) the screen uses.
    const resolved = resolveActiveRr(opp, args.decisionContext, args.analysis);
    return resolved.available ? resolved.value : 0;
  })();
  const expectedRrBlock =
    rank.top === 'HOLD' && (activeSideRr === null || activeSideRr === 0)
      ? { available: false as const, value: null, reason: 'no_directional_bias' as string | null }
      : { available: true as const, value: activeSideRr ?? 0, reason: null as string | null };
  const payload: OpportunityPayload = {
    source_tab: 'opportunity',
    meta,
    header: buildHeaderBlock(args.headerSpec),
    directional_bars: buildDirectionalBars(opp, (args.decisionContext?.bias ?? args.analysis?.bias) ?? null),
    header_block: buildOpportunityHeaderBlock(opp, lean),
    trade_setups: buildTradeSetups(opp, args.analysis, args.decisionContext, rank.top),
    no_clear_strip: buildNoClearStrip(opp),
    hold_scenario_note: buildHoldScenarioNote(rank.top),
    rr_internal: {
      expected_rr_available: expectedRrBlock.available,
      expected_rr_value: expectedRrBlock.value,
      expected_rr_reason: expectedRrBlock.reason,
      // Screen renders "—" when the horizon is absent.
      time_horizon: opp?.time_horizon ?? '\u2014',
    },
    invalidation_note:
      opp?.invalidation_note ??
      'Assessment conditions forming — invalidation level will be calculated when structural pivot confirms.',
    evaluated_setups: buildEvaluatedSetups(opp),
    // The screen renders at most the first 4 entry + 4 target levels.
    confluent_entry_levels: buildConfluentLevels(opp?.confluent_entry_levels).slice(0, 4),
    confluent_target_levels: buildConfluentLevels(opp?.confluent_target_levels).slice(0, 4),
    market_position: buildMarketPosition(args.analysis),
    environment: buildEnvironment(args.analysis),
  };
  return JSON.stringify(payload, null, 2);
}

// Silence unused-import warning
export type { OpportunityProfile };