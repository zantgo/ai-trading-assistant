<script lang="ts">
    // PAE L4 — Performance tab (regime-performance map + optimizer
    // recommendations). Renamed from "Regime Map" to its layer name.
    import styles from '../../styles/engine-dashboard.module.css';
    import local from '../PerformanceDashboard.module.css';
    import { fmtNum, fmtPct, fmtSigned } from '../../lib/format';
    import type { PerformanceMatrixRow, OptimizationReport } from '../../types/analytics';

    let { performanceRows, optimizationReport }: {
        performanceRows: PerformanceMatrixRow[];
        optimizationReport: OptimizationReport | null;
    } = $props();

    function compatibilityBadge(c: string): string {
        const map: Record<string, string> = {
            Strong: local.labelStrong,
            Favorable: local.labelFavorable,
            Marginal: local.labelMarginal,
            Avoid: local.labelAvoid,
        };
        return map[c] ?? local.labelAvoid;
    }

    function pnlClass(v: number): string {
        if (v > 0) return local.statPositive;
        if (v < 0) return local.statNegative;
        return local.statNeutral;
    }
</script>

<div class={styles.card}>
    <h3 class={styles.cardTitle}>Regime-Performance Map</h3>
    <p class={styles.infoLine}>
        Strategy performance segmented by market regime at trade entry. Regimes resolved from
        market_snapshots.
    </p>
    {#if performanceRows.length === 0}
        <div class={styles.empty}>No regime-performance data available. Requires closed trades with market regime context.</div>
    {:else}
        <div class={local.regimeGrid}>
            {#each performanceRows as row}
                <div class={local.regimeCard}>
                    <div class={local.regimeName}>
                        {row.regime}
                        <span class="{local.regimeLabel} {compatibilityBadge(row.compatibility_label)}" style="float:right">{row.compatibility_label}</span>
                    </div>
                    <div class={local.regimeStats}>
                        <span>{row.trade_count} trades</span>
                        <span>WR: {fmtNum(row.win_rate)}%</span>
                        <span>PF: {fmtNum(row.profit_factor)}</span>
                    </div>
                    <div class={local.regimeStats}>
                        <span>Avg R: {fmtNum(row.avg_r_multiple)}</span>
                        <span class={pnlClass(row.total_pnl)}>P&L: {fmtSigned(row.total_pnl)}</span>
                    </div>
                </div>
            {/each}
        </div>
    {/if}

    {#if optimizationReport?.recommendations?.length}
        <h3 class={styles.cardTitle} style="margin-top:16px">Recommendations</h3>
        {#each optimizationReport.recommendations as rec}
            <div class={local.recommendation}>{rec}</div>
        {/each}
    {/if}
</div>
