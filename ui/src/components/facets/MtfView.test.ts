// @vitest-environment jsdom
//
// v6.13 — MtfView stacked-tables contract.
//
// 1. The grid always lists every registered indicator (no filter state can
//    remove rows — the component takes no `filters` prop at all).
// 2. Below the grid sit three stacked cross-timeframe tables in the same
//    4-TF-column layout as the indicator grid: SIGNALS (12 signal kinds ×
//    per-TF active counts), DIVERGENCES (capable indicators × strongest
//    sub-type per TF), LEVELS (9 level kinds × per-TF LevelTest counts).
// 3. When no signals exist, each table shows its awaiting-data note
//    instead of hiding.

import { describe, it, expect } from 'vitest';
import { cleanup, render } from '@testing-library/svelte';
import MtfView from './MtfView.svelte';
import type {
    IndicatorMeta,
    IndicatorSignal,
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
    } as TimeframeTelemetry;
}

function makeRegistry(): IndicatorMeta[] {
    return [
        {
            key: 'rsi',
            display_name: 'RSI',
            group: 'Momentum',
            class: 'Leading',
            render: 'Pane',
            directional: true,
            supports_divergence: true,
            signal_types: ['Crossover', 'Threshold'],
            default_weight: 1,
            default_enabled: true,
            config_params: [],
            value_format: 'decimals2',
            value_source: 'indicator',
            color: '#22c55e',
            guide_section: 'oscillators',
        },
        {
            key: 'macd',
            display_name: 'MACD',
            group: 'Momentum',
            class: 'Lagging',
            render: 'Pane',
            directional: true,
            supports_divergence: true,
            signal_types: ['Crossover', 'ZeroLineCross'],
            default_weight: 1,
            default_enabled: true,
            config_params: [],
            value_format: 'decimals2',
            value_source: 'indicator',
            color: '#a855f7',
            guide_section: 'oscillators',
        },
        {
            key: 'volume',
            display_name: 'Volume',
            group: 'Volume',
            class: 'Hybrid',
            render: 'Pane',
            directional: false,
            supports_divergence: false,
            signal_types: [],
            default_weight: 1,
            default_enabled: true,
            config_params: [],
            value_format: 'price',
            value_source: 'indicator',
            color: '#eab308',
            guide_section: 'volume',
        },
    ] as IndicatorMeta[];
}

function makeSignal(kind: IndicatorSignal['kind'], label: string, status: IndicatorSignal['status'] = 'Active', strength = 0.7): IndicatorSignal {
    return {
        kind,
        direction: 'Bullish',
        status,
        label,
        strength,
        age_bars: 1,
        points: null,
    };
}

function makePair(overrides: Partial<Record<TimeframeSlotKind, Partial<TimeframeTelemetry>>> = {}) {
    const mk = (slot: TimeframeSlotKind, secs: number): TimeframeTelemetry =>
        makeTf({ slot, barDurationSec: secs, ...(overrides[slot] ?? {}) });
    return {
        microTerm: mk('micro', 60),
        fastTerm: mk('fast', 180),
        slowTerm: mk('slow', 300),
        macroTerm: mk('macro', 900),
    };
}

afterEach(() => cleanup());

describe('MtfView — grid is always unfiltered (v6.11)', () => {
    it('lists every registered indicator, including ones with no signals', () => {
        const pair = makePair();
        const { container } = render(MtfView, { props: { pair, registry: makeRegistry() } });
        const text = container.textContent ?? '';
        expect(text).toContain('RSI');
        expect(text).toContain('MACD');
        expect(text).toContain('Volume');
    });

    it('renders the normalized value grid across the 4 timeframes', () => {
        const pair = makePair({
            micro: { indicators: { rsi: { raw_value: 60, normalized: 0.4, state_label: 'POSITIVE', values: null, signals: [], confidence: 0.5 } } },
            macro: { indicators: { rsi: { raw_value: 30, normalized: -0.6, state_label: 'NEGATIVE', values: null, signals: [], confidence: 0.5 } } },
        });
        const { container } = render(MtfView, { props: { pair, registry: makeRegistry() } });
        const text = container.textContent ?? '';
        expect(text).toContain('+0.40');
        expect(text).toContain('-0.60');
    });
});

