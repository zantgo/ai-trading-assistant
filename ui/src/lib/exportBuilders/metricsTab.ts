// Metrics tab builder (single-TF) — scoped export payload mirroring the panel.
//
// The Metrics tab renders (single-TF mode):
//   1. MarketContextStrip (5 dimensions + regime + overall_score/label)
//   2. GroupConfluenceGrid (8 functional groups with directional bias summary)
//   3. StructuralAnchorsStrip (Fibonacci / Volume Profile / Liquidity ladder)
//   4. FacetTabs (Indicators / Signals / Divergences / Levels)
//      - Indicators facet: full registry with raw/norm/state/signals/sub_values/lifecycle
//      - Signals facet: signals grouped by SignalKind
//      - Divergences facet: divergence sub-kind + pivot coords
//      - Levels facet: LevelTest signals + Fibonacci ladder + Volume Profile + Liquidation Magnets
//
// This is the largest payload. Each block is reproduced 1:1.

import type {
  TimeframeTelemetry,
  IndicatorMeta,
  IndicatorDto,
  IndicatorSignal,
  VolumeProfileSnapshot,
  LiquidationClusterMatrix,
  LiquidationCluster,
  LiquidityFlow,
  LiquiditySignal,
  MarketContext,
} from '../../types';
import { buildMeta } from './shared';
import type { MetaEnvelope } from './shared';

// ── Payload types ────────────────────────────────────────────────────────

export interface MetricsMarketContextBlock {
  regime: string;
  overall_score: number;
  overall_label: string;
  trend: { score: number; confidence: number; label: string };
  momentum: { score: number; confidence: number; label: string };
  volatility: { score: number; confidence: number; label: string };
  volume: { score: number; confidence: number; label: string };
  liquidity: { score: number; confidence: number; label: string };
  signal_count: number;
  age_bars: number | null;
}

export interface GroupConfluenceRow {
  group: string;
  total: number;
  gates: number;
  bullish: number;
  bearish: number;
  neutral: number;
  active: number;
  active_signals: number;
  dominant: 'bull' | 'bear' | 'neutral';
  dots: Array<'bull' | 'bear' | 'neutral'>;
}

export interface MetricsStructuralAnchorsBlock {
  fibonacci: {
    present: boolean;
    gp_top: number | null;
    gp_bottom: number | null;
    ext_1618: number | null;
    ext_2618: number | null;
    retracement_coefficients: Record<string, number | null> | null;
  };
  volume_profile: VolumeProfileExport | null;
  liquidation_clusters: {
    top_short: Array<{ peak_price: number; distance_from_mid_pct: number; notional_usd: number; magnet_strength: number; cluster_kind: string }>;
    top_long: Array<{ peak_price: number; distance_from_mid_pct: number; notional_usd: number; magnet_strength: number; cluster_kind: string }>;
  } | null;
  cascade_alert: {
    state: string;
    intensity: number;
  } | null;
}

export interface IndicatorSignalExport {
  kind: string;
  direction: string;
  status: string;
  label: string;
  strength: number;
  age_bars: number | undefined;
}

export interface MetricsIndicatorRow {
  key: string;
  display_name: string;
  group: string;
  class: string;
  raw: number | null;
  normalized: number;
  state: string;
  pending_candle: boolean;
  confidence_pct: number;
  signals: IndicatorSignalExport[];
  sub_values: Record<string, number> | null;
  indicator_lifecycle: {
    state: string;
    bars_seen: number;
    bars_required: number;
  } | null;
}

export interface DivergenceRow {
  indicator_key: string;
  display_name: string;
  sub_kind: string;
  direction: string;
  status: string;
  strength: number;
  confidence_pct: number;
  age_bars: number | undefined;
  label: string;
  pivots: Array<{ time: number; value: number }> | null;
}

export interface LevelRow {
  indicator_key: string;
  display_name: string;
  level_name: string;
  kind: string;
  role: 'support' | 'resistance' | 'neutral';
  price_text: string;
  direction: string;
  status: string;
  strength: number;
  confidence_pct: number;
  age_bars: number | undefined;
}

