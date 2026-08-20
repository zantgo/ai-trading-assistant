<script lang="ts">
    // DistributionPanel — DIE L4 tab: egress telemetry — pipeline latencies,
    // ingest skew and the connected WebSocket client count.
    import { onMount } from 'svelte';
    import KpiStrip from './KpiStrip.svelte';
    import ExportDataButton from './ExportDataButton.svelte';
    import { buildEngineExport } from '../lib/engineExport';
    import styles from '../styles/engine-dashboard.module.css';

    interface DistributionReport {
        observation_loop_latency_ms: number;
        ingest_skew_ms: number;
        system_heartbeat_latency_ms: number;
        ws_clients_connected: number;
    }

    let report: DistributionReport | null = $state(null);
    let loading = $state(true);
    let error: string | null = $state(null);
    let pollInterval: ReturnType<typeof setInterval> | null = null;

    async function fetchReport() {
        try {
            const res = await fetch('/api/system/distribution');
            if (!res.ok) throw new Error(`HTTP ${res.status}`);
            report = (await res.json()) as DistributionReport;
            error = null;
        } catch (e) {
            error = e instanceof Error ? e.message : String(e);
        } finally {
            loading = false;
        }
    }

    onMount(() => {
        fetchReport();
        pollInterval = setInterval(fetchReport, 15_000);
        return () => {
            if (pollInterval) clearInterval(pollInterval);
        };
    });

    function latencyColor(ms: number): string {
        if (ms < 10) return '#22c55e';
        if (ms < 50) return '#f59e0b';
        return '#ef4444';
    }

    function buildExport(): string {
        return buildEngineExport('data_infra', 'distribution', null, {
            loading,
            error,
            report,
        });
    }
</script>

<div style="display:flex; flex-direction:column; gap:16px">
    <div style="display:flex; align-items:center; justify-content:flex-end">
        <ExportDataButton onExport={buildExport} title="Copy all Distribution telemetry as JSON" />
    </div>
    {#if loading}
        <div class={styles.empty}>Loading…</div>
    {:else if error}
        <div class="{styles.alertBanner} {styles.alertError}">Error: {error}</div>
    {:else if report}
        <KpiStrip items={[
            { label: 'Observation Loop', value: `${report.observation_loop_latency_ms}ms`, sub: 'analyzer cadence', color: latencyColor(report.observation_loop_latency_ms) },
            { label: 'Ingest Skew', value: `${report.ingest_skew_ms}ms`, sub: 'event → pipeline', color: latencyColor(report.ingest_skew_ms) },
            { label: 'Heartbeat Latency', value: `${report.system_heartbeat_latency_ms}ms`, sub: 'system tick', color: latencyColor(report.system_heartbeat_latency_ms) },
            { label: 'WS Clients', value: String(report.ws_clients_connected), sub: 'connected dashboards' },
        ]} />
        <div class={styles.card}>
            <h3 class={styles.cardTitle}>Distribution Contract</h3>
            <p class={styles.infoLine}>
                DIE L4 publishes validated <code>NormalizedCandle</code> frames to the Candle
                Aggregator for higher-timeframe rollup; the <code>MarketSnapshot</code> broadcast
                (MME L1 artifact) carries indicators and matrices to the UI WebSocket and the
                telemetry store. The latencies above measure that egress path.
            </p>
        </div>
    {:else}
        <div class={styles.empty}>No distribution telemetry yet.</div>
    {/if}
</div>
