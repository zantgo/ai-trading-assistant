<script lang="ts">
    import type { AlignmentMatrix, AlignmentDimension, TfAlignmentInfo } from '../types';
    import { useAppStore } from '../state.svelte';
    import styles from './AlignmentPanel.module.css';

    const app = useAppStore();
    let { pairKey } = $props<{ pairKey: string }>();

    const instance = $derived(app.instancesMap[pairKey]);
    const alignment = $derived<AlignmentMatrix | null>(instance?.alignment ?? null);

    function dimColor(state: string): string {
        if (state === 'BULLISH' || state === 'Bullish') return styles.dimFillBull;
        if (state === 'BEARISH' || state === 'Bearish') return styles.dimFillBear;
        return styles.dimFillNeutral;
    }
    function stateClass(state: string): string {
        if (state === 'BULLISH' || state === 'Bullish') return styles.stateBullish;
        if (state === 'BEARISH' || state === 'Bearish') return styles.stateBearish;
        return styles.stateNeutral;
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
    function scoreClass(s: number): string {
        if (s > 5) return styles.bullish;
        if (s < -5) return styles.bearish;
        return styles.neutral;
    }

    const dimNames = ['Trend', 'Momentum', 'Volume', 'Volatility', 'Structure', 'Signal', 'Regime', 'Confidence', 'Liquidity', 'Tradability'];

    const blendDesc = $derived(
        alignment
            ? `0.5 T + 0.3 M + 0.1 Vt + 0.1 Vm`
            : '');
</script>

<div class={styles.panel}>
    {#if !alignment || !alignment.timeframes_present}
        <div class={styles.placeholder}>
            Awaiting multi-timeframe alignment data...
        </div>
    {:else}
        <h2 class={styles.title}>Cross-Timeframe Alignment</h2>

        <div class={styles.section}>
            <div class={styles.mtfScore}>
                <span class="{styles.scoreValue} {scoreClass(alignment.mtf_overall_score)}">
                    {alignment.mtf_overall_score.toFixed(1)}
                </span>
                <span class="{styles.mtfLabel} {mLabelClass(alignment.mtf_overall_label)}">
                    {mLabel(alignment.mtf_overall_label)}
                </span>
            </div>
            <div class={styles.flags}>
                <span class={styles.flag}>{alignment.timeframes_present}/4 timeframes</span>
                <span class={styles.flag}>{alignment.signal_cross_tf_count} cross-TF signals</span>
                <span class={styles.flag}>{alignment.trend_agreement_pct.toFixed(0)}% agreement</span>
            </div>
            <div class={styles.formula}>{blendDesc}</div>
        </div>

        <div class={styles.section}>
            <div class={styles.sectionTitle}>
                Alignment Breakdown ({' '}
                T: {alignment.mtf_trend_alignment.toFixed(2)}
                {' '}M: {alignment.mtf_momentum_alignment.toFixed(2)}
                {' '}Vt: {alignment.mtf_volume_alignment.toFixed(2)}
                {' '}Vm: {alignment.mtf_volatility_alignment.toFixed(2)})
            </div>
            <div class={styles.dimList}>
                {#each alignment.dimensions as dim, i}
                    {@const name = dimNames[i] ?? `Dim ${i}`}
                    <div class={styles.dimRow}>
                        <span class={styles.dimName}>{name}</span>
                        <div class={styles.dimBar}>
                            <div class="{styles.dimFill} {dimColor(dim.state)}"
                                 style="width: {dim.score.toFixed(1)}%"></div>
                        </div>
                        <span class={styles.dimScore}>{dim.score.toFixed(0)}</span>
                        <span class="{styles.dimState} {stateClass(dim.state)}">{dim.state}</span>
                    </div>
                {/each}
            </div>
        </div>

        <div class={styles.section}>
            <div class={styles.sectionTitle}>Timeframe Consensus</div>
            <div class={styles.agreementRow}>
                <span class={styles.agreementLabel}>Agreement</span>
                <span class={styles.agreementValue}>{alignment.trend_agreement_pct.toFixed(0)}%</span>
                <span class={styles.agreementPct}>
                    ({alignment.trend_agreement_pct >= 75 ? 'Strong consensus' :
                      alignment.trend_agreement_pct >= 50 ? 'Partial consensus' : 'Conflict'})
                </span>
            </div>
        </div>

        {#if alignment.timeframe_alignments.length > 0}
            <div class={styles.section}>
                <div class={styles.sectionTitle}>Per-Timeframe</div>
                <table class={styles.tfTable}>
                    <thead>
                        <tr>
                            <th>TF</th><th>Trend</th><th>Momentum</th>
                            <th>Overall</th><th>Regime</th><th>Signals</th>
                        </tr>
                    </thead>
                    <tbody>
                        {#each alignment.timeframe_alignments as tf}
                            <tr>
                                <td class={styles.tfName}>{tf.timeframe}</td>
                                <td class={tf.trend_score > 0 ? styles.tfBull : styles.tfBear}>
                                    {tf.trend_score.toFixed(3)}
                                </td>
                                <td class={tf.momentum_score > 0 ? styles.tfBull : styles.tfBear}>
                                    {tf.momentum_score.toFixed(3)}
                                </td>
                                <td class={tf.overall_score > 0 ? styles.tfBull : tf.overall_score < 0 ? styles.tfBear : ''}>
                                    {tf.overall_score}
                                </td>
                                <td>{tf.regime}</td>
                                <td>{tf.active_signals}</td>
                            </tr>
                        {/each}
                    </tbody>
                </table>
            </div>
        {/if}
    {/if}
</div>