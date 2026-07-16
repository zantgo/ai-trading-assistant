<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import styles from './RiskPanel.module.css';

    const app = useAppStore();
    let { pairKey }: { pairKey: string } = $props();
</script>

<div class={styles.panel}>
    <div class={styles.featurePlaceholder}>
        <svg width="64" height="64" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="0.8" stroke-linecap="round" stroke-linejoin="round">
            <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
            <line x1="9" y1="12" x2="11" y2="14"/>
            <line x1="11" y1="14" x2="15" y2="10"/>
        </svg>
        <h2 class={styles.featurePlaceholderTitle}>Risk Assessment</h2>
        <p class={styles.featurePlaceholderMsg}>
            Multi-dimensional risk evaluation and position sizing.
            Coming in a future release.
        </p>
    </div>
</div>
