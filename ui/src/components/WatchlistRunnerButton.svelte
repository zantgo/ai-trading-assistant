<script lang="ts">
    // WatchlistRunnerButton — SCAN WATCHLIST CTA in `GeneralDashboard`.
    // Renders as a compact inline pill immediately to the right of
    // `<SnapshotSchedulerButton />` inside the unified bottom toolbar
    // (`GeneralDashboard.module.css` `.runnerBar` / `.actions`). The
    // descriptive caption lives in the toolbar's right edge, not here.
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

<WatchlistScannerModal
    {isOpen}
    {wssMap}
    onclose={() => (isOpen = false)}
/>
