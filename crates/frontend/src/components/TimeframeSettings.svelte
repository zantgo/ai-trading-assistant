<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import type { InstanceState, TimeframeTelemetry } from '../types';
    import styles from './TimeframeSettings.module.css';

    let { pair, tabKey, onApplied }: { pair: InstanceState; tabKey: string; onApplied?: () => void } = $props();
    const app = useAppStore();

    const TIMEFRAME_OPTIONS: { label: string; seconds: number }[] = [
        { label: '1 sec', seconds: 1 },
        { label: '5 sec', seconds: 5 },
        { label: '15 sec', seconds: 15 },
        { label: '30 sec', seconds: 30 },
        { label: '1 min', seconds: 60 },
        { label: '3 min', seconds: 180 },
        { label: '5 min', seconds: 300 },
        { label: '15 min', seconds: 900 },
        { label: '30 min', seconds: 1800 },
        { label: '1 h', seconds: 3600 },
        { label: '2 h', seconds: 7200 },
        { label: '4 h', seconds: 14400 },
        { label: '8 h', seconds: 28800 },
        { label: '12 h', seconds: 43200 },
        { label: '1 day', seconds: 86400 },
    ];

    interface TermDraft {
        durationSeconds: number;
        emaFast: number; emaMedium: number; emaSlow: number; emaLong: number;
        rsiPeriod: number;
        macdFast: number; macdSlow: number; macdSignal: number;
        adxPeriod: number; atrPeriod: number; squeezePeriod: number;
        bbwpPeriod: number; bbwpLookback: number;
        macdExtremeHigh: number; macdExtremeLow: number; macdContraction: number;
        adxTrendThreshold: number; adxExhaustionThreshold: number; adxSlopeLookback: number;
        squeezeMinDuration: number; squeezeBbPeriod: number; squeezeBbStdDev: number;
        squeezeKcPeriod: number; squeezeKcAtrMult: number;
        atrMultiplier: number; atrTargetRR: number;
        volumeAvgPeriod: number; rvolInstitutional: number; rvolClimax: number;
        analysisLimit: number;
    }

    function defaultTermDraft(): TermDraft {
        return {
            durationSeconds: 60,
            emaFast: 10, emaMedium: 50, emaSlow: 100, emaLong: 200,
            rsiPeriod: 14,
            macdFast: 12, macdSlow: 26, macdSignal: 9,
            adxPeriod: 14, atrPeriod: 14, squeezePeriod: 20,
            bbwpPeriod: 20, bbwpLookback: 252,
            macdExtremeHigh: 1000, macdExtremeLow: -1000, macdContraction: 0.30,
            adxTrendThreshold: 20, adxExhaustionThreshold: 40, adxSlopeLookback: 3,
            squeezeMinDuration: 5, squeezeBbPeriod: 20, squeezeBbStdDev: 2.0,
            squeezeKcPeriod: 20, squeezeKcAtrMult: 1.5,
            atrMultiplier: 2.0, atrTargetRR: 2.5,
            volumeAvgPeriod: 20, rvolInstitutional: 1.5, rvolClimax: 3.0,
            analysisLimit: 100,
        };
    }

    function readTermFromTelemetry(tf: TimeframeTelemetry): TermDraft {
        return {
            durationSeconds: tf.barDurationSec,
            emaFast: tf.emaFastVal, emaMedium: tf.emaMediumVal, emaSlow: tf.emaSlowVal, emaLong: tf.emaLongVal,
            rsiPeriod: tf.rsiPeriodVal,
            macdFast: tf.macdFastVal, macdSlow: tf.macdSlowVal, macdSignal: tf.macdSignalVal,
            adxPeriod: tf.adxPeriodVal, atrPeriod: tf.atrPeriodVal, squeezePeriod: tf.squeezePeriodVal,
            bbwpPeriod: tf.bbwpPeriodVal, bbwpLookback: tf.bbwpLookbackVal,
            macdExtremeHigh: tf.macdExtremeHighVal, macdExtremeLow: tf.macdExtremeLowVal, macdContraction: tf.macdContractionVal,
            adxTrendThreshold: tf.adxTrendThresholdVal, adxExhaustionThreshold: tf.adxExhaustionThresholdVal, adxSlopeLookback: tf.adxSlopeLookbackVal,
            squeezeMinDuration: tf.squeezeMinDurationVal, squeezeBbPeriod: tf.squeezeBbPeriodVal, squeezeBbStdDev: tf.squeezeBbStdDevVal,
            squeezeKcPeriod: tf.squeezeKcPeriodVal, squeezeKcAtrMult: tf.squeezeKcAtrMultVal,
            atrMultiplier: tf.atrMultiplierVal, atrTargetRR: tf.atrTargetRRVal,
            volumeAvgPeriod: tf.volumeAvgPeriodVal, rvolInstitutional: tf.rvolInstitutionalVal, rvolClimax: tf.rvolClimaxVal,
            analysisLimit: tf.analysisLimit,
        };
    }

    let draft = $state({
        micro: defaultTermDraft(),
        small: defaultTermDraft(),
        medium: defaultTermDraft(),
        large: defaultTermDraft(),
    });

    let saveStatus = $state<'idle' | 'saving' | 'success' | 'error'>('idle');

    $effect(() => {
        draft.micro = readTermFromTelemetry(pair.microTerm);
        draft.small = readTermFromTelemetry(pair.smallTerm);
        draft.medium = readTermFromTelemetry(pair.mediumTerm);
        draft.large = readTermFromTelemetry(pair.largeTerm);
    });

    function selectedOption(seconds: number): number {
        const found = TIMEFRAME_OPTIONS.find(o => o.seconds === seconds);
        return found ? found.seconds : -1;
    }

    function durationLabel(seconds: number): string {
        const found = TIMEFRAME_OPTIONS.find(o => o.seconds === seconds);
        return found ? found.label : `${seconds}s`;
    }

    function buildIndicators(term: TermDraft): Record<string, number> {
        return {
            ema_fast: term.emaFast, ema_medium: term.emaMedium, ema_slow: term.emaSlow, ema_long: term.emaLong,
            rsi_period: term.rsiPeriod,
            macd_fast: term.macdFast, macd_slow: term.macdSlow, macd_signal: term.macdSignal,
            adx_period: term.adxPeriod, atr_period: term.atrPeriod, squeeze_period: term.squeezePeriod,
            bbwp_period: term.bbwpPeriod, bbwp_lookback: term.bbwpLookback,
            macd_extreme_high_threshold: term.macdExtremeHigh, macd_extreme_low_threshold: term.macdExtremeLow,
            macd_histogram_contraction_threshold: term.macdContraction,
            adx_trend_threshold: term.adxTrendThreshold, adx_exhaustion_threshold: term.adxExhaustionThreshold,
            adx_slope_lookback: term.adxSlopeLookback,
            squeeze_min_duration: term.squeezeMinDuration, squeeze_bb_period: term.squeezeBbPeriod,
            squeeze_bb_std_dev: term.squeezeBbStdDev, squeeze_kc_period: term.squeezeKcPeriod,
            squeeze_kc_atr_multiplier: term.squeezeKcAtrMult,
            atr_multiplier_coefficient: term.atrMultiplier, atr_target_rr_ratio: term.atrTargetRR,
            volume_average_period: term.volumeAvgPeriod,
            rvol_threshold_institutional: term.rvolInstitutional, rvol_threshold_climax: term.rvolClimax,
        };
    }

    function applyTermToTelemetry(term: TermDraft, tf: TimeframeTelemetry) {
        tf.barDurationSec = term.durationSeconds;
        tf.emaFastVal = term.emaFast; tf.emaMediumVal = term.emaMedium;
        tf.emaSlowVal = term.emaSlow; tf.emaLongVal = term.emaLong;
        tf.rsiPeriodVal = term.rsiPeriod;
        tf.macdFastVal = term.macdFast; tf.macdSlowVal = term.macdSlow; tf.macdSignalVal = term.macdSignal;
        tf.adxPeriodVal = term.adxPeriod; tf.atrPeriodVal = term.atrPeriod; tf.squeezePeriodVal = term.squeezePeriod;
        tf.bbwpPeriodVal = term.bbwpPeriod; tf.bbwpLookbackVal = term.bbwpLookback;
        tf.macdExtremeHighVal = term.macdExtremeHigh; tf.macdExtremeLowVal = term.macdExtremeLow;
        tf.macdContractionVal = term.macdContraction;
        tf.adxTrendThresholdVal = term.adxTrendThreshold; tf.adxExhaustionThresholdVal = term.adxExhaustionThreshold;
        tf.adxSlopeLookbackVal = term.adxSlopeLookback;
        tf.squeezeMinDurationVal = term.squeezeMinDuration; tf.squeezeBbPeriodVal = term.squeezeBbPeriod;
        tf.squeezeBbStdDevVal = term.squeezeBbStdDev; tf.squeezeKcPeriodVal = term.squeezeKcPeriod;
        tf.squeezeKcAtrMultVal = term.squeezeKcAtrMult;
        tf.atrMultiplierVal = term.atrMultiplier; tf.atrTargetRRVal = term.atrTargetRR;
        tf.volumeAvgPeriodVal = term.volumeAvgPeriod;
        tf.rvolInstitutionalVal = term.rvolInstitutional; tf.rvolClimaxVal = term.rvolClimax;
        tf.analysisLimit = term.analysisLimit;
        tf.latestSnapshot = null;
        tf.priceText = '--';
        tf.vwapText = '--';
    }

    function fieldId(term: string, label: string): string {
        const slug = label.toLowerCase()
            .replace(/%/g, 'pct')
            .replace(/[:\s]+/g, '-')
            .replace(/[^a-z0-9-]/g, '');
        return `tf-${term}-${slug}`;
    }

    async function applySettings() {
        const durations = [
            draft.micro.durationSeconds,
            draft.small.durationSeconds,
            draft.medium.durationSeconds,
            draft.large.durationSeconds,
        ];
        const uniqueDurations = new Set(durations);
        if (uniqueDurations.size < 4) {
            alert('Each timeframe must have a unique duration. Duplicate durations are not allowed.');
            return;
        }

        const body = {
            micro_term: {
                candles: { duration_seconds: draft.micro.durationSeconds, analysis_limit: draft.micro.analysisLimit },
                indicators: buildIndicators(draft.micro),
            },
            short_term: {
                candles: { duration_seconds: draft.small.durationSeconds, analysis_limit: draft.small.analysisLimit },
                indicators: buildIndicators(draft.small),
            },
            medium_term: {
                candles: { duration_seconds: draft.medium.durationSeconds, analysis_limit: draft.medium.analysisLimit },
                indicators: buildIndicators(draft.medium),
            },
            large_term: {
                candles: { duration_seconds: draft.large.durationSeconds, analysis_limit: draft.large.analysisLimit },
                indicators: buildIndicators(draft.large),
            },
            automation: {
                enabled: pair.automationEnabled,
                interval_seconds: pair.automationIntervalUnit === 'hours'
                    ? pair.automationIntervalValue * 3600
                    : pair.automationIntervalUnit === 'minutes'
                        ? pair.automationIntervalValue * 60
                        : pair.automationIntervalValue,
            },
        };

        saveStatus = 'saving';
        try {
            const res = await fetch(`/api/instances/${encodeURIComponent(tabKey)}/config`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(body),
            });
            if (res.ok) {
                applyTermToTelemetry(draft.micro, pair.microTerm);
                applyTermToTelemetry(draft.small, pair.smallTerm);
                applyTermToTelemetry(draft.medium, pair.mediumTerm);
                applyTermToTelemetry(draft.large, pair.largeTerm);
                onApplied?.();
                saveStatus = 'success';
                setTimeout(() => { saveStatus = 'idle'; pair.currentView = 'terminal'; }, 800);
            } else {
                saveStatus = 'error';
            }
        } catch (e) {
            console.error('Timeframe config save error:', e);
            saveStatus = 'error';
        }
    }
