// Tests for the Analysis tab builder.

import { describe, it, expect } from 'vitest';
import { buildAnalysisTabExport, type AnalysisPayload } from './analysisTab';
import type { AnalysisMatrix, AlignmentMatrix, TfAlignmentInfo } from '../../types';

function makeAnalysis(overrides: Partial<AnalysisMatrix> = {}): AnalysisMatrix {
  return {
    symbol: 'BTC-USDT',
    bias: 'Bullish',
    confidence: 0.72,
    state_confidence: 0.72,
    market_regime: 'TRENDING_BULL',
    trend_assessment: 'Healthy',
    momentum_assessment: 'Increasing',
    structure_assessment: 'Strong',
    volatility_assessment: 'Normal',
    volume_assessment: 'Strong',
    opportunity_analysis: 'TrendContinuation',
    market_quality: 'Good',
    market_quality_score: 75,
    market_phase: 'MARKUP',
    market_interpretation: 'Trend is healthy with momentum increasing',
    rationale: 'Multi-timeframe alignment supports the bullish bias',
    supporting_signals: [
      '[MICRO] BULLISH regime score +60 — 4 signals',
      '[FAST] BULLISH regime score +45 — 3 signals',
    ],
    contradicting_signals: [
      '[MACRO] BEARISH regime score -30 — 2 signals',
    ],
    timeframes_considered: 4,
    ...overrides,
  };
}

function makeAlignment(): AlignmentMatrix {
  const tfs: TfAlignmentInfo[] = [
    { timeframe: 'MICRO', timeframe_secs: 60, trend_score: 0.5, momentum_score: 0.3, overall_score: 30, regime: 'TRENDING_BULL', active_signals: 3, price: 65000 },
    { timeframe: 'FAST',  timeframe_secs: 180, trend_score: 0.4, momentum_score: 0.2, overall_score: 20, regime: 'TRENDING_BULL', active_signals: 2, price: 65000 },
    { timeframe: 'SLOW',  timeframe_secs: 300, trend_score: 0.1, momentum_score: 0.0, overall_score: 10, regime: 'RANGE', active_signals: 1, price: 65000 },
    { timeframe: 'MACRO', timeframe_secs: 900, trend_score: -0.1, momentum_score: -0.2, overall_score: -10, regime: 'TRENDING_BEAR', active_signals: 4, price: 65000 },
  ];
  return {
    symbol: 'BTC-USDT',
    timeframes_present: 4,
    dimensions: [],
    mtf_trend_alignment: 0.5,
    mtf_momentum_alignment: 0.3,
    mtf_volume_alignment: 0.1,
    mtf_volatility_alignment: 0.2,
    mtf_overall_score: 40,
    mtf_overall_label: 'WEAK_BULL_MTF',
    timeframe_alignments: tfs,
    signal_cross_tf_count: 5,
    trend_agreement_pct: 75,
  };
}

