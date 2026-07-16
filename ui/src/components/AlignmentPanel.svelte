<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import styles from './AlignmentPanel.module.css';

    const app = useAppStore();
    let { pairKey }: { pairKey: string } = $props();
</script>

<div class={styles.panel}>
    <div class={styles.featurePlaceholder}>
        <svg width="64" height="64" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="0.8" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="6" cy="6" r="2"/><circle cx="18" cy="6" r="2"/>
            <circle cx="12" cy="12" r="2"/><circle cx="6" cy="18" r="2"/><circle cx="18" cy="18" r="2"/>
            <line x1="8" y1="6" x2="16" y2="6"/><line x1="6" y1="8" x2="6" y2="16"/>
            <line x1="18" y1="8" x2="18" y2="16"/><line x1="8" y1="18" x2="16" y2="18"/>
            <line x1="8" y1="8" x2="10" y2="10"/><line x1="16" y1="8" x2="14" y2="10"/>
            <line x1="8" y1="16" x2="10" y2="14"/><line x1="16" y1="16" x2="14" y2="14"/>
        </svg>
        <h2 class={styles.featurePlaceholderTitle}>MTF Alignment</h2>
        <p class={styles.featurePlaceholderMsg}>
            Cross-timeframe structural alignment analysis.
            Coming in a future release.
        </p>
    </div>
</div>
