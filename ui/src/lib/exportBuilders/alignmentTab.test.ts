// Regression tests for the v7.0-audit Alignment tab export.

import { describe, it, expect } from 'vitest';
import { buildAlignmentTabExport } from './alignmentTab';
import type { LayerHeaderSpec } from '../layerHeader';
import type { AlignmentMatrix } from '../../types';

const headerSpec: LayerHeaderSpec = {
  layerNumber: 2,
  layerName: 'Alignment',
  badge: { label: 'STRONG BULL', color: '#22c55e', background: 'rgba(34,197,94,0.08)', state: 'valid' },
  meta: [],
  status: 'live',
};

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
      { timeframe: 'MICRO', trend_score: 0.45, momentum_score: 0.3, overall_score: 1.0, regime: 'TRENDING_BULL', active_signals: 5 },
    ],
    dimensions: [
      // Confidence is 0..100 on the wire (Rust `alignment.rs`),
      // mirroring the screen's `confidence.toFixed(0)%` reading.
      { score: 75, state: 'STRONG_BULLISH', confidence: 78 },
      { score: 60, state: 'BULLISH', confidence: 72 },
      { score: 45, state: 'NEUTRAL', confidence: 65 },
      { score: 30, state: 'BEARISH', confidence: 58 },
      { score: 70, state: 'STRONG_BULLISH', confidence: 75 },
      { score: 65, state: 'BULLISH', confidence: 70 },
      { score: 80, state: 'STRONG_BULLISH', confidence: 82 },
      { score: 70, state: 'BULLISH', confidence: 70 },
      { score: 55, state: 'NEUTRAL', confidence: 62 },
      { score: 65, state: 'BULLISH', confidence: 68 },
    ],
  } as unknown as AlignmentMatrix;
}

describe('buildAlignmentTabExport', () => {
  it('consensus exposes label_display and integer-rounded agreement', () => {
    const p = JSON.parse(buildAlignmentTabExport({
      alignment: makeAlignment(),
      symbol: 'BTC-USDT',
      markPrice: 63390,
      headerSpec,
    }));
    expect(p.consensus.trend_agreement_pct).toBe(75);
    expect(p.consensus.label).toBe('strong_consensus');
    expect(p.consensus.label_display).toBe('Strong consensus — timeframes aligned');
  });

  it('polarization values carry sign-prefixed display strings', () => {
    const p = JSON.parse(buildAlignmentTabExport({
      alignment: makeAlignment(),
      symbol: 'BTC-USDT',
      markPrice: 63390,
      headerSpec,
    }));
    const trend = p.consensus.polarization.find((x: { key: string }) => x.key === 'T');
    expect(trend.value).toBe(0.45);
    expect(trend.value_display).toBe('+0.45');
    expect(trend.label).toBe('Trend');
  });

  it('dimensions use short state labels and percentage confidence', () => {
    const p = JSON.parse(buildAlignmentTabExport({
      alignment: makeAlignment(),
      symbol: 'BTC-USDT',
      markPrice: 63390,
      headerSpec,
    }));
    expect(p.dimensions[0].state).toBe('STRONG');
    expect(p.dimensions[0].score).toBe(75);
    // `confidence` is already 0..100 on the wire — the export mirrors the
    // screen's `confidence.toFixed(0)%` (no ×100 inflation).
    expect(p.dimensions[0].confidence).toBe(78);
    expect(p.dimensions[1].confidence).toBe(72);
  });

  it('per-timeframe rows carry display variants', () => {
    const p = JSON.parse(buildAlignmentTabExport({
      alignment: makeAlignment(),
      symbol: 'BTC-USDT',
      markPrice: 63390,
      headerSpec,
    }));
    expect(p.per_timeframe[0].trend_score_display).toBe('0.45');
    expect(p.per_timeframe[0].overall_score_display).toBe('1.0');
  });

  it('empty state still carries meta identity', () => {
    const p = JSON.parse(buildAlignmentTabExport({
      alignment: null,
      symbol: 'BTC-USDT',
      markPrice: 63390,
      headerSpec,
    }));
    expect(p.meta.pair).toBe('BTC-USDT');
    expect(p.hero.mtf_overall_label).toBe('NO_DATA');
    expect(p.dimensions).toEqual([]);
  });

  it('emits breakdown_meta caption and conflict banner when applicable', () => {
    const conflictAlignment = {
      ...makeAlignment(),
      trend_agreement_pct: 30,
    } as AlignmentMatrix;
    const p = JSON.parse(buildAlignmentTabExport({
      alignment: conflictAlignment,
      symbol: 'BTC-USDT',
      markPrice: 63390,
      headerSpec,
    }));
    expect(p.breakdown_meta).toContain('T:');
    expect(p.breakdown_meta).toContain('M:');
    expect(p.breakdown_meta).toContain('Vt:');
    expect(p.breakdown_meta).toContain('Vm:');
    expect(p.consensus_conflict_banner).toContain('TIMEFRAME CONFLICT');
    expect(p.hero.mtf_overall_label_display).toMatch(/STRONG|WEAK|NEUTRAL/);
  });

  it('null state mirrors the screen placeholders (—%, — verdict, +0.00, — weights)', () => {
    const p = JSON.parse(buildAlignmentTabExport({
      alignment: null,
      symbol: 'BTC-USDT',
      markPrice: 63390,
      headerSpec,
    }));
    // Consensus: screen renders "—%" + em-dash verdict — JSON carries the
    // same null-state instead of a fabricated "Conflict" verdict.
    expect(p.consensus.trend_agreement_pct).toBeNull();
    expect(p.consensus.label).toBeNull();
    expect(p.consensus.label_display).toBe('—');
    // Polarization: screen renders "+0.00" for the zero fallbacks.
    for (const axis of p.consensus.polarization) {
      expect(axis.value_display).toBe('+0.00');
    }
    // Score calculation: screen renders "—" for values, contributions and
    // the formula.
    for (const w of p.score_calculation.weights) {
      expect(w.value_display).toBe('—');
      expect(w.contribution_display).toBe('—');
    }
    expect(p.score_calculation.formula).toBe('—');
    expect(p.dimensions).toEqual([]);
  });

  it('NO_DATA dimension state renders "NO DATA" on both surfaces', () => {
    const p = JSON.parse(buildAlignmentTabExport({
      alignment: {
        ...makeAlignment(),
        dimensions: [
          { score: 0, state: 'NO_DATA', confidence: 0 },
          { score: 60, state: 'BULLISH', confidence: 72 },
        ],
      } as unknown as AlignmentMatrix,
      symbol: 'BTC-USDT',
      markPrice: 63390,
      headerSpec,
    }));
    expect(p.dimensions[0].state).toBe('NO DATA');
    expect(p.dimensions[1].state).toBe('BULLISH');
  });

  it('consensus trend_agreement_pct keeps raw float precision (bar width parity)', () => {
    const p = JSON.parse(buildAlignmentTabExport({
      alignment: {
        ...makeAlignment(),
        trend_agreement_pct: 71.6,
      } as AlignmentMatrix,
      symbol: 'BTC-USDT',
      markPrice: 63390,
      headerSpec,
    }));
    // Screen text uses toFixed(0) → "72%", bar width toFixed(1) → "71.6%".
    expect(p.consensus.trend_agreement_pct).toBe(71.6);
    expect(p.consensus.label).toBe('partial_consensus');
    expect(p.hero.trend_agreement_pct).toBe(71.6);
  });
});