export interface VolumeProfileExport {
  symbol: string;
  timeframe_slot: string;
  timeframe_secs: number;
  poc_price: number;
  value_area_high: number;
  value_area_low: number;
  total_volume: number;
  range_low: number;
  range_high: number;
  num_bins: number;
  timestamp_ms: number;
  top_hvn: Array<{ price_low: number; price_high: number; volume: number; buy_volume: number; sell_volume: number; strength_x_mean: number }>;
  buy_total: number;
  sell_total: number;
  buy_sell_bias: number;
  current_position: { in_va: boolean; range_pos_pct: number };
}

export interface LiquidityFlowExport {
  long_liquidations_usd: number;
  short_liquidations_usd: number;
  net_liquidation_usd: number;
  event_count: number;
  largest_event_usd: number;
  largest_event_price: number | null;
  largest_event_side: string | null;
  cascade_state: string;
  cascade_intensity: number;
}

export interface ClusterMatrixExport {
  mid_price: number;
  cascade_asymmetry: number;
  total_long_oi_usd: number;
  total_short_oi_usd: number;
  estimation_confidence: number;
  leverage_assumptions: { source: string; buckets: number[]; weights: number[]; funding_modulation_active: boolean };
  top_above: Array<{ peak_price: number; distance_from_mid_pct: number; notional_usd: number; magnet_strength: number; cluster_kind: string }>;
  top_below: Array<{ peak_price: number; distance_from_mid_pct: number; notional_usd: number; magnet_strength: number; cluster_kind: string }>;
}

export interface LiquiditySignalExport {
  kind: string;
  direction: string;
  strength: number;
  confidence: number;
  evidence: string[];
}

export interface MetricsPayload {
  source_tab: 'metrics';
  meta: MetaEnvelope;
  market_context: MetricsMarketContextBlock | null;
  group_confluence: GroupConfluenceRow[];
  structural_anchors: MetricsStructuralAnchorsBlock;
  indicators: MetricsIndicatorRow[];
  signals_total: number;
  signals_by_kind: Record<string, IndicatorSignalExport[]>;
  divergences: DivergenceRow[];
  levels: LevelRow[];
  liquidity_signals: LiquiditySignalExport[];
  liquidity_flow: LiquidityFlowExport | null;
  cluster_matrix: ClusterMatrixExport | null;
}

// ── Constants ────────────────────────────────────────────────────────────

export const GROUP_ORDER = [
  'Trend', 'Momentum', 'Volume', 'Volatility',
  'Structure', 'Regime', 'Institutional', 'DerivativesData',
] as const;

const SIGNAL_ABBR: Record<string, string> = {
  Divergence: 'DIV', Crossover: 'CRO', Threshold: 'TH', Breakout: 'BO',
  BandTouch: 'BT', ZeroLineCross: '0X', CompressionRelease: 'SQZ',
  LevelTest: 'LV', TrendFlip: 'FLIP', VolumeClimax: 'VOL',
  StackChange: 'STK', PatternForming: 'PAT',
};

const SIGNAL_KIND_ORDER = [
  'Divergence', 'Crossover', 'Threshold', 'Breakout', 'BandTouch',
  'ZeroLineCross', 'CompressionRelease', 'LevelTest', 'TrendFlip',
  'VolumeClimax', 'StackChange', 'PatternForming',
] as const;

const DIVERGENCE_KEYS = new Set([
  'rsi', 'macd', 'stochastic', 'chandemo',
  'obv', 'cmf', 'mfi', 'squeeze', 'oi_price_divergence',
]);

const BULL_THRESHOLD = 0.1;
const BEAR_THRESHOLD = -0.1;

// ── Helpers ──────────────────────────────────────────────────────────────

function iRaw(indicators: Record<string, IndicatorDto>, key: string): number | null {
  return indicators?.[key]?.raw_value ?? null;
}

function iSub(indicators: Record<string, IndicatorDto>, key: string, sub: string): number | null {
  const subValues = indicators?.[key]?.values ?? null;
  const raw = subValues?.[sub];
  if (raw == null || Number.isNaN(raw)) return null;
  return raw;
}

