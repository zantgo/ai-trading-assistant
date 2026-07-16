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
            <svg width="44" height="44" viewBox="0 0 24 24" fill="none" stroke="rgba(255,255,255,0.5)" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round">
                <circle cx="12" cy="12" r="10"/>
                <path d="M12 8v4"/>
                <circle cx="12" cy="16" r="0.5" fill="rgba(255,255,255,0.5)" stroke="none"/>
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
