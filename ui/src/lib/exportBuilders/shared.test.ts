// Tests for the shared types and helpers in `exportBuilders/shared.ts`.
//
// Covers the defensive `parseMarkPrice` parser (the most error-prone
// helper) plus the meta builder's number-validation. The other
// helpers are pure formatters — they are exercised by the per-tab
// builder tests.

import { describe, it, expect } from 'vitest';
import {
  parseMarkPrice,
  buildMeta,
  buildFilterStateBlock,
  buildCountsBlock,
  buildAccountBlock,
  fmtUsd,
  fmtPct,
  fmtPnl,
  fmtTimeHM,
} from './shared';

describe('parseMarkPrice', () => {
  it('returns null for the dash placeholder', () => {
    expect(parseMarkPrice('--')).toBeNull();
  });
  it('returns null for empty string', () => {
    expect(parseMarkPrice('')).toBeNull();
  });
  it('returns null for undefined and null', () => {
    expect(parseMarkPrice(undefined)).toBeNull();
    expect(parseMarkPrice(null)).toBeNull();
  });
  it('returns null for NaN-producing strings', () => {
    expect(parseMarkPrice('abc')).toBeNull();
  });
  it('returns null for zero', () => {
    expect(parseMarkPrice('0')).toBeNull();
    expect(parseMarkPrice('0.0')).toBeNull();
  });
  it('returns null for negative values', () => {
    expect(parseMarkPrice('-5')).toBeNull();
  });
  it('parses positive numbers', () => {
    expect(parseMarkPrice('65000.00')).toBe(65000);
    expect(parseMarkPrice('123.456')).toBeCloseTo(123.456);
  });
});

describe('buildMeta', () => {
  it('produces a valid envelope with all fields', () => {
    const meta = buildMeta({
      sourceTab: 'metrics',
      symbol: 'BTC-USDT',
      tfSecs: 60,
      timestamp: 1753950000,
      markPrice: 65000,
      isCompleted: true,
      pipelineState: 'LIVE',
      filterState: {
        active_only: false,
        confirmed_plus_only: false,
        hide_gates: false,
        hide_overlays: false,
      },
    });
    expect(meta.source_tab).toBe('metrics');
    expect(meta.symbol).toBe('BTC-USDT');
    expect(meta.tf_secs).toBe(60);
    expect(meta.timestamp).toBe(1753950000);
    expect(meta.mark_price).toBe(65000);
    expect(meta.is_completed).toBe(true);
    expect(meta.pipeline_state).toBe('LIVE');
    expect(meta.exported_at).toMatch(/^\d{4}-\d{2}-\d{2}T/);
  });
  it('emits mark_price as null for invalid values', () => {
    expect(buildMeta({ sourceTab: 'metrics', symbol: 'X', markPrice: NaN }).mark_price).toBeNull();
    expect(buildMeta({ sourceTab: 'metrics', symbol: 'X', markPrice: 0 }).mark_price).toBeNull();
    expect(buildMeta({ sourceTab: 'metrics', symbol: 'X', markPrice: -1 }).mark_price).toBeNull();
  });
  it('defaults fields to null when omitted', () => {
    const meta = buildMeta({ sourceTab: 'plan', symbol: '' });
    expect(meta.symbol).toBe('');
    expect(meta.timestamp).toBeNull();
    expect(meta.mark_price).toBeNull();
    expect(meta.pipeline_state).toBeNull();
  });
});

describe('buildFilterStateBlock', () => {
  it('translates the camelCase filter object to snake_case', () => {
    const out = buildFilterStateBlock({
      activeOnly: true,
      confirmedPlusOnly: false,
      hideGates: true,
      hideOverlays: false,
    });
    expect(out.active_only).toBe(true);
    expect(out.confirmed_plus_only).toBe(false);
    expect(out.hide_gates).toBe(true);
    expect(out.hide_overlays).toBe(false);
  });
});

describe('buildCountsBlock + buildAccountBlock', () => {
  it('emits shaped objects when given an AppStore-like mock', () => {
    const mockApp = {
      paperDirection: 'LONG',
      openOrders: [
        { is_reduce_only: false },
        { is_reduce_only: true },
        { is_reduce_only: false },
      ],
      paperHistory: [{}, {}, {}],
      paperTotalAccountValue: 10000,
      paperCashBalance: 9500,
      paperMarginUsed: 500,
      paperLeverage: 10,
    } as unknown as Parameters<typeof buildCountsBlock>[0];

    const counts = buildCountsBlock(mockApp);
    expect(counts.positions).toBe(1);
    expect(counts.open_orders).toBe(2);
    expect(counts.history).toBe(3);

    const account = buildAccountBlock(mockApp);
    expect(account.balance).toBe(10000);
    expect(account.available).toBe(9500);
    expect(account.margin_used).toBe(500);
    expect(account.leverage).toBe(10);
  });
});

describe('formatters', () => {
  it('fmtUsd handles B/M/K magnitudes', () => {
    expect(fmtUsd(1.5e9)).toBe('$1.50B');
    expect(fmtUsd(2.3e6)).toBe('$2.30M');
    expect(fmtUsd(4.5e3)).toBe('$4.50K');
    expect(fmtUsd(250)).toBe('$250');
    expect(fmtUsd(NaN)).toBe('—');
  });
  it('fmtPct multiplies by 100 with fixed decimals', () => {
    expect(fmtPct(0.5)).toBe('50.00%');
    expect(fmtPct(0.0123, 1)).toBe('1.2%');
    expect(fmtPct(NaN)).toBe('—');
  });
  it('fmtPnl prepends + for non-negative values', () => {
    expect(fmtPnl(50)).toBe('+$50.00');
    expect(fmtPnl(-25)).toBe('$-25.00');
    expect(fmtPnl(0)).toBe('+$0.00');
    expect(fmtPnl(NaN)).toBe('$0.00');
  });
  it('fmtTimeHM returns dash for falsy timestamps', () => {
    expect(fmtTimeHM(0)).toBe('—');
    expect(fmtTimeHM(null)).toBe('—');
    expect(fmtTimeHM(undefined)).toBe('—');
  });
});
