// Shared types and helpers for the per-tab export builder pipeline.
//
// Every panel's `Export Data` button now produces a JSON payload that
// mirrors the data the panel actually renders. This file defines the
// shared envelope, types, and helper utilities used by each builder in
// `exportBuilders/<tab>.ts`.
//
// The previous "kitchen-sink" design (every panel exported the entire
// snapshot) is replaced with per-tab scoped payloads. The dispatcher
// in `metricsExport.ts` routes by `SourceTab` to the right builder.

import type { AppStore } from '../../state.svelte';

/**
 * Discriminator for every export payload. The `metricsExport.ts`
 * dispatcher uses this to route to the correct builder.
 */
export type SourceTab =
  | 'metrics'
  | 'mtf'
  | 'alignment'
  | 'opportunity'
  | 'risk'
  | 'analysis'
  | 'recommendation'
  | 'positions'
  | 'orders'
  | 'history'
  | 'plan';

/**
 * Meta envelope shared across every payload. Every builder emits
 * exactly one of these as the top-level `meta` field (or flattened
 * into the top-level for backwards compatibility with the existing
 * `metricsExport.ts` consumers).
 */
export interface MetaEnvelope {
  source_tab: SourceTab;
  exported_at: string;
  symbol: string;
  tf_secs?: number | null;
  timestamp: number | null;
  mark_price: number | null;
  is_completed?: boolean;
  pipeline_state?: string | null;
  filter_state?: FilterStateBlock;
}

/**
 * Account mini-bar block rendered on every Charts sub-tab.
 * Mirrors `BottomConsole.svelte:511-528`.
 */
export interface AccountBlock {
  balance: number;
  available: number;
  margin_used: number;
  leverage: number;
}

/**
 * Counts block rendered as tab badges across the bottom console.
 * Mirrors `BottomConsole.svelte:111-117` (the span.count next to each tab).
 */
export interface CountsBlock {
  positions: number;
  open_orders: number;
  history: number;
}

/**
 * Filter state mirrors the four toggle pills on the Metrics tab.
 * `filter_state` is included on every Market Monitor payload.
 */
export interface FilterStateBlock {
  active_only: boolean;
  confirmed_plus_only: boolean;
  hide_gates: boolean;
  hide_overlays: boolean;
}

// ── Helpers ─────────────────────────────────────────────────────────────

/** Build a typed meta envelope for a payload. */
export function buildMeta(args: {
  sourceTab: SourceTab;
  symbol: string;
  tfSecs?: number | null;
  timestamp?: number | null;
  markPrice?: number | null;
  isCompleted?: boolean;
  pipelineState?: string | null;
  filterState?: FilterStateBlock;
}): MetaEnvelope {
  return {
    source_tab: args.sourceTab,
    exported_at: new Date().toISOString(),
    symbol: args.symbol,
    tf_secs: args.tfSecs ?? null,
    timestamp: args.timestamp ?? null,
    mark_price: isFinite(args.markPrice ?? NaN) && (args.markPrice ?? 0) > 0
      ? args.markPrice ?? null
      : null,
    is_completed: args.isCompleted,
    pipeline_state: args.pipelineState ?? null,
    filter_state: args.filterState,
  };
}

/**
 * Build the account-mini-bar block from the app store.
 * Mirrors `BottomConsole.svelte:511-528`.
 */
export function buildAccountBlock(app: AppStore): AccountBlock {
  return {
    balance: app.paperTotalAccountValue,
    available: app.paperCashBalance,
    margin_used: app.paperMarginUsed,
    leverage: app.paperLeverage,
  };
}

/**
 * Build the counts block from the app store. Mirrors the badges
 * rendered at `BottomConsole.svelte:111-117`.
 */
export function buildCountsBlock(app: AppStore): CountsBlock {
  const entryOrders = (app.openOrders ?? []).filter(
    (o) => !(o as { is_reduce_only?: boolean }).is_reduce_only,
  );
  return {
    positions: app.paperDirection !== '' ? 1 : 0,
    open_orders: entryOrders.length,
    history: (app.paperHistory ?? []).length,
  };
}

/**
 * Defensively parse a price-text string into a number. Returns `null`
 * for `'--'`, empty strings, `NaN`, and non-positive values.
 */
export function parseMarkPrice(priceText: string | undefined | null): number | null {
  const v = parseFloat(priceText ?? '');
  if (!isFinite(v) || v <= 0) return null;
  return v;
}

/**
 * Update a `FilterStateBlock` from the canonical camelCase filter
 * object used by the Metrics tab's `FilterState`.
 */
export function buildFilterStateBlock(filters: {
  activeOnly: boolean;
  confirmedPlusOnly: boolean;
  hideGates: boolean;
  hideOverlays: boolean;
}): FilterStateBlock {
  return {
    active_only: filters.activeOnly,
    confirmed_plus_only: filters.confirmedPlusOnly,
    hide_gates: filters.hideGates,
    hide_overlays: filters.hideOverlays,
  };
}

/**
 * Format a timestamp (ms) as HH:MM display string. Used by Charts
 * tab builders to mirror the table UI's HH:MM column.
 */
export function fmtTimeHM(ts: number | null | undefined): string {
  if (!ts) return '—';
  return new Date(ts).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
}

/**
 * Format a USD number with magnitude suffixes (B/M/K).
 */
export function fmtUsd(n: number): string {
  if (!isFinite(n)) return '—';
  const abs = Math.abs(n);
  if (abs >= 1e9) return `$${(n / 1e9).toFixed(2)}B`;
  if (abs >= 1e6) return `$${(n / 1e6).toFixed(2)}M`;
  if (abs >= 1e3) return `$${(n / 1e3).toFixed(2)}K`;
  return `$${n.toFixed(0)}`;
}

/**
 * Format a percentage as a string with a fixed number of decimals.
 */
export function fmtPct(n: number, decimals = 2): string {
  if (!isFinite(n)) return '—';
  return `${(n * 100).toFixed(decimals)}%`;
}

/**
 * Format a USD price with a number of decimals scaled off the mark price.
 * Mirrors `fmtPx` in `BottomConsole.svelte:107-110`.
 */
export function fmtPxScaled(n: number | null | undefined, markPrice: number): string {
  if (n == null || !isFinite(n) || n <= 0) return '—';
  const abs = Math.abs(markPrice);
  let decimals: number;
  if (abs >= 10000) decimals = 1;
  else if (abs >= 1000) decimals = 2;
  else if (abs >= 100) decimals = 3;
  else if (abs >= 10) decimals = 4;
  else if (abs >= 1) decimals = 6;
  else decimals = 8;
  return n.toFixed(decimals);
}

/**
 * Format a signed P&L string with a leading `+` for non-negative values.
 */
export function fmtPnl(val: number): string {
  if (!isFinite(val)) return '$0.00';
  return (val >= 0 ? '+' : '') + '$' + val.toFixed(2);
}
