<script lang="ts">
    import type { IndicatorMap, GroupSummary } from '../../types';
    import { useAppStore } from '../../state.svelte';

    const app = useAppStore();
    let { pairKey }: { pairKey: string } = $props();
    const pair = $derived(app.instancesMap[pairKey]);
    const tf = $derived(pair?.microTerm);
    const snap = $derived(tf?.latestSnapshot);
    const indicators = $derived((snap?.indicators ?? {}) as IndicatorMap);

    interface GroupAgg {
        group: string;
        dominant: string;
        domColor: string;
        confirmed: number;
        active: number;
        potential: number;
        confidence: number;
        consensus: number;
    }

    const groups: GroupAgg[] = $derived(
        ['Trend', 'Momentum', 'Volume', 'Volatility', 'Structure', 'Regime', 'Institutional'].map(group => {
            const dirSum = Object.entries(indicators).reduce((s, [k, v]) => {
                const meta = (app.indicatorRegistry as any[])?.find((m: any) => m.key === k && m.group === group);
                if (!meta || !meta.directional) return s;
                return s + (v?.normalized ?? 0) * (v?.confidence ?? 0);
            }, 0);
            const count = Object.entries(indicators).filter(([k]) => {
                const meta = (app.indicatorRegistry as any[])?.find((m: any) => m.key === k && m.group === group && m.directional);
                return !!meta;
            }).length;
            const dom = dirSum > 0.05 ? '▲' : dirSum < -0.05 ? '▼' : '·';
            const domColor = dirSum > 0.05 ? '#10b981' : dirSum < -0.05 ? '#ef4444' : '#94a3b8';
            const confSum = Object.entries(indicators).reduce((s, [k, v]) => {
                const meta = (app.indicatorRegistry as any[])?.find((m: any) => m.key === k && m.group === group && m.directional);
                return meta ? s + (v?.confidence ?? 0) : s;
            }, 0);
            const sigs = Object.entries(indicators).filter(([k]) => {
                const meta = (app.indicatorRegistry as any[])?.find((m: any) => m.key === k && m.group === group && m.directional);
                return !!meta;
            });
            let confirmed = 0, active = 0, potential = 0;
            sigs.forEach(([, v]) => {
                const signals = (v as any).signals as any[] | undefined;
                if (!signals) return;
                signals.forEach((s: any) => {
                    if (s.status === 'Confirmed') confirmed++;
                    else if (s.status === 'Active') active++;
                    else if (s.status === 'Potential') potential++;
                });
            });
            return {
                group,
                dominant: dom,
                domColor,
                confirmed,
                active,
                potential,
                confidence: count > 0 ? confSum / count : 0,
                consensus: Math.abs(dirSum) / (count || 1),
            };
        })
    );
</script>

{#if pair}
<div class="group-panel">
    <div class="panel-title">INDICATOR GROUPS</div>
    <div class="group-grid">
        {#each groups as g}
            <div class="group-card">
                <div class="group-header">
                    <span class="group-name">{g.group}</span>
                    <span class="group-dom" style="color:{g.domColor}">{g.dominant}</span>
                </div>
                <div class="group-sigs">
                    {#if g.confirmed > 0}<span class="sig sig-confirmed">{g.confirmed}c</span>{/if}
                    {#if g.active > 0}<span class="sig sig-active">{g.active}a</span>{/if}
                    {#if g.potential > 0}<span class="sig sig-potential">{g.potential}p</span>{/if}
                </div>
                <div class="group-bar-track">
                    <div class="group-bar-fill" style="width:{Math.round(g.confidence * 100)}%; background:{g.domColor}"></div>
                </div>
                <div class="group-conf">{Math.round(g.confidence * 100)}%</div>
            </div>
        {/each}
    </div>
</div>
{/if}

<style>
    .group-panel {
        padding: 6px 8px;
        border: 1px solid var(--border-muted);
        border-radius: 6px;
        margin: 4px 0;
        background: var(--bg-card);
    }
    .panel-title {
        font-size: 10px;
        color: var(--text-muted);
        text-transform: uppercase;
        letter-spacing: 0.5px;
        margin-bottom: 4px;
    }
    .group-grid {
        display: flex;
        flex-wrap: wrap;
        gap: 4px;
    }
    .group-card {
        flex: 1 1 80px;
        min-width: 70px;
        padding: 3px 5px;
        border: 1px solid var(--border-muted);
        border-radius: 4px;
        background: var(--bg-dark);
    }
    .group-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
    }
    .group-name {
        font-size: 9px;
        color: var(--text-secondary);
        font-weight: 500;
    }
    .group-dom {
        font-size: 11px;
    }
    .group-sigs {
        display: flex;
        gap: 3px;
        margin: 2px 0;
    }
    .sig {
        font-size: 8px;
        padding: 0 2px;
        border-radius: 2px;
        font-family: var(--font-mono);
    }
    .sig-confirmed { color: #10b981; border: 1px solid #10b98144; }
    .sig-active { color: #f59e0b; border: 1px solid #f59e0b44; }
    .sig-potential { color: #94a3b8; border: 1px solid #94a3b844; }
    .group-bar-track {
        height: 3px;
        background: var(--border-muted);
        border-radius: 2px;
        margin: 2px 0;
    }
    .group-bar-fill {
        height: 100%;
        border-radius: 2px;
    }
    .group-conf {
        font-size: 8px;
        color: var(--text-dim);
        font-family: var(--font-mono);
    }
</style>
