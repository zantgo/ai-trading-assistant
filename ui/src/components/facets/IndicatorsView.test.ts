// @vitest-environment jsdom
//
// Tests for the WARMING-placeholder display behavior in IndicatorsView.svelte.
//
// Regression background: the analyzer inserts a zero-valued `WARMING` placeholder
// (`raw_value = 0.0`, `normalized = 0.0`, `state_label = "WARMING"`) for every
// registered key that hasn't produced a real reading yet. The previous UI
// rendered that placeholder as `Raw 0.00 / Norm 0.00`, which misled traders
// into reading a real value out of an unread entry. The fix: when the entry
// is a WARMING placeholder, render Raw `--` and Norm `--` (regardless of
// `normalization_mode`) so the row reads as "no data yet" until a real
// reading replaces the placeholder.

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { cleanup, render } from '@testing-library/svelte';
import IndicatorsView from './IndicatorsView.svelte';
import { defaultFilters } from '../../lib/filtering';
import type {
    IndicatorDto,
    IndicatorMeta,
    IndicatorLifecycleMap,
    TimeframeTelemetry,
    TimeframeSlotKind,
} from '../../types';

function makeTf(overrides: Partial<TimeframeTelemetry> = {}): TimeframeTelemetry {
    return {
        slot: 'micro' as TimeframeSlotKind,
        symbol: 'BTC-USDT',
        exchange: 'Hyperliquid',
        barDurationSec: 60,
        indicators: {},
        priceText: '50000.00',
        volText: '0',
        avgVolText: '0',
        showPatterns: true,
        isCompleted: true,
        latestSnapshot: null,
        historyPrices: [],
        ...overrides,
    };
}

function warmingPlaceholder(): IndicatorDto {
    // Mirrors the placeholder the analyzer constructs at
    // `crates/market-analyzer/src/indicators/normalized/all.rs:1742`.
    return {
        raw_value: 0,
        normalized: 0,
        state_label: 'WARMING',
        values: null,
        signals: [],
        confidence: 0,
    };
}

function realReading(raw: number, normalized: number, stateLabel: string): IndicatorDto {
    return {
        raw_value: raw,
        normalized,
        state_label: stateLabel,
        values: null,
        signals: [],
        confidence: 0.5,
    };
}

function lifecycle(entries: Record<string, { state: 'Live' | 'Loading' | 'Stale' | 'Failed'; barsSeen: number; barsRequired: number; feed_state?: 'Live' | 'WaitingFeed' | 'Silent' | 'Stale' }>): IndicatorLifecycleMap {
    const out: IndicatorLifecycleMap = {};
    for (const [k, v] of Object.entries(entries)) {
        out[k] = {
            state: v.state,
            bars_seen: v.barsSeen,
            bars_required: v.barsRequired,
            stale_threshold_secs: 300,
            ...(v.feed_state !== undefined ? { feed_state: v.feed_state } : {}),
        };
    }
    return out;
}

function meta(key: string, overrides: Partial<IndicatorMeta> = {}): IndicatorMeta {
    return {
        key,
        display_name: key,
        group: 'Momentum',
        class: 'Leading',
        render: 'Pane',
        directional: true,
        supports_divergence: false,
        signal_types: [],
        default_weight: 1.0,
        default_enabled: true,
        config_params: [],
        value_format: 'decimals2',
        value_source: 'raw',
        color: '#000000',
        guide_section: '',
        ...overrides,
    };
}

beforeEach(() => {
    (globalThis as any).__appStore = { instancesMap: {} };
});

afterEach(() => {
    cleanup();
});

/**
 * The table layout renders column headers AND row cells with the same class
 * names (`colRaw`, `colNorm`, `colState`). Row cells live inside
 * `[class*="rowWrap"]` elements; headers do not. Use this helper to grab
 * only the row cells and skip the header row.
 */
function rowCells(container: HTMLElement, classFragment: string): string[] {
    const rows = container.querySelectorAll('[class*="rowWrap"]');
    const out: string[] = [];
    rows.forEach((row) => {
        const cell = row.querySelector(`[class*="${classFragment}"]`);
        if (cell?.textContent != null) out.push(cell.textContent.trim());
    });
    return out;
}

