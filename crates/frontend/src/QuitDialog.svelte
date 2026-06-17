<script lang="ts">
    import { getState } from './state.svelte';

    interface Props {
        onclose: () => void;
    }
    let { onclose }: Props = $props();

    const app = getState();
    let confirming = $state(false);

    async function handleConfirm() {
        confirming = true;
        await app.quitSession();
        confirming = false;
        onclose();
    }
</script>

<div class="quit-overlay" onclick={onclose} onkeydown={(e: KeyboardEvent) => { if (e.key === 'Escape') onclose(); }}>
    <div class="quit-dialog" onclick={(e: Event) => e.stopPropagation()}>
        <div class="quit-icon">⚠️</div>
        <h2 class="quit-title">Quit Application</h2>
        <p class="quit-message">
            Are you sure you want to quit?<br />
            <strong>All running instances will be terminated</strong> and any open
            positions will be closed at the current market price.
        </p>
        <div class="quit-actions">
            <button class="quit-cancel" onclick={onclose} disabled={confirming}>
                Cancel
            </button>
            <button class="quit-confirm" onclick={handleConfirm} disabled={confirming}>
                {#if confirming}
                    <span class="spinner"></span>
                    Shutting down...
                {:else}
                    Quit
                {/if}
            </button>
        </div>
    </div>
</div>

<style>
    .quit-overlay {
        position: fixed;
        inset: 0;
        background: rgba(0, 0, 0, 0.6);
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: 1000;
        backdrop-filter: blur(4px);
    }
    .quit-dialog {
        background: #1a1a2e;
        border: 1px solid #2a2a4a;
        border-radius: 12px;
        padding: 2rem;
        width: 100%;
        max-width: 400px;
        box-shadow: 0 8px 40px rgba(0, 0, 0, 0.5);
        text-align: center;
    }
    .quit-icon {
        font-size: 2.5rem;
        margin-bottom: 0.5rem;
    }
    .quit-title {
        color: #e0e0ff;
        font-size: 1.2rem;
        margin: 0 0 0.75rem 0;
    }
    .quit-message {
        color: #aaaacc;
        font-size: 0.85rem;
        line-height: 1.5;
        margin: 0 0 1.5rem 0;
    }
    .quit-message strong {
        color: #ff9999;
    }
    .quit-actions {
        display: flex;
        gap: 0.75rem;
        justify-content: center;
    }
    .quit-cancel {
        padding: 0.6rem 1.5rem;
        background: #333355;
        border: 1px solid #444466;
        border-radius: 8px;
        color: #aaaacc;
        font-size: 0.9rem;
        cursor: pointer;
        transition: background 0.2s;
    }
    .quit-cancel:hover:not(:disabled) {
        background: #3a3a5a;
    }
    .quit-confirm {
        padding: 0.6rem 1.5rem;
        background: #cc3333;
        border: none;
        border-radius: 8px;
        color: white;
        font-size: 0.9rem;
        font-weight: 600;
        cursor: pointer;
        transition: opacity 0.2s;
        display: flex;
        align-items: center;
        gap: 0.5rem;
    }
    .quit-confirm:hover:not(:disabled) {
        opacity: 0.85;
    }
    .quit-cancel:disabled,
    .quit-confirm:disabled {
        opacity: 0.5;
        cursor: not-allowed;
    }
    .spinner {
        width: 14px;
        height: 14px;
        border: 2px solid rgba(255,255,255,0.3);
        border-top-color: white;
        border-radius: 50%;
        animation: spin 0.6s linear infinite;
    }
    @keyframes spin {
        to { transform: rotate(360deg); }
    }
</style>
