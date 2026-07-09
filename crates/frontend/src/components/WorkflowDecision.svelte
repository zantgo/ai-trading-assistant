<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import type { DecisionOutput, FactorBreakdown, IndicatorMap, MonitorTimeframe } from '../types';
    import ConfluenceHero from './state/ConfluenceHero.svelte';
    import DecisionScorecard from './state/DecisionScorecard.svelte';
    import type { DecisionContext } from '../types';
    import styles from './WorkflowDecision.module.css';

    const app = useAppStore();
    let { pairKey }: { pairKey: string } = $props();
    const pair = $derived(app.instancesMap[pairKey]);
    const micro = $derived(pair?.microTerm);
    const fast = $derived(pair?.fastTerm);
    const slow = $derived(pair?.slowTerm);
    const macro = $derived(pair?.macroTerm);

    // ── Current micro snapshot data ──
    const snap = $derived(micro?.latestSnapshot);
    const indicators = $derived((snap?.indicators ?? {}) as IndicatorMap);
    const dc = $derived(snap?.decision_context as DecisionContext | undefined);
    const statCtx = $derived(snap?.statistical_context as Record<string, any> | undefined);
    const price = $derived((snap?.current_price ?? 0) as number);

    // ── Monitor timeframe data (for ConfluenceHero) ──
    const monTf = $derived.by<MonitorTimeframe | null>(() => {
        // Build a minimal MonitorTimeframe from snapshot data
        if (!snap) return null;
        return {
            tf_name: 'micro',
            label: 'micro',
            timeframe_secs: 60,
            current_price: snap.current_price ?? 0,
            overall_score: 0,
            overall_label: '',
            confluence_normalized: 0,
            confluence_score: dc?.confluence ?? 0,
            regime: statCtx?.regime_label ?? '—',
            regime_gate: 1.0,
            active_weight: 1.0,
            contributions: [],
            opposite_score_long: 0,
            opposite_score_short: 0,
            opposite_exit_threshold: 60,
        } as MonitorTimeframe;
    });

    // ── Decision output ──
    let decisionOutput = $state<DecisionOutput | null>(null);
    let decisionLoading = $state(false);
    let decisionError = $state<string | null>(null);

    async function runDecision() {
        if (!snap || !dc) return;
        decisionLoading = true;
        decisionError = null;
        try {
            const positionDir = app.paper?.paperDirection ?? '';
            const positioned = positionDir === 'LONG' || positionDir === 'SHORT';
            const position = positioned ? positionDir : 'None';

            const body: Record<string, any> = {
                symbol: pair?.symbol ?? pairKey,
                position: position,
                confluence_score: dc.confluence ?? 0,
                opposite_score: 0,
                trade_readiness: dc.trade_readiness ?? 0,
                trade_quality: dc.trade_quality ?? 0,
                trend_persistence: dc.trend_persistence ?? 0,
                risk_level: dc.risk_level ?? 0,
                regime: statCtx?.regime_label ?? 'trending',
                regime_confidence: dc.regime_confidence ?? 0,
                breakout_confidence: statCtx?.breakout_confidence ?? 0,
                anomaly_score: statCtx?.anomaly_score ?? 0,
                compressed: statCtx?.compression_percentile ? statCtx.compression_percentile < 20 : false,
                choppy: false,
                confirmed_opposing_divergence: false,
                signal_age_bars: 0,
            };

            const res = await fetch('/api/decision', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(body),
            });
            if (!res.ok) {
                const text = await res.text();
                throw new Error(text || `HTTP ${res.status}`);
            }
            decisionOutput = await res.json() as DecisionOutput;
        } catch (e: any) {
            decisionError = e.message ?? String(e);
        } finally {
            decisionLoading = false;
        }
    }

    // Auto-run on mount and when snapshot changes
    let prevPrice = $state(0);
    $effect(() => {
        if (price > 0 && price !== prevPrice) {
            prevPrice = price;
            runDecision();
        }
    });

    function actionColor(a: string): string {
        switch (a) {
            case 'Open Long': return '#10b981';
            case 'Open Short': return '#ef4444';
            case 'Close': return '#ef4444';
            case 'Hold': return '#f59e0b';
            default: return '#94a3b8';
        }
    }

    function confColor(v: number): string {
        if (v >= 75) return '#10b981';
        if (v >= 50) return '#f59e0b';
        return '#ef4444';
    }

    const fb = $derived(decisionOutput?.factor_breakdown as FactorBreakdown | undefined);

    const bars: { label: string; value: number; weight: number }[] = $derived.by<{ label: string; value: number; weight: number }[]>(() => {
        if (!fb) return [];
        return [
            { label: 'Confluence', value: fb.confluence_norm, weight: 0.25 },
            { label: 'Readiness', value: fb.trade_readiness, weight: 0.20 },
            { label: 'Quality', value: fb.trade_quality, weight: 0.15 },
            { label: 'Safety', value: fb.safety_score, weight: 0.15 },
            { label: 'Trend', value: fb.trend_persistence, weight: 0.10 },
            { label: 'Regime C.', value: fb.regime_confidence, weight: 0.10 },
            { label: 'Breakout', value: fb.breakout_confidence, weight: 0.05 },
        ];
    });

    function barPct(v: number): string { return `${Math.round(v * 100)}%`; }
    function barColor(v: number): string {
        if (v >= 0.7) return '#10b981';
        if (v >= 0.4) return '#f59e0b';
        return '#ef4444';
    }
