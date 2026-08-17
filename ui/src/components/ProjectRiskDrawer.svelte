<!--
    ProjectRiskDrawer — "PROJECT RISK AND RETURN" expandable panel (v7.0).

    An on-demand, stateless what-if calculator mounted on the
    Recommendation panel. It auto-pulls the active setup's geometry
    (entry / stop-loss / take-profit midpoints), defaults capital to the
    active risk profile (falling back to $100) and leverage to the saved
    risk profile, and reuses the existing `/api/risk/calculate` endpoint
    (with payload overrides) — no new server-side math. The derived
    projection state is lifted to the parent so the export JSON carries
    the same numbers the drawer renders.
-->
<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import { buildProjection, type ProjectionSetup, type ProjectionState } from '../lib/projection';
    import { fmtPrice } from '../lib/telemetry';
    import styles from './ProjectRiskDrawer.module.css';

    const app = useAppStore();

    let {
        setup,
        markPrice,
        onProjection,
    }: {
        setup: ProjectionSetup | null;
        markPrice: number;
        onProjection: (state: ProjectionState) => void;
    } = $props();

    $effect(() => {
        app.fetchRiskProfiles();
    });

    const activeProfile = $derived(
        app.riskProfiles.find((p) => p.id === app.activeRiskProfileId),
    );

    let capital = $state(100);
    let leverage = $state(10);
    let commissionPct = $state(0.06);
    let defaultsApplied = false;

    // Defaults ride the active risk profile — applied once when it loads
    // so a profile arriving mid-session never clobbers operator input.
    $effect(() => {
        const p = activeProfile;
        if (!p || defaultsApplied) return;
        defaultsApplied = true;
        capital = parseFloat(p.capital) || 100;
        leverage = p.leverage || 10;
        commissionPct = parseFloat(p.commission_pct) || 0.06;
    });

    const refPrice = $derived(markPrice || 0);

    function fmtPx(n: number | null | undefined): string {
        if (n == null || !isFinite(n) || n <= 0) return '\u2014';
        return `$${fmtPrice(n, refPrice)}`;
    }

    function fmtUsd(n: number | null | undefined, signed = false): string {
        if (n == null || !isFinite(n)) return '\u2014';
        const v = n.toFixed(2);
        return signed && n > 0 ? `+$${v}` : `$${v}`;
    }

    let debounce: ReturnType<typeof setTimeout> | null = null;

    function runCalc() {
        if (!setup) return;
        if (debounce) clearTimeout(debounce);
        debounce = setTimeout(() => {
            app.calculateRisk({ capital, leverage, commissionPct });
        }, 120);
    }

    // Prefill the store's operation fields from the active setup and
    // recalculate whenever the setup or the operator's inputs change.
    $effect(() => {
        const s = setup;
        if (!s) return;
        app.riskDirection = s.direction;
        app.riskEntryPrice = String(s.entry);
        app.riskStopLoss = String(s.stopLoss);
        app.riskTakeProfit = String(s.takeProfit);
        runCalc();
    });

    // Lift the derived projection to the parent (feeds the export JSON).
    $effect(() => {
        const calc = app.riskCalculation;
        const s = setup;
        if (!calc || !s) return;
        onProjection(buildProjection(s, capital, leverage, commissionPct, calc));
    });
</script>