describe('buildAnalysisTabExport', () => {
  it('produces a valid payload with all expected top-level fields', () => {
    const json = buildAnalysisTabExport({
      analysis: makeAnalysis(),
      alignment: makeAlignment(),
      symbol: 'BTC-USDT',
      tfSecs: 60,
    });
    const p = JSON.parse(json) as AnalysisPayload;
    expect(p.source_tab).toBe('analysis');
    expect(p.meta.symbol).toBe('BTC-USDT');
    expect(p.header).toBeDefined();
    expect(p.signals).toBeDefined();
    expect(p.qualitative_assessment).toBeDefined();
    expect(p.per_timeframe_alignment).toBeDefined();
    expect(p.interpretation).toBeDefined();
    expect(p.rationale).toBeDefined();
  });

  it('header captures bias, confidence, market_regime, market_quality', () => {
    const p = JSON.parse(buildAnalysisTabExport({
      analysis: makeAnalysis(), alignment: makeAlignment(), symbol: 'BTC-USDT',
    })) as AnalysisPayload;
    expect(p.header.bias).toBe('Bullish');
    expect(p.header.confidence).toBe(0.72);
    expect(p.header.market_regime).toBe('TRENDING_BULL');
    expect(p.header.market_quality).toBe('Good');
  });

  it('decomposes supporting signals into structured fields', () => {
    const p = JSON.parse(buildAnalysisTabExport({
      analysis: makeAnalysis(), alignment: makeAlignment(), symbol: 'BTC-USDT',
    })) as AnalysisPayload;
    expect(p.signals.supporting.length).toBe(2);
    expect(p.signals.supporting[0].timeframe).toBe('MICRO');
    expect(p.signals.supporting[0].score).toBe(60);
    expect(p.signals.supporting[0].signals_count).toBe(4);
    expect(p.signals.supporting[0].regime).toBe('BULLISH');
  });

  it('decomposes contradicting signals into structured fields', () => {
    const p = JSON.parse(buildAnalysisTabExport({
      analysis: makeAnalysis(), alignment: makeAlignment(), symbol: 'BTC-USDT',
    })) as AnalysisPayload;
    expect(p.signals.contradicting.length).toBe(1);
    expect(p.signals.contradicting[0].timeframe).toBe('MACRO');
    expect(p.signals.contradicting[0].score).toBe(-30);
  });

  it('computes lean correctly', () => {
    const p = JSON.parse(buildAnalysisTabExport({
      analysis: makeAnalysis(), alignment: makeAlignment(), symbol: 'BTC-USDT',
    })) as AnalysisPayload;
    expect(p.signals.lean.bullish).toBe(2);
    expect(p.signals.lean.bearish).toBe(1);
    expect(p.signals.lean.tone).toBe('bull');
  });

  it('qualitative_assessment captures all 6 fields including cycle_phase', () => {
    const p = JSON.parse(buildAnalysisTabExport({
      analysis: makeAnalysis(), alignment: makeAlignment(), symbol: 'BTC-USDT',
    })) as AnalysisPayload;
    expect(p.qualitative_assessment.trend).toBe('Healthy');
    expect(p.qualitative_assessment.momentum).toBe('Increasing');
    expect(p.qualitative_assessment.structure).toBe('Strong');
    expect(p.qualitative_assessment.volatility).toBe('Normal');
    expect(p.qualitative_assessment.volume).toBe('Strong');
    expect(p.qualitative_assessment.cycle_phase).toBe('MARKUP');
  });

  it('per_timeframe_alignment is sorted MICRO→FAST→SLOW→MACRO with active flag', () => {
    const p = JSON.parse(buildAnalysisTabExport({
      analysis: makeAnalysis(), alignment: makeAlignment(), symbol: 'BTC-USDT',
    })) as AnalysisPayload;
    expect(p.per_timeframe_alignment.map((t) => t.name)).toEqual(['MICRO', 'FAST', 'SLOW', 'MACRO']);
    expect(p.per_timeframe_alignment[0].active).toBe(true);
    expect(p.per_timeframe_alignment[0].trend).toBe(0.5);
  });

  it('interpretation and rationale capture raw text', () => {
    const p = JSON.parse(buildAnalysisTabExport({
      analysis: makeAnalysis(), alignment: makeAlignment(), symbol: 'BTC-USDT',
    })) as AnalysisPayload;
    expect(p.interpretation).toBe('Trend is healthy with momentum increasing');
    expect(p.rationale).toBe('Multi-timeframe alignment supports the bullish bias');
  });

  it('produces a valid payload when analysis is null', () => {
    const p = JSON.parse(buildAnalysisTabExport({
      analysis: null, alignment: null, symbol: 'BTC-USDT',
    })) as AnalysisPayload;
    expect(p.header.bias).toBe('—');
    expect(p.signals.supporting).toEqual([]);
    expect(p.signals.contradicting).toEqual([]);
    expect(p.per_timeframe_alignment.length).toBe(4);
    expect(p.per_timeframe_alignment[0].active).toBe(false);
  });
});
