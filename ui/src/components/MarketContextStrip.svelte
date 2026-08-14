<script lang="ts">
    // MarketContextStrip — row 1 of the redesigned Metrics view.
    //
    // Renders the per-TF `MarketContext` block that the analyzer already
    // attaches to every completed snapshot. Previously this block was
    // hidden inside `latestSnapshot: Record<string, unknown>` and never
    // surfaced — this is the canonical LOCAL synthesis (5 dimensions +
    // regime + overall score/label).
    //
    // Collapsed by default to a one-line summary (regime + overall);
    // clicking the header (or the caret) reveals the 5 dimension chips
    // (trend / momentum / volatility / volume / liquidity). The same five
    // dimensions are carried verbatim in the single-TF export
    // (`market_context`), so the screen and the JSON always mirror.
    // Only renders when the context block is present — no fabricated state.

    import type { MarketContext } from '../types';
    import { regimeTone } from '../lib/dashboardColors';
    import styles from './MarketContextStrip.module.css';

    interface Props {
        context: MarketContext | null | undefined;
        /** Snapshot timestamp (Unix seconds) for the "Age: N bars" label. */
        timestamp?: number | null;
        /** Bar duration in seconds — used to derive age in completed bars. */
        barDurationSec?: number;
        /** Number of active signals in this TF — shown as compact badge. */
        signalCount?: number;
    }

    let { context, timestamp = null, barDurationSec = 60, signalCount }: Props = $props();

    // M-1 (v6.10.11): the five L1 LOCAL synthesis dimensions live in the
    // snapshot context and in the export; the strip now surfaces them
    // (the template had lost the body the CSS still styled).
    let expanded = $state(false);

    const DIMS = [
        { key: 'trend', label: 'Trend' },
        { key: 'momentum', label: 'Momentum' },
        { key: 'volatility', label: 'Volatility' },
        { key: 'volume', label: 'Volume' },
        { key: 'liquidity', label: 'Liquidity' },
    ] as const;

    function fmtScore(n: number | undefined | null): string {
        if (n == null || isNaN(n)) return '--';
        const sign = n > 0 ? '+' : '';
        return `${sign}${n.toFixed(2)}`;
    }

    function dimConfidence(pct: number | undefined | null): string {
        // The wire carries confidence as 0..1; the export renders
        // `Math.round(confidence * 100)%` — mirror it exactly.
        if (pct == null || isNaN(pct)) return '--%';
        return `${Math.round(pct * 100)}%`;
    }

    function dimClass(n: number | undefined | null): string {
        if (n == null || isNaN(n)) return styles.dimNeutral ?? '';
        if (n > 0) return styles.dimBull ?? '';
        if (n < 0) return styles.dimBear ?? '';
        return styles.dimNeutral ?? '';
    }

    function ageBars(): number | null {
        // `timestamp` arrives as Unix **seconds** on the wire
        // (Rust: `candle.start_time_ms / 1000` at analyzer/mod.rs:549),
        // so subtract in seconds before dividing by bar duration.
        if (!timestamp) return null;
        const ageSec = (Date.now() / 1000) - timestamp;
        if (ageSec < 0 || !barDurationSec) return null;
        return Math.floor(ageSec / barDurationSec);
    }

    function regimeClass(regime: string | undefined): string {
        // M-6 (v6.10.11): tone classification via the shared `regimeTone`;
        // this panel colors the regime FAMILY (trending / compressed /
        // expanding / range) while the alignment panel colors direction.
        const r = (regime ?? '').toUpperCase();
        const tone = regimeTone(r);
        if (tone === 'bull' || tone === 'bear') return styles.regimeTrending ?? '';
        if (tone === 'vol') {
            return r.includes('COMPRESS') || r.includes('CONTRACT')
                ? styles.regimeCompress ?? ''
                : styles.regimeExpand ?? '';
        }
        if (tone === 'range') return styles.regimeRange ?? '';
        return styles.regimeNeutral ?? '';
    }

    function overallClass(label: string | undefined): string {
        const l = (label ?? '').toUpperCase();
        if (l.includes('STRONG_BULL')) return styles.overallBullStrong ?? '';
        if (l.includes('WEAK_BULL') || l === 'BULL') return styles.overallBull ?? '';
        if (l.includes('STRONG_BEAR')) return styles.overallBearStrong ?? '';
        if (l.includes('WEAK_BEAR') || l === 'BEAR') return styles.overallBear ?? '';
        return styles.overallNeutral ?? '';
    }
</script>

<div class={styles.strip}>
    {#if !context}
        <div class={styles.placeholder}>
            <span class={styles.dimLabel}>MARKET CONTEXT</span>
            <span class={styles.placeholderText}>Awaiting completed snapshot…</span>
        </div>
    {:else}
        <div class={styles.header} role="button" tabindex="0"
             onclick={() => expanded = !expanded}
             onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); expanded = !expanded; } }}
             aria-expanded={expanded}
             title={expanded ? 'Collapse dimensions' : 'Expand 5-dimension synthesis'}>
            <span class={styles.caret}>{expanded ? '▾' : '▸'}</span>
            <span class={styles.title}>MARKET CONTEXT</span>
            <span class="{styles.regimeBadge} {regimeClass(context.regime)}">
                {context.regime}
            </span>
            <span class={styles.divider}>·</span>
            <span class={styles.overallLabel}>Overall</span>
            <span class="{styles.overallScore} {overallClass(context.overall_label)}">
                {fmtScore(context.overall_score)}
            </span>
            <span class="{styles.overallLabelText} {overallClass(context.overall_label)}">
                {context.overall_label}
            </span>
            {#if ageBars() != null}
                <span class={styles.divider}>·</span>
                <span class={styles.ageLabel}>Age {ageBars()}b</span>
            {/if}
            {#if signalCount != null && signalCount > 0}
                <span class={styles.divider}>·</span>
                <span class={styles.signalCount}>{signalCount} signals</span>
            {/if}
        </div>
        <!-- M-1 (v6.10.11): the five L1 LOCAL synthesis dimensions —
             trend / momentum / volatility / volume / liquidity — rendered
             from the same `MarketContext` the export carries. -->
        {#if expanded}
            <div class={styles.body}>
                {#each DIMS as d (d.key)}
                    {@const dim = context[d.key]}
                    <div class="{styles.dim} {dimClass(dim?.score)}">
                        <span class={styles.dimLabel}>{d.label}</span>
                        <span class={styles.dimScore}>{fmtScore(dim?.score)}</span>
                        <span class={styles.dimSubLabel}>{dimConfidence(dim?.confidence)}</span>
                        <span class={styles.dimSubLabel}>{dim?.label}</span>
                    </div>
                {/each}
            </div>
        {/if}
    {/if}
</div>
