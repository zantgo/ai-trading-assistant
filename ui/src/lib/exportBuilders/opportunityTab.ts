// Opportunity tab builder — scoped export payload mirroring the panel.
//
// The Opportunities panel renders five sub-blocks:
//   1. Header (opportunity class, lean, setup score, setup quality)
//   2. Trade Setups (one card per qualifying profile with entry/target/SL/R:R)
//   3. R:R Internal (expected R:R, time horizon)
//   4. Evaluated Setups (every profile with score + preconditions)
//   5. Confluent Entry / Target Levels (top 4 each)
//   6. Market Position (bias, regime, trend, quality)
//   7. Environment (timeframes considered, confidence)

import type {
  OpportunityMatrix,
  OpportunityProfile,
  AnalysisMatrix,
  DecisionContext,
} from '../../types';
import {
  computeDecisionRank,
  selectProfileSide,
  profileZones,
} from '../../lib/decisionRank';
import { buildMeta } from './shared';
import type { MetaEnvelope, FilterStateBlock } from './shared';

// ── Payload types ────────────────────────────────────────────────────────

export interface OpportunityHeaderBlock {
  opportunity_class: string;
  lean: 'bullish_setups_dominate' | 'bearish_setups_dominate' | 'neutral';
  setup_score: number;
  setup_quality: 'PRIME' | 'STRONG' | 'MODERATE' | 'MARGINAL' | 'NONE';
}

export interface TradeSetupRow {
  opportunity_type: string;
  side: 'LONG' | 'SHORT';
  rank_idx: number;
  is_top: boolean;
  geometry_consistent: boolean;
  entry_mid: number | null;
  entry_zone: { low: number; high: number } | null;
  tp1: number;
  tp2: number;
  invalidation: number | null;
  rr: number | null;
  score: number;
  preconditions_met: number;
  preconditions_total: number;
  notes: string;
}

export interface RrInternalBlock {
  expected_rr: number | null;
  time_horizon: string;
}

export interface EvaluatedSetupRow {
  opportunity_type: string;
  score: number;
  preconditions_met: number;
  preconditions_total: number;
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
  confidence_pct: number;
}

export interface OpportunityPayload {
  source_tab: 'opportunity';
  meta: MetaEnvelope;
  header: OpportunityHeaderBlock;
  trade_setups: TradeSetupRow[];
  rr_internal: RrInternalBlock;
  invalidation_note: string;
  evaluated_setups: EvaluatedSetupRow[];
  confluent_entry_levels: ConfluentLevelRow[];
  confluent_target_levels: ConfluentLevelRow[];
  market_position: MarketPositionBlock;
  environment: EnvironmentBlock;
}

// ── Helpers ──────────────────────────────────────────────────────────────

function setupQuality(s: number): 'PRIME' | 'STRONG' | 'MODERATE' | 'MARGINAL' | 'NONE' {
  if (s >= 85) return 'PRIME';
  if (s >= 70) return 'STRONG';
  if (s >= 50) return 'MODERATE';
  if (s >= 30) return 'MARGINAL';
  return 'NONE';
}

function buildHeaderBlock(
  opportunity: OpportunityMatrix | null,
  analysis: AnalysisMatrix | null,
  lean: 'bullish_setups_dominate' | 'bearish_setups_dominate' | 'neutral',
): OpportunityHeaderBlock {
  return {
    opportunity_class: analysis?.opportunity_analysis ?? '—',
    lean,
    setup_score: opportunity?.opportunity_score ?? 0,
    setup_quality: setupQuality(opportunity?.opportunity_score ?? 0),
  };
}

function buildTradeSetups(
  opportunity: OpportunityMatrix | null,
  analysis: AnalysisMatrix | null,
  topAction: 'LONG' | 'SHORT' | 'HOLD',
): TradeSetupRow[] {
  if (!opportunity) return [];
  const profiles = opportunity.profiles ?? [];
  const qualifying = profiles
    .filter((p) => p.preconditions_met > 0 && p.opportunity_type !== 'NoClearOpportunity')
    .slice()
    .sort((a, b) => b.score - a.score);
  const macroBias = analysis?.bias ?? null;
  const out: TradeSetupRow[] = [];
  qualifying.forEach((p, idx) => {
    const side = selectProfileSide(p, macroBias);
    if (side === 'NEUTRAL') return;
    const z = profileZones(p, side);
    if (!z) return;
    const tpCandidates = [z.target.low, z.target.high].filter((v) => v > 0);
    const sortedTp = [...tpCandidates].sort(
      (a, b) => Math.abs(a - z.entry.low - ((z.entry.high - z.entry.low) / 2)) -
                Math.abs(b - z.entry.low - ((z.entry.high - z.entry.low) / 2)),
    );
    out.push({
      opportunity_type: p.opportunity_type,
      side,
      rank_idx: idx,
      is_top: idx === 0 && topAction !== 'HOLD',
      geometry_consistent: z.geometry_consistent,
      entry_mid: (z.entry.low + z.entry.high) / 2,
      entry_zone: { low: z.entry.low, high: z.entry.high },
      tp1: sortedTp[0] ?? 0,
      tp2: sortedTp.length > 1 ? sortedTp[1] : sortedTp[0] ?? 0,
      invalidation: z.invalidation,
      rr: z.rr,
      score: p.score,
      preconditions_met: p.preconditions_met,
      preconditions_total: p.preconditions_total,
      notes: p.notes,
    });
  });
  return out;
}

