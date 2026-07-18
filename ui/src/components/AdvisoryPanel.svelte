<script lang="ts">
    import type { AdvisoryMatrix } from '../types';
    import { useAppStore } from '../state.svelte';
    import styles from './AdvisoryPanel.module.css';

    const app = useAppStore();
    let { pairKey } = $props<{ pairKey: string }>();

    const instance = $derived(app.instancesMap[pairKey]);
    const advisory = $derived<AdvisoryMatrix | null>(instance?.advisory ?? null);

    const snapshot = $derived(instance?.microTerm.latestSnapshot as any);
    const decisionCtx = $derived(snapshot?.decision_context ?? null);

    function recClass(d: string): string {
        if (d.includes('Long')) return styles.recLong;
        if (d.includes('Short')) return styles.recShort;
        if (d.includes('Avoid')) return styles.recAvoid;
        return styles.recNeutral;
    }
    function stanceClass(m: string): string {
        switch (m) {
            case 'Aggressive': return styles.stanceAggressive;
            case 'Constructive': return styles.stanceConstructive;
            case 'Neutral': return styles.stanceNeutral;
            case 'Cautious': return styles.stanceCautious;
            default: return styles.stanceAvoid;
        }
    }
    function readinessClass(r: string): string {
        switch (r) {
            case 'READY': return styles.ready;
            case 'FORMING': return styles.forming;
            case 'WATCH': return styles.watch;
            default: return styles.aside;
        }
    }
    function fillColor(v: number, t: 'rr' | 'danger' | 'conf'): string {
        if (t === 'rr') return v >= 2 ? styles.green : v >= 1 ? styles.amber : styles.red;
        if (t === 'danger') return v >= 70 ? styles.red : v >= 40 ? styles.amber : styles.green;
        return v >= 60 ? styles.green : v >= 30 ? styles.amber : styles.red;
    }

    const rrDisplay = $derived(decisionCtx?.expected_reward_risk_ratio ?? 0);
    const dangerDisplay = $derived(decisionCtx?.entry_danger ?? 50);
    const readinessDisplay = $derived(decisionCtx?.trade_readiness ?? 'STAND_ASIDE');
    const confidenceDisplay = $derived(advisory?.confidence_assessment ?? 0);
</script>

<div class={styles.panel}>
    {#if !advisory}
        <div class={styles.placeholder}>Awaiting decision guidance data...</div>
    {:else}
        <h2 class={styles.title}>Decision Guidance</h2>

        <div class={styles.section}>
            <div class={styles.recRow}>
                <span class="{styles.recLabel} {recClass(advisory.directional_guidance)}">
                    {advisory.directional_guidance}
                </span>
                <span class="{styles.stanceBadge} {stanceClass(advisory.market_stance)}">
                    {advisory.market_stance}
                </span>
                {#if decisionCtx}
                    <span class="{styles.readinessBadge} {readinessClass(readinessDisplay)}">
                        {readinessDisplay}
                    </span>
                {/if}
            </div>
        </div>

        <div class={styles.section}>
            <div class={styles.sectionTitle}>Key Metrics</div>
            <div class={styles.metricBar}>
                <span class={styles.metricLabel}>R:R Ratio</span>
                <div class={styles.metricBarBg}>
                    <div class="{styles.metricFill} {fillColor(rrDisplay, 'rr')}"
                         style="width: {Math.min(rrDisplay / 3 * 100, 100).toFixed(1)}%"></div>
                </div>
                <span class={styles.metricVal}>{rrDisplay.toFixed(2)}</span>
            </div>
            <div class={styles.metricBar}>
                <span class={styles.metricLabel}>Entry Danger</span>
                <div class={styles.metricBarBg}>
                    <div class="{styles.metricFill} {fillColor(dangerDisplay, 'danger')}"
                         style="width: {dangerDisplay.toFixed(1)}%"></div>
                </div>
                <span class={styles.metricVal}>{dangerDisplay.toFixed(0)}</span>
            </div>
            <div class={styles.metricBar}>
                <span class={styles.metricLabel}>Confidence</span>
                <div class={styles.metricBarBg}>
                    <div class="{styles.metricFill} {fillColor(confidenceDisplay, 'conf')}"
                         style="width: {confidenceDisplay.toFixed(1)}%"></div>
                </div>
                <span class={styles.metricVal}>{confidenceDisplay.toFixed(0)}%</span>
            </div>
        </div>

        <div class={styles.section}>
            <div class={styles.sectionTitle}>Strategy</div>
            <div class={styles.grid2}>
                <div class={styles.card}>
                    <span class={styles.cardLabel}>Environment</span>
                    <span class={styles.cardValue}>{advisory.strategy_environment}</span>
                </div>
                <div class={styles.card}>
                    <span class={styles.cardLabel}>Opportunity</span>
                    <span class={styles.cardValue}>{advisory.opportunity_classification}</span>
                </div>
                <div class={styles.card}>
                    <span class={styles.cardLabel}>Entry</span>
                    <span class={styles.cardValue}>{advisory.entry_guidance}</span>
                </div>
                <div class={styles.card}>
                    <span class={styles.cardLabel}>Exit</span>
                    <span class={styles.cardValue}>{advisory.exit_guidance}</span>
                </div>
                <div class={styles.card}>
                    <span class={styles.cardLabel}>Protection</span>
                    <span class={styles.cardValue}>{advisory.protection_strategy}</span>
                </div>
                <div class={styles.card}>
                    <span class={styles.cardLabel}>Target</span>
                    <span class={styles.cardValue}>{advisory.target_strategy}</span>
                </div>
            </div>
        </div>

        {#if advisory.final_recommendation}
            <div class={styles.section}>
                <div class={styles.sectionTitle}>Recommendation</div>
                <div class={styles.recommendation}>{advisory.final_recommendation}</div>
            </div>
        {/if}
    {/if}
</div>