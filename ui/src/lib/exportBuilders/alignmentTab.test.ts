// Tests for the Alignment tab builder.

import { describe, it, expect } from 'vitest';
import { buildAlignmentTabExport, ALIGNMENT_DIMENSION_NAMES, type AlignmentPayload } from './alignmentTab';
import type { AlignmentMatrix, AlignmentDimension, TfAlignmentInfo } from '../../types';

function makeAlignment(overrides: Partial<AlignmentMatrix> = {}): AlignmentMatrix {
  const dims: AlignmentDimension[] = [
    { score: 75, state: 'BULLISH', confidence: 80 },
    { score: 60, state: 'BULLISH', confidence: 70 },
    { score: 50, state: 'NEUTRAL', confidence: 65 },
    { score: 40, state: 'NEUTRAL', confidence: 60 },
    { score: 55, state: 'BULLISH', confidence: 70 },
    { score: 65, state: 'BULLISH', confidence: 75 },
    { score: 70, state: 'BULLISH', confidence: 80 },
    { score: 80, state: 'BULLISH', confidence: 85 },
    { score: 45, state: 'NEUTRAL', confidence: 60 },
    { score: 60, state: 'BULLISH', confidence: 70 },
  ];
  const tfs: TfAlignmentInfo[] = [
    { timeframe: 'MICRO', timeframe_secs: 60, trend_score: 0.5, momentum_score: 0.3, overall_score: 30, regime: 'TRENDING_BULL', active_signals: 3, price: 65000 },
    { timeframe: 'FAST',  timeframe_secs: 180, trend_score: 0.4, momentum_score: 0.2, overall_score: 20, regime: 'TRENDING_BULL', active_signals: 2, price: 65000 },
    { timeframe: 'SLOW',  timeframe_secs: 300, trend_score: 0.1, momentum_score: 0.0, overall_score: 10, regime: 'RANGE', active_signals: 1, price: 65000 },
    { timeframe: 'MACRO', timeframe_secs: 900, trend_score: -0.1, momentum_score: -0.2, overall_score: -10, regime: 'TRENDING_BEAR', active_signals: 4, price: 65000 },
  ];
  return {
    symbol: 'BTC-USDT',
    timeframes_present: 4,
    dimensions: dims,
    mtf_trend_alignment: 0.5,
    mtf_momentum_alignment: 0.3,
    mtf_volume_alignment: 0.1,
    mtf_volatility_alignment: 0.2,
    mtf_overall_score: 40,
    mtf_overall_label: 'WEAK_BULL_MTF',
    timeframe_alignments: tfs,
    signal_cross_tf_count: 5,
    trend_agreement_pct: 75,
    ...overrides,
  };
}

