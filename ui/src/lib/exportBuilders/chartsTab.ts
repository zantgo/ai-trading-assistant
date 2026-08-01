// Charts tab builders — per-sub-tab scoped export payloads.
//
// The Charts tab (sub-tab `terminal`) renders two stacked components:
//   - The upper chart area (LiveTerminal.svelte).
//   - The lower console (BottomConsole.svelte) with four sub-tabs:
//     Positions, Open Orders, History, Plan.
//
// Only the lower console has an Export Data button. Each sub-tab emits
// a JSON payload that mirrors the table the user actually sees, plus
// the always-visible Account Mini Bar and the top-bar symbol + mark price.
//
// This is the "single source of truth" — `BottomConsole.handleCopyJson`
// and `BottomTable.handleCopyJson` both route through these builders.

import type { AppStore } from '../../state.svelte';
import type {
  AccountBlock,
  CountsBlock,
  MetaEnvelope,
  SourceTab,
} from './shared';
import {
  buildAccountBlock,
  buildCountsBlock,
  buildMeta,
  fmtPnl,
  fmtTimeHM,
  parseMarkPrice,
} from './shared';

// ── Per-tab payload types ───────────────────────────────────────────────

export interface PositionBlock {
  symbol: string;
  direction: string;
  size: number | null;
  average_entry_price: number | null;
  liq_price: number | null;
  mark_price: number | null;
  margin_used: number;
  unrealized_pnl: number;
  unrealized_pnl_display: string;
  unrealized_roi_pct: number;
  opened_at: number | null;
  leverage: number;
}

export interface SlotBlock {
  slot_index: number;
  entry_price: number | null;
  size: number | null;
  allocated_usd: number | null;
  is_active: boolean;
  mark_price: number | null;
  pnl: number | null;
  status: 'Active' | 'Vacant';
}

export interface BracketBlock {
  id: number | null;
  order_type: string;
  price: number | null;
  trigger_price: number | null;
  size_pct: number | null;
}

export interface BracketsBlock {
  take_profit: BracketBlock[];
  stop_loss: BracketBlock[];
}

export interface PositionsPayload {
  source_tab: SourceTab;
  exported_at: string;
  symbol: string;
  mark_price: number | null;
  active_view: 'positions';
  counts: CountsBlock;
  position: PositionBlock | null;
  slots: SlotBlock[];
  brackets: BracketsBlock;
  account: AccountBlock;
}

export interface OpenOrderRow {
  id: number | null;
  order_type: string;
  direction: string;
  price: number | null;
  trigger_price: number | null;
  size_pct: number | null;
  created_at: number | null;
  created_at_display: string;
}

export interface OrdersPayload {
  source_tab: SourceTab;
  exported_at: string;
  symbol: string;
  mark_price: number | null;
  active_view: 'orders';
  counts: CountsBlock;
  open_orders: OpenOrderRow[];
  account: AccountBlock;
}

export interface HistoryRow {
  exit_timestamp: number | null;
  exit_timestamp_display: string;
  symbol: string;
  direction: string;
  entry_price: number | null;
  exit_price: number | null;
  realized_pnl: number | null;
  realized_pnl_display: string;
  roi_pct: number | null;
  trigger: string;
}

export interface HistoryPayload {
  source_tab: SourceTab;
  exported_at: string;
  symbol: string;
  mark_price: number | null;
  active_view: 'history';
  counts: CountsBlock;
  history: HistoryRow[];
  account: AccountBlock;
}

export interface PlanTargetRow {
  label: string;
  price: number;
  size_pct: number;
}

export interface PlanStopRow {
  label: string;
  price: number;
  distance_pct: number | null;
}

export interface PlanPayload {
  source_tab: SourceTab;
  exported_at: string;
  symbol: string;
  mark_price: number | null;
  active_view: 'plan';
  plan_source: string;
  plan_visible: boolean;
  counts: CountsBlock;
  targets: PlanTargetRow[];
  stop: PlanStopRow | null;
  account: AccountBlock;
}

// ── Internal helpers ─────────────────────────────────────────────────────

/** Mirror `calcLiqPrice` from `lib/telemetry.ts` (frontend-only). */
function calcLiqPrice(
  entry: number,
  direction: string,
  leverage: number,
): number {
  if (leverage <= 1) return 0;
  if (direction === 'LONG') return entry * (1 - 1 / leverage);
  if (direction === 'SHORT') return entry * (1 + 1 / leverage);
  return 0;
}

function readNumberField(obj: Record<string, unknown>, key: string): number | null {
  const v = obj[key];
  return typeof v === 'number' && isFinite(v) ? v : null;
}

function readStringField(obj: Record<string, unknown>, key: string): string {
  const v = obj[key];
  return typeof v === 'string' ? v : '';
}

