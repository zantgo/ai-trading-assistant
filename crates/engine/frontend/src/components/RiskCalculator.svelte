<script lang="ts">
    import { getState } from '../state.svelte';
    import type { RiskProfile } from '../state.svelte';

    const app = getState();

    $effect(() => {
        app.fetchRiskProfiles();
    });

    function getActiveProfile(): RiskProfile | undefined {
        return app.riskProfiles.find(p => p.id === app.activeRiskProfileId);
    }

    let newProfileName = $state('');
    async function createProfile() {
        if (!newProfileName.trim()) return;
        await app.createRiskProfile(newProfileName.trim(), 1000, 2, 20, 0.06, 0, 0);
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
</script>

<div class="rc-layout">
    <!-- Left: Profiles -->
    <div class="rc-sidebar">
        <div class="rc-card">
            <h3 class="rc-card-title">RISK PROFILES</h3>
            <div class="rc-profile-list">
                {#each app.riskProfiles as profile (profile.id)}
                    <button class="rc-profile-btn"
                        class:active={profile.id === app.activeRiskProfileId}
                        onclick={() => app.activeRiskProfileId = profile.id}
                    >
                        <span>{profile.profile_name}</span>
                        {#if app.riskProfiles.length > 1}
                            <span class="rc-delete-icon" onclick={(e) => { e.stopPropagation(); app.deleteRiskProfile(profile.id); }}>×</span>
                        {/if}
                    </button>
                {/each}
            </div>
            <div class="rc-add-profile">
                <input type="text" class="rc-input" placeholder="New profile name..." bind:value={newProfileName}
                    onkeydown={(e) => { if (e.key === 'Enter') createProfile(); }} />
                <button class="rc-add-btn" onclick={createProfile}>+</button>
            </div>
        </div>
    </div>

    <!-- Right: Risk Calculator -->
    {#if getActiveProfile()}
        {@const profile = getActiveProfile()!}
        <div class="rc-main">
            <!-- Account & Risk -->
            <div class="rc-card">
                <h3 class="rc-card-title">ACCOUNT & RISK</h3>
                <div class="rc-field-row">
                    <label class="rc-label">ACCOUNT CAPITAL</label>
                    <div class="rc-input-wrap">
                        <span class="rc-input-prefix">$</span>
                        <input type="number" class="rc-field-input" value={profile.capital} readonly />
                    </div>
                </div>
                <div class="rc-field-row">
                    <label class="rc-label">MAX RISK %</label>
                    <div class="rc-input-wrap">
                        <input type="number" class="rc-field-input" value={profile.max_risk_pct} readonly />
                        <span class="rc-input-suffix">%</span>
                    </div>
                </div>
                <div class="rc-field-row">
                    <label class="rc-label">LEVERAGE</label>
                    <div class="rc-input-wrap">
                        <span class="rc-input-prefix">x</span>
                        <input type="number" class="rc-field-input" value={profile.leverage} readonly />
                    </div>
                </div>
            </div>

            <!-- Operation -->
            <div class="rc-card">
                <h3 class="rc-card-title">OPERATION</h3>
                <div class="rc-field-row">
                    <label class="rc-label">DIRECTION TYPE</label>
                    <div class="rc-toggle">
                        <button class="rc-toggle-btn" class:rc-toggle-long={app.riskDirection === 'LONG'} class:rc-toggle-active={app.riskDirection === 'LONG'}
                            onclick={() => app.riskDirection = 'LONG'}>LONG</button>
                        <button class="rc-toggle-btn" class:rc-toggle-short={app.riskDirection === 'SHORT'} class:rc-toggle-active={app.riskDirection === 'SHORT'}
                            onclick={() => app.riskDirection = 'SHORT'}>SHORT</button>
                    </div>
                </div>
                <div class="rc-field-row">
                    <label class="rc-label">ENTRY PRICE</label>
                    <div class="rc-input-wrap">
                        <span class="rc-input-prefix">$</span>
                        <input type="number" step="any" class="rc-field-input" bind:value={app.riskEntryPrice} placeholder="0" />
                    </div>
                </div>
                <div class="rc-field-row">
                    <label class="rc-label">STOP LOSS PRICE</label>
                    <div class="rc-input-wrap">
                        <span class="rc-input-prefix">$</span>
                        <input type="number" step="any" class="rc-field-input" bind:value={app.riskStopLoss} placeholder="0" />
                    </div>
                </div>
            </div>

            <!-- Objectives -->
            <div class="rc-card">
                <h3 class="rc-card-title">OBJECTIVES</h3>
                <div class="rc-field-row">
                    <label class="rc-label">TAKE PROFIT PRICE</label>
                    <div class="rc-input-wrap">
                        <span class="rc-input-prefix">$</span>
                        <input type="number" step="any" class="rc-field-input" bind:value={app.riskTakeProfit} placeholder="0" />
                    </div>
                </div>
                <div class="rc-field-row">
                    <label class="rc-label">RISK/REWARD RATIO</label>
                    <span class="rc-static-val">1 : {app.riskCalculation?.risk_reward_ratio != null ? app.riskCalculation!.risk_reward_ratio!.toFixed(2) : '--'}</span>
                </div>
                <div class="rc-field-row">
                    <label class="rc-label">ESTIMATED PROFIT</label>
                    <span class="rc-profit-val">{formatUsd(app.riskCalculation?.estimated_profit)}</span>
                </div>
            </div>

            <!-- Costs -->
            <div class="rc-card">
                <h3 class="rc-card-title">COSTS</h3>
                <div class="rc-field-row">
                    <label class="rc-label">COMMISSION %</label>
                    <div class="rc-input-wrap">
                        <input type="number" step="any" class="rc-field-input" value={profile.commission_pct} readonly />
                        <span class="rc-input-suffix">%</span>
                    </div>
                </div>
                <div class="rc-field-row">
                    <label class="rc-label">FUNDING RATE (8H)</label>
                    <div class="rc-input-wrap">
                        <input type="number" step="any" class="rc-field-input" value={profile.funding_rate_8h} readonly />
                        <span class="rc-input-suffix">%</span>
                    </div>
                </div>
                <div class="rc-field-row">
                    <label class="rc-label">SPREAD</label>
                    <div class="rc-input-wrap">
                        <span class="rc-input-prefix">$</span>
                        <input type="number" step="any" class="rc-field-input" value={profile.spread} readonly />
                    </div>
                </div>
            </div>

            <!-- Result Panel -->
            <div class="rc-card rc-result-card">
                {#if app.riskCalculation}
                    <div class="rc-result-grid">
                        <div class="rc-result-item">
                            <span class="rc-result-label">Risk Capital</span>
                            <span class="rc-result-value">{formatUsd(app.riskCalculation.risk_capital)}</span>
                        </div>
                        <div class="rc-result-item">
                            <span class="rc-result-label">Distance to SL</span>
                            <span class="rc-result-value">{formatUsd(app.riskCalculation.price_distance)}</span>
                        </div>
                        <div class="rc-result-item">
                            <span class="rc-result-label">Position Size</span>
                            <span class="rc-result-value">{app.riskCalculation.position_size_units.toFixed(6)}</span>
                        </div>
                        <div class="rc-result-item">
                            <span class="rc-result-label">Notional Value</span>
                            <span class="rc-result-value">{formatUsd(app.riskCalculation.position_notional)}</span>
                        </div>
                        <div class="rc-result-item">
                            <span class="rc-result-label">Leverage Required</span>
                            <span class="rc-result-value">{app.riskCalculation.leverage_required.toFixed(2)}x</span>
                        </div>
                        <div class="rc-result-item">
                            <span class="rc-result-label">Required Margin</span>
                            <span class="rc-result-value">{formatUsd(app.riskCalculation.margin_required)}</span>
                        </div>
                        <div class="rc-result-item rc-result-full">
                            <span class="rc-result-label">Liquidation Price</span>
                            <span class="rc-result-value rc-liq-price">{formatUsd(app.riskCalculation.liquidation_price)}</span>
                        </div>
                        <div class="rc-result-item">
                            <span class="rc-result-label">Total Costs</span>
                            <span class="rc-result-value rc-cost">{formatUsd(app.riskCalculation.total_fees)}</span>
                        </div>
                        <div class="rc-result-item">
                            <span class="rc-result-label">Net PnL</span>
                            <span class="rc-result-value" class:rc-pnl-pos={app.riskCalculation.net_pnl > 0} class:rc-pnl-neg={app.riskCalculation.net_pnl < 0}>
                                {formatUsd(app.riskCalculation.net_pnl)}
                            </span>
                        </div>
                    </div>
                {:else}
                    <p class="rc-placeholder">Input parameters to calculate</p>
                {/if}
            </div>
        </div>
    {/if}
</div>

<style>
    .rc-layout { display: grid; grid-template-columns: 240px 1fr; gap: 16px; max-width: 1100px; margin: 0 auto; width: 100%; padding: 16px; box-sizing: border-box; }
    .rc-sidebar { display: flex; flex-direction: column; }
    .rc-main { display: flex; flex-direction: column; gap: 16px; }
    .rc-card { background: #131722; border: 1px solid #2a2e39; border-radius: 8px; padding: 16px; }
    .rc-card-title { font-size: 11px; font-weight: 700; color: #64748b; text-transform: uppercase; letter-spacing: 0.05em; margin: 0 0 10px 0; }

    .rc-profile-list { display: flex; flex-direction: column; gap: 4px; margin-bottom: 8px; }
    .rc-profile-btn {
        background: #0f131c; border: 1px solid #1e293b; color: #94a3b8; padding: 8px 10px; border-radius: 4px;
        font-size: 11px; cursor: pointer; text-align: left; display: flex; justify-content: space-between; align-items: center;
    }
    .rc-profile-btn.active { border-color: #3b82f6; color: #3b82f6; background: rgba(59,130,246,0.08); }
    .rc-delete-icon { color: #ef4444; font-weight: bold; font-size: 14px; padding: 0 4px; }
    .rc-add-profile { display: flex; gap: 4px; }
    .rc-input {
        flex: 1; background: #0f131c; border: 1px solid #2a2e39; color: #e2e8f0; padding: 6px 8px;
        border-radius: 4px; font-size: 11px; outline: none;
    }
    .rc-input:focus { border-color: #3b82f6; }
    .rc-add-btn {
        background: #1e40af; border: 1px solid #3b82f6; color: #f1f5f9; padding: 6px 12px;
        border-radius: 4px; font-size: 10px; font-weight: 700; cursor: pointer; white-space: nowrap;
    }

    .rc-field-row { display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px; }
    .rc-label { font-size: 10px; font-weight: 600; color: #94a3b8; text-transform: uppercase; }
    .rc-input-wrap { display: flex; align-items: center; gap: 4px; }
    .rc-input-prefix, .rc-input-suffix { font-size: 11px; color: #64748b; font-weight: 600; }
    .rc-field-input {
        width: 120px; background: #0f131c; border: 1px solid #2a2e39; color: #e2e8f0;
        padding: 5px 8px; border-radius: 4px; font-size: 11px; text-align: right; outline: none; font-family: monospace;
    }
    .rc-field-input:focus { border-color: #3b82f6; }
    .rc-static-val { font-size: 11px; color: #cbd5e1; font-weight: 700; font-family: monospace; }
    .rc-profit-val { font-size: 13px; color: #10b981; font-weight: 700; font-family: monospace; }

    .rc-toggle { display: flex; gap: 0; border-radius: 4px; overflow: hidden; }
    .rc-toggle-btn {
        padding: 5px 16px; border: 1px solid #2a2e39; background: #0f131c; color: #64748b;
        font-size: 10px; font-weight: 700; cursor: pointer; text-transform: uppercase;
    }
    .rc-toggle-btn:first-child { border-radius: 4px 0 0 4px; }
    .rc-toggle-btn:last-child { border-radius: 0 4px 4px 0; }
    .rc-toggle-long.rc-toggle-active { background: rgba(16,185,129,0.12); border-color: #10b981; color: #10b981; }
    .rc-toggle-short.rc-toggle-active { background: rgba(239,68,68,0.12); border-color: #ef4444; color: #ef4444; }

    .rc-result-card { background: #0c1018; }
    .rc-result-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; }
    .rc-result-item { display: flex; flex-direction: column; gap: 2px; }
    .rc-result-full { grid-column: span 2; }
    .rc-result-label { font-size: 9px; color: #64748b; font-weight: 600; text-transform: uppercase; }
    .rc-result-value { font-size: 13px; color: #e2e8f0; font-weight: 700; font-family: monospace; }
    .rc-liq-price { color: #ef4444; }
    .rc-cost { color: #f59e0b; }
    .rc-pnl-pos { color: #10b981; }
    .rc-pnl-neg { color: #ef4444; }
    .rc-placeholder { font-size: 11px; color: #64748b; text-align: center; padding: 20px; font-style: italic; }

    @media (max-width: 768px) {
        .rc-layout { grid-template-columns: 1fr; }
    }
</style>
