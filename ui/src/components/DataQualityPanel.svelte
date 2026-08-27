<script lang="ts">
    import type { PipelineReliabilityMetrics } from '../types';
    import KpiStrip from './KpiStrip.svelte';
    import ExportDataButton from './ExportDataButton.svelte';
    import { buildEngineExport } from '../lib/engineExport';
    import styles from '../styles/engine-dashboard.module.css';

    let report: PipelineReliabilityMetrics | null = $state(null);
    let loading = $state(true);
    let error: string | null = $state(null);

    let pollInterval: ReturnType<typeof setInterval> | null = null;

    async function fetchQuality() {
        try {
            const res = await fetch('/api/data-quality');
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

    function coverageColor(pct: number): string {
        if (pct >= 99) return '#22c55e';
        if (pct >= 95) return '#84cc16';
        if (pct >= 80) return '#f59e0b';
        return '#ef4444';
    }

    function countColor(count: number, warnAt: number, badAt: number): string {
        if (count === 0) return '#22c55e';
        if (count <= warnAt) return '#f59e0b';
        return '#ef4444';
    }

    function buildExport(): string {
        return buildEngineExport('data_infra', 'data_quality', null, {
            loading,
            error,
            report: report ? {
                coverage: report.coverage,
                total_candles_processed: report.total_candles_processed,
                gap_count: report.gap_count,
                outliers_rejected: report.outliers_rejected,
                outliers_bypassed: report.outliers_bypassed,
                out_of_order_dropped: report.out_of_order_dropped,
                reconstructed_candles: report.reconstructed_candles,
            } : null,
        });
    }
</script>

<div style="display:flex; flex-direction:column; gap:16px">
    <div style="display:flex; align-items:center; justify-content:flex-end">
        <ExportDataButton onExport={buildExport} title="Copy all Data Quality data as JSON" />
    </div>
    {#if loading}
        <div class={styles.empty}>Loading…</div>
    {:else if error}
        <div class="{styles.alertBanner} {styles.alertError}">Error: {error}</div>
    {:else if report}
        <KpiStrip items={[
            {
                label: 'Coverage',
                value: report.total_candles_processed > 0 ? `${(report.coverage * 100).toFixed(2)}%` : 'No data yet',
                sub: 'expected candles delivered',
                color: report.total_candles_processed > 0 ? coverageColor(report.coverage * 100) : 'rgba(255,255,255,0.4)',
            },
            { label: 'Total Candles', value: report.total_candles_processed.toLocaleString('en-US'), sub: 'processed this session' },
            { label: 'Gaps Detected', value: String(report.gap_count), sub: 'REST gap-fill events', color: countColor(report.gap_count, 5, 6) },
            { label: 'Outliers Rejected', value: String(report.outliers_rejected), sub: 'median-filter drops', color: countColor(report.outliers_rejected, 10, 11) },
            { label: 'Out-of-Order Dropped', value: String(report.out_of_order_dropped), sub: 'late ticks', color: countColor(report.out_of_order_dropped, 0, 1) },
            { label: 'Reconstructed', value: String(report.reconstructed_candles), sub: 'synthesized candles' },
        ]} />
        {#if report.gap_count > 0}
            <div class="{styles.alertBanner} {styles.alertWarn}">GAPS: {report.gap_count} gap-fill event{report.gap_count === 1 ? '' : 's'} — historical REST fetch was used.</div>
        {/if}
        {#if report.outliers_rejected > 0}
            <div class="{styles.alertBanner} {styles.alertWarn}">OUTLIERS: {report.outliers_rejected} tick{report.outliers_rejected === 1 ? '' : 's'} rejected by the median filter.</div>
        {/if}
        {#if report.out_of_order_dropped > 0}
            <div class="{styles.alertBanner} {styles.alertError}">OUT-OF-ORDER: {report.out_of_order_dropped} late tick{report.out_of_order_dropped === 1 ? '' : 's'} dropped at L3.</div>
        {/if}
    {:else}
        <div class={styles.empty}>No data</div>
    {/if}
</div>
