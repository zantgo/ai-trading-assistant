<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import { fmtPrice } from '../lib/telemetry';
    import type { RiskProfile } from '../types';
    import styles from './RiskManagementPanel.module.css';

    const app = useAppStore();
    let { pairKey }: { pairKey: string } = $props();

    const pair = $derived(app.instancesMap[pairKey]);

    // Sub-tab state
    let activeTab = $state<'calculator' | 'risk_profile'>('calculator');

    // ─── Live ATR value from pair's telemetry ───
    const liveAtr = $derived.by(() => {
        const p = pair as any;
        const snap = p?.microTerm?.latestSnapshot;
        if (!snap) return null;
        const atr14 = snap.atr_14;
        if (atr14 == null) return null;
        const val = parseFloat(String(atr14));
        return isNaN(val) ? null : val;
    });

    // ATR-based stop state (local to this panel)
    let useAtrStop = $state(false);
    let atrMultiplier = $state(2.0);
    let maxDailyLoss = $state('');
    let maxAllocationPct = $state('');

    // Reference price for formatting
    const refPrice = $derived(parseFloat(app.riskEntryPrice) || 0);

    function getActiveProfile(): RiskProfile | undefined {
        return app.riskProfiles.find(p => p.id === app.activeRiskProfileId);
    }

    let newProfileName = $state('');
    async function createProfile() {
        if (!newProfileName.trim()) return;
        await app.createRiskProfile(newProfileName.trim(), 1000, 2, 20);
        newProfileName = '';
    }

    function formatUsd(v: number | undefined | null): string {
        if (v == null || isNaN(v)) return '$0.00';
        return '$' + v.toFixed(2);
    }

    function formatPx(v: number | undefined | null): string {
        if (v == null || isNaN(v)) return '$0.00';
        return '$' + fmtPrice(v, refPrice);
    }

    // Auto-calculate SL from ATR when useAtrStop is enabled
    $effect(() => {
        if (useAtrStop && liveAtr) {
            const ep = parseFloat(app.riskEntryPrice) || 0;
            const dir = app.riskDirection;
            const offset = liveAtr * atrMultiplier;
            if (ep > 0) {
                if (dir === 'LONG') {
                    app.riskStopLoss = String((ep - offset).toFixed(2));
                } else if (dir === 'SHORT') {
                    app.riskStopLoss = String((ep + offset).toFixed(2));
                }
            }
        }
    });

    $effect(() => {
        const entry = parseFloat(app.riskEntryPrice) || 0;
        const sl = parseFloat(app.riskStopLoss) || 0;
        const tp = parseFloat(app.riskTakeProfit) || 0;
        if (entry > 0 && sl > 0 && tp > 0) {
            app.calculateRisk();
        }
    });

    // Fetch risk profiles on mount
    $effect(() => {
        app.fetchRiskProfiles();
    });

    // Recalc on tab switch
    async function refreshRiskProfile() {
        app.riskProfile = null;
        app.riskProfileLoading = false;
        await app.fetchRiskProfile();
    }
</script>

