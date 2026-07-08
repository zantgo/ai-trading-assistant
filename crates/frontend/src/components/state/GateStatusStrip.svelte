<script lang="ts">
    import { iRaw } from '../../lib/telemetry';
    import type { IndicatorMap } from '../../types';
    import { useAppStore } from '../../state.svelte';

    const app = useAppStore();
    let { pairKey }: { pairKey: string } = $props();
    const pair = $derived(app.instancesMap[pairKey]);
    const tf = $derived(pair?.microTerm);
    const snap = $derived(tf?.latestSnapshot);
    const indicators = $derived((snap?.indicators ?? {}) as IndicatorMap);

    interface GateInfo {
        name: string;
        key: string;
        value: number;
        label: string;
        multiplier: number;
    }

    function computeGateMultiplier(key: string): number {
        const ind = indicators[key];
        if (!ind) return 1.0;
        const v = ind.raw_value ?? 0;
        const label = ind.state_label ?? '';
        switch (key) {
            case 'adx': return v < 20 || label.includes('CONGESTION') ? 0.6 : 1.0;
            case 'choppiness': return v >= 61.8 ? 0.5 : v <= 38.2 ? 1.0 : 0.85;
            case 'atr': return label.includes('CONTRACTING') ? 0.8 : 1.0;
            case 'bbwp': return v < 10 ? 0.5 : v > 90 ? 0.4 : 1.0;
            case 'hv': return v > 100 ? 0.6 : v > 60 ? 0.8 : v < 20 ? 0.9 : 1.0;
            case 'volume': return 1.0;
            case 'rvol': return v < 1.0 ? 0.5 : v >= 3.0 ? 0.3 : 1.0;
            default: return 1.0;
        }
    }

    function gateStatus(mult: number): string {
        if (mult >= 1.0) return 'OPEN';
        if (mult >= 0.7) return 'CAUTION';
        if (mult >= 0.4) return 'RESTRICT';
        return 'BLOCKED';
    }

    function statusColor(status: string): string {
        switch (status) {
            case 'OPEN': return '#10b981';
            case 'CAUTION': return '#f59e0b';
            case 'RESTRICT': return '#ef4444';
            case 'BLOCKED': return '#7f1d1d';
            default: return '#94a3b8';
        }
    }

    const gates = $derived(
        ['adx', 'choppiness', 'atr', 'bbwp', 'hv', 'volume', 'rvol'].map(k => {
            const mult = computeGateMultiplier(k);
            return {
                name: k.toUpperCase(),
                key: k,
                value: indicators[k]?.raw_value ?? 0,
                label: indicators[k]?.state_label ?? '—',
                multiplier: mult,
            };
        })
    );

    const effectiveGate = $derived(gates.reduce((p, g) => p * g.multiplier, 1.0));
</script>

{#if pair}
<div class="gate-strip">
    <div class="gate-title">CONFLUENCE GATES</div>
    <div class="gate-pills">
        {#each gates as g}
            {@const st = gateStatus(g.multiplier)}
            <span class="gate-pill" style="border-color:{statusColor(st)}; color:{statusColor(st)}" title="{g.name}: {g.label} (×{g.multiplier.toFixed(2)})">
                {g.name} ×{g.multiplier.toFixed(2)}
            </span>
        {/each}
    </div>
    <div class="gate-effective" style="color:{statusColor(gateStatus(effectiveGate))}">
        EFFECTIVE: ×{effectiveGate.toFixed(3)}
    </div>
</div>
{/if}

<style>
    .gate-strip {
        padding: 6px 8px;
        border: 1px solid var(--border-muted);
        border-radius: 6px;
        margin: 4px 0;
        background: var(--bg-card);
    }
    .gate-title {
        font-size: 10px;
        color: var(--text-muted);
        text-transform: uppercase;
        letter-spacing: 0.5px;
        margin-bottom: 4px;
    }
    .gate-pills {
        display: flex;
        flex-wrap: wrap;
        gap: 4px;
    }
    .gate-pill {
        font-size: 9px;
        font-family: var(--font-mono);
        padding: 1px 5px;
        border: 1px solid;
        border-radius: 3px;
        white-space: nowrap;
    }
    .gate-effective {
        font-size: 10px;
        font-family: var(--font-mono);
        margin-top: 4px;
        text-align: right;
    }
</style>