function buildEvaluatedSetups(opportunity: OpportunityMatrix | null): EvaluatedSetupRow[] {
  if (!opportunity?.profiles) return [];
  return opportunity.profiles.map((p) => ({
    opportunity_type: p.opportunity_type,
    score: p.score,
    preconditions_met: p.preconditions_met,
    preconditions_total: p.preconditions_total,
    notes: p.notes,
  }));
}

function buildConfluentLevels(
  levels: OpportunityMatrix['confluent_entry_levels'] | undefined,
): ConfluentLevelRow[] {
  if (!levels) return [];
  return levels.map((l) => ({
    price: l.price,
    sources: l.sources,
    strength: l.strength,
  }));
}

function buildMarketPosition(analysis: AnalysisMatrix | null): MarketPositionBlock {
  return {
    bias: analysis?.bias ?? '—',
    regime: analysis?.market_regime ?? '—',
    trend: analysis?.trend_assessment ?? '—',
    quality: analysis?.market_quality ?? '—',
  };
}

function buildEnvironment(analysis: AnalysisMatrix | null): EnvironmentBlock {
  return {
    timeframes_considered: analysis?.timeframes_considered ?? 0,
    confidence_pct: analysis ? Math.round(analysis.confidence * 100) : 0,
  };
}

function deriveLean(topAction: 'LONG' | 'SHORT' | 'HOLD'): 'bullish_setups_dominate' | 'bearish_setups_dominate' | 'neutral' {
  if (topAction === 'LONG') return 'bullish_setups_dominate';
  if (topAction === 'SHORT') return 'bearish_setups_dominate';
  return 'neutral';
}

// ── Public builder ───────────────────────────────────────────────────────

export interface OpportunityTabInputs {
  opportunity: OpportunityMatrix | null;
  analysis: AnalysisMatrix | null;
  decisionContext: DecisionContext | null;
  symbol: string;
  tfSecs?: number | null;
  timestamp?: number | null;
  markPrice?: number | null;
  filterState?: FilterStateBlock;
}

/**
 * Build the Opportunity tab export payload. Mirrors
 * `OpportunitiesPanel.svelte` 1:1.
 */
export function buildOpportunityTabExport(args: OpportunityTabInputs): string {
  const meta = buildMeta({
    sourceTab: 'opportunity',
    symbol: args.symbol,
    tfSecs: args.tfSecs ?? null,
    timestamp: args.timestamp ?? null,
    markPrice: args.markPrice ?? null,
    filterState: args.filterState,
  });
  const rank = computeDecisionRank({
    advisory: null,
    decisionContext: args.decisionContext,
    opportunity: args.opportunity,
    analysis: args.analysis,
  });
  const lean = deriveLean(rank.top);
  const opp = args.opportunity;
  // Active-side R:R (per-side, gated on macro bias). The legacy
  // matrix-level `opportunity.expected_rr_internal` was removed in
  // v6.9; we now read the per-side value that matches the active
  // bias. `null` when the opportunity matrix is absent.
  const activeSideRr = (() => {
    if (!opp) return null;
    const bias = args.analysis?.bias ?? 'Neutral';
    if (bias === 'Bullish' || bias === 'StrongBullish') {
      return opp.long_expected_rr_internal ?? 0;
    }
    if (bias === 'Bearish' || bias === 'StrongBearish') {
      return opp.short_expected_rr_internal ?? 0;
    }
    return 0;
  })();
  const payload: OpportunityPayload = {
    source_tab: 'opportunity',
    meta,
    header: buildHeaderBlock(opp, args.analysis, lean),
    trade_setups: buildTradeSetups(opp, args.analysis, rank.top),
    rr_internal: {
      expected_rr: activeSideRr,
      time_horizon: opp?.time_horizon ?? '—',
    },
    invalidation_note: opp?.invalidation_note ?? '',
    evaluated_setups: buildEvaluatedSetups(opp),
    confluent_entry_levels: buildConfluentLevels(opp?.confluent_entry_levels),
    confluent_target_levels: buildConfluentLevels(opp?.confluent_target_levels),
    market_position: buildMarketPosition(args.analysis),
    environment: buildEnvironment(args.analysis),
  };
  return JSON.stringify(payload, null, 2);
}
