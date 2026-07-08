<script lang="ts">
    import { iSub } from '../../lib/telemetry';
    import type { IndicatorMap } from '../../types';
    import { useAppStore } from '../../state.svelte';

    const app = useAppStore();
    let { pairKey }: { pairKey: string } = $props();
    const pair = $derived(app.instancesMap[pairKey]);
    const tf = $derived(pair?.microTerm);
    const snap = $derived(tf?.latestSnapshot);
    const indicators = $derived((snap?.indicators ?? {}) as IndicatorMap);
    const price = $derived(Number(tf?.priceText) || 0);

    interface LevelEntry {
        rank: number;
        name: string;
        price: number | null;
        distancePct: number;
        side: string;
        active: boolean;
    }

    const levels: LevelEntry[] = $derived([
        {
            rank: 1, name: 'Order Block',
            price: iSub(indicators, 'smc_order_blocks', 'ob_bullish_low')
                ?? iSub(indicators, 'smc_order_blocks', 'ob_bearish_high'),
            distancePct: 0, side: '—', active: false,
        },
        {
            rank: 2, name: 'Volume POC',
            price: iSub(indicators, 'volume_profile', 'poc'),
            distancePct: 0, side: '—', active: false,
        },
        {
            rank: 3, name: 'Fib Golden Pocket',
            price: iSub(indicators, 'fibonacci', 'gp_top')
                ?? iSub(indicators, 'fibonacci', 'gp_bottom'),
            distancePct: 0, side: '—', active: false,
        },
        {
            rank: 4, name: 'Pivot S/R',
            price: iSub(indicators, 'pivot_points', 's1')
                ?? iSub(indicators, 'pivot_points', 'r1'),
            distancePct: 0, side: '—', active: false,
        },
        {
            rank: 5, name: 'VWAP',
            price: iSub(indicators, 'vwap', 'vwap'),
            distancePct: 0, side: '—', active: false,
        },
        {
            rank: 6, name: 'Anchored VWAP',
            price: iSub(indicators, 'anchored_vwap', 'vwap_weekly'),
            distancePct: 0, side: '—', active: false,
        },
        {
            rank: 7, name: 'ATR ×2',
            price: iSub(indicators, 'atr', 'raw_value') !== null
                ? price - 2 * (iSub(indicators, 'atr', 'raw_value') ?? 0)
                : null,
            distancePct: 0, side: '—', active: false,
        },
    ].map(l => {
        const active = l.price != null && l.price > 0;
        const dist = active ? ((l.price! - price) / price) * 100 : 0;
        const side = active ? (dist > 0 ? 'ABOVE' : dist < 0 ? 'BELOW' : 'AT') : '—';
        return { ...l, distancePct: Math.abs(dist), side, active };
    }));

    function sideColor(side: string): string {
        return side === 'ABOVE' ? '#ef4444' : side === 'BELOW' ? '#10b981' : '#94a3b8';
    }
</script>

{#if pair}
<div class="level-panel">
    <div class="panel-title">LEVEL HIERARCHY</div>
    <div class="level-list">
        {#each levels.filter(l => l.active) as l}
            <div class="level-row">
                <span class="level-rank">#{l.rank}</span>
                <span class="level-name">{l.name}</span>
                <span class="level-price">${l.price?.toFixed(2) ?? '—'}</span>
                <span class="level-dist">{l.distancePct.toFixed(2)}%</span>
                <span class="level-side" style="color:{sideColor(l.side)}">{l.side}</span>
            </div>
        {/each}
        {#if levels.filter(l => l.active).length === 0}
            <div class="empty">Awaiting structural data…</div>
        {/if}
    </div>
</div>
{/if}

<style>
    .level-panel {
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
    .level-list {
        display: flex;
        flex-direction: column;
        gap: 2px;
    }
    .level-row {
        display: flex;
        align-items: center;
        gap: 6px;
        padding: 2px 4px;
        border-radius: 3px;
        background: var(--bg-dark);
        font-size: 10px;
        font-family: var(--font-mono);
    }
    .level-rank {
        color: var(--text-dim);
        min-width: 16px;
    }
    .level-name {
        color: var(--text-secondary);
        min-width: 80px;
        font-size: 9px;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }
    .level-price {
        color: var(--text-primary);
        min-width: 70px;
    }
    .level-dist {
        color: var(--text-dim);
        min-width: 48px;
    }
    .level-side {
        font-weight: 500;
        font-size: 9px;
    }
    .empty {
        color: var(--text-dim);
        font-size: 10px;
        text-align: center;
        padding: 4px;
    }
</style>
