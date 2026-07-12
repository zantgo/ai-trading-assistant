<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import styles from './AdvisoryPanel.module.css';

    const app = useAppStore();
    let { pairKey }: { pairKey: string } = $props();
</script>

<div class={styles.panel}>
    <div class={styles.featurePlaceholder}>
        <svg width="64" height="64" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="0.8" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="12" cy="12" r="10"/>
            <polygon points="12 6 14.4 11.2 20 12 14.4 12.8 12 18 9.6 12.8 4 12 9.6 11.2"/>
        </svg>
        <h2 class={styles.featurePlaceholderTitle}>Decision</h2>
        <p class={styles.featurePlaceholderMsg}>
            AI-assisted directional bias and trade ideas.
            Coming in a future release.
        </p>
    </div>
</div>
