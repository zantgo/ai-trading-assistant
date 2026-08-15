// Recommendation tab builder — scoped export payload mirroring the panel.
//
// v7.0-audit: rewrites the payload to use the new shared envelope (no
// filter_state, single current_price, structured header chrome) and the
// new R:R availability helper (replaces `rr: null` with `{available, value, reason}`).

import type {
  AdvisoryMatrix,
  AnalysisMatrix,
  DecisionContext,
  OpportunityMatrix,
  RiskDimension,
} from '../../types';
import {
  computeDecisionRank,
  resolveActiveRr,
  riskAdjRrExplanation,
  topSetupSummary,
  entryDangerLevel,
} from '../../lib/decisionRank';
import {
  buildPriceBlock,
  buildHeaderBlock,
  buildRrBlock,
  type MetaEnvelope,
  type HeaderBlock,
  type RrBlock,
} from './shared';
import type { LayerHeaderSpec } from '../layerHeader';

// ── Payload types ────────────────────────────────────────────────────────

export interface RecommendationEnvironmentBlock {
  directional_guidance: string;
  market_stance: string;
  strategy_environment: string;
  opportunity_classification: string;
  confidence_pct: number;
  readiness: string;
  entry_danger_score: number;
  entry_danger_level: string;
}

export interface RecommendationVerdictBlock {
  top: 'LONG' | 'SHORT' | 'HOLD';
  long_probability: number;
  short_probability: number;
  hold_probability: number;
}

export interface TopSetupBlock {
  opportunity_type: string;
  viability: string;
  badge_text: string;
  score: number;
  preconditions_met: number;
  preconditions_total: number;
  direction_label: string;
  entry_zone: { low: number; high: number } | null;
  target_zone: { low: number; high: number } | null;
  invalidation: number | null;
  /** Display strings — verbatim copies of the screen card values. */
  entry_zone_display: string;
  target_zone_display: string;
  invalidation_display: string;
  rr_display: string;
  rr_available: boolean;
  rr_value: number | null;
  rr_reason: string | null;
  rationale: string;
}

function viabilityBadgeText(viability: string, top: string, belowFloor = false): string {
  // Mirrors `RecommendationPanel.svelte` — emits the same literal badge
  // text the screen shows. Empty string when the screen shows nothing
  // (e.g. Actionable + HOLD verdict).
  // v6.10.19 (T3): a sub-1.0 reference bracket is NEVER framed as a trade.
  if (belowFloor) return 'R:R BELOW ACTIONABLE FLOOR';
  if (viability === 'Actionable' && top !== 'HOLD') return 'ACTIONABLE';
  if (viability === 'Qualifying') return 'QUALIFYING';
  if (viability === 'DirectionalNeutral') return 'HOLD · NO DIRECTIONAL EDGE';
  if (viability === 'GeometryInverted') return 'GEOMETRY INVERTED';
  if (viability === 'NoClear') return 'NO CLEAR SETUP';
  return '';
}

export interface SafetyFlagsBlock {
  readiness: string;
  rr_available: boolean;
  rr_value: number | null;
  rr_reason: string | null;
  /** The first-class R:R discount explanation (RR-008) — the same sentence
   *  the L6 header chip tooltip renders. `null` when there is no real
   *  risk-adjusted R:R. */
  risk_adj_rr_explanation: string | null;
  stop_loss_pct: number;
  confidence_pct: number;
  entry_danger_score: number;
  entry_danger_level: string;
  /** Display strings — verbatim copies of the screen KPI chips. */
  rr_display: string;
  stop_loss_display: string;
  confidence_display: string;
  entry_danger_display: string;
}

export interface GaugeBlock {
  net_bias_pct: number;
  bias_direction: string;
  long_pct: number;
  short_pct: number;
  hold_pct: number;
  /** v6.10.19 (P6): the graded-lean floors adjusted this split — the
   *  directional read is structurally boosted (LEAN annotation). */
  lean_floor_applied: boolean;
  /** Verbatim copy of the gauge needle label ("+37%" / "0%" / "-14%").
   *  v6.10.17: the needle is verdict-consistent — neutral only when the
   *  top action is HOLD; a directional lean gated by STAND ASIDE keeps
   *  its real net-bias display. */
  net_bias_display: string;
}

export interface NoClearCardBlock {
  title: string;
  body: string;
}

