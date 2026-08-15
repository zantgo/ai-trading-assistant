<script lang="ts">
    // SnapshotSchedulerButton — bottom-left CTA in `GeneralDashboard`.
    // Renders immediately to the left of `<WatchlistRunnerButton />`
    // in the unified bottom toolbar (see `GeneralDashboard.module.css`'s
    // `.runnerBar` / `.actions` containers).
    //
    // Visual: a horizontal inline pill identical in height, padding and
    // radius to the SCAN WATCHLIST button, with the status indicator
    // (`● ON · 12s ago` / `● OFF` / red `ERROR`) as a trailing label
    // separated by a bullet.
    //
    // Behaviour:
    // - Always polls the snapshot-export status (3s) so the status
    //   pill stays fresh even when the modal is closed.
    // - Click opens `<SnapshotSchedulerModal />` which owns the
    //   configuration form. The modal also takes over polling on
    //   open (the button's polling stops while the modal is mounted
    //   to avoid double-tap).
    import { onDestroy, onMount } from 'svelte';
    import { useAppStore } from '../state.svelte';
    import { formatRelativeTime } from '../lib/relTime';
    import styles from './SnapshotSchedulerButton.module.css';
    import SnapshotSchedulerModal from './SnapshotSchedulerModal.svelte';
    import brutalistStyles from '../styles/brutalist-grid.module.css';

    const app = useAppStore();
    let modalOpen = $state(false);

    let tick = $state(0);
    onMount(() => {
        app.startSnapshotExportPolling(3000);
        const id = setInterval(() => { tick = tick + 1; }, 1000);
        return () => clearInterval(id);
    });
    onDestroy(() => {
        app.stopSnapshotExportPolling();
    });

    function openModal() {
        modalOpen = true;
        app.stopSnapshotExportPolling();
        app.fetchSnapshotExportStatus();
    }

    function closeModal() {
        modalOpen = false;
        app.startSnapshotExportPolling(3000);
    }

    const status = $derived(app.snapshotExportStatus);

    type Pill = { dot: 'green' | 'amber' | 'red' | 'gray'; label: string };
    const pill = $derived.by((): Pill => {
        // tick is read for reactivity — `lastSnapshotExportFetchMs` is
        // also tracked via status being non-null.
        void tick;
        const s = status;
        if (!s) return { dot: 'gray', label: 'OFF …' };
        if (!s.enabled) return { dot: 'gray', label: 'OFF' };
        if (s.last_error) return { dot: 'red', label: 'ERROR' };
        const lastMs = s.last_snapshot_at ? Date.parse(s.last_snapshot_at) : null;
        if (lastMs != null) {
            const friendly = formatRelativeTime(lastMs).label;
            return { dot: 'green', label: `ON · ${friendly}` };
        }
        return { dot: 'amber', label: `ON · ${s.interval_secs}s` };
    });

    function pillClass(dot: Pill['dot']): string {
        return dot === 'green' ? styles.dotGreen
             : dot === 'amber' ? styles.dotAmber
             : dot === 'red' ? styles.dotRed
             : styles.dotGray;
    }
</script>

<button
    class={styles.cta}
    onclick={openModal}
    aria-label="Configure snapshot export schedule"
    type="button"
>
    <span class={styles.title}>SCHEDULE SNAPSHOTS</span>
    <span class={styles.sep} aria-hidden="true">•</span>
    <span class={styles.pill}>
        <span class="{styles.dot} {pillClass(pill.dot)}"></span>
        <span class={styles.pillLabel}>{pill.label}</span>
    </span>
</button>

{#if modalOpen}
    <div class={brutalistStyles.confirmOverlay} role="presentation" onclick={closeModal}>
        <div
            class="{brutalistStyles.confirmDialog} {styles.dialogShell}"
            role="dialog"
            aria-modal="true"
            tabindex="-1"
            onclick={(e) => e.stopPropagation()}
            onkeydown={(e) => e.stopPropagation()}
        >
            <SnapshotSchedulerModal onclose={closeModal} />
        </div>
    </div>
{/if}
