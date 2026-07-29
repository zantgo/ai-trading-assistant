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

    import type { MarketContext } from '../types';
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

    function fmtScore(n: number | undefined | null): string {
        if (n == null || isNaN(n)) return '--';
        const sign = n > 0 ? '+' : '';
        return `${sign}${n.toFixed(2)}`;
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
        <div class={styles.header}>
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
    {/if}
</div>
