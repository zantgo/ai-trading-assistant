<!--
    SummaryCard — unified [Subject] Summary chrome (v7.3).
    Every MME panel renders its top-of-panel summary as a section title
    ABOVE a near-black container: the title uses the standard 11px
    uppercase `.sectionTitle` language shared by SCORE / TRADE SETUPS /
    RISK DIMENSIONS, and the card beneath it is the premium dark-cockpit
    container (--bg-elev fill, hairline --line border, 10px radius, 14px
    vertical gutter). The body is an opaque snippet so panels can render
    prose, grids, or verdict blocks without per-panel styling drift;
    `strong` spans get the unified high-contrast keyword treatment.
    The VERDICT & RATIONALE card (and only it) may pass an `accent` prop
    to draw the 3px left-edge bias line (green long / red short / amber
    hold) directly on the card boundary.
-->
<script lang="ts">
    import type { Snippet } from 'svelte';
    import styles from './SummaryCard.module.css';

    interface Props {
        /** Section title rendered above the card, e.g. "ALIGNMENT SUMMARY". */
        label: string;
        /** Verdict-only left-edge accent: 'long' | 'short' | 'hold'. */
        accent?: 'long' | 'short' | 'hold';
        children: Snippet;
    }

    let { label, accent, children }: Props = $props();

    const accentClass = $derived(
        accent === 'long'
            ? styles.accentLong
            : accent === 'short'
                ? styles.accentShort
                : accent === 'hold'
                    ? styles.accentHold
                    : '',
    );
</script>

<div class={styles.summaryWrap}>
    <div class={styles.summaryTitle}>{label}</div>
    <section class="{styles.summaryCard} {accentClass}" aria-label={label}>
        <div class={styles.summaryBody}>
            {@render children()}
        </div>
    </section>
</div>
