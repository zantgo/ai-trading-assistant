// MTF (multi-timeframe) builder — scoped export payload mirroring the panel.
//
// The MTF view renders:
//   1. Header summary row (4 timeframe labels with duration_secs)
//   2. Per-group section headers (8 functional groups with indicator counts)
//   3. Per-indicator row (4 values normalized per TF + agreement + agreement_label)
//
// This builder emits:
//   - groups[] (rollup by functional group)
//   - indicators[] (cross-TF grid with per-TF normalized values)
//   - timeframes[] (per-TF full detail: indicators + fibonacci_summary + context)
//   - signals_total (unique labels across all 4 TFs)

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
import { buildMeta } from './shared';
import type { MetaEnvelope } from './shared';

export type MtfSlotLabel = 'Micro' | 'Fast' | 'Slow' | 'Macro';

export interface MtfTimeframeEntry {
  label: MtfSlotLabel;
  duration_seconds: number;
  mark_price: number | null;
  timestamp: number | null;
  pipeline_state: string | null;
  is_completed: boolean;
  context: Record<string, unknown> | null;
  fibonacci_summary: {
    present: boolean;
    gp_top: number | null;
    gp_bottom: number | null;
    ext_1618: number | null;
    ext_2618: number | null;
    retracement_coefficients: Record<string, number | null> | null;
  };
  indicators: Array<{
    key: string;
    display_name: string;
    group: string;
    class: string;
    raw: number | null;
    normalized: number;
    state: string;
    confidence_pct: number;
    signals: Array<{
      kind: string;
      direction: string;
      status: string;
      label: string;
      strength: number;
      age_bars: number | undefined;
    }>;
    sub_values: Record<string, number> | null;
    indicator_lifecycle: {
      state: string;
      bars_seen: number;
      bars_required: number;
    } | null;
  }>;
  liquidity_signals: Array<{
    kind: string;
    direction: string;
    strength: number;
    confidence: number;
    evidence: string[];
  }>;
  volume_profile: unknown | null;
  liquidity_flow: unknown | null;
  cluster_matrix: unknown | null;
}

export interface MtfIndicatorValue {
  timeframe: MtfSlotLabel;
  normalized: number;
  active: boolean;
}

export interface MtfIndicatorEntry {
  key: string;
  display_name: string;
  group: string;
  directional: boolean;
  values: MtfIndicatorValue[];
  agreement: number;
  agreement_label: 'BULL' | 'BEAR' | 'MIXED';
}

export interface MtfGroupEntry {
  key: string;
  label: string;
  accent: string;
  indicator_count: number;
}

export interface MtfPayload {
  source_tab: 'mtf';
  meta: MetaEnvelope;
  groups: MtfGroupEntry[];
  indicators: MtfIndicatorEntry[];
  timeframes: MtfTimeframeEntry[];
  signals_total: number;
}

const GROUP_ORDER = [
  'Trend', 'Momentum', 'Volume', 'Volatility',
  'Structure', 'Regime', 'Institutional', 'DerivativesData',
] as const;

const GROUP_META: Record<string, { label: string; accent: string }> = {
  Trend:           { label: 'Trend',        accent: '#22d3ee' },
  Momentum:        { label: 'Momentum',     accent: '#a78bfa' },
  Volume:          { label: 'Volume',       accent: '#fb923c' },
  Volatility:      { label: 'Volatility',   accent: '#ef4444' },
  Structure:       { label: 'Structure',    accent: '#60a5fa' },
  Regime:          { label: 'Regime',       accent: '#facc15' },
  Institutional:   { label: 'SMC',          accent: '#ec4899' },
  DerivativesData: { label: 'Derivatives',  accent: '#34d399' },
};

const SIGNAL_ABBR: Record<string, string> = {
  Divergence: 'DIV', Crossover: 'CRO', Threshold: 'TH', Breakout: 'BO',
  BandTouch: 'BT', ZeroLineCross: '0X', CompressionRelease: 'SQZ',
  LevelTest: 'LV', TrendFlip: 'FLIP', VolumeClimax: 'VOL',
  StackChange: 'STK', PatternForming: 'PAT',
};

function classifyAgreement(value: number): 'BULL' | 'BEAR' | 'MIXED' {
  if (value > 0.2) return 'BULL';
  if (value < -0.2) return 'BEAR';
  return 'MIXED';
}

function parseMarkPrice(priceText: string | undefined | null): number | null {
  const v = parseFloat(priceText ?? '');
  if (!isFinite(v) || v <= 0) return null;
  return v;
}

function parseSnapshotTimestamp(snap: unknown): number | null {
  if (!snap) return null;
  const ts = (snap as { timestamp?: unknown }).timestamp;
  return typeof ts === 'number' ? ts : null;
}

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

