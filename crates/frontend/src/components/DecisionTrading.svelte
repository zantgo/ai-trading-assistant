<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import styles from './DecisionTrading.module.css';
    import MomentumMeter from './MomentumMeter.svelte';
    import type { DecisionProfile, IndicatorRule } from '../types';

    const app = useAppStore();
    const activePair = $derived(app.instancesMap[app.activeTab]);
    const microInd = $derived(activePair?.microTerm?.indicators ?? {});
    let showNewIndicator = $state(false);
    let newIndicatorName = $state('');
    let newIndicatorWeight = $state(10);

    let eightFactorScore = $state(0);
    let eightFactorMax = $state(90);
    let eightFactorSignals = $state<Record<string, { passed: boolean; points: number; maxPoints: number; weight: number }>>({});
    let computedAllocPct = $state(1);

    const FACTOR_WEIGHTS: Record<string, number> = {
        'RSI (Oversold/Overbought)': 12,
        'MACD Crossover': 12,
        'Divergence': 10,
        'Support Level': 12,
        'Resistance Level': 8,
        '15m Trend': 12,
        '200 EMA Position': 12,
        'Chart Pattern': 12,
    };

    $effect(() => {
        app.fetchDecisionProfiles();
    });

    function getActiveProfile(): DecisionProfile | undefined {
        return app.decisionProfiles.find(p => p.id === app.activeDecisionProfileId);
    }

    function computeEightFactorScore() {
        const snap = app.latestSnapshot || {};
        const price = snap.mid_price ? parseFloat(String(snap.mid_price)) : 0;
        const rsi = snap.rsi_14 ? parseFloat(String(snap.rsi_14)) : 50;
        const macdLine = snap.macd_line ? parseFloat(String(snap.macd_line)) : 0;
        const macdSignal = snap.macd_signal ? parseFloat(String(snap.macd_signal)) : 0;
        const macdHist = snap.macd_hist ? parseFloat(String(snap.macd_hist)) : 0;
        const squeezeOn = snap.squeeze_on ?? false;
        const squeezeMom = snap.squeeze_momentum ? parseFloat(String(snap.squeeze_momentum)) : 0;
        const emaLong = snap.ema_long ? parseFloat(String(snap.ema_long)) : price;

        const supports = (app.markedSupportLevels.length > 0 ? app.markedSupportLevels : [price * 0.99, price * 0.98]);
        const resistances = (app.markedResistanceLevels.length > 0 ? app.markedResistanceLevels : [price * 1.01, price * 1.02]);

        const isBullish = app.currentPosition !== 'Short';

        const rsiAligned = isBullish ? rsi < 30 : rsi > 70;
        const macdAligned = isBullish ? macdLine > macdSignal : macdLine < macdSignal;
        const divergence = isBullish ? (rsi < 40 || macdHist > 0) : (rsi > 60 || macdHist < 0);
        const supportAligned = supports.some(s => Math.abs(price - s) < s * 0.005);
        const resistanceAligned = resistances.some(r => Math.abs(price - r) < r * 0.005);
        const trendAligned = isBullish;
        const ema200Aligned = isBullish ? price > emaLong : price < emaLong;
        const patternAligned = squeezeOn && (isBullish ? squeezeMom > 0 : squeezeMom < 0);

        const factorResults: Record<string, boolean> = {
            'RSI (Oversold/Overbought)': rsiAligned,
            'MACD Crossover': macdAligned,
            'Divergence': divergence,
            'Support Level': supportAligned,
            'Resistance Level': resistanceAligned,
            '15m Trend': trendAligned,
            '200 EMA Position': ema200Aligned,
            'Chart Pattern': patternAligned,
        };

        eightFactorSignals = {};
        eightFactorScore = 0;
        for (const [name, passed] of Object.entries(factorResults)) {
            const weight = FACTOR_WEIGHTS[name] ?? 10;
            eightFactorSignals[name] = { passed, points: passed ? weight : 0, maxPoints: weight, weight };
            if (passed) eightFactorScore += weight;
        }

        if (eightFactorScore >= 60) computedAllocPct = 3;
        else if (eightFactorScore >= 40) computedAllocPct = 2;
        else computedAllocPct = 1;

        app.totalPointsScore = eightFactorScore;
        app.allocatedCapitalPct = computedAllocPct;
        app.markedSupportLevels = supports;
        app.markedResistanceLevels = resistances;
    }

    async function handleEvaluate() {
        await app.evaluateDecision(app.activeDecisionProfileId);
        computeEightFactorScore();
    }

    async function addNewIndicator() {
        if (!newIndicatorName.trim()) return;
        await app.addProfileIndicator(app.activeDecisionProfileId, newIndicatorName.trim(), newIndicatorWeight, 'NONE');
        newIndicatorName = '';
        newIndicatorWeight = 10;
        showNewIndicator = false;
    }

    async function handleIndicatorOverride(ind: IndicatorRule, newOverride: string) {
        await app.updateProfileIndicator(ind.profile_id, ind.id, ind.weight, newOverride);
    }

    async function handleIndicatorWeightChange(ind: IndicatorRule, newWeight: number) {
        await app.updateProfileIndicator(ind.profile_id, ind.id, newWeight, ind.override_status);
    }

    async function handleThresholdChange(profile: DecisionProfile, field: 'long' | 'short', delta: number) {
        const longT = field === 'long' ? profile.long_threshold + delta : profile.long_threshold;
        const shortT = field === 'short' ? profile.short_threshold + delta : profile.short_threshold;
        await app.updateDecisionProfileThresholds(profile.id, longT, shortT);
    }

    let newProfileName = $state('');
    async function createProfile() {
        if (!newProfileName.trim()) return;
        await app.createDecisionProfile(newProfileName.trim(), 40, -40);
        newProfileName = '';
    }
