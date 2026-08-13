<script lang="ts">
    import type { AdvisoryMatrix, AnalysisMatrix, DecisionContext, MarketSnapshot, OpportunityMatrix, TimeframeTelemetry } from '../types';
    import { useAppStore } from '../state.svelte';
    import type { WsState } from '../lib/websocket.svelte';
    import { buildOpportunityTabExport } from '../lib/exportBuilders/opportunityTab';
    import ExportDataButton from './ExportDataButton.svelte';
    import LayerHeader from './LayerHeader.svelte';
    import { buildL4OpportunityHeader, type LayerHeaderSpec } from '../lib/layerHeader';
    import styles from './OpportunitiesPanel.module.css';
    import { computeDecisionRank, computeSymmetricSetups, selectProfileSide, profileZones, profileSummary } from '../lib/decisionRank';
import { computeOpportunityBars, type DirectionalBars } from '../lib/opportunityBars';

    const app = useAppStore();
    let { pairKey, wssState } = $props<{ pairKey: string; wssState?: WsState }>();

    const instance = $derived(app.instancesMap[pairKey]);
    const analysis = $derived<AnalysisMatrix | null>(instance?.analysis ?? null);
    const snap = $derived(instance?.microTerm?.latestSnapshot as unknown as MarketSnapshot | undefined);
    const opportunity = $derived<OpportunityMatrix | null>(instance?.opportunity ?? null);
    const microTerm = $derived<TimeframeTelemetry | undefined>(instance?.microTerm);
    // ── Bind contract: `instance.decisionContext` is the mirror field
    // populated once per completed candle by `applySnapshotToTimeframe`.
    // Reading it first avoids the shadow-tick wipe that used to null-out
    // `microTerm.latestSnapshot.decision_context` between candle closes.
    // The snapshot field is kept as a fallback for the brief warmup window.
    const decisionContext = $derived<DecisionContext | null>(
        (instance?.decisionContext ?? (snap as any)?.decision_context ?? null) as DecisionContext | null,
    );
    const advisory = $derived<AdvisoryMatrix | null>(instance?.advisory ?? null);
    const markPrice = $derived.by(() => {
        // Use the last completed-candle close (set once per candle close
        // by the WebSocket handler) instead of the live micro shadow
        // tick's priceText. Geometry that depends on markPrice
        // (entry_zone, target_zone, invalidation_level) must stay in
        // sync with the pair-level matrices, both of which only update
        // on completed candles. The micro shadow fallback exists only for
        // the brief warmup window before any slot has closed.
        const completedClose = parseFloat(instance?.lastCompletedClose ?? '');
        if (Number.isFinite(completedClose) && completedClose > 0) return completedClose;
        return parseFloat(instance?.microTerm?.priceText ?? '0') || 0;
    });
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

    // Directional conviction bars — normalized from the top-level opportunity
    // matrix R:R values and capped by opportunity_score so the remaining
    // uncertainty remains visible as a Hold buffer.
    //
    // All three bars (BULLISH/BEARISH/HOLD) are ALWAYS rendered, even at
    // 0%. The previous behaviour filtered out zero-value bars which hid
    // the dominant-HOLD case (the chart showed only a single HOLD=100%
    // bar and operators couldn't see that bullish/bearish were genuinely
    // zero). Showing all three explicitly communicates the full split.
    const directionBars = $derived.by((): DirectionalBars => computeOpportunityBars(opportunity));
    const sortedBars = $derived.by(() => [
        { id: 'bullish', label: 'BULLISH', value: directionBars.bullish, cls: 'bullish' },
        { id: 'bearish', label: 'BEARISH', value: directionBars.bearish, cls: 'bearish' },
        { id: 'hold', label: 'HOLD', value: directionBars.hold, cls: 'hold' },
    ]
        .sort((a, b) => b.value - a.value));

    const setups = $derived(computeSymmetricSetups({
        opportunity,
        markPrice,
        topAction: rank.top,
        readiness: rank.headline.state,
    }));

    // ── Per-profile Trade Setup cards ──────────────────────────────────────
    // The Opportunities panel renders the full leaderboard: every
    // qualifying profile (preconditions_met > 0) gets its own
    // actionable bracket. ENTRY / TARGET / SL / R:R are ALWAYS present:
    // `profileSummary` falls back from per-profile zones to the
    // aggregated primary bracket so even Neutral-family profiles
    // (e.g. MeanReversion + Neutral bias) surface the close-pinned
    // Neutral sentinel.
    //
    // NoClearOpportunity is filtered out of the Trade Setup list —
    // it's the unconditional "no actionable setup" fallback, rendered
    // separately as a muted placeholder in the Evaluated Setups section.
    type Viability = 'Actionable' | 'DirectionalNeutral' | 'GeometryInverted' | 'NoClear';
    interface ActiveSetup {
        opportunity_type: string;
        side: 'LONG' | 'SHORT' | 'NEUTRAL';
        entryMid: number;
        entryLow: number;
        entryHigh: number;
        tp1: number;
        tp2: number;
        invalidation: number;
        rr: number | null;
        geometry_consistent: boolean;
        score: number;
        preconditions_met: number;
        preconditions_total: number;
        notes: string;
        viability: Viability;
        rankIdx: number;
    }
    // Viability ordering: Actionable first, then DirectionalNeutral,
    // then GeometryInverted. Within each tier, sort by score desc.
    const viabilityRank: Record<Viability, number> = {
        Actionable: 0,
        DirectionalNeutral: 1,
        GeometryInverted: 2,
        NoClear: 3,
    };
    const activeSetups = $derived.by((): ActiveSetup[] => {
        const profiles = opportunity?.profiles ?? [];
        const qualifying = profiles
            .filter((p) => p.preconditions_met > 0 && p.opportunity_type !== 'NoClearOpportunity')
            .slice()
            .sort((a, b) => b.score - a.score);
        const macroBias = analysis?.bias ?? null;
        const out: ActiveSetup[] = [];
        qualifying.forEach((p, idx) => {
            const s = profileSummary(p, opportunity, analysis, decisionContext);
            // Even when zones are null we still emit a card — the
            // operator sees the viability tag and the missing-zone
            // indicator (we render `—` for empty zones).
            const z = s.zones;
            const entryMid = z ? (z.entry.low + z.entry.high) / 2 : 0;
            const tpCandidates = z ? [z.target.low, z.target.high].filter((v) => v > 0) : [];
            const sortedTp = z
                ? [...tpCandidates].sort(
                    (a, b) => Math.abs(a - z.entry.low - ((z.entry.high - z.entry.low) / 2)) -
                              Math.abs(b - z.entry.low - ((z.entry.high - z.entry.low) / 2)),
                )
                : [];
            out.push({
                opportunity_type: p.opportunity_type,
                side: s.side,
                entryMid,
                entryLow: z?.entry.low ?? 0,
                entryHigh: z?.entry.high ?? 0,
                tp1: sortedTp[0] ?? 0,
                tp2: sortedTp.length > 1 ? sortedTp[1] : sortedTp[0] ?? 0,
                invalidation: z?.invalidation ?? 0,
                rr: s.rr,
                geometry_consistent: z?.geometry_consistent ?? false,
                score: p.score,
                preconditions_met: p.preconditions_met,
                preconditions_total: p.preconditions_total,
                notes: p.notes,
                viability: s.viability,
                rankIdx: idx,
            });
        });
        // Sort by viability tier (Actionable first), then by score desc.
        return out.sort((a, b) => {
            const va = viabilityRank[a.viability];
            const vb = viabilityRank[b.viability];
            if (va !== vb) return va - vb;
            return b.score - a.score;
        });
    });
    const topSetup = $derived(activeSetups[0] ?? null);

    // NoClearOpportunity profile → muted placeholder strip (NOT a
    // Trade Setup card). Shown only when it exists in the profiles.
    const noClearProfile = $derived(
        (opportunity?.profiles ?? []).find((p) => p.opportunity_type === 'NoClearOpportunity') ?? null,
    );

    // Active-side R:R (per-side, gated on macro bias). The legacy
    // matrix-level `expected_rr_internal` was removed in v6.9; the
    // canonical R:R is now the per-side field. When the bias is
    // Neutral (no active side), surface "N/A — no directional bias"
    // instead of a misleading "0.00" that operators read as "this
    // trade has 0 R:R".
    const rrInternalDisplay = $derived.by((): { value: string; isNA: boolean } => {
        const bias = analysis?.bias ?? 'Neutral';
        const opp = opportunity;
        let v = 0;
        if (opp) {
            if (bias === 'Bullish' || bias === 'StrongBullish') {
                v = opp.long_expected_rr_internal ?? 0;
            } else if (bias === 'Bearish' || bias === 'StrongBearish') {
                v = opp.short_expected_rr_internal ?? 0;
            } else {
                v = 0;
            }
        }
        if (rank.top === 'HOLD' && v === 0) {
            return { value: 'N/A', isNA: true };
        }
        return { value: v.toFixed(2), isNA: false };
    });

    function buildExport() {
        return buildOpportunityTabExport({
            opportunity,
            analysis,
            decisionContext,
            advisory,
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

    const oppScore = $derived<number>(opportunity?.opportunity_score ?? 0);

    const q = $derived(setupQuality(oppScore));

    function fmtScore(n: number): string {
        if (n >= 1) return n.toFixed(0);
        if (n >= 0.1) return n.toFixed(1);
        if (n >= 0.001) return n.toFixed(3);
        if (n <= 0) return '0';
        return n.toFixed(4);
    }

    // L4 LayerHeader — primary badge reads `primary_opportunity`; the
    // meta chip rail reports Score + R:R + Horizon. The L3 bias can
    // override the per-side RR direction when both sides are valid
    // (`buildL4OpportunityHeader` handles the disambiguation).
    const headerSpec = $derived<LayerHeaderSpec>(
        buildL4OpportunityHeader(opportunity, analysis?.bias ?? null)
    );

    // ── Lean (bullish / bearish / neutral) derived from the rank.
    // Reading from `rank.top` keeps the lean chip aligned with the verdict
    // hero and the geometry of the Trade Setups cards below. The header
    // itself has absorbed the previous `PULLBACK + bullish-dominate`
    // dual-badge block — `lean` here only powers the lean chip and the
    // HOLD scenario note below.
    const lean = $derived.by((): { label: string; tone: 'bull' | 'bear' | 'neutral' } => {
        if (rank.top === 'LONG') return { label: 'Bullish setups dominate', tone: 'bull' };
        if (rank.top === 'SHORT') return { label: 'Bearish setups dominate', tone: 'bear' };
        return { label: 'Lean: neutral', tone: 'neutral' };
    });

    function fmtPx(n: number | undefined | null, mp: number = 0): string {
        if (n == null || !isFinite(n) || n <= 0) return '—';
        if (mp >= 1000) return n.toFixed(0);
        if (mp >= 1) return n.toFixed(2);
        if (mp >= 0.01) return n.toFixed(4);
        if (mp >= 0.0001) return n.toFixed(6);
        return n.toFixed(8);
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
    function setupHeaderClass(side: 'LONG' | 'SHORT' | 'NEUTRAL'): string {
        if (side === 'LONG') return styles.setupHeaderLong ?? '';
        if (side === 'SHORT') return styles.setupHeaderShort ?? '';
        return '';
    }
    function fmtPxDecimal(n: number, mp: number): string {
        if (n == null || !isFinite(n) || n <= 0) return '—';
        if (mp >= 1000) return `$${n.toFixed(0)}`;
        if (mp >= 1) return `$${n.toFixed(2)}`;
        if (mp >= 0.01) return `$${n.toFixed(4)}`;
        if (mp >= 0.0001) return `$${n.toFixed(6)}`;
        return `$${n.toFixed(8)}`;
    }
    function rrCls(rr: number | null): string {
        if (rr == null) return styles.rrNone ?? '';
        if (rr >= 2.0) return styles.green;
        if (rr >= 1.0) return styles.amber;
        return styles.red;
    }
</script>

<div class={styles.panel}>
    <!-- L4 HEADER (v7.0-prod — shared chrome across all MME tabs) -->
    <LayerHeader spec={headerSpec}>
        {#snippet trailing()}
            <h2 class={styles.title}>Market Opportunity</h2>
            <ExportDataButton onExport={buildExport} title="Copy all Opportunity data as JSON" />
        {/snippet}
    </LayerHeader>

    <!-- Directional conviction bars — normalized from the top-level opportunity
         matrix R:R values and capped by opportunity_score. -->
    <div class={styles.dirBarRow}>
        {#each sortedBars as bar (bar.id)}
            <div class={styles.dirBarCell}>
                <div class="{styles.dirBarFill} {styles[bar.cls]}" style="width: {bar.value.toFixed(1)}%"></div>
                <span class={styles.dirBarLabel}>{bar.label}</span>
                <span class={styles.dirBarPct}>{bar.value}%</span>
            </div>
        {/each}
    </div>

    <div class={styles.section}>
            <div class={styles.topSetupLabel}>TOP SETUP</div>
            <div class={styles.headerRow}>
                <span class="{styles.oppBadge} {analysis ? oppClass(analysis.opportunity_analysis) : styles.oppNone}">
                    {analysis ? oppLabel(analysis.opportunity_analysis) : '—'}
                </span>
                <span class="{styles.leanChip} {lean.tone === 'bull' ? styles.leanBull : lean.tone === 'bear' ? styles.leanBear : styles.leanNeutral}">
                    {lean.label}
                </span>
            </div>

            <div class={styles.scoreRow}>
                <span class={styles.scoreLabel}>Setup Score</span>
                <div class={styles.scoreBar}>
                    <div class={styles.scoreFill}
                         style="width: {oppScore.toFixed(1)}%; background: {scoreColor(oppScore)}"></div>
                </div>
                <span class={styles.scoreVal} style="color: {scoreColor(oppScore)}">{fmtScore(oppScore)}</span>
            </div>
            <div style="margin-top: 6px;">
                <span class="{styles.qualityBadge} {q.cls}">{q.label}</span>
            </div>
        </div>

        <!-- ── Trade Setups (one card per qualifying profile) ── -->
        <div class={styles.section}>
            <div class={styles.sectionTitle}>
                Trade Setups
                <span class={styles.sectionMeta}>
                    {activeSetups.length === 0
                        ? 'no qualifying setup yet'
                        : `${activeSetups.length} candidate${activeSetups.length === 1 ? '' : 's'}`}
                </span>
            </div>
            {#if rank.top === 'HOLD'}
                <div class={styles.scenarioNote}>
                    <span class={styles.scenarioBadge}>HOLD / NO CLEAR</span>
                    <span>No directional call. The cards below show each qualifying profile's aggregated bracket — when geometry is inverted (entry/target/SL on the wrong side of close, or zero-bound contamination), R:R reads N/A and the bracket is non-actionable. None are active.</span>
                </div>
            {/if}
            {#if activeSetups.length === 0}
                <div class={styles.noProfiles}>Awaiting qualifying profile (preconditions_met &gt; 0).</div>
            {:else}
                <div class={styles.setupList}>
                    {#each activeSetups as setup (setup.opportunity_type)}
                        <div class="{styles.setupCard} {setup.viability === 'Actionable' && rank.top !== 'HOLD' ? styles.setupCardActive : styles.setupCardHypo} {!setup.geometry_consistent ? styles.setupCardInverted : ''} {setup.viability === 'DirectionalNeutral' ? styles.setupCardMuted : ''}">
                            <div class="{styles.setupHeader} {setupHeaderClass(setup.side)}">
                                <span class={styles.setupHeaderTitle}>{`${oppLabel(setup.opportunity_type)} · ${setup.side}`}</span>
                                <span class={styles.setupScoreInline} style="color: {scoreColor(setup.score)}">{fmtScore(setup.score)}</span>
                            </div>
                            {#if setup.viability === 'Actionable' && setup.rankIdx === 0 && rank.top !== 'HOLD'}
                                <div class={styles.setupBadgeTop}>TOP · ACTIONABLE</div>
                            {:else if setup.viability === 'DirectionalNeutral'}
                                <div class={styles.setupBadgeNeutral}>NEUTRAL · HOLD</div>
                            {:else if setup.viability === 'GeometryInverted'}
                                <div class={styles.setupBadgeInverted}>GEOMETRY INVERTED</div>
                            {/if}
                            <div class={styles.setupBody}>
                                <div class={styles.setupRow}>
                                    <span class={styles.setupRowLabel}>ENTRY</span>
                                    <span class={styles.setupRowValue}>
                                        {setup.entryMid > 0 ? fmtPxDecimal(setup.entryMid, markPrice) : '—'}
                                    </span>
                                </div>
                                <div class={styles.setupRow}>
                                    <span class={styles.setupRowLabel}>TP1</span>
                                    <span class={styles.setupRowValue}>{setup.tp1 > 0 ? fmtPxDecimal(setup.tp1, markPrice) : '—'}</span>
                                </div>
                                <div class={styles.setupRow}>
                                    <span class={styles.setupRowLabel}>SL</span>
                                    <span class="{styles.setupRowValue} {styles.setupRowStop}">{setup.invalidation > 0 ? fmtPxDecimal(setup.invalidation, markPrice) : '—'}</span>
                                </div>
                                <div class={styles.setupRow}>
                                    <span class={styles.setupRowLabel}>R:R</span>
                                    <span class={styles.setupRowValue}>
                                        {#if setup.rr != null}
                                            <span class={rrCls(setup.rr)}>{setup.rr.toFixed(2)}</span>
                                        {:else}
                                            —
                                        {/if}
                                    </span>
                                </div>
                                <div class={styles.setupRowMeta}>
                                    {setup.preconditions_met}/{setup.preconditions_total} preconditions met · score {fmtScore(setup.score)}
                                </div>
                            </div>
                        </div>
                    {/each}
                </div>
            {/if}
            {#if noClearProfile}
                <div class={styles.noClearStrip}>
                    <span class={styles.noClearBadge}>NO CLEAR OPPORTUNITY</span>
                    <span class={styles.noClearMeta}>{noClearProfile.preconditions_met}/{noClearProfile.preconditions_total} preconditions met · informational only</span>
                </div>
            {/if}
        </div>

        <div class={styles.section}>
            <div class={styles.sectionTitle}>R:R (Internal)</div>
            <div class={styles.zoneGrid}>
                <div class={styles.zoneCard}>
                    <span class={styles.zoneLabel}>Expected R:R</span>
                    <span class={rrInternalDisplay.isNA ? styles.rrValueNA : styles.rrValue}>
                        {rrInternalDisplay.value}
                    </span>
                </div>
                <div class={styles.zoneCard}>
                    <span class={styles.zoneLabel}>Horizon</span>
                    <span class={styles.zoneValue}>{opportunity?.time_horizon ?? '—'}</span>
                </div>
            </div>
        </div>

        <!-- ── Invalidation note ── -->
        <div class={styles.section}>
            <div class={styles.sectionTitle}>Invalidation Note</div>
            <div class={styles.noteBox}>
                {opportunity?.invalidation_note || 'Assessment conditions forming — invalidation level will be calculated when structural pivot confirms.'}
            </div>
        </div>

        <!-- ── Evaluated profiles ── -->
        <div class={styles.section}>
            <div class={styles.sectionTitle}>Evaluated Setups</div>
            {#if opportunity?.profiles && opportunity.profiles.length > 0}
                <div class={styles.profileList}>
                    {#each (opportunity?.profiles ?? []).filter((p) => p.opportunity_type !== 'NoClearOpportunity') as profile (profile.opportunity_type)}
                        <div class="{styles.profileCard} {oppClass(profile.opportunity_type)}">
                            <div class={styles.profileHeader}>
                                <span class={styles.profileType}>{oppLabel(profile.opportunity_type)}</span>
                                <span class={styles.profileScore} style="color: {scoreColor(profile.score)}">{fmtScore(profile.score)}</span>
                            </div>
                            <div class={styles.profilePreconditions}>
                                <span class={styles.profilePreLabel}>Preconditions</span>
                                <span class={styles.profilePreValue}>{profile.preconditions_met}/{profile.preconditions_total} met</span>
                                <div class={styles.profilePreBar}>
                                    <div class={styles.profilePreFill}
                                         style="width: {profile.preconditions_total > 0 ? (profile.preconditions_met / profile.preconditions_total * 100).toFixed(0) : '0'}%; background: {scoreColor(profile.score)}"></div>
                                </div>
                            </div>
                            {#if profile.trade_viability && profile.trade_viability !== 'NoClear'}
                                <div class={styles.profileViability}>{profile.trade_viability}</div>
                            {/if}
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

        <!-- ── Confluent Levels ── -->
        <div class={styles.section}>
            <div class={styles.sectionTitle}>Confluent Levels</div>
            {#if (opportunity?.confluent_entry_levels?.length ?? 0) > 0 || (opportunity?.confluent_target_levels?.length ?? 0) > 0}
                {#if (opportunity?.confluent_entry_levels?.length ?? 0) > 0}
                    <div class={styles.confluenceSubheader}>Entry</div>
                    {#each (opportunity?.confluent_entry_levels ?? []).slice(0, 4) as level}
                        <div class={styles.confluenceRow}>
                            <span class={styles.confluencePrice}>{fmtPx(level.price, markPrice)}</span>
                            <div class={styles.confluenceSources}>
                                {#each level.sources as src}
                                    <span class={styles.sourceTag} style="background: {sourceColor(src)}22; color: {sourceColor(src)}; border-color: {sourceColor(src)}44">
                                        {fmtSource(src)}
                                    </span>
                                {/each}
                            </div>
                            <span class={styles.confluenceStr} style="color: {scoreColor(level.strength)}">{fmtScore(level.strength)}%</span>
                        </div>
                    {/each}
                {/if}
                {#if (opportunity?.confluent_target_levels?.length ?? 0) > 0}
                    <div class={styles.confluenceSubheader}>Target</div>
                    {#each (opportunity?.confluent_target_levels ?? []).slice(0, 4) as level}
                        <div class={styles.confluenceRow}>
                            <span class={styles.confluencePrice}>{fmtPx(level.price, markPrice)}</span>
                            <div class={styles.confluenceSources}>
                                {#each level.sources as src}
                                    <span class={styles.sourceTag} style="background: {sourceColor(src)}22; color: {sourceColor(src)}; border-color: {sourceColor(src)}44">
                                        {fmtSource(src)}
                                    </span>
                                {/each}
                            </div>
                            <span class={styles.confluenceStr} style="color: {scoreColor(level.strength)}">{fmtScore(level.strength)}%</span>
                        </div>
                    {/each}
                {/if}
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
