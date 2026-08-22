<script lang="ts">
    // StrategyEditorPanel (v9) — per-section JSON editing of one strategy.
    // The strategy JSON is the single source of truth; the editor edits
    // it as JSON (the exact format the CLI consumes), with a section tree
    // on the left, validation on save, and a full-JSON view.
    import { onMount } from 'svelte';
    import {
        fetchStrategies,
        fetchStrategyJson,
        saveStrategy,
        type StrategySummary,
    } from '../lib/api.svelte';
    import styles from './StrategyEditorPanel.module.css';
    import engine from '../styles/engine-dashboard.module.css';
    import SvgIcon from '../lib/SvgIcon.svelte';

    const SECTIONS: { key: string; label: string }[] = [
        { key: 'l1', label: 'L1 · Metrics' },
        { key: 'l1_5', label: 'L1.5 · Derivatives' },
        { key: 'l2', label: 'L2 · Alignment' },
        { key: 'l2_5', label: 'L2.5 · Liquidity Synthesis' },
        { key: 'l3', label: 'L3 · Analysis' },
        { key: 'l4', label: 'L4 · Opportunity' },
        { key: 'l5', label: 'L5 · Risk' },
        { key: 'l6', label: 'L6 · Decision' },
        { key: 'l7', label: 'L7 · Overview' },
        { key: 'tae', label: 'TAE · Execution' },
        { key: 'pme', label: 'PME · Portfolio' },
        { key: 'pae', label: 'PAE · Verdict' },
    ];

    interface Props {
        name?: string | null;
        onback?: () => void;
    }

    let { name = null, onback }: Props = $props();

    let strategies = $state<StrategySummary[]>([]);
    let selected = $state<string | null>((() => name)());
    let full = $state<Record<string, unknown> | null>(null);
    let section = $state<string>('l4');
    let sectionText = $state('');
    let viewFullJson = $state(false);
    let fullJsonText = $state('');
    let flash = $state<string | null>(null);
    let warnings = $state<string[]>([]);
    let busy = $state(false);
    let dirty = $state(false);

    async function loadStrategies() {
        strategies = await fetchStrategies();
        if (!selected && strategies.length > 0) selected = strategies[0].name;
    }

    async function loadStrategy(n: string) {
        const json = await fetchStrategyJson(n);
        full = json as Record<string, unknown>;
        fullJsonText = JSON.stringify(json, null, 2);
        selected = n;
        renderSection();
    }

    function renderSection() {
        if (!full) return;
        const value = (full as Record<string, unknown>)[section];
        sectionText = JSON.stringify(value, null, 2);
    }

    function switchSection(key: string) {
        // Persist the current section draft into the working object.
        applySectionDraft();
        section = key;
        renderSection();
    }

    function applySectionDraft() {
        if (!full || !dirty) return;
        try {
            (full as Record<string, unknown>)[section] = JSON.parse(sectionText);
        } catch {
            // keep the old value; validation flags it on save
        }
    }

    function syncFullJson() {
        if (!viewFullJson) return;
        try {
            full = JSON.parse(fullJsonText) as Record<string, unknown>;
            dirty = true;
        } catch {
            /* invalid draft — flagged on save */
        }
    }

    async function save() {
        applySectionDraft();
        if (viewFullJson) syncFullJson();
        if (!full || !selected) return;
        try {
            JSON.stringify(full);
        } catch {
            flash = 'The strategy JSON is invalid — fix it before saving.';
            return;
        }
        busy = true;
        const res = await saveStrategy(
            selected,
            full as Record<string, unknown>,
            (full.base as string | null) ?? null,
            (full.description as string | null) ?? null,
        );
        busy = false;
        flash = res.error ?? `Saved '${selected}' — running instances recharge at the next candle boundary.`;
        warnings = res.warnings ?? [];
        dirty = false;
        await loadStrategies();
    }

    function copyFullJson() {
        void navigator.clipboard?.writeText(fullJsonText);
    }

    onMount(async () => {
        await loadStrategies();
        const initial = name ?? strategies[0]?.name;
        if (initial) await loadStrategy(initial);
    });
</script>

<div class={styles.wrap}>
    <header class={engine.unifiedHeader}>
        <div class={engine.headerTop}>
            <div class={engine.titleGroup}>
                {#if onback}
                    <button class={engine.btn} onclick={onback} aria-label="Back to strategies">
                        <SvgIcon name="arrowLeft" size="sm" /> Back
                    </button>
                {/if}
                <h2 class={engine.title}>STRATEGY EDITOR</h2>
            </div>
            <div class={engine.headerRight}>
                <select class={engine.fieldInput} style="min-width:200px" bind:value={selected} onchange={() => void loadStrategy(selected!)}>
                    {#each strategies as s (s.name)}
                        <option value={s.name}>{s.name}</option>
                    {/each}
                </select>
                <button class="{engine.btn} {engine.btnPrimary}" disabled={!dirty || busy} onclick={() => void save()}>
                    <SvgIcon name="save" size="sm" /> {busy ? 'Saving…' : 'Save'}
                </button>
            </div>
        </div>
    </header>

    <div class={styles.content}>
        {#if flash}
            <div class="{engine.alertBanner} {warnings.length || flash.startsWith('Saved') ? '' : engine.alertError}" role="status">{flash}</div>
        {/if}
        {#if warnings.length > 0}
            <div class={engine.alertBanner}>
                Warnings:
                <ul style="margin:0.25rem 0 0 1.2rem">
                    {#each warnings as w (w)}
                        <li>{w}</li>
                    {/each}
                </ul>
            </div>
        {/if}

        {#if !full}
            <div class={engine.empty}>Select a strategy to edit.</div>
        {:else}
            <div class={styles.panes}>
                <nav class={styles.tree} aria-label="Strategy sections">
                    {#each SECTIONS as s (s.key)}
                        <button
                            class="{styles.treeItem} {section === s.key && !viewFullJson ? styles.treeActive : ''}"
                            onclick={() => { viewFullJson = false; switchSection(s.key); }}
                        >
                            {s.label}
                        </button>
                    {/each}
                    <button
                        class="{styles.treeItem} {viewFullJson ? styles.treeActive : ''}"
                        onclick={() => { applySectionDraft(); viewFullJson = true; fullJsonText = JSON.stringify(full, null, 2); }}
                    >
                        Full JSON
                    </button>
                </nav>

                <div class={styles.canvas}>
                    {#if viewFullJson}
                        <div class={styles.jsonToolbar}>
                            <span class={engine.infoLine}>The complete strategy JSON — exportable and CLI-compatible.</span>
                            <button class={engine.btn} onclick={copyFullJson}>Copy</button>
                        </div>
                        <textarea
                            class={styles.jsonArea}
                            spellcheck="false"
                            bind:value={fullJsonText}
                            onchange={syncFullJson}
                            aria-label="Full strategy JSON"
                        ></textarea>
                    {:else}
                        <h3 class={engine.cardTitle}>
                            {SECTIONS.find((s) => s.key === section)?.label ?? section}
                        </h3>
                        <p class={engine.infoLine}>
                            Edit this section as JSON — the exact format the CLI understands.
                            Missing keys inherit the base strategy; <code class={engine.code}>null</code> / empty = disabled.
                        </p>
                        <textarea
                            class={styles.jsonArea}
                            spellcheck="false"
                            bind:value={sectionText}
                            oninput={() => (dirty = true)}
                            aria-label="Section JSON"
                        ></textarea>
                    {/if}
                </div>
            </div>
        {/if}
    </div>
</div>
