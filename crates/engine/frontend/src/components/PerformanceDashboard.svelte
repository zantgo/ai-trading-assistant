<script lang="ts">
    import { onMount } from 'svelte';
    import { getState } from '../state.svelte';

    const app = getState();

    let symbol = $state('');
    let direction = $state<'Long' | 'Short'>('Long');
    let outcome = $state<'WIN' | 'LOSS'>('WIN');
    let risk = $state(1);
    let rewardRatio = $state(2);

    let isSubmitting = $state(false);
    let submitMsg = $state('');

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
        if (totalTrades === 0) return 1;
        const avgRiskVal = 1.0;
        const ev = (winRate * avgAchievedReward) - ((1 - winRate) * avgRiskVal);
        if (ev <= -0.5) return 1;
        if (ev >= 2.0) return 10;
        return Math.min(10, Math.max(1, Math.round(1 + ((ev + 0.5) / 2.5) * 9)));
    });

    async function handleLogTrade() {
        if (!symbol.trim()) return;
        isSubmitting = true;
        submitMsg = '';

        try {
            const res = await fetch('/api/trades', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    symbol: symbol.trim().toUpperCase(),
                    direction,
                    outcome,
                    risk_multiplier: risk,
                    reward_multiplier: Number(rewardRatio)
                })
            });

            if (res.ok) {
                submitMsg = 'Trade logged successfully!';
                symbol = '';
                await app.fetchTrades();
            } else {
                submitMsg = 'Logging failed. Try again.';
            }
        } catch (e) {
            submitMsg = 'Network error.';
        } finally {
            isSubmitting = false;
        }
    }

    onMount(() => {
        app.fetchTrades();
    });
</script>

