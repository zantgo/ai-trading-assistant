<script lang="ts">
    import type { AnalysisMatrix, MarketSnapshot, OpportunityMatrix } from '../types';
    import { useAppStore } from '../state.svelte';
    import styles from './OpportunitiesPanel.module.css';

    const app = useAppStore();
    let { pairKey } = $props<{ pairKey: string }>();

    const instance = $derived(app.instancesMap[pairKey]);
    const analysis = $derived<AnalysisMatrix | null>(instance?.analysis ?? null);
    const snap = $derived(instance?.microTerm?.latestSnapshot as unknown as MarketSnapshot | undefined);
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
        if (s >= 30) return '#94a3b8';
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
        const stateConf = analysis.confidence ?? 0;
        const baseScore = stateConf * 100;
        const qualMap: Record<string, number> = {
            STRONG_BULLISH: 90, STRONG_BEARISH: 90,
            BULLISH: 70, BEARISH: 70, NEUTRAL: 45,
        };
        const biasKey = typeof analysis.bias === 'string' ? analysis.bias : '';
        const biasScore = qualMap[biasKey] ?? 40;
        return Math.round((biasScore * 0.6) + (baseScore * 0.4));
    });

    const markPrice = $derived(parseFloat(instance?.microTerm?.priceText ?? '0') || 0);

    const q = $derived(setupQuality(oppScore));

    function fmtPx(n: number | undefined | null): string {
        if (n == null || !isFinite(n) || n <= 0) return '—';
        return n.toFixed(0);
    }
    function fmtRr(n: number | undefined | null): string {
        if (n == null || !isFinite(n)) return '—';
        return n.toFixed(2);
    }
    function fmtSource(s: string): string {
        switch (s) {
            case 'FIBONACCI': return 'FIB';
            case 'VOLUME_PROFILE': return 'VP';
            case 'PIVOT_POINTS': return 'PP';
            case 'SUPPORT_RESISTANCE': return 'SR';
            case 'LIQUIDITY_CLUSTER': return 'LIQ';
            default: return 'ATR';
        }
    }
</script>

