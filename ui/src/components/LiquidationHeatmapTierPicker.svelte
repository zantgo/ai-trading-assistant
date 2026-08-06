<!--
    LiquidationHeatmapTierPicker — chip + integer-stepper UI for the
    operator's currently-highlighted leverage tiers (each ∈ [1, 100]).

    Contract:
      • `tiers: number[]` — current set (sorted ascending, deduped).
      • `onChange: (next: number[]) => void` — fires whenever the set
        changes (chip removal or add). The parent owns persistence.

    Rules enforced in code:
      • Integer-only inputs (no fractional leverage).
      • Range [1, 100].
      • Reject duplicates (silent no-op).
      • Default seed: [10].
-->
<script lang="ts">
    import styles from './LiquidationHeatmapTierPicker.module.css';

    interface Props {
        tiers: number[];
        onChange: (next: number[]) => void;
        min?: number;
        max?: number;
    }

    let { tiers, onChange, min = 1, max = 100 }: Props = $props();

    let draft = $state<string>('');
    let inputEl: HTMLInputElement | null = $state(null);

    const sortedTiers = $derived(
        Array.from(new Set(tiers ?? [])).sort((a, b) => a - b)
    );

    function removeTier(t: number) {
        const next = sortedTiers.filter((x) => x !== t);
        onChange(next);
    }

    function addTier(raw: unknown) {
        // The `bind:value` on `<input type="number">` stores a number
        // (numeric) or empty string into the proxy. Normalize so the
        // strict-integer regex match below always sees a string.
        const trimmed = String(raw ?? '').trim();
        if (trimmed === '') return;
        // Strict integer parse: reject anything with decimal point or
        // exponent — matches D6 (only integers).
        if (!/^-?\d+$/.test(trimmed)) {
            draft = '';
            return;
        }
        const v = parseInt(trimmed, 10);
        if (!Number.isFinite(v) || v < min || v > max) {
            draft = '';
            return;
        }
        const present = sortedTiers.includes(v);
        if (present) {
            // Dedup: silently no-op.
            draft = '';
            return;
        }
        onChange([...sortedTiers, v]);
        draft = '';
    }

    function handleAdd() {
        addTier(draft);
    }

    function handleKeydown(ev: KeyboardEvent) {
        if (ev.key === 'Enter') {
            ev.preventDefault();
            addTier(draft);
        }
    }
</script>

<div class={styles.picker}>
    <div class={styles.chips}>
        {#each sortedTiers as t (t)}
            <span class={styles.chip}>
                <span class={styles.chipLabel}>{t}×</span>
                <button
                    type="button"
                    class={styles.remove}
                    aria-label="Remove {t}x tier"
                    onclick={() => removeTier(t)}
                >×</button>
            </span>
        {/each}
        {#if sortedTiers.length === 0}
            <span class={styles.empty}>No tiers selected — heatmap shows every cluster equally.</span>
        {/if}
    </div>

    <div class={styles.addRow}>
        <input
            type="number"
            class={styles.input}
            bind:value={draft}
            bind:this={inputEl}
            min={min}
            max={max}
            step={1}
            inputmode="numeric"
            placeholder="e.g. 25"
            aria-label="Add leverage tier (integer between {min} and {max})"
            onkeydown={handleKeydown}
        />
        <button
            type="button"
            class={styles.addBtn}
            onclick={handleAdd}
            disabled={String(draft).trim() === ''}
        >ADD</button>
    </div>

    <p class={styles.hint}>
        Integer leverage × in [{min}, {max}]. Matching clusters intensify, the rest dim.
    </p>
</div>
