<script lang="ts">
    // BteHistoryTab — the persisted run list (instance-independent):
    // loads any run's study into the report.
    import { onMount } from 'svelte';
    import styles from '../../styles/engine-dashboard.module.css';
    import { fmtNum } from '../../lib/format';

    interface Props {
        loadRun: (run: { id: number }) => Promise<void>;
        activeRunId: number | null;
    }

    let { loadRun, activeRunId }: Props = $props();

    interface RunRow {
        id: number;
        created_at: number;
        instance_id: string | null;
        mode: string | null;
        params: { symbol?: string; timeframe_secs?: number; from_secs?: number; to_secs?: number; initial_capital?: number };
        summary: {
            total_trades?: number; win_rate?: number; profit_factor?: number | null;
            max_drawdown_pct?: number;
        };
    }

    let runs = $state<RunRow[]>([]);
    let loading = $state(true);
    let error = $state('');

    async function fetchRuns() {
        loading = true;
        try {
            const res = await fetch('/api/backtest/list?limit=50');
            if (!res.ok) throw new Error('HTTP ' + res.status);
            runs = await res.json();
        } catch (e: any) {
            error = e?.message ?? 'Failed to load runs';
        } finally {
            loading = false;
        }
    }

    onMount(fetchRuns);

    function modeBadge(mode: string | null): string {
        if (mode === 'historical') return styles.badgeLong;
        if (mode === 'recorded') return styles.badgeNeutral;
        return styles.badgeEmpty;
    }
</script>

<div class={styles.card}>
    <div style="display:flex; align-items:center; justify-content:space-between; flex-wrap:wrap; gap:8px">
        <h3 class={styles.cardTitle} style="margin:0">Backtest History</h3>
        <button class={styles.btn} onclick={fetchRuns} disabled={loading}>{loading ? 'Loading…' : 'Refresh'}</button>
    </div>
    <p class={styles.infoLine}>
        Every run persists its summary here plus normalized trades / equity / portfolio /
        signals / metrics rows in the data-science tables. Click a run to load its study.
    </p>

    {#if error}
        <div class="{styles.alertBanner} {styles.alertError}" style="margin-top:10px">{error}</div>
    {/if}

    {#if runs.length === 0 && !loading}
        <div class={styles.empty} style="height:120px; display:flex; align-items:center; justify-content:center">
            No backtests yet — run one from the Overview tab.
        </div>
    {:else}
        <table class={styles.table} style="margin-top:12px">
            <thead>
                <tr>
                    <th>#</th><th>Mode</th><th>Instance</th><th>Symbol · TF</th>
                    <th>Window</th><th class={styles.tdRight}>Trades</th>
                    <th class={styles.tdRight}>Win Rate</th><th class={styles.tdRight}>PF</th>
                    <th class={styles.tdRight}>Max DD</th><th></th>
                </tr>
            </thead>
            <tbody>
                {#each runs as r (r.id)}
                    <tr class={r.id === activeRunId ? styles.rowActive ?? '' : ''} style={r.id === activeRunId ? 'outline:1px solid rgba(59,130,246,0.4)' : ''}>
                        <td class={styles.tdMono}>{r.id}</td>
                        <td><span class="{styles.badge} {modeBadge(r.mode)}">{r.mode ?? '—'}</span></td>
                        <td class={styles.tdMono}>{r.instance_id ?? '—'}</td>
                        <td class={styles.tdMono}>{r.params?.symbol ?? '—'} · {r.params?.timeframe_secs ?? '—'}s</td>
                        <td>{r.params?.from_secs ? new Date(r.params.from_secs * 1000).toLocaleDateString() : '—'} → {r.params?.to_secs ? new Date(r.params.to_secs * 1000).toLocaleDateString() : '—'}</td>
                        <td class={styles.tdRight}>{r.summary?.total_trades ?? 0}</td>
                        <td class={styles.tdRight}>{fmtNum(r.summary?.win_rate)}%</td>
                        <td class={styles.tdRight}>{fmtNum(r.summary?.profit_factor)}</td>
                        <td class={styles.tdRight}>-{fmtNum(r.summary?.max_drawdown_pct)}%</td>
                        <td><button class={styles.btn} onclick={() => loadRun(r)}>Load</button></td>
                    </tr>
                {/each}
            </tbody>
        </table>
    {/if}
</div>
