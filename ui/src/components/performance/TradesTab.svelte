<script lang="ts">
    // PAE L1 — Trade Analytics tab: reconstructed closed trades.
    import styles from '../../styles/engine-dashboard.module.css';
    import local from '../PerformanceDashboard.module.css';
    import { fmtNum, fmtPct, fmtSigned } from '../../lib/format';
    import type { TradeAnalyticsRecord } from '../../types/analytics';

    let { tradeRecords }: { tradeRecords: TradeAnalyticsRecord[] } = $props();

    function pnlClass(v: number): string {
        if (v > 0) return local.statPositive;
        if (v < 0) return local.statNegative;
        return local.statNeutral;
    }
</script>

<div class={styles.card}>
    <h3 class={styles.cardTitle}>Trade Analytics</h3>
    <p class={styles.infoLine}>Reconstructed closed trades with execution efficiency metrics.</p>
    {#if tradeRecords.length === 0}
        <div class={styles.empty}>No trade data available.</div>
    {:else}
        <table class={styles.table}>
            <thead>
                <tr>
                    <th>Trade ID</th><th>Symbol</th><th>Dir</th><th>Hold</th>
                    <th class={styles.tdRight}>Gross P&L</th><th class={styles.tdRight}>Net P&L</th>
                    <th class={styles.tdRight}>ROI</th><th class={styles.tdRight}>MFE</th>
                    <th class={styles.tdRight}>MAE</th><th>Flat</th>
                </tr>
            </thead>
            <tbody>
                {#each tradeRecords as t}
                    <tr>
                        <td>{t.trade_id}</td>
                        <td class={styles.tdMono}>{t.symbol}</td>
                        <td class={t.direction === 'LONG' ? local.statPositive : local.statNegative}>{t.direction}</td>
                        <td>{t.hold_time_seconds < 3600 ? Math.round(t.hold_time_seconds / 60) + 'm' : Math.round(t.hold_time_seconds / 3600) + 'h'}</td>
                        <td class={styles.tdRight + ' ' + pnlClass(t.gross_pnl)}>{fmtSigned(t.gross_pnl)}</td>
                        <td class={styles.tdRight + ' ' + pnlClass(t.net_pnl)}>{fmtSigned(t.net_pnl)}</td>
                        <td class={styles.tdRight + ' ' + pnlClass(t.roi_pct)}>{fmtPct(t.roi_pct)}</td>
                        <td class={styles.tdRight + ' ' + local.statPositive}>{fmtNum(t.mfe)}</td>
                        <td class={styles.tdRight + ' ' + local.statNegative}>{fmtNum(t.mae)}</td>
                        <td>{t.flat_trade ? 'Yes' : ''}</td>
                    </tr>
                {/each}
            </tbody>
        </table>
    {/if}
</div>
