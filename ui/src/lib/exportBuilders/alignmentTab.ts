// Alignment tab builder — scoped export payload mirroring the panel.
//
// The Alignment panel renders five sub-blocks:
//   1. Hero (mtf_overall_score, label, timeframes_present, signal_cross_tf_count, trend_agreement_pct)
//   2. Alignment Breakdown (10 dimension cards: name, score, state, confidence)
//   3. Timeframe Consensus (trend_agreement_pct + 4-axis polarization chips)
//   4. Per-Timeframe Snapshot (one card per TF: trend, momentum, overall, regime)
//   5. Score Calculation (4-axis weights + formula)
//   6. Interpretation paragraph

import type {
  AlignmentMatrix,
  AlignmentDimension,
  TfAlignmentInfo,
} from '../../types';
import { buildMeta } from './shared';
import type { MetaEnvelope } from './shared';

// ── Payload types ────────────────────────────────────────────────────────

export const ALIGNMENT_DIMENSION_NAMES = [
  'Trend', 'Momentum', 'Volume', 'Volatility',
  'Structure', 'Signal', 'Regime', 'Confidence', 'Liquidity', 'Tradability',
] as const;

export interface AlignmentHeroBlock {
  mtf_overall_score: number;
  mtf_overall_label: string;
  timeframes_present: number;
  signal_cross_tf_count: number;
  trend_agreement_pct: number;
}

export interface AlignmentDimensionRow {
  name: string;
  score: number;
  state: string;
  confidence: number;
}

export interface AlignmentConsensusBlock {
  trend_agreement_pct: number;
  label: 'strong_consensus' | 'partial_consensus' | 'conflict';
  polarization: Array<{
    key: 'T' | 'M' | 'Vt' | 'Vm';
    label: string;
    value: number;
  }>;
}

export interface AlignmentPerTimeframeRow {
  timeframe: string;
  trend_score: number;
  momentum_score: number;
  overall_score: number;
  regime: string;
  active_signals: number;
}

export interface AlignmentScoreCalcBlock {
  weights: Array<{
    key: 'T' | 'M' | 'Vt' | 'Vm';
    label: string;
    pct: number;
    color: string;
    value: number;
    contribution: number;
  }>;
  formula: string;
}

export interface AlignmentPayload {
  source_tab: 'alignment';
  meta: MetaEnvelope;
  hero: AlignmentHeroBlock;
  dimensions: AlignmentDimensionRow[];
  consensus: AlignmentConsensusBlock;
  per_timeframe: AlignmentPerTimeframeRow[];
  score_calculation: AlignmentScoreCalcBlock;
  interpretation: string;
}

// ── Constants ────────────────────────────────────────────────────────────

const SLOT_RANK: Record<string, number> = { MICRO: 0, FAST: 1, SLOW: 2, MACRO: 3 };

const SCORE_CALC_WEIGHTS: Array<{
  label: string;
  key: 'T' | 'M' | 'Vt' | 'Vm';
  pct: number;
  color: string;
}> = [
  { label: 'Trend',      key: 'T',  pct: 50, color: '#22c55e' },
  { label: 'Momentum',   key: 'M',  pct: 30, color: '#3b82f6' },
  { label: 'Vol.trend',  key: 'Vt', pct: 10, color: '#a78bfa' },
  { label: 'Vol.market', key: 'Vm', pct: 10, color: '#f59e0b' },
];

// ── Helpers ──────────────────────────────────────────────────────────────

function buildHeroBlock(alignment: AlignmentMatrix): AlignmentHeroBlock {
  return {
    mtf_overall_score: alignment.mtf_overall_score,
    mtf_overall_label: alignment.mtf_overall_label,
    timeframes_present: alignment.timeframes_present,
    signal_cross_tf_count: alignment.signal_cross_tf_count,
    trend_agreement_pct: alignment.trend_agreement_pct,
  };
}

function buildDimensionsBlock(alignment: AlignmentMatrix): AlignmentDimensionRow[] {
  return alignment.dimensions.map((dim: AlignmentDimension, i: number) => ({
    name: ALIGNMENT_DIMENSION_NAMES[i] ?? `Dim ${i}`,
    score: dim.score,
    state: dim.state,
    confidence: dim.confidence,
  }));
}

function classifyConsensus(pct: number): 'strong_consensus' | 'partial_consensus' | 'conflict' {
  if (pct >= 75) return 'strong_consensus';
  if (pct >= 50) return 'partial_consensus';
  return 'conflict';
}

function buildConsensusBlock(alignment: AlignmentMatrix): AlignmentConsensusBlock {
  return {
    trend_agreement_pct: alignment.trend_agreement_pct,
    label: classifyConsensus(alignment.trend_agreement_pct),
    polarization: [
      { key: 'T',  label: 'Trend',      value: alignment.mtf_trend_alignment },
      { key: 'M',  label: 'Momentum',   value: alignment.mtf_momentum_alignment },
      { key: 'Vt', label: 'Volume',     value: alignment.mtf_volume_alignment },
      { key: 'Vm', label: 'Volatility', value: alignment.mtf_volatility_alignment },
    ],
  };
}

