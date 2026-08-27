<script lang="ts">
    // BteSignalsTab (MME) — the simulated decision stream: per-tick bias,
    // trade readiness and opportunity scores persisted to the DS tables.
    import styles from '../../styles/engine-dashboard.module.css';
    import { fmtNum } from '../../lib/format';

    interface Props {
        signals: { ts_secs: number; timeframe_secs: number; label: string; kind: string; value: string }[];
        runId: number | null;
    }

    let { signals, runId }: Props = $props();

    const biasCounts = $derived.by(() => {
        const map = new Map<string, number>();
        for (const s of signals) {
            if (s.kind === 'bias') map.set(s.value, (map.get(s.value) ?? 0) + 1);
        }
        return [...map.entries()].sort((a, b) => b[1] - a[1]);
    });

    const readinessCounts = $derived.by(() => {
        const map = new Map<string, number>();
        for (const s of signals) {
            if (s.kind === 'trade_readiness') map.set(s.value, (map.get(s.value) ?? 0) + 1);
        }
        return [...map.entries()].sort((a, b) => b[1] - a[1]);
    });

    const totalBias = $derived(biasCounts.reduce((acc, [, n]) => acc + n, 0));
    const totalReadiness = $derived(readinessCounts.reduce((acc, [, n]) => acc + n, 0));
</script>

<div class={styles.card}>
    <h3 class={styles.cardTitle} style="margin-top:0">MME · Simulated Signals</h3>
    <p class={styles.infoLine}>
        The synthesized decision stream of the replayed MME pipeline
        ({runId ? `run #${runId}` : 'no run loaded'}) — every tick the MTF
        synthesizer emitted a bias, a trade readiness, and an opportunity score.
    </p>

    {#if signals.length === 0}
        <div class={styles.empty} style="height:120px; display:flex; align-items:center; justify-content:center">
            No decision snapshots persisted. Run a backtest first (Overview tab).
        </div>
    {:else}
        <div style="display:grid; grid-template-columns:repeat(auto-fit, minmax(260px, 1fr)); gap:16px">
            <div>
                <h4 class={styles.cardTitle}>Direction Bias Distribution</h4>
                <table class={styles.table}>
                    <tbody>
                        {#each biasCounts as [bias, count] (bias)}
                            <tr>
                                <td class={styles.tdMono}>{bias}</td>
                                <td class={styles.tdRight}>{count}</td>
                                <td class={styles.tdRight}>{totalBias > 0 ? fmtNum((count / totalBias) * 100) + '%' : '—'}</td>
                            </tr>
                        {/each}
                    </tbody>
                </table>
            </div>
            <div>
                <h4 class={styles.cardTitle}>Trade Readiness Distribution</h4>
                <table class={styles.table}>
                    <tbody>
                        {#each readinessCounts as [readiness, count] (readiness)}
                            <tr>
                                <td class={styles.tdMono}>{readiness}</td>
                                <td class={styles.tdRight}>{count}</td>
                                <td class={styles.tdRight}>{totalReadiness > 0 ? fmtNum((count / totalReadiness) * 100) + '%' : '—'}</td>
                            </tr>
                        {/each}
                    </tbody>
                </table>
            </div>
        </div>
        <p class={styles.infoLine} style="margin-top:12px">
            {signals.length.toLocaleString()} decision snapshots persisted to
            <span class={styles.tdMono}> backtest_signals</span> — queryable for later
            data-science work.
        </p>
    {/if}
</div>