describe('MtfView — stacked cross-timeframe tables (v6.13)', () => {
    it('renders the three stacked tables — Signals / Divergences / Levels — each with the 4-TF header', () => {
        const pair = makePair({
            micro: {
                indicators: {
                    rsi: { raw_value: 70, normalized: 0.7, state_label: 'POSITIVE', values: null, signals: [makeSignal('Crossover', 'RSI bullish crossover', 'Confirmed', 0.9)], confidence: 0.9 },
                },
            },
            slow: {
                indicators: {
                    macd: { raw_value: 5, normalized: 0.5, state_label: 'POSITIVE', values: null, signals: [makeSignal('ZeroLineCross', 'MACD zero-line cross', 'Active', 0.6)], confidence: 0.7 },
                },
            },
        });
        const { container } = render(MtfView, { props: { pair, registry: makeRegistry() } });
        const text = container.textContent ?? '';
        // Section titles for all three stacked tables.
        expect(text).toContain('Signals');
        expect(text).toContain('Divergences');
        expect(text).toContain('Levels');
        // The Signals table aggregates by kind — kind name + abbr badge.
        expect(text).toContain('Crossover');
        expect(text).toContain('CRO');
        expect(text).toContain('ZeroLineCross');
        expect(text).toContain('0X');
        // Header count sums the two active signals.
        expect(text).toContain('2 signals');
        // Per-TF headers render inside every table (Micro/Fast/Slow/Macro).
        expect(text).toContain('TOTAL');
    });

    it('Signals table counts the same indicator firing on multiple timeframes per timeframe', () => {
        const mkSignal = (label: string) =>
            makeSignal('Threshold', label, 'Active', 0.8);
        const pair = makePair({
            micro: {
                indicators: {
                    rsi: { raw_value: 75, normalized: 0.8, state_label: 'POSITIVE', values: null, signals: [mkSignal('Micro RSI overbought')], confidence: 0.9 },
                },
            },
            fast: {
                indicators: {
                    rsi: { raw_value: 72, normalized: 0.7, state_label: 'POSITIVE', values: null, signals: [mkSignal('Fast RSI overbought')], confidence: 0.9 },
                },
            },
        });
        const { container } = render(MtfView, { props: { pair, registry: makeRegistry() } });
        const text = container.textContent ?? '';
        expect(text).toContain('2 signals');
        expect(text).toContain('Threshold');
        expect(text).toContain('TH');
    });

    it('Divergences table shows the strongest divergence sub-type per oscillator per timeframe', () => {
        const pair = makePair({
            macro: {
                indicators: {
                    rsi: {
                        raw_value: 30, normalized: -0.6, state_label: 'NEGATIVE', values: null,
                        signals: [makeSignal('Divergence', 'BULLISH_DIVERGENCE', 'Active', 0.8)],
                        confidence: 0.7,
                    },
                },
            },
        });
        const { container } = render(MtfView, { props: { pair, registry: makeRegistry() } });
        const text = container.textContent ?? '';
        expect(text).toContain('1 divergence');
        // classifyDivergence('BULLISH_DIVERGENCE') → RegularBull → short 'BULL'.
        expect(text).toContain('BULL');
    });

    it('Levels table counts LevelTest signals per level kind per timeframe', () => {
        const pair = makePair({
            micro: {
                indicators: {
                    rsi: {
                        raw_value: 50, normalized: 0, state_label: 'NEUTRAL', values: null,
                        signals: [makeSignal('LevelTest', 'RSI_LEVEL_TEST', 'Active', 0.7)],
                        confidence: 0.5,
                    },
                },
            },
        });
        const { container } = render(MtfView, { props: { pair, registry: makeRegistry() } });
        const text = container.textContent ?? '';
        expect(text).toContain('1 level test');
        // classifyLevelKey('rsi') → Other — the "Other" kind row carries the count.
        expect(text).toContain('Other');
    });

    it('shows the awaiting-completed-candle note when no signals exist', () => {
        const pair = makePair();
        const { container } = render(MtfView, { props: { pair, registry: makeRegistry() } });
        const text = container.textContent ?? '';
        expect(text).toContain('Signals');
        expect(text).toContain('0 signals');
        expect(text).toContain('No signals active');
        expect(text).toContain('No active divergences');
        expect(text).toContain('No active level tests');
    });
});

// ── v6.14 — metrics-panel UX upgrade contracts ─────────────────────────
// 1. Section titles are standalone headings OUTSIDE the containers.
// 2. WARMING entries render '--' (never a misleading +0.00) and gated
//    (non-Directional) indicators render 'N/A'.
// 3. SIGNALS cells split each kind by direction (▲ bull / ▼ bear / — neutral)
//    and the TOTAL column lights up the dominant side (data-lit="true").
// 4. DIVERGENCES rows carry a BULL/BEAR/MIXED total direction badge.
// 5. LEVELS cells show chips of the actual level names; totals carry the
//    direction split plus a support-vs-resistance (S/R) split.

