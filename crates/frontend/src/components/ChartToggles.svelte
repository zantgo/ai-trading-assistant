<script lang="ts">
    import { getState } from '../state.svelte';

    const app = getState();
    let { pairKey }: { pairKey: string } = $props();
    const pair = $derived(app.instancesMap[pairKey]);

    function syncAll(fn: (tf: any) => void) {
        if (!pair) return;
        fn(pair.microTerm);
        fn(pair.smallTerm);
        fn(pair.mediumTerm);
        fn(pair.largeTerm);
    }

    function toggleLineMode() {
        if (!pair) return;
        pair.priceLineMode = !pair.priceLineMode;
    }

    function toggleVwap() {
        if (!pair) return;
        const v = !pair.microTerm.showVwap;
        syncAll(tf => { tf.showVwap = v; });
    }

    function toggleBb() {
        if (!pair) return;
        const v = !pair.microTerm.showBb;
        syncAll(tf => { tf.showBb = v; });
    }

    function toggleEma(label: 'Fast' | 'Medium' | 'Slow' | 'Long') {
        if (!pair) return;
        const key = `showEma${label}` as keyof typeof pair;
        (pair as any)[key] = !(pair as any)[key];
    }
</script>

{#if pair}
<div class="chart-toggles">
    <div class="toggles-group">
        <span class="toggles-label">PRICE</span>
        <button
            class="toggle-pill"
            class:active={!pair.priceLineMode}
            onclick={toggleLineMode}
        >CANDLES</button>
        <button
            class="toggle-pill"
            class:active={pair.priceLineMode}
            onclick={toggleLineMode}
        >LINE</button>
    </div>
    <div class="toggles-separator"></div>
    <div class="toggles-group">
        <span class="toggles-label">EMA</span>
        <button class="toggle-pill ema-fast" class:active={pair.showEmaFast}
            onclick={() => toggleEma('Fast')}>FAST</button>
        <button class="toggle-pill ema-medium" class:active={pair.showEmaMedium}
            onclick={() => toggleEma('Medium')}>MED</button>
        <button class="toggle-pill ema-slow" class:active={pair.showEmaSlow}
            onclick={() => toggleEma('Slow')}>SLOW</button>
        <button class="toggle-pill ema-long" class:active={pair.showEmaLong}
            onclick={() => toggleEma('Long')}>LONG</button>
    </div>
    <div class="toggles-separator"></div>
    <div class="toggles-group">
        <button class="toggle-pill vwap-pill" class:active={pair.microTerm.showVwap}
            onclick={toggleVwap}>VWAP</button>
        <button class="toggle-pill bb-pill" class:active={pair.microTerm.showBb}
            onclick={toggleBb}>BOLLINGER</button>
    </div>
</div>
{/if}

<style>
    .chart-toggles {
        display: flex;
        align-items: center;
        gap: 10px;
        padding: 4px 8px;
        background: #0a0d14;
        border-bottom: 1px solid #1e293b;
        flex-wrap: wrap;
    }
    .toggles-group {
        display: flex;
        align-items: center;
        gap: 4px;
    }
    .toggles-label {
        font-size: 8px;
        font-weight: 700;
        color: #4a5568;
        text-transform: uppercase;
        letter-spacing: 0.08em;
        font-family: 'Courier New', monospace;
        margin-right: 2px;
    }
    .toggles-separator {
        width: 1px;
        height: 18px;
        background: #1e293b;
    }
    .toggle-pill {
        padding: 2px 8px;
        border: 1px solid #1e293b;
        border-radius: 3px;
        background: transparent;
        color: #4a5568;
        font-size: 8px;
        font-weight: 700;
        font-family: 'Courier New', monospace;
        cursor: pointer;
        letter-spacing: 0.05em;
        text-transform: uppercase;
        transition: all 0.15s;
    }
    .toggle-pill:hover {
        border-color: #334155;
        color: #94a3b8;
    }
    .toggle-pill.active {
        border-color: #64ffda;
        color: #64ffda;
        background: rgba(100, 255, 218, 0.08);
    }
    .toggle-pill.ema-fast.active { border-color: #2962ff; color: #2962ff; background: rgba(41, 98, 255, 0.1); }
    .toggle-pill.ema-medium.active { border-color: #ff9800; color: #ff9800; background: rgba(255, 152, 0, 0.1); }
    .toggle-pill.ema-slow.active { border-color: #e91e63; color: #e91e63; background: rgba(233, 30, 99, 0.1); }
    .toggle-pill.ema-long.active { border-color: #9c27b0; color: #9c27b0; background: rgba(156, 39, 176, 0.1); }
    .toggle-pill.vwap-pill.active { border-color: #ffb300; color: #ffb300; background: rgba(255, 179, 0, 0.1); }
    .toggle-pill.bb-pill.active { border-color: #00e5ff; color: #00e5ff; background: rgba(0, 229, 255, 0.1); }
</style>
