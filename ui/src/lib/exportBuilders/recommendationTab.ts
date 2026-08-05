// Recommendation tab builder — scoped export payload mirroring the panel.
//
// The Recommendation panel renders:
//   1. Environment header (directional_guidance, market_stance, strategy_environment,
//      opportunity_classification, confidence, readiness, entry_danger)
//   2. Verdict hero (TOP CALL + headline + long/short/hold probabilities)
//   3. Runner-ups (winner excluded, sorted by probability desc)
//   4. Top Setup card (highest-scored qualifying profile with zones)
//   5. Safety Flags (readiness, internal R:R, risk-adj R:R, stop-loss %, confidence)
//   6. Why (top-3 rationale bullets)
//   7. Price Levels (per-direction; both scenarios when HOLD)
//   8. Strategy (entry, exit, protection, target)
//   9. Final Verdict (final_recommendation text)

import type {
  AdvisoryMatrix,
  AnalysisMatrix,
  DecisionContext,
  OpportunityMatrix,
  RiskDimension,
} from '../../types';
import {
  computeDecisionRank,
  selectProfileSide,
  profileZones,
} from '../../lib/decisionRank';
import { buildMeta } from './shared';
import type { MetaEnvelope, FilterStateBlock } from './shared';

// ── Payload types ────────────────────────────────────────────────────────

export interface RecommendationEnvironmentBlock {
  directional_guidance: string;
  market_stance: string;
  strategy_environment: string;
  opportunity_classification: string;
  confidence_pct: number;
  readiness: string;
  entry_danger: {
    score: number;
    level: string;
    state: string;
    confidence: number;
  };
}

export interface RecommendationVerdictBlock {
  top: 'LONG' | 'SHORT' | 'HOLD';
  top_prob_pct: number;
  headline: {
    action: string;
    label: string;
    state: string;
    confidence_pct: number;
  };
  long_probability: number;
  short_probability: number;
  hold_probability: number;
}

export interface RunnerUpRow {
  action: 'LONG' | 'SHORT' | 'HOLD';
  prob_pct: number;
}

export interface TopSetupBlock {
  opportunity_type: string;
  score: number;
  preconditions_met: number;
  preconditions_total: number;
  direction: 'long' | 'short' | 'neutral';
  direction_label: string;
  entry_zone: { low: number; high: number } | null;
  target_zone: { low: number; high: number } | null;
  invalidation: number | null;
  rr: number | null;
  notes: string;
}

export interface SafetyFlagsBlock {
  readiness: string;
  internal_rr: number;
  risk_adj_rr: number;
  stop_loss_pct: number;
  confidence_pct: number;
}

export interface PriceLevelsBlock {
  side: 'long' | 'short' | 'hold';
  entry_zone: { low: number; high: number } | null;
  target_zone: { low: number; high: number } | null;
  invalidation: number | null;
  horizon: string;
  scenarios: {
    long: { entry_zone: { low: number; high: number } | null; target_zone: { low: number; high: number } | null; invalidation: number | null };
    short: { entry_zone: { low: number; high: number } | null; target_zone: { low: number; high: number } | null; invalidation: number | null };
  } | null;
}

export interface StrategyBlock {
  entry: string;
  exit: string;
  protection: string;
  target: string;
}

export interface RecommendationPayload {
  source_tab: 'recommendation';
  meta: MetaEnvelope;
  environment: RecommendationEnvironmentBlock;
  verdict: RecommendationVerdictBlock;
  runner_ups: RunnerUpRow[];
  top_setup: TopSetupBlock | null;
  safety_flags: SafetyFlagsBlock;
  why: string[];
  price_levels: PriceLevelsBlock;
  strategy: StrategyBlock;
  final_verdict: string;
}

// ── Helpers ──────────────────────────────────────────────────────────────

function readEntryDanger(decisionContext: DecisionContext | null): {
  score: number;
  level: string;
  state: string;
  confidence: number;
} {
  if (!decisionContext) return { score: 0, level: 'UNKNOWN', state: 'UNKNOWN', confidence: 0 };
  const danger = decisionContext.entry_danger;
  if (typeof danger === 'number') {
    return { score: danger, level: 'UNKNOWN', state: 'UNKNOWN', confidence: 0 };
  }
  const d = danger as RiskDimension;
  return {
    score: d?.score ?? 0,
    level: d?.level ?? 'UNKNOWN',
    state: d?.state ?? 'UNKNOWN',
    confidence: d?.confidence ?? 0,
  };
}

