<script lang="ts">
    import { useAppStore } from '../../state.svelte';
    import type { IndicatorMeta } from '../../types';
    import styles from './settings.module.css';

    const app = useAppStore();
    const registry = $derived<IndicatorMeta[]>((app.indicatorRegistry ?? []) as IndicatorMeta[]);
    const directional = $derived(registry.filter((m) => m.directional));

    // Local editable state seeded from registry defaults.
    let weights = $state<Record<string, number>>({});
    let enabled = $state<Record<string, boolean>>({});
    let status = $state<'idle' | 'saving' | 'success' | 'error'>('idle');
    let seeded = false;

    $effect(() => {
        if (seeded || directional.length === 0) return;
        for (const m of directional) {
            if (!(m.key in weights)) weights[m.key] = m.default_weight;
            if (!(m.key in enabled)) enabled[m.key] = m.default_enabled;
        }
        seeded = true;
    });

    const GROUPS = ['Trend', 'Momentum', 'Volume', 'Volatility', 'Structure', 'Regime'];

    async function save() {
        status = 'saving';
        try {
            const res = await fetch('/api/config/scoring-weights', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ indicator_weights: weights, indicator_enabled: enabled }),
            });
            status = res.ok ? 'success' : 'error';
        } catch (_) {
            status = 'error';
        }
        if (status === 'success') setTimeout(() => (status = 'idle'), 1500);
    }

    function resetDefaults() {
        for (const m of directional) {
            weights[m.key] = m.default_weight;
            enabled[m.key] = m.default_enabled;
        }
    }
</script>

<div class={styles.settingGroupBox}>
    <span class={styles.selectorsLabel}>Confluence Scoring Weights (directional indicators)</span>
    <div class={styles.scoringGrid}>
        {#each GROUPS as group}
            {@const rows = directional.filter((m) => m.group === group)}
            {#if rows.length > 0}
                <div class={styles.scoringGroupHead}>{group}</div>
                {#each rows as m}
                    <div class={styles.scoringRow}>
                        <button
                            class="{styles.scoringToggle} {enabled[m.key] ? styles.on : styles.off}"
                            onclick={() => (enabled[m.key] = !enabled[m.key])}
                            title={enabled[m.key] ? 'Enabled' : 'Disabled'}
                        >{enabled[m.key] ? '●' : '○'}</button>
                        <span class={styles.scoringName}>{m.display_name}</span>
                        <input
                            class={styles.scoringWeight}
                            type="number"
                            step="0.1"
                            min="0"
                            bind:value={weights[m.key]}
                            disabled={!enabled[m.key]}
                        />
                    </div>
                {/each}
            {/if}
        {/each}
    </div>
    <div class={styles.scoringActions}>
        <button class={styles.scoringReset} onclick={resetDefaults}>Reset defaults</button>
        <button class={styles.scoringSave} onclick={save} disabled={status === 'saving'}>
            {status === 'saving' ? 'Saving…' : status === 'success' ? 'Saved ✓' : status === 'error' ? 'Error' : 'Save Weights'}
        </button>
    </div>
</div>
