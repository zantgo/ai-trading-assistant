<script lang="ts">
    import { useAppStore } from '../../state.svelte';

    const app = useAppStore();
    let { pairKey, regime, confidence }: {
        pairKey: string;
        regime: string | undefined;
        confidence: number | undefined;
    } = $props();

    interface RegimeRule {
        label: string;
        value: string;
        color: string;
    }

    const rules: Record<string, RegimeRule[]> = {
        TRENDING: [
            { label: 'Directional Entries', value: 'Long & Short (with-trend)', color: '#10b981' },
            { label: 'Prohibited', value: 'Counter-trend entries', color: '#ef4444' },
            { label: 'Max Allocation', value: '100% of base', color: '#10b981' },
            { label: 'Trend Weight', value: '×1.5', color: '#a855f7' },
            { label: 'Momentum Weight', value: '×1.0', color: '#94a3b8' },
            { label: 'Volume Gate', value: 'RVOL ≥ 1.0', color: '#f59e0b' },
            { label: 'Stop ATR Mult', value: '2.0×', color: '#94a3b8' },
            { label: 'TP Priority', value: '1.618 ext', color: '#4caf50' },
        ],
        COMPRESSION: [
            { label: 'Directional Entries', value: 'None (breakout prep only)', color: '#f59e0b' },
            { label: 'Prohibited', value: 'All aggressive entries', color: '#ef4444' },
            { label: 'Preferred', value: 'Breakout preparation, liquidity monitoring', color: '#10b981' },
            { label: 'Max Allocation', value: '25% of base', color: '#f59e0b' },
            { label: 'Volume Gate', value: 'RVOL ≥ 1.5 required', color: '#f59e0b' },
            { label: 'Stop ATR Mult', value: '1.5×', color: '#94a3b8' },
            { label: 'TP Priority', value: 'Nearest S/R', color: '#4caf50' },
        ],
        EXPANSION: [
            { label: 'Directional Entries', value: 'Long & Short (momentum)', color: '#10b981' },
            { label: 'Prohibited', value: 'Fading the expansion', color: '#ef4444' },
            { label: 'Max Allocation', value: '100% of base', color: '#10b981' },
            { label: 'Trend Weight', value: '×1.3', color: '#a855f7' },
            { label: 'Momentum Weight', value: '×1.2', color: '#a855f7' },
            { label: 'Volume Gate', value: 'RVOL ≥ 1.2', color: '#f59e0b' },
            { label: 'Stop ATR Mult', value: '2.5×', color: '#94a3b8' },
            { label: 'TP Priority', value: '2.618 ext', color: '#4caf50' },
        ],
        RANGE: [
            { label: 'Directional Entries', value: 'Long & Short (mean-reversion)', color: '#10b981' },
            { label: 'Prohibited', value: 'Trend-following trades', color: '#ef4444' },
            { label: 'Preferred', value: 'Mean reversion, S/R bounces', color: '#10b981' },
            { label: 'Max Allocation', value: '50% of base', color: '#f59e0b' },
            { label: 'Trend Weight', value: '×0.5', color: '#ef4444' },
            { label: 'Momentum Weight', value: '×1.5', color: '#a855f7' },
            { label: 'Volume Gate', value: 'RVOL ≥ 1.0', color: '#f59e0b' },
            { label: 'Stop ATR Mult', value: '1.0×', color: '#94a3b8' },
        ],
        TRANSITIONAL: [
            { label: 'Directional Entries', value: 'BLOCKED', color: '#ef4444' },
            { label: 'Prohibited', value: 'All directional trades', color: '#ef4444' },
            { label: 'Max Allocation', value: '0%', color: '#ef4444' },
            { label: 'Action', value: 'Wait for stabilization', color: '#f59e0b' },
            { label: 'Reason', value: '3+ regime indicators shifting', color: '#94a3b8' },
        ],
    };

    const currentRules = $derived(regime ? (rules[regime] ?? rules['RANGE']) : []);
    const regimeColor = $derived(
        regime === 'TRENDING' ? '#10b981' :
        regime === 'COMPRESSION' ? '#3b82f6' :
        regime === 'EXPANSION' ? '#a855f7' :
        regime === 'TRANSITIONAL' ? '#ef4444' : '#f59e0b'
    );
</script>

{#if regime}
<div class="gates-panel">
    <div class="panel-header">
        <span class="panel-title">REGIME STRATEGY GATES</span>
        <span class="panel-regime" style="color:{regimeColor}">{regime}</span>
        {#if confidence != null}
            <span class="panel-conf" style="color:{confidence >= 0.85 ? '#10b981' : confidence >= 0.60 ? '#f59e0b' : '#ef4444'}">
                {Math.round(confidence * 100)}%
            </span>
        {/if}
    </div>
    <div class="rules-list">
        {#each currentRules as rule}
            <div class="rule-row">
                <span class="rule-label">{rule.label}</span>
                <span class="rule-value" style="color:{rule.color}">{rule.value}</span>
            </div>
        {/each}
    </div>
</div>
{/if}

<style>
    .gates-panel {
        padding: 6px 8px;
        border: 1px solid var(--border-muted);
        border-radius: 6px;
        margin: 4px 0;
        background: var(--bg-card);
    }
    .panel-header {
        display: flex;
        align-items: center;
        gap: 6px;
        margin-bottom: 4px;
    }
    .panel-title {
        font-size: 10px;
        color: var(--text-muted);
        text-transform: uppercase;
        letter-spacing: 0.5px;
    }
    .panel-regime {
        font-size: 11px;
        font-weight: 600;
        font-family: var(--font-mono);
    }
    .panel-conf {
        font-size: 10px;
        font-family: var(--font-mono);
        margin-left: auto;
    }
    .rules-list {
        display: flex;
        flex-direction: column;
        gap: 1px;
    }
    .rule-row {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: 2px 4px;
        border-radius: 2px;
        background: var(--bg-dark);
        font-size: 9px;
        font-family: var(--font-mono);
    }
    .rule-label {
        color: var(--text-dim);
        min-width: 100px;
    }
    .rule-value {
        font-weight: 500;
        text-align: right;
    }
</style>