function rawVal(meta: IndicatorMeta, indicators: Record<string, IndicatorDto>): number | null {
  if (meta.value_source.startsWith('sub:')) {
    return iSub(indicators, meta.key, meta.value_source.slice(4));
  }
  return iRaw(indicators, meta.key);
}

function formatRawForExport(
  meta: IndicatorMeta,
  indicators: Record<string, IndicatorDto>,
): number | null {
  if (meta.value_format === 'onoff') {
    return rawVal(meta, indicators) != null ? 1 : 0;
  }
  const v = rawVal(meta, indicators);
  if (v == null) return null;
  switch (meta.value_format) {
    case 'percent1':  return Number(v.toFixed(1));
    case 'price':     return Number(v.toFixed(2));
    case 'ratio2':    return Number(v.toFixed(2));
    case 'decimals1': return Number(v.toFixed(1));
    case 'decimals4': return Number(v.toFixed(4));
    case 'decimals2':
    default:          return Number(v.toFixed(2));
  }
}

function confidencePct(indicators: Record<string, IndicatorDto>, key: string): number {
  const dto = indicators?.[key];
  if (!dto?.confidence) return 0;
  return Math.round(Math.abs(dto.confidence) * 100);
}

function buildMarketContext(
  tf: TimeframeTelemetry,
  signalCount: number,
): MetricsMarketContextBlock | null {
  const ctx = tf.context as MarketContext | null | undefined;
  if (!ctx) return null;
  return {
    regime: ctx.regime,
    overall_score: ctx.overall_score,
    overall_label: ctx.overall_label,
    trend:        { score: ctx.trend.score,        confidence: ctx.trend.confidence,        label: ctx.trend.label },
    momentum:     { score: ctx.momentum.score,     confidence: ctx.momentum.confidence,     label: ctx.momentum.label },
    volatility:   { score: ctx.volatility.score,   confidence: ctx.volatility.confidence,   label: ctx.volatility.label },
    volume:       { score: ctx.volume.score,       confidence: ctx.volume.confidence,       label: ctx.volume.label },
    liquidity:    { score: ctx.liquidity.score,    confidence: ctx.liquidity.confidence,    label: ctx.liquidity.label },
    signal_count: signalCount,
    age_bars: null,
  };
}

function deriveDominant(s: GroupConfluenceRow): 'bull' | 'bear' | 'neutral' {
  if (s.bullish > s.bearish && s.bullish > s.neutral) return 'bull';
  if (s.bearish > s.bullish && s.bearish > s.neutral) return 'bear';
  return 'neutral';
}

function buildDots(s: GroupConfluenceRow): Array<'bull' | 'bear' | 'neutral'> {
  const total = Math.max(s.bullish + s.bearish + s.neutral, 1);
  const out: Array<'bull' | 'bear' | 'neutral'> = [];
  const slots = Math.min(total, 5);
  const bullSlots = Math.round((s.bullish / total) * slots);
  const bearSlots = Math.round((s.bearish / total) * slots);
  for (let i = 0; i < bullSlots; i++) out.push('bull');
  for (let i = 0; i < bearSlots; i++) out.push('bear');
  while (out.length < slots) out.push('neutral');
  return out;
}

function buildGroupConfluence(
  registry: IndicatorMeta[],
  indicators: Record<string, IndicatorDto>,
): GroupConfluenceRow[] {
  const map = new Map<string, GroupConfluenceRow>();
  for (const g of GROUP_ORDER) {
    map.set(g, {
      group: g,
      total: 0,
      gates: 0,
      bullish: 0,
      bearish: 0,
      neutral: 0,
      active: 0,
      active_signals: 0,
      dominant: 'neutral',
      dots: [],
    });
  }
  for (const m of registry) {
    if (!m.default_enabled) continue;
    const bucket = map.get(m.group);
    if (!bucket) continue;
    const dto = indicators[m.key];
    bucket.total += 1;
    if (!m.directional) {
      bucket.gates += 1;
      continue;
    }
    const n = dto?.normalized ?? 0;
    if (n > BULL_THRESHOLD) bucket.bullish += 1;
    else if (n < BEAR_THRESHOLD) bucket.bearish += 1;
    else bucket.neutral += 1;
    const sigs = dto?.signals ?? [];
    if (sigs.length > 0) {
      bucket.active += 1;
      bucket.active_signals += sigs.length;
    }
  }
  const out: GroupConfluenceRow[] = [];
  for (const g of GROUP_ORDER) {
    const s = map.get(g);
    if (!s || s.total === 0) continue;
    s.dominant = deriveDominant(s);
    s.dots = buildDots(s);
    out.push(s);
  }
  return out;
}

