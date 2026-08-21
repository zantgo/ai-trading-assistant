<script lang="ts">
    // BteExecutionsTab (TAE) — the simulated execution log: the same
    // trade-log vocabulary the PAE Backtest tab used, fed by the study.
    import styles from '../../styles/engine-dashboard.module.css';
    import { fmtNum, fmtSigned } from '../../lib/format';
    import type { BteResult } from './BacktestingDashboard.svelte';

    interface Props {
        trades: BteResult['trades'];
        result: BteResult | null;
    }

    let { trades, result }: Props = $props();
</script>

<div class={styles.card}>
    <h3 class={styles.cardTitle} style="margin-top:0">TAE · Simulated Executions</h3>
    <p class={styles.infoLine}>
        Every simulated close from the replay — the same setup executor + unified paper engine
        the live session runs, driven by the recorded/archived decision stream
        ({result ? `run #${result.backtest_id}` : 'no run loaded'}).
    </p>

    {#if trades.length === 0}
        <div class={styles.empty} style="height:120px; display:flex; align-items:center; justify-content:center">
            No trades in this window. Run a backtest first (Overview tab).
        </div>
    {:else}
        <table class={styles.table}>
            <thead>
                <tr>
                    <th>Time</th><th>Dir</th><th class={styles.tdRight}>Entry</th>
                    <th class={styles.tdRight}>Exit</th><th class={styles.tdRight}>Size</th>
                    <th class={styles.tdRight}>P&L</th><th>Exit Reason</th>
                </tr>
            </thead>
            <tbody>
                {#each trades as tr, i (i)}
                    <tr>
                        <td>{new Date(tr.timestamp).toLocaleString()}</td>
                        <td style="color:{tr.direction === 'LONG' ? '#22c55e' : '#ef4444'}">{tr.direction}</td>
                        <td class={styles.tdRight}>${fmtNum(tr.entry_price)}</td>
                        <td class={styles.tdRight}>${fmtNum(tr.exit_price)}</td>
                        <td class={styles.tdRight}>{fmtNum(tr.size, 4)}</td>
                        <td class={styles.tdRight} style="color:{tr.pnl >= 0 ? '#22c55e' : '#ef4444'}">{fmtSigned(tr.pnl)}</td>
                        <td class={styles.tdMono}>{tr.exit_reason}</td>
                    </tr>
                {/each}
            </tbody>
        </table>
    {/if}
</div>
