// Tests for the Metrics tab (single-TF) builder.

import { describe, it, expect } from 'vitest';
import { buildMetricsTabExport, type MetricsPayload } from './metricsTab';
import type {
  TimeframeTelemetry,
  IndicatorMeta,
  IndicatorDto,
  VolumeProfileSnapshot,
  LiquidationClusterMatrix,
  LiquidityFlow,
  LiquiditySignal,
  MarketContext,
} from '../../types';

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

function makeMarketContext(overrides: Partial<MarketContext> = {}): MarketContext {
  return {
    regime: 'TRENDING_BULL',
    overall_score: 0.5,
    overall_label: 'BULLISH',
    trend: { score: 0.6, confidence: 0.8, label: 'BULLISH' },
    momentum: { score: 0.5, confidence: 0.7, label: 'BULLISH' },
    volatility: { score: 0.2, confidence: 0.6, label: 'NEUTRAL' },
    volume: { score: 0.4, confidence: 0.7, label: 'BULLISH' },
    liquidity: { score: 0.3, confidence: 0.6, label: 'NEUTRAL' },
    ...overrides,
  };
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
    {
      key: 'fibonacci',
      display_name: 'Fibonacci',
      group: 'Structure',
      class: 'Leading',
      directional: false,
      default_enabled: true,
      value_format: 'decimals2',
      value_source: 'fibonacci',
      updates_on_shadow: false,
    },
  ] as IndicatorMeta[];
}

function makeRsiDto(): IndicatorDto {
  return {
    raw_value: 65,
    normalized: 0.3,
    state_label: 'BULLISH',
    confidence: 0.75,
    signals: [
      { kind: 'Crossover', direction: 'Bullish', status: 'Active', label: 'RSI cross up', strength: 0.8, age_bars: 2 },
    ],
    values: null,
  };
}

function makeFibDto(): IndicatorDto {
  return {
    raw_value: 0,
    normalized: 0.2,
    state_label: 'NEUTRAL',
    confidence: 0.6,
    signals: [],
    values: {
      gp_top: 66000,
      gp_bottom: 64000,
      ext_1618: 68000,
      ext_2618: 70000,
      fib_0618: 64500,
    },
  };
}

function makeVolumeProfile(): VolumeProfileSnapshot {
  return {
    symbol: 'BTC-USDT',
    timeframe_slot: 'micro',
    timeframe_secs: 60,
    poc_price: 65000,
    value_area_high: 66000,
    value_area_low: 64000,
    total_volume: 1000000,
    range_low: 63000,
    range_high: 67000,
    num_bins: 30,
    timestamp_ms: 1753950000,
    bins: [
      { price_low: 64900, price_high: 65000, volume: 50000, buy_volume: 30000, sell_volume: 20000 },
      { price_low: 65000, price_high: 65100, volume: 40000, buy_volume: 25000, sell_volume: 15000 },
    ],
  };
}

function makeLiquidityFlow(): LiquidityFlow {
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
    short_clusters: [
      { peak_price: 55000, distance_from_mid_pct: 0.1, notional_usd: 1e6, magnet_strength: 80, cluster_kind: 'short' },
    ],
    long_clusters: [
      { peak_price: 45000, distance_from_mid_pct: 0.1, notional_usd: 1e6, magnet_strength: 70, cluster_kind: 'long' },
    ],
  };
}

function makeLiquiditySignals(): LiquiditySignal[] {
  return [
    { kind: 'CASCADE', direction: 'Bullish', strength: 0.8, confidence: 0.7, evidence: ['Liq surge'] },
  ];
}

