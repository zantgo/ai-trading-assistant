// Risk tab builder — scoped export payload mirroring the Risk panel.
//
// v7.0-audit: rewrites the payload to use the new shared envelope (no
// filter_state, single current_price, structured header chrome). Adds
// `headline_parts` as a structured block, `interpretation_full` as the
// screen paragraph, and reformats cascade telemetry into typed fields.

import type {
  RiskMatrix,
  RiskDimension,
  RiskLevel,
  LiquidityFlow,
  LiquidationClusterMatrix,
} from '../../types';
import {
  buildPriceBlock,
  buildHeaderBlock,
  type MetaEnvelope,
  type HeaderBlock,
  type InstanceTermsLike,
} from './shared';
import type { LayerHeaderSpec } from '../layerHeader';

// ── Payload types ────────────────────────────────────────────────────────

export interface RiskHeroBlock {
  overall_score: number;
  overall_level: string;
  overall_state: string;
  overall_confidence: number;
  top_severity: RiskLevel | null;
}

export interface RiskSummaryCountsBlock {
  very_low: { label: string; count: number };
  low:      { label: string; count: number };
  moderate: { label: string; count: number };
  high:     { label: string; count: number };
  extreme:  { label: string; count: number };
}

export interface RiskCascadeExtras {
  state_label: string;
  /** Null when no flow telemetry is present — mirrors the panel hiding the
   *  Intensity field (RiskPanel.svelte) instead of rendering `0.0`. */
  intensity_display: string | null;
  asymmetry_sign: string;
  asymmetry_magnitude_pct: number | null;
  asymmetry_description: string | null;
  /** The exact badge sentence the screen renders, e.g. "↑35.0% (short squeeze)". */
  asymmetry_display: string | null;
}

export interface RiskDimensionExport {
  name: string;
  key: string;
  weight: number;
  weight_pct: number;
  score: number;
  level: string;
  state: string;
  state_display: string;
  confidence: number;
  evidence: string[];
  /** Screen chip for High/Extreme dims with no recorded evidence. */
  no_evidence_text: string | null;
  /** Screen placeholder for dims whose data feed is inactive. */
  not_active_text: string | null;
  /** True when the risk matrix is absent — mirrors the screen's "AWAITING"
   *  placeholder cards (name + weight only). */
  awaiting: boolean;
  /** Verbatim badge text shown for awaiting rows ("AWAITING"). */
  awaiting_badge: string | null;
  bar_pct: number;
  weight_mark_pct: number;
  is_cascade_dim: boolean;
  not_active: boolean;
  cascade_extras: RiskCascadeExtras | null;
}

export interface RiskHeadlineParts {
  very_low_count: number;
  low_count: number;
  moderate_count: number;
  high_count: number;
  extreme_count: number;
  overall_level: string;
}

export interface RiskHeroHintBlock {
  hint: string;
}

export interface RiskDisclosureBlock {
  weights: Array<{ label: string; pct: number }>;
  note: string;
}

export interface RiskPayload {
  source_tab: 'risk';
  meta: MetaEnvelope;
  header: HeaderBlock;
  hero: (RiskHeroBlock & RiskHeroHintBlock) | null;
  summary_counts: RiskSummaryCountsBlock;
  dimensions: RiskDimensionExport[];
  headline_parts: RiskHeadlineParts | null;
  interpretation_headline: string;
  interpretation_full: string | null;
  disclosure: RiskDisclosureBlock;
  awaiting_dimensions_text: string;
}

// ── Constants (mirrors `RiskPanel.svelte::namedDims`) ────────────────────

const RISK_DIMENSION_DEFS: ReadonlyArray<{
  name: string;
  key: keyof RiskMatrix;
  weight: number;
  isCascade: boolean;
}> = [
  { name: 'Market Risk',              key: 'market_risk',              weight: 0.14, isCascade: false },
  { name: 'Volatility Risk',          key: 'volatility_risk',          weight: 0.14, isCascade: false },
  { name: 'Exec Liquidity Risk',      key: 'execution_liquidity_risk', weight: 0.14, isCascade: false },
  { name: 'Structure Risk',           key: 'structure_risk',           weight: 0.10, isCascade: false },
  { name: 'Momentum Risk',            key: 'momentum_risk',            weight: 0.14, isCascade: false },
  { name: 'Signal Risk',              key: 'signal_risk',              weight: 0.10, isCascade: false },
  { name: 'Execution Risk',           key: 'execution_risk',           weight: 0.10, isCascade: false },
  { name: 'Cascade Risk',             key: 'cascade_risk',             weight: 0.14, isCascade: true  },
];

