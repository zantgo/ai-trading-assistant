<script lang="ts">
    // SignalQualityCard — buckets each instance's L6
    // `confidence_assessment` into Strong / Moderate / Weak bands and
    // shows the count. Replaces the single "average confidence" view
    // with a distribution that is more operationally useful (an
    // average masks "all pairs are weak" vs "all pairs are strong").
    import { useAppStore } from '../../state.svelte';
    import { aggregateSignalQuality } from '../../lib/tradeAggregates';
    import styles from './SignalQualityCard.module.css';

    const app = useAppStore();

    const buckets = $derived.by(() => {
        const instances = Object.values(app.instancesMap);
        const q = aggregateSignalQuality(instances);
        const total = q.strong + q.moderate + q.weak;
        return {
            strong: q.strong,
            moderate: q.moderate,
            weak: q.weak,
            total,
            strongPct: total > 0 ? (q.strong / total) * 100 : 0,
            moderatePct: total > 0 ? (q.moderate / total) * 100 : 0,
            weakPct: total > 0 ? (q.weak / total) * 100 : 0,
        };
    });
</script>

<div class={styles.card}>
    <div class={styles.header}>
        <span class={styles.title}>SIGNAL QUALITY</span>
        <span class={styles.total}>{buckets.total} pairs</span>
    </div>

    <div class={styles.buckets}>
        <div class="{styles.bucket} {styles.strong}">
            <div class={styles.count}>{buckets.strong}</div>
            <div class={styles.bucketLabel}>STRONG</div>
            <div class={styles.bucketSub}>≥ 70 conf</div>
        </div>
        <div class="{styles.bucket} {styles.moderate}">
            <div class={styles.count}>{buckets.moderate}</div>
            <div class={styles.bucketLabel}>MODERATE</div>
            <div class={styles.bucketSub}>40–69 conf</div>
        </div>
        <div class="{styles.bucket} {styles.weak}">
            <div class={styles.count}>{buckets.weak}</div>
            <div class={styles.bucketLabel}>WEAK</div>
            <div class={styles.bucketSub}>&lt; 40 conf</div>
        </div>
    </div>

    <div class={styles.bar}>
        <div class={styles.barStrong} style="width: {buckets.strongPct}%"></div>
        <div class={styles.barModerate} style="width: {buckets.moderatePct}%"></div>
        <div class={styles.barWeak} style="width: {buckets.weakPct}%"></div>
    </div>
</div>