function buildFibonacciSummary(indicators: Record<string, IndicatorDto>): MetricsStructuralAnchorsBlock['fibonacci'] {
  const fibVals = (indicators['fibonacci']?.values ?? {}) as Record<string, number | undefined>;
  if (Object.keys(fibVals).length === 0) {
    return { present: false, gp_top: null, gp_bottom: null, ext_1618: null, ext_2618: null, retracement_coefficients: null };
  }
  return {
    present: true,
    gp_top: fibVals['gp_top'] ?? null,
    gp_bottom: fibVals['gp_bottom'] ?? null,
    ext_1618: fibVals['ext_1618'] ?? null,
    ext_2618: fibVals['ext_2618'] ?? null,
    retracement_coefficients: {
      fib_0236: fibVals['fib_0236'] ?? null,
      fib_0382: fibVals['fib_0382'] ?? null,
      fib_0500: fibVals['fib_0500'] ?? null,
      fib_0618: fibVals['fib_0618'] ?? null,
      fib_0660: fibVals['fib_0660'] ?? null,
      fib_0786: fibVals['fib_0786'] ?? null,
    },
  };
}

function buildVolumeProfileExport(vp: VolumeProfileSnapshot | null): VolumeProfileExport | null {
  if (!vp) return null;
  const meanVol = vp.bins.length > 0
    ? vp.bins.reduce((a, b) => a + b.volume, 0) / vp.bins.length
    : 0;
  const topHvn = (meanVol > 0 ? vp.bins
    .filter((b) => b.volume >= 1.5 * meanVol)
    .sort((a, b) => b.volume - a.volume)
    .slice(0, 3) : vp.bins
    .slice()
    .sort((a, b) => b.volume - a.volume)
    .slice(0, 3));
  const buy = vp.bins.reduce((a, b) => a + b.buy_volume, 0);
  const sell = vp.bins.reduce((a, b) => a + b.sell_volume, 0);
  const total = buy + sell;
  const range = vp.range_high - vp.range_low;
  const rangePos = vp.poc_price > 0 && range > 0 ? (vp.poc_price - vp.range_low) / range : 0;
  const inVa = true;
  return {
    symbol: vp.symbol,
    timeframe_slot: vp.timeframe_slot,
    timeframe_secs: vp.timeframe_secs,
    poc_price: vp.poc_price,
    value_area_high: vp.value_area_high,
    value_area_low: vp.value_area_low,
    total_volume: vp.total_volume,
    range_low: vp.range_low,
    range_high: vp.range_high,
    num_bins: vp.num_bins,
    timestamp_ms: vp.timestamp_ms,
    top_hvn: topHvn.map((b) => ({
      price_low: b.price_low,
      price_high: b.price_high,
      volume: b.volume,
      buy_volume: b.buy_volume,
      sell_volume: b.sell_volume,
      strength_x_mean: meanVol > 0 ? Number((b.volume / meanVol).toFixed(2)) : 0,
    })),
    buy_total: buy,
    sell_total: sell,
    buy_sell_bias: total > 0 ? Number(((buy - sell) / total).toFixed(4)) : 0,
    current_position: {
      in_va: inVa,
      range_pos_pct: Number((rangePos * 100).toFixed(2)),
    },
  };
}

