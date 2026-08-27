<script lang="ts">
    // DirectionDistributionCard — Long / Short / Neutral counts derived
    // from each instance's L6 `directional_guidance`. Symmetric three-
    // column layout with per-bucket color.
    import { useAppStore } from '../../state.svelte';
    import { aggregateDirections } from '../../lib/tradeAggregates';
    import styles from './DirectionDistributionCard.module.css';

    const app = useAppStore();

    // v7.2 parity: server-computed counts (single source, also rendered
    // by the CLI monitor); local derivation is the warmup fallback.
    const counts = $derived.by(() => {
        const server = app.overviewMatrix?.direction_distribution;
        if (server) {
            const total = server.long + server.short + server.neutral;
            return {
                long: server.long,
                short: server.short,
                neutral: server.neutral,
                total,
                longPct: total > 0 ? (server.long / total) * 100 : 0,
                shortPct: total > 0 ? (server.short / total) * 100 : 0,
                neutralPct: total > 0 ? (server.neutral / total) * 100 : 0,
            };
        }
        const instances = Object.values(app.instancesMap);
        const d = aggregateDirections(instances);
        const total = d.long + d.short + d.neutral;
        return {
            long: d.long,
            short: d.short,
            neutral: d.neutral,
            total,
            longPct: total > 0 ? (d.long / total) * 100 : 0,
            shortPct: total > 0 ? (d.short / total) * 100 : 0,
            neutralPct: total > 0 ? (d.neutral / total) * 100 : 0,
        };
    });
</script>

<div class={styles.card}>
    <div class={styles.header}>
        <span class={styles.title}>DIRECTION</span>
        <span class={styles.count}>{counts.total} pairs</span>
    </div>

    <div class={styles.buckets}>
        <div class={styles.bucket}>
            <div class={styles.count} style="color: #22c55e">{counts.long}</div>
            <div class={styles.bucketLabel}>LONG</div>
        </div>
        <div class={styles.bucket}>
            <div class={styles.count} style="color: #ef4444">{counts.short}</div>
            <div class={styles.bucketLabel}>SHORT</div>
        </div>
        <div class={styles.bucket}>
            <div class={styles.count} style="color: #f59e0b">{counts.neutral}</div>
            <div class={styles.bucketLabel}>NEUTRAL</div>
        </div>
    </div>

    <div class={styles.bar}>
        <div class={styles.barLong} style="width: {counts.longPct}%"></div>
        <div class={styles.barNeutral} style="width: {counts.neutralPct}%"></div>
        <div class={styles.barShort} style="width: {counts.shortPct}%"></div>
    </div>

    <div class={styles.legend}>
        <span class={styles.legendItem}>
            <span class={styles.swatch} style="background: #22c55e"></span>
            Bullish setups: {counts.long}
        </span>
        <span class={styles.legendItem}>
            <span class={styles.swatch} style="background: #ef4444"></span>
            Bearish setups: {counts.short}
        </span>
    </div>
</div>
