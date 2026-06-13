<script lang="ts">
    import { onMount } from 'svelte';
    import { getState } from '../state.svelte';

    const app = getState();

    const last100Trades = $derived(app.userTrades.slice(0, 100));
    const totalTrades = $derived(last100Trades.length);
    const winTrades = $derived(last100Trades.filter(t => t.outcome === 'WIN').length);
    const lossTrades = $derived(last100Trades.filter(t => t.outcome === 'LOSS').length);

    const winRate = $derived(totalTrades > 0 ? (winTrades / totalTrades) : 0);

    const avgAchievedReward = $derived.by(() => {
        if (winTrades === 0) return 0;
        const totalReward = last100Trades
            .filter(t => t.outcome === 'WIN')
            .reduce((sum, t) => sum + t.reward_multiplier, 0);
        return totalReward / winTrades;
    });

    const reqBreakevenReward = $derived(winRate > 0 ? ((1 - winRate) / winRate) : 9);

    const expectancyScore = $derived.by(() => {
        if (totalTrades === 0) return 5;
        const avgRiskVal = 1.0;
        const ev = (winRate * avgAchievedReward) - ((1 - winRate) * avgRiskVal);

        if (ev < -0.2) return 1;
        if (ev < 0.0) return 3;
        if (ev === 0.0) return 5;
        if (ev < 0.15) return 6;
        if (ev < 0.35) return 7;
        if (ev < 0.60) return 8;
        if (ev < 1.00) return 9;
        return 10;
    });

    onMount(() => {
        app.fetchTrades();
    });
</script>

