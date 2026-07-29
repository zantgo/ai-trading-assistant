<script lang="ts">
    import type { AdvisoryMatrix, AnalysisMatrix, DecisionContext, MarketSnapshot, OpportunityMatrix, TimeframeTelemetry } from '../types';
    import { useAppStore } from '../state.svelte';
    import { buildPanelExportJson } from '../lib/metricsExport';
    import ExportDataButton from './ExportDataButton.svelte';
    import styles from './OpportunitiesPanel.module.css';
    import { computeDecisionRank, computeSymmetricSetups } from '../lib/decisionRank';

    const app = useAppStore();
    let { pairKey } = $props<{ pairKey: string }>();

    const instance = $derived(app.instancesMap[pairKey]);
    const analysis = $derived<AnalysisMatrix | null>(instance?.analysis ?? null);
    const snap = $derived(instance?.microTerm?.latestSnapshot as unknown as MarketSnapshot | undefined);
    const opportunity = $derived<OpportunityMatrix | null>(instance?.opportunity ?? null);
    const microTerm = $derived<TimeframeTelemetry | undefined>(instance?.microTerm);
    const decisionContext = $derived<DecisionContext | null>((snap as any)?.decision_context ?? null);
    const advisory = $derived<AdvisoryMatrix | null>(instance?.advisory ?? null);
    const markPrice = $derived(parseFloat(instance?.microTerm?.priceText ?? '0') || 0);
    const timestamp = $derived<number | null>(
        snap && typeof (snap as any).timestamp === 'number'
            ? (snap as any).timestamp
            : null
    );
    const registry = $derived(app.indicatorRegistry ?? []);

    // ── Unified decision rank + symmetric setups ─────────────────────────
    const rank = $derived(computeDecisionRank({
        advisory,
        decisionContext,
        opportunity,
        analysis,
    }));
    const setups = $derived(computeSymmetricSetups({
        opportunity,
        markPrice,
        topAction: rank.top,
        readiness: rank.headline.state,
    }));

    function buildExport() {
        return buildPanelExportJson({
            sourceTab: 'opportunity',
            pairKey,
            resolvers: {
                symbol: pairKey,
                tfLabel: 'Micro',
                tfSecs: microTerm?.barDurationSec ?? 0,
                timestamp,
                markPrice,
                registry: registry as any,
                tf: (microTerm ?? { indicators: {} }) as TimeframeTelemetry,
                filters: { activeOnly: false, confirmedPlusOnly: false, hideGates: false, hideOverlays: false },
                analysis,
                risk: instance?.risk ?? null,
                alignment: (instance?.alignment as unknown as Record<string, unknown>) ?? null,
                opportunity,
                advisory: instance?.advisory ?? null,
                volumeProfile: (microTerm as any)?.volumeProfile ?? null,
                liquidity: (microTerm as any)?.liquidity ?? null,
                cluster: (microTerm as any)?.cluster ?? null,
                liquiditySignals: ((microTerm as any)?.liquiditySignals ?? []) as any[],
                decisionContext: (decisionContext as unknown as Record<string, unknown>) ?? null,
            },
        });
    }

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

    // ── Trade Setups helpers ─────────────────────────────────────────────
    function setupHeaderClass(side: 'LONG' | 'SHORT'): string {
        return side === 'LONG' ? (styles.setupHeaderLong ?? '') : (styles.setupHeaderShort ?? '');
    }
    function fmtPxDecimal(n: number, mp: number): string {
        if (n == null || !isFinite(n) || n <= 0) return '—';
        if (mp >= 1000) return `$${n.toFixed(0)}`;
        if (mp >= 1) return `$${n.toFixed(2)}`;
        return `$${n.toFixed(4)}`;
    }
    function rrCls(rr: number | null): string {
        if (rr == null) return styles.rrNone ?? '';
        if (rr >= 2.0) return styles.green;
        if (rr >= 1.0) return styles.amber;
        return styles.red;
    }
</script>

<div class={styles.panel}>
    <div class={styles.panelHeader}>
        <h2 class={styles.title}>Market Opportunity</h2>
        <ExportDataButton onExport={buildExport} title="Copy all Opportunity data as JSON" />
    </div>

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

    <!-- ── Trade Setups (symmetric Long + mirrored Short) ────────────────── -->
    <div class={styles.section}>
        <div class={styles.sectionTitle}>Trade Setups</div>
        <div class={styles.setupPair}>
            <!-- Long Setup -->
            <div class="{styles.setupCard} {setups.long.active ? styles.setupCardActive : styles.setupCardInactive}">
                <div class="{styles.setupHeader} {setupHeaderClass('LONG')}">
                    <span class={styles.setupHeaderTitle}>Long Setup</span>
                    <span class={styles.setupStatus}>{setups.long.status}</span>
                </div>
                <div class={styles.setupBody}>
                    <div class={styles.setupRow}>
                        <span class={styles.setupRowLabel}>ENTRY</span>
                        <span class={styles.setupRowValue}>
                            {setups.long.entry ? fmtPxDecimal(setups.long.entry.price, markPrice) : '—'}
                        </span>
                    </div>
                    {#each setups.long.targets as t (t.label)}
                        <div class={styles.setupRow}>
                            <span class={styles.setupRowLabel}>{t.label}</span>
                            <span class={styles.setupRowValue}>{fmtPxDecimal(t.price, markPrice)}</span>
                            {#if t.label === 'TP1' && setups.long.rrRatio != null}
                                <span class={styles.setupRowRr}>R:R <span class={rrCls(setups.long.rrRatio)}>{setups.long.rrRatio.toFixed(2)}</span></span>
                            {/if}
                        </div>
                    {/each}
                    {#if setups.long.stop}
                        <div class={styles.setupRow}>
                            <span class={styles.setupRowLabel}>SL</span>
                            <span class="{styles.setupRowValue} {styles.setupRowStop}">{fmtPxDecimal(setups.long.stop.price, markPrice)}</span>
                        </div>
                    {/if}
                </div>
            </div>

            <!-- Short Setup (mirror around markPrice) -->
            <div class="{styles.setupCard} {setups.short.active ? styles.setupCardActive : styles.setupCardInactive}">
                <div class="{styles.setupHeader} {setupHeaderClass('SHORT')}">
                    <span class={styles.setupHeaderTitle}>Short Setup</span>
                    <span class={styles.setupStatus}>{setups.short.status}</span>
                </div>
                <div class={styles.setupBody}>
                    <div class={styles.setupRow}>
                        <span class={styles.setupRowLabel}>ENTRY</span>
                        <span class={styles.setupRowValue}>
                            {setups.short.entry ? fmtPxDecimal(setups.short.entry.price, markPrice) : '—'}
                        </span>
                    </div>
                    {#each setups.short.targets as t (t.label)}
                        <div class={styles.setupRow}>
                            <span class={styles.setupRowLabel}>{t.label}</span>
                            <span class={styles.setupRowValue}>{fmtPxDecimal(t.price, markPrice)}</span>
                            {#if t.label === 'TP1' && setups.short.rrRatio != null}
                                <span class={styles.setupRowRr}>R:R <span class={rrCls(setups.short.rrRatio)}>{setups.short.rrRatio.toFixed(2)}</span></span>
                            {/if}
                        </div>
                    {/each}
                    {#if setups.short.stop}
                        <div class={styles.setupRow}>
                            <span class={styles.setupRowLabel}>SL</span>
                            <span class="{styles.setupRowValue} {styles.setupRowStop}">{fmtPxDecimal(setups.short.stop.price, markPrice)}</span>
                        </div>
                    {/if}
                </div>
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
