<script lang="ts">
    import { onMount } from 'svelte';
    import { getState } from '../state.svelte';

    const app = getState();

    let activePerfTab = $state<'manual' | 'ai' | 'paper'>('manual');

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

    // AI performance state
    let aiRecords = $state<any[]>([]);
    let aiPerfData = $state<any[]>([]);
    let aiLoading = $state(false);

    async function fetchAiPerformance() {
        aiLoading = true;
        try {
            const [recordsRes, perfRes] = await Promise.all([
                fetch('/api/assistant-records?trigger_type=Automated'),
                fetch('/api/automated-performance'),
            ]);
            if (recordsRes.ok) {
                const data = await recordsRes.json();
                aiRecords = data.records || [];
            }
            if (perfRes.ok) {
                aiPerfData = await perfRes.json();
            }
        } catch (_) {} finally {
            aiLoading = false;
        }
    }

    const aiTotalRuns = $derived(aiRecords.length);
    const aiBullishRuns = $derived(aiRecords.filter((r: any) =>
        r.trend_classification === 'UPWARD').length);
    const aiBearishRuns = $derived(aiRecords.filter((r: any) =>
        r.trend_classification === 'DOWNWARD').length);
    const aiSidewaysRuns = $derived(aiRecords.filter((r: any) =>
        r.trend_classification === 'SIDEWAYS').length);

    const aiHitRate1h = $derived.by(() => {
        const evaluated = aiPerfData.filter((p: any) => p.direction_correct_1h !== null);
        if (evaluated.length === 0) return 0;
        const correct = evaluated.filter((p: any) => p.direction_correct_1h).length;
        return (correct / evaluated.length) * 100;
    });
    const aiHitRate4h = $derived.by(() => {
        const evaluated = aiPerfData.filter((p: any) => p.direction_correct_4h !== null);
        if (evaluated.length === 0) return 0;
        const correct = evaluated.filter((p: any) => p.direction_correct_4h).length;
        return (correct / evaluated.length) * 100;
    });
    const aiHitRate24h = $derived.by(() => {
        const evaluated = aiPerfData.filter((p: any) => p.direction_correct_24h !== null);
        if (evaluated.length === 0) return 0;
        const correct = evaluated.filter((p: any) => p.direction_correct_24h).length;
        return (correct / evaluated.length) * 100;
    });

    // Paper trading state
    let paperPerfData = $state<any>({
        trades: [], total_trades: 0, wins: 0, losses: 0, win_rate: 0,
        profit_factor: 0, total_pnl: 0, avg_roi: 0, max_drawdown_pct: 0
    });
    let paperLoading = $state(false);

    async function fetchPaperPerformance(symbol?: string) {
        paperLoading = true;
        try {
            const url = symbol ? `/api/paper/performance?symbol=${encodeURIComponent(symbol)}` : '/api/paper/performance';
            const res = await fetch(url);
            if (res.ok) {
                paperPerfData = await res.json();
            }
        } catch (_) {} finally {
            paperLoading = false;
        }
    }

    onMount(() => {
        app.fetchTrades();
    });
</script>

