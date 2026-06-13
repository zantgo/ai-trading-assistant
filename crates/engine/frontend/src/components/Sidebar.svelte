<script lang="ts">
    import { getState } from '../state.svelte';
    const app = getState();

    let collapsed = $state(false);
    let newPairInput = $state('');
    let showAddInput = $state(false);

    function selectPair(pairKey: string) {
        app.activeTab = pairKey;
    }

    function confirmAdd() {
        const raw = newPairInput.trim().toUpperCase();
        if (raw.length < 2 || raw.length > 10) return;

        const parts = raw.split(':');
        const exchange = parts.length > 1 ? parts[0] : 'Hyperliquid';
        const symbol = parts.length > 1 ? parts[1] : raw;
        const pairKey = `${exchange}-${symbol}`;

        app.initPair(symbol, exchange);
        fetch('/api/pairs', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ symbol, exchange }),
        }).then(() => {
            app.activeTab = pairKey;
        }).catch(console.error);

        newPairInput = '';
        showAddInput = false;
    }

    function removePair(pairKey: string) {
        fetch(`/api/pairs/${encodeURIComponent(pairKey)}`, { method: 'DELETE' }).catch(console.error);
        app.removePair(pairKey);
        const remaining = Object.keys(app.pairsMap);
        if (remaining.length > 0 && pairKey === app.activeTab) {
            app.activeTab = remaining[0];
        }
    }

    function pairLabel(key: string): string {
        const parts = key.split('-');
        return `<span class="pair-label-exchange">${parts[0]}</span><span class="pair-label-symbol">${parts[1] || key}</span>`;
    }

    function shortName(key: string): string {
        const parts = key.split('-');
        return (parts[1] || key).substring(0, 4).toUpperCase();
    }
</script>

