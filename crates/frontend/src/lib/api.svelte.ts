import type { AppStore } from '../state.svelte';
import type { InstanceState } from '../types';

export function formatIntervalRemaining(totalSeconds: number): string {
    const h = Math.floor(totalSeconds / 3600);
    const m = Math.floor((totalSeconds % 3600) / 60);
    const s = totalSeconds % 60;
    if (h > 0) return `${h}h ${m.toString().padStart(2, '0')}m`;
    if (m > 0) return `${m}m ${s.toString().padStart(2, '0')}s`;
    return `${s}s`;
}

// ─── Raw API fetch wrappers (no state mutation) ─────────────────────────────

export async function fetchConfigFromServer(): Promise<Record<string, unknown>> {
    const res = await fetch(`/api/config?_=${Date.now()}`);
    if (!res.ok) throw new Error(`Config fetch failed: ${res.status}`);
    return res.json();
}

export async function saveRulesCall(content: string): Promise<boolean> {
    const res = await fetch('/api/rules', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ content }),
    });
    return res.ok;
}

export async function fetchRulesCall(): Promise<string> {
    const res = await fetch('/api/rules');
    const data = await res.json();
    return data.content || '';
}

export async function saveIntervalsConfigCall(slowSecs: number, normalSecs: number, fastSecs: number): Promise<boolean> {
    const res = await fetch('/api/config');
    const config = await res.json();
    config.intervals = {
        slow_seconds: slowSecs,
        normal_seconds: normalSecs,
        fast_seconds: fastSecs,
    };
    const saveRes = await fetch('/api/config', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(config),
    });
    return saveRes.ok;
}

// ─── Config application logic ──────────────────────────────────────────────

export interface ApplyConfigResult {
    firstSymbol: string;
}

