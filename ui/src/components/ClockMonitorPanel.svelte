<script lang="ts">
    import type { ClockStatusResponse } from '../types';
    import styles from './ClockMonitorPanel.module.css';

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

    function driftClass(driftUs: number | null | undefined, threshold: number): string {
        if (driftUs == null) return styles.driftUnknown;
        const absDrift = Math.abs(driftUs);
        if (absDrift <= threshold * 0.5) return styles.driftGood;
        if (absDrift <= threshold) return styles.driftWarn;
        return styles.driftBad;
    }

    function thresholdStatus(withinThreshold: boolean | null | undefined, breachAction: string): string {
        if (withinThreshold == null) return styles.statusUnknown;
        if (withinThreshold) return styles.statusGood;
        return breachAction === 'Panic' ? styles.statusCritical : styles.statusBad;
    }
</script>

<div class={styles.container}>
    <div class={styles.header}>
        <h2 class={styles.title}>NTP Clock Monitor</h2>
    </div>

    {#if loading}
        <div class={styles.placeholder}>Loading...</div>
    {:else if error}
        <div class={styles.error}>Error: {error}</div>
    {:else if report}
        <div class={styles.metrics}>
            <div class={styles.metric}>
                <div class={styles.metricLabel}>Status</div>
                <div class="{styles.metricValue} {thresholdStatus(report.within_threshold, report.breach_action)}">
                    {report.within_threshold ? 'Within Threshold' : report.breach_action === 'Panic' ? 'BREACH (PANIC)' : 'BREACH'}
                </div>
            </div>
            <div class={styles.metric}>
                  <div class={styles.metricLabel}>Drift</div>
                  <div class="{styles.metricValue} {driftClass(report.drift_us, report.threshold_micros)}">
                      {report.drift_us != null ? `${report.drift_us}µs` : 'No samples yet'}
                  </div>
            </div>
            <div class={styles.metric}>
                <div class={styles.metricLabel}>Threshold</div>
                <div class={styles.metricValue}>{report.threshold_micros}µs</div>
            </div>
            <div class={styles.metric}>
                <div class={styles.metricLabel}>RMS Jitter</div>
                <div class={styles.metricValue}>{report.jitter_rms_us != null ? `${report.jitter_rms_us.toFixed(2)}µs` : '—'}</div>
            </div>
            <div class={styles.metric}>
                <div class={styles.metricLabel}>Breach Action</div>
                <div class={styles.metricValue}>{report.breach_action}</div>
            </div>
            <div class={styles.metric}>
                <div class={styles.metricLabel}>Samples</div>
                <div class={styles.metricValue}>{report.sample_count}</div>
            </div>
            <div class={styles.metric}>
                <div class={styles.metricLabel}>NTP Servers</div>
                <div class={styles.metricList}>
                    {#each report.ntp_servers as server}
                        <div class={styles.serverItem}>{server}</div>
                    {/each}
                </div>
            </div>
        </div>
    {:else}
        <div class={styles.placeholder}>Clock monitor not active (disabled in config)</div>
    {/if}
</div>
