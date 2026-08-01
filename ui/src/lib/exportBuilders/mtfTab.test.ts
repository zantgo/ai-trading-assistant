// Tests for the MTF builder.

import { describe, it, expect } from 'vitest';
import { buildMtfExportJson, type MtfPayload } from './mtfTab';
import type { TimeframeTelemetry, IndicatorMeta, IndicatorDto } from '../../types';

function makeTf(overrides: Partial<TimeframeTelemetry> = {}): TimeframeTelemetry {
  return {
    slot: 'micro',
    symbol: 'BTC-USDT',
    exchange: 'Hyperliquid',
    barDurationSec: 60,
    indicators: {},
    priceText: '65000.00',
    volText: '100',
    avgVolText: '90',
    showPatterns: true,
    isCompleted: true,
    latestSnapshot: null,
    historyPrices: [],
    pipelineState: 'LIVE',
    indicatorLifecycle: {},
    ...overrides,
  } as TimeframeTelemetry;
}

function makeRegistry(): IndicatorMeta[] {
  return [
    {
      key: 'rsi',
      display_name: 'RSI',
      group: 'Momentum',
      class: 'Leading',
      directional: true,
      default_enabled: true,
      value_format: 'decimals2',
      value_source: 'rsi',
      updates_on_shadow: false,
    },
    {
      key: 'macd',
      display_name: 'MACD',
      group: 'Momentum',
      class: 'Leading',
      directional: true,
      default_enabled: true,
      value_format: 'decimals4',
      value_source: 'macd',
      updates_on_shadow: false,
    },
  ] as IndicatorMeta[];
}