/** Applies config from the server to the AppStore. Returns data needed for component-local state. */
export function applyConfigToStore(app: AppStore, config: Record<string, unknown>): ApplyConfigResult {
    app.apiKeyConfigured = (config.api_key_configured as boolean) ?? true;

    if (config.candles) app.globalCandlesConfig = config.candles as { duration_seconds: number; analysis_limit: number };
    if (config.indicators) app.globalIndicatorsConfig = config.indicators as Record<string, number>;
    if (config.indicator_registry) app.indicatorRegistry = config.indicator_registry as import('../types').IndicatorMeta[];

    const pairConfigs = (config.instances || {}) as Record<string, { micro_term?: { candles: { duration_seconds: number; analysis_limit?: number }; indicators: Record<string, number> }; fast_term?: { candles: { duration_seconds: number; analysis_limit?: number }; indicators: Record<string, number> }; slow_term?: { candles: { duration_seconds: number; analysis_limit?: number }; indicators: Record<string, number> }; macro_term?: { candles: { duration_seconds: number; analysis_limit?: number }; indicators: Record<string, number> }; automation?: { enabled?: boolean; interval_seconds?: number }; operational_mode?: string }>;
    const symbols: string[] = (config.symbols as string[]) || ['BTC'];

    for (const item of symbols) {
        const baseSymbol = item.includes(':') ? item.split(':')[1] : item;
        const pairKey = app.pairKeyFor(baseSymbol);
        const existing = !!app.instancesMap[pairKey];
        if (!existing) {
            app.initInstance(baseSymbol);
        }

        const specific = pairConfigs[pairKey];
        const targetState = app.instancesMap[pairKey];

        function advancedIndicators(ind: Record<string, unknown>) {
            return {
                bbwpLookbackVal: (ind.bbwp_lookback as number) ?? 252,
                bbwpPeriodVal: (ind.bbwp_period as number) ?? 20,
                stochKPeriodVal: (ind.stoch_k_period as number) ?? 18,
                stochDPeriodVal: (ind.stoch_d_period as number) ?? 5,
                stochSPeriodVal: (ind.stoch_s_period as number) ?? 9,
                chandemoPeriodVal: (ind.chandemo_period as number) ?? 12,
                supertrendPeriodVal: (ind.supertrend_period as number) ?? 10,
                supertrendMultiplierVal: (ind.supertrend_multiplier as number) ?? 3.0,
                keltnerEmaPeriodVal: (ind.keltner_ema_period as number) ?? 20,
                keltnerAtrPeriodVal: (ind.keltner_atr_period as number) ?? 10,
                keltnerMultiplierVal: (ind.keltner_multiplier as number) ?? 2.0,
                donchianPeriodVal: (ind.donchian_period as number) ?? 20,
                obvSmoothingVal: (ind.obv_smoothing as number) ?? 20,
                cmfPeriodVal: (ind.cmf_period as number) ?? 20,
                mfiPeriodVal: (ind.mfi_period as number) ?? 14,
                hvPeriodVal: (ind.hv_period as number) ?? 20,
                aroonPeriodVal: (ind.aroon_period as number) ?? 25,
                chopPeriodVal: (ind.chop_period as number) ?? 14,
                linregPeriodVal: (ind.linreg_period as number) ?? 20,
                zscorePeriodVal: (ind.zscore_period as number) ?? 20,
                macdExtremeHighVal: (ind.macd_extreme_high_threshold as number) ?? 1000,
                macdExtremeLowVal: (ind.macd_extreme_low_threshold as number) ?? -1000,
                macdContractionVal: (ind.macd_histogram_contraction_threshold as number) ?? 0.30,
                adxTrendThresholdVal: (ind.adx_trend_threshold as number) ?? 20,
                adxExhaustionThresholdVal: (ind.adx_exhaustion_threshold as number) ?? 40,
                adxSlopeLookbackVal: (ind.adx_slope_lookback as number) ?? 3,
                squeezeMinDurationVal: (ind.squeeze_min_duration as number) ?? 5,
                squeezeBbPeriodVal: (ind.squeeze_bb_period as number) ?? 20,
                squeezeBbStdDevVal: (ind.squeeze_bb_std_dev as number) ?? 2.0,
                squeezeKcPeriodVal: (ind.squeeze_kc_period as number) ?? 20,
                squeezeKcAtrMultVal: (ind.squeeze_kc_atr_multiplier as number) ?? 1.5,
                atrMultiplierVal: (ind.atr_multiplier_coefficient as number) ?? 2.0,
                atrTargetRRVal: (ind.atr_target_rr_ratio as number) ?? 2.5,
                volumeAvgPeriodVal: (ind.volume_average_period as number) ?? 20,
                rvolInstitutionalVal: (ind.rvol_threshold_institutional as number) ?? 1.5,
                rvolClimaxVal: (ind.rvol_threshold_climax as number) ?? 3.0,
            };
        }

        if (specific && targetState) {
            if (specific.micro_term) {
                targetState.microTerm.barDurationSec = specific.micro_term.candles.duration_seconds;
                Object.assign(targetState.microTerm, {
                    emaFastVal: specific.micro_term.indicators.ema_fast,
                    emaMediumVal: specific.micro_term.indicators.ema_medium,
                    emaSlowVal: specific.micro_term.indicators.ema_slow,
                    emaLongVal: specific.micro_term.indicators.ema_long,
                    rsiPeriodVal: specific.micro_term.indicators.rsi_period,
                    macdFastVal: specific.micro_term.indicators.macd_fast,
                    macdSlowVal: specific.micro_term.indicators.macd_slow,
                    macdSignalVal: specific.micro_term.indicators.macd_signal,
                    adxPeriodVal: specific.micro_term.indicators.adx_period,
                    atrPeriodVal: specific.micro_term.indicators.atr_period,
                    squeezePeriodVal: specific.micro_term.indicators.squeeze_period,
                    analysisLimit: specific.micro_term.candles.analysis_limit ?? 100,
                    ...advancedIndicators(specific.micro_term.indicators as unknown as Record<string, unknown>),
                });
            }
            if (specific.fast_term) {
                targetState.fastTerm.barDurationSec = specific.fast_term.candles.duration_seconds;
                Object.assign(targetState.fastTerm, {
                    emaFastVal: specific.fast_term.indicators.ema_fast,
                    emaMediumVal: specific.fast_term.indicators.ema_medium,
                    emaSlowVal: specific.fast_term.indicators.ema_slow,
                    emaLongVal: specific.fast_term.indicators.ema_long,
                    rsiPeriodVal: specific.fast_term.indicators.rsi_period,
                    macdFastVal: specific.fast_term.indicators.macd_fast,
                    macdSlowVal: specific.fast_term.indicators.macd_slow,
                    macdSignalVal: specific.fast_term.indicators.macd_signal,
                    adxPeriodVal: specific.fast_term.indicators.adx_period,
                    atrPeriodVal: specific.fast_term.indicators.atr_period,
                    squeezePeriodVal: specific.fast_term.indicators.squeeze_period,
                    analysisLimit: specific.fast_term.candles.analysis_limit ?? 100,
                    ...advancedIndicators(specific.fast_term.indicators as unknown as Record<string, unknown>),
                });
            }
            if (specific.slow_term) {
                targetState.slowTerm.barDurationSec = specific.slow_term.candles.duration_seconds;
                Object.assign(targetState.slowTerm, {
                    emaFastVal: specific.slow_term.indicators.ema_fast,
                    emaMediumVal: specific.slow_term.indicators.ema_medium,
                    emaSlowVal: specific.slow_term.indicators.ema_slow,
                    emaLongVal: specific.slow_term.indicators.ema_long,
                    rsiPeriodVal: specific.slow_term.indicators.rsi_period,
                    macdFastVal: specific.slow_term.indicators.macd_fast,
                    macdSlowVal: specific.slow_term.indicators.macd_slow,
                    macdSignalVal: specific.slow_term.indicators.macd_signal,
                    adxPeriodVal: specific.slow_term.indicators.adx_period,
                    atrPeriodVal: specific.slow_term.indicators.atr_period,
                    squeezePeriodVal: specific.slow_term.indicators.squeeze_period,
                    analysisLimit: specific.slow_term.candles.analysis_limit ?? 100,
                    ...advancedIndicators(specific.slow_term.indicators as unknown as Record<string, unknown>),
                });
            }
            if (specific.macro_term) {
                targetState.macroTerm.barDurationSec = specific.macro_term.candles.duration_seconds;
                Object.assign(targetState.macroTerm, {
                    emaFastVal: specific.macro_term.indicators.ema_fast,
                    emaMediumVal: specific.macro_term.indicators.ema_medium,
                    emaSlowVal: specific.macro_term.indicators.ema_slow,
                    emaLongVal: specific.macro_term.indicators.ema_long,
                    rsiPeriodVal: specific.macro_term.indicators.rsi_period,
                    macdFastVal: specific.macro_term.indicators.macd_fast,
                    macdSlowVal: specific.macro_term.indicators.macd_slow,
                    macdSignalVal: specific.macro_term.indicators.macd_signal,
                    adxPeriodVal: specific.macro_term.indicators.adx_period,
                    atrPeriodVal: specific.macro_term.indicators.atr_period,
                    squeezePeriodVal: specific.macro_term.indicators.squeeze_period,
                    analysisLimit: specific.macro_term.candles.analysis_limit ?? 100,
                    ...advancedIndicators(specific.macro_term.indicators as unknown as Record<string, unknown>),
                });
            }
        }
    }

    const firstSymbol = symbols.length > 0
        ? (symbols[0].includes(':') ? symbols[0].split(':')[1] : symbols[0])
        : '';

    return { firstSymbol };
}

