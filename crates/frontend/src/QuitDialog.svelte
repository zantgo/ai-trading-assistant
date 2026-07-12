<script lang="ts">
    import { useAppStore } from './state.svelte';
    import styles from './QuitDialog.module.css';

    interface Props {
        onclose: () => void;
    }
    let { onclose }: Props = $props();

    const app = useAppStore();
    let confirming = $state(false);

    async function handleConfirm() {
        confirming = true;
        await app.quitSession();
        confirming = false;
        onclose();
    }
</script>

<div class={styles.quitOverlay} role="presentation" onclick={onclose} onkeydown={(e: KeyboardEvent) => { if (e.key === 'Escape') onclose(); }}>
    <div class={styles.quitDialog} role="dialog" aria-modal="true" tabindex="-1" onclick={(e: Event) => e.stopPropagation()} onkeydown={() => {}}>
        <div class={styles.quitIcon}>
            <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="#ef4444" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"></path>
                <line x1="12" y1="9" x2="12" y2="13"></line>
                <line x1="12" y1="17" x2="12.01" y2="17"></line>
            </svg>
        </div>
        <h2 class={styles.quitTitle}>Quit Application</h2>
        <p class={styles.quitMessage}>
            Are you sure you want to quit?<br />
            <strong>All running workspaces will be terminated</strong> and any open
            positions will be closed at the current market price.
        </p>
        <div class={styles.quitActions}>
            <button class={styles.quitCancel} onclick={onclose} disabled={confirming}>
                Cancel
            </button>
            <button class={styles.quitConfirm} onclick={handleConfirm} disabled={confirming}>
                {#if confirming}
                    <span class={styles.spinner}></span>
                    Shutting down...
                {:else}
                    Quit
                {/if}
            </button>
        </div>
    </div>
</div>