describe('buildMtfExportJson', () => {
  it('produces a valid payload with all expected top-level fields', () => {
    const json = buildMtfExportJson({
      pair: {
        microTerm: makeTf({ barDurationSec: 60 }),
        fastTerm: makeTf({ barDurationSec: 180 }),
        slowTerm: makeTf({ barDurationSec: 300 }),
        macroTerm: makeTf({ barDurationSec: 900 }),
      },
      registry: makeRegistry(),
      symbol: 'BTC-USDT',
    });
    const p = JSON.parse(json) as MtfPayload;
    expect(p.source_tab).toBe('mtf');
    expect(p.meta.symbol).toBe('BTC-USDT');
    expect(p.groups).toBeDefined();
    expect(p.indicators).toBeDefined();
    expect(p.timeframes).toBeDefined();
    expect(p.signals_total).toBeDefined();
  });

  it('timeframes contains 4 entries with correct durations', () => {
    const p = JSON.parse(buildMtfExportJson({
      pair: {
        microTerm: makeTf({ barDurationSec: 60 }),
        fastTerm: makeTf({ barDurationSec: 180 }),
        slowTerm: makeTf({ barDurationSec: 300 }),
        macroTerm: makeTf({ barDurationSec: 900 }),
      },
      registry: makeRegistry(),
      symbol: 'BTC-USDT',
    })) as MtfPayload;
    expect(p.timeframes.length).toBe(4);
    expect(p.timeframes[0].label).toBe('Micro');
    expect(p.timeframes[0].duration_seconds).toBe(60);
    expect(p.timeframes[3].label).toBe('Macro');
    expect(p.timeframes[3].duration_seconds).toBe(900);
  });

  it('indicators captures per-TF values and agreement', () => {
    const p = JSON.parse(buildMtfExportJson({
      pair: {
        microTerm: makeTf({ indicators: { rsi: { normalized: 0.5 } as IndicatorDto } }),
        fastTerm: makeTf({ indicators: { rsi: { normalized: 0.3 } as IndicatorDto } }),
        slowTerm: makeTf({ indicators: { rsi: { normalized: -0.1 } as IndicatorDto } }),
        macroTerm: makeTf({ indicators: { rsi: { normalized: -0.5 } as IndicatorDto } }),
      },
      registry: [makeRegistry()[0]],
      symbol: 'BTC-USDT',
    })) as MtfPayload;
    const rsi = p.indicators[0];
    expect(rsi.key).toBe('rsi');
    expect(rsi.values.length).toBe(4);
    expect(rsi.values[0].normalized).toBe(0.5);
    expect(rsi.agreement).toBeCloseTo(0.05);
    expect(rsi.agreement_label).toBe('MIXED');
  });

  it('classifies agreement_label correctly', () => {
    const p = JSON.parse(buildMtfExportJson({
      pair: {
        microTerm: makeTf({ indicators: { rsi: { normalized: 0.5 } as IndicatorDto } }),
        fastTerm: makeTf({ indicators: { rsi: { normalized: 0.4 } as IndicatorDto } }),
        slowTerm: makeTf({ indicators: { rsi: { normalized: 0.3 } as IndicatorDto } }),
        macroTerm: makeTf({ indicators: { rsi: { normalized: 0.2 } as IndicatorDto } }),
      },
      registry: [makeRegistry()[0]],
      symbol: 'BTC-USDT',
    })) as MtfPayload;
    expect(p.indicators[0].agreement_label).toBe('BULL');

    const p2 = JSON.parse(buildMtfExportJson({
      pair: {
        microTerm: makeTf({ indicators: { rsi: { normalized: -0.5 } as IndicatorDto } }),
        fastTerm: makeTf({ indicators: { rsi: { normalized: -0.4 } as IndicatorDto } }),
        slowTerm: makeTf({ indicators: { rsi: { normalized: -0.3 } as IndicatorDto } }),
        macroTerm: makeTf({ indicators: { rsi: { normalized: -0.2 } as IndicatorDto } }),
      },
      registry: [makeRegistry()[0]],
      symbol: 'BTC-USDT',
    })) as MtfPayload;
    expect(p2.indicators[0].agreement_label).toBe('BEAR');
  });

  it('groups rollup contains only groups with indicators', () => {
    const p = JSON.parse(buildMtfExportJson({
      pair: {
        microTerm: makeTf(),
        fastTerm: makeTf(),
        slowTerm: makeTf(),
        macroTerm: makeTf(),
      },
      registry: makeRegistry(),
      symbol: 'BTC-USDT',
    })) as MtfPayload;
    expect(p.groups.length).toBe(1);
    expect(p.groups[0].key).toBe('Momentum');
    expect(p.groups[0].indicator_count).toBe(2);
  });

  it('signals_total counts unique signal labels across all 4 TFs', () => {
    const p = JSON.parse(buildMtfExportJson({
      pair: {
        microTerm: makeTf({
          indicators: { rsi: {
            normalized: 0,
            signals: [{ kind: 'Crossover', direction: 'Bullish', status: 'Active', label: 'A', strength: 1 }],
          } as IndicatorDto },
        }),
        fastTerm: makeTf({
          indicators: { rsi: {
            normalized: 0,
            signals: [{ kind: 'Crossover', direction: 'Bullish', status: 'Active', label: 'A', strength: 1 }],
          } as IndicatorDto },
        }),
        slowTerm: makeTf({
          indicators: { rsi: {
            normalized: 0,
            signals: [{ kind: 'Crossover', direction: 'Bullish', status: 'Active', label: 'B', strength: 1 }],
          } as IndicatorDto },
        }),
        macroTerm: makeTf(),
      },
      registry: [makeRegistry()[0]],
      symbol: 'BTC-USDT',
    })) as MtfPayload;
    expect(p.signals_total).toBe(2);
  });

  it('timeframes[].mark_price is null when priceText is `--`', () => {
    const p = JSON.parse(buildMtfExportJson({
      pair: {
        microTerm: makeTf({ priceText: '--' }),
        fastTerm: makeTf(),
        slowTerm: makeTf(),
        macroTerm: makeTf(),
      },
      registry: makeRegistry(),
      symbol: 'BTC-USDT',
    })) as MtfPayload;
    expect(p.timeframes[0].mark_price).toBeNull();
  });
});
