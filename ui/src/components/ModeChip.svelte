<script lang="ts">
    // ModeChip — read-only execution-mode chip (observe / paper / live).
    // The mode is fixed at launch; this chip is display-only.
    import styles from '../styles/engine-dashboard.module.css';
    import { MODE_LABEL, type ExecutionMode } from '../lib/modePresentation';

    let { mode }: { mode: ExecutionMode | undefined } = $props();
</script>

{#if mode}
    <span
        class="{styles.modeChip} {mode === 'observe'
            ? styles.modeChipObserve
            : mode === 'paper'
              ? styles.modeChipPaper
              : styles.modeChipLive}"
    >{MODE_LABEL[mode]}</span>
{/if}
