<script lang="ts">
    import { getState } from '../state.svelte';
    const app = getState();

    const pairKey = $derived(app.activeTab);
    const pair = $derived(app.instancesMap[pairKey]);

    let interval: ReturnType<typeof setInterval> | undefined;

    $effect(() => {
        if (pairKey) {
            app.fetchSystemStatus();
            app.fetchObservabilityBuffers(pairKey);

            clearInterval(interval);
            interval = setInterval(() => {
                app.fetchSystemStatus();
                app.fetchObservabilityBuffers(pairKey);
            }, 5000);
        }

        return () => {
            clearInterval(interval);
        };
    });

    function formatUsd(v: number | undefined): string {
        if (v == null || isNaN(v)) return '$0.0000';
        return '$' + v.toFixed(4);
    }

    function formatPnl(v: number | undefined): string {
        if (v == null || isNaN(v)) return '$0.00';
        return (v >= 0 ? '+' : '') + '$' + v.toFixed(2);
    }

    function formatPct(v: number | undefined): string {
        if (v == null || isNaN(v)) return '0.0%';
        return (v >= 0 ? '+' : '') + v.toFixed(1) + '%';
    }

    function formatTime(ts: number): string {
        if (!ts) return '--:--:--';
        return new Date(ts).toLocaleTimeString();
    }
</script>

