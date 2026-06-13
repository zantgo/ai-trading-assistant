<script lang="ts">
    import { getState } from '../state.svelte';
    import type { DecisionProfile, IndicatorRule } from '../state.svelte';

    const app = getState();
    let showNewIndicator = $state(false);
    let newIndicatorName = $state('');
    let newIndicatorWeight = $state(10);

    $effect(() => {
        app.fetchDecisionProfiles();
    });

    function getActiveProfile(): DecisionProfile | undefined {
        return app.decisionProfiles.find(p => p.id === app.activeDecisionProfileId);
    }

    async function handleEvaluate() {
        const snap = app.latestSnapshot || {};
        await app.evaluateDecision(app.activeDecisionProfileId, snap, app.historyPrices);
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
        await app.updateDecisionProfile(profile.id, profile.profile_name, longT, shortT);
    }

    let newProfileName = $state('');
    async function createProfile() {
        if (!newProfileName.trim()) return;
        await app.createDecisionProfile(newProfileName.trim(), 40, -40);
        newProfileName = '';
    }
</script>

<div class="dt-layout">
    <!-- Left: Profiles column -->
    <div class="dt-sidebar">
        <div class="dt-card">
            <h3 class="dt-card-title">PROFILES</h3>
            <div class="dt-profile-list">
                {#each app.decisionProfiles as profile (profile.id)}
                    <button class="dt-profile-btn"
                        class:active={profile.id === app.activeDecisionProfileId}
                        onclick={() => app.activeDecisionProfileId = profile.id}
                    >
                        <span>{profile.profile_name}</span>
                        {#if app.decisionProfiles.length > 1}
                            <span class="dt-delete-icon" onclick={(e) => { e.stopPropagation(); app.deleteDecisionProfile(profile.id); }}>×</span>
                        {/if}
                    </button>
                {/each}
            </div>
            <div class="dt-add-profile">
                <input type="text" class="dt-input" placeholder="New profile name..." bind:value={newProfileName}
                    onkeydown={(e) => { if (e.key === 'Enter') createProfile(); }} />
                <button class="dt-add-btn" onclick={createProfile}>+</button>
            </div>
        </div>
    </div>

    <!-- Right: Profile config -->
    {#if getActiveProfile()}
        {@const profile = getActiveProfile()!}
        <div class="dt-main">
            <!-- Calculated Result Card -->
            <div class="dt-card dt-result-card">
                <div class="dt-result-header">
                    <h3 class="dt-card-title">CALCULATED RESULT</h3>
                    <span class="dt-score-badge">{app.calculatedDecisionScore?.score ?? '--'}</span>
                </div>
                <div class="dt-result-display">
                    <div class="dt-recommendation" class:dt-rec-buy={app.calculatedDecisionScore?.recommendation === 'BUY'}
                        class:dt-rec-sell={app.calculatedDecisionScore?.recommendation === 'SELL'}
                        class:dt-rec-wait={app.calculatedDecisionScore?.recommendation === 'WAIT'}
                    >
                        {app.calculatedDecisionScore?.recommendation || 'WAIT'}
                    </div>
                    <div class="dt-momentum-slider">
                        <span class="dt-slider-label">SHORT</span>
                        <div class="dt-slider-track">
                            <div class="dt-slider-fill" style="left: 50%; width: {Math.abs(app.calculatedDecisionScore?.momentum_bias ?? 0) / 80 * 100}%;
                                background: {(app.calculatedDecisionScore?.momentum_bias ?? 0) >= 0 ? '#10b981' : '#ef4444'};
                                {(app.calculatedDecisionScore?.momentum_bias ?? 0) >= 0 ? 'border-radius: 0 999px 999px 0;' : 'border-radius: 999px 0 0 999px; left: ' + (50 + Math.min(0, (app.calculatedDecisionScore?.momentum_bias ?? 0)) / 80 * 100) + '%;'}">
                            </div>
                            <div class="dt-slider-pointer" style="left: {50 + (app.calculatedDecisionScore?.momentum_bias ?? 0) / 80 * 100}%"></div>
                        </div>
                        <span class="dt-slider-label">LONG</span>
                    </div>
                </div>
                <button class="dt-eval-btn" onclick={handleEvaluate} disabled={app.decisionLoading}>
                    {app.decisionLoading ? 'Evaluating...' : 'Evaluate Decision'}
                </button>
            </div>

            <!-- Threshold Config -->
            <div class="dt-card">
                <div class="dt-profile-header">
                    <h3 class="dt-card-title">PROFILE NAME: {profile.profile_name}</h3>
                </div>
                <div class="dt-thresholds">
                    <div class="dt-threshold-row">
                        <span class="dt-th-label">LONG THRESHOLD</span>
                        <div class="dt-stepper">
                            <button class="dt-step-btn" onclick={() => handleThresholdChange(profile, 'long', -5)}>−</button>
                            <span class="dt-step-val">{profile.long_threshold}</span>
                            <button class="dt-step-btn" onclick={() => handleThresholdChange(profile, 'long', 5)}>+</button>
                        </div>
                    </div>
                    <div class="dt-threshold-row">
                        <span class="dt-th-label">SHORT THRESHOLD</span>
                        <div class="dt-stepper">
                            <button class="dt-step-btn" onclick={() => handleThresholdChange(profile, 'short', 5)}>−</button>
                            <span class="dt-step-val">{profile.short_threshold}</span>
                            <button class="dt-step-btn" onclick={() => handleThresholdChange(profile, 'short', -5)}>+</button>
                        </div>
                    </div>
                </div>
            </div>

            <!-- Indicators List -->
            <div class="dt-card">
                <div class="dt-card-title-row">
                    <h3 class="dt-card-title">INDICATORS</h3>
                    <button class="dt-add-btn" onclick={() => showNewIndicator = !showNewIndicator}>+ NEW INDICATOR</button>
                </div>

                {#if showNewIndicator}
                    <div class="dt-new-indicator">
                        <input type="text" class="dt-input" placeholder="Indicator name..." bind:value={newIndicatorName} />
                        <input type="number" class="dt-input dt-input-small" placeholder="Weight" bind:value={newIndicatorWeight} min="1" max="100" />
                        <button class="dt-save-btn" onclick={addNewIndicator} disabled={!newIndicatorName.trim()}>Add</button>
                    </div>
                {/if}

                <div class="dt-indicator-list">
                    {#each profile.indicators as ind (ind.id)}
                        <div class="dt-indicator-row">
                            <div class="dt-ind-info">
                                <span class="dt-ind-name">{ind.indicator_name}</span>
                                <span class="dt-ind-badge">~{ind.weight}</span>
                            </div>
                            <div class="dt-ind-controls">
                                <input type="number" class="dt-ind-weight" value={ind.weight}
                                    min="1" max="100"
                                    onchange={(e) => handleIndicatorWeightChange(ind, parseInt((e.target as HTMLInputElement).value) || 10)} />
                                <select class="dt-ind-override" value={ind.override_status}
                                    onchange={(e) => handleIndicatorOverride(ind, (e.target as HTMLSelectElement).value)}>
                                    <option value="NONE">Auto</option>
                                    <option value="BULLISH">Bullish</option>
                                    <option value="BEARISH">Bearish</option>
                                </select>
                                <button class="dt-ind-delete" onclick={() => app.deleteProfileIndicator(ind.profile_id, ind.id)}>×</button>
                            </div>
                        </div>
                    {/each}
                </div>
            </div>
        </div>
    {/if}
</div>

<style>
    .dt-layout { display: grid; grid-template-columns: 240px 1fr; gap: 16px; max-width: 1100px; margin: 0 auto; width: 100%; padding: 16px; box-sizing: border-box; }
    .dt-sidebar { display: flex; flex-direction: column; }
    .dt-main { display: flex; flex-direction: column; gap: 16px; }
    .dt-card { background: #131722; border: 1px solid #2a2e39; border-radius: 8px; padding: 16px; }
    .dt-card-title { font-size: 11px; font-weight: 700; color: #64748b; text-transform: uppercase; letter-spacing: 0.05em; margin: 0 0 10px 0; }
    .dt-card-title-row { display: flex; justify-content: space-between; align-items: center; margin-bottom: 10px; }
    .dt-card-title-row .dt-card-title { margin: 0; }
    .dt-profile-list { display: flex; flex-direction: column; gap: 4px; margin-bottom: 8px; }
    .dt-profile-btn {
        background: #0f131c; border: 1px solid #1e293b; color: #94a3b8; padding: 8px 10px; border-radius: 4px;
        font-size: 11px; cursor: pointer; text-align: left; display: flex; justify-content: space-between; align-items: center;
    }
    .dt-profile-btn.active { border-color: #3b82f6; color: #3b82f6; background: rgba(59,130,246,0.08); }
    .dt-profile-btn:hover { border-color: #3b82f6; }
    .dt-delete-icon { color: #ef4444; font-weight: bold; font-size: 14px; padding: 0 4px; }
    .dt-delete-icon:hover { color: #f87171; }
    .dt-add-profile { display: flex; gap: 4px; }
    .dt-input {
        flex: 1; background: #0f131c; border: 1px solid #2a2e39; color: #e2e8f0; padding: 6px 8px;
        border-radius: 4px; font-size: 11px; outline: none;
    }
    .dt-input:focus { border-color: #3b82f6; }
    .dt-input-small { width: 70px; flex: 0 0 70px; }
    .dt-add-btn {
        background: #1e40af; border: 1px solid #3b82f6; color: #f1f5f9; padding: 6px 12px;
        border-radius: 4px; font-size: 10px; font-weight: 700; cursor: pointer; white-space: nowrap;
    }
    .dt-add-btn:hover { background: #1e3a8a; }
    .dt-save-btn {
        background: #10b981; border: 1px solid #10b981; color: #fff; padding: 6px 12px;
        border-radius: 4px; font-size: 10px; font-weight: 700; cursor: pointer;
    }
    .dt-save-btn:disabled { opacity: 0.5; }

    .dt-result-card { }
    .dt-result-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px; }
    .dt-score-badge {
        font-size: 16px; font-weight: 800; color: #f1f5f9; background: #1e293b;
        padding: 4px 12px; border-radius: 4px; font-family: monospace;
    }
    .dt-result-display { display: flex; flex-direction: column; gap: 12px; margin-bottom: 12px; }
    .dt-recommendation {
        font-size: 36px; font-weight: 800; text-align: center; padding: 16px; border-radius: 8px;
        background: rgba(251,191,36,0.08); border: 1px solid rgba(251,191,36,0.25); color: #f59e0b;
    }
    .dt-rec-buy { background: rgba(16,185,129,0.08); border-color: rgba(16,185,129,0.25); color: #10b981; }
    .dt-rec-sell { background: rgba(239,68,68,0.08); border-color: rgba(239,68,68,0.25); color: #ef4444; }
    .dt-rec-wait { background: rgba(251,191,36,0.08); border-color: rgba(251,191,36,0.25); color: #f59e0b; }
    .dt-momentum-slider { display: flex; align-items: center; gap: 8px; }
    .dt-slider-label { font-size: 9px; font-weight: 700; color: #64748b; text-transform: uppercase; }
    .dt-slider-track { flex: 1; height: 8px; background: #1e293b; border-radius: 4px; position: relative; overflow: visible; }
    .dt-slider-fill { position: absolute; top: 0; height: 100%; width: 50%; background: #3b82f6; border-radius: 0 999px 999px 0; }
    .dt-slider-pointer {
        position: absolute; top: -4px; width: 16px; height: 16px; background: #f1f5f9;
        border: 2px solid #3b82f6; border-radius: 50%; transform: translateX(-50%);
        box-shadow: 0 0 8px rgba(59,130,246,0.4);
    }

    .dt-eval-btn {
        width: 100%; padding: 10px; background: linear-gradient(135deg, #1e40af, #3b82f6);
        border: 1px solid #3b82f6; color: #f1f5f9; font-size: 11px; font-weight: 700; text-transform: uppercase;
        border-radius: 6px; cursor: pointer;
    }
    .dt-eval-btn:disabled { opacity: 0.5; cursor: not-allowed; }
    .dt-profile-header { display: flex; align-items: center; gap: 8px; margin-bottom: 10px; }
    .dt-thresholds { display: flex; flex-direction: column; gap: 10px; }
    .dt-threshold-row { display: flex; justify-content: space-between; align-items: center; }
    .dt-th-label { font-size: 11px; font-weight: 600; color: #94a3b8; }
    .dt-stepper { display: flex; align-items: center; gap: 8px; }
    .dt-step-btn {
        width: 28px; height: 28px; background: #0f131c; border: 1px solid #2a2e39; color: #94a3b8;
        font-size: 14px; font-weight: 700; border-radius: 4px; cursor: pointer; display: flex; align-items: center; justify-content: center;
    }
    .dt-step-btn:hover { border-color: #3b82f6; color: #3b82f6; }
    .dt-step-val {
        font-size: 18px; font-weight: 800; color: #f1f5f9; font-family: monospace; min-width: 40px; text-align: center;
    }

    .dt-new-indicator { display: flex; gap: 6px; margin-bottom: 10px; }
    .dt-indicator-list { display: flex; flex-direction: column; gap: 6px; }
    .dt-indicator-row {
        display: flex; justify-content: space-between; align-items: center;
        padding: 8px 10px; background: #0f131c; border: 1px solid #1e293b; border-radius: 4px;
    }
    .dt-ind-info { display: flex; align-items: center; gap: 8px; }
    .dt-ind-name { font-size: 11px; color: #cbd5e1; font-weight: 600; }
    .dt-ind-badge { font-size: 9px; background: rgba(59,130,246,0.12); color: #60a5fa; padding: 2px 6px; border-radius: 3px; font-weight: 700; }
    .dt-ind-controls { display: flex; align-items: center; gap: 6px; }
    .dt-ind-weight {
        width: 50px; background: #171b26; border: 1px solid #2a2e39; color: #cbd5e1;
        padding: 3px 6px; border-radius: 3px; font-size: 10px; text-align: center; outline: none;
    }
    .dt-ind-weight:focus { border-color: #3b82f6; }
    .dt-ind-override {
        background: #171b26; border: 1px solid #2a2e39; color: #cbd5e1;
        padding: 3px 6px; border-radius: 3px; font-size: 10px; outline: none; cursor: pointer;
    }
    .dt-ind-delete { background: none; border: none; color: #ef4444; font-size: 14px; cursor: pointer; padding: 0 4px; }

    @media (max-width: 768px) {
        .dt-layout { grid-template-columns: 1fr; }
    }
</style>
