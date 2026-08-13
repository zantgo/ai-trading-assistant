// Alignment tab builder — scoped export payload mirroring the panel.
//
// v7.0-audit: rewrites the payload to use the new shared envelope (no
// filter_state, single current_price, structured header chrome). Adds
// `label_display` for consensus, sign-prefixed display strings for
// polarization and per-TF scores.

import type {
  AlignmentMatrix,
  AlignmentDimension,
  TfAlignmentInfo,
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

export const ALIGNMENT_DIMENSION_NAMES = [
  'Trend', 'Momentum', 'Volume', 'Volatility',
  'Structure', 'Signal', 'Regime', 'Confidence', 'Liquidity', 'Tradability',
] as const;

export interface AlignmentHeroBlock {
  mtf_overall_score: number;
  mtf_overall_label: string;
  mtf_overall_label_display: string;
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
  /** Raw agreement percent (float) — screen text uses `toFixed(0)`, the
   *  consensus bar width uses `toFixed(1)`; null when no alignment yet
   *  (screen renders the "—%" placeholder). */
  trend_agreement_pct: number | null;
  label: 'strong_consensus' | 'partial_consensus' | 'conflict' | null;
  label_display: string;
  polarization: Array<{
    key: 'T' | 'M' | 'Vt' | 'Vm';
    label: string;
    value: number;
    value_display: string;
  }>;
}

export interface AlignmentPerTimeframeRow {
  timeframe: string;
  trend_score: number;
  trend_score_display: string;
  momentum_score: number;
  momentum_score_display: string;
  overall_score: number;
  overall_score_display: string;
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
    value_display: string;
    contribution: number;
    contribution_display: string;
  }>;
  formula: string;
}

