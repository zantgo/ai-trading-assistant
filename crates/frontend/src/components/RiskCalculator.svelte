<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import type { RiskProfile } from '../types';
    import { fmtPrice } from '../lib/telemetry';
    import styles from './RiskCalculator.module.css';

    const app = useAppStore();

    // Reference price for the panel's decimal resolution: the typed entry price.
    const refPrice = $derived(parseFloat(app.riskEntryPrice) || 0);

    $effect(() => {
        app.fetchRiskProfiles();
    });

    function getActiveProfile(): RiskProfile | undefined {
        return app.riskProfiles.find(p => p.id === app.activeRiskProfileId);
    }

    let newProfileName = $state('');
    async function createProfile() {
        if (!newProfileName.trim()) return;
        await app.createRiskProfile(newProfileName.trim(), 1000, 2, 20);
        newProfileName = '';
    }

    $effect(() => {
        const entry = parseFloat(app.riskEntryPrice) || 0;
        const sl = parseFloat(app.riskStopLoss) || 0;
        const tp = parseFloat(app.riskTakeProfit) || 0;
        if (entry > 0 && sl > 0 && tp > 0) {
            app.calculateRisk();
        }
    });

    function formatUsd(v: number | undefined | null): string {
        if (v == null || isNaN(v)) return '$0.00';
        return '$' + v.toFixed(2);
    }

    // Price-scaled USD formatter for price-level fields (entry/SL/TP/liq).
    function formatPx(v: number | undefined | null): string {
        if (v == null || isNaN(v)) return '$0.00';
        return '$' + fmtPrice(v, refPrice);
    }
</script>

