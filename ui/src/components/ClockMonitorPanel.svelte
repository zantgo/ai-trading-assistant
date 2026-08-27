<script lang="ts">
    import type { ClockStatusResponse } from '../types';
    import KpiStrip from './KpiStrip.svelte';
    import ExportDataButton from './ExportDataButton.svelte';
    import { buildEngineExport } from '../lib/engineExport';
    import styles from '../styles/engine-dashboard.module.css';

    let report: ClockStatusResponse | null = $state(null);
    let loading = $state(true);
    let error: string | null = $state(null);

    let pollInterval: ReturnType<typeof setInterval> | null = null;

    async function fetchClock() {
        try {
            const res = await fetch('/api/system/clock');
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
        fetchClock();
        if (pollInterval) clearInterval(pollInterval);
        pollInterval = setInterval(fetchClock, 30_000);
        return () => {
            if (pollInterval) clearInterval(pollInterval);
        };
    });

    function driftColor(driftUs: number | null | undefined, threshold: number): string {
        if (driftUs == null) return 'rgba(255,255,255,0.4)';
        const absDrift = Math.abs(driftUs);
        if (absDrift <= threshold * 0.5) return '#22c55e';
        if (absDrift <= threshold) return '#f59e0b';
        return '#ef4444';
    }

    function statusBadge(withinThreshold: boolean | null | undefined, breachAction: string): string {
        if (withinThreshold == null) return styles.badgeEmpty;
        if (withinThreshold) return styles.badgeLong;
        return breachAction === 'Panic' ? styles.badgeError : styles.badgeNeutral;
    }

    function statusLabel(withinThreshold: boolean | null | undefined, breachAction: string): string {
        if (withinThreshold == null) return 'NO SAMPLES';
        if (withinThreshold) return 'WITHIN THRESHOLD';
        return breachAction === 'Panic' ? 'BREACH (PANIC)' : 'BREACH';
    }

    function buildExport(): string {
        return buildEngineExport('data_infra', 'clock_monitor', null, {
            loading,
            error,
            report: report ? {
                within_threshold: report.within_threshold,
                drift_us: report.drift_us,
                jitter_rms_us: report.jitter_rms_us,
                last_poll_ms: report.last_poll_ms,
                breach_count: report.breach_count,
                breach_action: report.breach_action,
                ntp_servers: report.ntp_servers,
                sample_count: report.sample_count,
                threshold_micros: report.threshold_micros,
            } : null,
        });
    }
</script>

<div style="display:flex; flex-direction:column; gap:16px">
    <div style="display:flex; align-items:center; justify-content:space-between; flex-wrap:wrap; gap:8px">
        <div style="display:flex; align-items:center; gap:8px">
            {#if report}
                <span class="{styles.badge} {statusBadge(report.within_threshold, report.breach_action)}">
                    {statusLabel(report.within_threshold, report.breach_action)}
                </span>
                <span class={styles.metaChip}><span class={styles.metaChipLabel}>action</span><span class={styles.metaChipValue}>{report.breach_action}</span></span>
            {/if}
        </div>
        <ExportDataButton onExport={buildExport} title="Copy all NTP Clock Monitor data as JSON" />
    </div>

    {#if loading}
        <div class={styles.empty}>Loading…</div>
    {:else if error}
        <div class="{styles.alertBanner} {styles.alertError}">Error: {error}</div>
    {:else if report}
        <KpiStrip items={[
            { label: 'Drift', value: report.drift_us != null ? `${report.drift_us}µs` : 'No samples yet', sub: 'vs UTC', color: driftColor(report.drift_us, report.threshold_micros) },
            { label: 'Threshold', value: `${report.threshold_micros}µs`, sub: 'drift budget' },
            { label: 'RMS Jitter', value: report.jitter_rms_us != null ? `${report.jitter_rms_us.toFixed(2)}µs` : '—', sub: 'across samples' },
            { label: 'Breaches', value: String(report.breach_count), sub: 'total', color: report.breach_count > 0 ? '#ef4444' : '#22c55e' },
            { label: 'Samples', value: String(report.sample_count), sub: 'collected' },
        ]} />
        <div class={styles.card}>
            <h3 class={styles.cardTitle}>NTP Servers</h3>
            <div class={styles.monoList}>
                {#each report.ntp_servers as server (server)}
                    <span>{server}</span>
                {/each}
            </div>
        </div>
    {:else}
        <div class={styles.empty}>Clock monitor not active (disabled in config)</div>
    {/if}
</div>