function makeSignalEx(
    kind: IndicatorSignal['kind'],
    label: string,
    opts: {
        direction?: IndicatorSignal['direction'];
        status?: IndicatorSignal['status'];
        strength?: number;
        age_bars?: number;
    } = {},
): IndicatorSignal {
    return {
        kind,
        direction: opts.direction ?? 'Bullish',
        status: opts.status ?? 'Active',
        label,
        strength: opts.strength ?? 0.7,
        age_bars: opts.age_bars ?? 1,
        points: null,
    };
}

function pivotMeta(): IndicatorMeta {
    return {
        key: 'pivot_points',
        display_name: 'Pivot Points',
        group: 'Structure',
        class: 'Lagging',
        render: 'Pane',
        directional: false,
        supports_divergence: false,
        signal_types: ['LevelTest'],
        default_weight: 1,
        default_enabled: true,
        config_params: [],
        value_format: 'decimals2',
        value_source: 'indicator',
        color: '#60a5fa',
        guide_section: 'levels',
    } as IndicatorMeta;
}

describe('MtfView — v6.14 standalone section headings', () => {
    it('renders the four headings (Indicators / Signals / Divergences / Levels) outside the tables', () => {
        const pair = makePair({
            micro: {
                indicators: {
                    rsi: {
                        raw_value: 70, normalized: 0.7, state_label: 'POSITIVE', values: null,
                        signals: [makeSignal('Crossover', 'RSI bullish crossover', 'Active', 0.9)],
                        confidence: 0.9,
                    },
                },
            },
        });
        const { container } = render(MtfView, { props: { pair, registry: makeRegistry() } });
        const headings = Array.from(container.querySelectorAll('h3')).map((h) => h.textContent);
        expect(headings).toEqual(['Indicators', 'Signals', 'Divergences', 'Levels']);
    });
});

describe('MtfView — v6.14 warming and gated cells never mislead', () => {
    it('renders WARMING entries as -- (never +0.00) and drops them from agreement', () => {
        const pair = makePair({
            micro: {
                indicators: {
                    rsi: { raw_value: 0, normalized: 0, state_label: 'WARMING', values: null, signals: [], confidence: 0 },
                },
            },
        });
        const { container } = render(MtfView, { props: { pair, registry: makeRegistry() } });
        const text = container.textContent ?? '';
        expect(text).toContain('--');
        // Agreement is unavailable (no real readings) — no fabricated +0.00.
        expect(text).not.toContain('+0.00');
    });

    it('renders gated (ContextOnly) indicators as N/A instead of a directional value', () => {
        const registry = makeRegistry().map((m) =>
            m.key === 'volume' ? { ...m, normalization_mode: 'ContextOnly' as const } : m,
        );
        const pair = makePair({
            micro: {
                indicators: {
                    volume: { raw_value: 5, normalized: 0, state_label: 'POSITIVE', values: null, signals: [], confidence: 0.5 },
                },
            },
        });
        const { container } = render(MtfView, { props: { pair, registry } });
        const text = container.textContent ?? '';
        expect(text).toContain('N/A');
        expect(text).not.toContain('+0.00');
    });
});

describe('MtfView — v6.14 signals direction split with lit totals', () => {
    it('splits each kind per timeframe into ▲ bull / ▼ bear badges and lights the dominant total', () => {
        const pair = makePair({
            micro: {
                indicators: {
                    rsi: {
                        raw_value: 70, normalized: 0.7, state_label: 'POSITIVE', values: null,
                        signals: [
                            makeSignalEx('Crossover', 'RSI bull cross', { direction: 'Bullish' }),
                            makeSignalEx('Crossover', 'RSI bear cross', { direction: 'Bearish' }),
                        ],
                        confidence: 0.9,
                    },
                },
            },
            fast: {
                indicators: {
                    rsi: {
                        raw_value: 30, normalized: -0.7, state_label: 'NEGATIVE', values: null,
                        signals: [makeSignalEx('Crossover', 'Fast bear cross', { direction: 'Bearish' })],
                        confidence: 0.9,
                    },
                },
            },
        });
        const { container } = render(MtfView, { props: { pair, registry: makeRegistry() } });
        const text = container.textContent ?? '';
        // Per-cell split: micro carries ▲ 1 and ▼ 1; total row ▼ 2 (bear dominates).
        expect(text).toContain('▲ 1');
        expect(text).toContain('▼ 1');
        expect(text).toContain('▼ 2');
        // The dominant (bear) total badge is lit; the bull one is not.
        expect(container.querySelector('[data-dir="bear"][data-lit="true"]')).toBeTruthy();
        expect(container.querySelector('[data-dir="bull"][data-lit="true"]')).toBeFalsy();
    });

    it('renders a lit bullish total when bulls outnumber bears', () => {
        const pair = makePair({
            micro: {
                indicators: {
                    rsi: {
                        raw_value: 70, normalized: 0.7, state_label: 'POSITIVE', values: null,
                        signals: [
                            makeSignalEx('Threshold', 'RSI hot', { direction: 'Bullish' }),
                            makeSignalEx('Threshold', 'RSI hot 2', { direction: 'Bullish' }),
                        ],
                        confidence: 0.9,
                    },
                },
            },
        });
        const { container } = render(MtfView, { props: { pair, registry: makeRegistry() } });
        expect(container.querySelector('[data-dir="bull"][data-lit="true"]')).toBeTruthy();
        expect(container.querySelector('[data-dir="bear"][data-lit="true"]')).toBeFalsy();
    });
});