describe('IndicatorsView WARMING placeholder rendering', () => {
    it('renders -- for Raw and Norm when entry is a WARMING placeholder (Directional mode)', () => {
        const tf = makeTf({
            indicators: {
                smc_structure: warmingPlaceholder(),
            },
            indicatorLifecycle: lifecycle({
                smc_structure: { state: 'Loading', barsSeen: 12, barsRequired: 50 },
            }),
        });
        const registry = [meta('smc_structure', {
            group: 'Institutional',
            class: 'Leading',
            render: 'Marker',
            normalization_mode: 'Directional',
        })];

        const { container } = render(IndicatorsView, {
            props: { tf, registry, filters: defaultFilters() },
        });

        const raws = rowCells(container, 'colRaw');
        const norms = rowCells(container, 'colNorm');
        expect(raws.length).toBe(1);
        expect(norms.length).toBe(1);
        expect(raws[0]).toBe('--');
        expect(norms[0]).toBe('--');
    });

    it('renders -- for Raw and Norm when entry is a WARMING placeholder (ContextOnly mode)', () => {
        // Previously, ContextOnly mode rendered `N/A` in the Norm column even
        // for the WARMING placeholder — confusing, because `N/A` is the
        // canonical "non-directional gate" marker, not "no data yet".
        const tf = makeTf({
            indicators: {
                funding_rate: warmingPlaceholder(),
            },
            indicatorLifecycle: lifecycle({
                funding_rate: { state: 'Loading', barsSeen: 5, barsRequired: 1 },
            }),
        });
        const registry = [meta('funding_rate', {
            group: 'DerivativesData',
            value_format: 'percent1',
            normalization_mode: 'ContextOnly',
        })];

        const { container } = render(IndicatorsView, {
            props: { tf, registry, filters: defaultFilters() },
        });

        const raws = rowCells(container, 'colRaw');
        const norms = rowCells(container, 'colNorm');
        expect(raws[0]).toBe('--');
        expect(norms[0]).toBe('--');
    });

    it('renders real values when the entry is a non-WARMING reading (regression guard)', () => {
        // After the WARMING-aware fix, real readings must still render their
        // actual values; the guard must not over-trigger.
        const tf = makeTf({
            indicators: {
                rsi: realReading(62.4, 0.42, 'RSI_BULLISH_BUT_NOT_OVERBOUGHT'),
            },
            indicatorLifecycle: lifecycle({
                rsi: { state: 'Live', barsSeen: 100, barsRequired: 14 },
            }),
        });
        const registry = [meta('rsi', { group: 'Momentum', value_format: 'decimals2' })];

        const { container } = render(IndicatorsView, {
            props: { tf, registry, filters: defaultFilters() },
        });

        const raws = rowCells(container, 'colRaw');
        const norms = rowCells(container, 'colNorm');
        expect(raws[0]).toBe('62.40');
        expect(norms[0]).toBe('0.42');
    });

    it('renders N/A in Norm for ContextOnly real readings (not WARMING)', () => {
        // ContextOnly mode should still render `N/A` for real entries (the
        // canonical contract: normalized is contractually 0.0).
        const tf = makeTf({
            indicators: {
                bbwp: realReading(50.0, 0.0, 'NORMAL_VOLATILITY_BULL_CYCLE'),
            },
            indicatorLifecycle: lifecycle({
                bbwp: { state: 'Live', barsSeen: 100, barsRequired: 252 },
            }),
        });
        const registry = [meta('bbwp', {
            group: 'Volatility',
            value_format: 'decimals2',
            normalization_mode: 'ContextOnly',
        })];

        const { container } = render(IndicatorsView, {
            props: { tf, registry, filters: defaultFilters() },
        });

        const raws = rowCells(container, 'colRaw');
        const norms = rowCells(container, 'colNorm');
        expect(raws[0]).toBe('50.00');
        expect(norms[0]).toBe('N/A');
    });

    it('renders Warming (X/Y) in State column when lifecycle is Loading', () => {
        const tf = makeTf({
            indicators: {
                smc_structure: warmingPlaceholder(),
            },
            indicatorLifecycle: lifecycle({
                smc_structure: { state: 'Loading', barsSeen: 12, barsRequired: 50 },
            }),
        });
        const registry = [meta('smc_structure', {
            group: 'Institutional',
            class: 'Leading',
            render: 'Marker',
        })];

        const { container } = render(IndicatorsView, {
            props: { tf, registry, filters: defaultFilters() },
        });

        const states = rowCells(container, 'colState');
        expect(states[0]).toContain('Warming');
        expect(states[0]).toContain('12/50');
    });

    it('renders WAITING FEED ⏳ in State column when lifecycle is Live but feed has not arrived (v6.6+)', () => {
        // v6.6: distinguishes "feed hasn't arrived yet" (WaitingFeed)
        // from "feed says zero" (SILENT ⚡). This is the exact symptom
        // the user reported: Bitget ticker channel's `holdingAmount`
        // was absent on cold start → OI / funding / OI-Δ / OI-Price
        // Divergence all showed SILENT ⚡ when they should have shown
        // WAITING FEED ⏳.
        const tf = makeTf({
            indicators: {
                // No value-map entry: only a placeholder so the row renders.
                // In production the entry is absent (per the analyzer's
                // WaitingFeed contract) so we pass an empty indicators map.
            },
            indicatorLifecycle: lifecycle({
                open_interest: { state: 'Live', barsSeen: 100, barsRequired: 1, feed_state: 'WaitingFeed' },
                funding_rate: { state: 'Live', barsSeen: 100, barsRequired: 1, feed_state: 'WaitingFeed' },
                oi_delta: { state: 'Live', barsSeen: 100, barsRequired: 1, feed_state: 'WaitingFeed' },
                oi_price_divergence: { state: 'Live', barsSeen: 100, barsRequired: 1, feed_state: 'WaitingFeed' },
            }),
        });
        const registry = [
            meta('open_interest', { group: 'DerivativesData', value_format: 'usd_notional' }),
            meta('funding_rate', { group: 'DerivativesData', value_format: 'percent1' }),
            meta('oi_delta', { group: 'DerivativesData', value_format: 'decimals2' }),
            meta('oi_price_divergence', { group: 'DerivativesData', value_format: 'decimals2' }),
        ];

        const { container } = render(IndicatorsView, {
            props: { tf, registry, filters: defaultFilters() },
        });

        const states = rowCells(container, 'colState');
        expect(states.length).toBe(4);
        for (const s of states) {
            expect(s).toContain('WAITING FEED');
        }
    });
});