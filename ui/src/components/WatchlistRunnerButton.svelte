<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import type { WsState } from '../lib/websocket.svelte';
    import WatchlistScannerModal from './WatchlistScannerModal.svelte';
    import SvgIcon from '../lib/SvgIcon.svelte';
    import styles from './WatchlistRunnerButton.module.css';

    interface Props {
        wssMap: Record<string, WsState>;
    }

    let { wssMap }: Props = $props();

    const app = useAppStore();
    let isOpen = $state(false);

    const sessionReady = $derived(app.sessionActive);
</script>

<div class={styles.runnerBar}>
    <button
        class={styles.runnerBtn}
        onclick={() => (isOpen = true)}
        disabled={!sessionReady}
        title={sessionReady
            ? 'Add a watchlist of pairs and keep only those with a clear decision'
            : 'Start a session first'}
    >
        <span class={styles.runnerBtnIcon}>
            <SvgIcon name="search" size={14} />
        </span>
        Scan Watchlist
    </button>
    <span class={styles.runnerBtnHint}>
        Add a basket of pairs and keep only those with a clear decision.
    </span>
</div>

<WatchlistScannerModal
    {isOpen}
    {wssMap}
    onclose={() => (isOpen = false)}
/>
