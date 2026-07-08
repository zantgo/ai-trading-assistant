// Decision-stage mapping: organizes the registry's 51 indicators into the
// trade-lifecycle hierarchy Setup → Trigger → Confirmation → Execution → Monitoring,
// and assigns each a fine-grained trading category. The registry remains the single
// source of truth; this module is a pure, stateless lookup layer. New registry
// keys fall back to a group-based default so nothing is ever dropped.

import type { IndicatorGroup, IndicatorMeta } from '../types';

export type DecisionStage = 'Setup' | 'Trigger' | 'Confirmation' | 'Execution' | 'Monitoring';

/** Trade-lifecycle order (Execution and Monitoring are synthesis-only; no raw indicators map to them). */
export const STAGE_ORDER: DecisionStage[] = ['Setup', 'Trigger', 'Confirmation', 'Execution', 'Monitoring'];

/** Stages that are populated by raw registry indicators (Execution and Monitoring are synthesis). */
export const INDICATOR_STAGE_ORDER: DecisionStage[] = ['Setup', 'Trigger', 'Confirmation'];

export interface StageMeta {
    stage: DecisionStage;
    title: string;
    subtitle: string;
}

export const STAGE_META: Record<DecisionStage, StageMeta> = {
    Setup: { stage: 'Setup', title: 'SETUP', subtitle: 'Trend · Market Regime · Structure' },
    Trigger: { stage: 'Trigger', title: 'TRIGGER', subtitle: 'Momentum · Price Action · Breakouts' },
    Confirmation: { stage: 'Confirmation', title: 'CONFIRMATION', subtitle: 'Volume · Trend Strength · Volatility · Smart Money · Order Flow' },
    Execution: { stage: 'Execution', title: 'EXECUTION', subtitle: 'Confluence · Decision Context' },
    Monitoring: { stage: 'Monitoring', title: 'MONITORING', subtitle: 'Trade Management · Scale · Trailing · Exit' },
};

/** Fine-grained trading categories used for per-indicator labelling. */
export type FineCategory =
    | 'Trend' | 'Momentum' | 'Trend Strength' | 'Volatility' | 'Volume'
    | 'Oscillators' | 'Market Structure' | 'Support & Resistance' | 'Price Action'
    | 'Smart Money' | 'Order Flow' | 'Market Regime';

// Per-key overrides (authoritative). Keys not listed fall back to group defaults.
const STAGE_BY_KEY: Record<string, DecisionStage> = {
    // ── Setup: context (trend, regime, structure) ──
    ema_stack: 'Setup', supertrend: 'Setup', ichimoku: 'Setup', psar: 'Setup',
    vwap: 'Setup', anchored_vwap: 'Setup', hull_ma: 'Setup', linreg_slope: 'Setup',
    aroon: 'Setup', choppiness: 'Setup', zscore: 'Setup',
    smc_structure: 'Setup', support_resistance: 'Setup', pivot_points: 'Setup', fibonacci: 'Setup',
    // ── Trigger: momentum / price action / breakouts ──
    rsi: 'Trigger', rsi_divergence: 'Trigger', stochastic: 'Trigger', stochastic_divergence: 'Trigger',
    chandemo: 'Trigger', chandemo_divergence: 'Trigger', williams_r: 'Trigger',
    awesome_oscillator: 'Trigger', cci: 'Trigger', macd: 'Trigger', macd_divergence: 'Trigger',
    squeeze: 'Trigger', squeeze_divergence: 'Trigger',
    patterns: 'Trigger', candlestick: 'Trigger',
    donchian: 'Trigger', keltner: 'Trigger', bollinger: 'Trigger', stddev_channel: 'Trigger',
    // ── Confirmation: volume / trend strength / volatility / smart money / order flow ──
    volume: 'Confirmation', rvol: 'Confirmation', obv: 'Confirmation', obv_divergence: 'Confirmation',
    cmf: 'Confirmation', cmf_divergence: 'Confirmation', mfi: 'Confirmation', mfi_divergence: 'Confirmation',
    force_index: 'Confirmation', volume_profile: 'Confirmation', adx: 'Confirmation',
    smc_liquidity: 'Confirmation', smc_fvg: 'Confirmation', smc_order_blocks: 'Confirmation',
    atr: 'Confirmation', hv: 'Confirmation', bbwp: 'Confirmation',
};

