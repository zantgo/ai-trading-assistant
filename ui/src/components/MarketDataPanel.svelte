<script lang="ts">
    // MarketDataPanel — DIE L2 tab: per-instance × slot candle-pipeline
    // state (pipeline lifecycle, buffer depth, last completed close,
    // reconstructed candles). Served by GET /api/system/pipelines.
    import { onMount } from 'svelte';
    import { formatRelativeTime } from '../lib/relTime';
    import ExportDataButton from './ExportDataButton.svelte';
    import { buildEngineExport } from '../lib/engineExport';
    import styles from '../styles/engine-dashboard.module.css';

    interface PipelineRow {
        pair: string;
        slot: string;
        timeframe_secs: number;
        pipeline_state: string;
        buffer_depth: number;
        buffer_size: number;
        last_completed_close: string;
        last_completed_ts: number;
        reconstructed_candles: number;
    }

    let rows: PipelineRow[] = $state([]);
    let loading = $state(true);
    let error: string | null = $state(null);
    let pollInterval: ReturnType<typeof setInterval> | null = null;

    async function fetchPipelines() {
        try {
            const res = await fetch('/api/system/pipelines');
            if (!res.ok) throw new Error(`HTTP ${res.status}`);
            const data = await res.json();
            rows = data?.pipelines ?? [];
            error = null;
        } catch (e) {
            error = e instanceof Error ? e.message : String(e);
        } finally {
            loading = false;
        }
    }

    onMount(() => {
        fetchPipelines();
        pollInterval = setInterval(fetchPipelines, 15_000);
        return () => {
            if (pollInterval) clearInterval(pollInterval);
        };
    });

    function stateBadge(s: string): string {
        switch (s) {
            case 'LIVE': return styles.badgeLong;
            case 'LOADING': return styles.badgeNeutral;
            case 'STALE': return styles.badgeNeutral;
            case 'FAILED': return styles.badgeError;
            default: return styles.badgeEmpty;
        }
    }

    const ordered = $derived(
        rows.slice().sort((a, b) => a.pair.localeCompare(b.pair) || a.slot.localeCompare(b.slot)),
    );

    function buildExport(): string {
        return buildEngineExport('data_infra', 'market_data', null, {
            loading,
            error,
            pipelines: ordered.map((r) => ({
                pair: r.pair,
                slot: r.slot,
                timeframe_secs: r.timeframe_secs,
                pipeline_state: r.pipeline_state,
                buffer_depth: r.buffer_depth,
                buffer_size: r.buffer_size,
                last_completed_close: r.last_completed_close,
                last_completed_ts: r.last_completed_ts,
                reconstructed_candles: r.reconstructed_candles,
            })),
        });
    }
</script>

<div style="display:flex; flex-direction:column; gap:16px">
    <div style="display:flex; align-items:center; justify-content:flex-end">
        <ExportDataButton onExport={buildExport} title="Copy all Market Data (pipelines) as JSON" />
    </div>
    {#if loading}
        <div class={styles.empty}>Loading…</div>
    {:else if error}
        <div class="{styles.alertBanner} {styles.alertError}">Error: {error}</div>
    {:else if ordered.length === 0}
        <div class={styles.empty}>No pipelines running. Launch an instance to see candle-pipeline state.</div>
    {:else}
        <div class={styles.card}>
            <h3 class={styles.cardTitle}>Candle Pipelines</h3>
            <p class={styles.infoLine}>
                L2 market data — each slot aggregates normalized events into UTC-aligned OHLCV
                candles. The buffer fills to its configured size before the pipeline goes LIVE.
            </p>
            <table class={styles.table}>
                <thead>
                    <tr>
                        <th>Pair</th><th>Slot</th><th>TF</th><th>State</th>
                        <th class={styles.tdRight}>Buffer</th><th class={styles.tdRight}>Last Close</th>
                        <th class={styles.tdRight}>Updated</th><th class={styles.tdRight}>Reconstructed</th>
                    </tr>
                </thead>
                <tbody>
                    {#each ordered as r (r.pair + r.slot)}
                        <tr>
                            <td class={styles.tdMono}>{r.pair}</td>
                            <td class={styles.tdMono}>{r.slot}</td>
                            <td class={styles.tdMono}>{r.timeframe_secs}s</td>
                            <td><span class="{styles.badge} {stateBadge(r.pipeline_state)}">{r.pipeline_state}</span></td>
                            <td class={styles.tdRight}>{r.buffer_depth} / {r.buffer_size}</td>
                            <td class={styles.tdRight}>{r.last_completed_close || '—'}</td>
                            <td class={styles.tdRight}>{r.last_completed_ts > 0 ? formatRelativeTime(r.last_completed_ts).label : '—'}</td>
                            <td class={styles.tdRight} style="color:{r.reconstructed_candles > 0 ? '#f59e0b' : 'rgba(255,255,255,0.55)'}">{r.reconstructed_candles}</td>
                        </tr>
                    {/each}
                </tbody>
            </table>
        </div>
    {/if}
</div>