const LEVELS: RiskLevel[] = ['VeryLow', 'Low', 'Moderate', 'High', 'Extreme'];
const LEVEL_LABELS: Record<RiskLevel, string> = {
  VeryLow: 'Very Low',
  Low: 'Low',
  Moderate: 'Moderate',
  High: 'High',
  Extreme: 'Extreme',
};

function levelRank(l: RiskLevel): number {
  return LEVELS.indexOf(l);
}

function normalizeLevelKey(l: string): string {
  return l ? l.toLowerCase().replace(/_/g, '') : 'moderate';
}

function stateArrow(state: string): string {
  // Wire states are PascalCase ("Critical"); the screen normalizes the
  // arrow lookup to lowercase — mirror it exactly.
  switch (String(state).toLowerCase()) {
    case 'improving':  return '↘';
    case 'increasing': return '↗';
    case 'elevated':   return '↑';
    case 'critical':   return '⚠';
    case 'stable':
    default:           return '→';
  }
}

function buildHeroBlock(risk: RiskMatrix): RiskHeroBlock {
  const overall = risk.overall_risk;
  const dimCounts: Record<string, number> = {
    verylow: 0, low: 0, moderate: 0, high: 0, extreme: 0,
  };
  for (const def of RISK_DIMENSION_DEFS) {
    const dim = (risk as unknown as Record<string, RiskDimension | undefined>)[def.key];
    if (!dim) continue;
    const key = normalizeLevelKey(dim.level);
    if (key in dimCounts) dimCounts[key]++;
  }
  let topSeverity: RiskLevel | null = null;
  if (dimCounts.extreme > 0) topSeverity = 'Extreme';
  else if (dimCounts.high > 0) topSeverity = 'High';
  else if (dimCounts.moderate > 0) topSeverity = 'Moderate';
  else if (dimCounts.low > 0) topSeverity = 'Low';
  else topSeverity = 'VeryLow'; // mirrors RiskPanel.svelte (unconditional fallback)
  // The screen hides the "peak" chip when the top severity equals the
  // overall level — mirror that so the JSON says what the screen shows.
  if (topSeverity === overall.level) topSeverity = null;

  return {
    overall_score: Math.round(overall.score),
    overall_level: overall.level,
    overall_state: overall.state,
    overall_confidence: Math.round(overall.confidence),
    top_severity: topSeverity,
  };
}

function buildSummaryCounts(risk: RiskMatrix | null): RiskSummaryCountsBlock {
  const counts: RiskSummaryCountsBlock = {
    very_low: { label: 'Very Low', count: 0 },
    low:      { label: 'Low',      count: 0 },
    moderate: { label: 'Moderate', count: 0 },
    high:     { label: 'High',     count: 0 },
    extreme:  { label: 'Extreme',  count: 0 },
  };
  if (!risk) return counts;
  for (const def of RISK_DIMENSION_DEFS) {
    const dim = (risk as unknown as Record<string, RiskDimension | undefined>)[def.key];
    if (!dim) continue;
    const key = normalizeLevelKey(dim.level);
    if (key === 'verylow') counts.very_low.count++;
    else if (key === 'low') counts.low.count++;
    else if (key === 'moderate') counts.moderate.count++;
    else if (key === 'high') counts.high.count++;
    else if (key === 'extreme') counts.extreme.count++;
  }
  return counts;
}

function buildCascadeExtras(
  flow: LiquidityFlow | null,
  cluster: LiquidationClusterMatrix | null,
): RiskCascadeExtras | null {
  // The panel renders the cascade section only when flow or cluster exists
  // (RiskPanel.svelte) — emit null instead of placeholder values.
  if (!flow && !cluster) return null;
  const asym = cluster?.cascade_asymmetry;
  const sign = asym != null && asym > 0 ? '+' : asym != null && asym < 0 ? '-' : '';
  // The screen renders the magnitude as a percentage: `(asym * 100).toFixed(1)`.
  const magnitude = asym != null ? Math.abs(asym) * 100 : null;
  const description =
    asym == null
      ? null
      : asym > 0
        ? 'short squeeze'
        : asym < 0
          ? 'long cascade'
          : 'balanced';
  const display =
    asym == null
      ? null
      : asym > 0
        ? `↑${(asym * 100).toFixed(1)}% (short squeeze)`
        : asym < 0
          ? `↓${(Math.abs(asym) * 100).toFixed(1)}% (long cascade)`
          : '0.0% (balanced)';
  return {
    state_label:
      !flow?.cascade_state || String(flow.cascade_state).toUpperCase() === 'NONE'
        ? '—'
        : flow.cascade_state,
    intensity_display:
      flow?.cascade_intensity != null ? flow.cascade_intensity.toFixed(1) : null,
    asymmetry_sign: sign,
    asymmetry_magnitude_pct: magnitude,
    asymmetry_description: description,
    asymmetry_display: display,
  };
}