<div class={styles.panel}>
    <h2 class={styles.title}>Market Opportunity</h2>

    <div class={styles.section}>
        <span class="{styles.oppBadge} {analysis ? oppClass(analysis.opportunity_analysis) : styles.oppNone}">
            {analysis ? oppLabel(analysis.opportunity_analysis) : '—'}
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

    <!-- ── Tactical Bracket ── -->
    <div class={styles.section}>
        <div class={styles.sectionTitle}>Tactical Bracket</div>
        <div class={styles.bracket}>
            <div class="{styles.bracketStep} {styles.bracketTarget}">
                <span class={styles.bracketLabel}>TARGET ZONE</span>
                <span class={styles.bracketValue}>
                    {opportunity ? `${fmtPx(opportunity.target_zone.low)} \u2013 ${fmtPx(opportunity.target_zone.high)}` : '\u2014'}
                </span>
                <span class={styles.bracketHint}>
                    {opportunity ? 'Take-Profit Range' : 'Waiting for structural pivot to define targets...'}
                </span>
            </div>
            <div class={styles.bracketConnector}></div>
            <div class="{styles.bracketStep} {styles.bracketMid}">
                <span class={styles.bracketLabel}>CURRENT MID</span>
                <span class={styles.bracketValue}>{markPrice > 0 ? `$${markPrice.toFixed(0)}` : '\u2014'}</span>
                <span class={styles.bracketHint}>Market Reference</span>
            </div>
            <div class={styles.bracketConnector}></div>
            <div class="{styles.bracketStep} {styles.bracketEntry}">
                <span class={styles.bracketLabel}>ENTRY ZONE</span>
                <span class={styles.bracketValue}>
                    {opportunity ? `${fmtPx(opportunity.entry_zone.low)} \u2013 ${fmtPx(opportunity.entry_zone.high)}` : '\u2014'}
                </span>
                <span class={styles.bracketHint}>
                    {opportunity ? 'Optimal Buy Zone' : 'Analyzing structure for entry alignment...'}
                </span>
            </div>
            <div class={styles.bracketConnector}></div>
            <div class="{styles.bracketStep} {styles.bracketInvalidation}">
                <span class={styles.bracketLabel}>INVALIDATION</span>
                <span class={styles.bracketValue}>{fmtPx(opportunity?.invalidation_level)}</span>
                <span class={styles.bracketHint}>Hard Stop Price</span>
            </div>
        </div>
    </div>

    <div class={styles.section}>
        <div class={styles.sectionTitle}>R:R (Internal)</div>
        <div class={styles.zoneGrid}>
            <div class={styles.zoneCard}>
                <span class={styles.zoneLabel}>Expected R:R</span>
                <span class={styles.rrValue}>{fmtRr(opportunity?.expected_rr_internal)}</span>
            </div>
            <div class={styles.zoneCard}>
                <span class={styles.zoneLabel}>Horizon</span>
                <span class={styles.zoneValue}>{opportunity?.time_horizon ?? '\u2014'}</span>
            </div>
        </div>
    </div>

    <!-- ── Invalidation note — always visible ── -->
    <div class={styles.section}>
        <div class={styles.sectionTitle}>Invalidation Note</div>
        <div class={styles.noteBox}>
            {opportunity?.invalidation_note || 'Assessment conditions forming — invalidation level will be calculated when structural pivot confirms.'}
        </div>
    </div>

    <!-- ── Evaluated profiles — always visible ── -->
    <div class={styles.section}>
        <div class={styles.sectionTitle}>Evaluated Setups</div>
        {#if opportunity?.profiles && opportunity.profiles.length > 0}
            <div class={styles.profileList}>
                {#each opportunity.profiles as profile (profile.opportunity_type)}
                    <div class="{styles.profileCard} {oppClass(profile.opportunity_type)}">
                        <div class={styles.profileHeader}>
                            <span class={styles.profileType}>{oppLabel(profile.opportunity_type)}</span>
                            <span class={styles.profileScore} style="color: {scoreColor(profile.score)}">{profile.score.toFixed(0)}</span>
                        </div>
                        <div class={styles.profilePreconditions}>
                            <span class={styles.profilePreLabel}>Preconditions</span>
                            <span class={styles.profilePreValue}>{profile.preconditions_met}/{profile.preconditions_total} met</span>
                            <div class={styles.profilePreBar}>
                                <div class={styles.profilePreFill}
                                     style="width: {profile.preconditions_total > 0 ? (profile.preconditions_met / profile.preconditions_total * 100).toFixed(0) : '0'}%; background: {scoreColor(profile.score)}"></div>
                            </div>
                        </div>
                        {#if profile.notes}
                            <div class={styles.profileNotes}>{profile.notes}</div>
                        {/if}
                    </div>
                {/each}
            </div>
        {:else}
            <div class={styles.noProfiles}>No setup profiles evaluated yet</div>
        {/if}
    </div>

    <!-- ── Confluent Entry Levels — always visible ── -->
    <div class={styles.section}>
        <div class={styles.sectionTitle}>Confluent Entry Levels</div>
        {#if opportunity?.confluent_entry_levels && opportunity.confluent_entry_levels.length > 0}
            {#each opportunity.confluent_entry_levels.slice(0, 4) as level}
                <div class={styles.confluenceRow}>
                    <span class={styles.confluencePrice}>{level.price.toFixed(0)}</span>
                    <div class={styles.confluenceSources}>
                        {#each level.sources as src}
                            <span class={styles.sourceTag} style="background: {sourceColor(src)}22; color: {sourceColor(src)}; border-color: {sourceColor(src)}44">
                                {fmtSource(src)}
                            </span>
                        {/each}
                    </div>
                    <span class={styles.confluenceStr} style="color: {scoreColor(level.strength)}">{level.strength.toFixed(0)}%</span>
                </div>
            {/each}
        {:else}
            <div class={styles.noConfluence}>No confluent levels</div>
        {/if}
    </div>

    <!-- ── Confluent Targets — always visible ── -->
    <div class={styles.section}>
        <div class={styles.sectionTitle}>Confluent Targets</div>
        {#if opportunity?.confluent_target_levels && opportunity.confluent_target_levels.length > 0}
            {#each opportunity.confluent_target_levels.slice(0, 4) as level}
                <div class={styles.confluenceRow}>
                    <span class={styles.confluencePrice}>{level.price.toFixed(0)}</span>
                    <div class={styles.confluenceSources}>
                        {#each level.sources as src}
                            <span class={styles.sourceTag} style="background: {sourceColor(src)}22; color: {sourceColor(src)}; border-color: {sourceColor(src)}44">
                                {fmtSource(src)}
                            </span>
                        {/each}
                    </div>
                    <span class={styles.confluenceStr} style="color: {scoreColor(level.strength)}">{level.strength.toFixed(0)}%</span>
                </div>
            {/each}
        {:else}
            <div class={styles.noConfluence}>No confluent levels</div>
        {/if}
    </div>

    <div class={styles.section}>
        <div class={styles.sectionTitle}>Market Position</div>
        <div class={styles.zoneGrid}>
            <div class={styles.zoneCard}>
                <span class={styles.zoneLabel}>Bias</span>
                <span class={styles.zoneValue}>{analysis?.bias ?? '—'}</span>
            </div>
            <div class={styles.zoneCard}>
                <span class={styles.zoneLabel}>Regime</span>
                <span class={styles.zoneValue}>{analysis?.market_regime ?? '—'}</span>
            </div>
            <div class={styles.zoneCard}>
                <span class={styles.zoneLabel}>Trend</span>
                <span class={styles.zoneValue}>{analysis?.trend_assessment ?? '—'}</span>
            </div>
            <div class={styles.zoneCard}>
                <span class={styles.zoneLabel}>Quality</span>
                <span class={styles.zoneValue}>{analysis?.market_quality ?? '—'}</span>
            </div>
        </div>
    </div>

    <div class={styles.section}>
        <div class={styles.sectionTitle}>Environment</div>
        <div class={styles.infoRow}>
            <span class={styles.infoBadge}>{analysis?.timeframes_considered ?? 0}/4 TFs considered</span>
            <span class={styles.infoBadge}>Confidence: {analysis ? (analysis.confidence * 100).toFixed(0) : '—'}%</span>
        </div>
    </div>
</div>
