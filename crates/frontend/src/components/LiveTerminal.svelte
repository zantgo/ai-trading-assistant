<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import styles from './LiveTerminal.module.css';
    import type { TimeframeTelemetry } from '../types';

    const app = useAppStore();
    let { pairKey }: { pairKey: string } = $props();
    const pair = $derived(app.instancesMap[pairKey]);
</script>

<div class={styles.terminalWorkspace}>
    <div class={styles.featurePlaceholder}>
        <svg width="64" height="64" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="0.8" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="22 12 18 12 15 21 9 3 6 12 2 12"/>
            <line x1="2" y1="12" x2="22" y2="12"/>
        </svg>
        <h2 class={styles.featurePlaceholderTitle}>Live Charts</h2>
        <p class={styles.featurePlaceholderMsg}>
            Real-time multi-timeframe charts and indicator panels.
            Coming in a future release.
        </p>
    </div>
</div>