describe('buildMetricsTabExport', () => {
  it('produces a valid payload with all expected top-level fields', () => {
    const json = buildMetricsTabExport({
      tf: makeTf({
        context: makeMarketContext(),
        indicators: { rsi: makeRsiDto(), fibonacci: makeFibDto() },
      }),
      registry: makeRegistry(),
      volumeProfile: makeVolumeProfile(),
      liquidity: makeLiquidityFlow(),
      cluster: makeCluster(),
      liquiditySignals: makeLiquiditySignals(),
      symbol: 'BTC-USDT',
      tfLabel: 'Micro',
      tfSecs: 60,
      timestamp: 1753950000,
      markPrice: 65000,
      filterState: {
        active_only: false,
        confirmed_plus_only: false,
        hide_gates: false,
        hide_overlays: false,
      },
    });
    const p = JSON.parse(json) as MetricsPayload;
    expect(p.source_tab).toBe('metrics');
    expect(p.meta.symbol).toBe('BTC-USDT');
    expect(p.market_context).toBeDefined();
    expect(p.group_confluence).toBeDefined();
    expect(p.structural_anchors).toBeDefined();
    expect(p.indicators).toBeDefined();
    expect(p.signals_total).toBeDefined();
    expect(p.signals_by_kind).toBeDefined();
    expect(p.divergences).toBeDefined();
    expect(p.levels).toBeDefined();
    expect(p.liquidity_signals).toBeDefined();
    expect(p.liquidity_flow).toBeDefined();
    expect(p.cluster_matrix).toBeDefined();
  });

  it('market_context captures 5 dimensions + regime + overall', () => {
    const p = JSON.parse(buildMetricsTabExport({
      tf: makeTf({ context: makeMarketContext() }),
      registry: makeRegistry(),
      volumeProfile: null,
      liquidity: null,
      cluster: null,
      liquiditySignals: [],
      symbol: 'BTC-USDT',
      tfLabel: 'Micro',
      tfSecs: 60,
    })) as MetricsPayload;
    expect(p.market_context?.regime).toBe('TRENDING_BULL');
    expect(p.market_context?.overall_label).toBe('BULLISH');
    expect(p.market_context?.trend.score).toBe(0.6);
  });

  it('group_confluence counts bullish/bearish/neutral per group', () => {
    const tf = makeTf({
      indicators: {
        rsi: { ...makeRsiDto(), normalized: 0.5 },
        macd: { ...makeRsiDto(), normalized: -0.3 },
      },
    });
    const p = JSON.parse(buildMetricsTabExport({
      tf,
      registry: makeRegistry(),
      volumeProfile: null,
      liquidity: null,
      cluster: null,
      liquiditySignals: [],
      symbol: 'BTC-USDT',
      tfLabel: 'Micro',
      tfSecs: 60,
    })) as MetricsPayload;
    const momentumGroup = p.group_confluence.find(g => g.group === 'Momentum');
    expect(momentumGroup).toBeDefined();
    expect(momentumGroup?.bullish).toBe(1);
    expect(momentumGroup?.bearish).toBe(1);
  });

  it('structural_anchors.fibonacci captures summary values', () => {
    const p = JSON.parse(buildMetricsTabExport({
      tf: makeTf({ indicators: { fibonacci: makeFibDto() } }),
      registry: makeRegistry(),
      volumeProfile: null,
      liquidity: null,
      cluster: null,
      liquiditySignals: [],
      symbol: 'BTC-USDT',
      tfLabel: 'Micro',
      tfSecs: 60,
    })) as MetricsPayload;
    expect(p.structural_anchors.fibonacci.present).toBe(true);
    expect(p.structural_anchors.fibonacci.gp_top).toBe(66000);
    expect(p.structural_anchors.fibonacci.retracement_coefficients?.fib_0618).toBe(64500);
  });

  it('structural_anchors.volume_profile captures POC/VAH/VAL + top HVN', () => {
    const p = JSON.parse(buildMetricsTabExport({
      tf: makeTf(),
      registry: makeRegistry(),
      volumeProfile: makeVolumeProfile(),
      liquidity: null,
      cluster: null,
      liquiditySignals: [],
      symbol: 'BTC-USDT',
      tfLabel: 'Micro',
      tfSecs: 60,
    })) as MetricsPayload;
    expect(p.structural_anchors.volume_profile?.poc_price).toBe(65000);
    expect(p.structural_anchors.volume_profile?.value_area_high).toBe(66000);
    expect(p.structural_anchors.volume_profile?.value_area_low).toBe(64000);
  });

  it('structural_anchors.cascade_alert is null when state is None', () => {
    const p = JSON.parse(buildMetricsTabExport({
      tf: makeTf(),
      registry: makeRegistry(),
      volumeProfile: null,
      liquidity: {
        ...makeLiquidityFlow(),
        cascade_state: 'None',
      },
      cluster: null,
      liquiditySignals: [],
      symbol: 'BTC-USDT',
      tfLabel: 'Micro',
      tfSecs: 60,
    })) as MetricsPayload;
    expect(p.structural_anchors.cascade_alert).toBeNull();
  });

  it('structural_anchors.cascade_alert fires when state is DETECTED', () => {
    const p = JSON.parse(buildMetricsTabExport({
      tf: makeTf(),
      registry: makeRegistry(),
      volumeProfile: null,
      liquidity: makeLiquidityFlow(),
      cluster: null,
      liquiditySignals: [],
      symbol: 'BTC-USDT',
      tfLabel: 'Micro',
      tfSecs: 60,
    })) as MetricsPayload;
    expect(p.structural_anchors.cascade_alert).not.toBeNull();
    expect(p.structural_anchors.cascade_alert?.state).toBe('DETECTED');
  });

  it('indicators captures raw/normalized/state/signals/sub_values/lifecycle', () => {
    const p = JSON.parse(buildMetricsTabExport({
      tf: makeTf({ indicators: { rsi: makeRsiDto() } }),
      registry: makeRegistry(),
      volumeProfile: null,
      liquidity: null,
      cluster: null,
      liquiditySignals: [],
      symbol: 'BTC-USDT',
      tfLabel: 'Micro',
      tfSecs: 60,
    })) as MetricsPayload;
    const rsi = p.indicators.find(i => i.key === 'rsi');
    expect(rsi).toBeDefined();
    expect(rsi?.raw).toBe(65);
    expect(rsi?.normalized).toBe(0.3);
    expect(rsi?.state).toBe('BULLISH');
    expect(rsi?.signals.length).toBe(1);
    expect(rsi?.signals[0].kind).toBe('CRO');
  });

  it('append the Fibonacci summary row when fib data is present', () => {
    const p = JSON.parse(buildMetricsTabExport({
      tf: makeTf({ indicators: { fibonacci: makeFibDto() } }),
      registry: makeRegistry(),
      volumeProfile: null,
      liquidity: null,
      cluster: null,
      liquiditySignals: [],
      symbol: 'BTC-USDT',
      tfLabel: 'Micro',
      tfSecs: 60,
    })) as MetricsPayload;
    const fib = p.indicators.find(i => i.key === '__fibonacci_summary__');
    expect(fib).toBeDefined();
    expect(fib?.display_name).toBe('Fibonacci Levels (computed values)');
  });

  it('signals_by_kind groups signals by SignalKind', () => {
    const p = JSON.parse(buildMetricsTabExport({
      tf: makeTf({ indicators: { rsi: makeRsiDto() } }),
      registry: makeRegistry(),
      volumeProfile: null,
      liquidity: null,
      cluster: null,
      liquiditySignals: [],
      symbol: 'BTC-USDT',
      tfLabel: 'Micro',
      tfSecs: 60,
    })) as MetricsPayload;
    expect(p.signals_by_kind['Crossover'].length).toBe(1);
  });

  it('liquidity_flow captures long/short liquidations + cascade state', () => {
    const p = JSON.parse(buildMetricsTabExport({
      tf: makeTf(),
      registry: makeRegistry(),
      volumeProfile: null,
      liquidity: makeLiquidityFlow(),
      cluster: null,
      liquiditySignals: [],
      symbol: 'BTC-USDT',
      tfLabel: 'Micro',
      tfSecs: 60,
    })) as MetricsPayload;
    expect(p.liquidity_flow?.long_liquidations_usd).toBe(50000);
    expect(p.liquidity_flow?.cascade_state).toBe('DETECTED');
  });

  it('cluster_matrix captures top_above/top_below', () => {
    const p = JSON.parse(buildMetricsTabExport({
      tf: makeTf(),
      registry: makeRegistry(),
      volumeProfile: null,
      liquidity: null,
      cluster: makeCluster(),
      liquiditySignals: [],
      symbol: 'BTC-USDT',
      tfLabel: 'Micro',
      tfSecs: 60,
    })) as MetricsPayload;
    expect(p.cluster_matrix?.top_above.length).toBe(1);
    expect(p.cluster_matrix?.top_below.length).toBe(1);
  });

  it('handles null tf gracefully', () => {
    const p = JSON.parse(buildMetricsTabExport({
      tf: null,
      registry: makeRegistry(),
      volumeProfile: null,
      liquidity: null,
      cluster: null,
      liquiditySignals: [],
      symbol: 'BTC-USDT',
      tfLabel: 'Micro',
      tfSecs: 60,
    })) as MetricsPayload;
    expect(p.indicators).toEqual([]);
    expect(p.market_context).toBeNull();
  });
});