// ─── Apply settings API ────────────────────────────────────────────────────

export interface ApplySettingsBody { [key: string]: unknown }

/** Extract a human-friendly error message from a failed Response body. */
export async function readErrorMessage(res: Response, fallback: string): Promise<string> {
    try {
        const ct = res.headers.get('content-type') || '';
        if (ct.includes('application/json')) {
            const data = await res.json();
            return (data && (data.error || data.message)) || fallback;
        }
        const text = await res.text();
        return text.trim() || fallback;
    } catch {
        return fallback;
    }
}

/** Create an instance; returns success plus a friendly error message on failure. */
export async function createInstance(base: string, quote: string): Promise<{ ok: boolean; error?: string }> {
    try {
        const res = await fetch('/api/instances', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ base, quote }),
        });
        if (res.ok) return { ok: true };
        return { ok: false, error: await readErrorMessage(res, 'Failed to add instance.') };
    } catch (e: any) {
        return { ok: false, error: e?.message || 'Network error. Please try again.' };
    }
}

export async function postInstanceCreation(baseSymbol: string, quote: string): Promise<boolean> {
    const res = await fetch('/api/instances', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ base: baseSymbol, quote }),
    });
    return res.ok;
}

export async function postInstanceConfig(pairKey: string, body: ApplySettingsBody): Promise<boolean> {
    const res = await fetch(`/api/instances/${encodeURIComponent(pairKey)}/config`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
    });
    return res.ok;
}