function buildEnvironmentBlock(
  advisory: AdvisoryMatrix | null,
  decisionContext: DecisionContext | null,
  readiness: string,
): RecommendationEnvironmentBlock {
  const danger = readEntryDanger(decisionContext);
  return {
    directional_guidance: advisory?.directional_guidance ?? '—',
    market_stance: advisory?.market_stance ?? '—',
    strategy_environment: advisory?.strategy_environment ?? '—',
    opportunity_classification: advisory?.opportunity_classification ?? '—',
    confidence_pct: advisory?.confidence_assessment ?? 0,
    readiness,
    entry_danger: danger,
  };
}

function buildVerdictBlock(
  rank: ReturnType<typeof computeDecisionRank>,
): RecommendationVerdictBlock {
  return {
    top: rank.top,
    top_prob_pct: rank.top_prob,
    headline: {
      action: rank.headline.action,
      label: rank.headline.label,
      state: rank.headline.state,
      confidence_pct: rank.headline.confidence_pct,
    },
    long_probability: rank.long.probability,
    short_probability: rank.short.probability,
    hold_probability: rank.hold.probability,
  };
}

function buildRunnerUpsBlock(
  rank: ReturnType<typeof computeDecisionRank>,
): RunnerUpRow[] {
  const all: RunnerUpRow[] = [
    { action: 'LONG', prob_pct: rank.long.probability },
    { action: 'SHORT', prob_pct: rank.short.probability },
    { action: 'HOLD', prob_pct: rank.hold.probability },
  ];
  return all
    .filter((r) => r.action !== rank.top)
    .sort((a, b) => b.prob_pct - a.prob_pct);
}

function buildTopSetupBlock(
  opportunity: OpportunityMatrix | null,
  analysis: AnalysisMatrix | null,
): TopSetupBlock | null {
  if (!opportunity) return null;
  const profiles = opportunity.profiles ?? [];
  const qualifying = profiles
    .filter((p) => p.preconditions_met > 0 && p.opportunity_type !== 'NoClearOpportunity')
    .slice()
    .sort((a, b) => b.score - a.score);
  const top = qualifying[0];
  if (!top) return null;
  const side = selectProfileSide(top, analysis?.bias ?? null);
  const cardDir: 'long' | 'short' | 'neutral' =
    side === 'LONG' ? 'long' : side === 'SHORT' ? 'short' : 'neutral';
  const directionLabel = cardDir === 'long' ? 'LONG' : cardDir === 'short' ? 'SHORT' : 'NEUTRAL';
  const zones = side === 'NEUTRAL' ? null : profileZones(top, side);
  if (!zones) {
    return {
      opportunity_type: top.opportunity_type,
      score: top.score,
      preconditions_met: top.preconditions_met,
      preconditions_total: top.preconditions_total,
      direction: cardDir,
      direction_label: directionLabel,
      entry_zone: null,
      target_zone: null,
      invalidation: null,
      rr: null,
      notes: top.notes,
    };
  }
  return {
    opportunity_type: top.opportunity_type,
    score: top.score,
    preconditions_met: top.preconditions_met,
    preconditions_total: top.preconditions_total,
    direction: cardDir,
    direction_label: directionLabel,
    entry_zone: { low: zones.entry.low, high: zones.entry.high },
    target_zone: { low: zones.target.low, high: zones.target.high },
    invalidation: zones.invalidation,
    rr: zones.rr,
    notes: top.notes,
  };
}

function buildSafetyFlagsBlock(
  opportunity: OpportunityMatrix | null,
  decisionContext: DecisionContext | null,
  advisory: AdvisoryMatrix | null,
  readiness: string,
  activeSideRr: number,
): SafetyFlagsBlock {
  return {
    readiness,
    internal_rr: activeSideRr,
    risk_adj_rr: decisionContext?.expected_reward_risk_ratio ?? 0,
    stop_loss_pct: advisory?.stop_loss_distance_pct ?? 0,
    confidence_pct: advisory?.confidence_assessment ?? 0,
  };
}

