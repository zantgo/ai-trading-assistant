<script lang="ts">
    // StrategyForm — recursive schema-driven form over one strategy JSON
    // node (v10.1). Same power as the raw JSON editor, typed controls:
    //   boolean → toggle · number → constrained input · string → select/text
    //   object → collapsible group · array → repeatable rows · null → SET
    import SvelteSelf from './StrategyForm.svelte';
    import { fieldMeta, humanLabel, type FieldMeta } from '../../lib/strategyFormSchema';
    import styles from './StrategyForm.module.css';

    interface Props {
        value: any;
        path: (string | number)[];
        label?: string | null;
        keyLabel?: string | null;
    }

    let { value, path, label = null, keyLabel = null }: Props = $props();

    const meta = $derived<FieldMeta | null>(fieldMeta(path));

    const kind = $derived.by(() => {
        if (value === null || value === undefined) return 'null';
        if (typeof value === 'boolean') return 'boolean';
        if (typeof value === 'number') return 'number';
        if (typeof value === 'string') return 'string';
        if (Array.isArray(value)) return 'array';
        return 'object';
    });

    const displayLabel = $derived.by(() => {
        if (label !== null && label !== '') return label;
        if (meta?.label) return meta.label;
        if (keyLabel) return humanLabel(keyLabel);
        return humanLabel(String(path[path.length - 1] ?? 'value'));
    });

    function addArrayItem() {
        if (!Array.isArray(value)) return;
        const sample = value[value.length - 1];
        if (typeof sample === 'object' && sample !== null && !Array.isArray(sample)) {
            value.push({ ...sample });
        } else if (typeof sample === 'number') {
            value.push(0);
        } else if (typeof sample === 'boolean') {
            value.push(false);
        } else {
            value.push('');
        }
    }

    function removeArrayItem(i: number) {
        if (Array.isArray(value)) value.splice(i, 1);
    }

    function defaultForNull(childPath: (string | number)[]): any {
        const m = fieldMeta(childPath);
        if (m?.options?.length) return m.options[0];
        if (m?.min !== undefined) return m.min;
        return '';
    }
</script>

{#if kind === 'boolean'}
    <div class={styles.fieldRow}>
        <label class={styles.fieldLabel} title={meta?.help ?? undefined}>
            <input type="checkbox" bind:checked={value} class={styles.checkbox} />
            <span>{displayLabel}</span>
        </label>
        {#if meta?.help}
            <p class={styles.fieldHelp}>{meta.help}</p>
        {/if}
    </div>
{:else if kind === 'number'}
    <div class={styles.fieldRow}>
        <span class={styles.fieldLabel} title={meta?.help ?? undefined}>
            <span>{displayLabel}</span>
            {#if meta?.unit}
                <span class={styles.fieldUnit}>{meta.unit}</span>
            {/if}
        </span>
        <input
            class={styles.input}
            type="number"
            min={meta?.min}
            max={meta?.max}
            step={meta?.step}
            bind:value={value}
        />
        {#if meta?.help}
            <p class={styles.fieldHelp}>{meta.help}</p>
        {/if}
    </div>
{:else if kind === 'string'}
    <div class={styles.fieldRow}>
        <span class={styles.fieldLabel} title={meta?.help ?? undefined}>
            <span>{displayLabel}</span>
            {#if meta?.unit}
                <span class={styles.fieldUnit}>{meta.unit}</span>
            {/if}
        </span>
        {#if meta?.options?.length}
            <select class={styles.input} bind:value={value}>
                {#each meta.options as opt (opt)}
                    <option value={opt}>{opt}</option>
                {/each}
            </select>
        {:else}
            <input class={styles.input} type="text" bind:value={value} />
        {/if}
        {#if meta?.help}
            <p class={styles.fieldHelp}>{meta.help}</p>
        {/if}
    </div>
{:else if kind === 'array'}
    <div class={styles.group}>
        <div class={styles.groupHeader}>
            <span class={styles.groupTitle}>{displayLabel}</span>
            <button class={styles.addBtn} onclick={addArrayItem} title="Add item">+ Add</button>
        </div>
        {#each value as item, i (i)}
            <div class={styles.arrayRow}>
                {#if typeof item === 'object' && item !== null && !Array.isArray(item)}
                    <details open>
                        <summary class={styles.arraySummary}>#{i + 1}</summary>
                        <SvelteSelf value={item} path={[...path, i]} />
                    </details>
                {:else}
                    <SvelteSelf value={item} path={[...path, i]} label="" keyLabel={`#${i + 1}`} />
                {/if}
                <button class={styles.removeBtn} onclick={() => removeArrayItem(i)} title="Remove">✕</button>
            </div>
        {/each}
        {#if value.length === 0}
            <p class={styles.fieldHelp}>empty array</p>
        {/if}
    </div>
{:else if kind === 'object'}
    <div class={styles.group}>
        <div class={styles.groupHeader}>
            <span class={styles.groupTitle}>{displayLabel}</span>
            <span class={styles.unsetBadge}>{Object.keys(value).length} fields</span>
        </div>
        <div class={styles.groupBody}>
            {#each Object.keys(value) as key (key)}
                {#if value[key] === null || value[key] === undefined}
                    <div class={styles.fieldRow}>
                        <span class={styles.fieldLabel}>
                            <span>{humanLabel(key)}</span>
                        </span>
                        <div style="display:flex; gap:8px; align-items:center">
                            <span class={styles.unsetBadge}>unset — inherits base</span>
                            <button class={styles.addBtn} onclick={() => (value[key] = defaultForNull([...path, key]))}>SET</button>
                        </div>
                    </div>
                {:else}
                    <SvelteSelf value={value[key]} path={[...path, key]} keyLabel={key} />
                {/if}
            {/each}
        </div>
    </div>
{/if}
