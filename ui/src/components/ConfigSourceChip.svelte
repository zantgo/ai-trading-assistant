<script lang="ts">
    // ConfigSourceChip — provenance + apply-semantics chip for settings
    // cards: `config.toml → [workspace.x]` with a LIVE / NEW PIPELINES /
    // RESTART badge so the operator knows when a saved value bites.
    import styles from '../styles/engine-dashboard.module.css';

    type Apply = 'LIVE' | 'NEW_PIPELINES' | 'RESTART';

    let { source, apply }: { source: string; apply?: Apply } = $props();
</script>

<span class={styles.sourceChip}>
    <span class={styles.metaChip}>
        <span class={styles.metaChipLabel}>config</span>
        <span class={styles.metaChipValue}>{source}</span>
    </span>
    {#if apply}
        <span
            class="{styles.sourceApply} {apply === 'LIVE'
                ? styles.sourceApplyLive
                : apply === 'NEW_PIPELINES'
                  ? styles.sourceApplyNewPipelines
                  : styles.sourceApplyRestart}"
        >{apply}</span>
    {/if}
</span>