export interface RecommendationPayload {
  source_tab: 'recommendation';
  meta: MetaEnvelope;
  header: HeaderBlock;
  gauge: GaugeBlock;
  environment: RecommendationEnvironmentBlock;
  verdict: RecommendationVerdictBlock;
  top_setup: TopSetupBlock | null;
  /** Verbatim section-meta caption shown when no qualifying setup exists
   *  ("no qualifying setup yet") — null when a setup renders. */
  top_setup_empty_text: string | null;
  no_clear_card: NoClearCardBlock | null;
  safety_flags: SafetyFlagsBlock;
  why_note: string | null;
  why: string[];
  price_levels: {
    side: 'long' | 'short' | 'hold';
    entry_zone: { low: number; high: number } | null;
    target_zone: { low: number; high: number } | null;
    invalidation: number | null;
    horizon: string;
    hold_placeholder: string | null;
  };
  strategy: {
    entry: string;
    exit: string;
    protection: string;
    target: string;
    /** Panel caption shown under a HOLD verdict — null otherwise. */
    hold_caption: string | null;
  };
  /** Verdict-consistent final verdict (HOLD verdict → verdict sentence). */
  final_verdict: string;
  /** Advisory environment guidance rendered below the verdict under HOLD. */
  final_verdict_guidance: string | null;
}

// ── Helpers ──────────────────────────────────────────────────────────────

/**
 * v6.10.19 (T2 + T5): verdict-aware environment guidance. Under a genuine
 * HOLD top the guidance sentence must say exactly what the 0% needle
 * means — no "Long bias: …" claim leading the sentence, no "Entry: ….
 * Stop: …." execution instructions. Shared by the panel and the export so
 * screen and clipboard can never disagree. Non-HOLD verdicts pass the
 * advisory sentence through unchanged.
 */
export function verdictAwareGuidance(
  advisory: AdvisoryMatrix | null,
  top: 'LONG' | 'SHORT' | 'HOLD',
): string | null {
  if (!advisory?.final_recommendation) return null;
  if (top !== 'HOLD') return advisory.final_recommendation;
  let g = advisory.final_recommendation;
  // Strip any residual "Entry: …. Stop: …." clauses (server-side already
  // omits them under Neutral/Avoid — this is the defensive layer for
  // legacy payloads and directional-guidance HOLD states).
  g = g.replace(/\s*Entry: [^.]*\.?\s*Stop: [^.]*\.?\s*$/i, '');
  g = g.replace(/\s*Entry: [^.]*\.?\s*$/i, '');
  // Reword the leading directional claim — under HOLD the read exists
  // but the edge does not.
  const guidance = advisory.directional_guidance ?? '';
  const conf = Math.round(advisory.confidence_assessment ?? 0);
  g = g
    .replace(/^Strong long bias:/i, `${guidance} bias at ${conf}% — no actionable directional edge;`)
    .replace(/^Long bias:/i, `BULLISH bias at ${conf}% — no actionable directional edge;`)
    .replace(/^Strong short bias:/i, `${guidance} bias at ${conf}% — no actionable directional edge;`)
    .replace(/^Short bias:/i, `BEARISH bias at ${conf}% — no actionable directional edge;`);
  // Drop the duplicated "… bias with N% confidence" fragment that follows
  // the reworded claim (the confidence already lives in the new claim).
  g = g.replace(/,?\s*[A-Za-z]+ bias with \d+% confidence/i, '');
  // v6.10.19a (D2a): the strip can leave an orphaned ":," behind
  // ("…no actionable directional edge:, cautious…") — collapse both.
  g = g.replace(/:\s*,\s*/g, ', ');
  g = g.replace(/\s*:\s*$/g, '');
  return g.trim();
}

function sanitizeLabel(s: string): string {
  // Screen renders "—" for falsy input — mirror it.
  if (!s) return '\u2014';
  let cleaned = s.replace(/([a-z])([A-Z])/g, '$1 $2');
  cleaned = cleaned.replace(/_/g, ' ');
  cleaned = cleaned.trim().replace(/\s+/g, ' ');
  return cleaned
    .toLowerCase()
    .replace(/\b\w/g, (c) => c.toUpperCase());
}