</script>

<div class={styles.dtLayout}>
    <!-- Left: Profiles column -->
    <div class={styles.dtSidebar}>
        <div class={styles.dtCard}>
            <h3 class={styles.dtCardTitle}>PROFILES</h3>
            <div class={styles.dtProfileList}>
                {#each app.decisionProfiles as profile (profile.id)}
                    <button class="{styles.dtProfileBtn} {profile.id === app.activeDecisionProfileId ? styles.active : ''}"
                        onclick={() => app.activeDecisionProfileId = profile.id}
                    >
                        <span>{profile.profile_name}</span>
                        {#if app.decisionProfiles.length > 1}
                            <span class={styles.dtDeleteIcon} role="button" tabindex="0" onclick={(e) => { e.stopPropagation(); app.deleteDecisionProfile(profile.id); }} onkeydown={(e) => { if (e.key === 'Enter') { e.stopPropagation(); app.deleteDecisionProfile(profile.id); } }}>×</span>
                        {/if}
                    </button>
                {/each}
            </div>
            <div class={styles.dtAddProfile}>
                <input type="text" class={styles.dtInput} placeholder="New profile name..." bind:value={newProfileName}
                    onkeydown={(e) => { if (e.key === 'Enter') createProfile(); }} />
                <button class={styles.dtAddBtn} onclick={createProfile}>+</button>
            </div>
        </div>
    </div>

    <!-- Right: Profile config -->
    {#if getActiveProfile()}
        {@const profile = getActiveProfile()!}
        <div class={styles.dtMain}>
            <!-- Calculated Result Card -->
            <div class="{styles.dtCard} {styles.dtResultCard}">
                <div class={styles.dtResultHeader}>
                    <h3 class={styles.dtCardTitle}>CALCULATED RESULT</h3>
                    <span class={styles.dtScoreBadge}>{app.calculatedDecisionScore?.score ?? '--'}</span>
                </div>
                <div class={styles.dtResultDisplay}>
                    <div class="{styles.dtRecommendation} {app.calculatedDecisionScore?.recommendation === 'BUY' ? styles.dtRecBuy : app.calculatedDecisionScore?.recommendation === 'SELL' ? styles.dtRecSell : styles.dtRecWait}">
                        {app.calculatedDecisionScore?.recommendation || 'WAIT'}
                    </div>
                    <div class={styles.dtMomentumSlider}>
                        <span class={styles.dtSliderLabel}>SHORT</span>
                        <div class={styles.dtSliderTrack}>
                            <div class={styles.dtSliderFill} style="left: 50%; width: {Math.abs(app.calculatedDecisionScore?.momentum_bias ?? 0) / 80 * 100}%;
                                background: {(app.calculatedDecisionScore?.momentum_bias ?? 0) >= 0 ? '#10b981' : '#ef4444'};
                                {(app.calculatedDecisionScore?.momentum_bias ?? 0) >= 0 ? 'border-radius: 0 999px 999px 0;' : 'border-radius: 999px 0 0 999px; left: ' + (50 + Math.min(0, (app.calculatedDecisionScore?.momentum_bias ?? 0)) / 80 * 100) + '%;'}">
                            </div>
                            <div class={styles.dtSliderPointer} style="left: {50 + (app.calculatedDecisionScore?.momentum_bias ?? 0) / 80 * 100}%"></div>
                        </div>
                        <span class={styles.dtSliderLabel}>LONG</span>
                    </div>
                </div>
                <button class={styles.dtEvalBtn} onclick={handleEvaluate} disabled={app.decisionLoading}>
                    {app.decisionLoading ? 'Evaluating...' : 'Evaluate Decision'}
                </button>
            </div>

            <!-- Continuous Momentum Meters (RSI / MACD / Squeeze) -->
            <div class={styles.dtCard}>
                <h3 class={styles.dtCardTitle}>MOMENTUM METERS</h3>
                <MomentumMeter label="RSI" normalized={microInd['rsi']?.normalized ?? 0} stateLabel={microInd['rsi']?.state_label ?? 'UNKNOWN'} />
                <MomentumMeter label="MACD" normalized={microInd['macd']?.normalized ?? 0} stateLabel={microInd['macd']?.state_label ?? 'UNKNOWN'} />
                <MomentumMeter label="SQUEEZE" normalized={microInd['squeeze']?.normalized ?? 0} stateLabel={microInd['squeeze']?.state_label ?? 'UNKNOWN'} />
            </div>

            <!-- 8-Factor Score & Capital Allocation -->
            <div class={styles.dtCard}>
                <div class={styles.dtEightHeader}>
                    <h3 class={styles.dtCardTitle}>8-FACTOR WEIGHTED SCORING</h3>
                    <div class={styles.dtEightBadges}>
                        <span class={styles.dtScoreBadge}>Score: {eightFactorScore} / {eightFactorMax}</span>
                        <span class={styles.dtAllocBadge}>Capital to Use: {computedAllocPct}%</span>
                    </div>
                </div>
                <div class={styles.dtChecklist}>
                    {#each Object.entries(eightFactorSignals) as [name, factor] (name)}
                        <div class="{styles.dtCheckItem} {factor.passed ? styles.dtCheckPass : styles.dtCheckFail}">
                            <span class={styles.dtCheckIcon}>{factor.passed ? '✓' : '✗'}</span>
                            <span class={styles.dtCheckLabel}>{name}</span>
                            <span class={styles.dtCheckWeight}>{factor.points}/{factor.maxPoints}</span>
                        </div>
                    {/each}
                </div>
            </div>

            <!-- Threshold Config -->
            <div class={styles.dtCard}>
                <div class={styles.dtProfileHeader}>
                    <h3 class={styles.dtCardTitle}>PROFILE NAME: {profile.profile_name}</h3>
                </div>
                <div class={styles.dtThresholds}>
                    <div class={styles.dtThresholdRow}>
                        <span class={styles.dtThLabel}>LONG THRESHOLD</span>
                        <div class={styles.dtStepper}>
                            <button class={styles.dtStepBtn} onclick={() => handleThresholdChange(profile, 'long', -5)}>−</button>
                            <span class={styles.dtStepVal}>{profile.long_threshold}</span>
                            <button class={styles.dtStepBtn} onclick={() => handleThresholdChange(profile, 'long', 5)}>+</button>
                        </div>
                    </div>
                    <div class={styles.dtThresholdRow}>
                        <span class={styles.dtThLabel}>SHORT THRESHOLD</span>
                        <div class={styles.dtStepper}>
                            <button class={styles.dtStepBtn} onclick={() => handleThresholdChange(profile, 'short', 5)}>−</button>
                            <span class={styles.dtStepVal}>{profile.short_threshold}</span>
                            <button class={styles.dtStepBtn} onclick={() => handleThresholdChange(profile, 'short', -5)}>+</button>
                        </div>
                    </div>
                </div>
            </div>

            <!-- Indicators List -->
            <div class={styles.dtCard}>
                <div class={styles.dtCardTitleRow}>
                    <h3 class={styles.dtCardTitle}>INDICATORS</h3>
                    <button class={styles.dtAddBtn} onclick={() => showNewIndicator = !showNewIndicator}>+ NEW INDICATOR</button>
                </div>

                {#if showNewIndicator}
                    <div class={styles.dtNewIndicator}>
                        <input type="text" class={styles.dtInput} placeholder="Indicator name..." bind:value={newIndicatorName} />
                        <input type="number" class="{styles.dtInput} {styles.dtInputSmall}" placeholder="Weight" bind:value={newIndicatorWeight} min="1" max="100" />
                        <button class={styles.dtSaveBtn} onclick={addNewIndicator} disabled={!newIndicatorName.trim()}>Add</button>
                    </div>
                {/if}

                <div class={styles.dtIndicatorList}>
                    {#each profile.indicators as ind (ind.id)}
                        <div class={styles.dtIndicatorRow}>
                            <div class={styles.dtIndInfo}>
                                <span class={styles.dtIndName}>{ind.indicator_name}</span>
                                <span class={styles.dtIndBadge}>~{ind.weight}</span>
                            </div>
                            <div class={styles.dtIndControls}>
                                <input type="number" class={styles.dtIndWeight} value={ind.weight}
                                    min="1" max="100"
                                    onchange={(e) => handleIndicatorWeightChange(ind, parseInt((e.target as HTMLInputElement).value) || 10)} />
                                <select class={styles.dtIndOverride} value={ind.override_status}
                                    onchange={(e) => handleIndicatorOverride(ind, (e.target as HTMLSelectElement).value)}>
                                    <option value="NONE">Auto</option>
                                    <option value="BULLISH">Bullish</option>
                                    <option value="BEARISH">Bearish</option>
                                </select>
                                <button class={styles.dtIndDelete} onclick={() => app.deleteProfileIndicator(ind.profile_id, ind.id)}>×</button>
                            </div>
                        </div>
                    {/each}
                </div>
            </div>
        </div>
    {/if}
</div>
