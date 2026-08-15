<script lang="ts">
    import type { AdvisoryMatrix, DecisionContext, MarketSnapshot, OpportunityMatrix, TimeframeTelemetry } from '../types';
    import { useAppStore } from '../state.svelte';
    import type { WsState } from '../lib/websocket.svelte';
    import { buildRecommendationTabExport, verdictAwareGuidance } from '../lib/exportBuilders/recommendationTab';
    import ExportDataButton from './ExportDataButton.svelte';
    import LayerHeader from './LayerHeader.svelte';
    import { buildL6DecisionHeader, type LayerHeaderSpec } from '../lib/layerHeader';
    import styles from './RecommendationPanel.module.css';
    import { deriveTradePlan } from '../lib/tradePlan';
    import { computeDecisionRank, entryDangerLevel, selectProfileSide, profileZones, topSetupSummary } from '../lib/decisionRank';

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
    const markPrice = $derived.by(() => {
        // Use the last completed-candle close (set once per candle close
        // by the WebSocket handler) instead of the live micro shadow
        // tick's priceText. Trade-plan geometry (entry/target/invalidation)
        // must stay in sync with the pair-level matrices, both of which
        // only update on completed candles. Micro shadow fallback exists
        // only for the brief warmup window before any slot has closed.
        const completedClose = parseFloat(instance?.lastCompletedClose ?? '');
        if (Number.isFinite(completedClose) && completedClose > 0) return completedClose;
        return parseFloat(instance?.microTerm?.priceText ?? '0') || 0;
    });
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

    // ── Unified net bias (single gauge replacing the two runner bars) ────
    // v6.10.12 (GAUGE-001): the needle IS the final single number (net
    // bias long − short); no percentage split is rendered under the dial.
    // The raw probabilities remain in the export (`gauge.long_pct` /
    // `hold_pct` / `short_pct`) for data consumers.
    const longPct = $derived(rank.long.probability);
    const shortPct = $derived(rank.short.probability);
    // R1 + FIX-3 (v6.10.15) + v6.10.17 decoupling: the needle is the
    // VERDICT-consistent directional indicator. It renders neutral only
    // when the top action itself is HOLD — a STAND ASIDE badge no longer
    // erases a directional lean (the gate reports *when* you can act, not
    // *what* the market is saying). A directional-but-gated state now
    // shows the real net-bias needle next to the STAND ASIDE badge.
    const gaugeNeutral = $derived(rank.top === 'HOLD');
    const displayNet = $derived(gaugeNeutral ? 0 : longPct - shortPct);
    const netAbs = $derived(Math.abs(displayNet));
    const biasDirection = $derived<'LONG' | 'SHORT' | 'NEUTRAL'>(
        displayNet > 0 ? 'LONG' : displayNet < 0 ? 'SHORT' : 'NEUTRAL',
    );

    // SVG gauge: map displayNet [-100, +100] to angle [π (left), 0 (right)]
    // with π/2 (top center) = neutral.  SVG y is downward, so:
    //   needleX = cx + r * cos(angle)
    //   needleY = cy - r * sin(angle)
    const cx = 100; const cy = 105; const r = 70;
    const gaugeAngle = $derived(Math.PI - ((displayNet + 100) / 200) * Math.PI);
    const needleX = $derived(cx + r * Math.cos(gaugeAngle));
    const needleY = $derived(cy - r * Math.sin(gaugeAngle));
    const activeArcD = $derived(() => {
        // Draw arc from top-center (π/2) to needle position. The arc must
        // be a geometrically congruent segment of the Dome curve — same
        // radius (r=70), same curvature, bulging OUTWARD (away from the
        // circle center at (cx, cy)). The sweepFlag picks which of SVG's
        // four possible arcs is rendered:
        //   - sweep > 0 (SHORT bias): sweepFlag = 0 → counterclockwise
        //     from top-center, bulges UP-LEFT along the Dome.
        //   - sweep < 0 (LONG bias): sweepFlag = 1 → clockwise from
        //     top-center, bulges UP-RIGHT along the Dome.
        // Flipping these (the previous "sweep > 0 ? 1 : 0") forced SVG
        // onto the OTHER circle (center above the chord) and made the
        // active arc bulge inward — the inverse of the Dome.
        const midAngle = Math.PI / 2;
        const sweep = gaugeAngle - midAngle;
        const large = Math.abs(sweep) > Math.PI ? 1 : 0;
        const sweepFlag = sweep > 0 ? 0 : 1;
        const ex = cx + r * Math.cos(gaugeAngle);
        const ey = cy - r * Math.sin(gaugeAngle);
        return `M ${cx + r * Math.cos(midAngle)} ${cy - r * Math.sin(midAngle)} A ${r} ${r} 0 ${large} ${sweepFlag} ${ex} ${ey}`;
    });
    const arcColor = $derived(displayNet > 0 ? '#22c55e' : displayNet < 0 ? '#ef4444' : 'rgba(255,255,255,0.30)');
    const netLabel = $derived(displayNet === 0 ? '0%' : `${displayNet > 0 ? '+' : ''}${displayNet}%`);
    const netColor = $derived(displayNet > 0 ? '#22c55e' : displayNet < 0 ? '#ef4444' : '#f59e0b');

    // ── L6 LayerHeader — single authoritative verdict ────────────────────
    // The opportunity matrix is passed so the Risk-Adj R:R chip can
    // explain its discount in the tooltip (RR-001, v6.10.12).
    const headerSpec = $derived<LayerHeaderSpec>(buildL6DecisionHeader({
        rank,
        decisionContext: decisionCtx,
        advisory,
        opportunity,
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
            headerSpec,
            terms: {
                microTerm: instance?.microTerm as any,
                fastTerm: instance?.fastTerm as any,
                slowTerm: instance?.slowTerm as any,
                macroTerm: instance?.macroTerm as any,
            },
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
    // v6.10.17 (A3): `topSetupSummary` now ALWAYS publishes a bracket when
    // the opportunity matrix exists — a No Clear state yields the
    // aggregated bracket on the bias side marked NoClear (informational),
    // so the operator always has TPs/SLs/R:R to work with. The No Clear
    // explanation card renders alongside it (it explains WHY there is no
    // qualifying profile, not that no levels exist).
    const topSetup = $derived(topSetupSummary(opportunity, analysis, decisionCtx));
    const hasNoClearSetup = $derived(
        !!opportunity
            && !!advisory
            && (opportunity?.primary_opportunity ?? '') === 'NoClearOpportunity'
            && (topSetup === null || topSetup.viability === 'NoClear'),
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
    function rrColorCls(rr: number): string {
        if (rr >= 2.0) return styles.rrGood ?? '';
        if (rr >= 1.0) return styles.rrFair ?? '';
        return styles.rrPoor ?? '';
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

    <!-- Unified directional gauge — net bias from Long% − Short%,
         shown as a semi-circular dial. Center = Neutral, right = Long (green),
         left = Short (red). -->
    <div class={styles.gaugeCard}>
        <div class={styles.gaugeWrap}>
            <svg viewBox="0 0 200 115" class={styles.gauge}>
                <!-- Background arc: full semi-circle -->
                <path d="M 30 105 A 70 70 0 0 1 170 105"
                      stroke="rgba(255,255,255,0.08)" stroke-width="2.5" fill="none" stroke-linecap="round"/>
                <!-- Active bias arc: from top-center to needle. The arc itself is
                     the needle indicator — a geometrically congruent
                     segment of the Dome curve, bulging outward along
                     the Dome's curvature. -->
                <path d={activeArcD()} stroke={arcColor} stroke-width="4.5" fill="none"
                      stroke-linecap="round" opacity={netAbs > 0 ? 0.95 : 0}/>
                <!-- Needle: thin straight line from pivot to the active
                     arc's terminus. Always visible — at non-neutral
                     biases it carries the bias color (green/red); at
                     neutral it carries the amber "no lean" color and
                     renders as a vertical line straight up. -->
                <line x1={cx} y1={cy + 4} x2={needleX} y2={needleY}
                      stroke={netColor} stroke-width="2" stroke-linecap="round"
                      opacity="0.95"/>
            </svg>
            <div class={styles.gaugeLabels}>
                <span class="{styles.gaugeShort} {biasDirection !== 'SHORT' ? styles.dim : ''}">SHORT</span>
                <span class="{styles.gaugeNet} {biasDirection === 'LONG' ? styles.gaugeNetLong : biasDirection === 'SHORT' ? styles.gaugeNetShort : styles.gaugeNetNeutral}">{netLabel}</span>
                <span class="{styles.gaugeLong} {biasDirection !== 'LONG' ? styles.dim : ''}">LONG</span>
            </div>
            <!-- v6.10.19 (P6): cognitive transparency — the floors boosted
                 this lean; the operator sees it's not a deep consensus. -->
            {#if rank.lean_floor_applied}
                <div class={styles.leanFloorTag}>LEAN — floor-boosted (low-confidence read)</div>
            {/if}
        </div>
    </div>

    <!-- ── Top Setup card (single highest-scored profile) ──────────────────── -->
    <div class={styles.section}>
        <div class={styles.sectionTitle}>
            {topSetup && topSetup.below_floor ? 'Reference Bracket (Below Actionable Floor)' : 'Top Setup'}
            <span class={styles.sectionMeta}>
                {topSetup ? (topSetup.below_floor ? 'not actionable' : `score ${topSetup.score.toFixed(0)}`) : 'no qualifying setup yet'}
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
                {#if topSetup.below_floor}
                    <!-- v6.10.19 (T3): sub-1 R:R reference brackets are
                         red-flagged and never framed as a trade. -->
                    <div class={styles.profileCardBadgeInverted}>R:R BELOW ACTIONABLE FLOOR</div>
                    <div class={styles.profileCardNotes}>Reference only — the bracket's R:R is below the 1.0 actionable floor. Levels are shown for manual analysis; nothing here is a trade.</div>
                {:else if topSetup.viability === 'Actionable' && rank.top !== 'HOLD'}
                    <div class={styles.profileCardBadgeActionable}>ACTIONABLE</div>
                {:else if topSetup.viability === 'Qualifying'}
                    <!-- v6.10.17 (P1): a qualifying profile (preconditions
                         met, real bracket) is never a "no clear setup". -->
                    <div class={styles.profileCardBadgeNeutral}>QUALIFYING</div>
                {:else if topSetup.viability === 'DirectionalNeutral'}
                    <div class={styles.profileCardBadgeNeutral}>HOLD · NO DIRECTIONAL EDGE</div>
                {:else if topSetup.viability === 'GeometryInverted'}
                    <div class={styles.profileCardBadgeInverted}>GEOMETRY INVERTED</div>
                {:else if topSetup.viability === 'NoClear'}
                    <div class={styles.profileCardBadgeNoClear}>NO CLEAR SETUP</div>
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
                                ? (topSetup.zones.target.low > 0
                                    ? `$${fmtPriceScale(topSetup.zones.target.low, markPrice)}–$${fmtPriceScale(topSetup.zones.target.high, markPrice)}`
                                    : `$${fmtPriceScale(topSetup.zones.target.high, markPrice)}`)
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
                            {#if topSetup.rr != null}
                                <span class={rrColorCls(topSetup.rr)}>
                                    {topSetup.rr >= 9.99 ? 'R:R 1 : 9.99+' : topSetup.rr >= 5 ? `R:R 1 : ${topSetup.rr.toFixed(1)}` : `R:R 1 : ${topSetup.rr.toFixed(2)}`}
                                </span>
                            {:else}
                                <span class={styles.rrNa} title={topSetup.rr_reason ?? 'R:R not available'}>R:R N/A</span>
                            {/if}
                        </span>
                    </div>
                </div>
                {#if topSetup.rationale && topSetup.rationale !== `${topSetup.opportunity_type}: preconditions ${topSetup.preconditions_met}/${topSetup.preconditions_total}`}
                    <div class={styles.profileCardNotes}>{topSetup.rationale}</div>
                {/if}
            </div>
        {/if}
        {#if hasNoClearSetup}
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
                <span class={styles.kpiLabel}>ATR Stop Guide</span>
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
        {#if rank.top === 'HOLD'}
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
         single muted line. The Top Setup card above carries the
         aggregated bracket on the net-bias side — R:R reads N/A when
         geometry is inverted and the bracket is non-actionable. -->
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
                    The Top Setup card above carries the aggregated bracket on the net-bias side for reference; when geometry is inverted its R:R reads N/A and the bracket is non-actionable.
                </span>
            </div>
        {/if}
    </div>

    <!-- ── Strategy (environment playbook — never a trade call) ── -->
    <div class={styles.section}>
        <div class={styles.sectionTitle}>Environment Playbook</div>
        {#if rank.top === 'HOLD'}
            <div class={styles.whyNote}>For reference — no active directional call. The guidance below describes the environment, not a trade trigger.</div>
        {/if}
        <div class={styles.grid2}>
            <div class={styles.card}>
                <span class={styles.cardLabel}>Entry</span>
                <!-- v6.10.16 (FIX-O5) + v6.10.17: the playbook values are
                     non-actionable ONLY under a genuine HOLD verdict (the
                     directional-but-gated state carries a real lean, so its
                     playbook is real too). -->
                <span class={styles.cardValue}>{rank.top === 'HOLD' ? '—' : sanitizeLabel(advisory?.entry_guidance ?? '')}</span>
            </div>
            <div class={styles.card}>
                <span class={styles.cardLabel}>Exit</span>
                <span class={styles.cardValue}>{rank.top === 'HOLD' ? '—' : sanitizeLabel(advisory?.exit_guidance ?? '')}</span>
            </div>
            <div class={styles.card}>
                <span class={styles.cardLabel}>Protection</span>
                <span class={styles.cardValue}>{rank.top === 'HOLD' ? '—' : prettifyEnum(advisory?.protection_strategy ?? '')}</span>
            </div>
            <div class={styles.card}>
                <span class={styles.cardLabel}>Target</span>
                <span class={styles.cardValue}>{rank.top === 'HOLD' ? '—' : prettifyEnum(advisory?.target_strategy ?? '')}</span>
            </div>
        </div>
    </div>

    <!-- ── Final Verdict ──
         R6 + FIX-4 (v6.10.15) + v6.10.17: the verdict hero is the
         authoritative call. Under a genuine HOLD it reads "no directional
         call"; under a directional lean it carries the graded percentage
         AND the readiness gate ("SHORT lean 38% — STAND ASIDE (entry_danger
         HIGH)") — the operator always sees what the market says and when it
         can be acted on. The advisory's `final_recommendation` renders as
         muted environment guidance below. -->
    <div class={styles.section}>
        <div class={styles.sectionTitle}>Final Verdict</div>
        {#if rank.top === 'HOLD'}
            <blockquote class={styles.verdictQuote}>
                HOLD — no directional call (readiness: {rank.headline.state}).
            </blockquote>
            {#if verdictAwareGuidance(advisory, rank.top)}
                <div class={styles.verdictGuidance}>Environment guidance: {verdictAwareGuidance(advisory, rank.top)}</div>
            {/if}
        {:else}
            {@const verdictPct = Math.round(rank.top_prob)}
            {@const verdictSentence =
                rank.headline.state === 'STAND_ASIDE'
                    ? `${rank.top} lean ${verdictPct}% — STAND ASIDE (readiness: STAND_ASIDE, entry_danger ${dangerLevel}).`
                    : rank.headline.state === 'READY'
                        ? `${rank.top} ${verdictPct}% — READY (readiness: READY).`
                        : `${rank.top} lean ${verdictPct}% — awaiting confirmation (readiness: ${rank.headline.state}).`}
            <blockquote class={styles.verdictQuote}>{verdictSentence}</blockquote>
            {#if verdictAwareGuidance(advisory, rank.top)}
                <div class={styles.verdictGuidance}>Environment guidance: {verdictAwareGuidance(advisory, rank.top)}</div>
            {/if}
        {/if}
    </div>
</div>