function prettifyEnum(s: string): string {
  // Screen renders "—" for falsy input — mirror it.
  if (!s) return '\u2014';
  let cleaned = s.replace(/([A-Z]+)([A-Z][a-z])/g, '$1 $2');
  cleaned = cleaned.replace(/([a-z])([A-Z])/g, '$1 $2');
  cleaned = cleaned.replace(/_/g, ' ');
  cleaned = cleaned.trim().replace(/\s+/g, ' ');
  cleaned = cleaned
    .toLowerCase()
    .replace(/\b\w/g, (c) => c.toUpperCase());
  cleaned = cleaned.replace(/\sBased$/i, '-Based');
  cleaned = cleaned
    .replace(/^Atr\b/i, 'ATR')
    .replace(/^Sr\b/i, 'S/R')
    .replace(/^Rr\b/i, 'R:R')
    .replace(/^Sl\b/i, 'SL');
  return cleaned;
}

function readEntryDangerScore(decisionContext: DecisionContext | null): number {
  if (!decisionContext) return 50;
  const danger = decisionContext.entry_danger;
  // Screen default: missing entry_danger reads 50 (MODERATE).
  if (typeof danger === 'number') return danger;
  return (danger as RiskDimension | null)?.score ?? 50;
}

/** Price formatter — mirrors `RecommendationPanel.svelte::fmtPriceScale`. */
function fmtPriceScale(n: number, mp: number): string {
  if (mp >= 1000) return n.toFixed(0);
  if (mp >= 1) return n.toFixed(2);
  if (mp >= 0.01) return n.toFixed(4);
  if (mp >= 0.0001) return n.toFixed(6);
  return n.toFixed(8);
}

/** Risk-Adjusted R:R KPI string — mirrors the screen chip exactly. */
function rrKpiDisplay(rrRaw: number, isNA: boolean): string {
  if (isNA) return 'N/A';
  if (Number.isNaN(rrRaw) || rrRaw <= 0) return 'R:R \u2014';
  const norm = rrRaw >= 9.99 ? '9.99+' : rrRaw >= 5 ? rrRaw.toFixed(1) : rrRaw.toFixed(2);
  return `R:R 1 : ${norm}`;
}

function buildEnvironmentBlock(
  advisory: AdvisoryMatrix | null,
  decisionContext: DecisionContext | null,
  readiness: string,
): RecommendationEnvironmentBlock {
  const score = readEntryDangerScore(decisionContext);
  return {
    directional_guidance: advisory?.directional_guidance ?? '',
    market_stance: advisory?.market_stance ?? '',
    strategy_environment: advisory?.strategy_environment ?? '',
    opportunity_classification: advisory?.opportunity_classification ?? '',
    confidence_pct: advisory?.confidence_assessment ?? 0,
    readiness,
    entry_danger_score: score,
    entry_danger_level: entryDangerLevel(score),
  };
}

function buildVerdictBlock(
  rank: ReturnType<typeof computeDecisionRank>,
): RecommendationVerdictBlock {
  return {
    top: rank.top,
    long_probability: rank.long.probability,
    short_probability: rank.short.probability,
    hold_probability: rank.hold.probability,
  };
}

function buildGaugeBlock(
  rank: ReturnType<typeof computeDecisionRank>,
): GaugeBlock {
  const netBias = rank.long.probability - rank.short.probability;
  return {
    net_bias_pct: netBias,
    bias_direction: rank.long.probability > rank.short.probability
      ? 'LONG'
      : rank.short.probability > rank.long.probability
        ? 'SHORT'
        : 'NEUTRAL',
    long_pct: rank.long.probability,
    short_pct: rank.short.probability,
    hold_pct: rank.hold.probability,
    net_bias_display: netBias === 0 ? '0%' : `${netBias > 0 ? '+' : ''}${netBias}%`,
    lean_floor_applied: rank.lean_floor_applied === true,
  };
}

