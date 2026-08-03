<script lang="ts">
    // PaneGroupHeader — collapsible accordion header used by LiveTerminal to
    // group secondary panels under labels like MOMENTUM OSCILLATORS,
    // VOLUME FLOW, etc. Click the header to expand/collapse the slot.
    //
    // Default state is provided by `defaultOpen` and snapshotted once into
    // local $state — subsequent toggling is purely user-driven and not
    // overwritten by the parent.
    import type { Snippet } from 'svelte';
    import styles from './PaneGroupHeader.module.css';

    let { title, count = 0, defaultOpen = false, children }: {
        title: string;
        count?: number;
        defaultOpen?: boolean;
        children?: Snippet;
    } = $props();

    // Intentional: `defaultOpen` is the *initial* expansion state for this
    // header. Subsequent toggling is purely user-driven and not overwritten
    // by the parent, so capturing the initial value is correct here.
    // svelte-ignore state_referenced_locally
    let open = $state(defaultOpen);
</script>

<div class={styles.group}>
    <button
        type="button"
        class={styles.header}
        aria-expanded={open}
        onclick={() => (open = !open)}
    >
        <span class={styles.title}>
            <span class="{styles.caret} {open ? styles.caretOpen : ''}">▶</span>
            {title}
        </span>
        <span class={styles.count}>{count}</span>
    </button>
    <div class="{styles.body} {open ? '' : styles.bodyCollapsed}">
        {@render children?.()}
    </div>
</div>
