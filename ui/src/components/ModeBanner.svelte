<script lang="ts">
    // ModeBanner — full-width mode strip under the dashboard header.
    // Amber = observe, blue = paper, red = live; copy is engine-specific.
    import styles from '../styles/engine-dashboard.module.css';
    import { modeBannerCopy, type ExecutionMode, type ModeEngine } from '../lib/modePresentation';

    let { engine, mode }: { engine: ModeEngine; mode: ExecutionMode | undefined } = $props();
</script>

{#if mode}
    <div
        class="{styles.modeBanner} {mode === 'observe'
            ? styles.modeBannerObserve
            : mode === 'paper'
              ? styles.modeBannerPaper
              : styles.modeBannerLive}"
    >{modeBannerCopy(engine, mode)}</div>
{/if}