<div class="perf-dashboard animate-fade">
    <div class="trend-banner">
        <h2>🔥 THE TREND IS MORE IMPORTANT THAN INDICATORS 🔥</h2>
        <p>Mathematical expectancy secures capital. Trend alignment secures the targets.</p>
    </div>

    <div class="perf-grid">
        <div class="card score-card">
            <h3 class="card-title">System Rating</h3>
            <div class="score-display">
                <span class="score-value">{expectancyScore}</span>
                <span class="score-max">/10</span>
            </div>
            <div class="score-stats">
                <p><strong>Total Sequence (Max 100):</strong> {totalTrades}</p>
                <p><strong>Wins:</strong> {winTrades} | <strong>Losses:</strong> {lossTrades}</p>
                <p><strong>Realized Win Rate:</strong> {(winRate * 100).toFixed(1)}%</p>
                <p><strong>Avg. Realized R:R:</strong> 1 : {avgAchievedReward.toFixed(2)}</p>
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
                        <td>{winRate <= 0.15 && winRate > 0 ? '👈 Current Range' : 'Breakeven'}</td>
                    </tr>
                    <tr class:row-active={winRate > 0.15 && winRate <= 0.35}>
                        <td>2 of 10 (20%)</td>
                        <td>1 : 4.00</td>
                        <td>{winRate > 0.15 && winRate <= 0.35 ? '👈 Current Range' : 'Breakeven'}</td>
                    </tr>
                    <tr class:row-active={winRate > 0.35 && winRate <= 0.65}>
                        <td>5 of 10 (50%)</td>
                        <td>1 : 1.00</td>
                        <td>{winRate > 0.35 && winRate <= 0.65 ? '👈 Current Range' : 'Breakeven'}</td>
                    </tr>
                    <tr class:row-active={winRate > 0.65}>
                        <td>8 of 10 (80%)</td>
                        <td>1 : 0.25</td>
                        <td>{winRate > 0.65 ? '👈 Current Range' : 'Breakeven'}</td>
                    </tr>
                </tbody>
            </table>
        </div>

        <div class="card form-card">
            <h3 class="card-title">Log Trade Outcome</h3>
            <form class="log-form" onsubmit={(e) => { e.preventDefault(); handleLogTrade(); }}>
                <div class="form-row">
                    <label for="symbol">Symbol:</label>
                    <input id="symbol" type="text" placeholder="BTC" bind:value={symbol} required />
                </div>

                <!-- svelte-ignore a11y_label_has_associated_control -->
                <div class="form-row">
                    <label>Direction:</label>
                    <div class="toggle-group">
                        <button type="button" class:active={direction === 'Long'} onclick={() => direction = 'Long'}>Long</button>
                        <button type="button" class:active={direction === 'Short'} onclick={() => direction = 'Short'}>Short</button>
                    </div>
                </div>

                <!-- svelte-ignore a11y_label_has_associated_control -->
                <div class="form-row">
                    <label>Outcome:</label>
                    <div class="toggle-group">
                        <button type="button" class="btn-win" class:active={outcome === 'WIN'} onclick={() => outcome = 'WIN'}>WIN</button>
                        <button type="button" class="btn-loss" class:active={outcome === 'LOSS'} onclick={() => outcome = 'LOSS'}>LOSS</button>
                    </div>
                </div>

                <div class="form-row">
                    <label for="rewardRatio">Achieved Reward Ratio (Risk is 1):</label>
                    <input id="rewardRatio" type="number" step="0.05" min="0.01" bind:value={rewardRatio} />
                </div>

                <button type="submit" class="submit-btn" disabled={isSubmitting}>
                    {isSubmitting ? 'Logging...' : 'Save Record'}
                </button>
                {#if submitMsg}
                    <div class="submit-msg" class:text-green={submitMsg.includes('successfully')}>{submitMsg}</div>
                {/if}
            </form>
        </div>
    </div>

    <div class="card logs-card">
        <h3 class="card-title">Trade History (Last 100)</h3>
        <div class="logs-table-wrapper">
            {#if app.userTrades.length === 0}
                <p class="empty-msg">No trades logged yet.</p>
            {:else}
                <table class="logs-table">
                    <thead>
                        <tr>
                            <th>Time</th>
                            <th>Market</th>
                            <th>Type</th>
                            <th>Outcome</th>
                            <th>Achieved Ratio</th>
                        </tr>
                    </thead>
                    <tbody>
                        {#each app.userTrades as trade}
                            <tr>
                                <td>{new Date(trade.timestamp * 1000).toLocaleDateString()}</td>
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
    .trend-banner {
        background: linear-gradient(135deg, #1e1b4b, #0c0a09);
        border: 2px solid #ea580c;
        border-radius: 8px;
        padding: 20px;
        text-align: center;
        box-shadow: 0 10px 15px -3px rgba(0, 0, 0, 0.3);
    }
    .trend-banner h2 {
        color: #f97316;
        margin: 0 0 8px 0;
        font-size: 20px;
        letter-spacing: 0.05em;
    }
    .trend-banner p {
        color: #94a3b8;
        font-size: 13px;
        margin: 0;
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
    .log-form {
        display: flex;
        flex-direction: column;
        gap: 12px;
    }
    .form-row {
        display: flex;
        flex-direction: column;
        gap: 4px;
    }
    .form-row label {
        font-size: 10px;
        color: #64748b;
        font-weight: 600;
    }
    .form-row input {
        background-color: #0f111a;
        border: 1px solid #2a2e39;
        color: #cbd5e1;
        padding: 6px 10px;
        border-radius: 4px;
        font-size: 12px;
        outline: none;
    }
    .toggle-group {
        display: flex;
        gap: 4px;
    }
    .toggle-group button {
        flex: 1;
        background-color: #0f111a;
        border: 1px solid #2a2e39;
        color: #8f929d;
        padding: 6px 0;
        font-size: 10px;
        font-weight: 700;
        border-radius: 4px;
        cursor: pointer;
    }
    .toggle-group button.active {
        background-color: rgba(59, 130, 246, 0.15);
        border-color: #3b82f6;
        color: #3b82f6;
    }
    .toggle-group button.btn-win.active {
        background-color: rgba(16, 185, 129, 0.15);
        border-color: #10b981;
        color: #10b981;
    }
    .toggle-group button.btn-loss.active {
        background-color: rgba(239, 68, 68, 0.15);
        border-color: #ef4444;
        color: #ef4444;
    }
    .submit-btn {
        background: linear-gradient(135deg, #1e40af, #3b82f6);
        border: 1px solid #3b82f6;
        color: white;
        padding: 8px 0;
        font-size: 11px;
        font-weight: 700;
        text-transform: uppercase;
        border-radius: 4px;
        cursor: pointer;
    }
    .submit-msg {
        font-size: 10px;
        text-align: center;
        color: #ef4444;
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
