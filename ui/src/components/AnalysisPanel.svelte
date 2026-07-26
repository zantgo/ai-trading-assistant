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
    function phaseClass(p: string): string {
        switch (p) {
            case 'MARKUP': return styles.phaseMarkup;
            case 'MARKDOWN': return styles.phaseMarkdown;
            case 'ACCUMULATION': return styles.phaseAccumulation;
            case 'DISTRIBUTION': return styles.phaseDistribution;
            default: return styles.phaseUnknown;
        }
    }
    function displayBias(b: string): string {
        return b.replace(/([A-Z])/g, ' $1').trim().toUpperCase();
    }
    function displayRegime(r: string): string {
        return r.replace(/_/g, ' ');
    }
    function displayPhase(p: string): string {
        return p.replace(/_/g, ' ').replace(/([A-Z])/g, ' $1').trim().toUpperCase();
    }

    /** Parse signal text for (bullish/bearish/neutral) direction indicator. */
    function signalDirClass(text: string): string {
        const dir = text.match(/\((bullish|bearish|neutral)\)/i)?.[1]?.toLowerCase();
        if (dir === 'bullish') return styles.sigBullish;
        if (dir === 'bearish') return styles.sigBearish;
        return styles.sigNeutral;
    }

    /** Directional arrow prefix based on signal's own direction. */
    function signalArrow(text: string): string {
        const dir = text.match(/\((bullish|bearish|neutral)\)/i)?.[1]?.toLowerCase();
        if (dir === 'bullish') return '\u25B2';
        if (dir === 'bearish') return '\u25BC';
        return '\u2022';
    }

    function highlightKeywords(text: string): string {
        if (!text) return '\u2014';
        const keywords = /\b(TRANSITIONAL|DEVELOPING|WEAKENING|UNSTABLE|WEAK|STRONG|HEALTHY|EXHAUSTED|EXPANDING|COMPRESSED|NORMAL|EXTREME|INCREASING|STABLE|REVERSING|BROKEN|EXCEPTIONAL|BULLISH|BEARISH|NEUTRAL)\b/gi;
        return text.replace(keywords, '<strong>$1</strong>');
    }
</script>

<div class={styles.panel}>
    <h2 class={styles.title}>Market Analysis</h2>

    {#if !analysis || !analysis.timeframes_considered}
        <div class={styles.noData}>Awaiting market analysis data — all values will populate once cross-TF consensus forms</div>
    {/if}

    <div class={styles.section}>
        <div class={styles.biasRow}>
            <span class="{styles.biasBadge} {biasClass(analysis?.bias ?? '')}">
                {analysis ? displayBias(analysis.bias) : '—'}
            </span>
            <div class={styles.confidenceMeter}>
                <span class={styles.confLabel}>Confidence</span>
                <div class={styles.confBar}>
                    <div class="{styles.confFill} {confClass(analysis?.confidence ?? 0)}"
                         style="width: {((analysis?.confidence ?? 0) * 100).toFixed(1)}%"></div>
                </div>
                <span class={styles.confVal}>{analysis ? (analysis.confidence * 100).toFixed(0) : '—'}%</span>
            </div>
        </div>
    </div>

    <div class={styles.section}>
        <div class={styles.regimeRow}>
            <span class={styles.sectionTitle}>Regime:</span>
            <span class="{styles.regimeBadge} {regimeClass(analysis?.market_regime ?? '')}">
                {analysis ? displayRegime(analysis.market_regime) : '—'}
            </span>
            <span>| Quality: <span class="{styles.qualityBadge} {qualityClass(analysis?.market_quality ?? '')}">{analysis?.market_quality ?? '—'}</span></span>
        </div>
    </div>

    <div class={styles.section}>
        <div class={styles.sectionTitle}>Qualitative Assessment</div>
        <div class={styles.assessGrid}>
            <div class={styles.assessCard}>
                <span class={styles.assessLabel}>Trend</span>
                <span class={styles.assessValue}>{analysis?.trend_assessment ?? '—'}</span>
            </div>
            <div class={styles.assessCard}>
                <span class={styles.assessLabel}>Momentum</span>
                <span class={styles.assessValue}>{analysis?.momentum_assessment ?? '—'}</span>
            </div>
            <div class={styles.assessCard}>
                <span class={styles.assessLabel}>Structure</span>
                <span class={styles.assessValue}>{analysis?.structure_assessment ?? '—'}</span>
            </div>
            <div class={styles.assessCard}>
                <span class={styles.assessLabel}>Volatility</span>
                <span class={styles.assessValue}>{analysis?.volatility_assessment ?? '—'}</span>
            </div>
            <div class={styles.assessCard}>
                <span class={styles.assessLabel}>Volume</span>
                <span class={styles.assessValue}>{analysis?.volume_assessment ?? '—'}</span>
            </div>
            <div class={styles.assessCard}>
                <span class={styles.assessLabel}>Cycle Phase</span>
                <span class="{styles.phaseValue} {phaseClass(analysis?.market_phase ?? 'UNKNOWN')}">
                    {analysis ? displayPhase(analysis.market_phase) : '—'}
                </span>
            </div>
            <div class={styles.assessCard}>
                <span class={styles.assessLabel}>Timeframes</span>
                <span class={styles.assessValue}>{analysis?.timeframes_considered ?? 0}/4</span>
            </div>
        </div>
    </div>

    <div class={styles.section}>
        <div class={styles.sectionTitle}>Interpretation</div>
        <div class={styles.interpretation}>{@html highlightKeywords(analysis?.market_interpretation || '')}</div>
    </div>

    <div class={styles.rationale}>{analysis?.rationale || '—'}</div>

    <div class={styles.section}>
        <div class={styles.sectionTitle}>Signals</div>
        <div class={styles.signalList}>
            {#each analysis?.supporting_signals ?? [] as s}
                <span class="{styles.sigChip} {signalDirClass(s)}"><span class="{styles.sigArrow} {signalDirClass(s)}">{signalArrow(s)}</span> {s}</span>
            {/each}
            {#each analysis?.contradicting_signals ?? [] as c}
                <span class="{styles.sigChip} {signalDirClass(c)}"><span class="{styles.sigArrow} {signalDirClass(c)}">{signalArrow(c)}</span> {c}</span>
            {/each}
            {#if (!analysis?.supporting_signals?.length && !analysis?.contradicting_signals?.length)}
                <span class={styles.placeholder}>—</span>
            {/if}
        </div>
    </div>
</div>
