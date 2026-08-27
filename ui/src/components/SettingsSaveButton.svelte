<script lang="ts">
    // SettingsSaveButton — the canonical save control for every editable
    // settings panel. One state machine, one placement (panel header right
    // side, immediately before the Export button):
    //   idle   → disabled
    //   dirty  → enabled, "SAVE"
    //   saving → disabled, "SAVING…"
    //   saved  → disabled, "SAVED" (green, auto-reverts to idle)
    //   error  → enabled, "SAVE" (retry; the panel shows the error banner)
    // The button is never clickable unless dirty/error, never while saving,
    // and never right after a successful save.
    import styles from '../styles/engine-dashboard.module.css';

    export type SettingsSaveState = 'idle' | 'dirty' | 'saving' | 'saved' | 'error';

    let { state, onsave }: {
        state: SettingsSaveState;
        onsave: () => void;
    } = $props();

    function handleClick() {
        if (state !== 'dirty' && state !== 'error') return;
        onsave();
    }
</script>

<span class={styles.saveStatus}>
    {#if state === 'dirty'}
        Unsaved changes
    {:else if state === 'saving'}
        Saving…
    {:else if state === 'saved'}
        All changes saved
    {/if}
</span>
<button
    type="button"
    class="{styles.btn} {styles.btnPrimary} {styles.saveBtn} {state === 'saved' ? styles.saveBtnSaved : state === 'saving' ? styles.saveBtnSaving : ''}"
    disabled={state !== 'dirty' && state !== 'error'}
    onclick={handleClick}
>
    {state === 'saving' ? 'SAVING…' : state === 'saved' ? 'SAVED' : 'SAVE'}
</button>