<div class={styles.rcLayout}>
    <!-- Left: Profiles -->
    <div class={styles.rcSidebar}>
        <div class={styles.rcCard}>
            <h3 class={styles.rcCardTitle}>RISK PROFILES</h3>
            <div class={styles.rcProfileList}>
                {#each app.riskProfiles as profile (profile.id)}
                    <button class="{styles.rcProfileBtn} {profile.id === app.activeRiskProfileId ? styles.active : ''}"
                        onclick={() => app.activeRiskProfileId = profile.id}
                    >
                        <span>{profile.profile_name}</span>
                        {#if app.riskProfiles.length > 1}
                            <span class={styles.rcDeleteIcon} role="button" tabindex="0" onclick={(e) => { e.stopPropagation(); app.deleteRiskProfile(profile.id); }} onkeydown={(e) => { if (e.key === 'Enter') { e.stopPropagation(); app.deleteRiskProfile(profile.id); } }}>×</span>
                        {/if}
                    </button>
                {/each}
            </div>
            <div class={styles.rcAddProfile}>
                <input type="text" class={styles.rcInput} placeholder="New profile name..." bind:value={newProfileName}
                    onkeydown={(e) => { if (e.key === 'Enter') createProfile(); }} />
                <button class={styles.rcAddBtn} onclick={createProfile}>+</button>
            </div>
        </div>
    </div>

    <!-- Right: Risk Calculator -->
    {#if getActiveProfile()}
        {@const profile = getActiveProfile()!}
        <div class={styles.rcMain}>
            <!-- Account & Risk -->
            <div class={styles.rcCard}>
                <h3 class={styles.rcCardTitle}>ACCOUNT & RISK</h3>
                <div class={styles.rcFieldRow}>
                    <label class={styles.rcLabel} for="rc-capital">ACCOUNT CAPITAL</label>
                    <div class={styles.rcInputWrap}>
                        <span class={styles.rcInputPrefix}>$</span>
                        <input id="rc-capital" type="number" class={styles.rcFieldInput} value={profile.capital} readonly />
                    </div>
                </div>
                <div class={styles.rcFieldRow}>
                    <label class={styles.rcLabel} for="rc-maxrisk">MAX RISK %</label>
                    <div class={styles.rcInputWrap}>
                        <input id="rc-maxrisk" type="number" class={styles.rcFieldInput} value={profile.max_risk_pct} readonly />
                        <span class={styles.rcInputSuffix}>%</span>
                    </div>
                </div>
                <div class={styles.rcFieldRow}>
                    <label class={styles.rcLabel} for="rc-leverage">LEVERAGE</label>
                    <div class={styles.rcInputWrap}>
                        <span class={styles.rcInputPrefix}>x</span>
                        <input id="rc-leverage" type="number" class={styles.rcFieldInput} value={profile.leverage} readonly />
                    </div>
                </div>
            </div>

            <!-- Operation -->
            <div class={styles.rcCard}>
                <h3 class={styles.rcCardTitle}>OPERATION</h3>
                <div class={styles.rcFieldRow}>
                    <!-- svelte-ignore a11y_label_has_associated_control -->
                    <label class={styles.rcLabel}>DIRECTION TYPE</label>
                    <div class={styles.rcToggle}>
                        <button class="{styles.rcToggleBtn} {app.riskDirection === 'LONG' ? styles.rcToggleLong + ' ' + styles.rcToggleActive : ''}"
                            onclick={() => app.riskDirection = 'LONG'}>LONG</button>
                        <button class="{styles.rcToggleBtn} {app.riskDirection === 'SHORT' ? styles.rcToggleShort + ' ' + styles.rcToggleActive : ''}"
                            onclick={() => app.riskDirection = 'SHORT'}>SHORT</button>
                    </div>
                </div>
                <div class={styles.rcFieldRow}>
                    <label class={styles.rcLabel} for="rc-entry">ENTRY PRICE</label>
                    <div class={styles.rcInputWrap}>
                        <span class={styles.rcInputPrefix}>$</span>
                        <input id="rc-entry" type="number" step="any" class={styles.rcFieldInput} bind:value={app.riskEntryPrice} placeholder="0" />
                    </div>
                </div>
                <div class={styles.rcFieldRow}>
                    <label class={styles.rcLabel} for="rc-stoploss">STOP LOSS PRICE</label>
                    <div class={styles.rcInputWrap}>
                        <span class={styles.rcInputPrefix}>$</span>
                        <input id="rc-stoploss" type="number" step="any" class={styles.rcFieldInput} bind:value={app.riskStopLoss} placeholder="0" />
                    </div>
                </div>
            </div>

            <!-- Objectives -->
            <div class={styles.rcCard}>
                <h3 class={styles.rcCardTitle}>OBJECTIVES</h3>
                <div class={styles.rcFieldRow}>
                    <label class={styles.rcLabel} for="rc-takeprofit">TAKE PROFIT PRICE</label>
                    <div class={styles.rcInputWrap}>
                        <span class={styles.rcInputPrefix}>$</span>
                        <input id="rc-takeprofit" type="number" step="any" class={styles.rcFieldInput} bind:value={app.riskTakeProfit} placeholder="0" />
                    </div>
                </div>
                <div class={styles.rcFieldRow}>
                    <!-- svelte-ignore a11y_label_has_associated_control -->
                    <label class={styles.rcLabel}>RISK/REWARD RATIO</label>
                    <span class={styles.rcStaticVal}>1 : {app.riskCalculation?.risk_reward_ratio != null ? app.riskCalculation!.risk_reward_ratio!.toFixed(2) : '--'}</span>
                </div>
                <div class={styles.rcFieldRow}>
                    <!-- svelte-ignore a11y_label_has_associated_control -->
                    <label class={styles.rcLabel}>ESTIMATED PROFIT</label>
                    <span class={styles.rcProfitVal}>{formatUsd(app.riskCalculation?.estimated_profit)}</span>
                </div>
            </div>

            <!-- Costs -->
            <div class={styles.rcCard}>
                <h3 class={styles.rcCardTitle}>COSTS</h3>
                <div class={styles.rcFieldRow}>
                    <label class={styles.rcLabel} for="rc-commission">COMMISSION %</label>
                    <div class={styles.rcInputWrap}>
                        <input id="rc-commission" type="number" step="any" class={styles.rcFieldInput} value={profile.commission_pct} readonly />
                        <span class={styles.rcInputSuffix}>%</span>
                    </div>
                </div>
                <div class={styles.rcFieldRow}>
                    <label class={styles.rcLabel} for="rc-funding">FUNDING RATE (8H)</label>
                    <div class={styles.rcInputWrap}>
                        <input id="rc-funding" type="number" step="any" class={styles.rcFieldInput} value={profile.funding_rate_8h} readonly />
                        <span class={styles.rcInputSuffix}>%</span>
                    </div>
                </div>
                <div class={styles.rcFieldRow}>
                    <label class={styles.rcLabel} for="rc-spread">SPREAD</label>
                    <div class={styles.rcInputWrap}>
                        <span class={styles.rcInputPrefix}>$</span>
                        <input id="rc-spread" type="number" step="any" class={styles.rcFieldInput} value={profile.spread} readonly />
                    </div>
                </div>
            </div>

            <!-- Result Panel -->
            <div class="{styles.rcCard} {styles.rcResultCard}">
                {#if app.riskCalculation}
                    <div class={styles.rcResultGrid}>
                        <div class={styles.rcResultItem}>
                            <span class={styles.rcResultLabel}>Risk Capital</span>
                            <span class={styles.rcResultValue}>{formatUsd(app.riskCalculation.risk_capital)}</span>
                        </div>
                        <div class={styles.rcResultItem}>
                            <span class={styles.rcResultLabel}>Distance to SL</span>
                            <span class={styles.rcResultValue}>{formatUsd(app.riskCalculation.price_distance)}</span>
                        </div>
                        <div class={styles.rcResultItem}>
                            <span class={styles.rcResultLabel}>Position Size</span>
                            <span class={styles.rcResultValue}>{app.riskCalculation.position_size_units.toFixed(6)}</span>
                        </div>
                        <div class={styles.rcResultItem}>
                            <span class={styles.rcResultLabel}>Notional Value</span>
                            <span class={styles.rcResultValue}>{formatUsd(app.riskCalculation.position_notional)}</span>
                        </div>
                        <div class={styles.rcResultItem}>
                            <span class={styles.rcResultLabel}>Leverage Required</span>
                            <span class={styles.rcResultValue}>{app.riskCalculation.leverage_required.toFixed(2)}x</span>
                        </div>
                        <div class={styles.rcResultItem}>
                            <span class={styles.rcResultLabel}>Required Margin</span>
                            <span class={styles.rcResultValue}>{formatUsd(app.riskCalculation.margin_required)}</span>
                        </div>
                        <div class="{styles.rcResultItem} {styles.rcResultFull}">
                            <span class={styles.rcResultLabel}>Liquidation Price</span>
                            <span class="{styles.rcResultValue} {styles.rcLiqPrice}">{formatPx(app.riskCalculation.liquidation_price)}</span>
                        </div>
                        <div class={styles.rcResultItem}>
                            <span class={styles.rcResultLabel}>Total Costs</span>
                            <span class="{styles.rcResultValue} {styles.rcCost}">{formatUsd(app.riskCalculation.total_fees)}</span>
                        </div>
                        <div class={styles.rcResultItem}>
                            <span class={styles.rcResultLabel}>Net PnL</span>
                            <span class="{styles.rcResultValue} {app.riskCalculation.net_pnl > 0 ? styles.rcPnlPos : ''} {app.riskCalculation.net_pnl < 0 ? styles.rcPnlNeg : ''}">
                                {formatUsd(app.riskCalculation.net_pnl)}
                            </span>
                        </div>
                    </div>
                {:else}
                    <p class={styles.rcPlaceholder}>Input parameters to calculate</p>
                {/if}
            </div>
        </div>
    {/if}
</div>

