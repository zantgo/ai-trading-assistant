<script lang="ts">
    // TradeOpportunitiesCard — compact "what can I trade" summary.
    // Surfaces the actionable count, best pair, direction, R:R and
    // highest confidence. Inverse of the Header KPI strip — this is
    // the operator's "show me the plays" panel.
    import { useAppStore } from '../../state.svelte';
    import { collectActiveSetups, pickBestOpportunity } from '../../lib/tradeAggregates';
    import { directionColor, formatRR, scoreColor } from '../../lib/dashboardColors';
    import styles from './TradeOpportunitiesCard.module.css';

    const app = useAppStore();

    const summary = $derived.by(() => {
        const instances = Object.values(app.instancesMap);
        const setups = collectActiveSetups(instances);
        const actionable = setups.filter(
            (s) => s.viability === 'Actionable' && s.readiness === 'READY',
        );
        const all = setups;
        const best = pickBestOpportunity(instances);
        const total = instances.length;
        const valid = Math.min(actionable.length, total);
        const highestConfidence = setups.length > 0
            ? Math.max(...setups.map((s) => s.confidence))
            : 0;
        return {
            validSetups: valid,
            total,
            best: best,
            actionableCount: actionable.length,
            totalCandidates: all.length,
            highestConfidence: Number.isFinite(highestConfidence) ? highestConfidence : 0,
        };
    });
</script>

<div class={styles.card}>
    <div class={styles.header}>
        <span class={styles.title}>TRADE OPPORTUNITIES</span>
        <span class={styles.count}>
            {summary.validSetups}/{summary.total}
        </span>
    </div>

    {#if summary.best}
        <div class={styles.grid}>
            <div class={styles.row}>
                <span class={styles.label}>Best Pair</span>
                <span class={styles.value}>{summary.best.symbol}</span>
            </div>
            <div class={styles.row}>
                <span class={styles.label}>Direction</span>
                <span class={styles.value} style="color: {directionColor(summary.best.direction)}">
                    {summary.best.direction}
                </span>
            </div>
            <div class={styles.row}>
                <span class={styles.label}>Best R:R</span>
                <span class={styles.value}>{formatRR(summary.best.rr)}</span>
            </div>
            <div class={styles.row}>
                <span class={styles.label}>Confidence</span>
                <span class={styles.value} style="color: {scoreColor(summary.best.confidence)}">
                    {summary.best.confidence.toFixed(0)}%
                </span>
            </div>
        </div>
        <div class={styles.scoreBar}>
            <div class={styles.scoreBarLabel}>
                <span>Score</span>
                <span class={styles.scoreBarVal} style="color: {scoreColor(summary.best.opportunityScore)}">
                    {summary.best.opportunityScore.toFixed(0)}
                </span>
            </div>
            <div class={styles.scoreBarTrack}>
                <div
                    class={styles.scoreBarFill}
                    style="width: {Math.min(summary.best.opportunityScore, 100)}%; background: {scoreColor(summary.best.opportunityScore)}"
                ></div>
            </div>
        </div>
    {:else}
        <div class={styles.empty}>No qualifying opportunity yet.</div>
    {/if}
</div>
