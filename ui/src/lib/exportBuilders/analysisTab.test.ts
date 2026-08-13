// Regression tests for the v7.0-audit Analysis tab export.

import { describe, it, expect } from 'vitest';
import { buildAnalysisTabExport } from './analysisTab';
import type { LayerHeaderSpec } from '../layerHeader';
import type { AnalysisMatrix, AlignmentMatrix } from '../../types';

const headerSpec: LayerHeaderSpec = {
  layerNumber: 3,
  layerName: 'Analysis',
  badge: { label: 'Strong Bullish', color: '#22c55e', background: 'rgba(34,197,94,0.08)', state: 'valid' },
  meta: [],
  status: 'live',
};

function makeAnalysis(): AnalysisMatrix {
  return {
    bias: 'StrongBullish',
    confidence: 0.78,
    state_confidence: 0.78,
    market_regime: 'TRENDING_BULL',
    market_quality: 'Good',
    market_phase: 'MARK_UP',
    timeframes_considered: 4,
    supporting_signals: [
      'MICRO (bullish): score +5, TRENDING_BULL regime, 3 signals',
      'FAST (bullish): score +2, TRENDING_BULL regime, 2 signals',
    ],
    contradicting_signals: [
      'SLOW (bearish): score -3, RANGING regime, 1 signal',
    ],
    trend_assessment: 'Trending',
    momentum_assessment: 'Strong Bullish',
    structure_assessment: 'Breakout',
    volatility_assessment: 'Expanding',
    volume_assessment: 'Increasing',
    market_interpretation: 'Market is in a strong bullish phase.',
    rationale: 'Composite confluence score 78.',
  } as unknown as AnalysisMatrix;
}

function makeAlignment(): AlignmentMatrix {
  return {
    mtf_overall_score: 75,
    mtf_overall_label: 'STRONG_BULLISH',
    timeframes_present: 4,
    signal_cross_tf_count: 3,
    trend_agreement_pct: 75,
    mtf_trend_alignment: 0.45,
    mtf_momentum_alignment: 0.3,
    mtf_volume_alignment: 0.1,
    mtf_volatility_alignment: 0.2,
    timeframe_alignments: [
      { timeframe: 'MICRO', trend_score: 0.42, momentum_score: 0.3, overall_score: 1.0, regime: 'TRENDING_BULL', active_signals: 5 },
      { timeframe: 'FAST', trend_score: 0.5, momentum_score: 0.2, overall_score: 0.7, regime: 'EXPANSION', active_signals: 3 },
    ],
  } as unknown as AlignmentMatrix;
}

