<script lang="ts">
    import type { AdvisoryMatrix, DecisionContext, MarketSnapshot, OpportunityMatrix, TimeframeTelemetry } from '../types';
    import { useAppStore } from '../state.svelte';
    import type { WsState } from '../lib/websocket.svelte';
    import { buildRecommendationTabExport } from '../lib/exportBuilders/recommendationTab';
    import { buildFilterStateBlock } from '../lib/exportBuilders/shared';
    import ExportDataButton from './ExportDataButton.svelte';
    import LayerHeader from './LayerHeader.svelte';
    import { buildL6DecisionHeader, type LayerHeaderSpec } from '../lib/layerHeader';
    import styles from './RecommendationPanel.module.css';
    import { deriveTradePlan } from '../lib/tradePlan';
    import { computeDecisionRank, entryDangerLevel, selectProfileSide, profileZones, topSetupSummary } from '../lib/decisionRank';
    import { computeRiskReward, discountRiskReward, type RiskRewardDisplay } from '../lib/riskReward';

    const app = useAppStore();
    let { pairKey, wssState } = $props<{ pairKey: string; wssState?: WsState }>();

    const instance = $derived(app.instancesMap[pairKey]);
    const advisory = $derived<AdvisoryMatrix | null>(instance?.advisory ?? null);

    const microTerm = $derived<TimeframeTelemetry | undefined>(instance?.microTerm);
    const snapshot = $derived(instance?.microTerm.latestSnapshot as unknown as MarketSnapshot | undefined);
    // ── Bind contract: `instance.decisionContext` is the mirror field
    // populated once per completed candle by `applySnapshotToTimeframe`.
    // Reading it first avoids the shadow-tick wipe that used to null-out
    // `microTerm.latestSnapshot.decision_context` between candle closes.
    // The snapshot field is kept as a fallback for the brief warmup window.
    const decisionCtx = $derived<DecisionContext | null>(
        (instance?.decisionContext ?? (snapshot as any)?.decision_context ?? null) as DecisionContext | null,
    );
    const opportunity = $derived<OpportunityMatrix | null>(instance?.opportunity ?? null);
    const analysis = $derived(instance?.analysis ?? null);
    const markPrice = $derived(parseFloat(instance?.microTerm?.priceText ?? '0') || 0);
    const timestamp = $derived<number | null>(
        snapshot && typeof (snapshot as any).timestamp === 'number'
            ? (snapshot as any).timestamp
            : null
    );
    const registry = $derived(app.indicatorRegistry ?? []);

    // ── Unified decision rank ─────────────────────────────────────────────
    const rank = $derived(computeDecisionRank({
        advisory,
        decisionContext: decisionCtx,
        opportunity,
        analysis,
    }));

    // ── Runner-ups (winner excluded, ranked descending) ───────────────────
    const runners = $derived.by((): { action: 'LONG' | 'SHORT' | 'HOLD'; prob: number }[] => {
        const all = [
            { action: 'LONG' as const, prob: rank.long.probability },
            { action: 'SHORT' as const, prob: rank.short.probability },
            { action: 'HOLD' as const, prob: rank.hold.probability },
        ];
        return all.filter((r) => r.action !== rank.top).sort((a, b) => b.prob - a.prob);
    });

    // L6 LayerHeader — single authoritative verdict (the operator's
    // top-of-mind answer). The L3 bias is intentionally NOT consumed
    // here so the header can never echo the L3 input.
    const headerSpec = $derived<LayerHeaderSpec>(buildL6DecisionHeader({
        rank,
        decisionContext: decisionCtx,
        advisory,
    }));

    function buildExport() {
        return buildRecommendationTabExport({
            advisory,
            decisionContext: decisionCtx,
            opportunity,
            analysis,
            symbol: pairKey,
            tfSecs: microTerm?.barDurationSec ?? null,
            timestamp,
            markPrice,
            filterState: buildFilterStateBlock({
                activeOnly: false,
                confirmedPlusOnly: false,
                hideGates: false,
                hideOverlays: false,
            }),
        });
    }

    // Keep deriveTradePlan wired so BottomConsole / TradePlanStrip stay fed.
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

    // ── Cosmetic helpers ──────────────────────────────────────────────────
    function sanitizeLabel(s: string): string {
        if (!s) return '\u2014';
        let cleaned = s.replace(/([a-z])([A-Z])/g, '$1 $2');
        cleaned = cleaned.replace(/_/g, ' ');
        cleaned = cleaned.trim().replace(/\s+/g, ' ');
        return cleaned
            .toLowerCase()
            .replace(/\b\w/g, (c) => c.toUpperCase());
    }

    function prettifyEnum(s: string): string {
        if (!s) return '\u2014';
        let cleaned = s.replace(/([A-Z]+)([A-Z][a-z])/g, '$1 $2');
        cleaned = cleaned.replace(/([a-z])([A-Z])/g, '$1 $2');
        cleaned = cleaned.replace(/_/g, ' ');
        cleaned = cleaned.trim().replace(/\s+/g, ' ');
        cleaned = cleaned
            .toLowerCase()
            .replace(/\b\w/g, (c) => c.toUpperCase());
        cleaned = cleaned.replace(/\sBased$/i, '-Based');
        cleaned = cleaned
            .replace(/^Atr\b/i, 'ATR')
            .replace(/^Sr\b/i, 'S/R')
            .replace(/^Rr\b/i, 'R:R')
            .replace(/^Sl\b/i, 'SL');
        return cleaned;
    }

    const rrDisplay = $derived(decisionCtx?.expected_reward_risk_ratio ?? 0);
    // `entry_danger` is now a RiskDimension-shaped object on the wire.
    // The legacy code (`decisionCtx?.entry_danger ?? 50`) treated it as a
    // bare scalar and got the wrong numbers. Read `.score` defensively.
    const dangerRaw = $derived(decisionCtx?.entry_danger);
    const dangerDisplay = $derived(
        typeof dangerRaw === 'number'
            ? dangerRaw
            : (dangerRaw as { score?: number } | null)?.score ?? 50,
    );
    const dangerLevel = $derived(entryDangerLevel(dangerDisplay));
    const dangerState = $derived((dangerRaw as { state?: string } | null)?.state ?? 'Unknown');
    const confidenceDisplay = $derived(advisory?.confidence_assessment ?? 0);
    const stopLossPct = $derived(advisory?.stop_loss_distance_pct ?? 0);

    // ── Top Setup card model ──────────────────────────────────────────────
    // The Recommendation panel surfaces ONLY the highest-scored qualifying
    // profile (the operator's actionable decision). The Opportunities panel
    // renders the full leaderboard. Both panels read from the same wire
    // and produce the same numbers for the top profile.
    //
    // The top-scored profile card ALWAYS shows ENTRY / TARGET / SL / R:R:
    // `topSetupSummary` falls back from per-profile zones to the
    // aggregated primary bracket so even Neutral-family profiles
    // surface a usable price level. R:R prefers the wire's per-side
    // `expected_rr_internal` over the geometric computation.
    const topSetup = $derived(topSetupSummary(opportunity, analysis));
    const hasNoClearSetup = $derived(
        topSetup === null
            && !!opportunity
            && !!advisory
            && (opportunity?.primary_opportunity ?? '') === 'NoClearOpportunity',
    );

    // ── R:R (Risk-Adj R:R) display: when verdict is HOLD AND the
    // discount is 0, surface "N/A — no directional bias" instead of a
    // misleading "0.00" that operators read as "this trade has 0 R:R".
    const riskAdjRrDisplay = $derived.by((): { value: string; isNA: boolean } => {
        const v = decisionCtx?.expected_reward_risk_ratio ?? 0;
        if (rank.top === 'HOLD' && v === 0) {
            return { value: 'N/A', isNA: true };
        }
        return { value: v.toFixed(2), isNA: false };
    });

    // ── Hero direction-class mapping ──────────────────────────────────────
    function verdictClass(action: 'LONG' | 'SHORT' | 'HOLD'): string {
        if (action === 'LONG') return styles.verdictLong ?? '';
        if (action === 'SHORT') return styles.verdictShort ?? '';
        return styles.verdictHold ?? '';
    }
    function envHeaderClass(direction: 'long' | 'short' | 'neutral'): string {
        if (direction === 'long') return styles.envHeaderLong ?? '';
        if (direction === 'short') return styles.envHeaderShort ?? '';
        return styles.envHeaderNeutral ?? '';
    }
    function rankBarClass(action: 'LONG' | 'SHORT' | 'HOLD'): string {
        if (action === 'LONG') return styles.rankLong ?? '';
        if (action === 'SHORT') return styles.rankShort ?? '';
        return styles.rankHold ?? '';
    }
    function fmtPriceScale(n: number, mp: number): string {
        if (mp >= 1000) return n.toFixed(0);
        if (mp >= 1) return n.toFixed(2);
        if (mp >= 0.01) return n.toFixed(4);
        if (mp >= 0.0001) return n.toFixed(6);
        return n.toFixed(8);
    }
