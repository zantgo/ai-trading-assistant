<script lang="ts">
    // PAE L2 — Strategy Analytics tab: NHST table per setup type.
    import styles from '../../styles/engine-dashboard.module.css';
    import local from '../PerformanceDashboard.module.css';
    import { fmtNum, fmtPct, fmtSigned } from '../../lib/format';
    import type { StrategyAnalyticsRow } from '../../types/analytics';

    let { strategyRows }: { strategyRows: StrategyAnalyticsRow[] } = $props();

    function classificationBadge(c: string): string {
        const map: Record<string, string> = {
            StrongEdge: styles.badgeLong,
            ModerateEdge: styles.badgeLong,
            WeakMarginalEdge: styles.badgeNeutral,
            NoEdgeNegative: styles.badgeError,
            InsufficientData: styles.badgeEmpty,
        };
        return map[c] ?? styles.badgeEmpty;
    }

    function pnlClass(v: number): string {
        if (v > 0) return local.statPositive;
        if (v < 0) return local.statNegative;
        return local.statNeutral;
    }
</script>

<div class={styles.card}>
    <h3 class={styles.cardTitle}>Strategy Analytics</h3>
    <p class={styles.infoLine}>
        Null Hypothesis Significance Testing — determines whether each setup type generates a
        statistically significant positive edge (H₀: μ ≤ 0 vs H₁: μ > 0). An edge is significant at
        α = {fmtNum(strategyRows[0]?.alpha ?? 0.05, 2)} when both p-values are below it; p_mc comes
        from {strategyRows[0]?.monte_carlo_runs?.toLocaleString() ?? '10,000'} Monte Carlo
        sign-randomization runs.
    </p>
    {#if strategyRows.length === 0}
        <div class={styles.empty}>No strategy data available. Trades must be closed for NHST analysis.</div>
    {:else}
        <table class={styles.table}>
            <thead>
                <tr>
                    <th>Setup Type</th><th>Trades</th><th>Win Rate</th><th>Profit Factor</th>
                    <th>Expectancy</th><th>T-Stat</th><th>P-Value</th><th>P_MC (10k)</th><th>Edge</th>
                </tr>
            </thead>
            <tbody>
                {#each strategyRows as row}
                    <tr>
                        <td class={styles.tdMono}>{row.setup_type}</td>
                        <td>{row.total_trades}</td>
                        <td class={pnlClass(row.win_rate - 50)}>{fmtNum(row.win_rate)}%</td>
                        <td>{fmtNum(row.profit_factor)}</td>
                        <td class={pnlClass(row.expectancy)}>{fmtSigned(row.expectancy)}</td>
                        <td>{fmtNum(row.t_statistic)}</td>
                        <td>{fmtNum(row.p_value, 4)}</td>
                        <td>{fmtNum(row.p_mc, 4)}</td>
                        <td>
                            <span class="{styles.badge} {classificationBadge(row.classification)}">{row.classification.replace(/([A-Z])/g, ' $1').trim()}</span>
                            {#if row.is_significant}
                                <span class={styles.badgeLong} style="margin-left:6px; padding:2px 8px; font-size:10px; border-style:solid">sig @ α={fmtNum(row.alpha, 2)}</span>
                            {/if}
                        </td>
                    </tr>
                {/each}
            </tbody>
        </table>
    {/if}
</div>
