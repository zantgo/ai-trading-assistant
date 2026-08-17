<!--
    SummaryCard — unified [Subject] Summary chrome (v7.0).
    Every MME panel renders its top-of-panel summary inside this card so
    all five summaries share byte-identical chrome: a mono kicker label,
    an always-gray premium background (never semantic-tinted — color
    belongs to badges/chips, gray is the summary signal), a subtle border
    and a 10px radius. The body is an opaque snippet so panels can render
    prose, grids, or verdict blocks without per-panel styling drift.
-->
<script lang="ts">
    import type { Snippet } from 'svelte';
    import styles from './SummaryCard.module.css';

    interface Props {
        /** Kicker label, e.g. "ALIGNMENT SUMMARY". */
        label: string;
        children: Snippet;
    }

    let { label, children }: Props = $props();
</script>

<section class={styles.summaryCard} aria-label={label}>
    <span class={styles.summaryLabel}>{label}</span>
    <div class={styles.summaryBody}>
        {@render children()}
    </div>
</section>