<div class="perf-dashboard animate-fade">
    <div class="perf-grid">
        <div class="card score-card">
            <h3 class="card-title">System Rating</h3>
            <div class="score-display">
                <span class="score-value">{expectancyScore}</span>
                <span class="score-max">/10</span>
            </div>
            <div class="score-stats">
                <p><strong>Lookback Depth (Max 100):</strong> {totalTrades} trades</p>
                <p><strong>Wins:</strong> {winTrades} | <strong>Losses:</strong> {lossTrades}</p>
                <p><strong>Calculated Win Rate:</strong> {(winRate * 100).toFixed(1)}%</p>
                <p><strong>Avg Achieved R:R:</strong> 1 : {avgAchievedReward.toFixed(2)}</p>
            </div>
        </div>

        <div class="card matrix-card">
            <h3 class="card-title">Breakeven Reward Matrix</h3>
            <p class="matrix-info">Minimum reward multiplier required to break even versus your current win rate:</p>

            <div class="comparison-row">
                <div class="compare-val">
                    <span class="label">Current Win Rate</span>
                    <span class="value">{(winRate * 100).toFixed(1)}%</span>
                </div>
                <div class="compare-val border-highlight">
                    <span class="label">Min Reward Required</span>
                    <span class="value text-amber">1 : {reqBreakevenReward.toFixed(2)}</span>
                </div>
                <div class="compare-val">
                    <span class="label">Your Achieved Avg</span>
                    <span class="value" class:text-green={avgAchievedReward >= reqBreakevenReward} class:text-red={avgAchievedReward < reqBreakevenReward}>
                        1 : {avgAchievedReward.toFixed(2)}
                    </span>
                </div>
            </div>

            <table class="matrix-table">
                <thead>
                    <tr>
                        <th>Wins / 10</th>
                        <th>Required R:R</th>
                        <th>Status</th>
                    </tr>
                </thead>
                <tbody>
                    <tr class:row-active={winRate <= 0.15 && winRate > 0}>
                        <td>1 of 10 (10%)</td>
                        <td>1 : 9.00</td>
                        <td>{winRate <= 0.15 && winRate > 0 ? '👈 Current Target' : 'Breakeven'}</td>
                    </tr>
                    <tr class:row-active={winRate > 0.15 && winRate <= 0.35}>
                        <td>2 of 10 (20%)</td>
                        <td>1 : 4.00</td>
                        <td>{winRate > 0.15 && winRate <= 0.35 ? '👈 Current Target' : 'Breakeven'}</td>
                    </tr>
                    <tr class:row-active={winRate > 0.35 && winRate <= 0.65}>
                        <td>5 of 10 (50%)</td>
                        <td>1 : 1.00</td>
                        <td>{winRate > 0.35 && winRate <= 0.65 ? '👈 Current Target' : 'Breakeven'}</td>
                    </tr>
                    <tr class:row-active={winRate > 0.65}>
                        <td>8 of 10 (80%)</td>
                        <td>1 : 0.25</td>
                        <td>{winRate > 0.65 ? '👈 Current Target' : 'Breakeven'}</td>
                    </tr>
                </tbody>
            </table>
        </div>

        <div class="card auto-info-card">
            <h3 class="card-title">Automated Telemetry Capture</h3>
            <p class="auto-description">Your trades are calculated and logged automatically as you interact with the visual sidebar panel:</p>
            <ol class="auto-steps">
                <li>Toggle position state to <strong>Long</strong> or <strong>Short</strong>.</li>
                <li>Enter your <strong>Entry Price</strong> and <strong>Stop Loss</strong> triggers.</li>
                <li>When you close the trade by selecting <strong>None</strong>, the visual cockpit snaps the live market price, computes achieved R:R ratio, and records the outcome.</li>
            </ol>
        </div>
    </div>

    <div class="card logs-card">
        <h3 class="card-title">Automated Trade History</h3>
        <div class="logs-table-wrapper">
            {#if app.userTrades.length === 0}
                <p class="empty-msg">No trades logged yet. Set an active position in the sidebar and close it to trigger auto-logging.</p>
            {:else}
                <table class="logs-table">
                    <thead>
                        <tr>
                            <th>Time Stamp</th>
                            <th>Symbol</th>
                            <th>Direction</th>
                            <th>Outcome</th>
                            <th>Realized Ratio</th>
                        </tr>
                    </thead>
                    <tbody>
                        {#each app.userTrades as trade}
                            <tr>
                                <td>{new Date(trade.timestamp * 1000).toLocaleDateString()} {new Date(trade.timestamp * 1000).toLocaleTimeString()}</td>
                                <td>{trade.symbol}</td>
                                <td>{trade.direction}</td>
                                <td class={trade.outcome === 'WIN' ? 'text-green font-bold' : 'text-red'}>{trade.outcome}</td>
                                <td class="mono">1 : {trade.reward_multiplier.toFixed(2)}</td>
                            </tr>
                        {/each}
                    </tbody>
                </table>
            {/if}
        </div>
    </div>
</div>

<style>
    .perf-dashboard {
        max-width: 1400px;
        margin: 0 auto;
        padding: 12px;
        display: flex;
        flex-direction: column;
        gap: 16px;
        font-family: ui-sans-serif, system-ui, sans-serif;
    }
    .perf-grid {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
        gap: 16px;
    }
    .card {
        background-color: #131722;
        border: 1px solid #2a2e39;
        border-radius: 8px;
        padding: 16px;
    }
    .card-title {
        font-size: 13px;
        font-weight: 700;
        color: #f1f5f9;
        margin-top: 0;
        margin-bottom: 12px;
        text-transform: uppercase;
        letter-spacing: 0.05em;
        border-bottom: 1px solid #1e293b;
        padding-bottom: 6px;
    }
    .score-card {
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: space-between;
    }
    .score-display {
        display: flex;
        align-items: baseline;
        margin: 16px 0;
    }
    .score-value {
        font-size: 56px;
        font-weight: 900;
        color: #3b82f6;
    }
    .score-max {
        font-size: 18px;
        color: #64748b;
    }
    .score-stats {
        width: 100%;
        font-size: 11px;
        color: #94a3b8;
        display: flex;
        flex-direction: column;
        gap: 6px;
    }
    .score-stats p { margin: 0; }
    .matrix-card {
        display: flex;
        flex-direction: column;
        justify-content: space-between;
    }
    .matrix-info {
        font-size: 11px;
        color: #64748b;
        margin-top: 0;
        margin-bottom: 12px;
    }
    .comparison-row {
        display: flex;
        gap: 8px;
        margin-bottom: 16px;
    }
    .compare-val {
        flex: 1;
        background: #0f111a;
        padding: 8px;
        border-radius: 6px;
        text-align: center;
        border: 1px solid #1e293b;
    }
    .border-highlight {
        border-color: #ea580c;
    }
    .compare-val .label {
        display: block;
        font-size: 9px;
        color: #64748b;
        text-transform: uppercase;
        margin-bottom: 4px;
    }
    .compare-val .value {
        font-size: 14px;
        font-weight: 800;
    }
    .matrix-table {
        width: 100%;
        border-collapse: collapse;
        font-size: 11px;
    }
    .matrix-table th {
        text-align: left;
        color: #64748b;
        font-size: 10px;
        padding-bottom: 6px;
        border-bottom: 1px solid #1e293b;
    }
    .matrix-table td {
        padding: 6px 0;
        color: #94a3b8;
    }
    .row-active {
        background-color: rgba(234, 88, 12, 0.08);
        border-radius: 4px;
    }
    .row-active td {
        color: #ea580c;
        font-weight: 700;
    }
    .auto-info-card {
        font-size: 11px;
        line-height: 1.5;
        color: #94a3b8;
        display: flex;
        flex-direction: column;
    }
    .auto-description {
        margin-top: 0;
        margin-bottom: 12px;
        color: #94a3b8;
    }
    .auto-steps {
        margin: 0;
        padding-left: 16px;
        display: flex;
        flex-direction: column;
        gap: 10px;
    }
    .auto-steps li::marker {
        color: #3b82f6;
        font-weight: bold;
    }
    .logs-card {
        margin-top: 16px;
    }
    .logs-table-wrapper {
        max-height: 300px;
        overflow-y: auto;
    }
    .logs-table {
        width: 100%;
        border-collapse: collapse;
        font-size: 11px;
    }
    .logs-table th {
        text-align: left;
        color: #64748b;
        position: sticky;
        top: 0;
        background-color: #131722;
        padding: 6px 4px;
        border-bottom: 1px solid #1e293b;
    }
    .logs-table td {
        padding: 6px 4px;
        border-bottom: 1px solid #1e293b;
        color: #94a3b8;
    }
    .text-green { color: #10b981; }
    .text-red { color: #ef4444; }
    .text-amber { color: #f59e0b; }
    .mono { font-family: ui-monospace, monospace; }
    .empty-msg {
        font-size: 11px;
        color: #4c525e;
        text-align: center;
        padding: 20px 0;
    }
</style>
