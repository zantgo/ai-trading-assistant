// Risk tab builder — scoped export payload mirroring the Risk panel.
//
// The Risk panel renders five sub-blocks:
//   1. Hero ring + headline (overall score, level, state, confidence)
//   2. Summary tiles (count per level: VeryLow / Low / Moderate / High / Extreme)
//   3. Risk dimension cards (8 dimensions, panel-sorted by severity)
//   4. Cascade telemetry chips (under the cascade_risk card)
//   5. Interpretation paragraph
//
// This builder reproduces each block 1:1 in JSON.

import type {
  RiskMatrix,
  RiskDimension,
  RiskLevel,
  LiquidityFlow,
  LiquidationClusterMatrix,
} from '../../types';
import { buildMeta } from './shared';
import type { MetaEnvelope, FilterStateBlock } from './shared';

// ── Payload types ────────────────────────────────────────────────────────

export interface RiskHeroBlock {
  overall_score: number;
  overall_level: string;
  overall_state: string;
  overall_confidence: number;
  top_severity: RiskLevel | null;
  ring_pct: number;
}

export interface RiskSummaryCounts {
  very_low: number;
  low: number;
  moderate: number;
  high: number;
  extreme: number;
}

export interface RiskDimensionExport {
  name: string;
  key: string;
  weight: number;
  weight_pct: number;
  score: number;
  level: string;
  state: string;
  confidence: number;
  evidence: string[];
  bar_pct: number;
  weight_mark_pct: number;
  is_cascade_dim: boolean;
}

export interface RiskCascadeTelemetry {
  cascade_state: string;
  cascade_intensity: number;
  cascade_asymmetry: number | null;
}

export interface RiskPayload {
  source_tab: 'risk';
  meta: MetaEnvelope;
  hero: RiskHeroBlock;
  summary_counts: RiskSummaryCounts;
  dimensions: RiskDimensionExport[];
  cascade_telemetry: RiskCascadeTelemetry | null;
  interpretation: string;
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
  { name: 'Execution Liquidity Risk', key: 'execution_liquidity_risk', weight: 0.14, isCascade: false },
  { name: 'Structure Risk',           key: 'structure_risk',           weight: 0.10, isCascade: false },
  { name: 'Momentum Risk',            key: 'momentum_risk',            weight: 0.14, isCascade: false },
  { name: 'Signal Risk',              key: 'signal_risk',              weight: 0.10, isCascade: false },
  { name: 'Execution Risk',           key: 'execution_risk',           weight: 0.10, isCascade: false },
  { name: 'Cascade Risk',             key: 'cascade_risk',             weight: 0.14, isCascade: true  },
];

const LEVELS: RiskLevel[] = ['VeryLow', 'Low', 'Moderate', 'High', 'Extreme'];

function levelRank(l: RiskLevel): number {
  return LEVELS.indexOf(l);
}

function normalizeLevelKey(l: string): string {
  return l ? l.toLowerCase().replace(/_/g, '') : 'moderate';
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
  else if (dimCounts.verylow > 0) topSeverity = 'VeryLow';

  return {
    overall_score: overall.score,
    overall_level: overall.level,
    overall_state: overall.state,
    overall_confidence: overall.confidence,
    top_severity: topSeverity,
    ring_pct: Math.min(overall.score, 100),
  };
}

function buildSummaryCounts(risk: RiskMatrix): RiskSummaryCounts {
  const counts: RiskSummaryCounts = {
    very_low: 0, low: 0, moderate: 0, high: 0, extreme: 0,
  };
  for (const def of RISK_DIMENSION_DEFS) {
    const dim = (risk as unknown as Record<string, RiskDimension | undefined>)[def.key];
    if (!dim) continue;
    const key = normalizeLevelKey(dim.level);
    if (key === 'verylow') counts.very_low++;
    else if (key === 'low') counts.low++;
    else if (key === 'moderate') counts.moderate++;
    else if (key === 'high') counts.high++;
    else if (key === 'extreme') counts.extreme++;
  }
  return counts;
}

