<script lang="ts">
    import type { AlignmentMatrix, AlignmentDimension, TfAlignmentInfo, TimeframeTelemetry } from '../types';
    import { useAppStore } from '../state.svelte';
    import type { WsState } from '../lib/websocket.svelte';
    import { buildAlignmentTabExport } from '../lib/exportBuilders/alignmentTab';
    import ExportDataButton from './ExportDataButton.svelte';
    import LayerHeader from './LayerHeader.svelte';
    import { buildL2AlignmentHeader, mLabel, type LayerHeaderSpec } from '../lib/layerHeader';
    import { regimeTone } from '../lib/dashboardColors';
    import styles from './AlignmentPanel.module.css';

    const app = useAppStore();
    let { pairKey, wssState }: { pairKey: string; wssState?: WsState } = $props();
    const instance = $derived(app.instancesMap[pairKey]);
    const alignment = $derived<AlignmentMatrix | null>(instance?.alignment ?? null);
    const microTerm = $derived<TimeframeTelemetry | undefined>(instance?.microTerm);
    const microSnap = $derived(microTerm?.latestSnapshot as Record<string, unknown> | undefined);
    const markPrice = $derived(parseFloat(microTerm?.priceText ?? '0') || 0);
    const timestamp = $derived<number | null>(
        microSnap && typeof (microSnap as any).timestamp === 'number'
            ? (microSnap as any).timestamp
            : null
    );

    function buildExport() {
        return buildAlignmentTabExport({
            alignment,
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

    // AL-2 (v6.10.10): the wire's AlignState can be STRONG_BULLISH /
    // STRONG_BEARISH / MIXED / NO_DATA — the color helpers previously
    // only matched BULLISH/Bearish, so strongly-aligned dimensions
    // rendered as neutral gray cards.
    function dimFillClass(score: number, state: string): string {
        if (score >= 100) return styles.dimFillConfluent;
        const s = String(state || '').toUpperCase().replace(/_/g, '');
        if (s === 'STRONGBULLISH' || s === 'BULLISH') return styles.dimFillBull;
        if (s === 'STRONGBEARISH' || s === 'BEARISH') return styles.dimFillBear;
        return styles.dimFillNeutral;
    }
    function stateClass(state: string): string {
        const s = String(state || '').toUpperCase().replace(/_/g, '');
        if (s === 'STRONGBULLISH' || s === 'BULLISH') return styles.stateBullish;
        if (s === 'STRONGBEARISH' || s === 'BEARISH') return styles.stateBearish;
        return styles.stateNeutral;
    }
    function shortStateLabel(state: string): string {
        const s = state.toUpperCase();
        if (s === 'STRONG_BULLISH') return 'STRONG';
        if (s === 'STRONG_BEARISH') return 'STRONG';
        if (s === 'NODATA' || s === 'NO_DATA') return 'NO DATA';
        return s;
    }
    function scoreClass(s: number): string {
        if (s > 5) return styles.bullish;
        if (s < -5) return styles.bearish;
        return styles.neutral;
    }
    function mLabelClass(l: string): string {
        if (l.includes('BULL')) return styles.mtfLabelBullish;
        if (l.includes('BEAR')) return styles.mtfLabelBearish;
        return styles.mtfLabelNeutral;
    }
    function tfDirectionCls(score: number): string {
        if (score > 0) return styles.bullValue;
        if (score < 0) return styles.bearValue;
        return '';
    }
    function tfRegimeCls(r: string): string {
        // M-6 (v6.10.11): tone classification via the shared `regimeTone`
        // — this panel colors direction (bull/bear) and volatile regimes.
        const tone = regimeTone(r);
        if (tone === 'bull') return styles.tfRegimeBull;
        if (tone === 'bear') return styles.tfRegimeBear;
        if (tone === 'vol') return styles.tfRegimeVol;
        return styles.tfRegimeNeutral;
    }

    const SLOT_RANK: Record<string, number> = { MICRO: 0, FAST: 1, SLOW: 2, MACRO: 3 };
    const sortedTfAlignments = $derived(
        (alignment?.timeframe_alignments ?? [])
            .slice()
            .sort((a, b) => (SLOT_RANK[a.timeframe] ?? 99) - (SLOT_RANK[b.timeframe] ?? 99))
    );

    const dimNames = ['Trend', 'Momentum', 'Volume', 'Volatility', 'Structure', 'Signal', 'Regime', 'Confidence', 'Liquidity', 'Tradability'];

    // v6.10.17 (P1): the weight chips and formula read the WIRE
    // `blend_weights` — the backend publishes the ACTUAL blend (including
    // the thin-participation reweight, e.g. 55/35/5/5) so the panel can
    // never show a stale 50/30/10/10 beside a score the backend blended
    // differently. Legacy payloads fall back to the canonical 50/30/10/10.
    const weights = $derived.by(() => {
        const wire = alignment?.blend_weights ?? [];
        const chip = (key: string, pct: number): { label: string; key: string; pct: number; color: string } => ({
            key,
            pct: Math.round(pct * 100),
            color: key === 'T' ? '#22c55e' : key === 'M' ? '#3b82f6' : key === 'Vt' ? '#a78bfa' : '#f59e0b',
            label: key === 'T' ? 'Trend' : key === 'M' ? 'Momentum' : key === 'Vt' ? 'Volume' : 'Volatility',
        });
        if (wire.length === 4) return wire.map(([key, pct]) => chip(key, pct));
        return [
            { label: 'Trend', key: 'T', pct: 50, color: '#22c55e' },
            { label: 'Momentum', key: 'M', pct: 30, color: '#3b82f6' },
            { label: 'Volume', key: 'Vt', pct: 10, color: '#a78bfa' },
            { label: 'Volatility', key: 'Vm', pct: 10, color: '#f59e0b' },
        ];
    });

    function getRawValue(key: string): number {
        if (!alignment) return 0;
        if (key === 'T') return alignment.mtf_trend_alignment;
        if (key === 'M') return alignment.mtf_momentum_alignment;
        if (key === 'Vt') return alignment.mtf_volume_alignment;
        if (key === 'Vm') return alignment.mtf_volatility_alignment;
        return 0;
    }

    const blendDesc = $derived.by(() => {
        if (!alignment) return '';
        // AL-1 (v6.10.10): the backend scales the blend by ×100
        // (`mtf_overall_score = 100·Σ(wᵢ·axisᵢ)`, signed axes [−1, 1]) —
        // the displayed equation must carry the factor and the ACTUAL
        // weights (v6.10.17) or it never balances against the score.
        const parts = weights.map((w) => `${(w.pct / 100).toFixed(1)} * (${getRawValue(w.key).toFixed(2)})`);
        return `(${parts.join(' + ')}) \u00D7 100 = ${alignment.mtf_overall_score.toFixed(1)}`;
    });

    // AL-7 (v6.10.10): the backend's warmup sentinel (`AlignmentMatrix::empty` —
    // timeframes_present 0, label NO_DATA, agreement 0.0) must NOT drive the
    // consensus row and interpretation into a fabricated "Conflict" verdict.
    // The dimension cards keep their honest NO DATA states; only the
    // consensus/interpretation surfaces treat the sentinel as awaiting.
    const hasAlignment = $derived(!!alignment && alignment.timeframes_present > 0);

    const conflictWarning = $derived(
        hasAlignment && alignment!.trend_agreement_pct < 50
    );

    const headerSpec = $derived<LayerHeaderSpec>(buildL2AlignmentHeader(alignment));
</script>

<div class={styles.panel}>
    <!-- v7.0-prod: the panel-level banner above the LayerHeader was removed
         (D9 — no text above any badge). Per-section empty states still
         surface from within the body when a matrix hasn't loaded yet. -->
    <LayerHeader spec={headerSpec}>
        {#snippet trailing()}
            <h2 class={styles.title}>Cross-Timeframe Alignment</h2>
            <ExportDataButton onExport={buildExport} title="Copy all Alignment data as JSON" />
        {/snippet}
    </LayerHeader>

    <!-- ── Alignment Breakdown — card grid ── -->
    <div class={styles.section}>
        <div class={styles.sectionTitle}>
            Alignment Breakdown
            <span class={styles.sectionMeta}>
                T:{alignment ? alignment.mtf_trend_alignment.toFixed(2) : '\u2014'}
                {' '}M:{alignment ? alignment.mtf_momentum_alignment.toFixed(2) : '\u2014'}
                {' '}Vt:{alignment ? alignment.mtf_volume_alignment.toFixed(2) : '\u2014'}
                {' '}Vm:{alignment ? alignment.mtf_volatility_alignment.toFixed(2) : '\u2014'}
            </span>
        </div>
        {#if alignment?.dimensions?.length}
            <div class={styles.dimGrid}>
                {#each alignment.dimensions as dim, i}
                    {@const name = dimNames[i] ?? `Dim ${i}`}
                    <div class={styles.dimCard}>
                        <div class={styles.dimCardHead}>
                            <span class={styles.dimCardName}>{name}</span>
                            <span class="{styles.dimCardState} {stateClass(dim.state)}">
                                {shortStateLabel(dim.state)}
                            </span>
                        </div>
                        <div class={styles.dimCardBarRow}>
                            <div class={styles.dimCardBar}>
                                <div class="{styles.dimCardFill} {dimFillClass(dim.score, dim.state)}"
                                     style="width: {Math.min(Math.abs(dim.score), 100).toFixed(1)}%"></div>
                            </div>
                            <span class={styles.dimCardScore}>{dim.score.toFixed(0)}</span>
                        </div>
                        <div class={styles.dimCardConf}>
                            confidence {dim.confidence.toFixed(0)}%
                        </div>
                    </div>
                {/each}
            </div>
        {:else}
            <div class={styles.emptyGridNote}>Dimension scores computing — each axis will show a color-coded card with trend, momentum, volume, volatility, structure, signal, regime, confidence, liquidity, and tradability readings.</div>
        {/if}
    </div>

    <!-- ── Consensus ── -->
    <div class={styles.section}>
        <div class={styles.sectionTitle}>Timeframe Consensus</div>
        <div class={styles.consensusRow}>
            <div class={styles.consensusMeter}>
                <span class={styles.consensusVal}>{alignment && alignment.timeframes_present > 0 ? alignment.trend_agreement_pct.toFixed(0) : '\u2014'}%</span>
                <div class={styles.consensusBar}>
                    <div class="{styles.consensusFill} {alignment && alignment.timeframes_present > 0 ? (alignment.trend_agreement_pct >= 75 ? styles.consensusStrong : alignment.trend_agreement_pct >= 50 ? styles.consensusPartial : styles.consensusConflict) : ''}"
                         style="width: {alignment && alignment.timeframes_present > 0 ? alignment.trend_agreement_pct.toFixed(1) : '0'}%"></div>
                </div>
            </div>
            <span class={styles.consensusText}>
                {alignment && alignment.timeframes_present > 0
                    ? (alignment.trend_agreement_pct >= 75 ? 'Strong consensus — timeframes aligned' :
                       alignment.trend_agreement_pct >= 50 ? 'Partial consensus — mixed signals' :
                       // AL-3: "conflict" overstates when the low agreement
                       // comes from undecided (neutral) TFs, not opposition.
                       'Mixed consensus — timeframes not aligned')
                    : '\u2014'}
            </span>
        </div>
        <!-- ── Polarization — sign-coded axis values used in the blend -->
        <div class={styles.polarization}>
            <span class={styles.polarLabel}>Polarization</span>
            {#each [
                { key: 'T', label: 'Trend', v: alignment?.mtf_trend_alignment ?? 0 },
                { key: 'M', label: 'Momentum', v: alignment?.mtf_momentum_alignment ?? 0 },
                { key: 'Vt', label: 'Volume', v: alignment?.mtf_volume_alignment ?? 0 },
                { key: 'Vm', label: 'Volatility', v: alignment?.mtf_volatility_alignment ?? 0 },
            ] as axis (axis.key)}
                <span class={styles.polarChip}
                      style="border-color: {axis.v > 0.05 ? 'rgba(34, 197, 94, 0.4)' : axis.v < -0.05 ? 'rgba(239, 68, 68, 0.4)' : 'rgba(148, 163, 184, 0.4)'}; color: {axis.v > 0.05 ? '#22c55e' : axis.v < -0.05 ? '#ef4444' : '#94a3b8'}">
                    <span class={styles.polarKey}>{axis.key}</span>
                    <span class={styles.polarVal}>{(axis.v >= 0 ? '+' : '') + axis.v.toFixed(2)}</span>
                </span>
            {/each}
        </div>
        {#if conflictWarning}
            <div class={styles.conflictBadge}>TIMEFRAME MISALIGNMENT — time horizons are not working together</div>
        {/if}
    </div>

    <!-- ── Per-Timeframe cards ── -->
    <div class={styles.section}>
        <div class={styles.sectionTitle}>Per-Timeframe Snapshot</div>
        {#if sortedTfAlignments.length > 0}
            <div class={styles.tfCards}>
                {#each sortedTfAlignments as tf (tf.timeframe)}
                    <div class={styles.tfCard}>
                        <header class={styles.tfCardHead}>
                            <span class={styles.tfCardName}>{tf.timeframe}</span>
                            <span class={styles.tfCardSig}>{tf.active_signals} signals</span>
                        </header>
                        <div class={styles.tfCardChips}>
                            <span class="{styles.tfChip} {tfDirectionCls(tf.trend_score)}">
                                Trend {tf.trend_score.toFixed(2)}
                            </span>
                            <span class="{styles.tfChip} {tfDirectionCls(tf.momentum_score)}">
                                Mom {tf.momentum_score.toFixed(2)}
                            </span>
                            <span class="{styles.tfChip} {tfDirectionCls(tf.overall_score)}">
                                Ov {tf.overall_score.toFixed(1)}
                            </span>
                            <span class="{styles.tfChip} {tfRegimeCls(tf.regime)}">
                                {tf.regime}
                            </span>
                        </div>
                    </div>
                {/each}
            </div>
        {:else}
            <div class={styles.emptyGridNote}>Per-timeframe readings will appear once individual candle intervals complete — showing trend, momentum, overall, and regime for each of the 4 time horizons.</div>
        {/if}
    </div>

    <!-- ── Weight formula ── -->
    <div class={styles.section}>
        <div class={styles.sectionTitle}>Score Calculation</div>
        <div class={styles.weightGrid}>
            {#each weights as w}
                {@const val = getRawValue(w.key)}
                {@const contrib = val * (w.pct / 100)}
                <div class={styles.weightChip}>
                    <span class={styles.weightChipKey}>{w.key}</span>
                    <span class={styles.weightChipLabel}>
                        {w.label} <span style="color: var(--text-dim); font-size: 8px;">({w.pct}%)</span>
                    </span>
                    <span class={styles.weightChipPct} style="color: {w.color}">
                        {alignment ? (val >= 0 ? '+' : '') + val.toFixed(2) : '—'}
                    </span>
                    <span style="font-size: 9px; color: var(--text-dim); font-family: var(--mono); margin-top: 2px;">
                        contrib: {alignment ? (contrib >= 0 ? '+' : '') + contrib.toFixed(2) : '—'}
                    </span>
                </div>
            {/each}
        </div>
        <div class={styles.formula}>{blendDesc || '—'}</div>
    </div>

    <!-- ── Interpretation ── -->
    <div class={styles.section}>
        <div class={styles.sectionTitle}>Interpretation</div>
        <div class={styles.interpretation}>
            {#if alignment && alignment.timeframes_present > 0}
                {#if alignment.trend_agreement_pct >= 75}
                    Multi-timeframe alignment shows <strong>strong directional consensus</strong> ({alignment.trend_agreement_pct.toFixed(0)}% agreement across {alignment.timeframes_present}/4 timeframes).
                    The composite score of {alignment.mtf_overall_score.toFixed(1)} is classified as <strong>{mLabel(alignment.mtf_overall_label).toUpperCase()}</strong>.
                    {#if alignment.signal_cross_tf_count > 0}
                        {mLabel(alignment.mtf_overall_label).toUpperCase() !== 'NEUTRAL'
                            ? `${alignment.signal_cross_tf_count} cross-timeframe signal votes reinforce the current bias.`
                            : `${alignment.signal_cross_tf_count} cross-timeframe signal votes detected across the aligned timeframes.`}
                    {:else}
                        No cross-timeframe signal votes detected.
                    {/if}
                {:else if alignment.trend_agreement_pct >= 50}
                    Alignment shows <strong>partial consensus</strong> ({alignment.trend_agreement_pct.toFixed(0)}% agreement).
                    The composite score of {alignment.mtf_overall_score.toFixed(1)} reflects <strong>{mLabel(alignment.mtf_overall_label).toUpperCase()}</strong> conditions with mixed input from {alignment.timeframes_present} timeframes.
                {:else}
                    Timeframes are in <strong>conflict</strong> ({alignment.trend_agreement_pct.toFixed(0)}% agreement).
                    Exercise caution — different time horizons are pulling in opposite directions. Wait for re-alignment before committing to directional bias.
                {/if}
            {:else}
                Awaiting alignment data — this section will synthesize a human-readable interpretation of multi-timeframe consensus once indicators populate.
            {/if}
        </div>
    </div>
</div>