function readIntField(obj: Record<string, unknown>, key: string): number | null {
  const v = obj[key];
  return typeof v === 'number' ? v : null;
}

function readBoolField(obj: Record<string, unknown>, key: string): boolean {
  return obj[key] === true;
}

function buildPositionBlock(app: AppStore, markPrice: number): PositionBlock | null {
  if (app.paperDirection === '') return null;
  const pos = (app.activePaperPosition ?? {}) as Record<string, unknown>;
  const entry = readNumberField(pos, 'average_entry_price')
    ?? readNumberField(pos, 'entry_price')
    ?? 0;
  const size = readNumberField(pos, 'size');
  const openedAt = readNumberField(pos, 'opened_at')
    ?? readNumberField(pos, 'created_at');
  const liq = entry > 0 ? calcLiqPrice(entry, app.paperDirection, app.paperLeverage) : 0;
  return {
    symbol: app.activeTab,
    direction: app.paperDirection,
    size,
    average_entry_price: entry > 0 ? entry : null,
    liq_price: liq > 0 ? liq : null,
    mark_price: markPrice > 0 ? markPrice : null,
    margin_used: app.paperMarginUsed,
    unrealized_pnl: app.paperUnrealizedPnl,
    unrealized_pnl_display: fmtPnl(app.paperUnrealizedPnl),
    unrealized_roi_pct: app.paperUnrealizedRoi,
    opened_at: openedAt,
    leverage: app.paperLeverage,
  };
}

function buildSlotsBlock(app: AppStore, markPrice: number): SlotBlock[] {
  const slots = app.activeSlots ?? [];
  const direction = app.paperDirection;
  return slots.map((raw) => {
    const s = raw as Record<string, unknown>;
    const slotIndex = readIntField(s, 'slot_index') ?? 0;
    const entry = readNumberField(s, 'entry_price') ?? 0;
    const size = readNumberField(s, 'size');
    const allocated = readNumberField(s, 'allocated_usd');
    const active = readBoolField(s, 'is_active');
    let pnl: number | null = null;
    if (active && entry > 0 && size != null && markPrice > 0) {
      pnl = direction === 'LONG'
        ? (markPrice - entry) * size
        : (entry - markPrice) * size;
    }
    return {
      slot_index: slotIndex,
      entry_price: entry > 0 ? entry : null,
      size,
      allocated_usd: allocated,
      is_active: active,
      mark_price: markPrice > 0 ? markPrice : null,
      pnl,
      status: active ? 'Active' : 'Vacant',
    };
  });
}

function buildBracketsBlock(app: AppStore): BracketsBlock {
  const tp: BracketBlock[] = [];
  const sl: BracketBlock[] = [];
  for (const raw of app.openOrders ?? []) {
    const o = raw as Record<string, unknown>;
    if (!(o as { is_reduce_only?: boolean }).is_reduce_only) continue;
    const isLimit = (o as { order_type?: string }).order_type === 'LIMIT';
    const block: BracketBlock = {
      id: readIntField(o, 'id'),
      order_type: readStringField(o, 'order_type'),
      price: readNumberField(o, 'price'),
      trigger_price: readNumberField(o, 'trigger_price'),
      size_pct: readNumberField(o, 'size'),
    };
    if (isLimit) tp.push(block); else sl.push(block);
  }
  return { take_profit: tp, stop_loss: sl };
}

function buildEntryOrders(app: AppStore): OpenOrderRow[] {
  const orders = (app.openOrders ?? []).filter(
    (o) => !(o as { is_reduce_only?: boolean }).is_reduce_only,
  );
  return orders.map((raw) => {
    const o = raw as Record<string, unknown>;
    const createdAt = readNumberField(o, 'created_at');
    return {
      id: readIntField(o, 'id'),
      order_type: readStringField(o, 'order_type'),
      direction: readStringField(o, 'direction'),
      price: readNumberField(o, 'price'),
      trigger_price: readNumberField(o, 'trigger_price'),
      size_pct: readNumberField(o, 'size'),
      created_at: createdAt,
      created_at_display: fmtTimeHM(createdAt),
    };
  });
}

function buildHistoryRows(app: AppStore): HistoryRow[] {
  return (app.paperHistory ?? []).map((raw) => {
    const t = raw as Record<string, unknown>;
    const exitTs = readNumberField(t, 'exit_timestamp');
    const realized = readNumberField(t, 'realized_pnl');
    return {
      exit_timestamp: exitTs,
      exit_timestamp_display: fmtTimeHM(exitTs),
      symbol: readStringField(t, 'symbol') || app.activeTab,
      direction: readStringField(t, 'direction'),
      entry_price: readNumberField(t, 'entry_price'),
      exit_price: readNumberField(t, 'exit_price'),
      realized_pnl: realized,
      realized_pnl_display: fmtPnl(realized ?? 0),
      roi_pct: readNumberField(t, 'roi_pct'),
      trigger: readStringField(t, 'trigger'),
    };
  });
}

