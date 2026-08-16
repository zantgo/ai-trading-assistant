// @vitest-environment jsdom
//
// v6.11 — MtfView unfiltered-signals contract.
//
// 1. The grid always lists every registered indicator (no filter state can
//    remove rows — the component takes no `filters` prop at all).
// 2. The CROSS-TIMEFRAME SIGNALS section renders EVERY signal from EVERY
//    timeframe, tagged with its producing slot (Micro/Fast/Slow/Macro).
//    Signals firing on multiple timeframes appear once per timeframe.
// 3. When no signals exist, the section shows the "awaiting completed
//    candle" note instead of hiding.

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

describe('MtfView — cross-timeframe signals show EVERY signal (v6.11)', () => {
    it('renders signals from different timeframes, each tagged with its slot', () => {
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
        expect(text).toContain('Cross-Timeframe Signals');
        expect(text).toContain('RSI bullish crossover');
        expect(text).toContain('MACD zero-line cross');
        expect(text).toContain('Micro');
        expect(text).toContain('Slow');
        expect(text).toContain('CRO');
        expect(text).toContain('0X');
    });

    it('renders the same indicator firing on multiple timeframes once per timeframe', () => {
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
        expect(text).toContain('Micro RSI overbought');
        expect(text).toContain('Fast RSI overbought');
        expect(text).toContain('2 signals');
    });

    it('shows the awaiting-completed-candle note when no signals exist', () => {
        const pair = makePair();
        const { container } = render(MtfView, { props: { pair, registry: makeRegistry() } });
        const text = container.textContent ?? '';
        expect(text).toContain('Cross-Timeframe Signals');
        expect(text).toContain('0 signals');
        expect(text).toContain('No signals active');
    });
});
