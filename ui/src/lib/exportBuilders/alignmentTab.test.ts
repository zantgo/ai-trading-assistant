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
    expect(p.consensus.label_display).toBe('Strong Consensus');
  });

  it('axes values carry sign-prefixed display strings', () => {
    const p = JSON.parse(buildAlignmentTabExport({
      alignment: makeAlignment(),
      symbol: 'BTC-USDT',
      markPrice: 63390,
      headerSpec,
    }));
    const trend = p.consensus.axes.find((x: { key: string }) => x.key === 'Trend');
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

  it('v6.10.19d (A): breakdown_meta caption removed; conflict banner kept', () => {
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
    expect('breakdown_meta' in p).toBe(false);
    expect(p.consensus_conflict_banner).toContain('TIMEFRAME MISALIGNMENT');
    expect(p.hero.mtf_overall_label_display).toMatch(/STRONG|WEAK|NEUTRAL/);
  });

  it('FIX-H2: score_calculation mirrors the wire blend_weights (thin-participation reweight)', () => {
    const reweighted = {
      ...makeAlignment(),
      // Self-consistent with the reweighted blend:
      // 0.55×0.45 + 0.35×0.3 + 0.05×0.1 + 0.05×0.2 = 0.36 → 36.0
      mtf_overall_score: 36,
      blend_weights: [
        ['Trend', 0.55],
        ['Momentum', 0.35],
        ['Volume', 0.05],
        ['Volatility', 0.05],
      ] as Array<[string, number]>,
    } as AlignmentMatrix;
    const p = JSON.parse(buildAlignmentTabExport({
      alignment: reweighted,
      symbol: 'BTC-USDT',
      markPrice: 63390,
      headerSpec,
    }));
    const byKey = Object.fromEntries(p.score_calculation.weights.map((w: { key: string; pct: number }) => [w.key, w.pct]));
    expect(byKey.Trend).toBe(55);
    expect(byKey.Momentum).toBe(35);
    expect(byKey.Volume).toBe(5);
    expect(byKey.Volatility).toBe(5);
  });

  it('v6.10.18: legacy "Vt"/"Vm" wire keys normalize to Volume/Volatility', () => {
    const legacy = {
      ...makeAlignment(),
      mtf_overall_score: 36,
      blend_weights: [
        ['T', 0.55],
        ['M', 0.35],
        ['Vt', 0.05],
        ['Vm', 0.05],
      ] as Array<[string, number]>,
    } as AlignmentMatrix;
    const p = JSON.parse(buildAlignmentTabExport({
      alignment: legacy,
      symbol: 'BTC-USDT',
      markPrice: 63390,
      headerSpec,
    }));
    const byKey = Object.fromEntries(p.score_calculation.weights.map((w: { key: string; pct: number }) => [w.key, w.pct]));
    expect(byKey.Volume).toBe(5);
    expect(byKey.Volatility).toBe(5);
    expect(byKey.Vt).toBeUndefined();
    expect(byKey.Vm).toBeUndefined();
    // Volume stays bound to mtf_volume_alignment, Volatility to
    // mtf_volatility_alignment — the legacy keys were swapped vs. spec.
    const vol = p.score_calculation.weights.find((w: { key: string }) => w.key === 'Volume');
    expect(vol.value).toBe(0.1);
    const vola = p.score_calculation.weights.find((w: { key: string }) => w.key === 'Volatility');
    expect(vola.value).toBe(0.2);
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
    // Axes: screen renders "+0.00" for the zero fallbacks.
    for (const axis of p.consensus.axes) {
      expect(axis.value_display).toBe('+0.00');
    }
    // Score calculation: screen renders "—" for values and contributions.
    for (const w of p.score_calculation.weights) {
      expect(w.value_display).toBe('—');
      expect(w.contribution_display).toBe('—');
    }
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