function buildDimensionsBlock(
  risk: RiskMatrix | null,
  flow: LiquidityFlow | null,
  cluster: LiquidationClusterMatrix | null,
): RiskDimensionExport[] {
  if (!risk) {
    // Mirror the screen's awaiting placeholder cards (name + weight +
    // "AWAITING" badge + per-dimension paragraph) — 8 rows, def order.
    return RISK_DIMENSION_DEFS.map((def) => ({
      name: def.name,
      key: def.key,
      weight: def.weight,
      weight_pct: Math.round(def.weight * 100),
      score: 0,
      not_active: false,
      awaiting: true,
      awaiting_badge: 'AWAITING',
      level: 'UNKNOWN',
      state: 'UNKNOWN',
      state_display: '\u2192 UNKNOWN',
      confidence: 0,
      evidence: [],
      no_evidence_text: null,
      not_active_text: null,
      bar_pct: 0,
      weight_mark_pct: Math.round(def.weight * 100),
      is_cascade_dim: def.isCascade,
      cascade_extras: def.isCascade ? buildCascadeExtras(flow, cluster) : null,
    }));
  }
  const decorated = RISK_DIMENSION_DEFS.map((def) => {
    const dim = (risk as unknown as Record<string, RiskDimension | undefined>)[def.key];
    return { def, dim };
  });
  decorated.sort((a, b) => {
    const sa = a.dim?.score ?? -1;
    const sb = b.dim?.score ?? -1;
    if (sb !== sa) return sb - sa;
    const la = a.dim ? levelRank(a.dim.level) : -1;
    const lb = b.dim ? levelRank(b.dim.level) : -1;
    return lb - la;
  });

  return decorated.map(({ def, dim }) => ({
    name: def.name,
    key: def.key,
    weight: def.weight,
    weight_pct: Math.round(def.weight * 100),
    score: Math.round(dim?.score ?? 0),
    not_active: !dim,
    awaiting: false,
    awaiting_badge: null,
    level: dim?.level ?? 'UNKNOWN',
    state: dim?.state ?? 'UNKNOWN',
    state_display: `${stateArrow(dim?.state ?? 'STABLE')} ${(dim?.state ?? 'UNKNOWN').toUpperCase()}`,
    confidence: Math.round(dim?.confidence ?? 0),
    evidence: dim?.evidence ?? [],
    // Screen chip for High/Extreme dims that carry no recorded evidence.
    no_evidence_text:
      dim && (dim.level === 'High' || dim.level === 'Extreme') && (dim.evidence ?? []).length === 0
        ? 'No evidence recorded'
        : null,
    // Screen placeholder for inactive dimensions.
    not_active_text: !dim ? 'Data feed inactive for this dimension.' : null,
    bar_pct: Math.min(dim?.score ?? 0, 100),
    weight_mark_pct: Math.round(def.weight * 100),
    is_cascade_dim: def.isCascade,
    cascade_extras: def.isCascade ? buildCascadeExtras(flow, cluster) : null,
  }));
}

function buildHeadlineParts(risk: RiskMatrix | null): RiskHeadlineParts | null {
  if (!risk) return null;
  const overall = risk.overall_risk;
  const dimCounts: Record<string, number> = {
    verylow: 0, low: 0, moderate: 0, high: 0, extreme: 0,
  };
  for (const def of RISK_DIMENSION_DEFS) {
    const dim = (risk as unknown as Record<string, RiskDimension | undefined>)[def.key];
    if (!dim) continue;
    const key = normalizeLevelKey(dim.level);
    if (key in dimCounts) dimCounts[key]++;
  }
  return {
    very_low_count: dimCounts.verylow,
    low_count: dimCounts.low,
    moderate_count: dimCounts.moderate,
    high_count: dimCounts.high,
    extreme_count: dimCounts.extreme,
    overall_level: overall.level,
  };
}