function buildLiquidationClusters(
  cluster: LiquidationClusterMatrix | null,
): MetricsStructuralAnchorsBlock['liquidation_clusters'] {
  if (!cluster) return null;
  const sortBy = (a: LiquidationCluster, b: LiquidationCluster) =>
    (b.magnet_strength ?? 0) - (a.magnet_strength ?? 0);
  const topShort = [...(cluster.short_clusters ?? [])].sort(sortBy).slice(0, 4);
  const topLong = [...(cluster.long_clusters ?? [])].sort(sortBy).slice(0, 4);
  const mapCluster = (c: LiquidationCluster) => ({
    peak_price: c.peak_price,
    distance_from_mid_pct: c.distance_from_mid_pct,
    notional_usd: c.notional_usd,
    magnet_strength: c.magnet_strength,
    cluster_kind: c.cluster_kind,
  });
  return {
    top_short: topShort.map(mapCluster),
    top_long: topLong.map(mapCluster),
  };
}

function buildCascadeAlert(flow: LiquidityFlow | null): MetricsStructuralAnchorsBlock['cascade_alert'] {
  if (!flow) return null;
  const state = flow.cascade_state;
  if (state !== 'SUSTAINED' && state !== 'DETECTED') return null;
  return {
    state,
    intensity: flow.cascade_intensity,
  };
}

function buildIndicators(
  registry: IndicatorMeta[],
  indicators: Record<string, IndicatorDto>,
  tf: TimeframeTelemetry,
): MetricsIndicatorRow[] {
  const rows: MetricsIndicatorRow[] = [];
  for (const m of registry) {
    const dto = indicators[m.key];
    if (!dto) continue;
    const signals: IndicatorSignalExport[] = (dto.signals ?? []).map((s: IndicatorSignal) => ({
      kind: SIGNAL_ABBR[s.kind] ?? s.kind,
      direction: s.direction,
      status: s.status,
      label: s.label,
      strength: s.strength,
      age_bars: s.age_bars,
    }));
    const subValues: Record<string, number> = {};
    if (dto.values) {
      for (const [k, v] of Object.entries(dto.values)) {
        if (v != null && !Number.isNaN(v)) subValues[k] = v;
      }
    }
    const lc = tf.indicatorLifecycle?.[m.key];
    const pending = !tf.isCompleted
      && lc?.state === 'Live'
      && !(m.updates_on_shadow ?? false);
    rows.push({
      key: m.key,
      display_name: m.display_name,
      group: m.group,
      class: m.class,
      raw: formatRawForExport(m, indicators),
      normalized: dto.normalized ?? 0,
      state: dto.state_label ?? '--',
      pending_candle: pending,
      confidence_pct: confidencePct(indicators, m.key),
      signals,
      sub_values: Object.keys(subValues).length > 0 ? subValues : null,
      indicator_lifecycle: lc ? {
        state: lc.state,
        bars_seen: lc.bars_seen,
        bars_required: lc.bars_required,
      } : null,
    });
  }
  // Append the canonical Fibonacci summary row to mirror the single-TF
  // Metrics export 1:1.
  const fibVals = (indicators['fibonacci']?.values ?? {}) as Record<string, number | undefined>;
  if (Object.keys(fibVals).length > 0) {
    rows.push({
      key: '__fibonacci_summary__',
      display_name: 'Fibonacci Levels (computed values)',
      group: 'Fibonacci',
      class: 'Leading',
      raw: null,
      normalized: indicators['fibonacci']?.normalized ?? 0,
      state: indicators['fibonacci']?.state_label ?? '--',
      pending_candle: false,
      confidence_pct: confidencePct(indicators, 'fibonacci'),
      signals: [],
      sub_values: {
        gp_top: fibVals['gp_top'] ?? null,
        gp_bottom: fibVals['gp_bottom'] ?? null,
        ext_1618: fibVals['ext_1618'] ?? null,
        ext_2618: fibVals['ext_2618'] ?? null,
      } as unknown as Record<string, number>,
      indicator_lifecycle: null,
    });
  }
  return rows;
}

