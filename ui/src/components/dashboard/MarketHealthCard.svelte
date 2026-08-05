<script lang="ts">
    // MarketHealthCard — top-level HealthLevel + SyncLevel chip from
    // the L7 OverviewMatrix, plus the four sub-dimension quality bars
    // derived from each instance's L5 RiskMatrix (trend strength,
    // liquidity, volatility, signal stability — all on the wire).
    import { useAppStore } from '../../state.svelte';
    import { computeMarketHealth } from '../../lib/marketHealth';
    import { qualityColor } from '../../lib/dashboardColors';
    import styles from './MarketHealthCard.module.css';

    const app = useAppStore();

    const health = $derived.by(() => {
        const instances = Object.values(app.instancesMap);
        return computeMarketHealth(instances, app.overviewMatrix);
    });

    function healthColor(level: string | null): string {
        if (!level) return 'rgba(255,255,255,0.4)';
        const l = level.toUpperCase();
        if (l === 'STRONG' || l === 'HEALTHY') return '#22c55e';
        if (l === 'NEUTRAL') return '#f59e0b';
        if (l === 'WEAK' || l === 'POOR') return '#ef4444';
        return 'rgba(255,255,255,0.5)';
    }

    function syncLabel(level: string | null): string {
        if (!level) return '—';
        return level.toUpperCase().replace(/_/g, ' ');
    }
</script>

<div class={styles.card}>
    <div class={styles.header}>
        <span class={styles.title}>MARKET HEALTH</span>
        <div class={styles.chips}>
            <span class={styles.healthChip} style="color: {healthColor(health.overall)}">
                {health.overall ?? 'NO DATA'}
            </span>
            <span class={styles.syncChip} title="Cross-asset synchronization">
                <span class={styles.syncLabel}>SYNC</span>
                <span class={styles.syncVal}>{syncLabel(health.sync)}</span>
            </span>
        </div>
    </div>

    <div class={styles.bars}>
        {#each health.bars as bar (bar.label)}
            <div class={styles.barRow}>
                <span class={styles.barLabel}>{bar.label}</span>
                <div class={styles.barTrack}>
                    <div
                        class={styles.barFill}
                        style="width: {Math.max(0, Math.min(100, bar.value))}%; background: {qualityColor(bar.value)}"
                    ></div>
                </div>
                <span class={styles.barVal} style="color: {qualityColor(bar.value)}">
                    {bar.available ? bar.value.toFixed(0) : '—'}
                </span>
            </div>
        {/each}
    </div>

    <div class={styles.footer}>
        <span class={styles.footNote}>
            {health.activeInstanceCount} active instance{health.activeInstanceCount === 1 ? '' : 's'} contributing
        </span>
    </div>
</div>