</script>

<div class="{styles.timeframeSettingsTab} animate-fade">
    <div class={styles.cardsGrid}>

        <div class={styles.termCard}>
            <h3 class={styles.cardTitle}>Micro Term</h3>
            <div class={styles.timeframeRow}>
                <select class={styles.tfSelect}
                    value={selectedOption(draft.micro.durationSeconds)}
                    onchange={(e) => { const v = parseInt(e.currentTarget.value); if (v > 0) draft.micro.durationSeconds = v; }}>
                    <option value={-1} disabled>Custom: {durationLabel(draft.micro.durationSeconds)}</option>
                    {#each TIMEFRAME_OPTIONS as opt}
                        <option value={opt.seconds}>{opt.label}</option>
                    {/each}
                </select>
            </div>
            <div class="{styles.indicatorInputsScroll} font-mono">
                <div class={styles.inputRow}><label for={fieldId('micro', 'EMA Fast')}>EMA Fast</label><input id={fieldId('micro', 'EMA Fast')} type="number" bind:value={draft.micro.emaFast} /></div>
                <div class={styles.inputRow}><label for={fieldId('micro', 'EMA Med')}>EMA Med</label><input id={fieldId('micro', 'EMA Med')} type="number" bind:value={draft.micro.emaMedium} /></div>
                <div class={styles.inputRow}><label for={fieldId('micro', 'EMA Slow')}>EMA Slow</label><input id={fieldId('micro', 'EMA Slow')} type="number" bind:value={draft.micro.emaSlow} /></div>
                <div class={styles.inputRow}><label for={fieldId('micro', 'EMA Long')}>EMA Long</label><input id={fieldId('micro', 'EMA Long')} type="number" bind:value={draft.micro.emaLong} /></div>
                <div class={styles.inputRow}><label for={fieldId('micro', 'RSI Window')}>RSI Window</label><input id={fieldId('micro', 'RSI Window')} type="number" bind:value={draft.micro.rsiPeriod} /></div>
                <div class={styles.inputRow}><label for={fieldId('micro', 'MACD Fast')}>MACD Fast</label><input id={fieldId('micro', 'MACD Fast')} type="number" bind:value={draft.micro.macdFast} /></div>
                <div class={styles.inputRow}><label for={fieldId('micro', 'MACD Slow')}>MACD Slow</label><input id={fieldId('micro', 'MACD Slow')} type="number" bind:value={draft.micro.macdSlow} /></div>
                <div class={styles.inputRow}><label for={fieldId('micro', 'MACD Signal')}>MACD Signal</label><input id={fieldId('micro', 'MACD Signal')} type="number" bind:value={draft.micro.macdSignal} /></div>
                <div class={styles.inputRow}><label for={fieldId('micro', 'ADX Period')}>ADX Period</label><input id={fieldId('micro', 'ADX Period')} type="number" bind:value={draft.micro.adxPeriod} /></div>
                <div class={styles.inputRow}><label for={fieldId('micro', 'ATR Period')}>ATR Period</label><input id={fieldId('micro', 'ATR Period')} type="number" bind:value={draft.micro.atrPeriod} /></div>
                <div class={styles.inputRow}><label for={fieldId('micro', 'Squeeze Wave')}>Squeeze Wave</label><input id={fieldId('micro', 'Squeeze Wave')} type="number" bind:value={draft.micro.squeezePeriod} /></div>
                <div class={styles.inputRow}><label for={fieldId('micro', 'BBWP Period')}>BBWP Period</label><input id={fieldId('micro', 'BBWP Period')} type="number" bind:value={draft.micro.bbwpPeriod} /></div>
                <div class={styles.inputRow}><label for={fieldId('micro', 'BBWP Lookback')}>BBWP Lookback</label><input id={fieldId('micro', 'BBWP Lookback')} type="number" bind:value={draft.micro.bbwpLookback} /></div>
                <hr class={styles.sectionDivider} />
                <div class={styles.inputRow}><label for={fieldId('micro', 'MACD Extr High')}>MACD Extr High</label><input id={fieldId('micro', 'MACD Extr High')} type="number" step="0.01" bind:value={draft.micro.macdExtremeHigh} /></div>
                <div class={styles.inputRow}><label for={fieldId('micro', 'MACD Extr Low')}>MACD Extr Low</label><input id={fieldId('micro', 'MACD Extr Low')} type="number" step="0.01" bind:value={draft.micro.macdExtremeLow} /></div>
                <div class={styles.inputRow}><label for={fieldId('micro', 'MACD Contr %')}>MACD Contr %</label><input id={fieldId('micro', 'MACD Contr %')} type="number" step="0.01" min="0.05" max="0.95" bind:value={draft.micro.macdContraction} /></div>
                <div class={styles.inputRow}><label for={fieldId('micro', 'ADX Trend Th')}>ADX Trend Th</label><input id={fieldId('micro', 'ADX Trend Th')} type="number" bind:value={draft.micro.adxTrendThreshold} /></div>
                <div class={styles.inputRow}><label for={fieldId('micro', 'ADX Exhaustion')}>ADX Exhaustion</label><input id={fieldId('micro', 'ADX Exhaustion')} type="number" bind:value={draft.micro.adxExhaustionThreshold} /></div>
                <div class={styles.inputRow}><label for={fieldId('micro', 'ADX Slope Lbk')}>ADX Slope Lbk</label><input id={fieldId('micro', 'ADX Slope Lbk')} type="number" bind:value={draft.micro.adxSlopeLookback} /></div>
                <hr class={styles.sectionDivider} />
                <div class={styles.inputRow}><label for={fieldId('micro', 'Sqz Min Dur')}>Sqz Min Dur</label><input id={fieldId('micro', 'Sqz Min Dur')} type="number" bind:value={draft.micro.squeezeMinDuration} /></div>
                <div class={styles.inputRow}><label for={fieldId('micro', 'Sqz BB Period')}>Sqz BB Period</label><input id={fieldId('micro', 'Sqz BB Period')} type="number" bind:value={draft.micro.squeezeBbPeriod} /></div>
                <div class={styles.inputRow}><label for={fieldId('micro', 'Sqz BB Std Dev')}>Sqz BB Std Dev</label><input id={fieldId('micro', 'Sqz BB Std Dev')} type="number" step="0.1" bind:value={draft.micro.squeezeBbStdDev} /></div>
                <div class={styles.inputRow}><label for={fieldId('micro', 'Sqz KC Period')}>Sqz KC Period</label><input id={fieldId('micro', 'Sqz KC Period')} type="number" bind:value={draft.micro.squeezeKcPeriod} /></div>
                <div class={styles.inputRow}><label for={fieldId('micro', 'Sqz KC ATR Mult')}>Sqz KC ATR Mult</label><input id={fieldId('micro', 'Sqz KC ATR Mult')} type="number" step="0.1" bind:value={draft.micro.squeezeKcAtrMult} /></div>
                <hr class={styles.sectionDivider} />
                <div class={styles.inputRow}><label for={fieldId('micro', 'ATR Mult')}>ATR Mult</label><input id={fieldId('micro', 'ATR Mult')} type="number" step="0.1" bind:value={draft.micro.atrMultiplier} /></div>
                <div class={styles.inputRow}><label for={fieldId('micro', 'Target R:R')}>Target R:R</label><input id={fieldId('micro', 'Target R:R')} type="number" step="0.1" bind:value={draft.micro.atrTargetRR} /></div>
                <div class={styles.inputRow}><label for={fieldId('micro', 'Vol Avg Period')}>Vol Avg Period</label><input id={fieldId('micro', 'Vol Avg Period')} type="number" bind:value={draft.micro.volumeAvgPeriod} /></div>
                <div class={styles.inputRow}><label for={fieldId('micro', 'RVOL Inst')}>RVOL Inst</label><input id={fieldId('micro', 'RVOL Inst')} type="number" step="0.1" bind:value={draft.micro.rvolInstitutional} /></div>
                <div class={styles.inputRow}><label for={fieldId('micro', 'RVOL Climax')}>RVOL Climax</label><input id={fieldId('micro', 'RVOL Climax')} type="number" step="0.1" bind:value={draft.micro.rvolClimax} /></div>
                <div class={styles.inputRow}><label for={fieldId('micro', 'Analysis Limit')}>Analysis Limit</label><input id={fieldId('micro', 'Analysis Limit')} type="number" min="10" max="500" step="5" bind:value={draft.micro.analysisLimit} /></div>
            </div>
        </div>

        <div class={styles.termCard}>
            <h3 class={styles.cardTitle}>Small Term</h3>
            <div class={styles.timeframeRow}>
                <select class={styles.tfSelect}
                    value={selectedOption(draft.small.durationSeconds)}
                    onchange={(e) => { const v = parseInt(e.currentTarget.value); if (v > 0) draft.small.durationSeconds = v; }}>
                    <option value={-1} disabled>Custom: {durationLabel(draft.small.durationSeconds)}</option>
                    {#each TIMEFRAME_OPTIONS as opt}
                        <option value={opt.seconds}>{opt.label}</option>
                    {/each}
                </select>
            </div>
            <div class="{styles.indicatorInputsScroll} font-mono">
                <div class={styles.inputRow}><label for={fieldId('small', 'EMA Fast')}>EMA Fast</label><input id={fieldId('small', 'EMA Fast')} type="number" bind:value={draft.small.emaFast} /></div>
                <div class={styles.inputRow}><label for={fieldId('small', 'EMA Med')}>EMA Med</label><input id={fieldId('small', 'EMA Med')} type="number" bind:value={draft.small.emaMedium} /></div>
                <div class={styles.inputRow}><label for={fieldId('small', 'EMA Slow')}>EMA Slow</label><input id={fieldId('small', 'EMA Slow')} type="number" bind:value={draft.small.emaSlow} /></div>
                <div class={styles.inputRow}><label for={fieldId('small', 'EMA Long')}>EMA Long</label><input id={fieldId('small', 'EMA Long')} type="number" bind:value={draft.small.emaLong} /></div>
                <div class={styles.inputRow}><label for={fieldId('small', 'RSI Window')}>RSI Window</label><input id={fieldId('small', 'RSI Window')} type="number" bind:value={draft.small.rsiPeriod} /></div>
                <div class={styles.inputRow}><label for={fieldId('small', 'MACD Fast')}>MACD Fast</label><input id={fieldId('small', 'MACD Fast')} type="number" bind:value={draft.small.macdFast} /></div>
                <div class={styles.inputRow}><label for={fieldId('small', 'MACD Slow')}>MACD Slow</label><input id={fieldId('small', 'MACD Slow')} type="number" bind:value={draft.small.macdSlow} /></div>
                <div class={styles.inputRow}><label for={fieldId('small', 'MACD Signal')}>MACD Signal</label><input id={fieldId('small', 'MACD Signal')} type="number" bind:value={draft.small.macdSignal} /></div>
                <div class={styles.inputRow}><label for={fieldId('small', 'ADX Period')}>ADX Period</label><input id={fieldId('small', 'ADX Period')} type="number" bind:value={draft.small.adxPeriod} /></div>
                <div class={styles.inputRow}><label for={fieldId('small', 'ATR Period')}>ATR Period</label><input id={fieldId('small', 'ATR Period')} type="number" bind:value={draft.small.atrPeriod} /></div>
                <div class={styles.inputRow}><label for={fieldId('small', 'Squeeze Wave')}>Squeeze Wave</label><input id={fieldId('small', 'Squeeze Wave')} type="number" bind:value={draft.small.squeezePeriod} /></div>
                <div class={styles.inputRow}><label for={fieldId('small', 'BBWP Period')}>BBWP Period</label><input id={fieldId('small', 'BBWP Period')} type="number" bind:value={draft.small.bbwpPeriod} /></div>
                <div class={styles.inputRow}><label for={fieldId('small', 'BBWP Lookback')}>BBWP Lookback</label><input id={fieldId('small', 'BBWP Lookback')} type="number" bind:value={draft.small.bbwpLookback} /></div>
                <hr class={styles.sectionDivider} />
                <div class={styles.inputRow}><label for={fieldId('small', 'MACD Extr High')}>MACD Extr High</label><input id={fieldId('small', 'MACD Extr High')} type="number" step="0.01" bind:value={draft.small.macdExtremeHigh} /></div>
                <div class={styles.inputRow}><label for={fieldId('small', 'MACD Extr Low')}>MACD Extr Low</label><input id={fieldId('small', 'MACD Extr Low')} type="number" step="0.01" bind:value={draft.small.macdExtremeLow} /></div>
                <div class={styles.inputRow}><label for={fieldId('small', 'MACD Contr %')}>MACD Contr %</label><input id={fieldId('small', 'MACD Contr %')} type="number" step="0.01" min="0.05" max="0.95" bind:value={draft.small.macdContraction} /></div>
                <div class={styles.inputRow}><label for={fieldId('small', 'ADX Trend Th')}>ADX Trend Th</label><input id={fieldId('small', 'ADX Trend Th')} type="number" bind:value={draft.small.adxTrendThreshold} /></div>
                <div class={styles.inputRow}><label for={fieldId('small', 'ADX Exhaustion')}>ADX Exhaustion</label><input id={fieldId('small', 'ADX Exhaustion')} type="number" bind:value={draft.small.adxExhaustionThreshold} /></div>
                <div class={styles.inputRow}><label for={fieldId('small', 'ADX Slope Lbk')}>ADX Slope Lbk</label><input id={fieldId('small', 'ADX Slope Lbk')} type="number" bind:value={draft.small.adxSlopeLookback} /></div>
                <hr class={styles.sectionDivider} />
                <div class={styles.inputRow}><label for={fieldId('small', 'Sqz Min Dur')}>Sqz Min Dur</label><input id={fieldId('small', 'Sqz Min Dur')} type="number" bind:value={draft.small.squeezeMinDuration} /></div>
                <div class={styles.inputRow}><label for={fieldId('small', 'Sqz BB Period')}>Sqz BB Period</label><input id={fieldId('small', 'Sqz BB Period')} type="number" bind:value={draft.small.squeezeBbPeriod} /></div>
                <div class={styles.inputRow}><label for={fieldId('small', 'Sqz BB Std Dev')}>Sqz BB Std Dev</label><input id={fieldId('small', 'Sqz BB Std Dev')} type="number" step="0.1" bind:value={draft.small.squeezeBbStdDev} /></div>
                <div class={styles.inputRow}><label for={fieldId('small', 'Sqz KC Period')}>Sqz KC Period</label><input id={fieldId('small', 'Sqz KC Period')} type="number" bind:value={draft.small.squeezeKcPeriod} /></div>
                <div class={styles.inputRow}><label for={fieldId('small', 'Sqz KC ATR Mult')}>Sqz KC ATR Mult</label><input id={fieldId('small', 'Sqz KC ATR Mult')} type="number" step="0.1" bind:value={draft.small.squeezeKcAtrMult} /></div>
                <hr class={styles.sectionDivider} />
                <div class={styles.inputRow}><label for={fieldId('small', 'ATR Mult')}>ATR Mult</label><input id={fieldId('small', 'ATR Mult')} type="number" step="0.1" bind:value={draft.small.atrMultiplier} /></div>
                <div class={styles.inputRow}><label for={fieldId('small', 'Target R:R')}>Target R:R</label><input id={fieldId('small', 'Target R:R')} type="number" step="0.1" bind:value={draft.small.atrTargetRR} /></div>
                <div class={styles.inputRow}><label for={fieldId('small', 'Vol Avg Period')}>Vol Avg Period</label><input id={fieldId('small', 'Vol Avg Period')} type="number" bind:value={draft.small.volumeAvgPeriod} /></div>
                <div class={styles.inputRow}><label for={fieldId('small', 'RVOL Inst')}>RVOL Inst</label><input id={fieldId('small', 'RVOL Inst')} type="number" step="0.1" bind:value={draft.small.rvolInstitutional} /></div>
                <div class={styles.inputRow}><label for={fieldId('small', 'RVOL Climax')}>RVOL Climax</label><input id={fieldId('small', 'RVOL Climax')} type="number" step="0.1" bind:value={draft.small.rvolClimax} /></div>
                <div class={styles.inputRow}><label for={fieldId('small', 'Analysis Limit')}>Analysis Limit</label><input id={fieldId('small', 'Analysis Limit')} type="number" min="10" max="500" step="5" bind:value={draft.small.analysisLimit} /></div>
            </div>
        </div>

        <div class={styles.termCard}>
            <h3 class={styles.cardTitle}>Medium Term</h3>
            <div class={styles.timeframeRow}>
                <select class={styles.tfSelect}
                    value={selectedOption(draft.medium.durationSeconds)}
                    onchange={(e) => { const v = parseInt(e.currentTarget.value); if (v > 0) draft.medium.durationSeconds = v; }}>
                    <option value={-1} disabled>Custom: {durationLabel(draft.medium.durationSeconds)}</option>
                    {#each TIMEFRAME_OPTIONS as opt}
                        <option value={opt.seconds}>{opt.label}</option>
                    {/each}
                </select>
            </div>
            <div class="{styles.indicatorInputsScroll} font-mono">
                <div class={styles.inputRow}><label for={fieldId('medium', 'EMA Fast')}>EMA Fast</label><input id={fieldId('medium', 'EMA Fast')} type="number" bind:value={draft.medium.emaFast} /></div>
                <div class={styles.inputRow}><label for={fieldId('medium', 'EMA Med')}>EMA Med</label><input id={fieldId('medium', 'EMA Med')} type="number" bind:value={draft.medium.emaMedium} /></div>
                <div class={styles.inputRow}><label for={fieldId('medium', 'EMA Slow')}>EMA Slow</label><input id={fieldId('medium', 'EMA Slow')} type="number" bind:value={draft.medium.emaSlow} /></div>
                <div class={styles.inputRow}><label for={fieldId('medium', 'EMA Long')}>EMA Long</label><input id={fieldId('medium', 'EMA Long')} type="number" bind:value={draft.medium.emaLong} /></div>
                <div class={styles.inputRow}><label for={fieldId('medium', 'RSI Window')}>RSI Window</label><input id={fieldId('medium', 'RSI Window')} type="number" bind:value={draft.medium.rsiPeriod} /></div>
                <div class={styles.inputRow}><label for={fieldId('medium', 'MACD Fast')}>MACD Fast</label><input id={fieldId('medium', 'MACD Fast')} type="number" bind:value={draft.medium.macdFast} /></div>
                <div class={styles.inputRow}><label for={fieldId('medium', 'MACD Slow')}>MACD Slow</label><input id={fieldId('medium', 'MACD Slow')} type="number" bind:value={draft.medium.macdSlow} /></div>
                <div class={styles.inputRow}><label for={fieldId('medium', 'MACD Signal')}>MACD Signal</label><input id={fieldId('medium', 'MACD Signal')} type="number" bind:value={draft.medium.macdSignal} /></div>
                <div class={styles.inputRow}><label for={fieldId('medium', 'ADX Period')}>ADX Period</label><input id={fieldId('medium', 'ADX Period')} type="number" bind:value={draft.medium.adxPeriod} /></div>
                <div class={styles.inputRow}><label for={fieldId('medium', 'ATR Period')}>ATR Period</label><input id={fieldId('medium', 'ATR Period')} type="number" bind:value={draft.medium.atrPeriod} /></div>
                <div class={styles.inputRow}><label for={fieldId('medium', 'Squeeze Wave')}>Squeeze Wave</label><input id={fieldId('medium', 'Squeeze Wave')} type="number" bind:value={draft.medium.squeezePeriod} /></div>
                <div class={styles.inputRow}><label for={fieldId('medium', 'BBWP Period')}>BBWP Period</label><input id={fieldId('medium', 'BBWP Period')} type="number" bind:value={draft.medium.bbwpPeriod} /></div>
                <div class={styles.inputRow}><label for={fieldId('medium', 'BBWP Lookback')}>BBWP Lookback</label><input id={fieldId('medium', 'BBWP Lookback')} type="number" bind:value={draft.medium.bbwpLookback} /></div>
                <hr class={styles.sectionDivider} />
                <div class={styles.inputRow}><label for={fieldId('medium', 'MACD Extr High')}>MACD Extr High</label><input id={fieldId('medium', 'MACD Extr High')} type="number" step="0.01" bind:value={draft.medium.macdExtremeHigh} /></div>
                <div class={styles.inputRow}><label for={fieldId('medium', 'MACD Extr Low')}>MACD Extr Low</label><input id={fieldId('medium', 'MACD Extr Low')} type="number" step="0.01" bind:value={draft.medium.macdExtremeLow} /></div>
                <div class={styles.inputRow}><label for={fieldId('medium', 'MACD Contr %')}>MACD Contr %</label><input id={fieldId('medium', 'MACD Contr %')} type="number" step="0.01" min="0.05" max="0.95" bind:value={draft.medium.macdContraction} /></div>
                <div class={styles.inputRow}><label for={fieldId('medium', 'ADX Trend Th')}>ADX Trend Th</label><input id={fieldId('medium', 'ADX Trend Th')} type="number" bind:value={draft.medium.adxTrendThreshold} /></div>
                <div class={styles.inputRow}><label for={fieldId('medium', 'ADX Exhaustion')}>ADX Exhaustion</label><input id={fieldId('medium', 'ADX Exhaustion')} type="number" bind:value={draft.medium.adxExhaustionThreshold} /></div>
                <div class={styles.inputRow}><label for={fieldId('medium', 'ADX Slope Lbk')}>ADX Slope Lbk</label><input id={fieldId('medium', 'ADX Slope Lbk')} type="number" bind:value={draft.medium.adxSlopeLookback} /></div>
                <hr class={styles.sectionDivider} />
                <div class={styles.inputRow}><label for={fieldId('medium', 'Sqz Min Dur')}>Sqz Min Dur</label><input id={fieldId('medium', 'Sqz Min Dur')} type="number" bind:value={draft.medium.squeezeMinDuration} /></div>
                <div class={styles.inputRow}><label for={fieldId('medium', 'Sqz BB Period')}>Sqz BB Period</label><input id={fieldId('medium', 'Sqz BB Period')} type="number" bind:value={draft.medium.squeezeBbPeriod} /></div>
                <div class={styles.inputRow}><label for={fieldId('medium', 'Sqz BB Std Dev')}>Sqz BB Std Dev</label><input id={fieldId('medium', 'Sqz BB Std Dev')} type="number" step="0.1" bind:value={draft.medium.squeezeBbStdDev} /></div>
                <div class={styles.inputRow}><label for={fieldId('medium', 'Sqz KC Period')}>Sqz KC Period</label><input id={fieldId('medium', 'Sqz KC Period')} type="number" bind:value={draft.medium.squeezeKcPeriod} /></div>
                <div class={styles.inputRow}><label for={fieldId('medium', 'Sqz KC ATR Mult')}>Sqz KC ATR Mult</label><input id={fieldId('medium', 'Sqz KC ATR Mult')} type="number" step="0.1" bind:value={draft.medium.squeezeKcAtrMult} /></div>
                <hr class={styles.sectionDivider} />
                <div class={styles.inputRow}><label for={fieldId('medium', 'ATR Mult')}>ATR Mult</label><input id={fieldId('medium', 'ATR Mult')} type="number" step="0.1" bind:value={draft.medium.atrMultiplier} /></div>
                <div class={styles.inputRow}><label for={fieldId('medium', 'Target R:R')}>Target R:R</label><input id={fieldId('medium', 'Target R:R')} type="number" step="0.1" bind:value={draft.medium.atrTargetRR} /></div>
                <div class={styles.inputRow}><label for={fieldId('medium', 'Vol Avg Period')}>Vol Avg Period</label><input id={fieldId('medium', 'Vol Avg Period')} type="number" bind:value={draft.medium.volumeAvgPeriod} /></div>
                <div class={styles.inputRow}><label for={fieldId('medium', 'RVOL Inst')}>RVOL Inst</label><input id={fieldId('medium', 'RVOL Inst')} type="number" step="0.1" bind:value={draft.medium.rvolInstitutional} /></div>
                <div class={styles.inputRow}><label for={fieldId('medium', 'RVOL Climax')}>RVOL Climax</label><input id={fieldId('medium', 'RVOL Climax')} type="number" step="0.1" bind:value={draft.medium.rvolClimax} /></div>
                <div class={styles.inputRow}><label for={fieldId('medium', 'Analysis Limit')}>Analysis Limit</label><input id={fieldId('medium', 'Analysis Limit')} type="number" min="10" max="500" step="5" bind:value={draft.medium.analysisLimit} /></div>
            </div>
        </div>

        <div class={styles.termCard}>
            <h3 class={styles.cardTitle}>Large Term</h3>
            <div class={styles.timeframeRow}>
                <select class={styles.tfSelect}
                    value={selectedOption(draft.large.durationSeconds)}
                    onchange={(e) => { const v = parseInt(e.currentTarget.value); if (v > 0) draft.large.durationSeconds = v; }}>
                    <option value={-1} disabled>Custom: {durationLabel(draft.large.durationSeconds)}</option>
                    {#each TIMEFRAME_OPTIONS as opt}
                        <option value={opt.seconds}>{opt.label}</option>
                    {/each}
                </select>
            </div>
            <div class="{styles.indicatorInputsScroll} font-mono">
                <div class={styles.inputRow}><label for={fieldId('large', 'EMA Fast')}>EMA Fast</label><input id={fieldId('large', 'EMA Fast')} type="number" bind:value={draft.large.emaFast} /></div>
                <div class={styles.inputRow}><label for={fieldId('large', 'EMA Med')}>EMA Med</label><input id={fieldId('large', 'EMA Med')} type="number" bind:value={draft.large.emaMedium} /></div>
                <div class={styles.inputRow}><label for={fieldId('large', 'EMA Slow')}>EMA Slow</label><input id={fieldId('large', 'EMA Slow')} type="number" bind:value={draft.large.emaSlow} /></div>
                <div class={styles.inputRow}><label for={fieldId('large', 'EMA Long')}>EMA Long</label><input id={fieldId('large', 'EMA Long')} type="number" bind:value={draft.large.emaLong} /></div>
                <div class={styles.inputRow}><label for={fieldId('large', 'RSI Window')}>RSI Window</label><input id={fieldId('large', 'RSI Window')} type="number" bind:value={draft.large.rsiPeriod} /></div>
                <div class={styles.inputRow}><label for={fieldId('large', 'MACD Fast')}>MACD Fast</label><input id={fieldId('large', 'MACD Fast')} type="number" bind:value={draft.large.macdFast} /></div>
                <div class={styles.inputRow}><label for={fieldId('large', 'MACD Slow')}>MACD Slow</label><input id={fieldId('large', 'MACD Slow')} type="number" bind:value={draft.large.macdSlow} /></div>
                <div class={styles.inputRow}><label for={fieldId('large', 'MACD Signal')}>MACD Signal</label><input id={fieldId('large', 'MACD Signal')} type="number" bind:value={draft.large.macdSignal} /></div>
                <div class={styles.inputRow}><label for={fieldId('large', 'ADX Period')}>ADX Period</label><input id={fieldId('large', 'ADX Period')} type="number" bind:value={draft.large.adxPeriod} /></div>
                <div class={styles.inputRow}><label for={fieldId('large', 'ATR Period')}>ATR Period</label><input id={fieldId('large', 'ATR Period')} type="number" bind:value={draft.large.atrPeriod} /></div>
                <div class={styles.inputRow}><label for={fieldId('large', 'Squeeze Wave')}>Squeeze Wave</label><input id={fieldId('large', 'Squeeze Wave')} type="number" bind:value={draft.large.squeezePeriod} /></div>
                <div class={styles.inputRow}><label for={fieldId('large', 'BBWP Period')}>BBWP Period</label><input id={fieldId('large', 'BBWP Period')} type="number" bind:value={draft.large.bbwpPeriod} /></div>
                <div class={styles.inputRow}><label for={fieldId('large', 'BBWP Lookback')}>BBWP Lookback</label><input id={fieldId('large', 'BBWP Lookback')} type="number" bind:value={draft.large.bbwpLookback} /></div>
                <hr class={styles.sectionDivider} />
                <div class={styles.inputRow}><label for={fieldId('large', 'MACD Extr High')}>MACD Extr High</label><input id={fieldId('large', 'MACD Extr High')} type="number" step="0.01" bind:value={draft.large.macdExtremeHigh} /></div>
                <div class={styles.inputRow}><label for={fieldId('large', 'MACD Extr Low')}>MACD Extr Low</label><input id={fieldId('large', 'MACD Extr Low')} type="number" step="0.01" bind:value={draft.large.macdExtremeLow} /></div>
                <div class={styles.inputRow}><label for={fieldId('large', 'MACD Contr %')}>MACD Contr %</label><input id={fieldId('large', 'MACD Contr %')} type="number" step="0.01" min="0.05" max="0.95" bind:value={draft.large.macdContraction} /></div>
                <div class={styles.inputRow}><label for={fieldId('large', 'ADX Trend Th')}>ADX Trend Th</label><input id={fieldId('large', 'ADX Trend Th')} type="number" bind:value={draft.large.adxTrendThreshold} /></div>
                <div class={styles.inputRow}><label for={fieldId('large', 'ADX Exhaustion')}>ADX Exhaustion</label><input id={fieldId('large', 'ADX Exhaustion')} type="number" bind:value={draft.large.adxExhaustionThreshold} /></div>
                <div class={styles.inputRow}><label for={fieldId('large', 'ADX Slope Lbk')}>ADX Slope Lbk</label><input id={fieldId('large', 'ADX Slope Lbk')} type="number" bind:value={draft.large.adxSlopeLookback} /></div>
                <hr class={styles.sectionDivider} />
                <div class={styles.inputRow}><label for={fieldId('large', 'Sqz Min Dur')}>Sqz Min Dur</label><input id={fieldId('large', 'Sqz Min Dur')} type="number" bind:value={draft.large.squeezeMinDuration} /></div>
                <div class={styles.inputRow}><label for={fieldId('large', 'Sqz BB Period')}>Sqz BB Period</label><input id={fieldId('large', 'Sqz BB Period')} type="number" bind:value={draft.large.squeezeBbPeriod} /></div>
                <div class={styles.inputRow}><label for={fieldId('large', 'Sqz BB Std Dev')}>Sqz BB Std Dev</label><input id={fieldId('large', 'Sqz BB Std Dev')} type="number" step="0.1" bind:value={draft.large.squeezeBbStdDev} /></div>
                <div class={styles.inputRow}><label for={fieldId('large', 'Sqz KC Period')}>Sqz KC Period</label><input id={fieldId('large', 'Sqz KC Period')} type="number" bind:value={draft.large.squeezeKcPeriod} /></div>
                <div class={styles.inputRow}><label for={fieldId('large', 'Sqz KC ATR Mult')}>Sqz KC ATR Mult</label><input id={fieldId('large', 'Sqz KC ATR Mult')} type="number" step="0.1" bind:value={draft.large.squeezeKcAtrMult} /></div>
                <hr class={styles.sectionDivider} />
                <div class={styles.inputRow}><label for={fieldId('large', 'ATR Mult')}>ATR Mult</label><input id={fieldId('large', 'ATR Mult')} type="number" step="0.1" bind:value={draft.large.atrMultiplier} /></div>
                <div class={styles.inputRow}><label for={fieldId('large', 'Target R:R')}>Target R:R</label><input id={fieldId('large', 'Target R:R')} type="number" step="0.1" bind:value={draft.large.atrTargetRR} /></div>
                <div class={styles.inputRow}><label for={fieldId('large', 'Vol Avg Period')}>Vol Avg Period</label><input id={fieldId('large', 'Vol Avg Period')} type="number" bind:value={draft.large.volumeAvgPeriod} /></div>
                <div class={styles.inputRow}><label for={fieldId('large', 'RVOL Inst')}>RVOL Inst</label><input id={fieldId('large', 'RVOL Inst')} type="number" step="0.1" bind:value={draft.large.rvolInstitutional} /></div>
                <div class={styles.inputRow}><label for={fieldId('large', 'RVOL Climax')}>RVOL Climax</label><input id={fieldId('large', 'RVOL Climax')} type="number" step="0.1" bind:value={draft.large.rvolClimax} /></div>
                <div class={styles.inputRow}><label for={fieldId('large', 'Analysis Limit')}>Analysis Limit</label><input id={fieldId('large', 'Analysis Limit')} type="number" min="10" max="500" step="5" bind:value={draft.large.analysisLimit} /></div>
            </div>
        </div>
    </div>

    <div class={styles.applyRow}>
        {#if saveStatus === 'error'}
            <span class={styles.errorMsg}>Save failed. Check console for details.</span>
        {/if}
        <button class={styles.applyWorkspaceBtn} disabled={saveStatus === 'saving'} onclick={applySettings}>
            {saveStatus === 'saving' ? 'Applying...' : saveStatus === 'success' ? 'Applied! Returning...' : 'Apply Workspace Configuration'}
        </button>
    </div>
</div>
