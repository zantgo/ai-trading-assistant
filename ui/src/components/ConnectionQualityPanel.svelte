<script lang="ts">
    import type { ConnectionQualityReport, QualityWindow } from '../types';
    import KpiStrip from './KpiStrip.svelte';
    import ExportDataButton from './ExportDataButton.svelte';
    import { buildEngineExport } from '../lib/engineExport';
    import styles from '../styles/engine-dashboard.module.css';

    let activeWindow: QualityWindow = $state('one_hour');
    let report: ConnectionQualityReport | null = $state(null);
    let loading = $state(true);
    let error: string | null = $state(null);

    let pollInterval: ReturnType<typeof setInterval> | null = null;

    async function fetchQuality() {
        try {
            const res = await fetch(`/api/connection-quality?window=${activeWindow}`);
            if (!res.ok) throw new Error(`HTTP ${res.status}`);
            report = await res.json();
            error = null;
        } catch (e) {
            error = e instanceof Error ? e.message : String(e);
        } finally {
            loading = false;
        }
    }

    $effect(() => {
        fetchQuality();
        if (pollInterval) clearInterval(pollInterval);
        pollInterval = setInterval(fetchQuality, 30_000);
        return () => {
            if (pollInterval) clearInterval(pollInterval);
        };
    });

    function isIdleReport(r: ConnectionQualityReport | null): boolean {
        return r !== null && r.uptime_pct === 0 && r.disconnect_count === 0 && r.score === 100;
    }
    const isIdle = $derived(isIdleReport(report));

    function scoreColor(score: number): string {
        if (score >= 90) return '#22c55e';
        if (score >= 75) return '#84cc16';
        if (score >= 50) return '#f59e0b';
        return '#ef4444';
    }

    function uptimeColor(pct: number): string {
        if (pct >= 99) return '#22c55e';
        if (pct >= 95) return '#f59e0b';
        return '#ef4444';
    }

    function buildExport(): string {
        return buildEngineExport('data_infra', 'connectivity', null, {
            window: activeWindow,
            loading,
            error,
            idle: isIdle,
            report: report ? {
                score: report.score,
                uptime_pct: report.uptime_pct,
                disconnect_count: report.disconnect_count,
                avg_reconnect_ms: report.avg_reconnect_ms,
                total_data_loss_secs: report.total_data_loss_secs,
                reconstructed_candles: report.reconstructed_candles,
                window_start_ms: report.window_start_ms,
                window_end_ms: report.window_end_ms,
            } : null,
        });
    }
</script>

<div style="display:flex; flex-direction:column; gap:16px">
    <div style="display:flex; align-items:center; justify-content:space-between; flex-wrap:wrap; gap:8px">
        <div class={styles.pills} role="tablist" aria-label="Quality window">
            <button class="{styles.pill} {activeWindow === 'one_hour' ? styles.pillActive : ''}" role="tab" aria-selected={activeWindow === 'one_hour'} onclick={() => activeWindow = 'one_hour'}>1h</button>
            <button class="{styles.pill} {activeWindow === 'six_hour' ? styles.pillActive : ''}" role="tab" aria-selected={activeWindow === 'six_hour'} onclick={() => activeWindow = 'six_hour'}>6h</button>
            <button class="{styles.pill} {activeWindow === 'twenty_four_hour' ? styles.pillActive : ''}" role="tab" aria-selected={activeWindow === 'twenty_four_hour'} onclick={() => activeWindow = 'twenty_four_hour'}>24h</button>
        </div>
        <ExportDataButton onExport={buildExport} title="Copy all Connection Quality data as JSON" />
    </div>

    {#if loading}
        <div class={styles.empty}>Loading…</div>
    {:else if error}
        <div class="{styles.alertBanner} {styles.alertError}">Error: {error}</div>
    {:else if report}
        {#if isIdle}
            <div class={styles.empty}>Waiting for events — no connections recorded yet</div>
        {:else}
            <KpiStrip items={[
                { label: 'Score', value: report.score.toFixed(1), sub: 'composite 0–100', color: scoreColor(report.score) },
                { label: 'Uptime', value: `${report.uptime_pct.toFixed(2)}%`, sub: 'window uptime', color: uptimeColor(report.uptime_pct) },
                { label: 'Disconnects', value: String(report.disconnect_count), sub: 'within window' },
                { label: 'Avg Reconnect', value: `${report.avg_reconnect_ms.toFixed(0)}ms`, sub: 'per reconnect' },
                { label: 'Data Loss', value: `${report.total_data_loss_secs}s`, sub: 'total downtime' },
                { label: 'Reconstructed', value: String(report.reconstructed_candles), sub: 'gap-filled candles' },
            ]} />
        {/if}
    {:else}
        <div class={styles.empty}>No data</div>
    {/if}
</div>
