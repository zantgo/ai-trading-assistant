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

export async function saveApiKeyCall(apiKey: string): Promise<{ ok: boolean; error?: string }> {
    const res = await fetch('/api/config/key', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ api_key: apiKey }),
    });
    return { ok: res.ok, error: res.ok ? undefined : 'Rejected by Server' };
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

export async function saveCostConfigCall(inputPrice: number, outputPrice: number): Promise<boolean> {
    const res = await fetch('/api/config');
    const config = await res.json();
    config.costs = {
        price_per_1m_input_tokens: inputPrice,
        price_per_1m_output_tokens: outputPrice,
    };
    const saveRes = await fetch('/api/config', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(config),
    });
    return saveRes.ok;
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

export async function fetchAssistantHistoryFromServer(): Promise<{ records: Record<string, unknown>[]; latest_close: string }> {
    const res = await fetch('/api/assistant-records');
    const data = await res.json();
    return { records: data.records || [], latest_close: data.latest_close || '0' };
}

// ─── Config application logic ──────────────────────────────────────────────

export interface ApplyConfigResult {
    costInputPrice: number;
    costOutputPrice: number;
    firstSymbol: string;
}

/** Applies config from the server to the AppStore. Returns data needed for component-local state. */
export function applyConfigToStore(app: AppStore, config: Record<string, unknown>): ApplyConfigResult {
    app.apiKeyConfigured = (config.api_key_configured as boolean) ?? true;

    if (config.candles) app.globalCandlesConfig = config.candles as { duration_seconds: number; analysis_limit: number };
    if (config.indicators) app.globalIndicatorsConfig = config.indicators as { ema_fast: number; ema_medium: number; ema_slow: number; ema_long: number; rsi_period: number; macd_fast: number; macd_slow: number; macd_signal: number; adx_period: number; atr_period: number; squeeze_period: number };

    const costs = config.costs as Record<string, number> | undefined;
    const costInputPrice = costs?.price_per_1m_input_tokens ?? 0.27;
    const costOutputPrice = costs?.price_per_1m_output_tokens ?? 1.10;

    const pairConfigs = (config.instances || {}) as Record<string, { micro_term?: { candles: { duration_seconds: number; analysis_limit?: number }; indicators: Record<string, number> }; short_term?: { candles: { duration_seconds: number; analysis_limit?: number }; indicators: Record<string, number> }; medium_term?: { candles: { duration_seconds: number; analysis_limit?: number }; indicators: Record<string, number> }; large_term?: { candles: { duration_seconds: number; analysis_limit?: number }; indicators: Record<string, number> }; automation?: { enabled?: boolean; interval_seconds?: number } }>;
    const symbols: string[] = (config.symbols as string[]) || ['BTC'];

    for (const item of symbols) {
        const baseSymbol = item.includes(':') ? item.split(':')[1] : item;
        app.initInstance(baseSymbol);

        const pairKey = `${baseSymbol}-USDT`;
        const specific = pairConfigs[pairKey];
        const targetState = app.instancesMap[pairKey];

        function advancedIndicators(ind: Record<string, unknown>) {
            return {
                bbwpLookbackVal: (ind.bbwp_lookback as number) ?? 252,
                bbwpPeriodVal: (ind.bbwp_period as number) ?? 20,
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
            if (specific.short_term) {
                targetState.smallTerm.barDurationSec = specific.short_term.candles.duration_seconds;
                Object.assign(targetState.smallTerm, {
                    emaFastVal: specific.short_term.indicators.ema_fast,
                    emaMediumVal: specific.short_term.indicators.ema_medium,
                    emaSlowVal: specific.short_term.indicators.ema_slow,
                    emaLongVal: specific.short_term.indicators.ema_long,
                    rsiPeriodVal: specific.short_term.indicators.rsi_period,
                    macdFastVal: specific.short_term.indicators.macd_fast,
                    macdSlowVal: specific.short_term.indicators.macd_slow,
                    macdSignalVal: specific.short_term.indicators.macd_signal,
                    adxPeriodVal: specific.short_term.indicators.adx_period,
                    atrPeriodVal: specific.short_term.indicators.atr_period,
                    squeezePeriodVal: specific.short_term.indicators.squeeze_period,
                    analysisLimit: specific.short_term.candles.analysis_limit ?? 100,
                    ...advancedIndicators(specific.short_term.indicators as unknown as Record<string, unknown>),
                });
            }
            if (specific.medium_term) {
                targetState.mediumTerm.barDurationSec = specific.medium_term.candles.duration_seconds;
                Object.assign(targetState.mediumTerm, {
                    emaFastVal: specific.medium_term.indicators.ema_fast,
                    emaMediumVal: specific.medium_term.indicators.ema_medium,
                    emaSlowVal: specific.medium_term.indicators.ema_slow,
                    emaLongVal: specific.medium_term.indicators.ema_long,
                    rsiPeriodVal: specific.medium_term.indicators.rsi_period,
                    macdFastVal: specific.medium_term.indicators.macd_fast,
                    macdSlowVal: specific.medium_term.indicators.macd_slow,
                    macdSignalVal: specific.medium_term.indicators.macd_signal,
                    adxPeriodVal: specific.medium_term.indicators.adx_period,
                    atrPeriodVal: specific.medium_term.indicators.atr_period,
                    squeezePeriodVal: specific.medium_term.indicators.squeeze_period,
                    analysisLimit: specific.medium_term.candles.analysis_limit ?? 100,
                    ...advancedIndicators(specific.medium_term.indicators as unknown as Record<string, unknown>),
                });
            }
            if (specific.large_term) {
                targetState.largeTerm.barDurationSec = specific.large_term.candles.duration_seconds;
                Object.assign(targetState.largeTerm, {
                    emaFastVal: specific.large_term.indicators.ema_fast,
                    emaMediumVal: specific.large_term.indicators.ema_medium,
                    emaSlowVal: specific.large_term.indicators.ema_slow,
                    emaLongVal: specific.large_term.indicators.ema_long,
                    rsiPeriodVal: specific.large_term.indicators.rsi_period,
                    macdFastVal: specific.large_term.indicators.macd_fast,
                    macdSlowVal: specific.large_term.indicators.macd_slow,
                    macdSignalVal: specific.large_term.indicators.macd_signal,
                    adxPeriodVal: specific.large_term.indicators.adx_period,
                    atrPeriodVal: specific.large_term.indicators.atr_period,
                    squeezePeriodVal: specific.large_term.indicators.squeeze_period,
                    analysisLimit: specific.large_term.candles.analysis_limit ?? 100,
                    ...advancedIndicators(specific.large_term.indicators as unknown as Record<string, unknown>),
                });
            }
            if (specific.automation) {
                targetState.automationEnabled = specific.automation.enabled ?? false;
                const autoSec = specific.automation.interval_seconds ?? 900;
                if (autoSec % 3600 === 0) { targetState.automationIntervalValue = autoSec / 3600; targetState.automationIntervalUnit = 'hours'; }
                else if (autoSec % 60 === 0) { targetState.automationIntervalValue = autoSec / 60; targetState.automationIntervalUnit = 'minutes'; }
                else { targetState.automationIntervalValue = autoSec; targetState.automationIntervalUnit = 'seconds'; }
                targetState.nextEvaluationIn = targetState.automationEnabled ? formatIntervalRemaining(autoSec) : '--';
            }
        }
    }

    const firstSymbol = symbols.length > 0
        ? (symbols[0].includes(':') ? symbols[0].split(':')[1] : symbols[0])
        : '';

    return { costInputPrice, costOutputPrice, firstSymbol };
}

// ─── Apply settings API ────────────────────────────────────────────────────

export interface ApplySettingsBody { [key: string]: unknown }

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
    tpLevels: number; slLevels: number;
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
        tpLevels: pair.tpLevels || 1,
        slLevels: pair.slLevels || 1,
    };
}
