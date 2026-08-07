<script lang="ts">
    import type { AnalysisMatrix, AlignmentMatrix, TimeframeTelemetry } from '../types';
    import type { WsState } from '../lib/websocket.svelte';
    import { useAppStore } from '../state.svelte';
    import { buildAnalysisTabExport } from '../lib/exportBuilders/analysisTab';
    import { buildFilterStateBlock } from '../lib/exportBuilders/shared';
    import ExportDataButton from './ExportDataButton.svelte';
    import LayerHeader from './LayerHeader.svelte';
    import { buildL3AnalysisHeader, type LayerHeaderSpec } from '../lib/layerHeader';
    import styles from './AnalysisPanel.module.css';

    const app = useAppStore();
    let { wssState: _wssState }: { wssState?: WsState } = $props();
    const instance = $derived(app.activeInstance());
    const analysis = $derived<AnalysisMatrix | null>(instance?.analysis ?? null);
    const alignment = $derived<AlignmentMatrix | null>(instance?.alignment ?? null);
    const microTerm = $derived<TimeframeTelemetry | undefined>(instance?.microTerm);
    const microSnap = $derived(microTerm?.latestSnapshot as Record<string, unknown> | undefined);
    const opportunity = $derived((microSnap?.opportunity ?? null) as any);
    const decisionContext = $derived((microSnap?.decision_context ?? null) as Record<string, unknown> | null);
    const markPrice = $derived(parseFloat(microTerm?.priceText ?? '0') || 0);
    const timestamp = $derived<number | null>(
        microSnap && typeof (microSnap as any).timestamp === 'number'
            ? (microSnap as any).timestamp
            : null
    );
    const registry = $derived(app.indicatorRegistry ?? []);
    const pairKey = $derived(app.activeSymbol ?? '');

    function buildExport() {
        return buildAnalysisTabExport({
            analysis,
            alignment,
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
        return styles.tfRegimeNeutral;
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

    function signalDirection(text: string): 'bullish' | 'bearish' | 'neutral' {
        const dir = text.match(/\((bullish|bearish|neutral)\)/i)?.[1]?.toLowerCase();
        if (dir === 'bullish') return 'bullish';
        if (dir === 'bearish') return 'bearish';
        return 'neutral';
    }

    function highlightKeywords(text: string): string {
        if (!text) return '\u2014';
        const keywords = /\b(TRANSITIONAL|DEVELOPING|WEAKENING|UNSTABLE|WEAK|STRONG|HEALTHY|EXHAUSTED|EXPANDING|COMPRESSED|NORMAL|EXTREME|INCREASING|STABLE|REVERSING|BROKEN|EXCEPTIONAL|BULLISH|BEARISH|NEUTRAL)\b/gi;
        return text.replace(keywords, '<strong>$1</strong>');
    }

    // Dynamic extraction of timeframe alignments in exact micro -> fast -> slow -> macro order
    const timeframeSlots = $derived.by(() => {
        const order = ['MICRO', 'FAST', 'SLOW', 'MACRO'];
        const alignments = alignment?.timeframe_alignments ?? [];
        return order.map(slot => {
            const found = alignments.find(a => a.timeframe.toUpperCase() === slot);
            return {
                name: slot,
                active: !!found,
                trend: found?.trend_score ?? 0,
                momentum: found?.momentum_score ?? 0,
                overall: found?.overall_score ?? 0,
                regime: found?.regime ?? 'AWAITING'
            };
        });
    });

    function scoreColor(val: number): string {
        if (val > 0) return '#22c55e'; // Green
        if (val < 0) return '#ef4444'; // Red
        return '#64748b'; // Neutral Gray
    }

    function tfGaugeColor(score: number): string {
        if (score > 5) return '#22c55e';
        if (score < -5) return '#ef4444';
        return '#64748b';
    }

    function tfRegimeCls(r: string): string {
        const u = r.toUpperCase();
        if (u.includes('BULL')) return styles.tfRegimeBull;
        if (u.includes('BEAR')) return styles.tfRegimeBear;
        if (u === 'TRANSITION' || u === 'CONTRACTION' || u === 'EXPANSION') return styles.tfRegimeVol;
        return styles.tfRegimeNeutral;
    }

    // Timeframe sorting helper for signal lists
    function timeframeRank(signal: string): number {
        const s = (signal || '').toUpperCase();
        if (s.includes('MICRO')) return 0;
        if (s.includes('FAST')) return 1;
        if (s.includes('SLOW')) return 2;
        if (s.includes('MACRO')) return 3;
        
        // Fallback checks for explicit candle durations:
        if (s.includes('1S') || s.includes('3S') || s.includes('5S') || s.includes('15S') || s.includes('30S') || s.includes('1M')) return 0;
        if (s.includes('3M') || s.includes('5M')) return 1;
        if (s.includes('15M') || s.includes('30M')) return 2;
        if (s.includes('1H') || s.includes('4H') || s.includes('12H') || s.includes('1D') || s.includes('DAY')) return 3;
        
        return 4; // Default fallback rank for global/ambient signals
    }

    // Unifies and sorts supporting + contradicting signals so slots always remain grouped sequentially.
    // Direction is parsed from the signal text (e.g. "MICRO (bearish): ...") rather than assumed
    // from the bucket, because supporting_signals = agrees with bias, contradicting_signals = opposes bias.
    const sortedSignals = $derived.by(() => {
        const supporting = (analysis?.supporting_signals ?? []).map(s => ({ text: s, type: signalDirection(s) }));
        const contradicting = (analysis?.contradicting_signals ?? []).map(c => ({ text: c, type: signalDirection(c) }));
        const combined = [...supporting, ...contradicting];
        return combined.sort((a, b) => timeframeRank(a.text) - timeframeRank(b.text));
    });

    // ── Signal lean — the operator wants to see at-a-glance whether the
    // signals net bullish or bearish. Direction is parsed from each signal
    // text rather than assumed from the supporting/contradicting bucket.
    const signalLean = $derived.by((): {
        label: string;
        bullish: number;
        bearish: number;
        tone: 'bull' | 'bear' | 'split';
        callHtml: string;
        metaHtml: string;
    } => {
        const allTexts = [...(analysis?.supporting_signals ?? []), ...(analysis?.contradicting_signals ?? [])];
        const bull = allTexts.filter(t => signalDirection(t) === 'bullish').length;
        const bear = allTexts.filter(t => signalDirection(t) === 'bearish').length;
        const total = bull + bear;
        if (total === 0) return { label: 'No per-TF signals', bullish: 0, bearish: 0, tone: 'split',
            callHtml: 'No signals', metaHtml: 'Waiting for cross-TF consensus' };
        if (bull > bear * 1.5) {
            const ratio = bear > 0 ? (bull / bear).toFixed(1) : bull.toFixed(0);
            return { label: `Net bullish \u00b7 ${bull}\u2191 vs ${bear}\u2193`, bullish: bull, bearish: bear, tone: 'bull',
                callHtml: `Net bullish (${bull}\u2191 vs ${bear}\u2193)`, metaHtml: `${ratio}:1 signal ratio` };
        }
        if (bear > bull * 1.5) {
            const ratio = bull > 0 ? (bear / bull).toFixed(1) : bear.toFixed(0);
            return { label: `Net bearish \u00b7 ${bull}\u2191 vs ${bear}\u2193`, bullish: bull, bearish: bear, tone: 'bear',
                callHtml: `Net bearish (${bull}\u2191 vs ${bear}\u2193)`, metaHtml: `${ratio}:1 signal ratio` };
        }
        return { label: `Split signals \u00b7 ${bull}\u2191 vs ${bear}\u2193`, bullish: bull, bearish: bear, tone: 'split',
            callHtml: 'Split signals', metaHtml: `${bull}\u2191 vs ${bear}\u2193` };
    });

    // Helper to decompose raw signal strings into structural elements
    interface DecomposedSignal {
        raw: string;
        timeframe: string;
        score: number | null;
        regime: string;
        signalsCount: number | null;
    }

    function decomposeSignal(text: string): DecomposedSignal {
        const t = text || '';

        let timeframe = 'GLOBAL';
        const tfMatch = t.match(/\[?(MICRO|FAST|SLOW|MACRO|1S|3S|5S|15S|30S|1M|3M|5M|15M|30M|1H|4H|12H|1D)\]?/i);
        if (tfMatch) {
            timeframe = tfMatch[1].toUpperCase();
        }

        let score: number | null = null;
        const scoreMatch = t.match(/score\s+([+\-]?\d+)/i);
        if (scoreMatch) {
            score = parseInt(scoreMatch[1], 10);
        }

        let regime = 'UNKNOWN';
        const regimeMatch = t.match(/([a-zA-Z\-_]+)\s+regime/i);
        if (regimeMatch) {
            regime = regimeMatch[1].toUpperCase();
        }

        let signalsCount: number | null = null;
        const sigMatch = t.match(/(\d+)\s+signals?/i);
        if (sigMatch) {
            signalsCount = parseInt(sigMatch[1], 10);
        }

        return { raw: t, timeframe, score, regime, signalsCount };
    }

    // L3 LayerHeader — single authoritative badge; the regime is
    // suppressed from chips when it's redundant with the bias (e.g.
    // bias='BULLISH' ∧ regime='TRENDING_BULL' is one fact, not two).
    const headerSpec = $derived<LayerHeaderSpec>(buildL3AnalysisHeader(analysis));
</script>

<div class={styles.panel}>
    <!-- v7.0-prod: the panel-level banner above the LayerHeader was removed
         (D9 — no text above any badge). Per-section empty states still
         surface from within the body when a matrix hasn't loaded yet. -->
    <LayerHeader spec={headerSpec}>
        {#snippet trailing()}
            <h2 class={styles.title}>Market Analysis</h2>
            <ExportDataButton onExport={buildExport} title="Copy all Analysis data as JSON" />
        {/snippet}
    </LayerHeader>

    <!-- ── Signal Lean Hero (now lives below the canonical header — the
            bias badge + regime badge + quality badge previously in the
            header have all been absorbed into the LayerHeader) ── -->
    <div class={styles.section}>
        <div class={styles.signalLeanHeroLabel}>SIGNAL LEAN</div>
        <div class="{styles.signalLeanHero} {signalLean.tone === 'bull' ? styles.signalLeanBull : signalLean.tone === 'bear' ? styles.signalLeanBear : styles.signalLeanSplit}">
            <span class={styles.signalLeanHeroCall}>{signalLean.callHtml}</span>
            <span class={styles.signalLeanHeroMeta}>{signalLean.metaHtml}</span>
            {#if signalLean.bullish + signalLean.bearish > 0}
                {@const total = signalLean.bullish + signalLean.bearish}
                <div class={styles.signalLeanBar}>
                    <div class={styles.signalLeanBarBull} style="width: {(signalLean.bullish / total * 100).toFixed(1)}%"></div>
                    <div class={styles.signalLeanBarBear} style="width: {(signalLean.bearish / total * 100).toFixed(1)}%"></div>
                </div>
            {/if}
        </div>
    </div>

    <!-- ── Signals Grid Squares Section ── -->
    <div class={styles.section}>
        <div class={styles.signalsHeader}>
            <span class={styles.sectionTitle}>Signals</span>
            <span class="{styles.signalLeanChip} {signalLean.tone === 'bull' ? styles.signalLeanBull : signalLean.tone === 'bear' ? styles.signalLeanBear : styles.signalLeanSplit}">
                {signalLean.label}
            </span>
        </div>
        {#if sortedSignals.length > 0}
            <div class={styles.signalList}>
                {#each sortedSignals as sig (sig.text)}
                    {@const p = decomposeSignal(sig.text)}
                    {@const dir = sig.type}
                    <div class="{styles.sigSquare} {dir === 'bullish' ? styles.sigSquareBull : styles.sigSquareBear}" title={p.raw}>
                        <span class={styles.sigTf}>{p.timeframe}</span>
                        <div class={styles.sigIconWrap}>
                            {#if dir === 'bullish'}
                                <svg viewBox="0 0 24 24" class={styles.sigIcon} fill="none" stroke="#22c55e" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                                    <line x1="12" y1="19" x2="12" y2="5"></line>
                                    <polyline points="5 12 12 5 19 12"></polyline>
                                </svg>
                            {:else}
                                <svg viewBox="0 0 24 24" class={styles.sigIcon} fill="none" stroke="#ef4444" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                                    <line x1="12" y1="5" x2="12" y2="19"></line>
                                    <polyline points="19 12 12 19 5 12"></polyline>
                                </svg>
                            {/if}
                        </div>
                        <div class={styles.sigMetricsWrap}>
                            <div class={styles.sigMetricRow}>
                                <span class={styles.sigMetricLabel}>Score:</span>
                                <span class={styles.sigMetricValue} style="color: {scoreColor(p.score ?? 0)}">
                                    {p.score !== null ? (p.score >= 0 ? '+' : '') + p.score : '—'}
                                </span>
                            </div>
                            <div class={styles.sigMetricRow}>
                                <span class={styles.sigMetricLabel}>Regime:</span>
                                <span class={styles.sigMetricValue} style="color: {tfRegimeCls(p.regime) === styles.tfRegimeBull ? '#22c55e' : tfRegimeCls(p.regime) === styles.tfRegimeBear ? '#ef4444' : '#f59e0b'}">
                                    {p.regime}
                                </span>
                            </div>
                            <div class={styles.sigMetricRow}>
                                <span class={styles.sigMetricLabel}>Signals:</span>
                                <span class={styles.sigMetricValue} style="color: #22c55e;">
                                    {p.signalsCount !== null ? p.signalsCount : '—'}
                                </span>
                            </div>
                        </div>
                    </div>
                {/each}
            </div>
        {:else}
            <div class={styles.placeholder}>—</div>
        {/if}
    </div>

    <!-- ── Qualitative Assessment (Without Timeframes Card) ── -->
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
        </div>
    </div>

    <!-- ── Ordered Timeframe Grid Squares (2x2) ── -->
    <div class={styles.section}>
        <div class={styles.sectionTitle}>Per-Timeframe Alignment</div>
        <div class={styles.timeframeGrid}>
            {#each timeframeSlots as tf (tf.name)}
                <div class={styles.tfSquare}>
                    <div class={styles.tfHeader}>
                        <span class={styles.tfName}>{tf.name}</span>
                        {#if tf.active}
                            <div class={styles.tfGauge}>
                                <svg viewBox="0 0 24 24" class={styles.tfGaugeSvg}>
                                    <circle cx="12" cy="12" r="10" class={styles.tfGaugeTrack} />
                                    <circle
                                        cx="12"
                                        cy="12"
                                        r="10"
                                        class={styles.tfGaugeProgress}
                                        stroke={tfGaugeColor(tf.overall)}
                                        stroke-dasharray="62.8"
                                        stroke-dashoffset={62.8 * (1 - Math.min(Math.max((tf.overall + 100) / 200, 0), 1))}
                                        transform="rotate(-90 12 12)"
                                    />
                                </svg>
                            </div>
                        {/if}
                    </div>
                    <div class={styles.tfGridBody}>
                        <div class={styles.tfStatRow}>
                            <span class={styles.tfStatLabel}>Trend</span>
                            <span class={styles.tfStatValue} style="color: {scoreColor(tf.trend)}">
                                {tf.active ? (tf.trend >= 0 ? '+' : '') + tf.trend.toFixed(2) : '—'}
                            </span>
                        </div>
                        <div class={styles.tfStatRow}>
                            <span class={styles.tfStatLabel}>Momentum</span>
                            <span class={styles.tfStatValue} style="color: {scoreColor(tf.momentum)}">
                                {tf.active ? (tf.momentum >= 0 ? '+' : '') + tf.momentum.toFixed(2) : '—'}
                            </span>
                        </div>
                        <div class={styles.tfStatRow}>
                            <span class={styles.tfStatLabel}>Overall</span>
                            <span class={styles.tfStatValue} style="color: {tfGaugeColor(tf.overall)}">
                                {tf.active ? (tf.overall >= 0 ? '+' : '') + tf.overall.toFixed(1) : '—'}
                            </span>
                        </div>
                        <div class={styles.tfStatRow}>
                            <span class={styles.tfStatLabel}>Regime</span>
                            <span class="{styles.tfRegimeBadge} {tfRegimeCls(tf.regime)}">
                                {tf.active ? tf.regime : 'OFFLINE'}
                            </span>
                        </div>
                    </div>
                </div>
            {/each}
        </div>
    </div>

    <!-- ── Interpretation & Summary ── -->
    <div class={styles.section}>
        <div class={styles.sectionTitle}>Interpretation</div>
        <div class={styles.interpretation}>{@html highlightKeywords(analysis?.market_interpretation || '')}</div>
    </div>

    <div class={styles.rationale}>{analysis?.rationale || '—'}</div>
</div>
