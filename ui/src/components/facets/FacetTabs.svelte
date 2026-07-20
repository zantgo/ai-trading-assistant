<script lang="ts">
    // FacetTabs — shared 6-tab strip at the top of the Metrics facet body.
    //
    // The redesigned Metrics view pivots the same snapshot data through 6
    // cross-cuts (Indicators / Signals / Divergences / Levels / Liquidity /
    // MTF). This component is the canonical tab bar; the parent owns the
    // active-tab state and renders the matching facet view component.

    import styles from './FacetTabs.module.css';

    export type FacetId =
        | 'indicators'
        | 'signals'
        | 'divergences'
        | 'levels'
        | 'liquidity'
        | 'mtf';

    interface FacetSpec {
        id: FacetId;
        label: string;
        /** Optional badge count surfaced on the tab (e.g. active signal count). */
        count?: number;
    }

    interface Props {
        active: FacetId;
        facets: FacetSpec[];
        onChange: (id: FacetId) => void;
    }

    let { active, facets, onChange }: Props = $props();
</script>

<div class={styles.tabs} role="tablist">
    {#each facets as f (f.id)}
        <button
            class="{styles.tab} {active === f.id ? styles.tabActive : ''}"
            role="tab"
            aria-selected={active === f.id}
            onclick={() => onChange(f.id)}
        >
            <span class={styles.tabLabel}>{f.label}</span>
            {#if f.count != null && f.count > 0}
                <span class={styles.tabCount}>{f.count}</span>
            {/if}
        </button>
    {/each}
</div>