function buildPlanPayload(app: AppStore, meta: MetaEnvelope, account: AccountBlock): PlanPayload {
  const plan = (app.activePlan ?? null) as Record<string, unknown> | null;
  const targets: PlanTargetRow[] = [];
  let stop: PlanStopRow | null = null;
  let planVisible = false;
  if (plan) {
    const rawTargets = Array.isArray(plan.targets) ? plan.targets : [];
    for (const t of rawTargets) {
      const row = t as Record<string, unknown>;
      targets.push({
        label: readStringField(row, 'label') || 'TP',
        price: readNumberField(row, 'price') ?? 0,
        size_pct: readNumberField(row, 'sizePct') ?? 0,
      });
    }
    const rawStop = plan.stop as Record<string, unknown> | undefined;
    if (rawStop && (readNumberField(rawStop, 'price') ?? 0) > 0) {
      stop = {
        label: 'SL',
        price: readNumberField(rawStop, 'price') ?? 0,
        distance_pct: readNumberField(rawStop, 'distancePct'),
      };
    }
    planVisible = true;
  }
  return {
    source_tab: 'plan',
    exported_at: meta.exported_at,
    symbol: meta.symbol,
    mark_price: meta.mark_price,
    active_view: 'plan',
    plan_source: 'L4_opportunity_matrix',
    plan_visible: planVisible,
    counts: buildCountsBlock(app),
    targets,
    stop,
    account,
  };
}

// ── Public builders ─────────────────────────────────────────────────────

/**
 * Build the Positions sub-tab export payload.
 * Mirrors `BottomConsole.svelte:208-330` (positions table + slot detail).
 */
export function buildPositionsTabExport(app: AppStore): string {
  const markPrice = parseMarkPrice(app.priceText) ?? 0;
  const meta = buildMeta({
    sourceTab: 'positions',
    symbol: app.activeTab,
    timestamp: Date.now(),
    markPrice,
  });
  const payload: PositionsPayload = {
    source_tab: 'positions',
    exported_at: meta.exported_at,
    symbol: app.activeTab,
    mark_price: meta.mark_price,
    active_view: 'positions',
    counts: buildCountsBlock(app),
    position: buildPositionBlock(app, markPrice),
    slots: buildSlotsBlock(app, markPrice),
    brackets: buildBracketsBlock(app),
    account: buildAccountBlock(app),
  };
  return JSON.stringify(payload, null, 2);
}

/**
 * Build the Open Orders sub-tab export payload.
 * Mirrors `BottomConsole.svelte:422-459` (orders table).
 */
export function buildOrdersTabExport(app: AppStore): string {
  const markPrice = parseMarkPrice(app.priceText) ?? 0;
  const meta = buildMeta({
    sourceTab: 'orders',
    symbol: app.activeTab,
    timestamp: Date.now(),
    markPrice,
  });
  const payload: OrdersPayload = {
    source_tab: 'orders',
    exported_at: meta.exported_at,
    symbol: app.activeTab,
    mark_price: meta.mark_price,
    active_view: 'orders',
    counts: buildCountsBlock(app),
    open_orders: buildEntryOrders(app),
    account: buildAccountBlock(app),
  };
  return JSON.stringify(payload, null, 2);
}

/**
 * Build the History sub-tab export payload.
 * Mirrors `BottomConsole.svelte:461-507` (history table).
 */
export function buildHistoryTabExport(app: AppStore): string {
  const markPrice = parseMarkPrice(app.priceText) ?? 0;
  const meta = buildMeta({
    sourceTab: 'history',
    symbol: app.activeTab,
    timestamp: Date.now(),
    markPrice,
  });
  const payload: HistoryPayload = {
    source_tab: 'history',
    exported_at: meta.exported_at,
    symbol: app.activeTab,
    mark_price: meta.mark_price,
    active_view: 'history',
    counts: buildCountsBlock(app),
    history: buildHistoryRows(app),
    account: buildAccountBlock(app),
  };
  return JSON.stringify(payload, null, 2);
}

/**
 * Build the Plan sub-tab export payload.
 * Mirrors `BottomConsole.svelte:333-420` (plan tab).
 * Even when no plan is loaded, returns a valid payload with empty
 * `targets` + `stop: null` + `plan_visible: false`.
 */
export function buildPlanTabExport(app: AppStore): string {
  const markPrice = parseMarkPrice(app.priceText) ?? 0;
  const meta = buildMeta({
    sourceTab: 'plan',
    symbol: app.activeTab,
    timestamp: Date.now(),
    markPrice,
  });
  const payload = buildPlanPayload(app, meta, buildAccountBlock(app));
  return JSON.stringify(payload, null, 2);
}