function buildTopSetupBlock(
  opportunity: OpportunityMatrix | null,
  analysis: AnalysisMatrix | null,
  decisionContext: DecisionContext | null,
  markPrice: number,
  top: 'LONG' | 'SHORT' | 'HOLD',
): TopSetupBlock | null {
  if (!opportunity) return null;
  // The panel passes the real decisionContext so the aggregate-fallback
  // side resolution (net_bias_pct) matches what the screen shows.
  const summary = topSetupSummary(opportunity, analysis, decisionContext);
  if (!summary) return null;
  const z = summary.zones;
  const viability =
    summary.viability === 'Actionable'
      ? 'Actionable'
      : summary.viability === 'Qualifying'
        ? 'Qualifying'
        : summary.viability === 'DirectionalNeutral'
        ? 'DirectionalNeutral'
        : summary.viability === 'GeometryInverted'
          ? 'GeometryInverted'
          : 'NoClear';
  // v6.10.19 (T3): a sub-1.0 reference bracket is never framed as a trade.
  const belowFloor = summary.below_floor === true;
  const rr = buildRrBlock(summary.rr, 'no_actionable_setup');
  const entryDisplay = z
    ? `$${fmtPriceScale(z.entry.low, markPrice)}–$${fmtPriceScale(z.entry.high, markPrice)}`
    : '\u2014';
  const targetDisplay = z
    ? (z.target.low > 0
        ? `$${fmtPriceScale(z.target.low, markPrice)}–$${fmtPriceScale(z.target.high, markPrice)}`
        : `$${fmtPriceScale(z.target.high, markPrice)}`)
    : '\u2014';
  const invalidationDisplay =
    z && z.invalidation > 0
      ? `$${fmtPriceScale(z.invalidation, markPrice)}`
      : '\u2014';
  // R:R display derives from the canonical `summary.rr` (the shared
  // resolver: profile wire → matrix wire → aligned zones fallback) with
  // the same formatting as the header chip. When N/A, the resolver's
  // human-readable reason rides in `rr_reason`.
  let rrDisplay: string;
  if (summary.rr == null) {
    rrDisplay = 'R:R N/A';
  } else {
    rrDisplay = rrKpiDisplay(summary.rr, false);
  }
  const rrReason = summary.rr != null ? null : (summary.rr_reason ?? 'no actionable setup');
  return {
    opportunity_type: sanitizeLabel(summary.opportunity_type),
    viability,
    badge_text: viabilityBadgeText(viability, top, belowFloor),
    score: summary.score,
    preconditions_met: summary.preconditions_met,
    preconditions_total: summary.preconditions_total,
    direction_label:
      summary.direction === 'LONG' ? 'LONG'
      : summary.direction === 'SHORT' ? 'SHORT' : 'NEUTRAL',
    entry_zone: z ? { low: z.entry.low, high: z.entry.high } : null,
    target_zone: z ? { low: z.target.low, high: z.target.high } : null,
    invalidation: z?.invalidation ?? null,
    entry_zone_display: entryDisplay,
    target_zone_display: targetDisplay,
    invalidation_display: invalidationDisplay,
    rr_display: rrDisplay,
    rr_available: rr.available,
    rr_value: rr.value,
    rr_reason: rrReason,
    rationale: summary.rationale,
  };
}

function buildSafetyFlagsBlock(
  decisionContext: DecisionContext | null,
  advisory: AdvisoryMatrix | null,
  readiness: string,
  topAction: 'LONG' | 'SHORT' | 'HOLD',
  opportunity: OpportunityMatrix | null,
): SafetyFlagsBlock {
  const rrRaw = decisionContext?.expected_reward_risk_ratio ?? 0;
  const rrNotApplicable = topAction === 'HOLD' && rrRaw === 0;
  const rr = rrNotApplicable
    ? { available: false, value: null, reason: 'no_directional_bias' as string | null }
    : buildRrBlock(rrRaw > 0 ? rrRaw : null, 'no_wire_rr');
  const score = readEntryDangerScore(decisionContext);
  const stopLossPct = advisory?.stop_loss_distance_pct ?? 0;
  const confidence = advisory?.confidence_assessment ?? 0;
  const entryDangerLevelVal = entryDangerLevel(score);
  // RR-008 (v6.10.14): the first-class discount explanation — the SAME
  // sentence the L6 header chip tooltip renders, so consumers don't
  // recompute the factor. Null when there is no real risk-adjusted R:R.
  const geometricRr = resolveActiveRr(opportunity, decisionContext).value;
  const rrExplanation = rrRaw > 0 ? riskAdjRrExplanation(geometricRr, rrRaw) : null;
  return {
    readiness,
    rr_available: rr.available,
    rr_value: rr.value,
    rr_reason: rr.reason,
    risk_adj_rr_explanation: rrExplanation,
    stop_loss_pct: stopLossPct,
    confidence_pct: confidence,
    entry_danger_score: score,
    entry_danger_level: entryDangerLevelVal,
    // Verbatim screen chips:
    rr_display: rrKpiDisplay(rrRaw, rrNotApplicable),
    stop_loss_display: stopLossPct > 0 ? `${stopLossPct.toFixed(2)}%` : '\u2014',
    confidence_display: `${confidence.toFixed(0)}%`,
    entry_danger_display: `${score.toFixed(0)} (${entryDangerLevelVal})`,
  };
}

