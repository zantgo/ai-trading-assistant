<script lang="ts">
    // PAE L3 — Risk Analytics tab.
    import KpiStrip from './../KpiStrip.svelte';
    import styles from '../../styles/engine-dashboard.module.css';
    import local from '../PerformanceDashboard.module.css';
    import { fmtNum, fmtPct } from '../../lib/format';
    import type { RiskAnalyticsRow } from '../../types/analytics';

    let { riskData }: { riskData: RiskAnalyticsRow | null } = $props();

    function sharpeClass(v: number | null): string {
        if (v == null) return local.statNeutral;
        if (v >= 1.0) return local.statPositive;
        if (v >= 0.5) return local.statNeutral;
        return local.statNegative;
    }
</script>

<div class={styles.card}>
    <h3 class={styles.cardTitle}>Risk Analytics</h3>
    <p class={styles.infoLine}>
        Risk-adjusted return metrics computed from portfolio equity history. Sharpe/Sortino
        annualized assuming 365 trading days for crypto.
    </p>
    {#if !riskData}
        <div class={styles.empty}>No risk data available. Requires portfolio equity history.</div>
    {:else}
        {@const rd = riskData}
        <KpiStrip items={[
            { label: 'Sharpe Ratio', value: fmtNum(rd.sharpe_ratio), sub: rd.sharpe_ratio != null && rd.sharpe_ratio > 2 ? 'Excellent' : rd.sharpe_ratio != null && rd.sharpe_ratio >= 1 ? 'Good' : rd.sharpe_ratio != null && rd.sharpe_ratio >= 0.5 ? 'Acceptable' : 'Poor', color: sharpeClass(rd.sharpe_ratio) },
            { label: 'Sortino Ratio', value: fmtNum(rd.sortino_ratio), sub: 'downside only', color: sharpeClass(rd.sortino_ratio) },
            { label: 'Calmar Ratio', value: fmtNum(rd.calmar_ratio), sub: 'return / DD', color: sharpeClass(rd.calmar_ratio) },
            { label: 'Ulcer Index', value: fmtNum(rd.ulcer_index), sub: 'drawdown depth', color: undefined },
            { label: 'Max Drawdown', value: fmtPct(rd.maximum_drawdown_pct), sub: `${rd.drawdown_count} drawdown events`, color: rd.maximum_drawdown_pct > 20 ? local.statNegative : local.statNeutral },
            { label: 'Avg Drawdown', value: fmtPct(rd.average_drawdown_pct), sub: 'per event', color: rd.average_drawdown_pct > 10 ? local.statNegative : local.statNeutral },
            { label: 'Daily Volatility', value: fmtPct(rd.daily_volatility * 100), sub: 'per day', color: undefined },
            { label: 'Downside Deviation', value: fmtPct(rd.downside_deviation * 100), sub: 'negative moves', color: undefined },
            { label: 'VaR 95%', value: fmtPct(rd.value_at_risk_95 * 100), sub: 'worst daily in 95%', color: undefined },
            { label: 'Exp. Shortfall 95%', value: fmtPct(rd.expected_shortfall_95 * 100), sub: 'avg loss beyond VaR', color: undefined },
        ]} />
    {/if}
</div>