<div class="perf-dashboard animate-fade">
    <div class="perf-tabs">
        <button class="perf-tab-btn" class:perf-tab-active={activePerfTab === 'manual'}
                onclick={() => activePerfTab = 'manual'}>
            Manual Trades
        </button>
        <button class="perf-tab-btn" class:perf-tab-active={activePerfTab === 'ai'}
                onclick={() => { activePerfTab = 'ai'; fetchAiPerformance(); }}>
            AI Recommendations
        </button>
        <button class="perf-tab-btn" class:perf-tab-active={activePerfTab === 'paper'}
                onclick={() => { activePerfTab = 'paper'; fetchPaperPerformance(); }}>
            Paper Trade Log
        </button>
    </div>

    {#if activePerfTab === 'manual'}
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
                            <td>{winRate <= 0.15 && winRate > 0 ? 'Current Target' : 'Breakeven'}</td>
                        </tr>
                        <tr class:row-active={winRate > 0.15 && winRate <= 0.35}>
                            <td>2 of 10 (20%)</td>
                            <td>1 : 4.00</td>
                            <td>{winRate > 0.15 && winRate <= 0.35 ? 'Current Target' : 'Breakeven'}</td>
                        </tr>
                        <tr class:row-active={winRate > 0.35 && winRate <= 0.65}>
                            <td>5 of 10 (50%)</td>
                            <td>1 : 1.00</td>
                            <td>{winRate > 0.35 && winRate <= 0.65 ? 'Current Target' : 'Breakeven'}</td>
                        </tr>
                        <tr class:row-active={winRate > 0.65}>
                            <td>8 of 10 (80%)</td>
                            <td>1 : 0.25</td>
                            <td>{winRate > 0.65 ? 'Current Target' : 'Breakeven'}</td>
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
    {:else if activePerfTab === 'ai'}
        <!-- AI Recommendations Tab -->
        <div class="perf-grid">
            <div class="card score-card">
                <h3 class="card-title">AI Signal Hit Rate</h3>
                <div class="hit-rate-grid">
                    <div class="hit-rate-item">
                        <span class="hit-label">1 Hour</span>
                        <span class="hit-value">{aiHitRate1h.toFixed(1)}%</span>
                        <span class="hit-sub">({aiPerfData.filter((p: any) => p.direction_correct_1h !== null).length} eval)</span>
                    </div>
                    <div class="hit-rate-item">
                        <span class="hit-label">4 Hours</span>
                        <span class="hit-value">{aiHitRate4h.toFixed(1)}%</span>
                        <span class="hit-sub">({aiPerfData.filter((p: any) => p.direction_correct_4h !== null).length} eval)</span>
                    </div>
                    <div class="hit-rate-item">
                        <span class="hit-label">24 Hours</span>
                        <span class="hit-value">{aiHitRate24h.toFixed(1)}%</span>
                        <span class="hit-sub">({aiPerfData.filter((p: any) => p.direction_correct_24h !== null).length} eval)</span>
                    </div>
                </div>
            </div>

            <div class="card matrix-card">
                <h3 class="card-title">Consensus Distribution</h3>
                <div class="consensus-bars">
                    <div class="consensus-row">
                        <span class="consensus-label">Bullish</span>
                        <div class="consensus-bar-track">
                            <div class="consensus-bar-fill bullish-fill" style="width: {aiTotalRuns > 0 ? (aiBullishRuns / aiTotalRuns * 100) : 0}%"></div>
                        </div>
                        <span class="consensus-count">{aiBullishRuns}</span>
                    </div>
                    <div class="consensus-row">
                        <span class="consensus-label">Bearish</span>
                        <div class="consensus-bar-track">
                            <div class="consensus-bar-fill bearish-fill" style="width: {aiTotalRuns > 0 ? (aiBearishRuns / aiTotalRuns * 100) : 0}%"></div>
                        </div>
                        <span class="consensus-count">{aiBearishRuns}</span>
                    </div>
                    <div class="consensus-row">
                        <span class="consensus-label">Sideways</span>
                        <div class="consensus-bar-track">
                            <div class="consensus-bar-fill sideways-fill" style="width: {aiTotalRuns > 0 ? (aiSidewaysRuns / aiTotalRuns * 100) : 0}%"></div>
                        </div>
                        <span class="consensus-count">{aiSidewaysRuns}</span>
                    </div>
                </div>
                <p class="matrix-info" style="margin-top: 10px;">Total automated evaluations: {aiTotalRuns}</p>
            </div>

            <div class="card auto-info-card">
                <h3 class="card-title">How It Works</h3>
                <p class="auto-description">Automated AI evaluations run independently for each trading pair at your configured interval:</p>
                <ol class="auto-steps">
                    <li>The scheduler gathers the last 100 candle closes and current indicator values.</li>
                    <li>Phase 1: Seven parallel indicator agents evaluate RSI, MACD, Squeeze, ADX, Bollinger/ATR, Volume/EMA, and VWAP.</li>
                    <li>Phase 2: The master orchestrator synthesizes findings and issues a recommendation.</li>
                    <li>Results are stored with trigger: <strong>"Automated"</strong> and tracked for accuracy over 1h, 4h, and 24h horizons.</li>
                </ol>
            </div>
        </div>

        <div class="card logs-card">
            <h3 class="card-title">Automated Run History</h3>
            <div class="logs-table-wrapper">
                {#if aiLoading}
                    <p class="empty-msg">Loading automated records...</p>
                {:else if aiRecords.length === 0}
                    <p class="empty-msg">No automated AI evaluations recorded yet. Enable automation in Workspace Settings.</p>
                {:else}
                    <table class="logs-table">
                        <thead>
                            <tr>
                                <th>Time</th>
                                <th>Symbol</th>
                                <th>Trend</th>
                                <th>Consensus</th>
                                <th>Action</th>
                                <th>Price @ Analysis</th>
                                <th>Δ% (vs latest)</th>
                            </tr>
                        </thead>
                        <tbody>
                            {#each aiRecords as rec}
                                {@const recPrice = parseFloat(rec.price_at_analysis) || 0}
                                {@const latest = parseFloat(app.historyLatestClose) || 0}
                                {@const delta = recPrice > 0 ? ((latest - recPrice) / recPrice * 100) : 0}
                                <tr>
                                    <td>{rec.created_at.substring(0, 19)}</td>
                                    <td>{rec.symbol}</td>
                                    <td class={rec.trend_classification === 'UPWARD' ? 'text-green font-bold' : rec.trend_classification === 'DOWNWARD' ? 'text-red' : 'text-amber'}>
                                        {rec.trend_classification}
                                    </td>
                                    <td>{rec.indicator_alignment}</td>
                                    <td class={rec.recommended_action === 'Open Long' || rec.recommended_action === 'Hold' ? 'text-green font-bold' : rec.recommended_action === 'Close' ? 'text-red' : 'text-amber'}>
                                        {rec.recommended_action.substring(0, 10)}
                                    </td>
                                    <td class="mono">{rec.price_at_analysis.substring(0, 10)}</td>
                                    <td class="mono" class:delta-positive={delta > 0} class:delta-negative={delta < 0}>{delta.toFixed(2)}%</td>
                                </tr>
                            {/each}
                        </tbody>
                    </table>
                {/if}
            </div>
        </div>
    {:else if activePerfTab === 'paper'}
        <!-- Paper Trade Log Tab -->
        <div class="perf-grid">
            <div class="card score-card">
                <h3 class="card-title">Paper Trading Scorecard</h3>
                <div class="hit-rate-grid">
                    <div class="hit-rate-item">
                        <span class="hit-label">Profit Factor</span>
                        <span class="hit-value">{paperPerfData.profit_factor === Infinity ? '∞' : paperPerfData.profit_factor.toFixed(2)}</span>
                    </div>
                    <div class="hit-rate-item">
                        <span class="hit-label">Win Rate</span>
                        <span class="hit-value">{(paperPerfData.win_rate * 100).toFixed(1)}%</span>
                        <span class="hit-sub">{paperPerfData.wins}W / {paperPerfData.losses}L</span>
                    </div>
                    <div class="hit-rate-item">
                        <span class="hit-label">Max Drawdown</span>
                        <span class="hit-value" class:pnl-negative={paperPerfData.max_drawdown_pct > 0}>{paperPerfData.max_drawdown_pct.toFixed(2)}%</span>
                    </div>
                </div>
            </div>

            <div class="card matrix-card">
                <h3 class="card-title">Cumulative Metrics</h3>
                <div class="comparison-row" style="flex-direction: column; gap: 6px;">
                    <div class="compare-val" style="display: flex; justify-content: space-between;">
                        <span class="label">Total P&L</span>
                        <span class="value" class:pnl-positive={paperPerfData.total_pnl >= 0} class:pnl-negative={paperPerfData.total_pnl < 0}>
                            {paperPerfData.total_pnl >= 0 ? '+' : ''}${paperPerfData.total_pnl.toFixed(2)}
                        </span>
                    </div>
                    <div class="compare-val" style="display: flex; justify-content: space-between;">
                        <span class="label">Avg ROI / Trade</span>
                        <span class="value">{paperPerfData.avg_roi.toFixed(2)}%</span>
                    </div>
                    <div class="compare-val" style="display: flex; justify-content: space-between;">
                        <span class="label">Total Trades</span>
                        <span class="value">{paperPerfData.total_trades}</span>
                    </div>
                </div>
            </div>

            <div class="card auto-info-card">
                <h3 class="card-title">About Paper Trading</h3>
                <p class="auto-description">Paper trading simulates real trades using virtual capital without financial risk:</p>
                <ol class="auto-steps">
                    <li>Configure initial balance and per-trade allocation in Workspace Settings.</li>
                    <li>Open positions manually from the Positions tab or let automated AI signals execute them.</li>
                    <li>Track realized P&L, ROI, and performance metrics over time in this dashboard.</li>
                </ol>
            </div>
        </div>

        <div class="card logs-card">
            <h3 class="card-title">Paper Trade History</h3>
            <div class="logs-table-wrapper">
                {#if paperLoading}
                    <p class="empty-msg">Loading records...</p>
                {:else if !paperPerfData.trades || paperPerfData.trades.length === 0}
                    <p class="empty-msg">No paper trades recorded yet. Open a position from the Positions tab.</p>
                {:else}
                    <table class="logs-table">
                        <thead>
                            <tr>
                                <th>Entry Time</th>
                                <th>Exit Time</th>
                                <th>Symbol</th>
                                <th>Dir</th>
                                <th>Entry $</th>
                                <th>Exit $</th>
                                <th>P&L</th>
                                <th>ROI</th>
                                <th>Trigger</th>
                            </tr>
                        </thead>
                        <tbody>
                            {#each paperPerfData.trades as trade}
                                <tr>
                                    <td>{new Date(trade.entry_timestamp).toLocaleString()}</td>
                                    <td>{new Date(trade.exit_timestamp).toLocaleString()}</td>
                                    <td>{trade.symbol}</td>
                                    <td class={trade.direction === 'LONG' ? 'text-green font-bold' : 'text-red'}>{trade.direction}</td>
                                    <td class="mono">{trade.entry_price.toFixed(2)}</td>
                                    <td class="mono">{trade.exit_price.toFixed(2)}</td>
                                    <td class="mono" class:pnl-positive={trade.realized_pnl >= 0} class:pnl-negative={trade.realized_pnl < 0}>
                                        {trade.realized_pnl >= 0 ? '+' : ''}{trade.realized_pnl.toFixed(2)}
                                    </td>
                                    <td class="mono">{trade.roi_pct.toFixed(2)}%</td>
                                    <td>{trade.trigger}</td>
                                </tr>
                            {/each}
                        </tbody>
                    </table>
                {/if}
            </div>
        </div>
    {/if}
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
    .perf-tabs {
        display: flex;
        gap: 6px;
        margin-bottom: 4px;
    }
    .perf-tab-btn {
        background: transparent;
        border: 1px solid #2a2e39;
        color: #64748b;
        font-size: 11px;
        font-weight: 700;
        cursor: pointer;
        padding: 6px 14px;
        border-radius: 4px;
        transition: all 0.2s;
        text-transform: uppercase;
    }
    .perf-tab-btn:hover { color: #cbd5e1; background-color: rgba(255, 255, 255, 0.02); }
    .perf-tab-active { background: #1a2030; border-color: #3b82f6; color: #f8fafc; }
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
    .hit-rate-grid {
        display: flex;
        flex-direction: column;
        gap: 12px;
        width: 100%;
    }
    .hit-rate-item {
        display: flex;
        align-items: baseline;
        justify-content: space-between;
        padding: 8px 12px;
        background: #0f111a;
        border-radius: 6px;
        border: 1px solid #1e293b;
    }
    .hit-label { font-size: 11px; font-weight: 700; color: #64748b; text-transform: uppercase; }
    .hit-value { font-size: 22px; font-weight: 900; color: #3b82f6; }
    .hit-sub { font-size: 9px; color: #4c525e; }
    .consensus-bars { display: flex; flex-direction: column; gap: 10px; }
    .consensus-row { display: flex; align-items: center; gap: 8px; }
    .consensus-label { width: 70px; font-size: 10px; font-weight: 700; color: #94a3b8; text-transform: uppercase; }
    .consensus-bar-track { flex: 1; height: 10px; background: #0f111a; border-radius: 4px; overflow: hidden; }
    .consensus-bar-fill { height: 100%; border-radius: 4px; transition: width 0.4s ease; }
    .bullish-fill { background: linear-gradient(90deg, #10b981, #34d399); }
    .bearish-fill { background: linear-gradient(90deg, #ef4444, #f87171); }
    .sideways-fill { background: linear-gradient(90deg, #f59e0b, #fbbf24); }
    .consensus-count { width: 30px; text-align: right; font-size: 11px; font-weight: 700; color: #e2e8f0; }
    .delta-positive { color: #10b981; }
    .delta-negative { color: #ef4444; }
</style>
