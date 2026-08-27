<script lang="ts">
    // StrategiesHome (v9) — the Profile → Strategies surface: list + editor.
    import StrategyListPanel from './StrategyListPanel.svelte';
    import StrategyEditorPanel from './StrategyEditorPanel.svelte';

    let editing = $state<string | null>(null);
</script>

{#if editing}
    <StrategyEditorPanel name={editing} onback={() => (editing = null)} />
{:else}
    <StrategyListPanel onedit={(n) => (editing = n)} />
{/if}