function buildSignalsByKind(
  registry: IndicatorMeta[],
  indicators: Record<string, IndicatorDto>,
): { signalsByKind: Record<string, IndicatorSignalExport[]>; total: number } {
  const out: Record<string, IndicatorSignalExport[]> = {};
  const uniqueLabels = new Set<string>();
  for (const k of SIGNAL_KIND_ORDER) out[k] = [];
  for (const meta of registry) {
    const sigs = indicators?.[meta.key]?.signals ?? [];
    for (const sig of sigs) {
      const exportSig: IndicatorSignalExport = {
        kind: SIGNAL_ABBR[sig.kind] ?? sig.kind,
        direction: sig.direction,
        status: sig.status,
        label: sig.label,
        strength: sig.strength,
        age_bars: sig.age_bars,
      };
      if (!out[sig.kind]) out[sig.kind] = [];
      out[sig.kind].push(exportSig);
      uniqueLabels.add(sig.label);
    }
  }
  // Sort by strength desc per kind
  for (const k of Object.keys(out)) {
    out[k].sort((a, b) => b.strength - a.strength);
  }
  return { signalsByKind: out, total: uniqueLabels.size };
}

function buildDivergences(
  registry: IndicatorMeta[],
  indicators: Record<string, IndicatorDto>,
): DivergenceRow[] {
  const out: DivergenceRow[] = [];
  for (const meta of registry) {
    if (!DIVERGENCE_KEYS.has(meta.key) && !(meta as { supports_divergence?: boolean }).supports_divergence) continue;
    const sigs = indicators?.[meta.key]?.signals ?? [];
    for (const sig of sigs) {
      if (sig.kind !== 'Divergence') continue;
      const subKind = sig.label;
      out.push({
        indicator_key: meta.key,
        display_name: meta.display_name,
        sub_kind: subKind,
        direction: sig.direction,
        status: sig.status,
        strength: sig.strength,
        confidence_pct: Math.round(Math.abs(indicators?.[meta.key]?.confidence ?? 0) * 100),
        age_bars: sig.age_bars,
        label: sig.label,
        pivots: sig.points ?? null,
      });
    }
  }
  return out.sort((a, b) => b.strength - a.strength);
}

function buildLevels(
  registry: IndicatorMeta[],
  indicators: Record<string, IndicatorDto>,
): LevelRow[] {
  const out: LevelRow[] = [];
  for (const meta of registry) {
    const sigs = indicators?.[meta.key]?.signals ?? [];
    for (const sig of sigs) {
      if (sig.kind !== 'LevelTest') continue;
      // Derive role from "role" word in label or fallback
      const lower = sig.label.toLowerCase();
      let role: 'support' | 'resistance' | 'neutral' = 'neutral';
      if (lower.includes('support')) role = 'support';
      else if (lower.includes('resistance') || lower.includes('resist')) role = 'resistance';
      out.push({
        indicator_key: meta.key,
        display_name: meta.display_name,
        level_name: sig.label,
        kind: 'Other',
        role,
        price_text: '—',
        direction: sig.direction,
        status: sig.status,
        strength: sig.strength,
        confidence_pct: Math.round(Math.abs(indicators?.[meta.key]?.confidence ?? 0) * 100),
        age_bars: sig.age_bars,
      });
    }
  }
  return out.sort((a, b) => b.strength - a.strength);
}

function buildLiquidityFlowExport(flow: LiquidityFlow | null): LiquidityFlowExport | null {
  if (!flow) return null;
  return {
    long_liquidations_usd: flow.long_liquidations_usd,
    short_liquidations_usd: flow.short_liquidations_usd,
    net_liquidation_usd: flow.net_liquidation_usd,
    event_count: flow.event_count,
    largest_event_usd: flow.largest_event_usd,
    largest_event_price: flow.largest_event_price ?? null,
    largest_event_side: flow.largest_event_side ?? null,
    cascade_state: flow.cascade_state,
    cascade_intensity: flow.cascade_intensity,
  };
}