function buildWhyNote(
  rank: ReturnType<typeof computeDecisionRank>,
): string | null {
  // v6.10.17: the note applies ONLY under a genuine HOLD top — a
  // directional lean (even gated by STAND ASIDE or coexisting with the
  // No Clear explanation card) has real directional meaning and must
  // never carry the "no directional edge" disclaimer.
  if (rank.top === 'HOLD') {
    return 'No directional edge — these bullets read the same across all three arms (LONG/SHORT/HOLD). They trace the data, not a trade call.';
  }
  return null;
}

function buildNoClearCard(
  advisory: AdvisoryMatrix | null,
  opportunity: OpportunityMatrix | null,
  topSetup: TopSetupBlock | null,
): NoClearCardBlock | null {
  // Mirrors the panel (v6.10.17): the No Clear Setup card renders when
  // the primary opportunity is NoClearOpportunity AND the top setup is
  // either absent (warmup) or the informational aggregated bracket
  // (viability NoClear). It explains WHY there is no qualifying profile
  // — the bracket card above still carries TPs/SLs/R:R.
  const hasNoClear =
    !!opportunity
    && !!advisory
    && (opportunity?.primary_opportunity ?? '') === 'NoClearOpportunity'
    && (topSetup === null || topSetup.viability === 'NoClear');
  if (!hasNoClear) return null;
  const body =
    advisory?.final_recommendation ||
    opportunity?.invalidation_note ||
    'No qualifying setup; market conditions do not currently favor a directional trade.';
  return { title: 'No Clear Setup', body };
}

function buildPriceLevelsBlock(
  opportunity: OpportunityMatrix | null,
  topAction: 'LONG' | 'SHORT' | 'HOLD',
): RecommendationPayload['price_levels'] {
  if (topAction === 'LONG') {
    return {
      side: 'long',
      entry_zone: opportunity?.long_entry_zone ?? null,
      target_zone: opportunity?.long_target_zone ?? null,
      invalidation: opportunity?.long_invalidation_level ?? null,
      horizon: opportunity?.time_horizon ?? '\u2014',
      hold_placeholder: null,
    };
  }
  if (topAction === 'SHORT') {
    return {
      side: 'short',
      entry_zone: opportunity?.short_entry_zone ?? null,
      target_zone: opportunity?.short_target_zone ?? null,
      invalidation: opportunity?.short_invalidation_level ?? null,
      horizon: opportunity?.time_horizon ?? '\u2014',
      hold_placeholder: null,
    };
  }
  return {
    side: 'hold',
    entry_zone: null,
    target_zone: null,
    invalidation: null,
    horizon: opportunity?.time_horizon ?? '\u2014',
    // R5: describes the ACTUAL card state — under HOLD the Top Setup
    // card carries the aggregated bracket on the net-bias side (not the
    // close-pinned sentinel the legacy copy claimed), with R:R N/A when
    // geometry is inverted.
    hold_placeholder:
      'No active setup — verdict is HOLD. The Top Setup card above carries the aggregated bracket on the net-bias side for reference; when geometry is inverted its R:R reads N/A and the bracket is non-actionable.',
  };
}

function buildStrategyBlock(
  advisory: AdvisoryMatrix | null,
  noActiveCall: boolean,
): RecommendationPayload['strategy'] {
  // Entry/Exit use the sanitizeLabel title-casing the screen renders;
  // Protection/Target use prettifyEnum with the -Based / ATR / S-R /
  // R:R / SL overrides. Under a genuine HOLD verdict (no active directional
  // call) the screen renders a muted caption ("environment playbook, not a
  // trade trigger") — FIX-5 (v6.10.15) — and the actionable-sounding values
  // are replaced with "—" (FIX-O5 v6.10.16). v6.10.17: a directional lean
  // gated by STAND ASIDE is a REAL lean — its playbook values render real.
  // The advisory text survives in `final_verdict_guidance`.
  return {
    entry: noActiveCall ? '\u2014' : sanitizeLabel(advisory?.entry_guidance ?? ''),
    exit: noActiveCall ? '\u2014' : sanitizeLabel(advisory?.exit_guidance ?? ''),
    protection: noActiveCall ? '\u2014' : prettifyEnum(advisory?.protection_strategy ?? ''),
    target: noActiveCall ? '\u2014' : prettifyEnum(advisory?.target_strategy ?? ''),
    hold_caption: noActiveCall
      ? 'For reference — no active directional call. The guidance below describes the environment, not a trade trigger.'
      : null,
  };
}

