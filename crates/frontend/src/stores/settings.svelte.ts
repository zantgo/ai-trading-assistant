export class SettingsStore {
    apiKeyConfigured = $state(true);
    rulesContent = $state('');

    /** Authoritative indicator manifest fetched from /api/config (source of truth). */
    indicatorRegistry = $state<import('../types').IndicatorMeta[]>([]);

    globalCandlesConfig = $state({ duration_seconds: 60, analysis_limit: 100 });
    globalIndicatorsConfig = $state({
        ema_fast: 10, ema_medium: 50, ema_slow: 100, ema_long: 200,
        rsi_period: 14, macd_fast: 12, macd_slow: 26, macd_signal: 9,
        adx_period: 14, atr_period: 14, squeeze_period: 20,
        stoch_k_period: 18, stoch_d_period: 5, stoch_s_period: 9, chandemo_period: 12,
        supertrend_period: 10, supertrend_multiplier: 3.0,
        keltner_ema_period: 20, keltner_atr_period: 10, keltner_multiplier: 2.0,
        donchian_period: 20, obv_smoothing: 20, cmf_period: 20, mfi_period: 14, hv_period: 20,
        aroon_period: 25, chop_period: 14, linreg_period: 20, zscore_period: 20,
        williams_r_period: 14, hull_ma_period: 16, stddev_channel_period: 20, force_index_smoothing: 13,
    });

    emaFastLabel = $state('EMA-10'); emaMediumLabel = $state('EMA-50');
    emaSlowLabel = $state('EMA-100'); emaLongLabel = $state('EMA-200');
    rsiLabel = $state('RSI (14)'); adxLabel = $state('ADX (14)');
    atrLabel = $state('ATR (14)'); macdLabel = $state('MACD (12,26,9)');

    costPriceInput = $state(0.27);
    costPriceOutput = $state(1.10);
    costIntervalSecs = $state(900);
    costRunsPerDay = $state(0);
    costTokensPerRunInput = $state(0);
    costTokensPerRunOutput = $state(0);
    costDailyProjected = $state(0);
    costWeeklyProjected = $state(0);
    costMonthlyProjected = $state(0);
    costActualInputTokens = $state(0);
    costActualOutputTokens = $state(0);
    costActualTotal = $state(0);
    costLoading = $state(false);

    regimeWeightMultipliers = $state<Record<string, Record<string, number>>>({});

    async fetchScoringWeights() {
        try {
            const res = await fetch('/api/config/scoring-weights');
            if (res.ok) {
                const data = await res.json();
                this.regimeWeightMultipliers = data.regime_weight_multipliers ?? {};
            }
        } catch (_) {}
    }

    async fetchCostEstimate(pairKey: string) {
        this.costLoading = true;
        try {
            const res = await fetch(`/api/cost-estimate?pair_key=${encodeURIComponent(pairKey)}`);
            if (res.ok) {
                const data = await res.json();
                this.costPriceInput = data.price_per_1m_input_tokens ?? 0.27;
                this.costPriceOutput = data.price_per_1m_output_tokens ?? 1.10;
                this.costIntervalSecs = data.interval_seconds ?? 900;
                this.costRunsPerDay = data.runs_per_day ?? 0;
                this.costTokensPerRunInput = data.input_tokens_per_run ?? 0;
                this.costTokensPerRunOutput = data.output_tokens_per_run ?? 0;
                this.costDailyProjected = data.projected_daily_cost ?? 0;
                this.costWeeklyProjected = data.projected_weekly_cost ?? 0;
                this.costMonthlyProjected = data.projected_monthly_cost ?? 0;
                this.costActualInputTokens = data.actual_input_tokens_used ?? 0;
                this.costActualOutputTokens = data.actual_output_tokens_used ?? 0;
                this.costActualTotal = data.actual_total_cost ?? 0;
            }
        } catch (_) {} finally { this.costLoading = false; }
    }
}
