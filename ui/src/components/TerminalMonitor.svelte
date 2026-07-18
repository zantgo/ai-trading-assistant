<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import TelemetryTable from './TelemetryTable.svelte';
    import styles from './TerminalMonitor.module.css';

    const app = useAppStore();
    let { pairKey }: { pairKey: string } = $props();
    const pair = $derived(app.instancesMap[pairKey]);
    const registry = $derived(app.indicatorRegistry);
</script>

<div class={styles.monitor}>
    {#if pair && registry && registry.length > 0}
        <TelemetryTable {pairKey} />
    {:else}
        <div class={styles.featurePlaceholder}>
            <svg width="64" height="64" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="0.8" stroke-linecap="round" stroke-linejoin="round">
                <rect x="3" y="3" width="18" height="18" rx="2"/>
                <rect x="7" y="7" width="4" height="4" rx="0.5"/>
                <rect x="13" y="7" width="4" height="4" rx="0.5"/>
                <rect x="7" y="13" width="10" height="4" rx="0.5"/>
            </svg>
            <h2 class={styles.featurePlaceholderTitle}>Market Metrics</h2>
            <p class={styles.featurePlaceholderMsg}>
                Awaiting indicator registry and market data...
            </p>
        </div>
    {/if}
</div>
