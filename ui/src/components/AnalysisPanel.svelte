<script lang="ts">
    import type { AnalysisMatrix } from '../types';
    import { useAppStore } from '../state.svelte';
    import styles from './AnalysisPanel.module.css';

    const app = useAppStore();
    const instance = $derived(app.activeInstance());
    const analysis = $derived<AnalysisMatrix | null>(instance?.analysis ?? null);

    function biasClass(b: string): string {
        switch (b) {
            case 'StrongBullish': return styles.biasStrongBull;
            case 'Bullish': return styles.biasBull;
            case 'Neutral': return styles.biasNeutral;
            case 'Bearish': return styles.biasBear;
            case 'StrongBearish': return styles.biasStrongBear;
            default: return styles.biasNeutral;
        }
    }
    function regimeClass(r: string): string {
        if (r.includes('BULL')) return styles.regimeBull;
        if (r.includes('BEAR')) return styles.regimeBear;
        if (r === 'EXPANSION' || r === 'CONTRACTION') return styles.regimeVol;
        return styles.regimeNeutral;
    }
    function confClass(c: number): string {
        if (c >= 0.6) return styles.confHigh;
        if (c >= 0.35) return styles.confMid;
        return styles.confLow;
    }
    function qualityClass(q: string): string {
        switch (q) {
            case 'Excellent': return styles.qualityExc;
            case 'Good': return styles.qualityGood;
            case 'Average': return styles.qualityAvg;
            case 'Weak': return styles.qualityWeak;
            default: return styles.qualityPoor;
        }
    }
    function displayBias(b: string): string {
        return b.replace(/([A-Z])/g, ' $1').trim().toUpperCase();
    }
    function displayRegime(r: string): string {
        return r.replace(/_/g, ' ');
    }
</script>

<div class={styles.panel}>
    {#if !analysis || !analysis.timeframes_considered}
        <div class={styles.placeholder}>Awaiting market analysis data...</div>
    {:else}
        <h2 class={styles.title}>Market Analysis</h2>

        <div class={styles.section}>
            <div class={styles.biasRow}>
                <span class="{styles.biasBadge} {biasClass(analysis.bias)}">
                    {displayBias(analysis.bias)}
                </span>
                <div class={styles.confidenceMeter}>
                    <span class={styles.confLabel}>Confidence</span>
                    <div class={styles.confBar}>
                        <div class="{styles.confFill} {confClass(analysis.confidence)}"
                             style="width: {(analysis.confidence * 100).toFixed(1)}%"></div>
                    </div>
                    <span class={styles.confVal}>{(analysis.confidence * 100).toFixed(0)}%</span>
                </div>
            </div>
        </div>

        <div class={styles.section}>
            <div class={styles.regimeRow}>
                <span class={styles.sectionTitle}>Regime:</span>
                <span class="{styles.regimeBadge} {regimeClass(analysis.market_regime)}">
                    {displayRegime(analysis.market_regime)}
                </span>
                <span>| Quality: <span class="{styles.qualityBadge} {qualityClass(analysis.market_quality)}">{analysis.market_quality}</span></span>
            </div>
        </div>

        <div class={styles.section}>
            <div class={styles.sectionTitle}>Qualitative Assessment</div>
            <div class={styles.assessGrid}>
                <div class={styles.assessCard}>
                    <span class={styles.assessLabel}>Trend</span>
                    <span class={styles.assessValue}>{analysis.trend_assessment}</span>
                </div>
                <div class={styles.assessCard}>
                    <span class={styles.assessLabel}>Momentum</span>
                    <span class={styles.assessValue}>{analysis.momentum_assessment}</span>
                </div>
                <div class={styles.assessCard}>
                    <span class={styles.assessLabel}>Structure</span>
                    <span class={styles.assessValue}>{analysis.structure_assessment}</span>
                </div>
                <div class={styles.assessCard}>
                    <span class={styles.assessLabel}>Volatility</span>
                    <span class={styles.assessValue}>{analysis.volatility_assessment}</span>
                </div>
                <div class={styles.assessCard}>
                    <span class={styles.assessLabel}>Volume</span>
                    <span class={styles.assessValue}>{analysis.volume_assessment}</span>
                </div>
                <div class={styles.assessCard}>
                    <span class={styles.assessLabel}>Timeframes</span>
                    <span class={styles.assessValue}>{analysis.timeframes_considered}/4</span>
                </div>
            </div>
        </div>

        {#if analysis.market_interpretation}
            <div class={styles.section}>
                <div class={styles.sectionTitle}>Interpretation</div>
                <div class={styles.interpretation}>{analysis.market_interpretation}</div>
            </div>
        {/if}

        {#if analysis.rationale}
            <div class={styles.rationale}>{analysis.rationale}</div>
        {/if}

        <div class={styles.section}>
            <div class={styles.sectionTitle}>Signals</div>
            <div class={styles.signalList}>
                {#each analysis.supporting_signals as s}
                    <span class={styles.sigSupport}>+ {s}</span>
                {/each}
                {#each analysis.contradicting_signals as c}
                    <span class={styles.sigContra}>− {c}</span>
                {/each}
            </div>
        </div>
    {/if}
</div>