<div class="sidebar-container" class:collapsed>
    <button class="sidebar-toggle-btn" onclick={() => collapsed = !collapsed} title={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}>
        {collapsed ? '▶' : '◀'}
    </button>

    {#if !collapsed}
        <div class="sidebar-header">
            <span class="sidebar-logo">TRADING</span>
            <span class="sidebar-status">{@html app.isConnected ? '<span class="status-live">● LIVE</span>' : '<span class="status-offline">● OFFLINE</span>'}</span>
        </div>
    {:else}
        <div class="sidebar-header collapsed-header">
            <span class="sidebar-logo-small">AT</span>
        </div>
    {/if}

    <div class="sidebar-pairs-list">
        {#each Object.keys(app.pairsMap) as pairKey}
            <button
                class="pair-item"
                class:active={pairKey === app.activeTab}
                onclick={() => selectPair(pairKey)}
                title={pairKey}
            >
                {#if collapsed}
                    <span class="pair-short">{shortName(pairKey)}</span>
                {:else}
                    <span class="pair-name">{@html pairLabel(pairKey)}</span>
                    <span class="pair-status-dot" class:connected={app.pairsMap[pairKey].isConnected}></span>
                    <button class="pair-remove-btn" onclick={(e) => { e.stopPropagation(); removePair(pairKey); }} title="Remove pair">×</button>
                {/if}
            </button>
        {/each}
    </div>

    {#if !collapsed}
        <div class="sidebar-add-section">
            {#if showAddInput}
                <div class="add-pair-input-group">
                    <input
                        type="text"
                        placeholder="e.g. Hyperliquid:ETH"
                        bind:value={newPairInput}
                        onkeydown={(e) => { if (e.key === 'Enter') confirmAdd(); }}
                        class="add-pair-input"
                        autofocus
                    />
                    <button class="add-pair-confirm" onclick={confirmAdd}>+</button>
                    <button class="add-pair-cancel" onclick={() => { showAddInput = false; newPairInput = ''; }}>×</button>
                </div>
            {:else}
                <button class="add-pair-btn" onclick={() => showAddInput = true}>
                    + Add Pair
                </button>
            {/if}
        </div>
    {/if}
</div>

<style>
    .sidebar-container {
        width: 240px;
        height: 100vh;
        background-color: #0f111a;
        border-right: 1px solid #1e293b;
        display: flex;
        flex-direction: column;
        transition: width 0.2s cubic-bezier(0.4, 0, 0.2, 1);
        position: relative;
        z-index: 100;
        flex-shrink: 0;
    }
    .sidebar-container.collapsed {
        width: 60px;
    }
    .sidebar-toggle-btn {
        position: absolute;
        right: -12px;
        top: 24px;
        width: 24px;
        height: 24px;
        background-color: #1e293b;
        border: 1px solid #3498db;
        border-radius: 50%;
        color: #64ffda;
        cursor: pointer;
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: 110;
        transition: transform 0.2s;
        font-size: 10px;
        padding: 0;
    }
    .sidebar-toggle-btn:hover {
        background-color: #2d3a4a;
    }
    .sidebar-header {
        padding: 16px;
        border-bottom: 1px solid #1e293b;
        display: flex;
        justify-content: space-between;
        align-items: center;
    }
    .sidebar-logo {
        font-size: 13px;
        font-weight: 800;
        letter-spacing: 0.12em;
        color: #64ffda;
        font-family: 'Courier New', monospace;
    }
    .sidebar-logo-small {
        font-size: 12px;
        font-weight: 800;
        color: #64ffda;
        font-family: 'Courier New', monospace;
        display: block;
        text-align: center;
    }
    .collapsed-header {
        justify-content: center;
        padding: 12px 8px;
    }
    .status-live { color: #10b981; font-size: 10px; }
    .status-offline { color: #ef4444; font-size: 10px; }
    .sidebar-pairs-list {
        flex: 1;
        overflow-y: auto;
        padding: 8px 0;
    }
    .pair-item {
        display: flex;
        align-items: center;
        gap: 10px;
        padding: 10px 16px;
        width: 100%;
        border: none;
        background: transparent;
        color: #8892b0;
        cursor: pointer;
        text-align: left;
        font-family: 'Courier New', monospace;
        font-size: 13px;
        overflow: hidden;
        white-space: nowrap;
        transition: background 0.15s;
    }
    .pair-item:hover {
        background-color: rgba(255, 255, 255, 0.03);
        color: #cbd5e1;
    }
    .pair-item.active {
        background-color: rgba(59, 130, 246, 0.1);
        color: #64ffda;
        border-left: 3px solid #3498db;
        padding-left: 13px;
    }
    .pair-short {
        font-size: 11px;
        font-weight: 700;
        color: #cbd5e1;
        font-family: 'Courier New', monospace;
    }
    .pair-name {
        flex: 1;
        font-size: 12px;
    }
    .pair-label-exchange { color: #64748b; font-size: 10px; }
    .pair-label-symbol { color: #cbd5e1; font-weight: 600; }
    .pair-status-dot {
        width: 6px;
        height: 6px;
        border-radius: 50%;
        background: #ef4444;
        flex-shrink: 0;
    }
    .pair-status-dot.connected { background: #10b981; }
    .pair-remove-btn {
        background: none;
        border: none;
        color: #475569;
        cursor: pointer;
        font-size: 14px;
        padding: 0 4px;
        opacity: 0;
        transition: opacity 0.15s;
    }
    .pair-item:hover .pair-remove-btn { opacity: 1; }
    .pair-remove-btn:hover { color: #ef4444; }
    .sidebar-add-section {
        padding: 12px 16px;
        border-top: 1px solid #1e293b;
    }
    .add-pair-btn {
        width: 100%;
        padding: 8px;
        background: transparent;
        border: 1px dashed #3498db;
        border-radius: 6px;
        color: #3498db;
        font-size: 12px;
        cursor: pointer;
        font-family: 'Courier New', monospace;
    }
    .add-pair-btn:hover { background: rgba(52, 152, 219, 0.1); }
    .add-pair-input-group {
        display: flex;
        gap: 6px;
    }
    .add-pair-input {
        flex: 1;
        background: #1a1d2e;
        border: 1px solid #2e3440;
        border-radius: 4px;
        padding: 6px 8px;
        color: #cbd5e1;
        font-size: 12px;
        font-family: 'Courier New', monospace;
        outline: none;
    }
    .add-pair-input:focus { border-color: #3498db; }
    .add-pair-confirm {
        background: #10b981;
        border: none;
        border-radius: 4px;
        color: #0f111a;
        font-size: 14px;
        font-weight: 700;
        cursor: pointer;
        padding: 4px 8px;
    }
    .add-pair-cancel {
        background: none;
        border: none;
        color: #64748b;
        font-size: 14px;
        cursor: pointer;
        padding: 4px 6px;
    }
    .add-pair-cancel:hover { color: #ef4444; }
</style>
