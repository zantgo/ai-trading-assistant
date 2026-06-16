<script lang="ts">
    import { getState } from '../state.svelte';
    import type { RiskProfile, FeeTableRow, CommissionProjection } from '../state.svelte';

    const app = getState();

    $effect(() => {
        app.fetchRiskProfiles();
    });

    $effect(() => {
        app.fetchFeeTable();
    });

    function getActiveProfile(): RiskProfile | undefined {
        return app.riskProfiles.find(p => p.id === app.activeRiskProfileId);
    }

    $effect(() => {
        const e1 = parseFloat(app.commissionEntry1) || 0;
        const e2 = parseFloat(app.commissionEntry2) || 0;
        const sl1 = parseFloat(app.commissionSL1) || 0;
        const sl2 = parseFloat(app.commissionSL2) || 0;
        const tp1 = parseFloat(app.commissionTP1) || 0;
        const tp2 = parseFloat(app.commissionTP2) || 0;
        if (e1 > 0 && e2 > 0 && sl1 > 0 && sl2 > 0 && tp1 > 0 && tp2 > 0) {
            app.calculateCommissionProjection();
        }
    });

    function formatUsd(v: number | undefined | null): string {
        if (v == null || isNaN(v)) return '$0.00';
        return '$' + v.toFixed(2);
    }

    function formatPct(v: number | undefined | null): string {
        if (v == null || isNaN(v)) return '0.00%';
        return v.toFixed(2) + '%';
    }

    function formatUnits(v: number | undefined | null, decimals: number = 6): string {
        if (v == null || isNaN(v)) return '0';
        return v.toFixed(decimals);
    }
</script>

