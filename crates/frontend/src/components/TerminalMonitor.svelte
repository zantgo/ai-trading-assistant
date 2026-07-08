<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { useAppStore } from '../state.svelte';
    import styles from './TerminalMonitor.module.css';
    import TelemetryTable from './TelemetryTable.svelte';
    import type {
        MonitorResponse, MarketContext, ContextDimension, IndicatorSignal,
        TimeframeTelemetry, IndicatorMap,
    } from '../types';

    const app = useAppStore();
    let { pairKey }: { pairKey: string } = $props();
    const pair = $derived(app.instancesMap[pairKey]);

    let monitor = $state<MonitorResponse | null>(null);
    let loading = $state(false);
    let timer: ReturnType<typeof setInterval> | null = null;

    async function fetchMonitor() {
        if (loading) return;
        loading = true;
        try {
            const res = await fetch(`/api/monitor?symbol=${encodeURIComponent(pairKey)}`);
            if (res.ok) monitor = await res.json();
        } catch (_) { /* transient */ }
        loading = false;
    }

    onMount(() => {
        fetchMonitor();
        timer = setInterval(fetchMonitor, 5000);
    });
    onDestroy(() => { if (timer) clearInterval(timer); });

    // Prefer authoritative backend synthesis; fall back to live micro context.
    const context = $derived<MarketContext | null>(
        monitor?.market_context
        ?? ((pair?.microTerm?.latestSnapshot as any)?.context ?? null)
    );

    const CTX_DIMS: Array<[string, (c: MarketContext) => ContextDimension]> = [
        ['TREND', (c) => c.trend],
        ['MOMENTUM', (c) => c.momentum],
        ['VOLATILITY', (c) => c.volatility],
        ['VOLUME', (c) => c.volume],
        ['LIQUIDITY', (c) => c.liquidity],
    ];

    function scoreColor(score: number): string {
        const mag = Math.min(Math.abs(score), 1);
        if (mag >= 0.9) return '#a855f7';
        if (score > 0.1) return `rgb(16, ${Math.round(120 + 135 * mag)}, 129)`;
        if (score < -0.1) return `rgb(${Math.round(180 + 59 * mag)}, 68, 68)`;
        return '#94a3b8';
    }
    function overallColor(score: number): string { return scoreColor(score / 100); }
    function pct(v: number): string { return `${Math.round(v)}%`; }

    function agreementColor(a: number): string {
        if (a >= 0.75) return '#10b981';
        if (a >= 0.5) return '#94a3b8';
        return '#ef4444';
    }
    function dirGlyph(d: number): string { return d > 0 ? '▲' : d < 0 ? '▼' : '·'; }
    function dirColor(d: number): string { return d > 0 ? '#10b981' : d < 0 ? '#ef4444' : '#64748b'; }

    // ── Live signals feed across all four timeframes (freshness-sorted) ──
    type FeedItem = { tf: string; indicator: string; sig: IndicatorSignal };
    const TF_KEYS = [
        ['MICRO', 'microTerm'], ['FAST', 'fastTerm'], ['SLOW', 'slowTerm'], ['MACRO', 'macroTerm'],
    ] as const;
    const signalsFeed = $derived.by<FeedItem[]>(() => {
        if (!pair) return [];
        const out: FeedItem[] = [];
        for (const [label, key] of TF_KEYS) {
            const tf = (pair as any)[key] as TimeframeTelemetry;
            const map = (tf?.indicators ?? {}) as IndicatorMap;
            for (const [ind, dto] of Object.entries(map)) {
                for (const s of dto.signals ?? []) {
                    out.push({ tf: label, indicator: ind, sig: s });
                }
            }
        }
        return out.sort((a, b) => (a.sig.age_bars ?? 0) - (b.sig.age_bars ?? 0)).slice(0, 24);
    });
    function sigColor(s: IndicatorSignal): string {
        return s.direction === 'Bullish' ? '#10b981' : s.direction === 'Bearish' ? '#ef4444' : '#94a3b8';
    }
</script>