function buildInterpretationHeadline(headline: RiskHeadlineParts | null): string {
  if (!headline) return '';
  const parts: string[] = [];
  if (headline.extreme_count > 0) parts.push(`${headline.extreme_count} extreme`);
  if (headline.high_count > 0) parts.push(`${headline.high_count} high`);
  if (headline.moderate_count > 0) parts.push(`${headline.moderate_count} moderate`);
  if (parts.length === 0) return `all dimensions calm · overall ${headline.overall_level.toLowerCase()}`;
  return `${parts.join(' · ')} · overall ${headline.overall_level.toLowerCase()}`;
}

function buildInterpretationFull(risk: RiskMatrix | null): string | null {
  if (!risk) {
    // Verbatim screen copy — the empty-state interpretation paragraph the
    // panel renders when no risk matrix has arrived yet.
    return 'Risk synthesis is initializing — this section will provide a human-readable summary of the overall risk environment, highlighting which dimensions require attention and suggesting position-sizing guidance.';
  }
  const headline = buildHeadlineParts(risk);
  if (!headline) return null;
  const overall = risk.overall_risk;
  const overallLevel = overall.level.toLowerCase().replace(/_/g, ' ');
  const confidence = Math.round(overall.confidence);
  let body: string;
  if (headline.extreme_count > 0 || headline.high_count > 0) {
    // Zero-count sentences are omitted on screen — mirror that exactly.
    body = `<strong>Elevated risk environment.</strong>`;
    if (headline.extreme_count > 0) {
      body += ` ${headline.extreme_count} dimension${headline.extreme_count === 1 ? '' : 's'} at extreme levels.`;
    }
    if (headline.high_count > 0) {
      body += ` ${headline.high_count} dimension${headline.high_count === 1 ? '' : 's'} at high levels.`;
    }
    body += ` Consider reduced position sizing and wider stops. Monitor the highest-severity dimensions for evidence of improvement before committing capital.`;
  } else if (headline.moderate_count > 0) {
    body = `<strong>Moderate risk environment.</strong> ${headline.moderate_count} dimension${headline.moderate_count === 1 ? '' : 's'} at moderate levels. Standard position sizing applies, but stay alert to dimensions trending toward higher severity.`;
  } else {
    body = `<strong>Low risk environment.</strong> All dimensions are within acceptable bounds. Favorable conditions for disciplined execution with standard risk parameters.`;
  }
  return `${body} Overall composite score is <strong>${overallLevel}</strong> at ${confidence}% confidence.`;
}

// ── Public builder ───────────────────────────────────────────────────────

export interface RiskTabInputs {
  risk: RiskMatrix | null;
  flow: LiquidityFlow | null;
  cluster: LiquidationClusterMatrix | null;
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
 * Build the Risk tab export payload. Mirrors `RiskPanel.svelte` 1:1.
 */
export function buildRiskTabExport(args: RiskTabInputs): string {
  const { meta } = buildPriceBlock({
    symbol: args.symbol,
    exchange: args.exchange,
    terms: args.terms,
    fallbackMarkPrice: args.markPrice,
    tfSecs: args.tfSecs,
    timestamp: args.timestamp,
    isCompleted: args.isCompleted,
  });
  const headline = buildHeadlineParts(args.risk);
  const disclosure: RiskDisclosureBlock = {
    weights: [
      { label: 'Market', pct: 14 },
      { label: 'Volatility', pct: 14 },
      { label: 'ExecLiq', pct: 14 },
      { label: 'Structure', pct: 10 },
      { label: 'Momentum', pct: 14 },
      { label: 'Signal', pct: 10 },
      { label: 'Execution', pct: 10 },
      { label: 'Cascade', pct: 14 },
    ],
    note: 'Overall risk is a weighted sum of the 8 dimension scores. State and confidence modify each dimension\'s contribution, but do not alter the headline score directly.',
  };
  const baseHero = args.risk ? buildHeroBlock(args.risk) : null;
  const hero = baseHero
    // Verbatim screen copy (RiskPanel.svelte hero hint).
    ? { ...baseHero, hint: 'Lower is safer. State modifiers adjust each dimension\'s contribution but not the headline score.' }
    : null;
  const payload: RiskPayload = {
    source_tab: 'risk',
    meta,
    header: buildHeaderBlock(args.headerSpec),
    hero,
    summary_counts: buildSummaryCounts(args.risk),
    dimensions: buildDimensionsBlock(args.risk, args.flow, args.cluster),
    headline_parts: headline,
    interpretation_headline: buildInterpretationHeadline(headline),
    interpretation_full: buildInterpretationFull(args.risk),
    disclosure,
    awaiting_dimensions_text: 'Awaiting risk assessment — this dimension will populate once market data stabilizes.',
  };
  return JSON.stringify(payload, null, 2);
}