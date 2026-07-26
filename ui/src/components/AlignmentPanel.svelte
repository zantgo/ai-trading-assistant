<script lang="ts">
    import type { AlignmentMatrix, AlignmentDimension, TfAlignmentInfo } from '../types';
    import { useAppStore } from '../state.svelte';
    import styles from './AlignmentPanel.module.css';

    const app = useAppStore();
    let { pairKey } = $props<{ pairKey: string }>();

    const instance = $derived(app.instancesMap[pairKey]);
    const alignment = $derived<AlignmentMatrix | null>(instance?.alignment ?? null);

    function dimFillClass(score: number, state: string): string {
        if (score >= 100) return styles.dimFillConfluent;
        if (state === 'BULLISH' || state === 'Bullish') return styles.dimFillBull;
        if (state === 'BEARISH' || state === 'Bearish') return styles.dimFillBear;
        return styles.dimFillNeutral;
    }
    function stateClass(state: string): string {
        if (state === 'BULLISH' || state === 'Bullish') return styles.stateBullish;
        if (state === 'BEARISH' || state === 'Bearish') return styles.stateBearish;
        return styles.stateNeutral;
    }
    function shortStateLabel(state: string): string {
        const s = state.toUpperCase();
        if (s === 'STRONG_BULLISH') return 'STRONG';
        if (s === 'STRONG_BEARISH') return 'STRONG';
        if (s === 'NODATA') return 'NO DATA';
        return s;
    }
    function scoreClass(s: number): string {
        if (s > 5) return styles.bullish;
        if (s < -5) return styles.bearish;
        return styles.neutral;
    }
    function mLabel(l: string): string {
        if (l.startsWith('STRONG_BULL')) return 'STRONG BULL';
        if (l.startsWith('STRONG_BEAR')) return 'STRONG BEAR';
        if (l.startsWith('WEAK_BULL')) return 'WEAK BULL';
        if (l.startsWith('WEAK_BEAR')) return 'WEAK BEAR';
        if (l === 'NEUTRAL_MTF') return 'NEUTRAL';
        return l;
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
        const u = r.toUpperCase();
        if (u.includes('BULL')) return styles.tfRegimeBull;
        if (u.includes('BEAR')) return styles.tfRegimeBear;
        if (u === 'TRANSITION' || u === 'CONTRACTION' || u === 'EXPANSION') return styles.tfRegimeVol;
        return styles.tfRegimeNeutral;
    }

    const SLOT_RANK: Record<string, number> = { MICRO: 0, FAST: 1, SLOW: 2, MACRO: 3 };
    const sortedTfAlignments = $derived(
        (alignment?.timeframe_alignments ?? [])
            .slice()
            .sort((a, b) => (SLOT_RANK[a.timeframe] ?? 99) - (SLOT_RANK[b.timeframe] ?? 99))
    );

    const dimNames = ['Trend', 'Momentum', 'Volume', 'Volatility', 'Structure', 'Signal', 'Regime', 'Confidence', 'Liquidity', 'Tradability'];

    const weights = [
        { label: 'Trend', key: 'T', pct: 50, color: '#22c55e' },
        { label: 'Momentum', key: 'M', pct: 30, color: '#3b82f6' },
        { label: 'Vol.trend', key: 'Vt', pct: 10, color: '#a78bfa' },
        { label: 'Vol.market', key: 'Vm', pct: 10, color: '#f59e0b' },
    ];

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
        const t = alignment.mtf_trend_alignment;
        const m = alignment.mtf_momentum_alignment;
        const vt = alignment.mtf_volume_alignment;
        const vm = alignment.mtf_volatility_alignment;
        const sum = alignment.mtf_overall_score;
        return `0.5 * (${t.toFixed(2)}) + 0.3 * (${m.toFixed(2)}) + 0.1 * (${vt.toFixed(2)}) + 0.1 * (${vm.toFixed(2)}) = ${sum.toFixed(1)}`;
    });

    const conflictWarning = $derived(
        alignment && alignment.timeframes_present > 0 && alignment.trend_agreement_pct < 50
    );
</script>

<div class={styles.panel}>
    <h2 class={styles.title}>Cross-Timeframe Alignment</h2>

    {#if !alignment || !alignment.timeframes_present}
        <div class={styles.noDataBanner}>Multi-timeframe alignment forming — all values will populate once candles complete across all timeframes.</div>
    {/if}

    <!-- ── Hero section ── -->
    <div class="{styles.hero} {alignment ? (alignment.mtf_overall_score > 5 ? styles.heroBull : alignment.mtf_overall_score < -5 ? styles.heroBear : styles.heroNeutral) : styles.heroNeutral}">
        <div class={styles.heroScoreBlock}>
            <span class="{styles.heroScore} {scoreClass(alignment?.mtf_overall_score ?? 0)}">
                {alignment ? alignment.mtf_overall_score.toFixed(1) : '\u2014'}
            </span>
            <span class="{styles.heroLabel} {mLabelClass(alignment?.mtf_overall_label ?? '')}">
                {alignment ? mLabel(alignment.mtf_overall_label) : 'AWAITING DATA'}
            </span>
        </div>
        <div class={styles.heroMeta}>
            <span class={styles.heroFlag}>{alignment?.timeframes_present ?? 0}/4 TFs</span>
            <span class={styles.heroFlag}>{alignment?.signal_cross_tf_count ?? 0} cross-TF</span>
            <span class="{styles.heroFlag} {conflictWarning ? styles.heroFlagWarn : ''}">
                {alignment ? alignment.trend_agreement_pct.toFixed(0) : '\u2014'}% agree
            </span>
        </div>
    </div>

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
                            confidence {(dim.confidence * 100).toFixed(0)}%
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
                <span class={styles.consensusVal}>{alignment ? alignment.trend_agreement_pct.toFixed(0) : '\u2014'}%</span>
                <div class={styles.consensusBar}>
                    <div class="{styles.consensusFill} {alignment ? (alignment.trend_agreement_pct >= 75 ? styles.consensusStrong : alignment.trend_agreement_pct >= 50 ? styles.consensusPartial : styles.consensusConflict) : ''}"
                         style="width: {alignment ? alignment.trend_agreement_pct.toFixed(1) : '0'}%"></div>
                </div>
            </div>
            <span class={styles.consensusText}>
                {alignment
                    ? (alignment.trend_agreement_pct >= 75 ? 'Strong consensus — timeframes aligned' :
                       alignment.trend_agreement_pct >= 50 ? 'Partial consensus — mixed signals' :
                       'Conflict — time horizons diverging')
                    : '\u2014'}
            </span>
        </div>
        {#if conflictWarning}
            <div class={styles.conflictBadge}>TIMEFRAME CONFLICT — time horizons are working against each other</div>
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
            {#if alignment}
                {#if alignment.trend_agreement_pct >= 75}
                    Multi-timeframe alignment shows <strong>strong directional consensus</strong> ({alignment.trend_agreement_pct.toFixed(0)}% agreement across {alignment.timeframes_present}/4 timeframes).
                    The composite score of {alignment.mtf_overall_score.toFixed(1)} is classified as <strong>{mLabel(alignment.mtf_overall_label).toUpperCase()}</strong>.
                    {alignment.signal_cross_tf_count > 0 ? `${alignment.signal_cross_tf_count} cross-timeframe signals reinforce the current bias.` : 'No cross-timeframe signals detected.'}
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