function buildDimensionsBlock(risk: RiskMatrix): RiskDimensionExport[] {
  // Panel-sorted by severity score (highest first), ties broken by level rank.
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
    score: dim?.score ?? 0,
    level: dim?.level ?? 'UNKNOWN',
    state: dim?.state ?? 'UNKNOWN',
    confidence: dim?.confidence ?? 0,
    evidence: dim?.evidence ?? [],
    bar_pct: Math.min(dim?.score ?? 0, 100),
    weight_mark_pct: Math.round(def.weight * 100),
    is_cascade_dim: def.isCascade,
  }));
}

function buildCascadeTelemetry(
  flow: LiquidityFlow | null,
  cluster: LiquidationClusterMatrix | null,
): RiskCascadeTelemetry | null {
  if (!flow && !cluster) return null;
  return {
    cascade_state: flow?.cascade_state ?? 'None',
    cascade_intensity: flow?.cascade_intensity ?? 0,
    cascade_asymmetry: cluster?.cascade_asymmetry ?? null,
  };
}

function buildInterpretation(risk: RiskMatrix | null): string {
  if (!risk) return 'Risk assessment engine initializing — the dashboard skeleton shows all dimensions that will populate once market data stabilizes.';
  const overall = risk.overall_risk;
  const counts: Record<string, number> = {
    verylow: 0, low: 0, moderate: 0, high: 0, extreme: 0,
  };
  for (const def of RISK_DIMENSION_DEFS) {
    const dim = (risk as unknown as Record<string, RiskDimension | undefined>)[def.key];
    if (!dim) continue;
    const key = normalizeLevelKey(dim.level);
    if (key in counts) counts[key]++;
  }
  const c = counts;
  const headlineParts: string[] = [];
  if (c.extreme > 0) headlineParts.push(`${c.extreme} extreme`);
  if (c.high > 0) headlineParts.push(`${c.high} high`);
  if (c.moderate > 0) headlineParts.push(`${c.moderate} moderate`);
  const overallLevel = overall.level.toLowerCase().replace(/_/g, ' ');
  if (headlineParts.length > 0) {
    return `${headlineParts.join(' · ')} · overall ${overallLevel}`;
  }
  return `all dimensions calm · overall ${overallLevel}`;
}

// ── Public builder ───────────────────────────────────────────────────────

export interface RiskTabInputs {
  risk: RiskMatrix | null;
  flow: LiquidityFlow | null;
  cluster: LiquidationClusterMatrix | null;
  symbol: string;
  tfSecs?: number | null;
  timestamp?: number | null;
  markPrice?: number | null;
  filterState?: FilterStateBlock;
}

/**
 * Build the Risk tab export payload. Mirrors `RiskPanel.svelte` 1:1.
 * Returns valid JSON with safety defaults when `risk` is null
 * (the panel renders a "no data" banner in that case).
 */
export function buildRiskTabExport(args: RiskTabInputs): string {
  const meta = buildMeta({
    sourceTab: 'risk',
    symbol: args.symbol,
    tfSecs: args.tfSecs ?? null,
    timestamp: args.timestamp ?? null,
    markPrice: args.markPrice ?? null,
    filterState: args.filterState,
  });
  const risk = args.risk;
  const payload: RiskPayload = {
    source_tab: 'risk',
    meta,
    hero: risk ? buildHeroBlock(risk) : {
      overall_score: 0,
      overall_level: 'UNKNOWN',
      overall_state: 'UNKNOWN',
      overall_confidence: 0,
      top_severity: null,
      ring_pct: 0,
    },
    summary_counts: risk ? buildSummaryCounts(risk) : {
      very_low: 0, low: 0, moderate: 0, high: 0, extreme: 0,
    },
    dimensions: risk ? buildDimensionsBlock(risk) : [],
    cascade_telemetry: buildCascadeTelemetry(args.flow, args.cluster),
    interpretation: buildInterpretation(risk),
  };
  return JSON.stringify(payload, null, 2);
}