{#if pair}
<div class={styles.panel}>
    <!-- Sub-tab navigation -->
    <div class={styles.tabBar}>
        <button class={activeTab === 'calculator' ? styles.tabActive : styles.tab} onclick={() => activeTab = 'calculator'}>
            📐 Calculator
        </button>
        <button class={activeTab === 'risk_profile' ? styles.tabActive : styles.tab} onclick={() => { activeTab = 'risk_profile'; refreshRiskProfile(); }}>
            🛡 Risk Profile
        </button>
    </div>

    {#if activeTab === 'calculator'}
        <div class={styles.calcLayout}>
            <!-- Left: Profiles -->
            <div class={styles.sidebar}>
                <div class={styles.card}>
                    <h3 class={styles.cardTitle}>RISK PROFILES</h3>
                    <div class={styles.profileList}>
                        {#each app.riskProfiles as profile (profile.id)}
                            <button class="{styles.profileBtn} {profile.id === app.activeRiskProfileId ? styles.profileBtnActive : ''}"
                                onclick={() => app.activeRiskProfileId = profile.id}
                            >
                                <span>{profile.profile_name}</span>
                                {#if app.riskProfiles.length > 1}
                                    <span class={styles.deleteIcon} role="button" tabindex="0" onclick={(e) => { e.stopPropagation(); app.deleteRiskProfile(profile.id); }} onkeydown={(e) => { if (e.key === 'Enter') { e.stopPropagation(); app.deleteRiskProfile(profile.id); } }}>×</span>
                                {/if}
                            </button>
                        {/each}
                    </div>
                    <div class={styles.addProfile}>
                        <input type="text" class={styles.input} placeholder="New profile..." bind:value={newProfileName}
                            onkeydown={(e) => { if (e.key === 'Enter') createProfile(); }} />
                        <button class={styles.addBtn} onclick={createProfile}>+</button>
                    </div>
                </div>
            </div>

            <!-- Right: Calculator -->
            {#if getActiveProfile()}
                {@const profile = getActiveProfile()!}
                <div class={styles.main}>
                    <!-- Account & Risk -->
                    <div class={styles.card}>
                        <h3 class={styles.cardTitle}>ACCOUNT & RISK</h3>
                        <div class={styles.fieldRow}>
                            <label class={styles.label} for="rm-capital">Account Capital</label>
                            <div class={styles.inputWrap}>
                                <span class={styles.inputPrefix}>$</span>
                                <input id="rm-capital" type="number" class={styles.fieldInput} value={profile.capital} readonly />
                            </div>
                        </div>
                        <div class={styles.fieldRow}>
                            <label class={styles.label} for="rm-maxrisk">Max Risk %</label>
                            <div class={styles.inputWrap}>
                                <input id="rm-maxrisk" type="number" class={styles.fieldInput} value={profile.max_risk_pct} readonly />
                                <span class={styles.inputSuffix}>%</span>
                            </div>
                        </div>
                        <div class={styles.fieldRow}>
                            <label class={styles.label} for="rm-leverage">Leverage</label>
                            <div class={styles.inputWrap}>
                                <span class={styles.inputPrefix}>x</span>
                                <input id="rm-leverage" type="number" class={styles.fieldInput} value={profile.leverage} readonly />
                            </div>
                        </div>
                    </div>

                    <!-- New: Risk Limits -->
                    <div class={styles.card}>
                        <h3 class={styles.cardTitle}>RISK LIMITS</h3>
                        <div class={styles.fieldRow}>
                            <label class={styles.label} for="rm-dailyloss">Max Daily Loss</label>
                            <div class={styles.inputWrap}>
                                <span class={styles.inputPrefix}>$</span>
                                <input id="rm-dailyloss" type="number" step="any" class={styles.fieldInput} bind:value={maxDailyLoss} placeholder="0" />
                            </div>
                        </div>
                        <div class={styles.fieldRow}>
                            <label class={styles.label} for="rm-alloc">Max Allocation %</label>
                            <div class={styles.inputWrap}>
                                <input id="rm-alloc" type="number" step="any" class={styles.fieldInput} bind:value={maxAllocationPct} placeholder="25" />
                                <span class={styles.inputSuffix}>%</span>
                            </div>
                        </div>
                    </div>

                    <!-- Operation -->
                    <div class={styles.card}>
                        <h3 class={styles.cardTitle}>OPERATION</h3>
                        <div class={styles.fieldRow}>
                            <span class={styles.label}>Direction</span>
                            <div class={styles.toggle}>
                                <button class="{styles.toggleBtn} {app.riskDirection === 'LONG' ? styles.toggleLong + ' ' + styles.toggleActive : ''}"
                                    onclick={() => app.riskDirection = 'LONG'}>LONG</button>
                                <button class="{styles.toggleBtn} {app.riskDirection === 'SHORT' ? styles.toggleShort + ' ' + styles.toggleActive : ''}"
                                    onclick={() => app.riskDirection = 'SHORT'}>SHORT</button>
                            </div>
                        </div>

                        <!-- ATR-based stop toggle -->
                        {#if liveAtr}
                            <div class={styles.fieldRow}>
                                <label class={styles.label} for="rm-atr-toggle">ATR Stop</label>
                                <div class={styles.atrToggleRow}>
                                    <label class={styles.checkboxLabel}>
                                        <input type="checkbox" id="rm-atr-toggle" bind:checked={useAtrStop} />
                                        <span>Use ATR (${liveAtr.toFixed(4)})</span>
                                    </label>
                                </div>
                            </div>
                            {#if useAtrStop}
                                <div class={styles.fieldRow}>
                                    <label class={styles.label} for="rm-atr-mult">ATR Multiplier</label>
                                    <div class={styles.inputWrap}>
                                        <input id="rm-atr-mult" type="number" step="0.1" class={styles.fieldInput} bind:value={atrMultiplier} min="0.5" max="10" />
                                        <span class={styles.inputSuffix}>×</span>
                                    </div>
                                </div>
                                <div class={styles.atrHint}>
                                    SL offset: ${((liveAtr as number) * atrMultiplier).toFixed(4)} from entry
                                </div>
                            {/if}
                        {/if}

                        <div class={styles.fieldRow}>
                            <label class={styles.label} for="rm-entry">Entry Price</label>
                            <div class={styles.inputWrap}>
                                <span class={styles.inputPrefix}>$</span>
                                <input id="rm-entry" type="number" step="any" class={styles.fieldInput} bind:value={app.riskEntryPrice} placeholder="0" />
                            </div>
                        </div>
                        <div class={styles.fieldRow}>
                            <label class={styles.label} for="rm-sl">Stop Loss</label>
                            <div class={styles.inputWrap}>
                                <span class={styles.inputPrefix}>$</span>
                                <input id="rm-sl" type="number" step="any" class={styles.fieldInput} bind:value={app.riskStopLoss} placeholder="0" readonly={useAtrStop} />
                            </div>
                        </div>
                    </div>

                    <!-- Objectives -->
                    <div class={styles.card}>
                        <h3 class={styles.cardTitle}>OBJECTIVES</h3>
                        <div class={styles.fieldRow}>
                            <label class={styles.label} for="rm-tp">Take Profit</label>
                            <div class={styles.inputWrap}>
                                <span class={styles.inputPrefix}>$</span>
                                <input id="rm-tp" type="number" step="any" class={styles.fieldInput} bind:value={app.riskTakeProfit} placeholder="0" />
                            </div>
                        </div>
                        <div class={styles.fieldRow}>
                            <span class={styles.label}>Risk/Reward</span>
                            <span class={styles.staticVal}>1 : {app.riskCalculation?.risk_reward_ratio != null ? app.riskCalculation!.risk_reward_ratio!.toFixed(2) : '--'}</span>
                        </div>
                        <div class={styles.fieldRow}>
                            <span class={styles.label}>Est. Profit</span>
                            <span class={styles.profitVal}>{formatUsd(app.riskCalculation?.estimated_profit)}</span>
                        </div>
                    </div>

                    <!-- Costs -->
                    <div class={styles.card}>
                        <h3 class={styles.cardTitle}>COSTS</h3>
                        <div class={styles.fieldRow}>
                            <label class={styles.label} for="rm-comm">Commission %</label>
                            <div class={styles.inputWrap}>
                                <input id="rm-comm" type="number" step="any" class={styles.fieldInput} value={profile.commission_pct} readonly />
                                <span class={styles.inputSuffix}>%</span>
                            </div>
                        </div>
                        <div class={styles.fieldRow}>
                            <label class={styles.label} for="rm-funding">Funding (8H)</label>
                            <div class={styles.inputWrap}>
                                <input id="rm-funding" type="number" step="any" class={styles.fieldInput} value={profile.funding_rate_8h} readonly />
                                <span class={styles.inputSuffix}>%</span>
                            </div>
                        </div>
                        <div class={styles.fieldRow}>
                            <label class={styles.label} for="rm-spread">Spread</label>
                            <div class={styles.inputWrap}>
                                <span class={styles.inputPrefix}>$</span>
                                <input id="rm-spread" type="number" step="any" class={styles.fieldInput} value={profile.spread} readonly />
                            </div>
                        </div>
                    </div>

                    <!-- Result Panel -->
                    <div class="{styles.card} {styles.resultCard}">
                        {#if app.riskCalculation}
                            <div class={styles.resultGrid}>
                                <div class={styles.resultItem}>
                                    <span class={styles.resultLabel}>Risk Capital</span>
                                    <span class={styles.resultValue}>{formatUsd(app.riskCalculation.risk_capital)}</span>
                                </div>
                                <div class={styles.resultItem}>
                                    <span class={styles.resultLabel}>Distance to SL</span>
                                    <span class={styles.resultValue}>{formatUsd(app.riskCalculation.price_distance)}</span>
                                </div>
                                <div class={styles.resultItem}>
                                    <span class={styles.resultLabel}>Position Size</span>
                                    <span class={styles.resultValue}>{app.riskCalculation.position_size_units.toFixed(6)}</span>
                                </div>
                                <div class={styles.resultItem}>
                                    <span class={styles.resultLabel}>Notional Value</span>
                                    <span class={styles.resultValue}>{formatUsd(app.riskCalculation.position_notional)}</span>
                                </div>
                                <div class={styles.resultItem}>
                                    <span class={styles.resultLabel}>Leverage Req.</span>
                                    <span class={styles.resultValue}>{app.riskCalculation.leverage_required.toFixed(2)}x</span>
                                </div>
                                <div class={styles.resultItem}>
                                    <span class={styles.resultLabel}>Required Margin</span>
                                    <span class={styles.resultValue}>{formatUsd(app.riskCalculation.margin_required)}</span>
                                </div>
                                <div class="{styles.resultItem} {styles.resultFull}">
                                    <span class={styles.resultLabel}>Liquidation Price</span>
                                    <span class="{styles.resultValue} {styles.liqPrice}">{formatPx(app.riskCalculation.liquidation_price)}</span>
                                </div>
                                <div class={styles.resultItem}>
                                    <span class={styles.resultLabel}>Total Costs</span>
                                    <span class="{styles.resultValue} {styles.costVal}">{formatUsd(app.riskCalculation.total_fees)}</span>
                                </div>
                                <div class={styles.resultItem}>
                                    <span class={styles.resultLabel}>Net PnL</span>
                                    <span class="{styles.resultValue} {app.riskCalculation.net_pnl > 0 ? styles.pnlPos : ''} {app.riskCalculation.net_pnl < 0 ? styles.pnlNeg : ''}">
                                        {formatUsd(app.riskCalculation.net_pnl)}
                                    </span>
                                </div>
                            </div>
                        {:else}
                            <p class={styles.placeholder}>Input parameters to calculate</p>
                        {/if}
                    </div>
                </div>
            {/if}
        </div>
    {:else}
        <!-- Risk Profile Tab -->
        <div class={styles.profileTabContent}>
            {#if app.riskProfileLoading}
                <div class={styles.loadingState}>Loading risk profile...</div>
            {:else if app.riskProfileError}
                <div class={styles.errorState}>{app.riskProfileError}</div>
            {:else if !app.riskProfile?.profile}
                <div class={styles.emptyState}>
                    <span>No risk profile data available</span>
                    <button class={styles.actionBtn} onclick={refreshRiskProfile}>Fetch Risk Profile</button>
                </div>
            {:else}
                {@const prof = app.riskProfile.profile}
                {@const rr = prof.reward_risk}
                <div class={styles.summaryRow}>
                    <div class={styles.riskGauge}>
                        <span class={styles.gaugeLabel}>Overall Risk</span>
                        <span class={styles.gaugeValue} class:severity-critical={prof.overall_risk >= 0.75} class:severity-high={prof.overall_risk >= 0.6 && prof.overall_risk < 0.75} class:severity-elevated={prof.overall_risk >= 0.45 && prof.overall_risk < 0.6} class:severity-normal={prof.overall_risk >= 0.3 && prof.overall_risk < 0.45} class:severity-safe={prof.overall_risk < 0.3}>
                            {(prof.overall_risk * 100).toFixed(0)}%
                        </span>
                        <span class={styles.gaugeConf}>Confidence: {(prof.overall_confidence * 100).toFixed(0)}%</span>
                    </div>
                    <div class={styles.permissionBadge} class:perm-allowed={prof.permission === 'ALLOWED'} class:perm-caution={prof.permission === 'HIGH_CAUTION'} class:perm-blocked={prof.permission === 'BLOCKED'}>
                        {prof.permission || 'N/A'}
                    </div>
                    <div class={styles.exposureInfo}>
                        <span>Exposure: {prof.exposure?.replace(/_/g, ' ') || 'N/A'}</span>
                        <span>Allocation: {prof.recommended_allocation_pct?.toFixed(1)}%</span>
                    </div>
                </div>
                <div class={styles.rrCard}>
                    <span class={styles.cardTitle}>ADAPTIVE R:R</span>
                    <div class={styles.rrGrid}>
                        <div><span>Win Rate</span><span>{(rr?.win_rate_estimate ?? 0).toFixed(3)}</span></div>
                        <div><span>Breakeven</span><span>1:{(rr?.breakeven_ratio ?? 0).toFixed(2)}</span></div>
                        <div><span>Recommended</span><span>1:{(rr?.recommended_ratio ?? 0).toFixed(2)}</span></div>
                        <div><span>Confidence</span><span>{((rr?.confidence ?? 0) * 100).toFixed(0)}%</span></div>
                    </div>
                </div>
                <div class={styles.categoryGrid}>
                    {#each ([
                        { key: 'market', label: 'Market', obj: prof.market },
                        { key: 'structural', label: 'Structural', obj: prof.structural },
                        { key: 'momentum', label: 'Momentum', obj: prof.momentum },
                        { key: 'volatility', label: 'Volatility', obj: prof.volatility },
                        { key: 'liquidity', label: 'Liquidity', obj: prof.liquidity },
                        { key: 'behavioral', label: 'Behavioral', obj: prof.behavioral },
                    ]) as cat}
                        {#if cat.obj}
                            <div class={styles.catCard}>
                                <span class={styles.catLabel}>{cat.label}</span>
                                <span class={styles.catScore}>{(cat.obj.score * 100).toFixed(0)}%</span>
                                <span class={styles.catLevel}>{cat.obj.level}</span>
                            </div>
                        {/if}
                    {/each}
                </div>
                {#if prof.explanation}
                    <div class={styles.explanation}>{prof.explanation}</div>
                {/if}
            {/if}
        </div>
    {/if}
</div>
{/if}
