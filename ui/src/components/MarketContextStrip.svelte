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
    // expanded reveals the 5 dimension chips. Only renders when the
    // context block is present in the snapshot — no fabricated state.

    import type { ContextDimension, MarketContext } from '../types';
    import styles from './MarketContextStrip.module.css';

    interface Props {
        context: MarketContext | null | undefined;
        /** Snapshot timestamp (ms) for the "Age: N bars" label. */
        timestamp?: number | null;
        /** Bar duration in seconds — used to derive age in completed bars. */
        barDurationSec?: number;
    }

    let { context, timestamp = null, barDurationSec = 60 }: Props = $props();

    let expanded = $state(false);

    const DIM_ORDER: Array<{ key: keyof Omit<MarketContext, 'regime' | 'overall_score' | 'overall_label'>; label: string }> = [
        { key: 'trend',     label: 'Trend' },
        { key: 'momentum',  label: 'Momentum' },
        { key: 'volatility',label: 'Volatility' },
        { key: 'volume',    label: 'Volume' },
        { key: 'liquidity', label: 'Liquidity' },
    ];

    function dimClass(d: ContextDimension | undefined | null): string {
        if (!d) return styles.dimNeutral ?? '';
        const s = d.score ?? 0;
        if (s > 0.5) return styles.dimBull ?? '';
        if (s < -0.5) return styles.dimBear ?? '';
        return styles.dimNeutral ?? '';
    }

    function fmtScore(n: number | undefined | null): string {
        if (n == null || isNaN(n)) return '--';
        const sign = n > 0 ? '+' : '';
        return `${sign}${n.toFixed(2)}`;
    }

    function ageBars(): number | null {
        if (!timestamp) return null;
        const ageMs = Date.now() - timestamp;
        if (ageMs < 0 || !barDurationSec) return null;
        return Math.floor(ageMs / 1000 / barDurationSec);
    }

    function regimeClass(regime: string | undefined): string {
        const r = (regime ?? '').toUpperCase();
        if (r.includes('TRENDING')) return styles.regimeTrending ?? '';
        if (r.includes('COMPRESS') || r.includes('CONTRACT')) return styles.regimeCompress ?? '';
        if (r.includes('EXPAND')) return styles.regimeExpand ?? '';
        if (r.includes('RANGE')) return styles.regimeRange ?? '';
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
        <button
            class={styles.header}
            onclick={() => expanded = !expanded}
            aria-expanded={expanded}
        >
            <span class={styles.caret}>{expanded ? '▼' : '▶'}</span>
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
        </button>

        {#if expanded}
            <div class={styles.body}>
                {#each DIM_ORDER as dim (dim.key)}
                    {@const d = context[dim.key]}
                    <div class="{styles.dim} {dimClass(d)}">
                        <div class={styles.dimLabel}>{dim.label}</div>
                        <div class={styles.dimScore}>{fmtScore(d?.score)}</div>
                        <div class={styles.dimSubLabel}>{d?.label ?? '—'}</div>
                    </div>
                {/each}
            </div>
        {/if}
    {/if}
</div>
