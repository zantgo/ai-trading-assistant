<script lang="ts">
    import type { PipelineReliabilityMetrics } from '../types';
    import styles from './DataQualityPanel.module.css';

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

    function coverageClass(pct: number): string {
        if (pct >= 99) return styles.coverageExcellent;
        if (pct >= 95) return styles.coverageGood;
        if (pct >= 80) return styles.coverageModerate;
        return styles.coveragePoor;
    }

    function gapClass(count: number): string {
        if (count === 0) return styles.metricNormal;
        if (count <= 5) return styles.metricWarn;
        return styles.metricBad;
    }

    function outlierClass(count: number): string {
        if (count === 0) return styles.metricNormal;
        if (count <= 10) return styles.metricWarn;
        return styles.metricBad;
    }

    function oooClass(count: number): string {
        if (count === 0) return styles.metricNormal;
        return styles.metricBad;
    }
</script>

<div class={styles.container}>
    <div class={styles.header}>
        <h2 class={styles.title}>Data Quality</h2>
    </div>

    {#if loading}
        <div class={styles.placeholder}>Loading...</div>
    {:else if error}
        <div class={styles.error}>Error: {error}</div>
    {:else if report}
          <div class={styles.metrics}>
              <div class={styles.metric}>
                  <div class={styles.metricLabel}>Coverage</div>
                  {#if report.total_candles_processed > 0}
                      <div class="{styles.metricValue} {coverageClass(report.coverage * 100)}">
                          {(report.coverage * 100).toFixed(2)}%
                      </div>
                  {:else}
                      <div class={styles.metricValue} style="color: #999">No data yet</div>
                  {/if}
              </div>
            <div class={styles.metric}>
                <div class={styles.metricLabel}>Total Candles</div>
                <div class={styles.metricValue}>{report.total_candles_processed.toLocaleString('en-US')}</div>
            </div>
            <div class={styles.metric}>
                <div class={styles.metricLabel}>Gaps Detected</div>
                <div class="{styles.metricValue} {gapClass(report.gap_count)}">
                    {report.gap_count}
                </div>
            </div>
            <div class={styles.metric}>
                <div class={styles.metricLabel}>Outliers Rejected</div>
                <div class="{styles.metricValue} {outlierClass(report.outliers_rejected)}">
                    {report.outliers_rejected}
                </div>
            </div>
            <div class={styles.metric}>
                <div class={styles.metricLabel}>Out-of-Order Dropped</div>
                <div class="{styles.metricValue} {oooClass(report.out_of_order_dropped)}">
                    {report.out_of_order_dropped}
                </div>
            </div>
            <div class={styles.metric}>
                <div class={styles.metricLabel}>Reconstructed Candles</div>
                <div class={styles.metricValue}>{report.reconstructed_candles}</div>
            </div>
        </div>
    {:else}
        <div class={styles.placeholder}>No data</div>
    {/if}
</div>
