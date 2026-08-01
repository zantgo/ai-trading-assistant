// Tests for the Risk tab builder.

import { describe, it, expect } from 'vitest';
import { buildRiskTabExport, type RiskPayload } from './riskTab';
import type { RiskMatrix, RiskDimension, LiquidityFlow, LiquidationClusterMatrix } from '../../types';

function makeDim(overrides: Partial<RiskDimension> = {}): RiskDimension {
  return {
    score: 50,
    level: 'Moderate',
    state: 'Stable',
    confidence: 75,
    evidence: ['test evidence'],
    ...overrides,
  };
}

function makeRiskMatrix(overrides: Partial<RiskMatrix> = {}): RiskMatrix {
  return {
    symbol: 'BTC-USDT',
    market_risk: makeDim({ score: 60, level: 'Moderate' }),
    volatility_risk: makeDim({ score: 80, level: 'High' }),
    execution_liquidity_risk: makeDim({ score: 40, level: 'Moderate' }),
    structure_risk: makeDim({ score: 30, level: 'Low' }),
    momentum_risk: makeDim({ score: 55, level: 'Moderate' }),
    signal_risk: makeDim({ score: 25, level: 'Low' }),
    execution_risk: makeDim({ score: 20, level: 'VeryLow' }),
    cascade_risk: makeDim({ score: 90, level: 'Extreme' }),
    overall_risk: makeDim({ score: 65, level: 'High' }),
    ...overrides,
  };
}

function makeFlow(): LiquidityFlow {
  return {
    long_liquidations_usd: 50000,
    short_liquidations_usd: 10000,
    net_liquidation_usd: 40000,
    event_count: 3,
    largest_event_usd: 30000,
    largest_event_price: 49500,
    largest_event_side: 'LONG',
    cascade_state: 'DETECTED',
    cascade_intensity: 65,
  };
}

function makeCluster(): LiquidationClusterMatrix {
  return {
    mid_price: 50000,
    cascade_asymmetry: 0.3,
    total_long_oi_usd: 1e8,
    total_short_oi_usd: 9e7,
    estimation_confidence: 0.8,
    leverage_assumptions: {
      source: 'default',
      buckets: [1, 3, 5, 10, 20, 50, 100],
      weights: [0.05, 0.1, 0.2, 0.3, 0.2, 0.1, 0.05],
      funding_modulation_active: true,
    },
    short_clusters: [],
    long_clusters: [],
  };
}

describe('buildRiskTabExport', () => {
  it('produces a valid payload with all expected top-level fields', () => {
    const json = buildRiskTabExport({
      risk: makeRiskMatrix(),
      flow: makeFlow(),
      cluster: makeCluster(),
      symbol: 'BTC-USDT',
      tfSecs: 60,
    });
    const p = JSON.parse(json) as RiskPayload;
    expect(p.source_tab).toBe('risk');
    expect(p.meta.symbol).toBe('BTC-USDT');
    expect(p.meta.tf_secs).toBe(60);
    expect(p.hero).toBeDefined();
    expect(p.summary_counts).toBeDefined();
    expect(p.dimensions).toBeDefined();
    expect(p.cascade_telemetry).toBeDefined();
    expect(p.interpretation).toBeDefined();
  });

  it('captures the hero block correctly', () => {
    const risk = makeRiskMatrix();
    const p = JSON.parse(buildRiskTabExport({
      risk, flow: null, cluster: null, symbol: 'BTC-USDT',
    })) as RiskPayload;
    expect(p.hero.overall_score).toBe(65);
    expect(p.hero.overall_level).toBe('High');
    expect(p.hero.top_severity).toBe('Extreme');
    expect(p.hero.ring_pct).toBe(65);
  });

  it('captures summary_counts correctly', () => {
    const risk = makeRiskMatrix();
    const p = JSON.parse(buildRiskTabExport({
      risk, flow: null, cluster: null, symbol: 'BTC-USDT',
    })) as RiskPayload;
    expect(p.summary_counts.very_low).toBe(1);
    expect(p.summary_counts.low).toBe(2);
    expect(p.summary_counts.moderate).toBe(3);
    expect(p.summary_counts.high).toBe(1);
    expect(p.summary_counts.extreme).toBe(1);
  });

  it('exports all 8 dimensions in panel-sorted order (severity desc)', () => {
    const risk = makeRiskMatrix();
    const p = JSON.parse(buildRiskTabExport({
      risk, flow: null, cluster: null, symbol: 'BTC-USDT',
    })) as RiskPayload;
    expect(p.dimensions.length).toBe(8);
    // First dimension should be cascade_risk (highest score: 90)
    expect(p.dimensions[0].name).toBe('Cascade Risk');
    expect(p.dimensions[0].is_cascade_dim).toBe(true);
    expect(p.dimensions[0].score).toBe(90);
  });

  it('emits weight_mark_pct correctly', () => {
    const risk = makeRiskMatrix();
    const p = JSON.parse(buildRiskTabExport({
      risk, flow: null, cluster: null, symbol: 'BTC-USDT',
    })) as RiskPayload;
    const market = p.dimensions.find(d => d.key === 'market_risk');
    expect(market?.weight_mark_pct).toBe(14);
    expect(market?.weight_pct).toBe(14);
    const structure = p.dimensions.find(d => d.key === 'structure_risk');
    expect(structure?.weight_mark_pct).toBe(10);
  });

  it('captures cascade_telemetry when flow and cluster are present', () => {
    const p = JSON.parse(buildRiskTabExport({
      risk: makeRiskMatrix(), flow: makeFlow(), cluster: makeCluster(), symbol: 'BTC-USDT',
    })) as RiskPayload;
    expect(p.cascade_telemetry).not.toBeNull();
    expect(p.cascade_telemetry?.cascade_state).toBe('DETECTED');
    expect(p.cascade_telemetry?.cascade_intensity).toBe(65);
    expect(p.cascade_telemetry?.cascade_asymmetry).toBe(0.3);
  });

  it('emits null cascade_telemetry when both flow and cluster are null', () => {
    const p = JSON.parse(buildRiskTabExport({
      risk: makeRiskMatrix(), flow: null, cluster: null, symbol: 'BTC-USDT',
    })) as RiskPayload;
    expect(p.cascade_telemetry).toBeNull();
  });

  it('produces a valid payload even when risk is null', () => {
    const p = JSON.parse(buildRiskTabExport({
      risk: null, flow: null, cluster: null, symbol: 'BTC-USDT',
    })) as RiskPayload;
    expect(p.source_tab).toBe('risk');
    expect(p.hero.top_severity).toBeNull();
    expect(p.dimensions).toEqual([]);
    expect(p.summary_counts).toEqual({
      very_low: 0, low: 0, moderate: 0, high: 0, extreme: 0,
    });
  });

  it('interpretation reflects severity counts', () => {
    const risk = makeRiskMatrix();
    const p = JSON.parse(buildRiskTabExport({
      risk, flow: null, cluster: null, symbol: 'BTC-USDT',
    })) as RiskPayload;
    expect(p.interpretation).toContain('1 extreme');
    expect(p.interpretation).toContain('1 high');
    expect(p.interpretation).toContain('3 moderate');
  });
});