// ── Public builder ───────────────────────────────────────────────────────

export interface RecommendationTabInputs {
  advisory: AdvisoryMatrix | null;
  decisionContext: DecisionContext | null;
  opportunity: OpportunityMatrix | null;
  analysis: AnalysisMatrix | null;
  symbol: string;
  exchange?: string;
  tfSecs?: number | null;
  timestamp?: number | null;
  markPrice?: number | null;
  isCompleted?: boolean;
  terms?: import('./shared').InstanceTermsLike;
  headerSpec: LayerHeaderSpec;
}

/**
 * Build the Recommendation tab export payload. Mirrors
 * `RecommendationPanel.svelte` 1:1.
 */
export function buildRecommendationTabExport(args: RecommendationTabInputs): string {
  const { meta } = buildPriceBlock({
    symbol: args.symbol,
    exchange: args.exchange,
    terms: args.terms,
    fallbackMarkPrice: args.markPrice,
    tfSecs: args.tfSecs,
    timestamp: args.timestamp,
    isCompleted: args.isCompleted,
  });
  const rank = computeDecisionRank({
    advisory: args.advisory,
    decisionContext: args.decisionContext,
    opportunity: args.opportunity,
    analysis: args.analysis,
  });
  const topSetup = buildTopSetupBlock(args.opportunity, args.analysis, args.decisionContext, args.markPrice ?? 0, rank.top);
  const noClearCard = buildNoClearCard(args.advisory, args.opportunity, topSetup);
  // FIX-4/FIX-5 (v6.10.15) + v6.10.17 decoupling: "no active directional
  // call" applies ONLY under a genuine HOLD verdict — a directional lean
  // gated by STAND ASIDE carries a real (graded) read whose sentence
  // reports both the lean and the gate.
  const noActiveCall = rank.top === 'HOLD';
  const header = buildHeaderBlock(args.headerSpec);
  const score = readEntryDangerScore(args.decisionContext);
  const verdictSentence = noActiveCall
    ? `HOLD — no directional call (readiness: ${rank.headline.state}).`
    : rank.headline.state === 'STAND_ASIDE'
      ? `${rank.top} lean ${Math.round(rank.top_prob)}% — STAND ASIDE (readiness: STAND_ASIDE, entry_danger ${entryDangerLevel(score)}).`
      : rank.headline.state === 'READY'
        ? `${rank.top} ${Math.round(rank.top_prob)}% — READY (readiness: READY).`
        : `${rank.top} lean ${Math.round(rank.top_prob)}% — awaiting confirmation (readiness: ${rank.headline.state}).`;
  const payload: RecommendationPayload = {
    source_tab: 'recommendation',
    meta,
    header,
    gauge: buildGaugeBlock(rank),
    environment: buildEnvironmentBlock(args.advisory, args.decisionContext, rank.headline.state),
    verdict: buildVerdictBlock(rank),
    top_setup: topSetup,
    // Verbatim section-meta caption the panel renders when no setup exists
    // (top_setup null — opportunity matrix absent).
    top_setup_empty_text: topSetup ? null : 'no qualifying setup yet',
    no_clear_card: noClearCard,
    safety_flags: buildSafetyFlagsBlock(args.decisionContext, args.advisory, rank.headline.state, rank.top, args.opportunity),
    why_note: buildWhyNote(rank),
    why: rank.rationale.slice(0, 3),
    price_levels: buildPriceLevelsBlock(args.opportunity, rank.top),
    strategy: buildStrategyBlock(args.advisory, noActiveCall),
    // R6 + FIX-4 + v6.10.17: the final verdict is the verdict — under a
    // genuine HOLD it reads "no directional call"; under a directional
    // lean it carries the graded percentage AND the gate. The advisory
    // text is carried separately as environment guidance.
    final_verdict: verdictSentence,
    // v6.10.19 (T2/T5): verdict-aware — under a HOLD top the guidance is
    // reworded and stripped of execution instructions.
    final_verdict_guidance:
      verdictAwareGuidance(args.advisory, rank.top) != null
        ? `Environment guidance: ${verdictAwareGuidance(args.advisory, rank.top)}`
        : null,
  };
  return JSON.stringify(payload, null, 2);
}