<script lang="ts">
    import { getState } from '../state.svelte';
    const app = getState();

    let newPairInput = $state('');
    let showAddInput = $state(false);

    function selectTab(pairKey: string) {
        app.activeTab = pairKey;
    }

    function confirmAdd() {
        const raw = newPairInput.trim().toUpperCase();
        if (raw.length < 2 || raw.length > 10) return;

        // Support "Exchange:Symbol" format (e.g. "Bybit:SOL"), default to Hyperliquid
        const parts = raw.split(':');
        const exchange = parts.length > 1 ? parts[0] : 'Hyperliquid';
        const symbol = parts.length > 1 ? parts[1] : raw;
        const pairKey = `${exchange}-${symbol}`;

        app.initPair(symbol, exchange);
        fetch(`/api/pairs`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ symbol: symbol, exchange: exchange }),
        }).then(() => {
            app.activeTab = pairKey;
        }).catch(console.error);

        newPairInput = '';
        showAddInput = false;
    }

    function removePair(pairKey: string) {
        fetch(`/api/pairs/${encodeURIComponent(pairKey)}`, {
            method: 'DELETE',
        }).catch(console.error);
        app.removePair(pairKey);
        const remaining = Object.keys(app.pairsMap);
        if (remaining.length > 0) {
            if (pairKey === app.activeTab) {
                app.activeTab = remaining[0];
            }
        }
    }
</script>