{#if setup}
    <div class={styles.drawer} aria-label="Project Risk and Return">
        <div class={styles.drawerHead}>
            <span class={styles.drawerTitle}>PROJECTED RISK AND RETURN</span>
            <span class={styles.drawerSetup}>
                {setup.direction} · entry {fmtPx(setup.entry)} · sl {fmtPx(setup.stopLoss)} · tp {fmtPx(setup.takeProfit)}
            </span>
        </div>

        <div class={styles.drawerFields}>
            <label class={styles.field} for="prr-capital">
                <span class={styles.fieldLabel}>Capital Allocation</span>
                <div class={styles.fieldRow}>
                    <span class={styles.fieldPrefix}>$</span>
                    <input
                        id="prr-capital"
                        type="number"
                        min="1"
                        step="any"
                        class={styles.fieldInput}
                        bind:value={capital}
                        oninput={runCalc}
                    />
                </div>
            </label>
            <label class={styles.field} for="prr-leverage">
                <span class={styles.fieldLabel}>Leverage</span>
                <div class={styles.fieldRow}>
                    <span class={styles.fieldPrefix}>×</span>
                    <input
                        id="prr-leverage"
                        type="number"
                        min="1"
                        step="any"
                        class={styles.fieldInput}
                        bind:value={leverage}
                        oninput={runCalc}
                    />
                </div>
            </label>
        </div>

        {#if app.riskCalculating}
            <p class={styles.drawerNote}>Calculating…</p>
        {:else if app.riskCalculation}
            <div class={styles.drawerGrid}>
                <div class={styles.resultItem}>
                    <span class={styles.resultLabel}>Position Size</span>
                    <span class={styles.resultValue}>{(parseFloat(app.riskCalculation.position_size_units) || 0).toFixed(6)}</span>
                </div>
                <div class={styles.resultItem}>
                    <span class={styles.resultLabel}>Notional Value</span>
                    <span class={styles.resultValue}>{fmtUsd(parseFloat(app.riskCalculation.position_notional) || 0)}</span>
                </div>
                <div class={styles.resultItem}>
                    <span class={styles.resultLabel}>Margin Required</span>
                    <span class={styles.resultValue}>{fmtUsd(parseFloat(app.riskCalculation.margin_required) || 0)}</span>
                </div>
                <div class={styles.resultItem}>
                    <span class={styles.resultLabel}>Entry Fee (est.)</span>
                    <span class={styles.resultValue}>{fmtUsd(parseFloat(app.riskCalculation.position_notional) * Math.max(0, commissionPct) / 100 || 0)}</span>
                </div>
                <div class={styles.resultItem}>
                    <span class={styles.resultLabel}>Exit Fee (est.)</span>
                    <span class={styles.resultValue}>{fmtUsd(parseFloat(app.riskCalculation.position_notional) * Math.max(0, commissionPct) / 100 || 0)}</span>
                </div>
                <div class={styles.resultItem}>
                    <span class={styles.resultLabel}>Total Costs</span>
                    <span class={styles.resultValue}>{fmtUsd(parseFloat(app.riskCalculation.total_fees) || 0)}</span>
                </div>
                <div class={styles.resultItem}>
                    <span class={styles.resultLabel}>Liquidation Price</span>
                    <span class={styles.resultValue}>{fmtPx(parseFloat(app.riskCalculation.liquidation_price) || 0)}</span>
                </div>
                <div class={styles.resultItem}>
                    <span class={styles.resultLabel}>Net Profit</span>
                    <span class="{styles.resultValue} {parseFloat(app.riskCalculation.net_pnl) > 0 ? styles.resultValuePos : parseFloat(app.riskCalculation.net_pnl) < 0 ? styles.resultValueNeg : ''}">
                        {fmtUsd(parseFloat(app.riskCalculation.net_pnl) || 0, true)}
                    </span>
                </div>
                <div class={styles.resultItem}>
                    <span class={styles.resultLabel}>Return on Investment</span>
                    <span
                        class="{styles.resultValue} {(() => {
                            const margin = parseFloat(app.riskCalculation.margin_required) || 0;
                            const pnl = parseFloat(app.riskCalculation.net_pnl) || 0;
                            const roi = margin > 0 ? (pnl / margin) * 100 : null;
                            return roi != null && roi > 0 ? styles.resultValuePos : roi != null && roi < 0 ? styles.resultValueNeg : '';
                        })()}"
                    >
                        {(() => {
                            const margin = parseFloat(app.riskCalculation.margin_required) || 0;
                            const pnl = parseFloat(app.riskCalculation.net_pnl) || 0;
                            const roi = margin > 0 ? (pnl / margin) * 100 : null;
                            return roi != null ? `${roi >= 0 ? '+' : ''}${roi.toFixed(1)}%` : '\u2014';
                        })()}
                    </span>
                </div>
            </div>
        {:else}
            <p class={styles.drawerNote}>Waiting for a valid setup — enter capital and leverage to project the trade economics.</p>
        {/if}
    </div>
{/if}
