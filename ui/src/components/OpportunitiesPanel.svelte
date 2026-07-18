<script lang="ts">
    import type { AnalysisMatrix } from '../types';
    import { useAppStore } from '../state.svelte';
    import styles from './OpportunitiesPanel.module.css';

    const app = useAppStore();
    let { pairKey } = $props<{ pairKey: string }>();

    const instance = $derived(app.instancesMap[pairKey]);
    const analysis = $derived<AnalysisMatrix | null>(instance?.analysis ?? null);

    function oppClass(o: string): string {
        switch (o) {
            case 'TrendContinuation': return styles.oppTrend;
            case 'Breakout': return styles.oppBreakout;
            case 'Pullback': return styles.oppPullback;
            case 'MeanReversion': return styles.oppDefault;
            case 'Reversal': return styles.oppReversal;
            case 'LiquiditySqueeze': return styles.oppReversal;
            case 'NoClearOpportunity': return styles.oppNone;
            default: return styles.oppNone;
        }
    }
    function oppLabel(o: string): string {
        return o.replace(/([A-Z])/g, ' $1').trim();
    }
    function scoreColor(s: number): string {
        if (s >= 85) return '#22c55e';
        if (s >= 70) return '#4ade80';
        if (s >= 50) return '#f59e0b';
        if (s >= 30) return '#f87171';
        return '#ef4444';
    }
    function setupQuality(s: number): { label: string; cls: string } {
        if (s >= 85) return { label: 'PRIME', cls: styles.prime };
        if (s >= 70) return { label: 'STRONG', cls: styles.strong };
        if (s >= 50) return { label: 'MODERATE', cls: styles.moderate };
        if (s >= 30) return { label: 'MARGINAL', cls: styles.marginal };
        return { label: 'NONE', cls: styles.none };
    }

    const oppScore = $derived(analysis
        ? (analysis.bias === 'StrongBullish' ? 85 : analysis.bias === 'Bullish' ? 65 :
           analysis.bias === 'StrongBearish' ? 85 : analysis.bias === 'Bearish' ? 65 : 30)
        : 0);

    const q = $derived(setupQuality(oppScore));
</script>

<div class={styles.panel}>
    {#if !analysis || !analysis.timeframes_considered}
        <div class={styles.placeholder}>Awaiting opportunity data...</div>
    {:else}
        <h2 class={styles.title}>Market Opportunity</h2>

        <div class={styles.section}>
            <span class="{styles.oppBadge} {oppClass(analysis.opportunity_analysis)}">
                {oppLabel(analysis.opportunity_analysis)}
            </span>

            <div class={styles.scoreRow}>
                <span class={styles.scoreLabel}>Setup Score</span>
                <div class={styles.scoreBar}>
                    <div class={styles.scoreFill}
                         style="width: {oppScore.toFixed(1)}%; background: {scoreColor(oppScore)}"></div>
                </div>
                <span class={styles.scoreVal} style="color: {scoreColor(oppScore)}">{oppScore.toFixed(0)}</span>
            </div>
            <div style="margin-top: 6px;">
                <span class="{styles.qualityBadge} {q.cls}">{q.label}</span>
            </div>
        </div>

        <div class={styles.section}>
            <div class={styles.sectionTitle}>Market Position</div>
            <div class={styles.zoneGrid}>
                <div class={styles.zoneCard}>
                    <span class={styles.zoneLabel}>Bias</span>
                    <span class={styles.zoneValue}>{analysis.bias}</span>
                </div>
                <div class={styles.zoneCard}>
                    <span class={styles.zoneLabel}>Regime</span>
                    <span class={styles.zoneValue}>{analysis.market_regime}</span>
                </div>
                <div class={styles.zoneCard}>
                    <span class={styles.zoneLabel}>Trend</span>
                    <span class={styles.zoneValue}>{analysis.trend_assessment}</span>
                </div>
                <div class={styles.zoneCard}>
                    <span class={styles.zoneLabel}>Quality</span>
                    <span class={styles.zoneValue}>{analysis.market_quality}</span>
                </div>
            </div>
        </div>

        <div class={styles.section}>
            <div class={styles.sectionTitle}>Environment</div>
            <div class={styles.infoRow}>
                <span class={styles.infoBadge}>{analysis.timeframes_considered}/4 TFs considered</span>
                <span class={styles.infoBadge}>Confidence: {(analysis.confidence * 100).toFixed(0)}%</span>
            </div>
        </div>
    {/if}
</div>