/** Syncs draft state from an existing pair. Pure function — no side effects. */
export function readDraftFromPair(pair: InstanceState): {
    symbol: string; exchange: string; durationValue: number; durationUnit: 'seconds' | 'minutes' | 'hours';
    emaFast: number; emaMedium: number; emaSlow: number; emaLong: number;
    rsiPeriod: number; macdFast: number; macdSlow: number; macdSignal: number;
    adxPeriod: number; atrPeriod: number; squeezePeriod: number;
    analysisLimit: number;
    showEmas: boolean; showBb: boolean; showVwap: boolean; showVolume: boolean;
    showAdx: boolean; showAtr: boolean; showRsi: boolean; showMacd: boolean;
    showSqueeze: boolean; showBbwp: boolean; showFib: boolean; showRvol: boolean;
    automationEnabled: boolean; automationIntervalValue: number;
    automationIntervalUnit: 'seconds' | 'minutes' | 'hours';
    slowInterval: number; normalInterval: number; fastInterval: number;
} {
    const sec = pair.microTerm.barDurationSec;
    let durationValue: number, durationUnit: 'seconds' | 'minutes' | 'hours';
    if (sec % 3600 === 0) { durationValue = sec / 3600; durationUnit = 'hours'; }
    else if (sec % 60 === 0) { durationValue = sec / 60; durationUnit = 'minutes'; }
    else { durationValue = sec; durationUnit = 'seconds'; }

    const autoSec = pair.automationIntervalUnit === 'hours' ? pair.automationIntervalValue * 3600
        : pair.automationIntervalUnit === 'minutes' ? pair.automationIntervalValue * 60 : pair.automationIntervalValue;
    let autoValue: number, autoUnit: 'seconds' | 'minutes' | 'hours';
    if (autoSec % 3600 === 0) { autoValue = autoSec / 3600; autoUnit = 'hours'; }
    else if (autoSec % 60 === 0) { autoValue = autoSec / 60; autoUnit = 'minutes'; }
    else { autoValue = autoSec; autoUnit = 'seconds'; }

    return {
        symbol: pair.symbol,
        exchange: pair.exchange,
        durationValue, durationUnit,
        emaFast: pair.microTerm.emaFastVal,
        emaMedium: pair.microTerm.emaMediumVal,
        emaSlow: pair.microTerm.emaSlowVal,
        emaLong: pair.microTerm.emaLongVal,
        rsiPeriod: pair.microTerm.rsiPeriodVal,
        macdFast: pair.microTerm.macdFastVal,
        macdSlow: pair.microTerm.macdSlowVal,
        macdSignal: pair.microTerm.macdSignalVal,
        adxPeriod: pair.microTerm.adxPeriodVal,
        atrPeriod: pair.microTerm.atrPeriodVal,
        squeezePeriod: pair.microTerm.squeezePeriodVal,
        analysisLimit: pair.microTerm.analysisLimit,
        showEmas: pair.microTerm.showEmas,
        showBb: pair.microTerm.showBb,
        showVwap: pair.microTerm.showVwap,
        showVolume: pair.microTerm.showVolume,
        showAdx: pair.microTerm.showAdx,
        showAtr: pair.microTerm.showAtr,
        showRsi: pair.microTerm.showRsi,
        showMacd: pair.microTerm.showMacd,
        showSqueeze: pair.microTerm.showSqueeze,
    showBbwp: pair.microTerm.showBbwp,
    showFib: pair.microTerm.showFib,
    showRvol: pair.microTerm.showRvol,
    automationEnabled: pair.automationEnabled,
        automationIntervalValue: autoValue,
        automationIntervalUnit: autoUnit,
        slowInterval: pair.slowIntervalSecs || 3600,
        normalInterval: pair.normalIntervalSecs || 900,
        fastInterval: pair.fastIntervalSecs || 300,
    };
}