function buildClusterMatrixExport(cm: LiquidationClusterMatrix | null): ClusterMatrixExport | null {
  if (!cm) return null;
  function topSide(arr: LiquidationCluster[] | undefined, dir: 'asc' | 'desc') {
    if (!arr) return [];
    return [...arr]
      .sort((a, b) => dir === 'asc' ? Math.abs(a.distance_from_mid_pct) - Math.abs(b.distance_from_mid_pct) : Math.abs(b.distance_from_mid_pct) - Math.abs(a.distance_from_mid_pct))
      .slice(0, 3);
  }
  return {
    mid_price: cm.mid_price,
    cascade_asymmetry: cm.cascade_asymmetry,
    total_long_oi_usd: cm.total_long_oi_usd,
    total_short_oi_usd: cm.total_short_oi_usd,
    estimation_confidence: cm.estimation_confidence,
    leverage_assumptions: {
      source: cm.leverage_assumptions.source,
      buckets: cm.leverage_assumptions.buckets,
      weights: cm.leverage_assumptions.weights,
      funding_modulation_active: cm.leverage_assumptions.funding_modulation_active,
    },
    top_above: topSide(cm.short_clusters, 'asc').map((c) => ({
      peak_price: c.peak_price,
      distance_from_mid_pct: c.distance_from_mid_pct,
      notional_usd: c.notional_usd,
      magnet_strength: c.magnet_strength,
      cluster_kind: c.cluster_kind,
    })),
    top_below: topSide(cm.long_clusters, 'asc').map((c) => ({
      peak_price: c.peak_price,
      distance_from_mid_pct: c.distance_from_mid_pct,
      notional_usd: c.notional_usd,
      magnet_strength: c.magnet_strength,
      cluster_kind: c.cluster_kind,
    })),
  };
}

function buildLiquiditySignals(signals: LiquiditySignal[]): LiquiditySignalExport[] {
  return (signals ?? []).map((s) => ({
    kind: s.kind,
    direction: s.direction,
    strength: s.strength,
    confidence: s.confidence,
    evidence: s.evidence,
  }));
}

// ── Public builder ───────────────────────────────────────────────────────

export interface MetricsTabInputs {
  tf: TimeframeTelemetry | null | undefined;
  registry: IndicatorMeta[];
  volumeProfile: VolumeProfileSnapshot | null;
  liquidity: LiquidityFlow | null;
  cluster: LiquidationClusterMatrix | null;
  liquiditySignals: LiquiditySignal[];
  symbol: string;
  tfLabel: string;
  tfSecs?: number | null;
  timestamp?: number | null;
  markPrice?: number | null;
  filterState?: {
    activeOnly: boolean;
    confirmedPlusOnly: boolean;
    hideGates: boolean;
    hideOverlays: boolean;
  };
}

/**
 * Build the Metrics tab (single-TF) export payload. Mirrors
 * `TerminalMonitor.svelte` single-TF mode 1:1.
 */
export function buildMetricsTabExport(args: MetricsTabInputs): string {
  const meta = buildMeta({
    sourceTab: 'metrics',
    symbol: args.symbol,
    tfSecs: args.tfSecs ?? null,
    timestamp: args.timestamp ?? null,
    markPrice: args.markPrice ?? null,
    isCompleted: args.tf?.isCompleted,
    pipelineState: args.tf?.pipelineState ?? null,
    filterState: args.filterState,
  });
  const tf = args.tf;
  const indicators = (tf?.indicators ?? {}) as Record<string, IndicatorDto>;
  const { signalsByKind, total: signalsTotal } = buildSignalsByKind(args.registry, indicators);
  const signalCount = Object.values(signalsByKind).reduce((sum, list) => sum + list.length, 0);
  const payload: MetricsPayload = {
    source_tab: 'metrics',
    meta,
    market_context: tf ? buildMarketContext(tf, signalCount) : null,
    group_confluence: buildGroupConfluence(args.registry, indicators),
    structural_anchors: {
      fibonacci: buildFibonacciSummary(indicators),
      volume_profile: buildVolumeProfileExport(args.volumeProfile),
      liquidation_clusters: buildLiquidationClusters(args.cluster),
      cascade_alert: buildCascadeAlert(args.liquidity),
    },
    indicators: tf ? buildIndicators(args.registry, indicators, tf) : [],
    signals_total: signalsTotal,
    signals_by_kind: signalsByKind,
    divergences: buildDivergences(args.registry, indicators),
    levels: buildLevels(args.registry, indicators),
    liquidity_signals: buildLiquiditySignals(args.liquiditySignals),
    liquidity_flow: buildLiquidityFlowExport(args.liquidity),
    cluster_matrix: buildClusterMatrixExport(args.cluster),
  };
  return JSON.stringify(payload, null, 2);
}
