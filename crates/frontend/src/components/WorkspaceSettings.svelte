<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import { createInstance } from '../lib/api.svelte';
    import type { InstanceState, PositionScalingConfig, TimeframeTelemetry } from '../types';
    import { TIMEFRAME_OPTIONS } from '../types';
    import IndicatorWeightPanel from './settings/IndicatorWeightPanel.svelte';
    import ScoringWeightsPanel from './settings/ScoringWeightsPanel.svelte';
    import PositionScalingPanel from './settings/PositionScalingPanel.svelte';
    import TriggerConfigPanel from './settings/TriggerConfigPanel.svelte';
    import styles from './WorkspaceSettings.module.css';

    let { pair, tabKey }: { pair: InstanceState; tabKey: string } = $props();

    const app = useAppStore();

    let identityError = $state<string | null>(null);

    const panelToggles: Array<[string, string]> = [
        ['showVolume', 'Volume'], ['showAdx', 'ADX'], ['showAtr', 'ATR'], ['showRsi', 'RSI'],
        ['showMacd', 'MACD'], ['showSqueeze', 'Squeeze'], ['showBbwp', 'BBWP'], ['showRvol', 'RVOL'],
        ['showFib', 'Fibonacci'], ['showStochastic', 'STOCH'], ['showChandeMo', 'CHANDE MO'],
        ['showSupertrend', 'SUPERTREND'], ['showKeltner', 'KELTNER'], ['showDonchian', 'DONCHIAN'],
        ['showObv', 'OBV'], ['showCmf', 'CMF'], ['showMfi', 'MFI'], ['showHv', 'HIST VOL'],
        ['showAroon', 'AROON'], ['showChoppiness', 'CHOP'], ['showLinregSlope', 'LINREG'], ['showZscore', 'Z-SCORE'],
    ];

    let draft = $state({
        symbol: '',
        exchange: 'Hyperliquid' as string,
        analysisLimit: 100 as number,
        visuals: {
            showEmas: true, showBb: true, showVwap: true, showVolume: true,
            showAdx: true, showAtr: true, showRsi: true, showMacd: true,
            showSqueeze: true, showBbwp: true, showFib: true,
            showRvol: true, showStochastic: true, showChandeMo: true,
            showSupertrend: true, showKeltner: true, showDonchian: true,
            showObv: true, showCmf: true, showMfi: true, showHv: true,
            showAroon: true, showChoppiness: true, showLinregSlope: true, showZscore: true,
        },
        automation: {
            enabled: false as boolean,
            intervalValue: 15 as number,
            intervalUnit: 'minutes' as 'seconds' | 'minutes' | 'hours',
        },
        rules: '' as string,
    });

    let rulesStatus = $state<'idle' | 'loading' | 'saving' | 'success' | 'error'>('idle');
    let draftCostInputPrice = $state(app.costPriceInput);
    let draftCostOutputPrice = $state(app.costPriceOutput);
    let costSaveStatus = $state<'idle' | 'saving' | 'success' | 'error'>('idle');

    let weightOverrides = $state<Record<string, number>>({});
    let positionScaling = $state<PositionScalingConfig | null>(null);
    let aiTriggerConfig = $state<{ trigger: import('../types').TriggerModeConfig } | null>(null);
    let operationalMode = $state<import('../types').OperationalMode>('HybridAiCopilot');

    let saveStatus = $state<'idle' | 'saving' | 'success' | 'error'>('idle');
    let aiConfigSaveStatus = $state<'idle' | 'saving' | 'success' | 'error'>('idle');
    let showTimeframeConfig = $state(false);

    // ─── Timeframe Indicator Configuration ──────────────────────────────────

    interface TermDraft {
        durationSeconds: number;
        emaFast: number; emaMedium: number; emaSlow: number; emaLong: number;
        rsiPeriod: number;
        macdFast: number; macdSlow: number; macdSignal: number;
        adxPeriod: number; atrPeriod: number; squeezePeriod: number;
        bbwpPeriod: number; bbwpLookback: number;
        stochKPeriod: number; stochDPeriod: number; stochSPeriod: number; chandemoPeriod: number;
        supertrendPeriod: number; supertrendMultiplier: number;
        keltnerEmaPeriod: number; keltnerAtrPeriod: number; keltnerMultiplier: number;
        donchianPeriod: number; obvSmoothing: number; cmfPeriod: number; mfiPeriod: number; hvPeriod: number;
        aroonPeriod: number; chopPeriod: number; linregPeriod: number; zscorePeriod: number;
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
            stochKPeriod: 18, stochDPeriod: 5, stochSPeriod: 9, chandemoPeriod: 12,
            supertrendPeriod: 10, supertrendMultiplier: 3.0,
            keltnerEmaPeriod: 20, keltnerAtrPeriod: 10, keltnerMultiplier: 2.0,
            donchianPeriod: 20, obvSmoothing: 20, cmfPeriod: 20, mfiPeriod: 14, hvPeriod: 20,
            aroonPeriod: 25, chopPeriod: 14, linregPeriod: 20, zscorePeriod: 20,
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
            stochKPeriod: tf.stochKPeriodVal, stochDPeriod: tf.stochDPeriodVal, stochSPeriod: tf.stochSPeriodVal, chandemoPeriod: tf.chandemoPeriodVal,
            supertrendPeriod: tf.supertrendPeriodVal, supertrendMultiplier: tf.supertrendMultiplierVal,
            keltnerEmaPeriod: tf.keltnerEmaPeriodVal, keltnerAtrPeriod: tf.keltnerAtrPeriodVal, keltnerMultiplier: tf.keltnerMultiplierVal,
            donchianPeriod: tf.donchianPeriodVal, obvSmoothing: tf.obvSmoothingVal, cmfPeriod: tf.cmfPeriodVal, mfiPeriod: tf.mfiPeriodVal, hvPeriod: tf.hvPeriodVal,
            aroonPeriod: tf.aroonPeriodVal, chopPeriod: tf.chopPeriodVal, linregPeriod: tf.linregPeriodVal, zscorePeriod: tf.zscorePeriodVal,
            macdExtremeHigh: tf.macdExtremeHighVal, macdExtremeLow: tf.macdExtremeLowVal, macdContraction: tf.macdContractionVal,
            adxTrendThreshold: tf.adxTrendThresholdVal, adxExhaustionThreshold: tf.adxExhaustionThresholdVal, adxSlopeLookback: tf.adxSlopeLookbackVal,
            squeezeMinDuration: tf.squeezeMinDurationVal, squeezeBbPeriod: tf.squeezeBbPeriodVal, squeezeBbStdDev: tf.squeezeBbStdDevVal,
            squeezeKcPeriod: tf.squeezeKcPeriodVal, squeezeKcAtrMult: tf.squeezeKcAtrMultVal,
            atrMultiplier: tf.atrMultiplierVal, atrTargetRR: tf.atrTargetRRVal,
            volumeAvgPeriod: tf.volumeAvgPeriodVal, rvolInstitutional: tf.rvolInstitutionalVal, rvolClimax: tf.rvolClimaxVal,
            analysisLimit: tf.analysisLimit,
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
        tf.stochKPeriodVal = term.stochKPeriod; tf.stochDPeriodVal = term.stochDPeriod;
        tf.stochSPeriodVal = term.stochSPeriod; tf.chandemoPeriodVal = term.chandemoPeriod;
        tf.supertrendPeriodVal = term.supertrendPeriod; tf.supertrendMultiplierVal = term.supertrendMultiplier;
        tf.keltnerEmaPeriodVal = term.keltnerEmaPeriod; tf.keltnerAtrPeriodVal = term.keltnerAtrPeriod;
        tf.keltnerMultiplierVal = term.keltnerMultiplier; tf.donchianPeriodVal = term.donchianPeriod;
        tf.obvSmoothingVal = term.obvSmoothing; tf.cmfPeriodVal = term.cmfPeriod;
        tf.mfiPeriodVal = term.mfiPeriod; tf.hvPeriodVal = term.hvPeriod;
        tf.aroonPeriodVal = term.aroonPeriod; tf.chopPeriodVal = term.chopPeriod;
        tf.linregPeriodVal = term.linregPeriod; tf.zscorePeriodVal = term.zscorePeriod;
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
        tf.indicators = {};
    }

    function buildIndicators(term: TermDraft): Record<string, number> {
        return {
            ema_fast: term.emaFast, ema_medium: term.emaMedium, ema_slow: term.emaSlow, ema_long: term.emaLong,
            rsi_period: term.rsiPeriod,
            macd_fast: term.macdFast, macd_slow: term.macdSlow, macd_signal: term.macdSignal,
            adx_period: term.adxPeriod, atr_period: term.atrPeriod, squeeze_period: term.squeezePeriod,
            bbwp_period: term.bbwpPeriod, bbwp_lookback: term.bbwpLookback,
            stoch_k_period: term.stochKPeriod, stoch_d_period: term.stochDPeriod,
            stoch_s_period: term.stochSPeriod, chandemo_period: term.chandemoPeriod,
            supertrend_period: term.supertrendPeriod, supertrend_multiplier: term.supertrendMultiplier,
            keltner_ema_period: term.keltnerEmaPeriod, keltner_atr_period: term.keltnerAtrPeriod,
            keltner_multiplier: term.keltnerMultiplier, donchian_period: term.donchianPeriod,
            obv_smoothing: term.obvSmoothing, cmf_period: term.cmfPeriod,
            mfi_period: term.mfiPeriod, hv_period: term.hvPeriod,
            aroon_period: term.aroonPeriod, chop_period: term.chopPeriod,
            linreg_period: term.linregPeriod, zscore_period: term.zscorePeriod,
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

    function fieldId(term: string, label: string): string {
        const slug = label.toLowerCase().replace(/%/g, 'pct').replace(/[:\s]+/g, '-').replace(/[^a-z0-9-]/g, '');
        return `tf-${term}-${slug}`;
    }

    let tfDraft = $state({
        micro: defaultTermDraft(),
        fast: defaultTermDraft(),
        slow: defaultTermDraft(),
        macro: defaultTermDraft(),
    });

    function selectedOption(seconds: number): number {
        const found = TIMEFRAME_OPTIONS.find(o => o.seconds === seconds);
        return found ? found.seconds : -1;
    }

    function durationLabel(seconds: number): string {
        const found = TIMEFRAME_OPTIONS.find(o => o.seconds === seconds);
        return found ? found.label : `${seconds}s`;
    }

    $effect(() => {
        draft.symbol = pair.symbol; draft.exchange = pair.exchange;
        draft.analysisLimit = pair.microTerm.analysisLimit;
        for (const f of ['showEmas','showBb','showVwap','showVolume','showAdx','showAtr','showRsi','showMacd','showSqueeze','showBbwp','showFib','showRvol','showStochastic','showChandeMo','showSupertrend','showKeltner','showDonchian','showObv','showCmf','showMfi','showHv','showAroon','showChoppiness','showLinregSlope','showZscore']) {
            (draft.visuals as any)[f] = (pair.microTerm as any)[f];
        }
        draft.automation.enabled = pair.automationEnabled;
        draft.automation.intervalValue = pair.automationIntervalValue;
        draft.automation.intervalUnit = pair.automationIntervalUnit;
        tfDraft.micro = readTermFromTelemetry(pair.microTerm);
        tfDraft.fast = readTermFromTelemetry(pair.fastTerm);
        tfDraft.slow = readTermFromTelemetry(pair.slowTerm);
        tfDraft.macro = readTermFromTelemetry(pair.macroTerm);
    });

    let calculatedAutomationInterval = $derived.by(() => {
        const val = Number(draft.automation.intervalValue) || 1;
        if (draft.automation.intervalUnit === 'hours') return val * 3600;
        if (draft.automation.intervalUnit === 'minutes') return val * 60;
        return val;
    });

    function formatIntervalRemaining(totalSeconds: number): string {
        const h = Math.floor(totalSeconds / 3600);
        const m = Math.floor((totalSeconds % 3600) / 60);
        const s = totalSeconds % 60;
        if (h > 0) return `${h}h ${m.toString().padStart(2, '0')}m`;
        if (m > 0) return `${m}m ${s.toString().padStart(2, '0')}s`;
        return `${s}s`;
    }

    async function saveCostConfig() {
        costSaveStatus = 'saving';
        try {
            const res = await fetch('/api/config/costs', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    price_per_1m_input_tokens: Number(draftCostInputPrice),
                    price_per_1m_output_tokens: Number(draftCostOutputPrice),
                }),
            });
            costSaveStatus = res.ok ? 'success' : 'error';
            if (res.ok) {
                app.costPriceInput = Number(draftCostInputPrice);
                app.costPriceOutput = Number(draftCostOutputPrice);
                setTimeout(() => { costSaveStatus = 'idle'; }, 2000);
            }
        } catch (_) {
            costSaveStatus = 'error';
        }
    }

    async function fetchRules() {
        rulesStatus = 'loading';
        try {
            const res = await fetch('/api/rules');
            const data = await res.json();
            draft.rules = data.content || '';
            app.rulesContent = draft.rules;
            rulesStatus = 'idle';
        } catch (_) {
            rulesStatus = 'error';
        }
    }

    async function saveRules() {
        rulesStatus = 'saving';
        try {
            const res = await fetch('/api/rules', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ content: draft.rules }),
            });
            if (res.ok) {
                app.rulesContent = draft.rules;
                rulesStatus = 'success';
                setTimeout(() => { rulesStatus = 'idle'; }, 2000);
            } else {
                rulesStatus = 'error';
            }
        } catch (_) {
            rulesStatus = 'error';
        }
    }

    function applyVisualsToTerm(term: Record<string, any>, vis: typeof draft.visuals) {
        Object.assign(term, {
            showEmas: vis.showEmas, showBb: vis.showBb, showVwap: vis.showVwap,
            showVolume: vis.showVolume, showAdx: vis.showAdx, showAtr: vis.showAtr,
            showRsi: vis.showRsi, showMacd: vis.showMacd, showSqueeze: vis.showSqueeze,
            showBbwp: vis.showBbwp, showFib: vis.showFib,
            showRvol: vis.showRvol,
            showStochastic: vis.showStochastic, showChandeMo: vis.showChandeMo,
            showSupertrend: vis.showSupertrend, showKeltner: vis.showKeltner, showDonchian: vis.showDonchian,
            showObv: vis.showObv, showCmf: vis.showCmf, showMfi: vis.showMfi, showHv: vis.showHv,
            showAroon: vis.showAroon, showChoppiness: vis.showChoppiness, showLinregSlope: vis.showLinregSlope, showZscore: vis.showZscore,
        });
    }

    async function applySettings() {
        const cleanedSymbol = draft.symbol.trim().toUpperCase();
        identityError = null;
        if (!/^[A-Z0-9]{2,10}$/.test(cleanedSymbol)) {
            identityError = 'Invalid ticker. Must be 2-10 alphanumeric characters.';
            return;
        }

        const durations = [
            tfDraft.micro.durationSeconds,
            tfDraft.fast.durationSeconds,
            tfDraft.slow.durationSeconds,
            tfDraft.macro.durationSeconds,
        ];
        if (new Set(durations).size < 4) {
            alert('Each timeframe must have a unique duration.');
            return;
        }

        const { automation: auto, visuals: vis } = draft;
        const isIdentityChanged = cleanedSymbol !== pair.symbol || draft.exchange !== pair.exchange;
        let targetTabKey = tabKey;
        let target = pair;

        if (isIdentityChanged) {
            const newPairKey = app.pairKeyFor(cleanedSymbol);
            const result = await createInstance(cleanedSymbol, app.quote);
            if (!result.ok) {
                identityError = result.error || 'Failed to update instance.';
                return;
            }
            app.initInstance(cleanedSymbol, draft.exchange);
            target = app.instancesMap[newPairKey] || pair;
            app.removeInstance(tabKey);
            app.activeTab = newPairKey;
            targetTabKey = newPairKey;
        }

        for (const tf of [target.microTerm, target.fastTerm, target.slowTerm, target.macroTerm]) {
            applyVisualsToTerm(tf, vis);
            tf.analysisLimit = draft.analysisLimit;
        }

        target.automationEnabled = auto.enabled;
        target.automationIntervalValue = auto.intervalValue;
        target.automationIntervalUnit = auto.intervalUnit;

        // POST timeframe indicator config to backend
        saveStatus = 'saving';
        try {
            const body = {
                micro_term: { candles: { duration_seconds: tfDraft.micro.durationSeconds, analysis_limit: tfDraft.micro.analysisLimit }, indicators: buildIndicators(tfDraft.micro) },
                fast_term: { candles: { duration_seconds: tfDraft.fast.durationSeconds, analysis_limit: tfDraft.fast.analysisLimit }, indicators: buildIndicators(tfDraft.fast) },
                slow_term: { candles: { duration_seconds: tfDraft.slow.durationSeconds, analysis_limit: tfDraft.slow.analysisLimit }, indicators: buildIndicators(tfDraft.slow) },
                macro_term: { candles: { duration_seconds: tfDraft.macro.durationSeconds, analysis_limit: tfDraft.macro.analysisLimit }, indicators: buildIndicators(tfDraft.macro) },
                automation: { enabled: auto.enabled, interval_seconds: calculatedAutomationInterval },
            };
            const res = await fetch(`/api/instances/${encodeURIComponent(targetTabKey)}/config`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(body),
            });
            if (res.ok) {
                applyTermToTelemetry(tfDraft.micro, target.microTerm);
                applyTermToTelemetry(tfDraft.fast, target.fastTerm);
                applyTermToTelemetry(tfDraft.slow, target.slowTerm);
                applyTermToTelemetry(tfDraft.macro, target.macroTerm);
                saveStatus = 'success';
                setTimeout(() => { saveStatus = 'idle'; }, 2000);
            } else {
                saveStatus = 'error';
            }
        } catch (e) {
            console.error('Config save error:', e);
            saveStatus = 'error';
        }
    }

    async function saveAiConfig() {
        aiConfigSaveStatus = 'saving';
        try {
            const payload: Record<string, unknown> = {
                operational_mode: operationalMode,
                weight_overrides: weightOverrides && Object.keys(weightOverrides).length > 0 ? weightOverrides : null,
                position_scaling: positionScaling,
                ai_trigger: aiTriggerConfig,
            };
            const res = await fetch(`/api/instances/${encodeURIComponent(tabKey)}/config`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(payload),
            });
            aiConfigSaveStatus = res.ok ? 'success' : 'error';
            if (res.ok) setTimeout(() => { aiConfigSaveStatus = 'idle'; }, 2000);
        } catch (_) {
            aiConfigSaveStatus = 'error';
        }
    }