</script>

<div class={styles.container}>
    {#if decisionLoading && !decisionOutput}
        <div class={styles.loading}>Computing decision matrix...</div>
    {/if}

    {#if decisionError}
        <div class={styles.error}>Error: {decisionError}</div>
    {/if}

    {#if decisionOutput}
        <!-- Decision Output -->
        <div class={styles.decisionSection}>
            <div class={styles.actionRow}>
                <span class={styles.actionLabel}>ACTION</span>
                <span class={styles.actionValue} style="color:{actionColor(decisionOutput.action)};border-color:{actionColor(decisionOutput.action)}44;">
                    {decisionOutput.action}
                </span>
                <span class={styles.confLabel}>CONFIDENCE</span>
                <span class={styles.confValue} style="color:{confColor(decisionOutput.confidence)}">
                    {Math.round(decisionOutput.confidence)}%
                </span>
                <div class={styles.confTrack}>
                    <div class={styles.confFill} style="width:{Math.round(decisionOutput.confidence)}%;background:{confColor(decisionOutput.confidence)}"></div>
                </div>
            </div>

            <div class={styles.rationale}>
                <span class={styles.rationaleLabel}>RATIONALE</span>
                <p class={styles.rationaleText}>{decisionOutput.rationale}</p>
            </div>

            {#if decisionOutput.risk_notes}
                <div class={styles.riskNotes}>
                    <span class={styles.riskLabel}>RISK NOTES</span>
                    <p class={styles.riskText}>{decisionOutput.risk_notes}</p>
                </div>
            {/if}
        </div>

        <!-- Factor Breakdown -->
        {#if fb}
            <div class={styles.factorSection}>
                <div class={styles.factorTitle}>FACTOR BREAKDOWN</div>
                {#each bars as b}
                    <div class={styles.factorRow}>
                        <span class={styles.factorName}>{b.label}</span>
                        <span class={styles.factorWeight}>×{b.weight.toFixed(2)}</span>
                        <div class={styles.factorTrack}>
                            <div class={styles.factorFill} style="width:{barPct(b.value)};background:{barColor(b.value)}"></div>
                        </div>
                        <span class={styles.factorVal} style="color:{barColor(b.value)}">{Math.round(b.value * 100)}</span>
                    </div>
                {/each}
                <div class={styles.factorMeta}>
                    <span>Regime: {fb.regime} (×{fb.regime_multiplier.toFixed(2)})</span>
                    <span>Signal Decay: ×{fb.signal_decay.toFixed(2)}</span>
                    <span>Base: {fb.base_score.toFixed(2)} → Final: {fb.final_score.toFixed(2)}</span>
                </div>
            </div>

            <!-- Hard Gates -->
            <div class={styles.gatesSection}>
                <div class={styles.gatesTitle}>
                    HARD GATES — {fb.hard_gates_passed ? 'ALL PASSED' : 'FAILED'}
                </div>
                {#if fb.failing_gates.length > 0}
                    <div class={styles.failingGates}>
                        {#each fb.failing_gates as gate}
                            <span class={styles.failingGate}>{gate}</span>
                        {/each}
                    </div>
                {/if}
            </div>
        {/if}
    {/if}

    <!-- Confluence Hero -->
    {#if monTf}
        <div class={styles.confluenceSection}>
            <ConfluenceHero tf={monTf} topN={5} />
        </div>
    {/if}

    <!-- Decision Context -->
    {#if dc}
        <div class={styles.dcSection}>
            <DecisionScorecard {dc} />
        </div>
    {/if}
</div>