describe('buildAlignmentTabExport', () => {
  it('produces a valid payload with all expected top-level fields', () => {
    const json = buildAlignmentTabExport({
      alignment: makeAlignment(),
      symbol: 'BTC-USDT',
      tfSecs: 60,
    });
    const p = JSON.parse(json) as AlignmentPayload;
    expect(p.source_tab).toBe('alignment');
    expect(p.meta.symbol).toBe('BTC-USDT');
    expect(p.hero).toBeDefined();
    expect(p.dimensions).toBeDefined();
    expect(p.consensus).toBeDefined();
    expect(p.per_timeframe).toBeDefined();
    expect(p.score_calculation).toBeDefined();
    expect(p.interpretation).toBeDefined();
  });

  it('captures hero block correctly', () => {
    const p = JSON.parse(buildAlignmentTabExport({
      alignment: makeAlignment(), symbol: 'BTC-USDT',
    })) as AlignmentPayload;
    expect(p.hero.mtf_overall_score).toBe(40);
    expect(p.hero.mtf_overall_label).toBe('WEAK_BULL_MTF');
    expect(p.hero.timeframes_present).toBe(4);
    expect(p.hero.signal_cross_tf_count).toBe(5);
    expect(p.hero.trend_agreement_pct).toBe(75);
  });

  it('captures all 10 named dimensions in canonical order', () => {
    const p = JSON.parse(buildAlignmentTabExport({
      alignment: makeAlignment(), symbol: 'BTC-USDT',
    })) as AlignmentPayload;
    expect(p.dimensions.length).toBe(10);
    p.dimensions.forEach((dim, i) => {
      expect(dim.name).toBe(ALIGNMENT_DIMENSION_NAMES[i]);
    });
  });

  it('classifies consensus label correctly', () => {
    const p1 = JSON.parse(buildAlignmentTabExport({
      alignment: makeAlignment({ trend_agreement_pct: 80 }), symbol: 'BTC-USDT',
    })) as AlignmentPayload;
    expect(p1.consensus.label).toBe('strong_consensus');

    const p2 = JSON.parse(buildAlignmentTabExport({
      alignment: makeAlignment({ trend_agreement_pct: 60 }), symbol: 'BTC-USDT',
    })) as AlignmentPayload;
    expect(p2.consensus.label).toBe('partial_consensus');

    const p3 = JSON.parse(buildAlignmentTabExport({
      alignment: makeAlignment({ trend_agreement_pct: 40 }), symbol: 'BTC-USDT',
    })) as AlignmentPayload;
    expect(p3.consensus.label).toBe('conflict');
  });

  it('polarization captures 4-axis values', () => {
    const p = JSON.parse(buildAlignmentTabExport({
      alignment: makeAlignment(), symbol: 'BTC-USDT',
    })) as AlignmentPayload;
    expect(p.consensus.polarization.length).toBe(4);
    expect(p.consensus.polarization[0].key).toBe('T');
    expect(p.consensus.polarization[0].value).toBe(0.5);
    expect(p.consensus.polarization[1].key).toBe('M');
    expect(p.consensus.polarization[2].key).toBe('Vt');
    expect(p.consensus.polarization[3].key).toBe('Vm');
  });

  it('per_timeframe is sorted MICRO→FAST→SLOW→MACRO', () => {
    const p = JSON.parse(buildAlignmentTabExport({
      alignment: makeAlignment({
        timeframe_alignments: [
          { timeframe: 'MACRO', timeframe_secs: 900, trend_score: -0.1, momentum_score: -0.2, overall_score: -10, regime: 'TRENDING_BEAR', active_signals: 4, price: 65000 },
          { timeframe: 'MICRO', timeframe_secs: 60, trend_score: 0.5, momentum_score: 0.3, overall_score: 30, regime: 'TRENDING_BULL', active_signals: 3, price: 65000 },
          { timeframe: 'SLOW',  timeframe_secs: 300, trend_score: 0.1, momentum_score: 0.0, overall_score: 10, regime: 'RANGE', active_signals: 1, price: 65000 },
          { timeframe: 'FAST',  timeframe_secs: 180, trend_score: 0.4, momentum_score: 0.2, overall_score: 20, regime: 'TRENDING_BULL', active_signals: 2, price: 65000 },
        ],
      }),
      symbol: 'BTC-USDT',
    })) as AlignmentPayload;
    expect(p.per_timeframe.map((t) => t.timeframe)).toEqual(['MICRO', 'FAST', 'SLOW', 'MACRO']);
  });

  it('score_calculation captures the formula string', () => {
    const p = JSON.parse(buildAlignmentTabExport({
      alignment: makeAlignment(), symbol: 'BTC-USDT',
    })) as AlignmentPayload;
    expect(p.score_calculation.formula).toContain('0.5');
    expect(p.score_calculation.formula).toContain('0.3');
    expect(p.score_calculation.formula).toContain('0.1');
    expect(p.score_calculation.weights.length).toBe(4);
    expect(p.score_calculation.weights[0].pct).toBe(50);
    expect(p.score_calculation.weights[1].pct).toBe(30);
  });

  it('interpretation reflects consensus level', () => {
    const p1 = JSON.parse(buildAlignmentTabExport({
      alignment: makeAlignment({ trend_agreement_pct: 80 }), symbol: 'BTC-USDT',
    })) as AlignmentPayload;
    expect(p1.interpretation).toContain('strong directional consensus');

    const p2 = JSON.parse(buildAlignmentTabExport({
      alignment: makeAlignment({ trend_agreement_pct: 40 }), symbol: 'BTC-USDT',
    })) as AlignmentPayload;
    expect(p2.interpretation).toContain('conflict');
  });

  it('produces a valid payload when alignment is null', () => {
    const p = JSON.parse(buildAlignmentTabExport({
      alignment: null, symbol: 'BTC-USDT',
    })) as AlignmentPayload;
    expect(p.hero.mtf_overall_score).toBe(0);
    expect(p.dimensions).toEqual([]);
    expect(p.per_timeframe).toEqual([]);
    expect(p.interpretation).toContain('Awaiting alignment data');
  });
});
