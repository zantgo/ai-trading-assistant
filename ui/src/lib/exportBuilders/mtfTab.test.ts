// Regression tests for the v7.0-audit MTF tab export.

import { describe, it, expect } from 'vitest';
import { buildMtfExportJson } from './mtfTab';
import type { LayerHeaderSpec } from '../layerHeader';
import type { TimeframeTelemetry, IndicatorMeta, IndicatorDto } from '../../types';

const headerSpec: LayerHeaderSpec = {
  layerNumber: 1,
  layerName: 'Metrics · MTF',
  badge: { label: 'MTF SYNC', color: '#22c55e', background: 'rgba(34,197,94,0.08)', state: 'valid' },
  meta: [],
  status: 'live',
};

const registry: IndicatorMeta[] = [
  {
    key: 'rsi_14',
    display_name: 'RSI 14',
    group: 'Momentum',
    class: 'Hybrid',
    value_format: 'decimals2',
    value_source: 'raw',
    default_enabled: true,
    directional: true,
  } as unknown as IndicatorMeta,
];

function makeTf(label: string, priceText: string): TimeframeTelemetry {
  return {
    barDurationSec: 60,
    isCompleted: true,
    pipelineState: 'OK',
    priceText,
    latestSnapshot: { timestamp: Math.floor(Date.now() / 1000) - 5 },
    indicators: {
      rsi_14: { raw_value: 62.4, normalized: 0.24, confidence: 0.78, state_label: 'BULLISH', signals: [], values: {} },
    } as unknown as Record<string, IndicatorDto>,
  } as unknown as TimeframeTelemetry;
}