function buildPriceLevelsBlock(
  opportunity: OpportunityMatrix | null,
  topAction: 'LONG' | 'SHORT' | 'HOLD',
): PriceLevelsBlock {
  if (topAction === 'LONG' || topAction === 'SHORT') {
    const side = topAction === 'LONG'
      ? {
          entry: opportunity?.long_entry_zone,
          target: opportunity?.long_target_zone,
          inval: opportunity?.long_invalidation_level,
        }
      : {
          entry: opportunity?.short_entry_zone,
          target: opportunity?.short_target_zone,
          inval: opportunity?.short_invalidation_level,
        };
    return {
      side: topAction === 'LONG' ? 'long' : 'short',
      entry_zone: side.entry ?? null,
      target_zone: side.target ?? null,
      invalidation: side.inval ?? null,
      horizon: opportunity?.time_horizon ?? '—',
      scenarios: null,
    };
  }
  return {
    side: 'hold',
    entry_zone: null,
    target_zone: null,
    invalidation: null,
    horizon: opportunity?.time_horizon ?? '—',
    scenarios: {
      long: {
        entry_zone: opportunity?.long_entry_zone ?? null,
        target_zone: opportunity?.long_target_zone ?? null,
        invalidation: opportunity?.long_invalidation_level ?? null,
      },
      short: {
        entry_zone: opportunity?.short_entry_zone ?? null,
        target_zone: opportunity?.short_target_zone ?? null,
        invalidation: opportunity?.short_invalidation_level ?? null,
      },
    },
  };
}

function buildStrategyBlock(advisory: AdvisoryMatrix | null): StrategyBlock {
  return {
    entry: advisory?.entry_guidance ?? '—',
    exit: advisory?.exit_guidance ?? '—',
    protection: advisory?.protection_strategy ?? '—',
    target: advisory?.target_strategy ?? '—',
  };
}

// ── Public builder ───────────────────────────────────────────────────────

export interface RecommendationTabInputs {
  advisory: AdvisoryMatrix | null;
  decisionContext: DecisionContext | null;
  opportunity: OpportunityMatrix | null;
  analysis: AnalysisMatrix | null;
  symbol: string;
  tfSecs?: number | null;
  timestamp?: number | null;
  markPrice?: number | null;
  filterState?: FilterStateBlock;
}

/**
 * Build the Recommendation tab export payload. Mirrors
 * `RecommendationPanel.svelte` 1:1.
 */
export function buildRecommendationTabExport(args: RecommendationTabInputs): string {
  const meta = buildMeta({
    sourceTab: 'recommendation',
    symbol: args.symbol,
    tfSecs: args.tfSecs ?? null,
    timestamp: args.timestamp ?? null,
    markPrice: args.markPrice ?? null,
    filterState: args.filterState,
  });
  const rank = computeDecisionRank({
    advisory: args.advisory,
    decisionContext: args.decisionContext,
    opportunity: args.opportunity,
    analysis: args.analysis,
  });
  // Active-side R:R (per-side, gated on macro bias). The legacy
  // matrix-level `opportunity.expected_rr_internal` was removed in
  // v6.9; we now read the per-side value that matches the active
  // bias.
  const activeSideRr = (() => {
    if (!args.opportunity) return 0;
    const bias = args.analysis?.bias ?? 'Neutral';
    if (bias === 'Bullish' || bias === 'StrongBullish') {
      return args.opportunity.long_expected_rr_internal ?? 0;
    }
    if (bias === 'Bearish' || bias === 'StrongBearish') {
      return args.opportunity.short_expected_rr_internal ?? 0;
    }
    return 0;
  })();
  const payload: RecommendationPayload = {
    source_tab: 'recommendation',
    meta,
    environment: buildEnvironmentBlock(args.advisory, args.decisionContext, rank.headline.state),
    verdict: buildVerdictBlock(rank),
    runner_ups: buildRunnerUpsBlock(rank),
    top_setup: buildTopSetupBlock(args.opportunity, args.analysis),
    safety_flags: buildSafetyFlagsBlock(
      args.opportunity,
      args.decisionContext,
      args.advisory,
      rank.headline.state,
      activeSideRr,
    ),
    why: rank.rationale.slice(0, 3),
    price_levels: buildPriceLevelsBlock(args.opportunity, rank.top),
    strategy: buildStrategyBlock(args.advisory),
    final_verdict: args.advisory?.final_recommendation ?? '',
  };
  return JSON.stringify(payload, null, 2);
}
