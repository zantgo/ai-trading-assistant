<script lang="ts">
    import styles from './FullscreenToolbar.module.css';

    interface Props {
        onScreenshot: () => void;
        onClose: () => void;
    }

    let { onScreenshot, onClose }: Props = $props();
</script>

<div class={styles.toolbar} role="toolbar" aria-label="Fullscreen chart controls">
    <button
        type="button"
        class={styles.screenshotBtn}
        onclick={onScreenshot}
        title="Download chart as PNG"
        aria-label="Download chart as PNG"
    >
        Screenshot
    </button>
    <button
        type="button"
        class={styles.closeBtn}
        onclick={onClose}
        title="Close"
        aria-label="Close fullscreen"
    >
        ✕
    </button>
</div>