describe('buildMtfExportJson', () => {
  it('emits groups with display labels and indicators with key/period split', () => {
    const p = JSON.parse(buildMtfExportJson({
      symbol: 'BTC-USDT',
      markPrice: 63390,
      headerSpec,
      registry,
      pair: {
        microTerm: makeTf('Micro', '63390'),
        fastTerm: makeTf('Fast', '63395'),
        slowTerm: makeTf('Slow', '63300'),
        macroTerm: makeTf('Macro', '63000'),
      },
    }));
    expect(p.meta.pair).toBe('BTC-USDT');
    expect(p.header.layer_name).toBe('Metrics · MTF');
    expect(p.groups[0].label).toBe('Momentum');
    const rsi = p.indicators.find((i: { key: string }) => i.key === 'rsi');
    expect(rsi.period).toBe(14);
    expect(rsi.display_name).toBe('RSI 14');
    expect(rsi.values).toHaveLength(4);
    expect(rsi.values[0].normalized_display).toBe('+0.24');
    expect(typeof rsi.agreement).toBe('number');
    expect(['BULL', 'BEAR', 'MIXED']).toContain(rsi.agreement_label);
  });

  it('timeframes carry fibonacci summary with status strings', () => {
    const p = JSON.parse(buildMtfExportJson({
      symbol: 'BTC-USDT',
      markPrice: 63390,
      headerSpec,
      registry,
      pair: {
        microTerm: makeTf('Micro', '63390'),
        fastTerm: makeTf('Fast', '63395'),
        slowTerm: makeTf('Slow', '63300'),
        macroTerm: makeTf('Macro', '63000'),
      },
    }));
    expect(p.timeframes).toHaveLength(4);
    expect(p.timeframes[0].fibonacci_summary.present).toBe(false);
    expect(p.timeframes[0].fibonacci_summary.swing_direction).toBe('NEUTRAL SWING');
  });

  // Regression (D1): the cross-TF merge compared `ind.confidence_pct`
  // (0..100) against the stored `confidence` (0..1 fraction), so every
  // later TF with confidence >= 2% won and the LAST timeframe (Macro)
  // always supplied the merged values. MTF aggregates must come from the
  // HIGHEST-CONFIDENCE timeframe instead.
  it('merged aggregates come from the highest-confidence TF, not the last one', () => {
    const confRegistry: IndicatorMeta[] = [
      {
        key: 'rsi_14',
        display_name: 'RSI 14',
        group: 'Momentum',
        class: 'Hybrid',
        value_format: 'decimals2',
        value_source: 'raw',
        default_enabled: true,
        directional: true,
      } as unknown as IndicatorMeta,
    ];
    const makeTfWith = (label: string, conf: number, norm: number, signalLabel: string): TimeframeTelemetry => ({
      barDurationSec: 60,
      isCompleted: true,
      pipelineState: 'OK',
      priceText: '63000',
      latestSnapshot: { timestamp: Math.floor(Date.now() / 1000) - 5 },
      indicators: {
        rsi_14: {
          raw_value: 62.4,
          normalized: norm,
          confidence: conf,
          state_label: norm >= 0 ? 'BULLISH' : 'BEARISH',
          signals: [{
            kind: 'Threshold',
            direction: norm >= 0 ? 'Bullish' : 'Bearish',
            status: 'Active',
            label: signalLabel,
            strength: 0,
            age_bars: 0,
            display_label: signalLabel,
          }],
          values: {},
        },
      } as unknown as Record<string, IndicatorDto>,
    } as unknown as TimeframeTelemetry);

    // Micro has the HIGHEST confidence (1.00) with a bullish reading;
    // Fast/Slow/Macro have lower confidence. Pre-fix, Macro (confidence
    // 0.5 = 50%) beat the stored fraction 1.0... 50 > 1.0 → macro won.
    const p = JSON.parse(buildMtfExportJson({
      symbol: 'BTC-USDT',
      markPrice: 63000,
      headerSpec,
      registry: confRegistry,
      pair: {
        microTerm: makeTfWith('Micro', 1.0, 0.9, 'RSI_MICRO_BULLISH'),
        fastTerm: makeTfWith('Fast', 0.6, -0.5, 'RSI_FAST_BEARISH'),
        slowTerm: makeTfWith('Slow', 0.4, 0.2, 'RSI_SLOW_BULLISH'),
        macroTerm: makeTfWith('Macro', 0.5, -0.1, 'RSI_MACRO_BEARISH'),
      },
    }));

    // The merged indicator must reflect the MICRO reading (conf 1.00):
    // normalized 0.9 → bullish confluence, and its signal label must be
    // the one that surfaces in signals_by_kind.
    const confluence = p.group_confluence.find((g: { group: string }) => g.group === 'Momentum');
    expect(confluence).toEqual(expect.objectContaining({ bullish: 1, bearish: 0, neutral: 0 }));

    const thSignals = p.signals_by_kind.Threshold ?? [];
    const labels = thSignals.map((s: { label: string }) => s.label);
    expect(labels).toContain('RSI_MICRO_BULLISH');
    expect(labels).not.toContain('RSI_MACRO_BEARISH');
  });
});
// Regression (D7): unlike the single-TF Metrics builder (which omits
// missing-DTO indicators from its rows), the MTF top-level indicators
// list zero-fills registry entries with no store DTO (active: false).
it('zero-fills missing-DTO registry entries in the MTF indicator list', () => {
  const wideRegistry = [
    ...registry,
    {
      key: 'smc_liquidity',
      display_name: 'SMC Liquidity',
      group: 'Institutional',
      class: 'Leading',
      value_format: 'decimals2',
      value_source: 'raw',
      default_enabled: true,
      directional: true,
    } as unknown as IndicatorMeta,
  ];
  const p = JSON.parse(buildMtfExportJson({
    symbol: 'BTC-USDT',
    markPrice: 63390,
    headerSpec,
    registry: wideRegistry,
    pair: {
      microTerm: makeTf('Micro', '63390'),
      fastTerm: makeTf('Fast', '63395'),
      slowTerm: makeTf('Slow', '63300'),
      macroTerm: makeTf('Macro', '63000'),
    },
  }));
  const rows = p.indicators as Array<{ key: string; values: Array<{ active: boolean; normalized: number }> }>;
  expect(rows).toHaveLength(2);
  const smc = rows.find((i) => i.key === 'smc_liquidity');
  expect(smc).toBeDefined();
  expect(smc!.values.every((v) => v.active === false && v.normalized === 0)).toBe(true);
});