const STAGE_BY_GROUP: Record<IndicatorGroup, DecisionStage> = {
    Trend: 'Setup',
    Regime: 'Setup',
    Structure: 'Setup',
    Momentum: 'Trigger',
    Volume: 'Confirmation',
    Institutional: 'Confirmation',
    Volatility: 'Confirmation',
};

const CATEGORY_BY_KEY: Record<string, FineCategory> = {
    ema_stack: 'Trend', supertrend: 'Trend', ichimoku: 'Trend', psar: 'Trend',
    vwap: 'Trend', anchored_vwap: 'Trend', hull_ma: 'Trend', linreg_slope: 'Trend',
    donchian: 'Trend', keltner: 'Trend', stddev_channel: 'Volatility',
    adx: 'Trend Strength', aroon: 'Market Regime', choppiness: 'Market Regime', zscore: 'Market Regime',
    rsi: 'Oscillators', rsi_divergence: 'Momentum', stochastic: 'Oscillators', stochastic_divergence: 'Momentum',
    chandemo: 'Oscillators', chandemo_divergence: 'Momentum', williams_r: 'Oscillators',
    awesome_oscillator: 'Momentum', cci: 'Oscillators', macd: 'Momentum', macd_divergence: 'Momentum',
    squeeze: 'Volatility', squeeze_divergence: 'Momentum',
    patterns: 'Price Action', candlestick: 'Price Action',
    bollinger: 'Volatility', atr: 'Volatility', hv: 'Volatility', bbwp: 'Volatility',
    volume: 'Volume', rvol: 'Volume', obv: 'Volume', obv_divergence: 'Volume',
    cmf: 'Order Flow', cmf_divergence: 'Order Flow', mfi: 'Volume', mfi_divergence: 'Volume',
    force_index: 'Order Flow', volume_profile: 'Volume',
    fibonacci: 'Support & Resistance', support_resistance: 'Support & Resistance', pivot_points: 'Support & Resistance',
    smc_structure: 'Market Structure', smc_liquidity: 'Smart Money',
    smc_fvg: 'Order Flow', smc_order_blocks: 'Order Flow',
};

const CATEGORY_BY_GROUP: Record<IndicatorGroup, FineCategory> = {
    Trend: 'Trend',
    Momentum: 'Momentum',
    Volume: 'Volume',
    Volatility: 'Volatility',
    Structure: 'Market Structure',
    Regime: 'Market Regime',
    Institutional: 'Smart Money',
};

/** Resolve the decision stage for an indicator (override → group fallback). */
export function stageForKey(key: string, group?: IndicatorGroup): DecisionStage {
    return STAGE_BY_KEY[key] ?? (group ? STAGE_BY_GROUP[group] : 'Setup');
}

/** Resolve the fine trading category for an indicator (override → group fallback). */
export function categoryForKey(key: string, group?: IndicatorGroup): FineCategory {
    return CATEGORY_BY_KEY[key] ?? (group ? CATEGORY_BY_GROUP[group] : 'Trend');
}

/**
 * Bucket a registry manifest by decision stage, preserving registry order
 * within each stage. Only indicator-populated stages are returned.
 */
export function groupIndicatorsByStage(registry: IndicatorMeta[]): Array<[DecisionStage, IndicatorMeta[]]> {
    const map = new Map<DecisionStage, IndicatorMeta[]>();
    for (const stage of INDICATOR_STAGE_ORDER) map.set(stage, []);
    for (const meta of registry) {
        const stage = stageForKey(meta.key, meta.group);
        if (stage === 'Execution' || stage === 'Monitoring') continue; // synthesis-only
        (map.get(stage) ?? map.set(stage, []).get(stage)!).push(meta);
    }
    return INDICATOR_STAGE_ORDER
        .filter((s) => (map.get(s)?.length ?? 0) > 0)
        .map((s) => [s, map.get(s)!] as [DecisionStage, IndicatorMeta[]]);
}
