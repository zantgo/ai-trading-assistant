<script lang="ts">
    // IndicatorStatusBadge — visual badge for the operational lifecycle of one
    // indicator (v6.5, 03-02-15). States: Loading (spinner + count) | Live
    // (green dot) | Stale (amber dot + age) | Failed (red icon + tooltip).
    //
    // Usage:
    //   <IndicatorStatusBadge status={tf.indicatorLifecycle?.[key]} />
    //
    // When `status` is undefined (legacy snapshot, pre-v6.5), the badge is
    // hidden — the value column falls back to its previous rendering.

    import type { IndicatorLifecycleStatus } from '../../types';
    import styles from './IndicatorStatusBadge.module.css';

    interface Props {
        status?: IndicatorLifecycleStatus;
    }

    let { status }: Props = $props();

    const label = $derived.by(() => {
        if (!status) return null;
        switch (status.state) {
            case 'Loading':
                return `Loading (${status.bars_seen}/${status.bars_required})`;
            case 'Live':
                return 'Live';
            case 'Stale': {
                const secs = status.last_updated_at
                    ? Math.max(0, Math.round((Date.now() - status.last_updated_at) / 1000))
                    : 0;
                return `Stale (${secs}s)`;
            }
            case 'Failed':
                return `Failed: ${status.last_error ?? 'unknown'}`;
        }
    });
</script>

{#if status && label}
    <span
        class="{styles.badge} {styles[status.state.toLowerCase()]}"
        title={status.last_error ?? label}
        aria-label={label}
    >
        {#if status.state === 'Loading'}
            <span class={styles.spinner}></span>
        {:else if status.state === 'Live'}
            <span class={styles.dot}></span>
        {:else if status.state === 'Stale'}
            <span class={styles.dot}></span>
        {:else}
            <span class={styles.icon}>!</span>
        {/if}
        <span class={styles.text}>{label}</span>
    </span>
{/if}