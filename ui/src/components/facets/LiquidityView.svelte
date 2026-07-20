<script lang="ts">
    // LiquidityView — Facet #5 of the redesigned Metrics view.
    //
    // Thin wrapper around the existing `LiquidityPanel.svelte` that re-houses
    // it as a peer of the other facets under the FacetTabs strip.
    //
    // Bug fix: the original panel compared PascalCase strings
    // (`'Sustained'`, `'Long'`, `'AboveCurrentPrice'`, `'FundingAdaptive'`)
    // against the wire format which is SCREAMING_SNAKE_CASE
    // (`'SUSTAINED'`, `'LONG'`, `'ABOVE_CURRENT_PRICE'`, `'FUNDING_ADAPTIVE'`).
    // Every real backend snapshot silently fell through to the default style.
    // The TypeScript types in `types.ts` were also updated to match the wire
    // format; this component hands the snapshot data to the LiquidityPanel
    // unchanged so the styling works as intended.

    import LiquidityPanel from '../LiquidityPanel.svelte';
    import styles from './LiquidityView.module.css';

    interface Props {
        pairKey: string;
    }

    let { pairKey }: Props = $props();
</script>

<div class={styles.view}>
    <LiquidityPanel {pairKey} />
</div>
