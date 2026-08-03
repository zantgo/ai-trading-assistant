// Analysis tab builder — scoped export payload mirroring the panel.
//
// The Analysis panel renders:
//   1. Header (bias badge, confidence gauge, market_regime, market_quality)
//   2. Signals section (supporting + contradicting signals with lean)
//   3. Qualitative Assessment (6 cards: trend, momentum, structure, volatility, volume, cycle_phase)
//   4. Per-Timeframe Alignment (4 squares: trend/momentum/overall/regime)
//   5. Interpretation (market_interpretation + rationale)

import type {
  AnalysisMatrix,
  AlignmentMatrix,
  TfAlignmentInfo,
} from '../../types';
import { buildMeta } from './shared';
import type { MetaEnvelope, FilterStateBlock } from './shared';

// ── Payload types ────────────────────────────────────────────────────────

export interface AnalysisHeaderBlock {
  bias: string;
  confidence: number;
  state_confidence: number;
  market_regime: string;
  market_quality: string;
}

export interface DecomposedSignal {
  raw: string;
  timeframe: string;
  score: number | null;
  regime: string;
  signals_count: number | null;
}

export interface AnalysisSignalsBlock {
  supporting: DecomposedSignal[];
  contradicting: DecomposedSignal[];
  lean: {
    label: string;
    bullish: number;
    bearish: number;
    tone: 'bull' | 'bear' | 'split';
  };
}

export interface QualitativeAssessmentBlock {
  trend: string;
  momentum: string;
  structure: string;
  volatility: string;
  volume: string;
  cycle_phase: string;
}

export interface PerTimeframeAlignmentRow {
  name: string;
  active: boolean;
  trend: number;
  momentum: number;
  overall: number;
  regime: string;
}

export interface AnalysisPayload {
  source_tab: 'analysis';
  meta: MetaEnvelope;
  header: AnalysisHeaderBlock;
  signals: AnalysisSignalsBlock;
  qualitative_assessment: QualitativeAssessmentBlock;
  per_timeframe_alignment: PerTimeframeAlignmentRow[];
  interpretation: string;
  rationale: string;
}

// ── Helper: decompose raw signal text (matches `AnalysisPanel.svelte::decomposeSignal`) ──

function decomposeSignal(text: string): DecomposedSignal {
  const t = text || '';
  let timeframe = 'GLOBAL';
  const tfMatch = t.match(/\[?(MICRO|FAST|SLOW|MACRO|1S|3S|5S|15S|30S|1M|3M|5M|15M|30M|1H|4H|12H|1D)\]?/i);
  if (tfMatch) timeframe = tfMatch[1].toUpperCase();
  let score: number | null = null;
  const scoreMatch = t.match(/score\s+([+\-]?\d+)/i);
  if (scoreMatch) score = parseInt(scoreMatch[1], 10);
  let regime = 'UNKNOWN';
  const regimeMatch = t.match(/([a-zA-Z\-_]+)\s+regime/i);
  if (regimeMatch) regime = regimeMatch[1].toUpperCase();
  let signalsCount: number | null = null;
  const sigMatch = t.match(/(\d+)\s+signals?/i);
  if (sigMatch) signalsCount = parseInt(sigMatch[1], 10);
  return { raw: t, timeframe, score, regime, signals_count: signalsCount };
}

function buildHeaderBlock(analysis: AnalysisMatrix | null): AnalysisHeaderBlock {
  return {
    bias: analysis?.bias ?? '—',
    confidence: analysis?.confidence ?? 0,
    state_confidence: analysis?.state_confidence ?? 0,
    market_regime: analysis?.market_regime ?? '—',
    market_quality: analysis?.market_quality ?? '—',
  };
}

function buildSignalsBlock(analysis: AnalysisMatrix | null): AnalysisSignalsBlock {
  const supporting = (analysis?.supporting_signals ?? []).map((s) => ({
    text: s,
    type: 'bullish' as const,
  }));
  const contradicting = (analysis?.contradicting_signals ?? []).map((c) => ({
    text: c,
    type: 'bearish' as const,
  }));
  const bull = supporting.length;
  const bear = contradicting.length;
  const total = bull + bear;
  let lean: AnalysisSignalsBlock['lean'];
  if (total === 0) {
    lean = { label: 'No per-TF signals', bullish: 0, bearish: 0, tone: 'split' };
  } else if (bull > bear * 1.5) {
    lean = { label: `Net bullish · ${bull}↑ vs ${bear}↓`, bullish: bull, bearish: bear, tone: 'bull' };
  } else if (bear > bull * 1.5) {
    lean = { label: `Net bearish · ${bull}↑ vs ${bear}↓`, bullish: bull, bearish: bear, tone: 'bear' };
  } else {
    lean = { label: `Split signals · ${bull}↑ vs ${bear}↓`, bullish: bull, bearish: bear, tone: 'split' };
  }
  return {
    supporting: supporting.map((s) => decomposeSignal(s.text)),
    contradicting: contradicting.map((c) => decomposeSignal(c.text)),
    lean,
  };
}

function buildQualitativeBlock(analysis: AnalysisMatrix | null): QualitativeAssessmentBlock {
  return {
    trend: analysis?.trend_assessment ?? '—',
    momentum: analysis?.momentum_assessment ?? '—',
    structure: analysis?.structure_assessment ?? '—',
    volatility: analysis?.volatility_assessment ?? '—',
    volume: analysis?.volume_assessment ?? '—',
    cycle_phase: analysis?.market_phase ?? 'UNKNOWN',
  };
}

function buildPerTimeframeBlock(alignment: AlignmentMatrix | null): PerTimeframeAlignmentRow[] {
  const order = ['MICRO', 'FAST', 'SLOW', 'MACRO'];
  const alignments = alignment?.timeframe_alignments ?? [];
  return order.map((slot) => {
    const found = alignments.find((a) => a.timeframe.toUpperCase() === slot);
    return {
      name: slot,
      active: !!found,
      trend: found?.trend_score ?? 0,
      momentum: found?.momentum_score ?? 0,
      overall: found?.overall_score ?? 0,
      regime: found?.regime ?? 'AWAITING',
    };
  });
}

// ── Public builder ───────────────────────────────────────────────────────

export interface AnalysisTabInputs {
  analysis: AnalysisMatrix | null;
  alignment: AlignmentMatrix | null;
  symbol: string;
  tfSecs?: number | null;
  timestamp?: number | null;
  markPrice?: number | null;
  filterState?: FilterStateBlock;
}

/**
 * Build the Analysis tab export payload. Mirrors `AnalysisPanel.svelte` 1:1.
 */
export function buildAnalysisTabExport(args: AnalysisTabInputs): string {
  const meta = buildMeta({
    sourceTab: 'analysis',
    symbol: args.symbol,
    tfSecs: args.tfSecs ?? null,
    timestamp: args.timestamp ?? null,
    markPrice: args.markPrice ?? null,
    filterState: args.filterState,
  });
  const analysis = args.analysis;
  const payload: AnalysisPayload = {
    source_tab: 'analysis',
    meta,
    header: buildHeaderBlock(analysis),
    signals: buildSignalsBlock(analysis),
    qualitative_assessment: buildQualitativeBlock(analysis),
    per_timeframe_alignment: buildPerTimeframeBlock(args.alignment),
    interpretation: analysis?.market_interpretation ?? '',
    rationale: analysis?.rationale ?? '',
  };
  return JSON.stringify(payload, null, 2);
}
