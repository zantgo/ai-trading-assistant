<script lang="ts">
    import type { AnalysisMatrix, OpportunityMatrix } from '../types';
    import { useAppStore } from '../state.svelte';
    import styles from './OpportunitiesPanel.module.css';

    const app = useAppStore();
    let { pairKey } = $props<{ pairKey: string }>();

    const instance = $derived(app.instancesMap[pairKey]);
    const analysis = $derived<AnalysisMatrix | null>(instance?.analysis ?? null);
    const snap = $derived(instance?.microTerm?.latestSnapshot as any);
    const opportunity = $derived<OpportunityMatrix | null>(snap?.opportunity ?? null);

    function oppClass(o: string): string {
        switch (o) {
            case 'TrendContinuation': return styles.oppTrend;
            case 'Breakout': return styles.oppBreakout;
            case 'Pullback': return styles.oppPullback;
            case 'MeanReversion': return styles.oppDefault;
            case 'Reversal': return styles.oppReversal;
            case 'LiquiditySqueeze': return styles.oppReversal;
            case 'Scalp': return styles.oppTrend;
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
    function sourceColor(s: string): string {
        switch (s) {
            case 'FIBONACCI': return '#ff9800';
            case 'VOLUME_PROFILE': return '#00bcd4';
            case 'PIVOT_POINTS': return '#ab47bc';
            case 'SUPPORT_RESISTANCE': return '#66bb6a';
            case 'LIQUIDITY_CLUSTER': return '#ef5350';
            default: return '#78909c';
        }
    }

    const oppScore = $derived.by(() => {
        if (!analysis) return 0;
        const stateConf = (analysis as any).confidence ?? 0;
        const baseScore = stateConf * 100;
        const qualMap: Record<string, number> = {
            STRONG_BULLISH: 90, STRONG_BEARISH: 90,
            BULLISH: 70, BEARISH: 70, NEUTRAL: 45,
        };
        const biasKey = typeof analysis.bias === 'string' ? analysis.bias : '';
        const biasScore = qualMap[biasKey] ?? 40;
        return Math.round((biasScore * 0.6) + (baseScore * 0.4));
    });

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

        {#if opportunity}
            <div class={styles.section}>
                <div class={styles.sectionTitle}>Entry & Target Zones</div>
                <div class={styles.zoneGrid}>
                    <div class={styles.zoneCard}>
                        <span class={styles.zoneLabel}>Entry</span>
                        <span class={styles.zoneValue}>{opportunity.entry_zone.low.toFixed(0)} – {opportunity.entry_zone.high.toFixed(0)}</span>
                    </div>
                    <div class={styles.zoneCard}>
                        <span class={styles.zoneLabel}>Target</span>
                        <span class={styles.zoneValue}>{opportunity.target_zone.low.toFixed(0)} – {opportunity.target_zone.high.toFixed(0)}</span>
                    </div>
                    <div class={styles.zoneCard}>
                        <span class={styles.zoneLabel}>Invalidation</span>
                        <span class={styles.zoneValue}>{opportunity.invalidation_level.toFixed(0)}</span>
                    </div>
                    <div class={styles.zoneCard}>
                        <span class={styles.zoneLabel}>R:R (Internal)</span>
                        <span class={styles.zoneValue}>{opportunity.expected_rr_internal.toFixed(2)}</span>
                    </div>
                </div>
            </div>

            {#if opportunity.confluent_entry_levels?.length > 0}
                <div class={styles.section}>
                    <div class={styles.sectionTitle}>Confluent Entry Levels</div>
                    {#each opportunity.confluent_entry_levels.slice(0, 4) as level}
                        <div class={styles.confluenceRow}>
                            <span class={styles.confluencePrice}>{level.price.toFixed(0)}</span>
                            <div class={styles.confluenceSources}>
                                {#each level.sources as src}
                                    <span class={styles.sourceTag} style="background: {sourceColor(src)}22; color: {sourceColor(src)}; border-color: {sourceColor(src)}44">
                                        {src === 'FIBONACCI' ? 'FIB' : src === 'VOLUME_PROFILE' ? 'VP' : src === 'PIVOT_POINTS' ? 'PP' : src === 'SUPPORT_RESISTANCE' ? 'SR' : src === 'LIQUIDITY_CLUSTER' ? 'LIQ' : 'ATR'}
                                    </span>
                                {/each}
                            </div>
                            <span class={styles.confluenceStr} style="color: {scoreColor(level.strength)}">{level.strength.toFixed(0)}%</span>
                        </div>
                    {/each}
                </div>
            {/if}

            {#if opportunity.confluent_target_levels?.length > 0}
                <div class={styles.section}>
                    <div class={styles.sectionTitle}>Confluent Targets</div>
                    {#each opportunity.confluent_target_levels.slice(0, 4) as level}
                        <div class={styles.confluenceRow}>
                            <span class={styles.confluencePrice}>{level.price.toFixed(0)}</span>
                            <div class={styles.confluenceSources}>
                                {#each level.sources as src}
                                    <span class={styles.sourceTag} style="background: {sourceColor(src)}22; color: {sourceColor(src)}; border-color: {sourceColor(src)}44">
                                        {src === 'FIBONACCI' ? 'FIB' : src === 'VOLUME_PROFILE' ? 'VP' : src === 'PIVOT_POINTS' ? 'PP' : src === 'SUPPORT_RESISTANCE' ? 'SR' : src === 'LIQUIDITY_CLUSTER' ? 'LIQ' : 'ATR'}
                                    </span>
                                {/each}
                            </div>
                            <span class={styles.confluenceStr} style="color: {scoreColor(level.strength)}">{level.strength.toFixed(0)}%</span>
                        </div>
                    {/each}
                </div>
            {/if}
        {/if}

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