<div class="tab-bar">
    <div class="tab-left-section">
        <button class="settings-btn" onclick={() => app.showSettingsPanel = !app.showSettingsPanel} aria-label="Toggle Settings">
            ⚙️ Settings
        </button>

        <div class="tabs-container">
            {#each Object.keys(app.pairsMap) as symbol (symbol)}
                <button
                    class="tab-btn"
                    class:tab-active={symbol === app.activeTab}
                    onclick={() => selectTab(symbol)}
                >
                    <span class="tab-label">[{symbol}]</span>
                    <!-- svelte-ignore a11y_no_static_element_interactions -->
                    <span class="tab-close" role="button" tabindex="0" onkeydown={(e: KeyboardEvent) => { if (e.key === 'Enter') removePair(symbol); }} onclick={(e: MouseEvent) => { e.stopPropagation(); removePair(symbol); }}>&times;</span>
                </button>
            {/each}

            {#if showAddInput}
                <div class="add-pair-field">
                    <input
                        type="text"
                        class="pair-input"
                        placeholder="SYMBOL"
                        maxlength="10"
                        bind:value={newPairInput}
                        onkeydown={(e: KeyboardEvent) => { if (e.key === 'Enter') confirmAdd(); if (e.key === 'Escape') { showAddInput = false; newPairInput = ''; } }}
                    />
                    <button class="add-confirm-btn" onclick={confirmAdd}>+</button>
                    <button class="add-cancel-btn" onclick={() => { showAddInput = false; newPairInput = ''; }}>&times;</button>
                </div>
            {:else}
                <button class="tab-btn add-tab-btn" onclick={() => showAddInput = true}>[ + Add Pair ]</button>
            {/if}
        </div>

        <div class="view-toggles">
            <button
                class="view-toggle-btn"
                class:vt-active={app.currentView === 'terminal'}
                onclick={() => app.currentView = 'terminal'}
            >
                📈 Terminal
            </button>
            <button
                class="view-toggle-btn"
                class:vt-active={app.currentView === 'performance'}
                onclick={() => app.currentView = 'performance'}
            >
                📊 Performance
            </button>
        </div>
    </div>

    <div class="status-badge" class:status-online={app.isConnected} class:status-offline={!app.isConnected}>
        <span class="status-pulse-dot {app.isConnected ? 'dot-online' : 'dot-offline'} animate-pulse"></span>
        <span>{app.isConnected ? 'LIVE' : 'OFFLINE'}</span>
    </div>
</div>

<style>
    .tab-bar {
        background-color: #131722;
        border-bottom: 1px solid #1e293b;
        padding: 6px 16px;
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 16px;
    }
    .tab-left-section {
        display: flex;
        align-items: center;
        gap: 16px;
    }
    .tabs-container {
        display: flex;
        align-items: center;
        gap: 4px;
    }
    .settings-btn {
        background-color: #171b26;
        border: 1px solid #2a2e39;
        color: #8f929d;
        font-size: 11px;
        font-weight: 700;
        cursor: pointer;
        padding: 5px 12px;
        border-radius: 4px;
        transition: all 0.2s;
        text-transform: uppercase;
        display: flex;
        align-items: center;
        gap: 6px;
        font-family: 'Courier New', monospace;
    }
    .settings-btn:hover {
        color: #ccd6f6;
        border-color: #3498db;
        background-color: rgba(52, 152, 219, 0.1);
    }
    .tab-btn {
        background: #1a2030;
        border: 1px solid #2a3040;
        color: #8892b0;
        font-family: 'Courier New', monospace;
        font-size: 13px;
        padding: 4px 10px;
        border-radius: 4px;
        cursor: pointer;
        display: flex;
        align-items: center;
        gap: 6px;
        transition: all 0.15s;
    }
    .tab-btn:hover {
        background: #232d40;
        color: #ccd6f6;
        border-color: #3a4560;
    }
    .tab-active {
        background: #1e3a5f;
        border-color: #3498db;
        color: #64ffda;
    }
    .tab-label {
        white-space: nowrap;
    }
    .tab-close {
        font-size: 11px;
        color: #546080;
        line-height: 1;
        padding: 0 2px;
        border-radius: 2px;
    }
    .tab-close:hover {
        color: #ff6b6b;
        background: rgba(255, 107, 107, 0.15);
    }
    .add-tab-btn {
        opacity: 0.5;
        border-style: dashed;
    }
    .add-tab-btn:hover {
        opacity: 1;
    }
    .add-pair-field {
        display: flex;
        align-items: center;
        gap: 4px;
    }
    .pair-input {
        width: 70px;
        background: #1a2030;
        border: 1px solid #3498db;
        color: #64ffda;
        font-family: 'Courier New', monospace;
        font-size: 13px;
        padding: 3px 6px;
        border-radius: 4px;
        outline: none;
        text-transform: uppercase;
    }
    .add-confirm-btn, .add-cancel-btn {
        background: #1a2030;
        border: 1px solid #2a3040;
        color: #64ffda;
        font-size: 13px;
        padding: 2px 6px;
        border-radius: 4px;
        cursor: pointer;
    }
    .add-cancel-btn {
        color: #ff6b6b;
    }
    .status-badge {
        display: flex;
        align-items: center;
        gap: 6px;
        font-size: 11px;
        font-weight: 600;
        letter-spacing: 0.5px;
        font-family: 'Courier New', monospace;
        padding: 3px 10px;
        border-radius: 4px;
        white-space: nowrap;
    }
    .status-online {
        background: rgba(16, 185, 129, 0.1);
        border: 1px solid rgba(16, 185, 129, 0.3);
        color: #10b981;
    }
    .status-offline {
        background: rgba(239, 68, 68, 0.1);
        border: 1px solid rgba(239, 68, 68, 0.3);
        color: #ef4444;
    }
    .status-pulse-dot {
        width: 6px;
        height: 6px;
        border-radius: 50%;
        display: inline-block;
    }
    .dot-online {
        background: #10b981;
        box-shadow: 0 0 6px rgba(16, 185, 129, 0.6);
    }
    .dot-offline {
        background: #ef4444;
        box-shadow: 0 0 6px rgba(239, 68, 68, 0.6);
    }
    .view-toggles {
        display: flex;
        gap: 4px;
        margin-left: 8px;
    }
    .view-toggle-btn {
        background: #171b26;
        border: 1px solid #2a2e39;
        color: #8f929d;
        font-size: 11px;
        font-family: 'Courier New', monospace;
        font-weight: 700;
        cursor: pointer;
        padding: 5px 12px;
        border-radius: 4px;
        transition: all 0.2s;
    }
    .view-toggle-btn:hover {
        color: #cbd5e1;
        border-color: #3b82f6;
    }
    .vt-active {
        background: #1e3a5f;
        border-color: #3b82f6;
        color: #64ffda;
    }
</style>
