<script lang="ts">
    import type { AdvisoryMatrix, OpportunityMatrix } from '../types';
    import { useAppStore } from '../state.svelte';
    import styles from './AdvisoryPanel.module.css';
    import { deriveTradePlan } from '../lib/tradePlan';

    const app = useAppStore();
    let { pairKey } = $props<{ pairKey: string }>();

    const instance = $derived(app.instancesMap[pairKey]);
    const advisory = $derived<AdvisoryMatrix | null>(instance?.advisory ?? null);

    const snapshot = $derived(instance?.microTerm.latestSnapshot as any);
    const decisionCtx = $derived(snapshot?.decision_context ?? null);
    const opportunity = $derived<OpportunityMatrix | null>(snapshot?.opportunity ?? null);
    const markPrice = $derived(parseFloat(instance?.microTerm?.priceText ?? '0') || 0);

    const tradePlan = $derived(deriveTradePlan({
        symbol: pairKey,
        markPrice,
        opportunity,
        advisory,
        analysis: instance?.analysis ?? null,
        decisionContext: decisionCtx,
        tf: instance?.microTerm,
        microTf: instance?.microTerm,
        overallRisk: instance?.risk?.overall_risk?.score,
    }));

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
    const stopLossPct = $derived((advisory as any)?.stop_loss_distance_pct ?? 0);

    function fmtPx(n: number, mp: number): string {
        if (n == null || !isFinite(n) || n <= 0) return '—';
        if (mp >= 1000) return `$${n.toFixed(0)}`;
        if (mp >= 1) return `$${n.toFixed(2)}`;
        return `$${n.toFixed(4)}`;
    }

    function handleApplyPlan() {
        app.activePlan = tradePlan as unknown as Record<string, unknown>;
        app.activeConsoleOpen = true;
    }

    function rrCls(rr: number | null): string {
        if (rr == null) return styles.rrNone ?? '';
        if (rr >= 2.0) return styles.green;
        if (rr >= 1.0) return styles.amber;
        return styles.red;
    }
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
            {#if stopLossPct > 0}
                <div class={styles.metricBar}>
                    <span class={styles.metricLabel}>Stop-Loss</span>
                    <div class={styles.metricBarBg}>
                        <div class="{styles.metricFill} {styles.blue}"
                             style="width: {Math.min(stopLossPct * 500, 100).toFixed(1)}%"></div>
                    </div>
                    <span class={styles.metricVal}>{(stopLossPct * 100).toFixed(2)}%</span>
                </div>
            {/if}
        </div>

        {#if opportunity}
            <div class={styles.section}>
                <div class={styles.sectionTitle}>Levels</div>
                <div class={styles.grid2}>
                    <div class={styles.card}>
                        <span class={styles.cardLabel}>Entry Zone</span>
                        <span class={styles.cardValue}>{opportunity.entry_zone.low.toFixed(0)} – {opportunity.entry_zone.high.toFixed(0)}</span>
                    </div>
                    <div class={styles.card}>
                        <span class={styles.cardLabel}>Target Zone</span>
                        <span class={styles.cardValue}>{opportunity.target_zone.low.toFixed(0)} – {opportunity.target_zone.high.toFixed(0)}</span>
                    </div>
                    <div class={styles.card}>
                        <span class={styles.cardLabel}>Invalidation</span>
                        <span class={styles.cardValue}>{opportunity.invalidation_level.toFixed(0)}</span>
                    </div>
                    <div class={styles.card}>
                        <span class={styles.cardLabel}>Horizon</span>
                        <span class={styles.cardValue}>{opportunity.time_horizon}</span>
                    </div>
                </div>
            </div>
        {/if}

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

        <!-- ── Structured TP1/TP2/TP3 + SL ladder (institutional Plan view) ── -->
        <div class={styles.section}>
            <div class={styles.sectionTitle}>
                Structured Plan
                <span class={styles.planHorizon}>{tradePlan.timeHorizon}</span>
            </div>

            <div class={styles.planGrid}>
                <div class={styles.planEntry}>
                    <div class={styles.planLabel}>ENTRY</div>
                    {#if tradePlan.entryMid > 0}
                        <div class={styles.planPrice}>{fmtPx(tradePlan.entryMid, markPrice)}</div>
                        <div class={styles.planRange}>{fmtPx(tradePlan.entryZone.low, markPrice)} – {fmtPx(tradePlan.entryZone.high, markPrice)}</div>
                        {#if tradePlan.entrySources.length > 0}
                            <div class={styles.planSources}>
                                {#each tradePlan.entrySources as src, i (i)}
                                    <span class="{styles.planSourceTag} {src.tag === 'FIB' ? styles.tagFib ?? '' :
                                                                  src.tag === 'VP'  ? styles.tagVp  ?? '' :
                                                                  src.tag === 'PP'  ? styles.tagPp  ?? '' :
                                                                  src.tag === 'SR'  ? styles.tagSr  ?? '' :
                                                                  src.tag === 'LIQ' ? styles.tagLiq ?? '' : ''}">
                                        {src.tag}
                                    </span>
                                {/each}
                            </div>
                        {/if}
                    {:else}
                        <div class={styles.planEmpty}>—</div>
                    {/if}
                </div>

                <div class={styles.planTps}>
                    <div class={styles.planLabel}>TARGETS</div>
                    {#if tradePlan.targets.length > 0}
                        {#each tradePlan.targets as t (t.label)}
                            <div class={styles.planTpRow}>
                                <span class={styles.planTpLabel}>{t.label}</span>
                                <span class={styles.planTpPrice}>{fmtPx(t.price, markPrice)}</span>
                                <span class={styles.planTpPct}>{t.sizePct}%</span>
                                <span class={styles.planTpRr}>
                                    R:R <span class={rrCls(t.rrRatio)}>{t.rrRatio == null ? '—' : t.rrRatio.toFixed(2)}</span>
                                </span>
                                <span class={styles.planTpSource}>
                                    {t.source === 'FIB_EXT_1618' ? '1.618 ext' :
                                     t.source === 'FIB_EXT_2618' ? '2.618 ext' :
                                     t.source === 'L4_TARGET_ZONE' ? 'L4 zone' :
                                     t.source === 'CONFLUENT' ? 'confluent' : ''}
                                </span>
                            </div>
                        {/each}
                    {:else}
                        <div class={styles.planEmpty}>—</div>
                    {/if}
                </div>

                <div class={styles.planStop}>
                    <div class={styles.planLabel}>STOP</div>
                    {#if tradePlan.stop}
                        <div class={styles.planPrice}>{fmtPx(tradePlan.stop.price, markPrice)}</div>
                        <div class={styles.planStopDist}>−{tradePlan.stop.distancePct.toFixed(2)}% · {tradePlan.stop.method.replace(/_/g, ' ').toLowerCase()}</div>
                        {#if tradePlan.stop.fallbackPrice}
                            <div class={styles.planFallback}>
                                <span class={styles.planFallbackLabel}>fallback:</span>
                                <span class={styles.planFallbackPrice}>{fmtPx(tradePlan.stop.fallbackPrice, markPrice)}</span>
                            </div>
                        {/if}
                    {:else}
                        <div class={styles.planEmpty}>—</div>
                    {/if}
                </div>
            </div>

            <button
                class={styles.applyPlanBtn}
                onclick={handleApplyPlan}
                disabled={!tradePlan.actionable}
                title="Pre-fill the BottomConsole bracket creator with TP1/TP2/TP3/SL"
            >
                Apply Plan to Console →
            </button>
        </div>

        {#if advisory.final_recommendation}
            <div class={styles.section}>
                <div class={styles.sectionTitle}>Recommendation</div>
                <div class={styles.recommendation}>{advisory.final_recommendation}</div>
            </div>
        {/if}
    {/if}
</div>
