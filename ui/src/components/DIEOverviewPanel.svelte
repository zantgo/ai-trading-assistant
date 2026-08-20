<script lang="ts">
    // DIEOverviewPanel — the Data Infrastructure landing tab: one composite
    // view of the four layers. Pure frontend aggregation of the existing
    // DIE endpoints (no backend).
    import { onMount } from 'svelte';
    import KpiStrip from './KpiStrip.svelte';
    import ExportDataButton from './ExportDataButton.svelte';
    import { buildEngineExport } from '../lib/engineExport';
    import styles from '../styles/engine-dashboard.module.css';
    import { useAppStore } from '../state.svelte';

    const app = useAppStore();

    let quality: { score?: number; uptime_pct?: number; disconnect_count?: number } | null = $state(null);
    let exchanges: { name: string; state: string; active_pairs: number; last_heartbeat_ms: number }[] = $state([]);
    let clock: { within_threshold?: boolean; drift_us?: number | null; breach_count?: number } | null = $state(null);
    let dataQuality: { coverage?: number; gap_count?: number; total_candles_processed?: number; reconstructed_candles?: number } | null = $state(null);
    let loading = $state(true);
    let error: string | null = $state(null);

    let pollInterval: ReturnType<typeof setInterval> | null = null;

    async function fetchAll() {
        try {
            const [qRes, eRes, cRes, dRes] = await Promise.all([
                fetch('/api/connection-quality'),
                fetch('/api/exchange-status'),
                fetch('/api/system/clock'),
                fetch('/api/data-quality'),
            ]);
            if (qRes.ok) quality = (await qRes.json()) as typeof quality;
            if (eRes.ok) {
                const rep = await eRes.json();
                exchanges = rep?.exchanges ?? [];
            }
            if (cRes.ok) clock = (await cRes.json()) as typeof clock;
            if (dRes.ok) dataQuality = (await dRes.json()) as typeof dataQuality;
            error = null;
        } catch (e) {
            error = e instanceof Error ? e.message : String(e);
        } finally {
            loading = false;
        }
    }

    onMount(() => {
        fetchAll();
        pollInterval = setInterval(fetchAll, 30_000);
        return () => {
            if (pollInterval) clearInterval(pollInterval);
        };
    });

    function scoreColor(score: number | undefined): string | undefined {
        if (score == null) return undefined;
        if (score >= 90) return '#22c55e';
        if (score >= 75) return '#84cc16';
        if (score >= 50) return '#f59e0b';
        return '#ef4444';
    }

    function buildExport(): string {
        return buildEngineExport('data_infra', 'overview', null, {
            loading,
            error,
            connection_quality: quality,
            exchanges: exchanges,
            clock: clock,
            data_quality: dataQuality,
            platform: `${app.session?.sessionExchange ?? '—'} · ${app.session?.sessionCurrency ?? '—'}`,
        });
    }
</script>

<div style="display:flex; flex-direction:column; gap:16px">
    <div style="display:flex; align-items:center; justify-content:flex-end">
        <ExportDataButton onExport={buildExport} title="Copy all Data Infrastructure Overview data as JSON" />
    </div>
    {#if loading}
        <div class={styles.empty}>Loading…</div>
    {:else if error}
        <div class="{styles.alertBanner} {styles.alertError}">Error: {error}</div>
    {:else}
        <KpiStrip items={[
            { label: 'Connection Score', value: quality?.score?.toFixed(1) ?? '—', sub: 'composite, 24h', color: scoreColor(quality?.score) },
            { label: 'Exchanges Connected', value: String(exchanges.filter((e) => e.state === 'Connected').length), sub: `${exchanges.length} configured`, color: exchanges.some((e) => e.state !== 'Connected') ? '#f59e0b' : '#22c55e' },
            { label: 'NTP Drift', value: clock?.drift_us != null ? `${clock.drift_us}µs` : '—', sub: clock?.within_threshold ? 'within threshold' : 'BREACH', color: clock?.within_threshold ? '#22c55e' : '#ef4444' },
            { label: 'Data Coverage', value: dataQuality?.coverage != null ? `${(dataQuality.coverage * 100).toFixed(2)}%` : '—', sub: 'expected candles', color: dataQuality?.coverage != null && dataQuality.coverage < 0.95 ? '#f59e0b' : '#22c55e' },
            { label: 'Reconstructed', value: String(dataQuality?.reconstructed_candles ?? 0), sub: 'gap-filled candles' },
            { label: 'Active Pipelines', value: String(exchanges.reduce((acc, e) => acc + e.active_pairs, 0)), sub: 'pairs across exchanges' },
        ]} />

        <div class={styles.card}>
            <h3 class={styles.cardTitle}>Exchange Health</h3>
            {#if exchanges.length === 0}
                <div class={styles.empty}>No exchange status available.</div>
            {:else}
                <div style="display:flex; flex-direction:column; gap:6px">
                    {#each exchanges as e (e.name)}
                        <div class={styles.inlineGroup}>
                            <span class={styles.metaChip}><span class={styles.metaChipValue}>{e.name}</span></span>
                            <span class="{styles.badge} {e.state === 'Connected' ? styles.badgeLong : e.state === 'Reconnecting' ? styles.badgeNeutral : e.state === 'Disabled' ? styles.badgeEmpty : styles.badgeError}">{e.state.toUpperCase()}</span>
                            <span class={styles.metaChip}><span class={styles.metaChipLabel}>pairs</span><span class={styles.metaChipValue}>{e.active_pairs}</span></span>
                        </div>
                    {/each}
                </div>
            {/if}
        </div>

        <div class={styles.card}>
            <h3 class={styles.cardTitle}>Quality Windows</h3>
            <div style="display:grid; grid-template-columns:repeat(auto-fit, minmax(140px, 1fr)); gap:10px">
                <div class={styles.kpi}><div class={styles.kpiLabel}>1 Hour</div><div class={styles.kpiValue} style="color:{scoreColor(quality?.score) ?? ''}">{quality?.score?.toFixed(1) ?? '—'}</div></div>
                <div class={styles.kpi}><div class={styles.kpiLabel}>Uptime</div><div class={styles.kpiValue}>{quality?.uptime_pct != null ? `${quality.uptime_pct.toFixed(2)}%` : '—'}</div></div>
                <div class={styles.kpi}><div class={styles.kpiLabel}>Disconnects</div><div class={styles.kpiValue}>{quality?.disconnect_count ?? '—'}</div></div>
            </div>
        </div>

        {#if (dataQuality?.gap_count ?? 0) > 0}
            <div class="{styles.alertBanner} {styles.alertWarn}">GAPS: {dataQuality?.gap_count} gap-fill event{(dataQuality?.gap_count ?? 0) === 1 ? '' : 's'} — historical REST fetch was used.</div>
        {/if}
        {#if clock && !clock.within_threshold}
            <div class="{styles.alertBanner} {styles.alertError}">CLOCK BREACH: {clock.breach_count} breach{clock.breach_count === 1 ? '' : 'es'} — UTC drift exceeded the budget.</div>
        {/if}
        <p class={styles.infoLine} style="color:rgba(255,255,255,0.35)">Platform identity: {app.session?.sessionExchange ?? '—'} · {app.session?.sessionCurrency ?? '—'}</p>
    {/if}
</div>
