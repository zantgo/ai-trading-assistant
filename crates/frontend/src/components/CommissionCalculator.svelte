<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import type { RiskProfile, CommissionProjection } from '../types';
    import { fmtPrice } from '../lib/telemetry';
    import styles from './CommissionCalculator.module.css';

    const app = useAppStore();

    // Reference price for the panel's decimal resolution: the Entry 1 price.
    const refPrice = $derived(parseFloat(app.commissionEntry1) || 0);

    $effect(() => {
        app.fetchRiskProfiles();
    });

    $effect(() => {
        app.fetchFeeTable();
    });

    // ── Fee reference calculator (client-side) ──
    let calcLeverage = $state(10);
    let calcCapital = $state(1000);
    let calcFeePct = $state(0.06);

    const calcNotional = $derived(calcCapital * calcLeverage);
    const calcFees = $derived((calcFeePct / 100) * calcNotional * 2);
    const calcMinProfitPct = $derived(calcCapital > 0 ? (calcFees / calcCapital) * 100 : 0);

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

    // Price-scaled USD formatter for price-level fields (weighted entry, SL/TP).
    function formatPx(v: number | undefined | null): string {
        if (v == null || isNaN(v)) return '$0.00';
        return '$' + fmtPrice(v, refPrice);
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

<div class={styles.ccLayout}>
    <div class={styles.ccTop}>
        <div class="{styles.ccCard} {styles.ccWide}">
            <h3 class={styles.ccCardTitle}>FEE REFERENCE CALCULATOR</h3>
            <p class={styles.ccCardSub}>Calculate round-trip fees and minimum profit needed to break even</p>
            <div class={styles.ccCalcRow}>
                <div class={styles.ccCalcField}>
                    <label class={styles.ccCalcLabel} for="cc-calc-leverage">Leverage</label>
                    <input id="cc-calc-leverage" type="number" min="1" max="150" step="1" bind:value={calcLeverage} class={styles.ccCalcInput} />
                </div>
                <div class={styles.ccCalcField}>
                    <label class={styles.ccCalcLabel} for="cc-calc-capital">Capital ($)</label>
                    <input id="cc-calc-capital" type="number" min="1" step="100" bind:value={calcCapital} class={styles.ccCalcInput} />
                </div>
                <div class={styles.ccCalcField}>
                    <label class={styles.ccCalcLabel} for="cc-calc-fee">Exchange Fee (%)</label>
                    <input id="cc-calc-fee" type="number" min="0" max="10" step="0.01" bind:value={calcFeePct} class={styles.ccCalcInput} />
                </div>
            </div>
            <div class={styles.ccCalcResults}>
                <div class={styles.ccCalcResultItem}>
                    <span class={styles.ccCalcResultLabel}>Notional Value</span>
                    <span class={styles.ccCalcResultValue}>{formatUsd(calcNotional)}</span>
                </div>
                <div class={styles.ccCalcResultItem}>
                    <span class={styles.ccCalcResultLabel}>Round-Trip Fees</span>
                    <span class="{styles.ccCalcResultValue} {calcMinProfitPct > 3 ? styles.ccFeeWarn : ''}">{formatUsd(calcFees)}</span>
                </div>
                <div class={styles.ccCalcResultItem}>
                    <span class={styles.ccCalcResultLabel}>Min Profit to Cover</span>
                    <span class={styles.ccCalcResultValue}>{formatUsd(calcFees)} <span class={styles.ccCalcResultSub}>(open + close)</span></span>
                </div>
                <div class={styles.ccCalcResultItem}>
                    <span class={styles.ccCalcResultLabel}>Min Profit %</span>
                    <span class="{styles.ccCalcResultValue} {calcMinProfitPct > 3 ? styles.ccFeeWarn : ''}">{formatPct(calcMinProfitPct)}</span>
                </div>
            </div>
        </div>
    </div>

    <div class={styles.ccMain}>
        <div class={styles.ccSidebar}>
            <div class={styles.ccCard}>
                <h3 class={styles.ccCardTitle}>RISK PROFILE</h3>
                <div class={styles.ccProfileList}>
                    {#each app.riskProfiles as profile (profile.id)}
                        <button class="{styles.ccProfileBtn} {profile.id === app.activeRiskProfileId ? styles.active : ''}"
                            onclick={() => app.activeRiskProfileId = profile.id}
                        >
                            <span>{profile.profile_name}</span>
                        </button>
                    {/each}
                </div>
            </div>

            {#if getActiveProfile()}
                {@const profile = getActiveProfile()!}
                <div class={styles.ccCard}>
                    <h3 class={styles.ccCardTitle}>PROFILE DETAILS</h3>
                    <div class={styles.ccDetailRow}><span>Capital</span><span>{formatUsd(profile.capital)}</span></div>
                    <div class={styles.ccDetailRow}><span>Max Risk</span><span>{formatPct(profile.max_risk_pct)}</span></div>
                    <div class={styles.ccDetailRow}><span>Leverage</span><span>{profile.leverage}x</span></div>
                    <div class={styles.ccDetailRow}><span>Commission</span><span>{formatPct(profile.commission_pct)}</span></div>
                </div>
            {/if}
        </div>

        <div class={styles.ccInputs}>
            <div class={styles.ccCard}>
                <h3 class={styles.ccCardTitle}>TRADE SETUP</h3>
                <div class={styles.ccFieldRow}>
                    <!-- svelte-ignore a11y_label_has_associated_control -->
                    <label class={styles.ccLabel}>DIRECTION</label>
                    <div class={styles.ccToggle}>
                        <button class="{styles.ccToggleBtn} {app.commissionDirection === 'LONG' ? styles.ccToggleLong + ' ' + styles.ccToggleActive : ''}"
                            onclick={() => app.commissionDirection = 'LONG'}>LONG</button>
                        <button class="{styles.ccToggleBtn} {app.commissionDirection === 'SHORT' ? styles.ccToggleShort + ' ' + styles.ccToggleActive : ''}"
                            onclick={() => app.commissionDirection = 'SHORT'}>SHORT</button>
                    </div>
                </div>
                <div class={styles.ccFieldRow}>
                    <!-- svelte-ignore a11y_label_has_associated_control -->
                    <label class={styles.ccLabel}>ORDER TYPE</label>
                    <div class={styles.ccToggle}>
                        <button class="{styles.ccToggleBtn} {app.commissionOrderType === 'maker' ? styles.ccToggleActive : ''}"
                            onclick={() => { app.commissionOrderType = 'maker'; app.fetchFeeTable(); }}>MAKER</button>
                        <button class="{styles.ccToggleBtn} {app.commissionOrderType === 'taker' ? styles.ccToggleActive : ''}"
                            onclick={() => { app.commissionOrderType = 'taker'; app.fetchFeeTable(); }}>TAKER</button>
                    </div>
                </div>
                <div class={styles.ccFieldRow}>
                    <label class={styles.ccLabel} for="cc-split">CAPITAL SPLIT (Entry 1)</label>
                    <div class={styles.ccSplitWrap}>
                        <input id="cc-split" type="range" min="10" max="90" step="5" bind:value={app.commissionCapitalSplit} class={styles.ccSplitSlider} />
                        <span class={styles.ccSplitVal}>{app.commissionCapitalSplit}%</span>
                    </div>
                </div>
            </div>

            <div class={styles.ccTwoCol}>
                <div class={styles.ccCard}>
                    <h3 class={styles.ccCardTitle} style="color: #f8fafc;">ENTRY 1</h3>
                    <div class={styles.ccFieldRow}>
                        <label class={styles.ccLabel} for="cc-entry1">ENTRY PRICE</label>
                        <div class={styles.ccInputWrap}>
                            <span class={styles.ccInputPrefix}>$</span>
                            <input id="cc-entry1" type="number" step="any" class={styles.ccFieldInput} bind:value={app.commissionEntry1} placeholder="0" />
                        </div>
                    </div>
                    <div class={styles.ccFieldRow}>
                        <label class={styles.ccLabel} for="cc-sl1">STOP LOSS</label>
                        <div class={styles.ccInputWrap}>
                            <span class={styles.ccInputPrefix}>$</span>
                            <input id="cc-sl1" type="number" step="any" class={styles.ccFieldInput} bind:value={app.commissionSL1} placeholder="0" />
                        </div>
                    </div>
                    <div class={styles.ccFieldRow}>
                        <label class={styles.ccLabel} for="cc-tp1">TAKE PROFIT</label>
                        <div class={styles.ccInputWrap}>
                            <span class={styles.ccInputPrefix}>$</span>
                            <input id="cc-tp1" type="number" step="any" class={styles.ccFieldInput} bind:value={app.commissionTP1} placeholder="0" />
                        </div>
                    </div>
                </div>

                <div class={styles.ccCard}>
                    <h3 class={styles.ccCardTitle} style="color: #f8fafc;">ENTRY 2</h3>
                    <div class={styles.ccFieldRow}>
                        <label class={styles.ccLabel} for="cc-entry2">ENTRY PRICE</label>
                        <div class={styles.ccInputWrap}>
                            <span class={styles.ccInputPrefix}>$</span>
                            <input id="cc-entry2" type="number" step="any" class={styles.ccFieldInput} bind:value={app.commissionEntry2} placeholder="0" />
                        </div>
                    </div>
                    <div class={styles.ccFieldRow}>
                        <label class={styles.ccLabel} for="cc-sl2">STOP LOSS</label>
                        <div class={styles.ccInputWrap}>
                            <span class={styles.ccInputPrefix}>$</span>
                            <input id="cc-sl2" type="number" step="any" class={styles.ccFieldInput} bind:value={app.commissionSL2} placeholder="0" />
                        </div>
                    </div>
                    <div class={styles.ccFieldRow}>
                        <label class={styles.ccLabel} for="cc-tp2">TAKE PROFIT</label>
                        <div class={styles.ccInputWrap}>
                            <span class={styles.ccInputPrefix}>$</span>
                            <input id="cc-tp2" type="number" step="any" class={styles.ccFieldInput} bind:value={app.commissionTP2} placeholder="0" />
                        </div>
                    </div>
                </div>
            </div>
        </div>
    </div>

    {#if app.commissionProjection}
        {@const proj = app.commissionProjection as CommissionProjection}
        <div class={styles.ccResults}>
            <div class="{styles.ccCard} {styles.ccViabilityCard} {proj.trade_viable ? styles.ccViable : styles.ccNotViable}">
                <div class={styles.ccViabilityHeader}>
                    <span class={styles.ccViabilityBadge}>{proj.trade_viable ? '✓ TRADE VIABLE' : '✗ TRADE NOT VIABLE'}</span>
                </div>
                <p class={styles.ccViabilityReason}>{proj.viability_reason}</p>
            </div>

            <div class={styles.ccCard}>
                <h3 class={styles.ccCardTitle}>COMBINED POSITION</h3>
                <div class="{styles.ccResultGrid} {styles.ccResultGrid3}">
                    <div class={styles.ccResultItem}>
                        <span class={styles.ccResultLabel}>Weighted Avg Entry</span>
                        <span class={styles.ccResultValue}>{formatPx(proj.weighted_avg_entry)}</span>
                    </div>
                    <div class={styles.ccResultItem}>
                        <span class={styles.ccResultLabel}>Effective Stop Loss</span>
                        <span class="{styles.ccResultValue} {styles.ccResultSl}">{formatPx(proj.effective_stop_loss)}</span>
                    </div>
                    <div class={styles.ccResultItem}>
                        <span class={styles.ccResultLabel}>Effective Take Profit</span>
                        <span class="{styles.ccResultValue} {styles.ccResultTp}">{formatPx(proj.effective_take_profit)}</span>
                    </div>
                    <div class={styles.ccResultItem}>
                        <span class={styles.ccResultLabel}>Total Notional</span>
                        <span class={styles.ccResultValue}>{formatUsd(proj.total_position_notional)}</span>
                    </div>
                    <div class={styles.ccResultItem}>
                        <span class={styles.ccResultLabel}>Total Margin</span>
                        <span class={styles.ccResultValue}>{formatUsd(proj.total_margin_required)}</span>
                    </div>
                    <div class={styles.ccResultItem}>
                        <span class={styles.ccResultLabel}>Total Risk Amount</span>
                        <span class="{styles.ccResultValue} {styles.ccResultSl}">{formatUsd(proj.total_risk_amount)}</span>
                    </div>
                </div>
            </div>

            <div class={styles.ccTwoCol}>
                <div class={styles.ccCard}>
                    <h3 class={styles.ccCardTitle} style="color: #f8fafc;">ENTRY 1 METRICS</h3>
                    <div class={styles.ccResultGrid}>
                        <div class={styles.ccResultItem}>
                            <span class={styles.ccResultLabel}>Capital Allocated</span>
                            <span class={styles.ccResultValue}>{formatUsd(proj.entry_1.capital_allocated)}</span>
                        </div>
                        <div class={styles.ccResultItem}>
                            <span class={styles.ccResultLabel}>Position Size</span>
                            <span class={styles.ccResultValue}>{formatUnits(proj.entry_1.position_size_units)}</span>
                        </div>
                        <div class={styles.ccResultItem}>
                            <span class={styles.ccResultLabel}>Notional</span>
                            <span class={styles.ccResultValue}>{formatUsd(proj.entry_1.position_notional)}</span>
                        </div>
                        <div class={styles.ccResultItem}>
                            <span class={styles.ccResultLabel}>Margin Required</span>
                            <span class={styles.ccResultValue}>{formatUsd(proj.entry_1.margin_required)}</span>
                        </div>
                        <div class={styles.ccResultItem}>
                            <span class={styles.ccResultLabel}>Risk Amount</span>
                            <span class="{styles.ccResultValue} {styles.ccResultSl}">{formatUsd(proj.entry_1.risk_amount)}</span>
                        </div>
                        <div class={styles.ccResultItem}>
                            <span class={styles.ccResultLabel}>Potential Profit</span>
                            <span class="{styles.ccResultValue} {styles.ccResultTp}">{formatUsd(proj.entry_1.potential_profit)}</span>
                        </div>
                        <div class={styles.ccResultItem}>
                            <span class={styles.ccResultLabel}>Fees</span>
                            <span class="{styles.ccResultValue} {styles.ccResultFee}">{formatUsd(proj.entry_1.fees)}</span>
                        </div>
                        <div class={styles.ccResultItem}>
                            <span class={styles.ccResultLabel}>Net Profit</span>
                            <span class="{styles.ccResultValue} {proj.entry_1.net_profit > 0 ? styles.ccPnlPos : styles.ccPnlNeg}">
                                {formatUsd(proj.entry_1.net_profit)}
                            </span>
                        </div>
                    </div>
                </div>

                <div class={styles.ccCard}>
                    <h3 class={styles.ccCardTitle} style="color: #f8fafc;">ENTRY 2 METRICS</h3>
                    <div class={styles.ccResultGrid}>
                        <div class={styles.ccResultItem}>
                            <span class={styles.ccResultLabel}>Capital Allocated</span>
                            <span class={styles.ccResultValue}>{formatUsd(proj.entry_2.capital_allocated)}</span>
                        </div>
                        <div class={styles.ccResultItem}>
                            <span class={styles.ccResultLabel}>Position Size</span>
                            <span class={styles.ccResultValue}>{formatUnits(proj.entry_2.position_size_units)}</span>
                        </div>
                        <div class={styles.ccResultItem}>
                            <span class={styles.ccResultLabel}>Notional</span>
                            <span class={styles.ccResultValue}>{formatUsd(proj.entry_2.position_notional)}</span>
                        </div>
                        <div class={styles.ccResultItem}>
                            <span class={styles.ccResultLabel}>Margin Required</span>
                            <span class={styles.ccResultValue}>{formatUsd(proj.entry_2.margin_required)}</span>
                        </div>
                        <div class={styles.ccResultItem}>
                            <span class={styles.ccResultLabel}>Risk Amount</span>
                            <span class="{styles.ccResultValue} {styles.ccResultSl}">{formatUsd(proj.entry_2.risk_amount)}</span>
                        </div>
                        <div class={styles.ccResultItem}>
                            <span class={styles.ccResultLabel}>Potential Profit</span>
                            <span class="{styles.ccResultValue} {styles.ccResultTp}">{formatUsd(proj.entry_2.potential_profit)}</span>
                        </div>
                        <div class={styles.ccResultItem}>
                            <span class={styles.ccResultLabel}>Fees</span>
                            <span class="{styles.ccResultValue} {styles.ccResultFee}">{formatUsd(proj.entry_2.fees)}</span>
                        </div>
                        <div class={styles.ccResultItem}>
                            <span class={styles.ccResultLabel}>Net Profit</span>
                            <span class="{styles.ccResultValue} {proj.entry_2.net_profit > 0 ? styles.ccPnlPos : styles.ccPnlNeg}">
                                {formatUsd(proj.entry_2.net_profit)}
                            </span>
                        </div>
                    </div>
                </div>
            </div>

            <div class={styles.ccTwoCol}>
                <div class={styles.ccCard}>
                    <h3 class={styles.ccCardTitle}>FEE BREAKDOWN</h3>
                    <div class={styles.ccResultGrid}>
                        <div class={styles.ccResultItem}>
                            <span class={styles.ccResultLabel}>Order Type</span>
                            <span class={styles.ccResultValue}>{proj.fee_breakdown.order_type.toUpperCase()}</span>
                        </div>
                        <div class={styles.ccResultItem}>
                            <span class={styles.ccResultLabel}>Effective Fee %</span>
                            <span class={styles.ccResultValue}>{formatPct(proj.fee_breakdown.effective_fee_pct)}</span>
                        </div>
                        <div class={styles.ccResultItem}>
                            <span class={styles.ccResultLabel}>Entry 1 Fees</span>
                            <span class="{styles.ccResultValue} {styles.ccResultFee}">{formatUsd(proj.fee_breakdown.entry_1_fees)}</span>
                        </div>
                        <div class={styles.ccResultItem}>
                            <span class={styles.ccResultLabel}>Entry 2 Fees</span>
                            <span class="{styles.ccResultValue} {styles.ccResultFee}">{formatUsd(proj.fee_breakdown.entry_2_fees)}</span>
                        </div>
                        <div class={styles.ccResultItem}>
                            <span class={styles.ccResultLabel}>Total Commission</span>
                            <span class="{styles.ccResultValue} {styles.ccResultFee}">{formatUsd(proj.fee_breakdown.total_fees)}</span>
                        </div>
                        <div class={styles.ccResultItem}>
                            <span class={styles.ccResultLabel}>Funding Cost</span>
                            <span class={styles.ccResultValue}>{formatUsd(proj.fee_breakdown.funding_cost)}</span>
                        </div>
                        <div class="{styles.ccResultItem} {styles.ccResultFull}">
                            <span class={styles.ccResultLabel}>Min Profit % to Cover Fees</span>
                            <span class="{styles.ccResultValue} {proj.min_profit_pct_to_cover_fees > 3 ? styles.ccFeeWarn : ''}">{formatPct(proj.min_profit_pct_to_cover_fees)}</span>
                        </div>
                    </div>
                </div>

                <div class={styles.ccCard}>
                    <h3 class={styles.ccCardTitle}>SCENARIO PROJECTIONS</h3>
                    <div class={styles.ccResultGrid}>
                        <div class={styles.ccResultItem}>
                            <span class={styles.ccResultLabel}>Max Gain (Gross)</span>
                            <span class="{styles.ccResultValue} {styles.ccResultTp}">{formatUsd(proj.max_gain_scenario)}</span>
                        </div>
                        <div class={styles.ccResultItem}>
                            <span class={styles.ccResultLabel}>Max Loss (Gross)</span>
                            <span class="{styles.ccResultValue} {styles.ccResultSl}">{formatUsd(proj.max_loss_scenario)}</span>
                        </div>
                        <div class="{styles.ccResultItem} {styles.ccResultFull}">
                            <span class={styles.ccResultLabel}>Max Gain NET (after fees)</span>
                            <span class="{styles.ccResultValue} {proj.max_gain_net_after_fees > 0 ? styles.ccPnlPos : styles.ccPnlNeg}">
                                {formatUsd(proj.max_gain_net_after_fees)}
                            </span>
                        </div>
                        <div class="{styles.ccResultItem} {styles.ccResultFull}">
                            <span class={styles.ccResultLabel}>Max Loss NET (with fees)</span>
                            <span class="{styles.ccResultValue} {styles.ccResultSl}">{formatUsd(proj.max_loss_net_after_fees)}</span>
                        </div>
                        <div class="{styles.ccResultItem} {styles.ccResultFull}">
                            <span class={styles.ccResultLabel}>Required Price Move %</span>
                            <span class={styles.ccResultValue}>{formatPct(proj.required_price_move_pct)}</span>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    {/if}
</div>