function extractFibSummary(indicators: Record<string, IndicatorDto>): MtfTimeframeEntry['fibonacci_summary'] {
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

function buildTimeframeEntry(
  label: MtfSlotLabel,
  tf: TimeframeTelemetry,
  registry: IndicatorMeta[],
): MtfTimeframeEntry {
  const indicators = (tf.indicators ?? {}) as Record<string, IndicatorDto>;
  const markPrice = parseMarkPrice(tf.priceText);
  const exportIndicators = registry.map((m) => {
    const dto = indicators[m.key];
    if (!dto) return null;
    const signals = (dto.signals ?? []).map((s) => ({
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
    return {
      key: m.key,
      display_name: m.display_name,
      group: m.group,
      class: m.class,
      raw: formatRawForExport(m, indicators),
      normalized: dto.normalized ?? 0,
      state: dto.state_label ?? '--',
      confidence_pct: confidencePct(indicators, m.key),
      signals,
      sub_values: Object.keys(subValues).length > 0 ? subValues : null,
      indicator_lifecycle: lc ? {
        state: lc.state,
        bars_seen: lc.bars_seen,
        bars_required: lc.bars_required,
      } : null,
    };
  }).filter((x): x is NonNullable<typeof x> => x !== null);

  const fibSummary = extractFibSummary(indicators);
  const ctx = (tf.context ?? null) as MarketContext | null;

  return {
    label,
    duration_seconds: tf.barDurationSec ?? 0,
    mark_price: markPrice,
    timestamp: parseSnapshotTimestamp(tf.latestSnapshot),
    pipeline_state: (tf.pipelineState ?? null) as string | null,
    is_completed: tf.isCompleted ?? false,
    context: ctx as unknown as Record<string, unknown> | null,
    fibonacci_summary: fibSummary,
    indicators: exportIndicators,
    liquidity_signals: [],
    volume_profile: (tf as { volumeProfile?: VolumeProfileSnapshot })?.volumeProfile ?? null,
    liquidity_flow: (tf as { liquidity?: LiquidityFlow })?.liquidity ?? null,
    cluster_matrix: (tf as { cluster?: LiquidationClusterMatrix })?.cluster ?? null,
  };
}

// ── Public builder ───────────────────────────────────────────────────────

export interface MtfTabInputs {
  pair: {
    microTerm: TimeframeTelemetry;
    fastTerm: TimeframeTelemetry;
    slowTerm: TimeframeTelemetry;
    macroTerm: TimeframeTelemetry;
  };
  registry: IndicatorMeta[];
  symbol: string;
  filterState?: {
    activeOnly: boolean;
    confirmedPlusOnly: boolean;
    hideGates: boolean;
    hideOverlays: boolean;
  };
}

/**
 * Build the MTF tab export payload. Mirrors `MtfView.svelte` 1:1.
 */
export function buildMtfExportJson(args: MtfTabInputs): string {
  const meta = buildMeta({
    sourceTab: 'mtf',
    symbol: args.symbol,
    filterState: args.filterState,
  });
  const slotDefs: { label: MtfSlotLabel; tf: TimeframeTelemetry }[] = [
    { label: 'Micro', tf: args.pair.microTerm },
    { label: 'Fast',  tf: args.pair.fastTerm },
    { label: 'Slow',  tf: args.pair.slowTerm },
    { label: 'Macro', tf: args.pair.macroTerm },
  ];
  const timeframes: MtfTimeframeEntry[] = slotDefs.map(({ label, tf }) =>
    buildTimeframeEntry(label, tf, args.registry),
  );

  const indicators: MtfIndicatorEntry[] = args.registry.map((meta) => {
    const values: MtfIndicatorValue[] = slotDefs.map(({ label, tf }) => {
      const dto = (tf.indicators ?? {})[meta.key];
      return {
        timeframe: label,
        normalized: dto?.normalized ?? 0,
        active: dto != null,
      };
    });
    const presentNorms = values.filter((v) => v.active).map((v) => v.normalized);
    const agreement = presentNorms.length > 0
      ? presentNorms.reduce((a, b) => a + b, 0) / presentNorms.length
      : 0;
    return {
      key: meta.key,
      display_name: meta.display_name,
      group: meta.group,
      directional: meta.directional ?? true,
      values,
      agreement,
      agreement_label: classifyAgreement(agreement),
    };
  });

  const groupCounts = new Map<string, number>();
  for (const ind of indicators) {
    groupCounts.set(ind.group, (groupCounts.get(ind.group) ?? 0) + 1);
  }
  const groups: MtfGroupEntry[] = GROUP_ORDER
    .filter((k) => (groupCounts.get(k) ?? 0) > 0)
    .map((k) => ({
      key: k,
      label: GROUP_META[k]?.label ?? k,
      accent: GROUP_META[k]?.accent ?? 'rgba(255,255,255,0.4)',
      indicator_count: groupCounts.get(k) ?? 0,
    }));

  const uniqueLabels = new Set<string>();
  for (const { tf } of slotDefs) {
    const inds = (tf.indicators ?? {}) as Record<string, IndicatorDto>;
    for (const k of Object.keys(inds)) {
      for (const s of inds[k]?.signals ?? []) {
        if (s.label) uniqueLabels.add(s.label);
      }
    }
  }

  const payload: MtfPayload = {
    source_tab: 'mtf',
    meta,
    groups,
    indicators,
    timeframes,
    signals_total: uniqueLabels.size,
  };
  return JSON.stringify(payload, null, 2);
}
