<script lang="ts">
    import { getState } from '../state.svelte';
    const state = getState();
</script>

<header class="terminal-header">
    <div class="header-logo-group">
        <span class="text-xl">⚡</span>
        <h1 class="logo-title">{state.activeSymbol}USD</h1>
        <div class="time-badge">
            {state.candleTimeframeLabel}
        </div>
    </div>

    <div class="visibility-selectors font-sans">
        <span class="selectors-label">Overlays:</span>
        <button class="selector-btn" class:active={state.showEmas} onclick={() => state.showEmas = !state.showEmas}>
            EMAs
        </button>
        <button class="selector-btn" class:active={state.showBb} onclick={() => state.showBb = !state.showBb}>
            BB
        </button>
        <button class="selector-btn" class:active={state.showVwap} onclick={() => state.showVwap = !state.showVwap}>
            VWAP
        </button>

        <span class="selectors-label ml-4">Panels:</span>
        <button class="selector-btn" class:active={state.showVolume} onclick={() => state.showVolume = !state.showVolume}>
            Volume
        </button>
        <button class="selector-btn" class:active={state.showAdx} onclick={() => state.showAdx = !state.showAdx}>
            ADX
        </button>
        <button class="selector-btn" class:active={state.showAtr} onclick={() => state.showAtr = !state.showAtr}>
            ATR
        </button>
        <button class="selector-btn" class:active={state.showRsi} onclick={() => state.showRsi = !state.showRsi}>
            RSI
        </button>
        <button class="selector-btn" class:active={state.showMacd} onclick={() => state.showMacd = !state.showMacd}>
            MACD
        </button>
        <button class="selector-btn" class:active={state.showSqueeze} onclick={() => state.showSqueeze = !state.showSqueeze}>
            Squeeze
        </button>
    </div>

    <div class="status-badge" class:status-online={state.isConnected} class:status-offline={!state.isConnected}>
        <span class="status-pulse-dot {state.isConnected ? 'dot-online' : 'dot-offline'} animate-pulse"></span>
        <span>{state.isConnected ? 'LIVE STREAM ACTIVE' : 'OFFLINE'}</span>
    </div>
</header>

<style>
    .terminal-header {
        border-bottom: 1px solid #1e293b;
        background-color: rgba(19, 23, 34, 0.9);
        padding: 12px 24px;
        display: flex;
        justify-content: space-between;
        align-items: center;
    }
    .header-logo-group {
        display: flex;
        align-items: center;
        gap: 16px;
    }
    .logo-title {
        font-size: 0.75rem;
        font-weight: 700;
        letter-spacing: 0.1em;
        color: #cbd5e1;
        text-transform: uppercase;
    }
    .time-badge {
        padding: 2px 8px;
        border-radius: 4px;
        font-size: 10px;
        font-weight: 900;
        background-color: rgba(59, 130, 246, 0.1);
        color: #60a5fa;
        border: 1px solid rgba(59, 130, 246, 0.2);
        text-transform: uppercase;
        letter-spacing: 0.1em;
    }
    .visibility-selectors {
        display: flex;
        align-items: center;
        gap: 8px;
    }
    .selectors-label {
        font-size: 9px;
        font-weight: 700;
        text-transform: uppercase;
        letter-spacing: 0.08em;
        color: #64748b;
        margin-right: 4px;
    }
    .ml-4 { margin-left: 16px; }
    .selector-btn {
        background-color: #171b26;
        border: 1px solid #2a2e39;
        color: #8f929d;
        font-size: 9px;
        font-weight: 800;
        padding: 4px 10px;
        border-radius: 4px;
        cursor: pointer;
        transition: all 0.2s ease-in-out;
        text-transform: uppercase;
    }
    .selector-btn:hover {
        border-color: #4c526e;
        color: #cbd5e1;
    }
    .selector-btn.active {
        background-color: rgba(59, 130, 246, 0.12);
        border-color: #3b82f6;
        color: #3b82f6;
        box-shadow: 0 0 8px rgba(59, 130, 246, 0.15);
    }
    .status-badge {
        padding: 4px 12px;
        border-radius: 4px;
        font-size: 0.75rem;
        font-weight: 600;
        border: 1px solid;
        display: flex;
        align-items: center;
        gap: 8px;
    }
    .status-online {
        background-color: rgba(16, 185, 129, 0.1);
        color: rgb(52, 211, 153);
        border-color: rgba(16, 185, 129, 0.2);
    }
    .status-offline {
        background-color: rgba(239, 68, 68, 0.1);
        color: rgb(248, 113, 113);
        border-color: rgba(239, 68, 68, 0.2);
    }
    .status-pulse-dot {
        height: 8px;
        width: 8px;
        border-radius: 9999px;
    }
    .dot-online { background-color: #10b981; }
    .dot-offline { background-color: #ef5350; }
</style>