</script>

<div class={styles.panel}>
    <!-- v7.0-prod: the panel-level banner above the LayerHeader was removed
         (D9 — no text above any badge). Per-section empty states still
         surface from within the body when a matrix hasn't loaded yet. -->
    <LayerHeader spec={headerSpec}>
        {#snippet trailing()}
            <h2 class={styles.title}>Recommendation</h2>
            <ExportDataButton onExport={buildExport} title="Copy all Recommendation data as JSON" />
        {/snippet}
    </LayerHeader>

    <!-- Probability bars survive — but restyled as a chip rail (LONG/SHORT/HOLD
         per-action percentages) so they read as an extension of the meta chip
         vocabulary rather than as a second header. -->
    <div class={styles.runnerRow}>
        {#each runners as r (r.action)}
            <div class="{styles.runnerCell} {rankBarClass(r.action)}">
                <div class={styles.runnerBar} style="width: {r.prob.toFixed(1)}%"></div>
                <span class={styles.runnerAction}>{r.action}</span>
                <span class={styles.runnerPct}>{r.prob}%</span>
            </div>
        {/each}
    </div>

    <!-- ── Top Setup card (single highest-scored profile) ──────────────────── -->
    <div class={styles.section}>
        <div class={styles.sectionTitle}>
            Top Setup
            <span class={styles.sectionMeta}>
                {topSetup ? `score ${topSetup.score.toFixed(0)}` : 'no qualifying setup yet'}
            </span>
        </div>
        {#if topSetup}
            <div class="{styles.profileCardRec} {topSetup.direction === 'LONG' ? styles.recLong : topSetup.direction === 'SHORT' ? styles.recShort : styles.recNeutral}">
                <div class={styles.profileCardHead}>
                    <span class={styles.profileCardTitle}>
                        {sanitizeLabel(topSetup.opportunity_type)}
                    </span>
                    <span class="{styles.profileCardDir} {topSetup.direction === 'LONG' ? styles.dirLong : topSetup.direction === 'SHORT' ? styles.dirShort : styles.dirNeutral}">
                        {topSetup.direction === 'LONG' ? 'LONG' : topSetup.direction === 'SHORT' ? 'SHORT' : 'NEUTRAL'}
                    </span>
                    <span class={styles.profileCardScore}>{topSetup.score.toFixed(0)}</span>
                </div>
                {#if topSetup.viability === 'Actionable' && rank.top !== 'HOLD'}
                    <div class={styles.profileCardBadgeActionable}>ACTIONABLE</div>
                {:else if topSetup.viability === 'DirectionalNeutral'}
                    <div class={styles.profileCardBadgeNeutral}>HOLD · NO DIRECTIONAL EDGE</div>
                {:else if topSetup.viability === 'GeometryInverted'}
                    <div class={styles.profileCardBadgeInverted}>GEOMETRY INVERTED</div>
                {/if}
                <div class={styles.profileCardPre}>
                    <span class={styles.profileCardPreLabel}>Preconditions</span>
                    <span class={styles.profileCardPreVal}>{topSetup.preconditions_met}/{topSetup.preconditions_total}</span>
                </div>
                <!-- ENTRY / TARGET / SL / R:R are ALWAYS rendered. The fallback
                     chain goes per-profile zones → aggregated primary bracket,
                     so even Neutral-family profiles surface the close-pinned
                     Neutral sentinel with R:R = 0. -->
                <div class={styles.profileRecZones}>
                    <div class={styles.profileRecZone}>
                        <span class={styles.profileRecZoneLabel}>ENTRY</span>
                        <span class={styles.profileRecZoneValue}>
                            {topSetup.zones
                                ? `$${fmtPriceScale(topSetup.zones.entry.low, markPrice)}–$${fmtPriceScale(topSetup.zones.entry.high, markPrice)}`
                                : '—'}
                        </span>
                    </div>
                    <div class={styles.profileRecZone}>
                        <span class={styles.profileRecZoneLabel}>TARGET</span>
                        <span class={styles.profileRecZoneValue}>
                            {topSetup.zones
                                ? `$${fmtPriceScale(topSetup.zones.target.low, markPrice)}–$${fmtPriceScale(topSetup.zones.target.high, markPrice)}`
                                : '—'}
                        </span>
                    </div>
                    <div class={styles.profileRecZone}>
                        <span class={styles.profileRecZoneLabel}>SL</span>
                        <span class={styles.profileRecZoneValue}>
                            {topSetup.zones && topSetup.zones.invalidation > 0
                                ? `$${fmtPriceScale(topSetup.zones.invalidation, markPrice)}`
                                : '—'}
                        </span>
                    </div>
                    <div class={styles.profileRecZone}>
                        <span class={styles.profileRecZoneLabel}>R:R</span>
                        <span class={styles.profileRecZoneValue}>
                            {(() => {
                                const z = topSetup.zones;
                                if (z && z.entry && z.target && z.invalidation) {
                                    const disp = computeRiskReward(z.entry, z.target, z.invalidation, z.side, markPrice);
                                    return disp.display;
                                }
                                return topSetup.rr != null ? `R:R ${topSetup.rr.toFixed(2)}` : '—';
                            })()}
                        </span>
                    </div>
                </div>
                {#if topSetup.zones && !topSetup.zones.geometry_consistent}
                    <div class={styles.profileRecNote}>
                        ⚠ entry/target layout inverted — falling back to aggregated Neutral sentinel.
                    </div>
                {/if}
                {#if topSetup.rationale && topSetup.rationale !== `${topSetup.opportunity_type}: preconditions ${topSetup.preconditions_met}/${topSetup.preconditions_total}`}
                    <div class={styles.profileCardNotes}>{topSetup.rationale}</div>
                {/if}
            </div>
        {:else if hasNoClearSetup}
            <div class={styles.noClearCard}>
                <div class={styles.noClearTitle}>No Clear Setup</div>
                <div class={styles.noClearBody}>
                    {advisory?.final_recommendation || opportunity?.invalidation_note || 'No qualifying setup; market conditions do not currently favor a directional trade.'}
                </div>
            </div>
        {/if}
    </div>

    <!-- ── Safety flags (5 chips) -->
    <div class={styles.section}>
        <div class={styles.sectionTitle}>Safety Flags</div>
        <div class={styles.kpiStrip}>
            <div class={styles.kpi}>
                <span class={styles.kpiLabel}>Readiness</span>
                <span class={styles.kpiVal} style="color: {rank.headline.state === 'READY' ? '#22c55e' : rank.headline.state === 'STAND_ASIDE' ? '#ef4444' : '#f59e0b'}">
                    {rank.headline.state}
                </span>
            </div>
            <div class={styles.kpi}>
                <span class={styles.kpiLabel}>Risk-Adj R:R</span>
                <span class={styles.kpiVal} style="color: {riskAdjRrDisplay.isNA ? '#94a3b8' : rrDisplay >= 2 ? '#22c55e' : rrDisplay >= 1 ? '#f59e0b' : '#ef4444'}">
                    {(() => {
                        if (riskAdjRrDisplay.isNA) return riskAdjRrDisplay.value;
                        const v = Number(riskAdjRrDisplay.value);
                        if (Number.isNaN(v) || v <= 0) return 'R:R \u2014';
                        const norm = v >= 9.99 ? '9.99+' : v >= 5 ? v.toFixed(1) : v.toFixed(2);
                        return `R:R 1 : ${norm}`;
                    })()}
                </span>
            </div>
            <div class={styles.kpi}>
                <span class={styles.kpiLabel}>Stop-Loss</span>
                <span class={styles.kpiVal}>
                    {stopLossPct > 0 ? `${stopLossPct.toFixed(2)}%` : '—'}
                </span>
            </div>
            <div class={styles.kpi}>
                <span class={styles.kpiLabel}>Confidence</span>
                <span class={styles.kpiVal} style="color: {confidenceDisplay >= 60 ? '#22c55e' : confidenceDisplay >= 30 ? '#f59e0b' : '#ef4444'}">
                    {confidenceDisplay.toFixed(0)}%
                </span>
            </div>
            <div class={styles.kpi}>
                <span class={styles.kpiLabel}>Entry Danger</span>
                <span class={styles.kpiVal} style="color: {dangerDisplay >= 60 ? '#ef4444' : dangerDisplay >= 30 ? '#f59e0b' : '#22c55e'}">
                    {dangerDisplay.toFixed(0)} ({dangerLevel})
                </span>
            </div>
        </div>
    </div>

    <!-- ── Why (top-3 rationale, gated by rank consistency) -->
    <div class={styles.section}>
        <div class={styles.sectionTitle}>Why</div>
        {#if rank.headline.action === 'STAND_ASIDE' || hasNoClearSetup}
            <div class={styles.whyNote}>
                No directional edge — these bullets read the same across all three arms (LONG/SHORT/HOLD). They trace the data, not a trade call.
            </div>
        {/if}
        <ul class={styles.why}>
            {#each rank.rationale.slice(0, 3) as line, i (i)}
                <li class={styles.whyItem}>{line}</li>
            {/each}
        </ul>
    </div>

    <!-- ── Price Levels ──
         LONG/SHORT verdict: show the active side's per-side zones (the
         primary actionable bracket).
         HOLD verdict: collapse the two-card hypothetical grid into a
         single muted line. The Top Setup card above already shows the
         Neutral sentinel (entry = target = invalidation = close; R:R =
         0.00) so the operator still has the actionable numbers. -->
    <div class={styles.section}>
        <div class={styles.sectionTitle}>Price Levels</div>
        {#if rank.top === 'LONG' || rank.top === 'SHORT'}
            {@const side = rank.top === 'LONG'
                ? { entry: opportunity?.long_entry_zone, target: opportunity?.long_target_zone, inval: opportunity?.long_invalidation_level }
                : { entry: opportunity?.short_entry_zone, target: opportunity?.short_target_zone, inval: opportunity?.short_invalidation_level }}
            <div class={styles.grid2}>
                <div class={styles.card}>
                    <span class={styles.cardLabel}>Entry Zone — {rank.top}</span>
                    <span class={styles.cardValue}>
                        {side.entry ? `$${fmtPriceScale(side.entry.low, markPrice)} – $${fmtPriceScale(side.entry.high, markPrice)}` : '—'}
                    </span>
                </div>
                <div class={styles.card}>
                    <span class={styles.cardLabel}>Target Zone — {rank.top}</span>
                    <span class={styles.cardValue}>
                        {side.target ? `$${fmtPriceScale(side.target.low, markPrice)} – $${fmtPriceScale(side.target.high, markPrice)}` : '—'}
                    </span>
                </div>
                <div class={styles.card}>
                    <span class={styles.cardLabel}>Invalidation — {rank.top}</span>
                    <span class={styles.cardValue}>
                        {side.inval ? `$${fmtPriceScale(side.inval, markPrice)}` : '—'}
                    </span>
                </div>
                <div class={styles.card}>
                    <span class={styles.cardLabel}>Horizon</span>
                    <span class={styles.cardValue}>{opportunity?.time_horizon ?? '—'}</span>
                </div>
            </div>
        {:else}
            <div class={styles.holdPriceLevels}>
                <span class={styles.holdPriceLevelsLabel}>No active setup — verdict is HOLD.</span>
                <span class={styles.holdPriceLevelsSub}>
                    Top Setup card above carries the Neutral primary bracket (entry = target = invalidation = close; R:R = 0.00).
                </span>
            </div>
        {/if}
    </div>

    <!-- ── Strategy -->
    <div class={styles.section}>
        <div class={styles.sectionTitle}>Strategy</div>
        <div class={styles.grid2}>
            <div class={styles.card}>
                <span class={styles.cardLabel}>Entry</span>
                <span class={styles.cardValue}>{sanitizeLabel(advisory?.entry_guidance ?? '')}</span>
            </div>
            <div class={styles.card}>
                <span class={styles.cardLabel}>Exit</span>
                <span class={styles.cardValue}>{sanitizeLabel(advisory?.exit_guidance ?? '')}</span>
            </div>
            <div class={styles.card}>
                <span class={styles.cardLabel}>Protection</span>
                <span class={styles.cardValue}>{prettifyEnum(advisory?.protection_strategy ?? '')}</span>
            </div>
            <div class={styles.card}>
                <span class={styles.cardLabel}>Target</span>
                <span class={styles.cardValue}>{prettifyEnum(advisory?.target_strategy ?? '')}</span>
            </div>
        </div>
    </div>

    <!-- ── Final Verdict (final_recommendation) -->
    <div class={styles.section}>
        <div class={styles.sectionTitle}>Final Verdict</div>
        <blockquote class={styles.verdictQuote}>{advisory?.final_recommendation || '—'}</blockquote>
    </div>
</div>
