<script lang="ts">
    import type { AdvisoryMatrix, DecisionContext, MarketSnapshot, OpportunityMatrix, TimeframeTelemetry } from '../types';
    import { useAppStore } from '../state.svelte';
    import { buildPanelExportJson } from '../lib/metricsExport';
    import ExportDataButton from './ExportDataButton.svelte';
    import styles from './AdvisoryPanel.module.css';
    import { deriveTradePlan } from '../lib/tradePlan';
    import { computeDecisionRank } from '../lib/decisionRank';

    const app = useAppStore();
    let { pairKey } = $props<{ pairKey: string }>();

    const instance = $derived(app.instancesMap[pairKey]);
    const advisory = $derived<AdvisoryMatrix | null>(instance?.advisory ?? null);

    const microTerm = $derived<TimeframeTelemetry | undefined>(instance?.microTerm);
    const snapshot = $derived(instance?.microTerm.latestSnapshot as unknown as MarketSnapshot | undefined);
    const decisionCtx = $derived<DecisionContext | null>(snapshot?.decision_context ?? null);
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

    function buildExport() {
        return buildPanelExportJson({
            sourceTab: 'decision',
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
                analysis: instance?.analysis ?? null,
                risk: instance?.risk ?? null,
                alignment: (instance?.alignment as unknown as Record<string, unknown>) ?? null,
                opportunity,
                advisory,
                volumeProfile: (microTerm as any)?.volumeProfile ?? null,
                liquidity: (microTerm as any)?.liquidity ?? null,
                cluster: (microTerm as any)?.cluster ?? null,
                liquiditySignals: ((microTerm as any)?.liquiditySignals ?? []) as any[],
                decisionContext: (decisionCtx as unknown as Record<string, unknown>) ?? null,
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
    const dangerDisplay = $derived(decisionCtx?.entry_danger ?? 50);
    const confidenceDisplay = $derived(advisory?.confidence_assessment ?? 0);
    const stopLossPct = $derived((advisory as any)?.stop_loss_distance_pct ?? 0);

    // ── Hero direction-class mapping ──────────────────────────────────────
    function verdictClass(action: 'LONG' | 'SHORT' | 'HOLD'): string {
        if (action === 'LONG') return styles.verdictLong ?? '';
        if (action === 'SHORT') return styles.verdictShort ?? '';
        return styles.verdictHold ?? '';
    }
    function rankBarClass(action: 'LONG' | 'SHORT' | 'HOLD'): string {
        if (action === 'LONG') return styles.rankLong ?? '';
        if (action === 'SHORT') return styles.rankShort ?? '';
        return styles.rankHold ?? '';
    }
</script>

<div class={styles.panel}>
    <div class={styles.panelHeader}>
        <h2 class={styles.title}>Decision Guidance</h2>
        <ExportDataButton onExport={buildExport} title="Copy all Decision data as JSON" />
    </div>

    {#if !advisory}
        <div class={styles.noData}>Awaiting decision guidance data — all values will populate once L6 synthesis runs</div>
    {/if}

    <!-- ── Direction-coded verdict (LONG=green, SHORT=red, HOLD=amber) ─────── -->
    <div class="{styles.verdict} {verdictClass(rank.top)}">
        <div class={styles.verdictLabel}>RECOMMENDATION</div>
        <div class={styles.verdictRow}>
            <div class={styles.verdictAction}>{rank.top}</div>
            <div class={styles.verdictPct}>{rank.top_prob}%</div>
        </div>
        <div class={styles.verdictMeta}>
            Confidence {rank.headline.confidence_pct}% · {rank.headline.state}
        </div>
    </div>

    <!-- ── Runner-ups (winner excluded, dispersion at a glance) ─────────────── -->
    <div class={styles.runnerRow}>
        {#each runners as r (r.action)}
            <div class="{styles.runnerCell} {rankBarClass(r.action)}">
                <span class={styles.runnerAction}>{r.action}</span>
                <span class={styles.runnerPct}>{r.prob}%</span>
            </div>
        {/each}
    </div>

    <!-- ── Why (top-3 rationale) ────────────────────────────────────────────── -->
    <div class={styles.section}>
        <div class={styles.sectionTitle}>Why</div>
        <ul class={styles.why}>
            {#each rank.rationale.slice(0, 3) as line, i (i)}
                <li class={styles.whyItem}>{line}</li>
            {/each}
        </ul>
    </div>

    <!-- ── KPI strip (compact 4-cell) ──────────────────────────────────────── -->
    <div class={styles.section}>
        <div class={styles.sectionTitle}>The Numbers</div>
        <div class={styles.kpiStrip}>
            <div class={styles.kpi}>
                <span class={styles.kpiLabel}>Confidence</span>
                <span class={styles.kpiVal} style="color: {confidenceDisplay >= 60 ? '#22c55e' : confidenceDisplay >= 30 ? '#f59e0b' : '#ef4444'}">
                    {confidenceDisplay.toFixed(0)}%
                </span>
            </div>
            <div class={styles.kpi}>
                <span class={styles.kpiLabel}>R:R</span>
                <span class={styles.kpiVal} style="color: {rrDisplay >= 2 ? '#22c55e' : rrDisplay >= 1 ? '#f59e0b' : '#ef4444'}">
                    {rrDisplay.toFixed(2)}
                </span>
            </div>
            <div class={styles.kpi}>
                <span class={styles.kpiLabel}>Entry Danger</span>
                <span class={styles.kpiVal} style="color: {dangerDisplay >= 70 ? '#ef4444' : dangerDisplay >= 40 ? '#f59e0b' : '#22c55e'}">
                    {dangerDisplay.toFixed(0)}
                </span>
            </div>
            <div class={styles.kpi}>
                <span class={styles.kpiLabel}>Stop-Loss</span>
                <span class={styles.kpiVal}>
                    {stopLossPct > 0 ? `${(stopLossPct * 100).toFixed(2)}%` : '—'}
                </span>
            </div>
        </div>
    </div>

    <!-- ── Price Levels ───────────────────────────────────────────────────── -->
    <div class={styles.section}>
        <div class={styles.sectionTitle}>Price Levels</div>
        <div class={styles.grid2}>
            <div class={styles.card}>
                <span class={styles.cardLabel}>Entry Zone</span>
                <span class={styles.cardValue}>
                    {opportunity ? `${opportunity.entry_zone.low.toFixed(0)} – ${opportunity.entry_zone.high.toFixed(0)}` : '—'}
                </span>
            </div>
            <div class={styles.card}>
                <span class={styles.cardLabel}>Target Zone</span>
                <span class={styles.cardValue}>
                    {opportunity ? `${opportunity.target_zone.low.toFixed(0)} – ${opportunity.target_zone.high.toFixed(0)}` : '—'}
                </span>
            </div>
            <div class={styles.card}>
                <span class={styles.cardLabel}>Invalidation</span>
                <span class={styles.cardValue}>
                    {opportunity ? opportunity.invalidation_level.toFixed(0) : '—'}
                </span>
            </div>
            <div class={styles.card}>
                <span class={styles.cardLabel}>Horizon</span>
                <span class={styles.cardValue}>{opportunity?.time_horizon ?? '—'}</span>
            </div>
        </div>
    </div>

    <!-- ── Strategy ───────────────────────────────────────────────────────── -->
    <div class={styles.section}>
        <div class={styles.sectionTitle}>Strategy</div>
        <div class={styles.grid2}>
            <div class={styles.card}>
                <span class={styles.cardLabel}>Environment</span>
                <span class={styles.cardValue}>{sanitizeLabel(advisory?.strategy_environment ?? '')}</span>
            </div>
            <div class={styles.card}>
                <span class={styles.cardLabel}>Opportunity</span>
                <span class={styles.cardValue}>{sanitizeLabel(advisory?.opportunity_classification ?? '')}</span>
            </div>
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

    <!-- ── Final Verdict (final_recommendation) ────────────────────────────── -->
    <div class={styles.section}>
        <div class={styles.sectionTitle}>Final Verdict</div>
        <blockquote class={styles.verdictQuote}>{advisory?.final_recommendation || '—'}</blockquote>
    </div>
</div>
