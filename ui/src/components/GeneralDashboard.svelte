<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import styles from './GeneralDashboard.module.css';

    const app = useAppStore();
</script>

<div class={styles.dashboardView}>
    <div class={styles.featurePlaceholder}>
        <svg width="64" height="64" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="0.8" stroke-linecap="round" stroke-linejoin="round">
            <rect x="3" y="3" width="7" height="9" rx="1"/>
            <rect x="14" y="3" width="7" height="5" rx="1"/>
            <rect x="14" y="12" width="7" height="9" rx="1"/>
            <rect x="3" y="16" width="7" height="5" rx="1"/>
        </svg>
        <h2 class={styles.featurePlaceholderTitle}>Market Overview</h2>
        <p class={styles.featurePlaceholderMsg}>
            System-wide state matrix, market confluence, and pair overview tables.
            Coming in a future release.
        </p>
    </div>
</div>