</script>

<div class="{styles.settingsWorkspaceTab} animate-fade">
    <div class={styles.settingsGrid}>

        <!-- Visual Layout Column -->
        <div class={styles.settingsCol}>
            <h3 class={styles.cardTitle}>Visual Overlays</h3>
            <div class={styles.settingGroupBox}>
                <span class={styles.selectorsLabel}>Chart Display Items</span>
                <div class={styles.toggleGrid}>
                    <button class="{styles.selectorBtn} {draft.visuals.showEmas ? styles.active : ''}" onclick={() => draft.visuals.showEmas = !draft.visuals.showEmas}>EMAs</button>
                    <button class="{styles.selectorBtn} {draft.visuals.showBb ? styles.active : ''}" onclick={() => draft.visuals.showBb = !draft.visuals.showBb}>Bollinger</button>
                    <button class="{styles.selectorBtn} {draft.visuals.showVwap ? styles.active : ''}" onclick={() => draft.visuals.showVwap = !draft.visuals.showVwap}>VWAP</button>
                </div>
            </div>

            <div class={styles.settingGroupBox} style="margin-top: 12px;">
                <span class={styles.selectorsLabel}>Indicator Panels</span>
                <div class={styles.toggleGrid}>
                    {#each panelToggles as [key, lbl]}
                        <button class="{styles.selectorBtn} {(draft.visuals as any)[key] ? styles.active : ''}" onclick={() => (draft.visuals as any)[key] = !(draft.visuals as any)[key]}>{lbl}</button>
                    {/each}
                </div>
            </div>

            <div style="margin-top: 12px;">
                <ScoringWeightsPanel />
            </div>

            <div class={styles.settingGroupBox} style="margin-top: 12px;">
                <span class={styles.selectorsLabel}>AI & Automation</span>
                <div class={styles.toggleRow}>
                    <span class={styles.toggleLabel}>Status</span>
                    <button class="{styles.selectorBtn} {draft.automation.enabled ? styles.active : ''}"
                            onclick={() => draft.automation.enabled = !draft.automation.enabled}>
                        {draft.automation.enabled ? 'ON' : 'OFF'}
                    </button>
                </div>
                <div class={styles.inputRow} style="margin-top: 8px;">
                    <label for="opMode">Operational Mode:</label>
                    <select id="opMode" bind:value={operationalMode} class={styles.tfUnitSelect}>
                        <option value="HybridAiCopilot">AI Copilot</option>
                        <option value="DeterministicHeuristics">Heuristics Only</option>
                        <option value="ManualOnly">Manual Only</option>
                    </select>
                </div>
                <p style="font-size: 8px; color: #64748b; margin: 4px 0 0 0;">
                    AI Copilot: full LLM pipeline. Heuristics Only: local indicators, no AI calls. Manual Only: local indicators + on-demand AI via sidebar.
                </p>
                {#if draft.automation.enabled}
                    <div class={styles.inputRow} style="margin-top: 8px;">
                        <label for="autoInterval">Interval:</label>
                        <div class={styles.tfSplitGroup}>
                            <input id="autoInterval" type="number" bind:value={draft.automation.intervalValue} min="1" class={styles.tfNumberInput} />
                            <select bind:value={draft.automation.intervalUnit} class={styles.tfUnitSelect}>
                                <option value="seconds">Seconds</option>
                                <option value="minutes">Minutes</option>
                                <option value="hours">Hours</option>
                            </select>
                        </div>
                    </div>
                    <div class={styles.liveCounter} style="margin-top: 8px; font-size: 10px; color: #3b82f6;">
                        Next evaluation in: {pair.nextEvaluationIn}
                    </div>
                {/if}
            </div>

            <div class={styles.settingGroupBox} style="margin-top: 12px;">
                <span class={styles.selectorsLabel}>Identity</span>
                <div class={styles.inputRow} style="margin-top: 4px;">
                    <label for="exchange">Exchange Source:</label>
                    <select id="exchange" bind:value={draft.exchange} class={styles.tfUnitSelect}>
                        <option value="Hyperliquid">Hyperliquid</option>
                    </select>
                </div>
                <div class={styles.inputRow} style="margin-top: 8px;">
                    <label for="symbol">Market Pair:</label>
                    <input id="symbol" type="text" bind:value={draft.symbol} />
                </div>
            </div>

            <div class="settings-footer-row" style="margin-top: 16px;">
                <button class={styles.applyWorkspaceBtn} disabled={saveStatus === 'saving'} onclick={applySettings}>
                    {saveStatus === 'saving' ? 'Saving...' : saveStatus === 'success' ? 'Saved!' : 'Save Workspace Configuration'}
                </button>
            </div>
            {#if identityError}
                <div class={styles.identityError} role="alert">{identityError}</div>
            {/if}
            {#if saveStatus === 'error'}
                <div class={styles.identityError} role="alert">Save failed. Check console.</div>
            {/if}
        </div>

        <!-- Backend & AI Prompts Column -->
        <div class={styles.settingsCol}>
            <h3 class={styles.cardTitle}>AI Configuration</h3>

            <div class={styles.settingGroupBox}>
                <div class={styles.inputRow}>
                    <label for="wsAnalysisLimit">AI Analysis Lookback (Candles):</label>
                    <input id="wsAnalysisLimit" type="number" bind:value={draft.analysisLimit} min="10" max="500" step="5" />
                </div>
            </div>

            <div style="margin-top: 12px;">
                <IndicatorWeightPanel initial={weightOverrides} onchange={(w) => { weightOverrides = w; }} />
            </div>

            <div style="margin-top: 12px;">
                <PositionScalingPanel initial={positionScaling} onchange={(c) => { positionScaling = c; }} />
            </div>

            <div style="margin-top: 12px;">
                <TriggerConfigPanel initial={aiTriggerConfig} onchange={(c) => { aiTriggerConfig = c; }} />
            </div>

            <button class={styles.keySaveBtn} style="margin-top: 8px; width: 100%;"
                    disabled={aiConfigSaveStatus === 'saving'} onclick={saveAiConfig}>
                {aiConfigSaveStatus === 'saving' ? 'Saving...' : 'Save AI Configuration'}
            </button>
            {#if aiConfigSaveStatus === 'success'}
                <div class="{styles.statusMsg} {styles.successMsg}">AI config saved.</div>
            {/if}

            <div class={styles.settingGroupBox} style="margin-top: 12px;">
                <span class={styles.selectorsLabel}>Technical rules guide handbook (Markdown)</span>
                <textarea class={styles.rulesEditor} rows="6" bind:value={draft.rules}></textarea>
                <div style="display: flex; justify-content: space-between; align-items: center; margin-top: 4px;">
                    <button class={styles.keySaveBtn} onclick={fetchRules}>Fetch</button>
                    <button class={styles.keySaveBtn} disabled={rulesStatus === 'saving'} onclick={saveRules}>
                        {rulesStatus === 'saving' ? '...' : 'Update Rules'}
                    </button>
                </div>
            </div>
        </div>

        <!-- Paper Trading Rules Column -->
        <div class={styles.settingsCol}>
            <h3 class={styles.cardTitle}>Paper Trading Rules</h3>

            <div class={styles.settingGroupBox}>
                <span class={styles.selectorsLabel}>Account Configuration</span>
                <div class={styles.inputRow} style="margin-top: 4px;">
                    <label for="paperUSD">Initial USD:</label>
                    <input id="paperUSD" type="number" bind:value={app.paperInitialUSD} min="100" step="100" />
                </div>
                <div class={styles.inputRow} style="margin-top: 8px;">
                    <label for="paperAlloc">Allocation %:</label>
                    <input id="paperAlloc" type="number" bind:value={app.paperAllocationPct} min="1" max="100" step="1" />
                </div>
                <div class={styles.inputRow} style="margin-top: 8px;">
                    <label for="paperMaxRisk">Max Risk %:</label>
                    <input id="paperMaxRisk" type="number" bind:value={app.paperMaxRiskPct} min="0.5" max="10" step="0.1" />
                </div>
                <div class={styles.inputRow} style="margin-top: 8px;">
                    <label for="paperLeverage">Leverage:</label>
                    <input id="paperLeverage" type="number" bind:value={app.paperLeverage} min="1" max="20" step="1" />
                </div>
                <button class={styles.keySaveBtn} style="margin-top: 8px; width: 100%;"
                        onclick={() => app.savePaperConfig(app.paperInitialUSD, app.paperAllocationPct, app.paperAutoExecute)}>
                    Save Paper Config
                </button>
            </div>

            <div class={styles.settingGroupBox} style="margin-top: 12px;">
                <span class={styles.selectorsLabel}>AI Orchestrator Settings</span>
                <div class={styles.inputRow} style="margin-top: 4px;">
                    <label for="paperInterval">Eval Interval (min):</label>
                    <input id="paperInterval" type="number" bind:value={app.paperAutoExecuteIntervals} min="1" max="1440" step="1" />
                </div>
                <div class={styles.inputRow} style="margin-top: 8px;">
                    <label for="paperLookback">Lookback Trades:</label>
                    <input id="paperLookback" type="number" bind:value={app.paperLookbackTrades} min="1" max="50" step="1" />
                </div>
                <p style="font-size: 9px; color: #64748b; margin: 6px 0 0 0;">
                    Number of past trades fed to the Master Orchestrator for context.
                </p>
            </div>

            <div class={styles.settingGroupBox} style="margin-top: 12px;">
                <span class={styles.selectorsLabel}>Auto-Execution</span>
                <div class={styles.toggleRow}>
                    <span class={styles.toggleLabel}>Auto-Place Orders</span>
                    <button class="{styles.selectorBtn} {app.paperAutoExecute ? styles.active : ''}"
                            onclick={() => {
                                app.paperAutoExecute = !app.paperAutoExecute;
                                app.savePaperConfig(app.paperInitialUSD, app.paperAllocationPct, app.paperAutoExecute);
                            }}>
                        {app.paperAutoExecute ? 'ON' : 'OFF'}
                    </button>
                </div>
                <p style="font-size: 9px; color: #64748b; margin: 6px 0 0 0;">
                    When enabled, automated AI signals will automatically place paper orders.
                </p>
            </div>

            <div class={styles.settingGroupBox} style="margin-top: 12px;">
                <span class={styles.selectorsLabel}>AI Token Cost Calculator (per 1M tokens)</span>
                <div class={styles.inputRow} style="margin-top: 4px;">
                    <label for="costInput">Input Price $/1M:</label>
                    <input id="costInput" type="number" bind:value={draftCostInputPrice} min="0" step="0.01" />
                </div>
                <div class={styles.inputRow} style="margin-top: 8px;">
                    <label for="costOutput">Output Price $/1M:</label>
                    <input id="costOutput" type="number" bind:value={draftCostOutputPrice} min="0" step="0.01" />
                </div>
                <button class={styles.keySaveBtn} style="margin-top: 8px; width: 100%;"
                        disabled={costSaveStatus === 'saving'} onclick={saveCostConfig}>
                    {costSaveStatus === 'saving' ? 'Saving...' : 'Save Cost Config'}
                </button>
                {#if costSaveStatus === 'success'}
                    <div class="{styles.statusMsg} {styles.successMsg}">Pricing saved.</div>
                {/if}
            </div>

            <div class={styles.settingGroupBox} style="margin-top: 12px;">
                <button class={styles.paperResetBtn} onclick={() => {
                    if (confirm('Reset paper account? This will close any active position and restore initial balance.')) {
                        app.resetPaperAccount();
                    }
                }}>
                    Reset Account Balance
                </button>
            </div>
        </div>

    </div>

    <!-- Timeframe Configuration (collapsible) -->
    <div style="margin-top: 16px;">
        <button class="{styles.selectorBtn} {showTimeframeConfig ? styles.active : ''}"
                style="margin-bottom: 12px;"
                onclick={() => showTimeframeConfig = !showTimeframeConfig}>
            {showTimeframeConfig ? 'Hide' : 'Show'} Timeframe Indicator Configuration
        </button>
    </div>

    {#if showTimeframeConfig}
        {#snippet indicatorInputs(p: string, t: TermDraft)}
            <div class={styles.tfInputRow}><label for={fieldId(p, 'EMA Fast')}>EMA Fast</label><input id={fieldId(p, 'EMA Fast')} type="number" bind:value={t.emaFast} /></div>
            <div class={styles.tfInputRow}><label for={fieldId(p, 'EMA Med')}>EMA Med</label><input id={fieldId(p, 'EMA Med')} type="number" bind:value={t.emaMedium} /></div>
            <div class={styles.tfInputRow}><label for={fieldId(p, 'EMA Slow')}>EMA Slow</label><input id={fieldId(p, 'EMA Slow')} type="number" bind:value={t.emaSlow} /></div>
            <div class={styles.tfInputRow}><label for={fieldId(p, 'EMA Long')}>EMA Long</label><input id={fieldId(p, 'EMA Long')} type="number" bind:value={t.emaLong} /></div>
            <div class={styles.tfInputRow}><label for={fieldId(p, 'RSI Window')}>RSI Window</label><input id={fieldId(p, 'RSI Window')} type="number" bind:value={t.rsiPeriod} /></div>
            <div class={styles.tfInputRow}><label for={fieldId(p, 'MACD Fast')}>MACD Fast</label><input id={fieldId(p, 'MACD Fast')} type="number" bind:value={t.macdFast} /></div>
            <div class={styles.tfInputRow}><label for={fieldId(p, 'MACD Slow')}>MACD Slow</label><input id={fieldId(p, 'MACD Slow')} type="number" bind:value={t.macdSlow} /></div>
            <div class={styles.tfInputRow}><label for={fieldId(p, 'MACD Signal')}>MACD Signal</label><input id={fieldId(p, 'MACD Signal')} type="number" bind:value={t.macdSignal} /></div>
            <div class={styles.tfInputRow}><label for={fieldId(p, 'ADX Period')}>ADX Period</label><input id={fieldId(p, 'ADX Period')} type="number" bind:value={t.adxPeriod} /></div>
            <div class={styles.tfInputRow}><label for={fieldId(p, 'ATR Period')}>ATR Period</label><input id={fieldId(p, 'ATR Period')} type="number" bind:value={t.atrPeriod} /></div>
            <div class={styles.tfInputRow}><label for={fieldId(p, 'Squeeze Wave')}>Squeeze Wave</label><input id={fieldId(p, 'Squeeze Wave')} type="number" bind:value={t.squeezePeriod} /></div>
            <div class={styles.tfInputRow}><label for={fieldId(p, 'BBWP Period')}>BBWP Period</label><input id={fieldId(p, 'BBWP Period')} type="number" bind:value={t.bbwpPeriod} /></div>
            <div class={styles.tfInputRow}><label for={fieldId(p, 'BBWP Lookback')}>BBWP Lookback</label><input id={fieldId(p, 'BBWP Lookback')} type="number" bind:value={t.bbwpLookback} /></div>
            <div class={styles.tfInputRow}><label for={fieldId(p, 'Stoch %K')}>Stoch %K Period</label><input id={fieldId(p, 'Stoch %K')} type="number" bind:value={t.stochKPeriod} /></div>
            <div class={styles.tfInputRow}><label for={fieldId(p, 'Stoch %D')}>Stoch %D Period</label><input id={fieldId(p, 'Stoch %D')} type="number" bind:value={t.stochDPeriod} /></div>
            <div class={styles.tfInputRow}><label for={fieldId(p, 'Stoch Slowing')}>Stoch Slowing</label><input id={fieldId(p, 'Stoch Slowing')} type="number" bind:value={t.stochSPeriod} /></div>
            <div class={styles.tfInputRow}><label for={fieldId(p, 'ChandeMO Period')}>ChandeMO Period</label><input id={fieldId(p, 'ChandeMO Period')} type="number" bind:value={t.chandemoPeriod} /></div>
            <div class={styles.tfInputRow}><label for={fieldId(p, 'Supertrend Period')}>Supertrend Period</label><input id={fieldId(p, 'Supertrend Period')} type="number" bind:value={t.supertrendPeriod} /></div>
            <div class={styles.tfInputRow}><label for={fieldId(p, 'Supertrend Mult')}>Supertrend Mult</label><input id={fieldId(p, 'Supertrend Mult')} type="number" step="0.1" bind:value={t.supertrendMultiplier} /></div>
            <div class={styles.tfInputRow}><label for={fieldId(p, 'Keltner EMA')}>Keltner EMA</label><input id={fieldId(p, 'Keltner EMA')} type="number" bind:value={t.keltnerEmaPeriod} /></div>
            <div class={styles.tfInputRow}><label for={fieldId(p, 'Keltner ATR')}>Keltner ATR</label><input id={fieldId(p, 'Keltner ATR')} type="number" bind:value={t.keltnerAtrPeriod} /></div>
            <div class={styles.tfInputRow}><label for={fieldId(p, 'Keltner Mult')}>Keltner Mult</label><input id={fieldId(p, 'Keltner Mult')} type="number" step="0.1" bind:value={t.keltnerMultiplier} /></div>
            <div class={styles.tfInputRow}><label for={fieldId(p, 'Donchian Period')}>Donchian Period</label><input id={fieldId(p, 'Donchian Period')} type="number" bind:value={t.donchianPeriod} /></div>
            <div class={styles.tfInputRow}><label for={fieldId(p, 'OBV Smoothing')}>OBV Smoothing</label><input id={fieldId(p, 'OBV Smoothing')} type="number" bind:value={t.obvSmoothing} /></div>
            <div class={styles.tfInputRow}><label for={fieldId(p, 'CMF Period')}>CMF Period</label><input id={fieldId(p, 'CMF Period')} type="number" bind:value={t.cmfPeriod} /></div>
            <div class={styles.tfInputRow}><label for={fieldId(p, 'MFI Period')}>MFI Period</label><input id={fieldId(p, 'MFI Period')} type="number" bind:value={t.mfiPeriod} /></div>
            <div class={styles.tfInputRow}><label for={fieldId(p, 'HV Period')}>HV Period</label><input id={fieldId(p, 'HV Period')} type="number" bind:value={t.hvPeriod} /></div>
            <div class={styles.tfInputRow}><label for={fieldId(p, 'Aroon Period')}>Aroon Period</label><input id={fieldId(p, 'Aroon Period')} type="number" bind:value={t.aroonPeriod} /></div>
            <div class={styles.tfInputRow}><label for={fieldId(p, 'Chop Period')}>Chop Period</label><input id={fieldId(p, 'Chop Period')} type="number" bind:value={t.chopPeriod} /></div>
            <div class={styles.tfInputRow}><label for={fieldId(p, 'LinReg Period')}>LinReg Period</label><input id={fieldId(p, 'LinReg Period')} type="number" bind:value={t.linregPeriod} /></div>
            <div class={styles.tfInputRow}><label for={fieldId(p, 'ZScore Period')}>ZScore Period</label><input id={fieldId(p, 'ZScore Period')} type="number" bind:value={t.zscorePeriod} /></div>
            <hr class={styles.sectionDivider} />
            <div class={styles.tfInputRow}><label for={fieldId(p, 'MACD Extr High')}>MACD Extr High</label><input id={fieldId(p, 'MACD Extr High')} type="number" step="0.01" bind:value={t.macdExtremeHigh} /></div>
            <div class={styles.tfInputRow}><label for={fieldId(p, 'MACD Extr Low')}>MACD Extr Low</label><input id={fieldId(p, 'MACD Extr Low')} type="number" step="0.01" bind:value={t.macdExtremeLow} /></div>
            <div class={styles.tfInputRow}><label for={fieldId(p, 'MACD Contr %')}>MACD Contr %</label><input id={fieldId(p, 'MACD Contr %')} type="number" step="0.01" min="0.05" max="0.95" bind:value={t.macdContraction} /></div>
            <div class={styles.tfInputRow}><label for={fieldId(p, 'ADX Trend Th')}>ADX Trend Th</label><input id={fieldId(p, 'ADX Trend Th')} type="number" bind:value={t.adxTrendThreshold} /></div>
            <div class={styles.tfInputRow}><label for={fieldId(p, 'ADX Exhaustion')}>ADX Exhaustion</label><input id={fieldId(p, 'ADX Exhaustion')} type="number" bind:value={t.adxExhaustionThreshold} /></div>
            <div class={styles.tfInputRow}><label for={fieldId(p, 'ADX Slope Lbk')}>ADX Slope Lbk</label><input id={fieldId(p, 'ADX Slope Lbk')} type="number" bind:value={t.adxSlopeLookback} /></div>
            <hr class={styles.sectionDivider} />
            <div class={styles.tfInputRow}><label for={fieldId(p, 'Sqz Min Dur')}>Sqz Min Dur</label><input id={fieldId(p, 'Sqz Min Dur')} type="number" bind:value={t.squeezeMinDuration} /></div>
            <div class={styles.tfInputRow}><label for={fieldId(p, 'Sqz BB Period')}>Sqz BB Period</label><input id={fieldId(p, 'Sqz BB Period')} type="number" bind:value={t.squeezeBbPeriod} /></div>
            <div class={styles.tfInputRow}><label for={fieldId(p, 'Sqz BB Std Dev')}>Sqz BB Std Dev</label><input id={fieldId(p, 'Sqz BB Std Dev')} type="number" step="0.1" bind:value={t.squeezeBbStdDev} /></div>
            <div class={styles.tfInputRow}><label for={fieldId(p, 'Sqz KC Period')}>Sqz KC Period</label><input id={fieldId(p, 'Sqz KC Period')} type="number" bind:value={t.squeezeKcPeriod} /></div>
            <div class={styles.tfInputRow}><label for={fieldId(p, 'Sqz KC ATR Mult')}>Sqz KC ATR Mult</label><input id={fieldId(p, 'Sqz KC ATR Mult')} type="number" step="0.1" bind:value={t.squeezeKcAtrMult} /></div>
            <hr class={styles.sectionDivider} />
            <div class={styles.tfInputRow}><label for={fieldId(p, 'ATR Mult')}>ATR Mult</label><input id={fieldId(p, 'ATR Mult')} type="number" step="0.1" bind:value={t.atrMultiplier} /></div>
            <div class={styles.tfInputRow}><label for={fieldId(p, 'Target R:R')}>Target R:R</label><input id={fieldId(p, 'Target R:R')} type="number" step="0.1" bind:value={t.atrTargetRR} /></div>
            <div class={styles.tfInputRow}><label for={fieldId(p, 'Vol Avg Period')}>Vol Avg Period</label><input id={fieldId(p, 'Vol Avg Period')} type="number" bind:value={t.volumeAvgPeriod} /></div>
            <div class={styles.tfInputRow}><label for={fieldId(p, 'RVOL Inst')}>RVOL Inst</label><input id={fieldId(p, 'RVOL Inst')} type="number" step="0.1" bind:value={t.rvolInstitutional} /></div>
            <div class={styles.tfInputRow}><label for={fieldId(p, 'RVOL Climax')}>RVOL Climax</label><input id={fieldId(p, 'RVOL Climax')} type="number" step="0.1" bind:value={t.rvolClimax} /></div>
            <div class={styles.tfInputRow}><label for={fieldId(p, 'Analysis Limit')}>Analysis Limit</label><input id={fieldId(p, 'Analysis Limit')} type="number" min="10" max="500" step="5" bind:value={t.analysisLimit} /></div>
        {/snippet}

        <div class={styles.tfCardsGrid}>
            <div class={styles.tfCard}>
                <h3 class={styles.tfCardTitle}>Micro Term</h3>
                <div class={styles.tfRow}>
                    <select class={styles.tfSelect}
                        value={selectedOption(tfDraft.micro.durationSeconds)}
                        onchange={(e) => { const v = parseInt(e.currentTarget.value); if (v > 0) tfDraft.micro.durationSeconds = v; }}>
                        <option value={-1} disabled>Custom: {durationLabel(tfDraft.micro.durationSeconds)}</option>
                        {#each TIMEFRAME_OPTIONS as opt}
                            <option value={opt.seconds}>{opt.label}</option>
                        {/each}
                    </select>
                </div>
                <div class={styles.tfInputScroll}>
                    {@render indicatorInputs('micro', tfDraft.micro)}
                </div>
            </div>
            <div class={styles.tfCard}>
                <h3 class={styles.tfCardTitle}>Fast Term</h3>
                <div class={styles.tfRow}>
                    <select class={styles.tfSelect}
                        value={selectedOption(tfDraft.fast.durationSeconds)}
                        onchange={(e) => { const v = parseInt(e.currentTarget.value); if (v > 0) tfDraft.fast.durationSeconds = v; }}>
                        <option value={-1} disabled>Custom: {durationLabel(tfDraft.fast.durationSeconds)}</option>
                        {#each TIMEFRAME_OPTIONS as opt}
                            <option value={opt.seconds}>{opt.label}</option>
                        {/each}
                    </select>
                </div>
                <div class={styles.tfInputScroll}>
                    {@render indicatorInputs('small', tfDraft.fast)}
                </div>
            </div>
            <div class={styles.tfCard}>
                <h3 class={styles.tfCardTitle}>Slow Term</h3>
                <div class={styles.tfRow}>
                    <select class={styles.tfSelect}
                        value={selectedOption(tfDraft.slow.durationSeconds)}
                        onchange={(e) => { const v = parseInt(e.currentTarget.value); if (v > 0) tfDraft.slow.durationSeconds = v; }}>
                        <option value={-1} disabled>Custom: {durationLabel(tfDraft.slow.durationSeconds)}</option>
                        {#each TIMEFRAME_OPTIONS as opt}
                            <option value={opt.seconds}>{opt.label}</option>
                        {/each}
                    </select>
                </div>
                <div class={styles.tfInputScroll}>
                    {@render indicatorInputs('medium', tfDraft.slow)}
                </div>
            </div>
            <div class={styles.tfCard}>
                <h3 class={styles.tfCardTitle}>Macro Term</h3>
                <div class={styles.tfRow}>
                    <select class={styles.tfSelect}
                        value={selectedOption(tfDraft.macro.durationSeconds)}
                        onchange={(e) => { const v = parseInt(e.currentTarget.value); if (v > 0) tfDraft.macro.durationSeconds = v; }}>
                        <option value={-1} disabled>Custom: {durationLabel(tfDraft.macro.durationSeconds)}</option>
                        {#each TIMEFRAME_OPTIONS as opt}
                            <option value={opt.seconds}>{opt.label}</option>
                        {/each}
                    </select>
                </div>
                <div class={styles.tfInputScroll}>
                    {@render indicatorInputs('large', tfDraft.macro)}
                </div>
            </div>
        </div>
    {/if}
</div>
