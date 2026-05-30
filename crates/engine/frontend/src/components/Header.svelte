<script lang="ts">
    import { getState } from '../state.svelte';
    const app = getState();
</script>

<header class="terminal-header">
    <div class="header-left-col">
        <button class="settings-btn" onclick={() => app.showSettingsPanel = !app.showSettingsPanel} aria-label="Toggle Settings">
            ⚙️ Settings
        </button>
    </div>

    <div class="header-center-col">
        <h1 class="logo-title">{app.activeSymbol}USD</h1>
        <div class="time-badge">
            {app.candleTimeframeLabel}
        </div>
    </div>

    <div class="header-right-col">
        <div class="status-badge" class:status-online={app.isConnected} class:status-offline={!app.isConnected}>
            <span class="status-pulse-dot {app.isConnected ? 'dot-online' : 'dot-offline'} animate-pulse"></span>
            <span>{app.isConnected ? 'LIVE STREAM ACTIVE' : 'OFFLINE'}</span>
        </div>
    </div>
</header>

<style>
    .terminal-header {
        border-bottom: 1px solid #1e293b;
        background-color: rgba(19, 23, 34, 0.9);
        padding: 12px 24px;
        display: grid;
        grid-template-columns: 1fr auto 1fr;
        align-items: center;
    }
    .header-left-col {
        display: flex;
        justify-content: flex-start;
    }
    .header-center-col {
        display: flex;
        align-items: center;
        gap: 12px;
        justify-content: center;
    }
    .header-right-col {
        display: flex;
        justify-content: flex-end;
    }
    .logo-title {
        font-size: 1.1rem;
        font-weight: 800;
        letter-spacing: 0.05em;
        color: #f1f5f9;
        margin: 0;
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
    .settings-btn {
        background-color: #171b26;
        border: 1px solid #2a2e39;
        color: #8f929d;
        font-size: 11px;
        font-weight: 700;
        cursor: pointer;
        padding: 6px 14px;
        border-radius: 6px;
        transition: all 0.2s;
        text-transform: uppercase;
        display: flex;
        align-items: center;
        gap: 6px;
    }
    .settings-btn:hover {
        color: #cbd5e1;
        border-color: #3b82f6;
        background-color: rgba(59, 130, 246, 0.05);
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
