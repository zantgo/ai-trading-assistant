<script lang="ts">
    import { useAppStore } from './state.svelte';
    import styles from './QuitDialog.module.css';
    import SvgIcon from './lib/SvgIcon.svelte';

    interface Props {
        onclose: () => void;
    }
    let { onclose }: Props = $props();

    const app = useAppStore();
    const QUIT_TIMEOUT_MS = 10_000;
    const ERROR_DISMISS_MS = 3_000;

    let confirming = $state(false);
    let errorMessage = $state<string | null>(null);
    let errorTimer: ReturnType<typeof setTimeout> | null = null;

    function surfaceError(msg: string) {
        errorMessage = msg;
        if (errorTimer) clearTimeout(errorTimer);
        errorTimer = setTimeout(() => {
            errorMessage = null;
            errorTimer = null;
        }, ERROR_DISMISS_MS);
    }

    function withTimeout<T>(p: Promise<T>, ms: number, reason: string): Promise<T> {
        return Promise.race([
            p,
            new Promise<never>((_, reject) =>
                setTimeout(() => reject(new Error(reason)), ms)),
        ]);
    }

    async function handleConfirm() {
        if (confirming) return;
        confirming = true;
        let hasError = false;
        try {
            const ok = await withTimeout(
                app.quitSession(),
                QUIT_TIMEOUT_MS,
                'Backend did not respond to /api/session/quit within 10 s.',
            );
            if (!ok) {
                surfaceError('Quit did not complete — instances may still be running.');
                hasError = true;
            }
        } catch (e: any) {
            surfaceError(`Quit failed: ${e?.message ?? 'unknown error'}`);
            hasError = true;
        } finally {
            confirming = false;
            // Show the error for ERROR_DISMISS_MS before closing so the
            // user actually reads it. On success close immediately.
            if (hasError) {
                setTimeout(onclose, ERROR_DISMISS_MS);
            } else {
                onclose();
            }
        }
    }
</script>

<div class={styles.quitOverlay} role="presentation" onclick={onclose} onkeydown={(e: KeyboardEvent) => { if (e.key === 'Escape') onclose(); }}>
    <div class={styles.quitDialog} role="dialog" aria-modal="true" tabindex="-1" onclick={(e: Event) => e.stopPropagation()} onkeydown={() => {}}>
        <div class={styles.quitIcon}>
            <SvgIcon name="info" size={44} />
        </div>
        <h2 class={styles.quitTitle}>Quit Application</h2>
        <p class={styles.quitMessage}>
            Are you sure you want to quit?<br />
            <strong>All instances will be deleted permanently</strong> and any open
            positions will be closed at the current market price. The next session
            starts with an empty workspace.
        </p>
        {#if errorMessage}
            <p class={styles.quitError} role="alert">{errorMessage}</p>
        {/if}
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