{#if pair}
<div class={styles.monitor}>
    <div class={styles.header}>
        <span class={styles.title}>TERMINAL MONITOR</span>
        <span class={styles.symbol}>{app.pairDisplayFor(pair.symbol)}</span>
        {#if monitor}
            <span class={styles.regimeTag}>REGIME: {context?.regime ?? '—'}</span>
            {#if context}
                {@const rc = (context as any).regime_confidence ?? 0}
                <span class={styles.regimeConf} style="color:{rc >= 0.85 ? '#10b981' : rc >= 0.60 ? '#f59e0b' : rc >= 0.40 ? '#ef4444' : '#94a3b8'}">
                    {rc >= 0.85 ? 'STRONG' : rc >= 0.60 ? 'MODERATE' : rc >= 0.40 ? 'WEAK' : 'TRANSITIONAL'}
                </span>
                {@const rs = (context as any).regime_stability ?? 0}
                <span class={styles.regimeStab}>STB {Math.round(rs * 100)}%</span>
            {/if}
            <span class={styles.trendAgree} style="color:{agreementColor(monitor.mtf.trend_agreement_pct / 100)}">
                MTF AGREEMENT {pct(monitor.mtf.trend_agreement_pct)}
            </span>
        {/if}
        <button class={styles.refreshBtn} onclick={fetchMonitor}>{loading ? '…' : '⟳'}</button>
    </div>

    <!-- Market Context -->
    {#if context}
        <div class={styles.contextGrid}>
            {#each CTX_DIMS as [label, sel]}
                {@const d = sel(context)}
                <div class={styles.ctxCard}>
                    <div class={styles.ctxLabel}>{label}</div>
                    <div class={styles.ctxValue} style="color:{scoreColor(d.score)}">{d.label}</div>
                    <div class={styles.ctxBarTrack}>
                        <div class={styles.ctxBarFill} style="width:{Math.round(d.confidence * 100)}%; background:{scoreColor(d.score)}"></div>
                    </div>
                    <div class={styles.ctxConf}>conf {Math.round(d.confidence * 100)}%</div>
                </div>
            {/each}
            <div class="{styles.ctxCard} {styles.overallCard}">
                <div class={styles.ctxLabel}>OVERALL</div>
                <div class={styles.overallScore} style="color:{overallColor(context.overall_score)}">
                    {context.overall_score > 0 ? '+' : ''}{context.overall_score}
                </div>
                <div class={styles.ctxValue} style="color:{overallColor(context.overall_score)}">{context.overall_label}</div>
            </div>
        </div>
    {:else}
        <div class={styles.empty}>Awaiting market-context synthesis…</div>
    {/if}

    <div class={styles.twoCol}>
        <!-- MTF Confirmation matrix -->
        <div class={styles.panel}>
            <div class={styles.panelTitle}>MULTI-TIMEFRAME CONFIRMATION</div>
            {#if monitor && monitor.mtf.rows.length > 0}
                <table class={styles.mtfTable}>
                    <thead>
                        <tr><th>Indicator</th><th>M</th><th>F</th><th>S</th><th>Mac</th><th>Agree</th></tr>
                    </thead>
                    <tbody>
                        {#each monitor.mtf.rows as row}
                            <tr>
                                <td class={styles.mtfLabel}>{row.display_name}</td>
                                {#each row.per_tf as d}
                                    <td style="color:{dirColor(d)}; text-align:center">{dirGlyph(d)}</td>
                                {/each}
                                <td style="color:{agreementColor(row.agreement)}; text-align:right">{pct(row.agreement * 100)}</td>
                            </tr>
                        {/each}
                    </tbody>
                </table>
            {:else}
                <div class={styles.empty}>No cross-timeframe data yet.</div>
            {/if}
        </div>

        <!-- Per-timeframe confluence + signals feed -->
        <div class={styles.panel}>
            <div class={styles.panelTitle}>CONFLUENCE BY TIMEFRAME</div>
            {#if monitor}
                {#each monitor.timeframes as tf}
                    <div class={styles.confRow}>
                        <span class={styles.confTf}>{tf.label}</span>
                        <div class={styles.confBarTrack}>
                            <div class={styles.confBarZero}></div>
                            <div class={styles.confBarFill}
                                style="width:{Math.min(Math.abs(tf.confluence_score), 100) / 2}%;
                                       margin-left:{tf.confluence_score >= 0 ? '50' : (50 - Math.min(Math.abs(tf.confluence_score), 100) / 2)}%;
                                       background:{overallColor(tf.confluence_score)}"></div>
                        </div>
                        <span class={styles.confScore} style="color:{overallColor(tf.confluence_score)}">
                            {tf.confluence_score > 0 ? '+' : ''}{tf.confluence_score}
                        </span>
                        <span class={styles.confRegime}>{tf.regime}</span>
                    </div>
                {/each}
            {/if}

            <div class="{styles.panelTitle} {styles.panelTitleSpacer}">LIVE SIGNALS (freshest)</div>
            <div class={styles.feed}>
                {#each signalsFeed as f}
                    <div class={styles.feedRow}>
                        <span class={styles.feedTf}>{f.tf}</span>
                        <span class={styles.feedInd}>{f.indicator}</span>
                        <span class={styles.feedKind} style="color:{sigColor(f.sig)}">{f.sig.kind}</span>
                        <span class={styles.feedAge}>{(f.sig.age_bars ?? 0) === 0 ? 'now' : `${f.sig.age_bars}b`}</span>
                    </div>
                {:else}
                    <div class={styles.empty}>No active signals.</div>
                {/each}
            </div>
        </div>
    </div>

    <!-- Detailed telemetry matrix (moved from the charts tab) -->
    <TelemetryTable {pairKey} />
</div>
{/if}