<div class="cc-layout">
    <div class="cc-top">
        <div class="cc-card cc-wide">
            <h3 class="cc-card-title">FEE REFERENCE TABLE</h3>
            <p class="cc-card-sub">Minimum profit % needed to cover round-trip fees at different leverage × capital combinations</p>
            <div class="cc-table-wrap">
                <table class="cc-fee-table">
                    <thead>
                        <tr>
                            <th>Exchange Fee</th>
                            <th>Leverage</th>
                            <th>Capital</th>
                            <th>Min % Profit</th>
                            <th>Fees ($)</th>
                        </tr>
                    </thead>
                    <tbody>
                        {#each app.feeTable as row (row.leverage + '-' + row.capital)}
                            <tr>
                                <td>{row.exchange_fee_pct}%</td>
                                <td>{row.leverage}x</td>
                                <td>${row.capital}</td>
                                <td class:cc-fee-warn={row.min_profit_pct_to_cover_fees > 3}>{formatPct(row.min_profit_pct_to_cover_fees)}</td>
                                <td>{formatUsd(row.fees_in_dollars)}</td>
                            </tr>
                        {/each}
                    </tbody>
                </table>
            </div>
        </div>
    </div>

    <div class="cc-main">
        <div class="cc-sidebar">
            <div class="cc-card">
                <h3 class="cc-card-title">RISK PROFILE</h3>
                <div class="cc-profile-list">
                    {#each app.riskProfiles as profile (profile.id)}
                        <button class="cc-profile-btn"
                            class:active={profile.id === app.activeRiskProfileId}
                            onclick={() => app.activeRiskProfileId = profile.id}
                        >
                            <span>{profile.profile_name}</span>
                        </button>
                    {/each}
                </div>
            </div>

            {#if getActiveProfile()}
                {@const profile = getActiveProfile()!}
                <div class="cc-card">
                    <h3 class="cc-card-title">PROFILE DETAILS</h3>
                    <div class="cc-detail-row"><span>Capital</span><span>{formatUsd(profile.capital)}</span></div>
                    <div class="cc-detail-row"><span>Max Risk</span><span>{formatPct(profile.max_risk_pct)}</span></div>
                    <div class="cc-detail-row"><span>Leverage</span><span>{profile.leverage}x</span></div>
                    <div class="cc-detail-row"><span>Commission</span><span>{formatPct(profile.commission_pct)}</span></div>
                </div>
            {/if}
        </div>

        <div class="cc-inputs">
            <div class="cc-card">
                <h3 class="cc-card-title">TRADE SETUP</h3>
                <div class="cc-field-row">
                    <label class="cc-label">DIRECTION</label>
                    <div class="cc-toggle">
                        <button class="cc-toggle-btn" class:cc-toggle-long={app.commissionDirection === 'LONG'} class:cc-toggle-active={app.commissionDirection === 'LONG'}
                            onclick={() => app.commissionDirection = 'LONG'}>LONG</button>
                        <button class="cc-toggle-btn" class:cc-toggle-short={app.commissionDirection === 'SHORT'} class:cc-toggle-active={app.commissionDirection === 'SHORT'}
                            onclick={() => app.commissionDirection = 'SHORT'}>SHORT</button>
                    </div>
                </div>
                <div class="cc-field-row">
                    <label class="cc-label">ORDER TYPE</label>
                    <div class="cc-toggle">
                        <button class="cc-toggle-btn" class:cc-toggle-active={app.commissionOrderType === 'maker'}
                            onclick={() => { app.commissionOrderType = 'maker'; app.fetchFeeTable(); }}>MAKER</button>
                        <button class="cc-toggle-btn" class:cc-toggle-active={app.commissionOrderType === 'taker'}
                            onclick={() => { app.commissionOrderType = 'taker'; app.fetchFeeTable(); }}>TAKER</button>
                    </div>
                </div>
                <div class="cc-field-row">
                    <label class="cc-label">CAPITAL SPLIT (Entry 1)</label>
                    <div class="cc-split-wrap">
                        <input type="range" min="10" max="90" step="5" bind:value={app.commissionCapitalSplit} class="cc-split-slider" />
                        <span class="cc-split-val">{app.commissionCapitalSplit}%</span>
                    </div>
                </div>
            </div>

            <div class="cc-two-col">
                <div class="cc-card">
                    <h3 class="cc-card-title" style="color: #60a5fa;">ENTRY 1</h3>
                    <div class="cc-field-row">
                        <label class="cc-label">ENTRY PRICE</label>
                        <div class="cc-input-wrap">
                            <span class="cc-input-prefix">$</span>
                            <input type="number" step="any" class="cc-field-input" bind:value={app.commissionEntry1} placeholder="0" />
                        </div>
                    </div>
                    <div class="cc-field-row">
                        <label class="cc-label">STOP LOSS</label>
                        <div class="cc-input-wrap">
                            <span class="cc-input-prefix">$</span>
                            <input type="number" step="any" class="cc-field-input" bind:value={app.commissionSL1} placeholder="0" />
                        </div>
                    </div>
                    <div class="cc-field-row">
                        <label class="cc-label">TAKE PROFIT</label>
                        <div class="cc-input-wrap">
                            <span class="cc-input-prefix">$</span>
                            <input type="number" step="any" class="cc-field-input" bind:value={app.commissionTP1} placeholder="0" />
                        </div>
                    </div>
                </div>

                <div class="cc-card">
                    <h3 class="cc-card-title" style="color: #a78bfa;">ENTRY 2</h3>
                    <div class="cc-field-row">
                        <label class="cc-label">ENTRY PRICE</label>
                        <div class="cc-input-wrap">
                            <span class="cc-input-prefix">$</span>
                            <input type="number" step="any" class="cc-field-input" bind:value={app.commissionEntry2} placeholder="0" />
                        </div>
                    </div>
                    <div class="cc-field-row">
                        <label class="cc-label">STOP LOSS</label>
                        <div class="cc-input-wrap">
                            <span class="cc-input-prefix">$</span>
                            <input type="number" step="any" class="cc-field-input" bind:value={app.commissionSL2} placeholder="0" />
                        </div>
                    </div>
                    <div class="cc-field-row">
                        <label class="cc-label">TAKE PROFIT</label>
                        <div class="cc-input-wrap">
                            <span class="cc-input-prefix">$</span>
                            <input type="number" step="any" class="cc-field-input" bind:value={app.commissionTP2} placeholder="0" />
                        </div>
                    </div>
                </div>
            </div>
        </div>
    </div>

    {#if app.commissionProjection}
        {@const proj = app.commissionProjection as CommissionProjection}
        <div class="cc-results">
            <div class="cc-card cc-viability-card" class:cc-viable={proj.trade_viable} class:cc-not-viable={!proj.trade_viable}>
                <div class="cc-viability-header">
                    <span class="cc-viability-badge">{proj.trade_viable ? '✓ TRADE VIABLE' : '✗ TRADE NOT VIABLE'}</span>
                </div>
                <p class="cc-viability-reason">{proj.viability_reason}</p>
            </div>

            <div class="cc-card">
                <h3 class="cc-card-title">COMBINED POSITION</h3>
                <div class="cc-result-grid cc-result-grid-3">
                    <div class="cc-result-item">
                        <span class="cc-result-label">Weighted Avg Entry</span>
                        <span class="cc-result-value">{formatUsd(proj.weighted_avg_entry)}</span>
                    </div>
                    <div class="cc-result-item">
                        <span class="cc-result-label">Effective Stop Loss</span>
                        <span class="cc-result-value cc-result-sl">{formatUsd(proj.effective_stop_loss)}</span>
                    </div>
                    <div class="cc-result-item">
                        <span class="cc-result-label">Effective Take Profit</span>
                        <span class="cc-result-value cc-result-tp">{formatUsd(proj.effective_take_profit)}</span>
                    </div>
                    <div class="cc-result-item">
                        <span class="cc-result-label">Total Notional</span>
                        <span class="cc-result-value">{formatUsd(proj.total_position_notional)}</span>
                    </div>
                    <div class="cc-result-item">
                        <span class="cc-result-label">Total Margin</span>
                        <span class="cc-result-value">{formatUsd(proj.total_margin_required)}</span>
                    </div>
                    <div class="cc-result-item">
                        <span class="cc-result-label">Total Risk Amount</span>
                        <span class="cc-result-value cc-result-sl">{formatUsd(proj.total_risk_amount)}</span>
                    </div>
                </div>
            </div>

            <div class="cc-two-col">
                <div class="cc-card">
                    <h3 class="cc-card-title" style="color: #60a5fa;">ENTRY 1 METRICS</h3>
                    <div class="cc-result-grid">
                        <div class="cc-result-item">
                            <span class="cc-result-label">Capital Allocated</span>
                            <span class="cc-result-value">{formatUsd(proj.entry_1.capital_allocated)}</span>
                        </div>
                        <div class="cc-result-item">
                            <span class="cc-result-label">Position Size</span>
                            <span class="cc-result-value">{formatUnits(proj.entry_1.position_size_units)}</span>
                        </div>
                        <div class="cc-result-item">
                            <span class="cc-result-label">Notional</span>
                            <span class="cc-result-value">{formatUsd(proj.entry_1.position_notional)}</span>
                        </div>
                        <div class="cc-result-item">
                            <span class="cc-result-label">Margin Required</span>
                            <span class="cc-result-value">{formatUsd(proj.entry_1.margin_required)}</span>
                        </div>
                        <div class="cc-result-item">
                            <span class="cc-result-label">Risk Amount</span>
                            <span class="cc-result-value cc-result-sl">{formatUsd(proj.entry_1.risk_amount)}</span>
                        </div>
                        <div class="cc-result-item">
                            <span class="cc-result-label">Potential Profit</span>
                            <span class="cc-result-value cc-result-tp">{formatUsd(proj.entry_1.potential_profit)}</span>
                        </div>
                        <div class="cc-result-item">
                            <span class="cc-result-label">Fees</span>
                            <span class="cc-result-value cc-result-fee">{formatUsd(proj.entry_1.fees)}</span>
                        </div>
                        <div class="cc-result-item">
                            <span class="cc-result-label">Net Profit</span>
                            <span class="cc-result-value" class:cc-pnl-pos={proj.entry_1.net_profit > 0} class:cc-pnl-neg={proj.entry_1.net_profit <= 0}>
                                {formatUsd(proj.entry_1.net_profit)}
                            </span>
                        </div>
                    </div>
                </div>

                <div class="cc-card">
                    <h3 class="cc-card-title" style="color: #a78bfa;">ENTRY 2 METRICS</h3>
                    <div class="cc-result-grid">
                        <div class="cc-result-item">
                            <span class="cc-result-label">Capital Allocated</span>
                            <span class="cc-result-value">{formatUsd(proj.entry_2.capital_allocated)}</span>
                        </div>
                        <div class="cc-result-item">
                            <span class="cc-result-label">Position Size</span>
                            <span class="cc-result-value">{formatUnits(proj.entry_2.position_size_units)}</span>
                        </div>
                        <div class="cc-result-item">
                            <span class="cc-result-label">Notional</span>
                            <span class="cc-result-value">{formatUsd(proj.entry_2.position_notional)}</span>
                        </div>
                        <div class="cc-result-item">
                            <span class="cc-result-label">Margin Required</span>
                            <span class="cc-result-value">{formatUsd(proj.entry_2.margin_required)}</span>
                        </div>
                        <div class="cc-result-item">
                            <span class="cc-result-label">Risk Amount</span>
                            <span class="cc-result-value cc-result-sl">{formatUsd(proj.entry_2.risk_amount)}</span>
                        </div>
                        <div class="cc-result-item">
                            <span class="cc-result-label">Potential Profit</span>
                            <span class="cc-result-value cc-result-tp">{formatUsd(proj.entry_2.potential_profit)}</span>
                        </div>
                        <div class="cc-result-item">
                            <span class="cc-result-label">Fees</span>
                            <span class="cc-result-value cc-result-fee">{formatUsd(proj.entry_2.fees)}</span>
                        </div>
                        <div class="cc-result-item">
                            <span class="cc-result-label">Net Profit</span>
                            <span class="cc-result-value" class:cc-pnl-pos={proj.entry_2.net_profit > 0} class:cc-pnl-neg={proj.entry_2.net_profit <= 0}>
                                {formatUsd(proj.entry_2.net_profit)}
                            </span>
                        </div>
                    </div>
                </div>
            </div>

            <div class="cc-two-col">
                <div class="cc-card">
                    <h3 class="cc-card-title">FEE BREAKDOWN</h3>
                    <div class="cc-result-grid">
                        <div class="cc-result-item">
                            <span class="cc-result-label">Order Type</span>
                            <span class="cc-result-value">{proj.fee_breakdown.order_type.toUpperCase()}</span>
                        </div>
                        <div class="cc-result-item">
                            <span class="cc-result-label">Effective Fee %</span>
                            <span class="cc-result-value">{formatPct(proj.fee_breakdown.effective_fee_pct)}</span>
                        </div>
                        <div class="cc-result-item">
                            <span class="cc-result-label">Entry 1 Fees</span>
                            <span class="cc-result-value cc-result-fee">{formatUsd(proj.fee_breakdown.entry_1_fees)}</span>
                        </div>
                        <div class="cc-result-item">
                            <span class="cc-result-label">Entry 2 Fees</span>
                            <span class="cc-result-value cc-result-fee">{formatUsd(proj.fee_breakdown.entry_2_fees)}</span>
                        </div>
                        <div class="cc-result-item">
                            <span class="cc-result-label">Total Commission</span>
                            <span class="cc-result-value cc-result-fee">{formatUsd(proj.fee_breakdown.total_fees)}</span>
                        </div>
                        <div class="cc-result-item">
                            <span class="cc-result-label">Funding Cost</span>
                            <span class="cc-result-value">{formatUsd(proj.fee_breakdown.funding_cost)}</span>
                        </div>
                        <div class="cc-result-item cc-result-full">
                            <span class="cc-result-label">Min Profit % to Cover Fees</span>
                            <span class="cc-result-value" class:cc-fee-warn={proj.min_profit_pct_to_cover_fees > 3}>{formatPct(proj.min_profit_pct_to_cover_fees)}</span>
                        </div>
                    </div>
                </div>

                <div class="cc-card">
                    <h3 class="cc-card-title">SCENARIO PROJECTIONS</h3>
                    <div class="cc-result-grid">
                        <div class="cc-result-item">
                            <span class="cc-result-label">Max Gain (Gross)</span>
                            <span class="cc-result-value cc-result-tp">{formatUsd(proj.max_gain_scenario)}</span>
                        </div>
                        <div class="cc-result-item">
                            <span class="cc-result-label">Max Loss (Gross)</span>
                            <span class="cc-result-value cc-result-sl">{formatUsd(proj.max_loss_scenario)}</span>
                        </div>
                        <div class="cc-result-item cc-result-full">
                            <span class="cc-result-label">Max Gain NET (after fees)</span>
                            <span class="cc-result-value" class:cc-pnl-pos={proj.max_gain_net_after_fees > 0} class:cc-pnl-neg={proj.max_gain_net_after_fees <= 0}>
                                {formatUsd(proj.max_gain_net_after_fees)}
                            </span>
                        </div>
                        <div class="cc-result-item cc-result-full">
                            <span class="cc-result-label">Max Loss NET (with fees)</span>
                            <span class="cc-result-value cc-result-sl">{formatUsd(proj.max_loss_net_after_fees)}</span>
                        </div>
                        <div class="cc-result-item cc-result-full">
                            <span class="cc-result-label">Required Price Move %</span>
                            <span class="cc-result-value">{formatPct(proj.required_price_move_pct)}</span>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    {/if}
</div>

<style>
    .cc-layout { display: flex; flex-direction: column; gap: 16px; max-width: 1200px; margin: 0 auto; width: 100%; padding: 16px; box-sizing: border-box; }
    .cc-top { display: flex; flex-direction: column; }
    .cc-main { display: grid; grid-template-columns: 220px 1fr; gap: 16px; }
    .cc-sidebar { display: flex; flex-direction: column; gap: 16px; }
    .cc-inputs { display: flex; flex-direction: column; gap: 16px; }
    .cc-results { display: flex; flex-direction: column; gap: 16px; }
    .cc-two-col { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; }
    .cc-card { background: #131722; border: 1px solid #2a2e39; border-radius: 8px; padding: 16px; }
    .cc-wide { grid-column: 1 / -1; }
    .cc-card-title { font-size: 11px; font-weight: 700; color: #64748b; text-transform: uppercase; letter-spacing: 0.05em; margin: 0 0 10px 0; }
    .cc-card-sub { font-size: 10px; color: #64748b; margin: 0 0 10px 0; }

    .cc-table-wrap { overflow-x: auto; }
    .cc-fee-table { width: 100%; border-collapse: collapse; font-size: 11px; font-family: monospace; }
    .cc-fee-table th { text-align: left; padding: 6px 10px; border-bottom: 1px solid #2a2e39; color: #64748b; font-weight: 600; font-size: 9px; text-transform: uppercase; }
    .cc-fee-table td { padding: 5px 10px; border-bottom: 1px solid #1e293b; color: #cbd5e1; }
    .cc-fee-table tr:hover td { background: rgba(59,130,246,0.04); }

    .cc-profile-list { display: flex; flex-direction: column; gap: 4px; }
    .cc-profile-btn {
        background: #0f131c; border: 1px solid #1e293b; color: #94a3b8; padding: 8px 10px; border-radius: 4px;
        font-size: 11px; cursor: pointer; text-align: left;
    }
    .cc-profile-btn.active { border-color: #3b82f6; color: #3b82f6; background: rgba(59,130,246,0.08); }
    .cc-detail-row { display: flex; justify-content: space-between; font-size: 10px; padding: 4px 0; color: #94a3b8; border-bottom: 1px solid #1e293b; }
    .cc-detail-row span:last-child { color: #cbd5e1; font-weight: 600; font-family: monospace; }

    .cc-field-row { display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px; }
    .cc-label { font-size: 10px; font-weight: 600; color: #94a3b8; text-transform: uppercase; }
    .cc-input-wrap { display: flex; align-items: center; gap: 4px; }
    .cc-input-prefix { font-size: 11px; color: #64748b; font-weight: 600; }
    .cc-field-input {
        width: 130px; background: #0f131c; border: 1px solid #2a2e39; color: #e2e8f0;
        padding: 5px 8px; border-radius: 4px; font-size: 11px; text-align: right; outline: none; font-family: monospace;
    }
    .cc-field-input:focus { border-color: #3b82f6; }
    .cc-split-wrap { display: flex; align-items: center; gap: 8px; }
    .cc-split-slider { width: 160px; accent-color: #3b82f6; }
    .cc-split-val { font-size: 12px; font-weight: 700; color: #cbd5e1; font-family: monospace; min-width: 36px; }

    .cc-toggle { display: flex; gap: 0; border-radius: 4px; overflow: hidden; }
    .cc-toggle-btn {
        padding: 5px 12px; border: 1px solid #2a2e39; background: #0f131c; color: #64748b;
        font-size: 10px; font-weight: 700; cursor: pointer; text-transform: uppercase;
    }
    .cc-toggle-btn:first-child { border-radius: 4px 0 0 4px; }
    .cc-toggle-btn:last-child { border-radius: 0 4px 4px 0; }
    .cc-toggle-long.cc-toggle-active { background: rgba(16,185,129,0.12); border-color: #10b981; color: #10b981; }
    .cc-toggle-short.cc-toggle-active { background: rgba(239,68,68,0.12); border-color: #ef4444; color: #ef4444; }
    .cc-toggle-btn.cc-toggle-active:not(.cc-toggle-long):not(.cc-toggle-short) { background: rgba(59,130,246,0.12); border-color: #60a5fa; color: #60a5fa; }

    .cc-viability-card { border: 1px solid #2a2e39; }
    .cc-viable { border-color: rgba(16,185,129,0.3); background: rgba(16,185,129,0.04); }
    .cc-not-viable { border-color: rgba(239,68,68,0.3); background: rgba(239,68,68,0.04); }
    .cc-viability-header { margin-bottom: 6px; }
    .cc-viability-badge {
        font-size: 14px; font-weight: 800; padding: 4px 12px; border-radius: 4px; font-family: monospace;
    }
    .cc-viable .cc-viability-badge { color: #10b981; background: rgba(16,185,129,0.1); }
    .cc-not-viable .cc-viability-badge { color: #ef4444; background: rgba(239,68,68,0.1); }
    .cc-viability-reason { font-size: 11px; color: #94a3b8; margin: 0; line-height: 1.5; }

    .cc-result-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; }
    .cc-result-grid-3 { grid-template-columns: 1fr 1fr 1fr; }
    .cc-result-item { display: flex; flex-direction: column; gap: 2px; }
    .cc-result-full { grid-column: span 2; }
    .cc-result-grid-3 .cc-result-full { grid-column: span 3; }
    .cc-result-label { font-size: 9px; color: #64748b; font-weight: 600; text-transform: uppercase; }
    .cc-result-value { font-size: 13px; color: #e2e8f0; font-weight: 700; font-family: monospace; }
    .cc-result-sl { color: #ef4444; }
    .cc-result-tp { color: #10b981; }
    .cc-result-fee { color: #f59e0b; }
    .cc-pnl-pos { color: #10b981; }
    .cc-pnl-neg { color: #ef4444; }
    .cc-fee-warn { color: #f59e0b; }

    @media (max-width: 768px) {
        .cc-main { grid-template-columns: 1fr; }
        .cc-two-col { grid-template-columns: 1fr; }
        .cc-result-grid-3 { grid-template-columns: 1fr 1fr; }
    }
</style>
