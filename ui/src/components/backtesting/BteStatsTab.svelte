<script lang="ts">
    // BteStatsTab (PAE) — the NHST treatment of the last run: t-test,
    // Monte Carlo sign randomization, α, and the edge verdict — the same
    // significance machinery the live PAE applies.
    import styles from '../../styles/engine-dashboard.module.css';
    import { fmtNum } from '../../lib/format';

    interface Props {
        stats: any;
        summary: { total_trades: number } | null;
    }

    let { stats, summary }: Props = $props();

    const rows = $derived.by(() => {
        if (!stats) return [];
        return [
            { label: 'Sample Size', value: String(summary?.total_trades ?? stats.total_trades ?? 0) },
            { label: 't-statistic', value: fmtNum(stats.t_statistic, 4) },
            { label: 't-test p-value', value: fmtNum(stats.p_value, 6) },
            { label: 'Monte Carlo p', value: fmtNum(stats.p_mc, 6) },
            { label: 'Monte Carlo runs', value: (stats.monte_carlo_runs ?? 0).toLocaleString() },
            { label: 'Significance level α', value: fmtNum(stats.alpha, 3) },
            { label: 'Significant?', value: stats.is_significant ? 'YES' : 'NO', color: stats.is_significant ? '#22c55e' : '#ef4444' },
        ];
    });
</script>

<div class={styles.card}>
    <h3 class={styles.cardTitle} style="margin-top:0">PAE · Statistical Treatment</h3>
    <p class={styles.infoLine}>
        The edge verdict: a result is significant when BOTH the one-tailed t-test p-value and
        the Monte Carlo sign-randomization p-value fall below α (configured in
        [workspace.analytics]). Below the minimum trade count no verdict is issued.
    </p>

    {#if !stats}
        <div class={styles.empty} style="height:120px; display:flex; align-items:center; justify-content:center">
            No statistics. Run a backtest first (Overview tab).
        </div>
    {:else}
        <div class="{styles.alertBanner} {stats.is_significant ? styles.alertWarn : styles.alertError}">
            EDGE VERDICT — {String(stats.classification).replace(/([A-Z])/g, ' $1').trim().toUpperCase()}
        </div>
        <table class={styles.table} style="margin-top:12px">
            <tbody>
                {#each rows as r (r.label)}
                    <tr>
                        <td>{r.label}</td>
                        <td class={styles.tdRight} style={r.color ? `color:${r.color}` : ''}>{r.value}</td>
                    </tr>
                {/each}
            </tbody>
        </table>
        <p class={styles.infoLine} style="margin-top:12px">
            Methodology: one-tailed t-test on the trade PnL sample; empirical p via 10,000
            sign-randomizations (fixed seed — deterministic); verdicts mirror the live PAE
            classifier exactly.
        </p>
    {/if}
</div>