export interface AlignmentPayload {
  source_tab: 'alignment';
  meta: MetaEnvelope;
  header: HeaderBlock;
  hero: AlignmentHeroBlock;
  breakdown_meta: string;
  dimensions: AlignmentDimensionRow[];
  consensus: AlignmentConsensusBlock;
  per_timeframe: AlignmentPerTimeframeRow[];
  score_calculation: AlignmentScoreCalcBlock;
  interpretation: string;
  consensus_conflict_banner: string;
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

const CONSENSUS_DISPLAY: Record<'strong_consensus' | 'partial_consensus' | 'conflict', string> = {
  strong_consensus: 'Strong consensus — timeframes aligned',
  partial_consensus: 'Partial consensus — mixed signals',
  conflict: 'Conflict — time horizons diverging',
};

// ── Helpers ──────────────────────────────────────────────────────────────

function shortStateLabel(state: string): string {
  // "STRONG_BULLISH" → "STRONG"; "NO_DATA" → "NO DATA"
  const s = state.replace(/_/g, ' ');
  if (s === 'NO DATA' || s === 'NODATA') return 'NO DATA';
  if (s.startsWith('STRONG')) return 'STRONG';
  return s.toUpperCase();
}

function signedStr(n: number, decimals: number): string {
  // Screen convention (AlignmentPanel.svelte): values >= 0 render with a
  // leading '+' (so 0.00 shows as "+0.00"); only negatives are bare.
  const s = n.toFixed(decimals);
  return n >= 0 ? '+' + s : s;
}

function mLabel(label: string): string {
  // Mirrors `lib/layerHeader.ts::mLabel` (the screen-side mapper for
  // mtf_overall_label tokens).
  if (label.startsWith('STRONG_BULL')) return 'STRONG BULL';
  if (label.startsWith('STRONG_BEAR')) return 'STRONG BEAR';
  if (label.startsWith('WEAK_BULL')) return 'WEAK BULL';
  if (label.startsWith('WEAK_BEAR')) return 'WEAK BEAR';
  if (label === 'NEUTRAL_MTF') return 'NEUTRAL';
  return label;
}

function buildBreakdownMeta(alignment: AlignmentMatrix | null): string {
  if (!alignment) return 'T:— M:— Vt:— Vm:—';
  return `T:${alignment.mtf_trend_alignment.toFixed(2)} M:${alignment.mtf_momentum_alignment.toFixed(2)} Vt:${alignment.mtf_volume_alignment.toFixed(2)} Vm:${alignment.mtf_volatility_alignment.toFixed(2)}`;
}

function buildConflictBanner(alignment: AlignmentMatrix | null): string {
  if (!alignment) return '';
  if (alignment.trend_agreement_pct < 50 && alignment.timeframes_present > 0) {
    return 'TIMEFRAME CONFLICT — time horizons are working against each other';
  }
  return '';
}

function buildHeroBlock(alignment: AlignmentMatrix): AlignmentHeroBlock {
  return {
    mtf_overall_score: alignment.mtf_overall_score,
    mtf_overall_label: alignment.mtf_overall_label,
    mtf_overall_label_display: mLabel(alignment.mtf_overall_label),
    timeframes_present: alignment.timeframes_present,
    signal_cross_tf_count: alignment.signal_cross_tf_count,
    trend_agreement_pct: alignment.trend_agreement_pct,
  };
}

function buildDimensionsBlock(alignment: AlignmentMatrix): AlignmentDimensionRow[] {
  return alignment.dimensions.map((dim: AlignmentDimension, i: number) => ({
    name: ALIGNMENT_DIMENSION_NAMES[i] ?? `Dim ${i}`,
    score: Math.round(dim.score),
    state: shortStateLabel(dim.state ?? ''),
    // `dim.confidence` is already 0..100 on the wire (Rust alignment.rs
    // `(score / 100.0) * 100.0`); the screen renders `confidence.toFixed(0)%`.
    confidence: Math.round(dim.confidence ?? 0),
  }));
}

function classifyConsensus(pct: number): 'strong_consensus' | 'partial_consensus' | 'conflict' {
  if (pct >= 75) return 'strong_consensus';
  if (pct >= 50) return 'partial_consensus';
  return 'conflict';
}

function buildConsensusBlock(alignment: AlignmentMatrix): AlignmentConsensusBlock {
  const pct = alignment.trend_agreement_pct;
  return {
    trend_agreement_pct: pct,
    label: classifyConsensus(pct),
    label_display: CONSENSUS_DISPLAY[classifyConsensus(pct)],
    polarization: [
      { key: 'T',  label: 'Trend',      value: alignment.mtf_trend_alignment,      value_display: signedStr(alignment.mtf_trend_alignment, 2) },
      { key: 'M',  label: 'Momentum',   value: alignment.mtf_momentum_alignment,   value_display: signedStr(alignment.mtf_momentum_alignment, 2) },
      { key: 'Vt', label: 'Volume',     value: alignment.mtf_volume_alignment,     value_display: signedStr(alignment.mtf_volume_alignment, 2) },
      { key: 'Vm', label: 'Volatility', value: alignment.mtf_volatility_alignment, value_display: signedStr(alignment.mtf_volatility_alignment, 2) },
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
      // Screen chips render unsigned (`Trend 0.15`) — no '+' prefix.
      trend_score_display: tf.trend_score.toFixed(2),
      momentum_score: tf.momentum_score,
      momentum_score_display: tf.momentum_score.toFixed(2),
      overall_score: tf.overall_score,
      overall_score_display: tf.overall_score.toFixed(1),
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
      value_display: signedStr(value, 2),
      contribution: value * (w.pct / 100),
      // Screen shows the contribution at 2 decimals (Weight chips row).
      contribution_display: signedStr(value * (w.pct / 100), 2),
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
  const label = mLabel(alignment.mtf_overall_label).toUpperCase();
  if (pct >= 75) {
    const crossLine = crossTf > 0
      ? `${crossTf} cross-timeframe signals reinforce the current bias.`
      : 'No cross-timeframe signals detected.';
    // Mirrors the screen paragraph verbatim — the label is the REAL
    // mtf_overall_label, never a hardcoded token.
    return `Multi-timeframe alignment shows <strong>strong directional consensus</strong> (${pct.toFixed(0)}% agreement across ${present}/4 timeframes). The composite score of ${overall} is classified as <strong>${label}</strong>. ${crossLine}`;
  }
  if (pct >= 50) {
    return `Alignment shows <strong>partial consensus</strong> (${pct.toFixed(0)}% agreement). The composite score of ${overall} reflects <strong>${label}</strong> conditions with mixed input from ${present} timeframes.`;
  }
  return `Timeframes are in <strong>conflict</strong> (${pct.toFixed(0)}% agreement). Exercise caution — different time horizons are pulling in opposite directions. Wait for re-alignment before committing to directional bias.`;
}

// ── Public builder ───────────────────────────────────────────────────────

export interface AlignmentTabInputs {
  alignment: AlignmentMatrix | null;
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
 * Build the Alignment tab export payload. Mirrors
 * `AlignmentPanel.svelte` 1:1.
 */
export function buildAlignmentTabExport(args: AlignmentTabInputs): string {
  const { meta } = buildPriceBlock({
    symbol: args.symbol,
    exchange: args.exchange,
    terms: args.terms,
    fallbackMarkPrice: args.markPrice,
    tfSecs: args.tfSecs,
    timestamp: args.timestamp,
    isCompleted: args.isCompleted,
  });
  const alignment = args.alignment;
  const empty: AlignmentPayload = {
    source_tab: 'alignment',
    meta,
    header: buildHeaderBlock(args.headerSpec),
    hero: {
      mtf_overall_score: 0,
      mtf_overall_label: 'NO_DATA',
      mtf_overall_label_display: '—',
      timeframes_present: 0,
      signal_cross_tf_count: 0,
      trend_agreement_pct: 0,
    },
    breakdown_meta: buildBreakdownMeta(alignment),
    dimensions: [],
    consensus: {
      // Screen renders the "—%" placeholder + em-dash verdict; JSON carries
      // the same null-state instead of fabricating a definitive verdict.
      trend_agreement_pct: null,
      label: null,
      label_display: '\u2014',
      polarization: [
        { key: 'T',  label: 'Trend',      value: 0, value_display: signedStr(0, 2) },
        { key: 'M',  label: 'Momentum',   value: 0, value_display: signedStr(0, 2) },
        { key: 'Vt', label: 'Volume',     value: 0, value_display: signedStr(0, 2) },
        { key: 'Vm', label: 'Volatility', value: 0, value_display: signedStr(0, 2) },
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
        value_display: '\u2014',
        contribution: 0,
        contribution_display: '\u2014',
      })),
      formula: '\u2014',
    },
    interpretation: buildInterpretation(null),
    consensus_conflict_banner: '',
  };
  if (!alignment) {
    return JSON.stringify(empty, null, 2);
  }
  const payload: AlignmentPayload = {
    source_tab: 'alignment',
    meta,
    header: buildHeaderBlock(args.headerSpec),
    hero: buildHeroBlock(alignment),
    breakdown_meta: buildBreakdownMeta(alignment),
    dimensions: buildDimensionsBlock(alignment),
    consensus: buildConsensusBlock(alignment),
    per_timeframe: buildPerTimeframeBlock(alignment),
    score_calculation: buildScoreCalcBlock(alignment),
    interpretation: buildInterpretation(alignment),
    consensus_conflict_banner: buildConflictBanner(alignment),
  };
  return JSON.stringify(payload, null, 2);
}