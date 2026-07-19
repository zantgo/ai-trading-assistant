<script lang="ts">
    import SvgIcon from '../../lib/SvgIcon.svelte';
    import styles from '../../styles/brutalist-grid.module.css';

    interface Props {
        action: 'start' | 'pause' | 'stop' | 'delete';
        id: string;
        pair?: string;
        displaySymbol: string;
        oncancel: () => void;
        onconfirm: () => void;
    }

    let { action, id, pair, displaySymbol, oncancel, onconfirm }: Props = $props();

    const actionLabels: Record<string, string> = { start: 'Start', pause: 'Pause', stop: 'Stop', delete: 'Delete' };
    const actionLabel = $derived(actionLabels[action] ?? action);
    const isDelete = $derived(action === 'delete');
</script>

<div class={styles.confirmOverlay} role="presentation" onclick={oncancel}>
    <div class={styles.confirmDialog} role="dialog" aria-modal="true" tabindex="-1" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()}>
        <div class={styles.confirmIcon}>
            {#if isDelete}
                <SvgIcon name="x" size={32} />
            {:else}
                <SvgIcon name="info" size={32} />
            {/if}
        </div>
        <h2 class={styles.confirmTitle}>{actionLabel} {displaySymbol}?</h2>
        <p class={styles.confirmText}>
            {#if isDelete}
                This will permanently delete <strong>{displaySymbol}</strong> and all associated data.
            {:else if action === 'start'}
                This will start the <strong>{displaySymbol}</strong> instance.
            {:else if action === 'stop'}
                This will stop the <strong>{displaySymbol}</strong> instance.
            {:else}
                This will pause the <strong>{displaySymbol}</strong> instance. It can be resumed later.
            {/if}
        </p>
        <div class={styles.confirmActions}>
            <button class={styles.confirmCancelBtn} onclick={oncancel}>Cancel</button>
            <button class={styles.confirmDangerBtn} onclick={onconfirm}
                style={isDelete ? 'background:#ef5350;color:#fff;border:none' : ''}>
                {actionLabel}
            </button>
        </div>
    </div>
</div>
