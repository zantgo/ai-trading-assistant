// @vitest-environment jsdom
import { describe, it, expect } from 'vitest';
import {
    STAGE_ORDER, INDICATOR_STAGE_ORDER, STAGE_META,
    stageForKey, categoryForKey, groupIndicatorsByStage,
} from '../lib/decisionStages';
import type { IndicatorMeta, IndicatorGroup } from '../types';

// The 51 registry keys (mirror crates/shared/src/indicators/registry.rs).
const ALL_KEYS: Array<[string, IndicatorGroup]> = [
    ['ema_stack', 'Trend'], ['supertrend', 'Trend'], ['donchian', 'Trend'], ['keltner', 'Trend'],
    ['adx', 'Trend'], ['vwap', 'Trend'], ['anchored_vwap', 'Trend'], ['ichimoku', 'Trend'], ['psar', 'Trend'],
    ['rsi', 'Momentum'], ['rsi_divergence', 'Momentum'], ['stochastic', 'Momentum'], ['stochastic_divergence', 'Momentum'],
    ['chandemo', 'Momentum'], ['chandemo_divergence', 'Momentum'], ['williams_r', 'Momentum'],
    ['awesome_oscillator', 'Momentum'], ['cci', 'Momentum'], ['macd', 'Momentum'], ['macd_divergence', 'Momentum'],
    ['hull_ma', 'Trend'],
    ['force_index', 'Volume'], ['volume', 'Volume'], ['rvol', 'Volume'], ['volume_profile', 'Volume'],
    ['obv', 'Volume'], ['obv_divergence', 'Volume'], ['cmf', 'Volume'], ['cmf_divergence', 'Volume'],
    ['mfi', 'Volume'], ['mfi_divergence', 'Volume'],
    ['atr', 'Volatility'], ['bollinger', 'Volatility'], ['bbwp', 'Volatility'], ['squeeze', 'Volatility'],
    ['squeeze_divergence', 'Volatility'], ['hv', 'Volatility'], ['stddev_channel', 'Volatility'],
    ['fibonacci', 'Structure'], ['support_resistance', 'Structure'], ['pivot_points', 'Structure'],
    ['patterns', 'Structure'], ['candlestick', 'Structure'],
    ['aroon', 'Regime'], ['choppiness', 'Regime'], ['linreg_slope', 'Regime'], ['zscore', 'Regime'],
    ['smc_structure', 'Institutional'], ['smc_liquidity', 'Institutional'],
    ['smc_fvg', 'Institutional'], ['smc_order_blocks', 'Institutional'],
];

function meta(key: string, group: IndicatorGroup): IndicatorMeta {
    return {
        key, display_name: key, group, class: 'Leading', render: 'Pane',
        directional: true, supports_divergence: false, signal_types: [],
        default_weight: 1, default_enabled: true, config_params: [],
        value_format: 'decimals2', value_source: 'raw', color: '#fff', guide_section: '',
    };
}

describe('TEST-UI: Decision Stage Mapping', () => {
    it('assigns every registry key to exactly one indicator stage', () => {
        for (const [key, group] of ALL_KEYS) {
            const stage = stageForKey(key, group);
            expect(INDICATOR_STAGE_ORDER).toContain(stage);
            expect(stage).not.toBe('Execution');
            expect(stage).not.toBe('Monitoring');
        }
    });

    it('places context indicators in Setup', () => {
        for (const k of ['ema_stack', 'supertrend', 'ichimoku', 'support_resistance', 'aroon', 'choppiness']) {
            expect(stageForKey(k, 'Trend')).toBe('Setup');
        }
    });

    it('places momentum/oscillators/price-action/breakouts in Trigger', () => {
        for (const k of ['rsi', 'macd', 'stochastic', 'patterns', 'candlestick', 'donchian', 'bollinger', 'squeeze']) {
            expect(stageForKey(k)).toBe('Trigger');
        }
    });

    it('places volume/trend-strength/volatility/smart-money/order-flow in Confirmation', () => {
        for (const k of ['volume', 'obv', 'cmf', 'mfi', 'adx', 'volume_profile', 'smc_liquidity', 'smc_fvg', 'smc_order_blocks']) {
            expect(stageForKey(k)).toBe('Confirmation');
        }
    });

    it('places volatility gauges (atr, hv, bbwp) in Confirmation', () => {
        for (const k of ['atr', 'hv', 'bbwp']) {
            expect(stageForKey(k)).toBe('Confirmation');
        }
    });

    it('falls back to a group-based stage for unknown keys', () => {
        expect(stageForKey('some_new_momentum_ind', 'Momentum')).toBe('Trigger');
        expect(stageForKey('some_new_volatility_ind', 'Volatility')).toBe('Confirmation');
        expect(stageForKey('unknown_no_group')).toBe('Setup');
    });

    it('assigns a fine category to every registry key', () => {
        const valid = new Set([
            'Trend', 'Momentum', 'Trend Strength', 'Volatility', 'Volume', 'Oscillators',
            'Market Structure', 'Support & Resistance', 'Price Action', 'Smart Money',
            'Order Flow', 'Market Regime',
        ]);
        for (const [key, group] of ALL_KEYS) {
            expect(valid.has(categoryForKey(key, group))).toBe(true);
        }
    });

    it('buckets a full registry preserving order and losing nothing', () => {
        const registry = ALL_KEYS.map(([k, g]) => meta(k, g));
        const buckets = groupIndicatorsByStage(registry);
        const total = buckets.reduce((n, [, metas]) => n + metas.length, 0);
        expect(total).toBe(ALL_KEYS.length);
        // Registry order preserved within each stage.
        for (const [, metas] of buckets) {
            const idxs = metas.map((m) => registry.findIndex((r) => r.key === m.key));
            const sorted = [...idxs].sort((a, b) => a - b);
            expect(idxs).toEqual(sorted);
        }
    });

    it('exposes stage metadata for all five lifecycle stages', () => {
        expect(STAGE_ORDER).toEqual(['Setup', 'Trigger', 'Confirmation', 'Execution', 'Monitoring']);
        for (const s of STAGE_ORDER) {
            expect(STAGE_META[s].title.length).toBeGreaterThan(0);
        }
    });
    
    it('has exactly 3 indicator-populated stages', () => {
        expect(INDICATOR_STAGE_ORDER).toEqual(['Setup', 'Trigger', 'Confirmation']);
    });
});