describe('buildAnalysisTabExport', () => {
  it('header chrome is present; body bias is display-formatted', () => {
    const p = JSON.parse(buildAnalysisTabExport({
      analysis: makeAnalysis(),
      alignment: makeAlignment(),
      symbol: 'BTC-USDT',
      markPrice: 63390,
      headerSpec,
    }));
    expect(p.header.layer_name).toBe('Analysis');
    expect(p.body.bias).toBe('Strong Bullish');
    expect(p.body.confidence_pct).toBe(78);
  });

  it('signal_lean_hero is emitted with raw percentage numbers', () => {
    const p = JSON.parse(buildAnalysisTabExport({
      analysis: makeAnalysis(),
      alignment: makeAlignment(),
      symbol: 'BTC-USDT',
      markPrice: 63390,
      headerSpec,
    }));
    expect(p.signal_lean_hero).not.toBeNull();
    expect(p.signal_lean_hero.bullish_pct).toBe(67);
    expect(p.signal_lean_hero.bearish_pct).toBe(33);
    expect(p.signal_lean_hero.tone).toBe('bull');
  });

  it('signals decompose into key/period/score_display', () => {
    const p = JSON.parse(buildAnalysisTabExport({
      analysis: makeAnalysis(),
      alignment: makeAlignment(),
      symbol: 'BTC-USDT',
      markPrice: 63390,
      headerSpec,
    }));
    const supporting = p.signals.supporting;
    expect(supporting.length).toBe(2);
    expect(supporting[0].score).toBe(5);
    expect(supporting[0].score_display).toBe('+5');
    expect(supporting[0].timeframe).toBe('MICRO');
    expect(supporting[0].regime).toBe('TRENDING_BULL');
    expect(supporting[0].signals_count).toBe(3);
  });

  it('per-TF alignment uses OFFLINE for missing TFs and sign-prefixed displays', () => {
    const p = JSON.parse(buildAnalysisTabExport({
      analysis: makeAnalysis(),
      alignment: makeAlignment(),
      symbol: 'BTC-USDT',
      markPrice: 63390,
      headerSpec,
    }));
    const tfs = p.per_timeframe_alignment;
    expect(tfs).toHaveLength(4);
    const slow = tfs.find((t: { name: string }) => t.name === 'SLOW');
    expect(slow.active).toBe(false);
    expect(slow.regime).toBe('OFFLINE');
    const micro = tfs.find((t: { name: string }) => t.name === 'MICRO');
    expect(micro.trend_display).toBe('+0.42');
    expect(micro.overall_display).toBe('+1.0');
  });

  it('qualitative cycle_phase is display-formatted', () => {
    const p = JSON.parse(buildAnalysisTabExport({
      analysis: makeAnalysis(),
      alignment: makeAlignment(),
      symbol: 'BTC-USDT',
      markPrice: 63390,
      headerSpec,
    }));
    expect(p.qualitative_assessment.cycle_phase).toBe('MARK UP');
  });

  it('split-tone hero label matches the screen ("Split signals", no parenthetical)', () => {
    const split = {
      ...makeAnalysis(),
      supporting_signals: ['MICRO (bullish): score +5, TRENDING_BULL regime, 1 signal'],
      contradicting_signals: ['SLOW (bearish): score -3, RANGING regime, 1 signal'],
    };
    const p = JSON.parse(buildAnalysisTabExport({
      analysis: split,
      alignment: makeAlignment(),
      symbol: 'BTC-USDT',
      markPrice: 63390,
      headerSpec,
    }));
    expect(p.signal_lean_hero.tone).toBe('split');
    expect(p.signal_lean_hero.label_html).toBe('Split signals');
    expect(p.signal_lean_hero.meta_html).toBe('1↑ vs 1↓');
  });

  it('null analysis still emits the hero placeholders the screen renders', () => {
    const p = JSON.parse(buildAnalysisTabExport({
      analysis: null,
      alignment: null,
      symbol: 'BTC-USDT',
      markPrice: 63390,
      headerSpec,
    }));
    expect(p.signal_lean_hero).not.toBeNull();
    expect(p.signal_lean_hero.label_html).toBe('No signals');
    expect(p.signal_lean_hero.meta_html).toBe('Waiting for cross-TF consensus');
  });

  it('null analysis emits em-dash placeholders for qualitative, per-TF and rationale', () => {
    const p = JSON.parse(buildAnalysisTabExport({
      analysis: null,
      alignment: null,
      symbol: 'BTC-USDT',
      markPrice: 63390,
      headerSpec,
    }));
    for (const key of ['trend', 'momentum', 'structure', 'volatility', 'volume']) {
      expect(p.qualitative_assessment[key]).toBe('—');
    }
    expect(p.qualitative_assessment.cycle_phase).toBe('—');
    expect(p.rationale).toBe('—');
    expect(p.per_timeframe_alignment).toHaveLength(4);
    for (const tf of p.per_timeframe_alignment) {
      expect(tf.trend_display).toBe('—');
      expect(tf.regime).toBe('OFFLINE');
    }
  });

  it('signal score/count placeholders use "—" exactly like the screen', () => {
    const p = JSON.parse(buildAnalysisTabExport({
      analysis: {
        ...makeAnalysis(),
        supporting_signals: ['GLOBAL (neutral): no score, awaiting feed'],
      } as unknown as AnalysisMatrix,
      alignment: makeAlignment(),
      symbol: 'BTC-USDT',
      markPrice: 63390,
      headerSpec,
    }));
    const row = p.signals.supporting[0];
    expect(row.score).toBeNull();
    expect(row.score_display).toBe('—');
    expect(row.signals_count).toBeNull();
    expect(row.signals_count_display).toBe('—');
  });
});