<script lang="ts">
    // RiskDistributionCard — reads directly from the L7
    // `OverviewMatrix.risk_distribution` (low/moderate/high
    // percentages + environment label). Falls back to local aggregation
    // when the L7 matrix is not yet populated.
    import { useAppStore } from '../../state.svelte';
    import { aggregateRisk } from '../../lib/tradeAggregates';
    import { riskDangerColor } from '../../lib/dashboardColors';
    import styles from './RiskDistributionCard.module.css';

    const app = useAppStore();

    const data = $derived.by(() => {
        const overview = app.overviewMatrix;
        const rd = overview?.risk_distribution;
        const instances = Object.values(app.instancesMap);
        if (rd && (rd.low_pct + rd.moderate_pct + rd.high_pct) > 0) {
            return {
                low: rd.low_pct,
                moderate: rd.moderate_pct,
                high: rd.high_pct,
                environment: rd.risk_environment ?? 'NO_DATA',
                source: 'L7',
            };
        }
        const local = aggregateRisk(instances);
        if (local.count > 0) {
            // Bucket by overall_risk.score band.
            const low = instances.filter((i) => (i.risk?.overall_risk?.score ?? 50) <= 30).length;
            const high = instances.filter((i) => (i.risk?.overall_risk?.score ?? 50) >= 70).length;
            const moderate = instances.length - low - high;
            return {
                low: (low / instances.length) * 100,
                moderate: (moderate / instances.length) * 100,
                high: (high / instances.length) * 100,
                environment: 'LOCAL',
                source: 'L5',
            };
        }
        return {
            low: 0,
            moderate: 0,
            high: 0,
            environment: 'NO_DATA',
            source: null,
        };
    });

    function envColor(env: string): string {
        if (env === 'LOW_RISK') return '#22c55e';
        if (env === 'MODERATE') return '#f59e0b';
        if (env === 'HIGH_RISK') return '#ef4444';
        return 'rgba(255,255,255,0.4)';
    }
</script>

<div class={styles.card}>
    <div class={styles.header}>
        <span class={styles.title}>RISK DISTRIBUTION</span>
        <span class={styles.env} style="color: {envColor(data.environment)}">
            {data.environment.replace('_', ' ')}
        </span>
    </div>

    <div class={styles.bars}>
        <div class={styles.row}>
            <span class={styles.label}>Low</span>
            <div class={styles.track}>
                <div class={styles.fill} style="width: {data.low}%; background: #22c55e"></div>
            </div>
            <span class={styles.val}>{data.low.toFixed(0)}%</span>
        </div>
        <div class={styles.row}>
            <span class={styles.label}>Moderate</span>
            <div class={styles.track}>
                <div class={styles.fill} style="width: {data.moderate}%; background: #f59e0b"></div>
            </div>
            <span class={styles.val}>{data.moderate.toFixed(0)}%</span>
        </div>
        <div class={styles.row}>
            <span class={styles.label}>High</span>
            <div class={styles.track}>
                <div class={styles.fill} style="width: {data.high}%; background: #ef4444"></div>
            </div>
            <span class={styles.val}>{data.high.toFixed(0)}%</span>
        </div>
    </div>

    <div class={styles.footer}>
        <span class={styles.footLabel}>Source</span>
        <span class={styles.footVal}>{data.source ?? 'no data'}</span>
    </div>
</div>
