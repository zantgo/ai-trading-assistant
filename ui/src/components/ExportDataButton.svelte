<script lang="ts">
    // ExportDataButton — shared "copy JSON to clipboard" control.
    //
    // Visual style mirrors TerminalMonitor's legacy .exportBtn so it slots
    // into any panel header without per-panel overrides:
    //   • uppercase 10px mono label
    //   • subtle dark fill, light border, hover-bright
    //   • copy icon leading the label
    //   • margin-left:auto so it sits at the far right when placed inside a
    //     flex row (`.panelHeader`, `.header`, `.head`, etc.)
    //
    // Behaviour is fully delegated: the parent supplies an async `onExport`
    // that returns true on success / false on failure. The button reflects
    // the result via the temporary "Copied!" / "Copy failed" caption that
    // auto-resets after 2 seconds.

    import { copyJsonToClipboard } from '../lib/metricsExport';
    import styles from './ExportDataButton.module.css';

    interface Props {
        /** Async builder that returns a JSON string ready for the clipboard. */
        onExport: () => Promise<string> | string | null | undefined;
        /** Tooltip shown on hover. */
        title?: string;
        /** Disable the button (e.g. before data is available). */
        disabled?: boolean;
        /** Optional caption override (default: "EXPORT DATA"). */
        label?: string;
    }

    let {
        onExport,
        title = 'Copy all data on this tab as JSON',
        disabled = false,
        label = 'EXPORT DATA',
    }: Props = $props();

    // Intentional: `label` is the *initial* caption. The `$effect` below
    // tracks `label` reactively to reset the caption when it changes
    // (skipping transient feedback states), so the local `caption` only
    // needs the initial value here. Capturing the initial value is correct.
    // svelte-ignore state_referenced_locally
    let caption = $state<string>(label);
    let resetTimer: ReturnType<typeof setTimeout> | null = null;

    $effect(() => {
        // Reset caption whenever the requested label changes (skipping the
        // transient feedback states so a fast label swap doesn't visually
        // fight with a freshly-issued "Copied!" / "Copy failed").
        if (caption !== 'Copied!' && caption !== 'Copy failed' && caption !== label) {
            caption = label;
        }
    });

    async function handleClick() {
        try {
            const text = await onExport();
            if (text == null) {
                caption = 'Copy failed';
            } else {
                const ok = await copyJsonToClipboard(text);
                caption = ok ? 'Copied!' : 'Copy failed';
            }
        } catch (_) {
            caption = 'Copy failed';
        }
        if (resetTimer) clearTimeout(resetTimer);
        resetTimer = setTimeout(() => { caption = label; }, 2000);
    }
</script>

<button
    type="button"
    class={styles.exportBtn}
    onclick={handleClick}
    {title}
    {disabled}
>
    <svg
        class={styles.icon}
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="1.6"
        stroke-linecap="round"
        stroke-linejoin="round"
        aria-hidden="true"
    >
        <rect x="9" y="9" width="11" height="11" rx="2"></rect>
        <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
    </svg>
    <span class={styles.label}>{caption}</span>
</button>