<div class="obs-hud animate-fade">
    {#if pair && app.systemHeartbeat}
        <!-- Top Row: Vital Senses Metrics -->
        <div class="vital-stats-grid">
            <div class="vital-card">
                <span class="vital-label">Websocket Status</span>
                <span class="vital-value" class:text-green={app.systemHeartbeat.connected} class:text-red={!app.systemHeartbeat.connected}>
                    {app.systemHeartbeat.connected ? 'CONNECTED' : 'DISCONNECTED'}
                </span>
            </div>
            <div class="vital-card">
                <span class="vital-label">Core Latency</span>
                <span class="vital-value text-blue">{app.systemHeartbeat.latency_ms} ms</span>
            </div>
            <div class="vital-card">
                <span class="vital-label">Database Mode</span>
                <span class="vital-value text-emerald">{app.systemHeartbeat.journal_mode}</span>
            </div>
            <div class="vital-card">
                <span class="vital-label">Cumulative AI Cost</span>
                <span class="vital-value text-amber">{formatUsd(app.systemHeartbeat.total_ai_token_costs_usd)}</span>
            </div>
        </div>

        <!-- Middle Row: Market Regime Classification -->
        <div class="regime-banner"
             class:regime-trending={pair.microTerm.atrVolatilityRegime === 'expanding'}
             class:regime-compression={pair.microTerm.atrVolatilityRegime === 'contracting'}
             class:regime-stable={pair.microTerm.atrVolatilityRegime === 'stable'}>
            <div class="regime-title">
                ACTIVE VOLATILITY REGIME: {pair.microTerm.atrVolatilityRegime?.toUpperCase() || 'STABLE'}
            </div>
            <div class="regime-metrics font-mono">
                <span>BBWP Percentile: {pair.microTerm.bbwpText || '--'}%</span>
                <span>Relative Volume (RVOL): {pair.microTerm.rvol ? pair.microTerm.rvol.toFixed(2) : '--'}</span>
            </div>
        </div>

        <!-- Parallel Agent Progress Map -->
        <div class="agent-matrix-card">
            <h3 class="card-title">Parallel Agent Matrix</h3>
            <div class="agent-grid">
                <!-- Trend Agent -->
                <div class="agent-node" class:complete={pair.microTerm.emaStackState !== 'tangled'}>
                    <div class="agent-node-header">
                        <span class="agent-node-name">TREND AGENT</span>
                        <span class="agent-node-status">{pair.microTerm.emaStackState?.toUpperCase() || 'OFF'}</span>
                    </div>
                    <p class="agent-node-thought">
                        Evaluating trend fanning order. Stacking order is: {pair.microTerm.emaStackState?.toUpperCase() || 'Tangled'}.
                        Price relative to long EMA: {pair.microTerm.priceText}.
                    </p>
                </div>

                <!-- Volatility Agent -->
                <div class="agent-node" class:complete={pair.microTerm.bbwpText !== '--'}>
                    <div class="agent-node-header">
                        <span class="agent-node-name">VOLATILITY AGENT</span>
                        <span class="agent-node-status">{pair.microTerm.atrVolatilityRegime?.toUpperCase() || 'STABLE'}</span>
                    </div>
                    <p class="agent-node-thought">
                        Monitoring Bollinger Band compression limits and ATR average true range lines.
                        BBWP Percentile currently evaluating at {pair.microTerm.bbwpText || '--'}%.
                    </p>
                </div>

                <!-- Structure Agent -->
                <div class="agent-node" class:complete={app.markedSupportLevels.length > 0}>
                    <div class="agent-node-header">
                        <span class="agent-node-name">STRUCTURE AGENT</span>
                        <span class="agent-node-status">ACTIVE</span>
                    </div>
                    <p class="agent-node-thought">
                        Parsing pivot high and low points.
                        Tracking active levels: Support [{app.markedSupportLevels.slice(0,2).join(', ')}] |
                        Resistance [{app.markedResistanceLevels.slice(0,2).join(', ')}].
                    </p>
                </div>

                <!-- Risk Agent -->
                <div class="agent-node" class:complete={pair.paperMaxRiskPct > 0}>
                    <div class="agent-node-header">
                        <span class="agent-node-name">RISK GATE AGENT</span>
                        <span class="agent-node-status">{pair.paperMaxRiskPct}% RISK</span>
                    </div>
                    <p class="agent-node-thought">
                        Enforcing dynamic portfolio limits. Maximum allowable risk per trade: {pair.paperMaxRiskPct}%.
                        Leverage bound is limited strictly to {pair.paperLeverage}x.
                    </p>
                </div>

                <!-- Position Agent -->
                <div class="agent-node" class:complete={pair.currentPosition !== 'None'}>
                    <div class="agent-node-header">
                        <span class="agent-node-name">POSITION AGENT</span>
                        <span class="agent-node-status">{pair.currentPosition?.toUpperCase()}</span>
                    </div>
                    <p class="agent-node-thought">
                        Monitoring open execution states. Active position: {pair.currentPosition}.
                        Stop Loss target configured dynamically at ${pair.paperInvalidationLevel.toFixed(2)}.
                    </p>
                </div>

                <!-- Master Orchestrator -->
                <div class="agent-node" class:complete={app.totalPointsScore > 0}>
                    <div class="agent-node-header">
                        <span class="agent-node-name">MASTER ORCHESTRATOR</span>
                        <span class="agent-node-status">CONFLUENCE: {app.totalPointsScore} pt</span>
                    </div>
                    <p class="agent-node-thought">
                        Synthesizing inputs from sub-agents.
                        Point Score evaluates to {app.totalPointsScore}/90.
                        Capital allocation multiplier maps to: {app.allocatedCapitalPct}%.
                    </p>
                </div>
            </div>
        </div>

        <!-- Bottom Row: Historical Decision & Trade Buffers -->
        <div class="buffers-two-col">
            <div class="buffer-box card">
                <h4 class="card-title">Decision Memory Buffer</h4>
                <div class="buffer-list">
                    {#if app.recentDecisions.length === 0}
                        <p class="buffer-empty">No logged orchestrator runs in memory.</p>
                    {:else}
                        {#each app.recentDecisions as dec (dec.id)}
                            <div class="buffer-row">
                                <span class="buffer-time">{formatTime(dec.timestamp)}</span>
                                <span class="buffer-regime text-blue font-semibold">{dec.regime_classification}</span>
                                <span class="buffer-action">{dec.orchestrator_decision}</span>
                                <span class="buffer-score font-mono">{dec.eight_factor_score} pts</span>
                            </div>
                        {/each}
                    {/if}
                </div>
            </div>

            <div class="buffer-box card">
                <h4 class="card-title">Completed Trades Buffer</h4>
                <div class="buffer-list">
                    {#if app.completedTrades.length === 0}
                        <p class="buffer-empty">No completed trade ledger entries discovered.</p>
                    {:else}
                        {#each app.completedTrades as trade (trade.id)}
                            <div class="buffer-row">
                                <span class="buffer-time">{formatTime(trade.closed_at)}</span>
                                <span class="buffer-dir" class:text-green={trade.realized_pnl >= 0} class:text-red={trade.realized_pnl < 0}>
                                    {trade.direction}
                                </span>
                                <span class="buffer-pnl font-semibold" class:text-green={trade.realized_pnl >= 0} class:text-red={trade.realized_pnl < 0}>
                                    {formatPnl(trade.realized_pnl)} ({formatPct(trade.roi_pct)})
                                </span>
                                <span class="buffer-score font-mono">{trade.execution_score.toFixed(1)}/10</span>
                            </div>
                        {/each}
                    {/if}
                </div>
            </div>
        </div>
    {/if}
</div>

<style>
    .obs-hud {
        display: flex;
        flex-direction: column;
        gap: 16px;
        padding: 16px;
        max-width: 1400px;
        margin: 0 auto;
        width: 100%;
        box-sizing: border-box;
    }
    .vital-stats-grid {
        display: grid;
        grid-template-columns: repeat(4, 1fr);
        gap: 12px;
    }
    .vital-card {
        background: #131722;
        border: 1px solid #2a2e39;
        border-radius: 8px;
        padding: 12px;
        display: flex;
        flex-direction: column;
        gap: 4px;
    }
    .vital-label {
        font-size: 9px;
        color: #64748b;
        text-transform: uppercase;
        font-weight: 700;
        letter-spacing: 0.05em;
    }
    .vital-value {
        font-size: 14px;
        font-weight: 800;
        color: #cbd5e1;
        font-family: ui-monospace, monospace;
    }
    .text-green { color: #10b981; }
    .text-red { color: #ef4444; }
    .text-blue { color: #3b82f6; }
    .text-emerald { color: #059669; }
    .text-amber { color: #f59e0b; }

    .regime-banner {
        background: #131722;
        border: 1px solid #2a2e39;
        border-radius: 8px;
        padding: 12px 16px;
        display: flex;
        justify-content: space-between;
        align-items: center;
    }
    .regime-trending { border-color: #10b981; background: rgba(16, 185, 129, 0.04); }
    .regime-compression { border-color: #f59e0b; background: rgba(245, 158, 11, 0.04); }
    .regime-stable { border-color: #475569; }
    .regime-title {
        font-size: 11px;
        font-weight: 800;
        color: #f1f5f9;
        letter-spacing: 0.05em;
    }
    .regime-metrics {
        display: flex;
        gap: 16px;
        font-size: 11px;
        color: #94a3b8;
    }

    .agent-matrix-card {
        background: #131722;
        border: 1px solid #2a2e39;
        border-radius: 8px;
        padding: 16px;
    }
    .card-title {
        font-size: 12px;
        font-weight: 700;
        color: #f1f5f9;
        margin-top: 0;
        margin-bottom: 12px;
        text-transform: uppercase;
        letter-spacing: 0.05em;
        border-bottom: 1px solid #1e293b;
        padding-bottom: 6px;
    }
    .agent-grid {
        display: grid;
        grid-template-columns: repeat(3, 1fr);
        gap: 12px;
    }
    .agent-node {
        background: #0f111a;
        border: 1px solid #1e293b;
        border-radius: 6px;
        padding: 12px;
        min-height: 110px;
        display: flex;
        flex-direction: column;
        gap: 8px;
    }
    .agent-node.complete {
        border-color: rgba(16, 185, 129, 0.3);
        background: rgba(16, 185, 129, 0.02);
    }
    .agent-node-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        border-bottom: 1px dashed #1e293b;
        padding-bottom: 4px;
    }
    .agent-node-name {
        font-size: 10px;
        font-weight: 800;
        color: #e2e8f0;
        font-family: 'Courier New', monospace;
    }
    .agent-node-status {
        font-size: 8px;
        font-weight: 700;
        text-transform: uppercase;
        color: #64748b;
    }
    .complete .agent-node-status {
        color: #10b981;
    }
    .agent-node-thought {
        font-size: 10px;
        color: #94a3b8;
        margin: 0;
        line-height: 1.4;
    }

    .buffers-two-col {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 16px;
    }
    .buffer-box {
        display: flex;
        flex-direction: column;
        gap: 8px;
    }
    .card {
        background-color: #131722;
        border: 1px solid #2a2e39;
        border-radius: 8px;
        padding: 16px;
    }
    .buffer-list {
        display: flex;
        flex-direction: column;
        gap: 6px;
    }
    .buffer-row {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: 8px 12px;
        background: #0f111a;
        border-radius: 4px;
        border: 1px solid #1e293b;
        font-size: 11px;
    }
    .buffer-time {
        color: #64748b;
        font-family: ui-monospace, monospace;
    }
    .buffer-action {
        font-weight: 700;
        color: #cbd5e1;
    }
    .buffer-empty {
        font-size: 11px;
        color: #4c525e;
        text-align: center;
        padding: 12px 0;
        font-style: italic;
    }

    @media (max-width: 1024px) {
        .vital-stats-grid { grid-template-columns: 1fr 1fr; }
        .agent-grid { grid-template-columns: 1fr; }
        .buffers-two-col { grid-template-columns: 1fr; }
    }
</style>