function buildPerTimeframeBlock(alignment: AlignmentMatrix): AlignmentPerTimeframeRow[] {
  return (alignment.timeframe_alignments ?? [])
    .slice()
    .sort((a, b) => (SLOT_RANK[a.timeframe] ?? 99) - (SLOT_RANK[b.timeframe] ?? 99))
    .map((tf: TfAlignmentInfo) => ({
      timeframe: tf.timeframe,
      trend_score: tf.trend_score,
      momentum_score: tf.momentum_score,
      overall_score: tf.overall_score,
      regime: tf.regime,
      active_signals: tf.active_signals,
    }));
}

function buildScoreCalcBlock(alignment: AlignmentMatrix): AlignmentScoreCalcBlock {
  const getValue = (key: 'T' | 'M' | 'Vt' | 'Vm'): number => {
    if (key === 'T') return alignment.mtf_trend_alignment;
    if (key === 'M') return alignment.mtf_momentum_alignment;
    if (key === 'Vt') return alignment.mtf_volume_alignment;
    return alignment.mtf_volatility_alignment;
  };
  const weights = SCORE_CALC_WEIGHTS.map((w) => {
    const value = getValue(w.key);
    return {
      key: w.key,
      label: w.label,
      pct: w.pct,
      color: w.color,
      value,
      contribution: value * (w.pct / 100),
    };
  });
  const t = alignment.mtf_trend_alignment.toFixed(2);
  const m = alignment.mtf_momentum_alignment.toFixed(2);
  const vt = alignment.mtf_volume_alignment.toFixed(2);
  const vm = alignment.mtf_volatility_alignment.toFixed(2);
  const overall = alignment.mtf_overall_score.toFixed(1);
  return {
    weights,
    formula: `0.5 * (${t}) + 0.3 * (${m}) + 0.1 * (${vt}) + 0.1 * (${vm}) = ${overall}`,
  };
}

function buildInterpretation(alignment: AlignmentMatrix | null): string {
  if (!alignment) return 'Awaiting alignment data — this section will synthesize a human-readable interpretation of multi-timeframe consensus once indicators populate.';
  const pct = alignment.trend_agreement_pct;
  const overall = alignment.mtf_overall_score.toFixed(1);
  const present = alignment.timeframes_present;
  const crossTf = alignment.signal_cross_tf_count;
  if (pct >= 75) {
    const crossLine = crossTf > 0
      ? `${crossTf} cross-timeframe signals reinforce the current bias.`
      : 'No cross-timeframe signals detected.';
    return `Multi-timeframe alignment shows strong directional consensus (${pct.toFixed(0)}% agreement across ${present}/4 timeframes). The composite score of ${overall} is classified as STRONG. ${crossLine}`;
  }
  if (pct >= 50) {
    return `Alignment shows partial consensus (${pct.toFixed(0)}% agreement). The composite score of ${overall} reflects mixed input from ${present} timeframes.`;
  }
  return `Timeframes are in conflict (${pct.toFixed(0)}% agreement). Exercise caution — different time horizons are pulling in opposite directions. Wait for re-alignment before committing to directional bias.`;
}

// ── Public builder ───────────────────────────────────────────────────────

export interface AlignmentTabInputs {
  alignment: AlignmentMatrix | null;
  symbol: string;
  tfSecs?: number | null;
  timestamp?: number | null;
  markPrice?: number | null;
  filterState?: {
    activeOnly: boolean;
    confirmedPlusOnly: boolean;
    hideGates: boolean;
    hideOverlays: boolean;
  };
}

/**
 * Build the Alignment tab export payload. Mirrors
 * `AlignmentPanel.svelte` 1:1.
 */
export function buildAlignmentTabExport(args: AlignmentTabInputs): string {
  const meta = buildMeta({
    sourceTab: 'alignment',
    symbol: args.symbol,
    tfSecs: args.tfSecs ?? null,
    timestamp: args.timestamp ?? null,
    markPrice: args.markPrice ?? null,
    filterState: args.filterState,
  });
  const alignment = args.alignment;
  const empty: AlignmentPayload = {
    source_tab: 'alignment',
    meta,
    hero: {
      mtf_overall_score: 0,
      mtf_overall_label: 'NO_DATA',
      timeframes_present: 0,
      signal_cross_tf_count: 0,
      trend_agreement_pct: 0,
    },
    dimensions: [],
    consensus: {
      trend_agreement_pct: 0,
      label: 'conflict',
      polarization: [
        { key: 'T',  label: 'Trend',      value: 0 },
        { key: 'M',  label: 'Momentum',   value: 0 },
        { key: 'Vt', label: 'Volume',     value: 0 },
        { key: 'Vm', label: 'Volatility', value: 0 },
      ],
    },
    per_timeframe: [],
    score_calculation: {
      weights: SCORE_CALC_WEIGHTS.map((w) => ({
        key: w.key,
        label: w.label,
        pct: w.pct,
        color: w.color,
        value: 0,
        contribution: 0,
      })),
      formula: '0.5 * (0.00) + 0.3 * (0.00) + 0.1 * (0.00) + 0.1 * (0.00) = 0.0',
    },
    interpretation: buildInterpretation(null),
  };
  if (!alignment) {
    return JSON.stringify(empty, null, 2);
  }
  const payload: AlignmentPayload = {
    source_tab: 'alignment',
    meta,
    hero: buildHeroBlock(alignment),
    dimensions: buildDimensionsBlock(alignment),
    consensus: buildConsensusBlock(alignment),
    per_timeframe: buildPerTimeframeBlock(alignment),
    score_calculation: buildScoreCalcBlock(alignment),
    interpretation: buildInterpretation(alignment),
  };
  return JSON.stringify(payload, null, 2);
}