describe('MtfView — v6.14 divergence totals carry a direction badge', () => {
    it('shows MIXED when bullish and bearish divergences balance', () => {
        const pair = makePair({
            micro: {
                indicators: {
                    rsi: {
                        raw_value: 70, normalized: 0.7, state_label: 'POSITIVE', values: null,
                        signals: [makeSignalEx('Divergence', 'BULLISH_DIVERGENCE', { direction: 'Bullish' })],
                        confidence: 0.9,
                    },
                },
            },
            fast: {
                indicators: {
                    rsi: {
                        raw_value: 30, normalized: -0.7, state_label: 'NEGATIVE', values: null,
                        signals: [makeSignalEx('Divergence', 'BEARISH_DIVERGENCE', { direction: 'Bearish' })],
                        confidence: 0.9,
                    },
                },
            },
        });
        const { container } = render(MtfView, { props: { pair, registry: makeRegistry() } });
        expect(container.querySelector('[data-dir="mixed"]')).toBeTruthy();
        // A balanced 1:1 split must not light up either side.
        expect(container.querySelector('[data-dir="bull"][data-lit="true"]')).toBeFalsy();
        expect(container.querySelector('[data-dir="bear"][data-lit="true"]')).toBeFalsy();
    });

    it('shows BULL when bullish divergences dominate', () => {
        const pair = makePair({
            micro: {
                indicators: {
                    rsi: {
                        raw_value: 70, normalized: 0.7, state_label: 'POSITIVE', values: null,
                        signals: [
                            makeSignalEx('Divergence', 'BULLISH_DIVERGENCE', { direction: 'Bullish' }),
                            makeSignalEx('Divergence', 'HIDDEN_BULLISH_DIVERGENCE', { direction: 'Bullish' }),
                        ],
                        confidence: 0.9,
                    },
                },
            },
        });
        const { container } = render(MtfView, { props: { pair, registry: makeRegistry() } });
        expect(container.querySelector('[data-dir="bull"]')).toBeTruthy();
    });
});

describe('MtfView — v6.14 levels show actual level chips with S/R split totals', () => {
    it('renders level-name chips per cell and direction + S/R totals per row', () => {
        const registry = [...makeRegistry(), pivotMeta()];
        const pair = makePair({
            micro: {
                indicators: {
                    pivot_points: {
                        raw_value: 50000, normalized: 0, state_label: 'POSITIVE',
                        values: { r2: 52000, s1: 48000 },
                        signals: [
                            makeSignalEx('LevelTest', 'PIVOT_R2_RESISTANCE_TEST', { direction: 'Bearish' }),
                            makeSignalEx('LevelTest', 'PIVOT_R2_RESISTANCE_TEST', { direction: 'Bearish' }),
                            makeSignalEx('LevelTest', 'PIVOT_S1_SUPPORT_TEST', { direction: 'Bullish' }),
                        ],
                        confidence: 0.5,
                    },
                },
            },
        });
        const { container } = render(MtfView, { props: { pair, registry } });
        const text = container.textContent ?? '';
        // Actual level names surfaced as chips (deduped with a repeat count).
        expect(text).toContain('R2 ×2');
        expect(text).toContain('S1');
        // Row total: direction split (▼ 2 lit vs ▲ 1) + S/R role split.
        expect(text).toContain('▲ 1');
        expect(text).toContain('▼ 2');
        expect(text).toContain('S 1');
        expect(text).toContain('R 2');
        expect(container.querySelector('[data-dir="bear"][data-lit="true"]')).toBeTruthy();
    });
});
