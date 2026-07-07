<script lang="ts">
    import { onMount } from 'svelte';
    import { useAppStore } from '../state.svelte';
    import styles from './PerformanceDashboard.module.css';

    const app = useAppStore();

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
        if (aiLoading) return;
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

    const aiEvaluated1h = $derived(aiPerfData.filter((p: any) => p.direction_correct_1h !== null));
    const aiEvaluated4h = $derived(aiPerfData.filter((p: any) => p.direction_correct_4h !== null));
    const aiEvaluated24h = $derived(aiPerfData.filter((p: any) => p.direction_correct_24h !== null));

    const aiHitRate1h = $derived.by(() => {
        if (aiEvaluated1h.length === 0) return 0;
        const correct = aiEvaluated1h.filter((p: any) => p.direction_correct_1h).length;
        return (correct / aiEvaluated1h.length) * 100;
    });
    const aiHitRate4h = $derived.by(() => {
        if (aiEvaluated4h.length === 0) return 0;
        const correct = aiEvaluated4h.filter((p: any) => p.direction_correct_4h).length;
        return (correct / aiEvaluated4h.length) * 100;
    });
    const aiHitRate24h = $derived.by(() => {
        if (aiEvaluated24h.length === 0) return 0;
        const correct = aiEvaluated24h.filter((p: any) => p.direction_correct_24h).length;
        return (correct / aiEvaluated24h.length) * 100;
    });

    // Paper trading state
    let paperPerfData = $state<any>({
        trades: [], total_trades: 0, wins: 0, losses: 0, win_rate: 0,
        profit_factor: 0, total_pnl: 0, avg_roi: 0, max_drawdown_pct: 0
    });
    let paperLoading = $state(false);
    let paperFetched = $state(false);

    const paperProfitFactorDisplay = $derived(
        paperPerfData.profit_factor === Infinity ? '∞' : paperPerfData.profit_factor.toFixed(2)
    );
    const paperWinRateDisplay = $derived((paperPerfData.win_rate * 100).toFixed(1));

    async function fetchPaperPerformance(symbol?: string) {
        if (paperLoading) return;
        paperLoading = true;
        try {
            const url = symbol ? `/api/paper/performance?symbol=${encodeURIComponent(symbol)}` : '/api/paper/performance';
            const res = await fetch(url);
            if (res.ok) {
                paperPerfData = await res.json();
                paperFetched = true;
            }
        } catch (_) {} finally {
            paperLoading = false;
        }
    }

    onMount(() => {
        app.fetchTrades();
    });
</script>

<div class="{styles.perfDashboard} animate-fade">
    <div class={styles.perfTabs}>
        <button class="{styles.perfTabBtn} {activePerfTab === 'manual' ? styles.perfTabActive : ''}"
                onclick={() => activePerfTab = 'manual'}>
            Manual Trades
        </button>
        <button class="{styles.perfTabBtn} {activePerfTab === 'ai' ? styles.perfTabActive : ''}"
                onclick={() => { activePerfTab = 'ai'; fetchAiPerformance(); }}>
            AI Recommendations
        </button>
        <button class="{styles.perfTabBtn} {activePerfTab === 'paper' ? styles.perfTabActive : ''}"
                onclick={() => { activePerfTab = 'paper'; fetchPaperPerformance(); }}>
            Paper Trade Log
        </button>
    </div>

    {#if activePerfTab === 'manual'}
        <div class={styles.perfGrid}>
            <div class="{styles.card} {styles.scoreCard}">
                <h3 class={styles.cardTitle}>System Rating</h3>
                <div class={styles.scoreDisplay}>
                    <span class={styles.scoreValue}>{expectancyScore}</span>
                    <span class={styles.scoreMax}>/10</span>
                </div>
                <div class={styles.scoreStats}>
                    <p><strong>Lookback Depth (Max 100):</strong> {totalTrades} trades</p>
                    <p><strong>Wins:</strong> {winTrades} | <strong>Losses:</strong> {lossTrades}</p>
                    <p><strong>Calculated Win Rate:</strong> {(winRate * 100).toFixed(1)}%</p>
                    <p><strong>Avg Achieved R:R:</strong> 1 : {avgAchievedReward.toFixed(2)}</p>
                </div>
            </div>

            <div class="{styles.card} {styles.matrixCard}">
                <h3 class={styles.cardTitle}>Breakeven Reward Matrix</h3>
                <p class={styles.matrixInfo}>Minimum reward multiplier required to break even versus your current win rate:</p>

                <div class={styles.comparisonRow}>
                    <div class={styles.compareVal}>
                        <span class={styles.label}>Current Win Rate</span>
                        <span class={styles.value}>{(winRate * 100).toFixed(1)}%</span>
                    </div>
                    <div class="{styles.compareVal} {styles.borderHighlight}">
                        <span class={styles.label}>Min Reward Required</span>
                        <span class="{styles.value} {styles.textAmber}">1 : {reqBreakevenReward.toFixed(2)}</span>
                    </div>
                    <div class={styles.compareVal}>
                        <span class={styles.label}>Your Achieved Avg</span>
                        <span class="{styles.value} {avgAchievedReward >= reqBreakevenReward ? styles.textGreen : ''} {avgAchievedReward < reqBreakevenReward ? styles.textRed : ''}">
                            1 : {avgAchievedReward.toFixed(2)}
                        </span>
                    </div>
                </div>

                <table class={styles.matrixTable}>
                    <thead>
                        <tr>
                            <th>Wins / 10</th>
                            <th>Required R:R</th>
                            <th>Status</th>
                        </tr>
                    </thead>
                    <tbody>
                        <tr class={winRate <= 0.15 && winRate > 0 ? styles.rowActive : ''}>
                            <td>1 of 10 (10%)</td>
                            <td>1 : 9.00</td>
                            <td>{winRate <= 0.15 && winRate > 0 ? 'Current Target' : 'Breakeven'}</td>
                        </tr>
                        <tr class={winRate > 0.15 && winRate <= 0.35 ? styles.rowActive : ''}>
                            <td>2 of 10 (20%)</td>
                            <td>1 : 4.00</td>
                            <td>{winRate > 0.15 && winRate <= 0.35 ? 'Current Target' : 'Breakeven'}</td>
                        </tr>
                        <tr class={winRate > 0.35 && winRate <= 0.65 ? styles.rowActive : ''}>
                            <td>5 of 10 (50%)</td>
                            <td>1 : 1.00</td>
                            <td>{winRate > 0.35 && winRate <= 0.65 ? 'Current Target' : 'Breakeven'}</td>
                        </tr>
                        <tr class={winRate > 0.65 ? styles.rowActive : ''}>
                            <td>8 of 10 (80%)</td>
                            <td>1 : 0.25</td>
                            <td>{winRate > 0.65 ? 'Current Target' : 'Breakeven'}</td>
                        </tr>
                    </tbody>
                </table>
            </div>

            <div class="{styles.card} {styles.autoInfoCard}">
                <h3 class={styles.cardTitle}>Automated Telemetry Capture</h3>
                <p class={styles.autoDescription}>Your trades are calculated and logged automatically as you interact with the visual sidebar panel:</p>
                <ol class={styles.autoSteps}>
                    <li>Toggle position state to <strong>Long</strong> or <strong>Short</strong>.</li>
                    <li>Enter your <strong>Entry Price</strong> and <strong>Stop Loss</strong> triggers.</li>
                    <li>When you close the trade by selecting <strong>None</strong>, the visual cockpit snaps the live market price, computes achieved R:R ratio, and records the outcome.</li>
                </ol>
            </div>
        </div>

        <div class="{styles.card} {styles.logsCard}">
            <h3 class={styles.cardTitle}>Automated Trade History</h3>
            <div class={styles.logsTableWrapper}>
                {#if app.userTrades.length === 0}
                    <p class={styles.emptyMsg}>No trades logged yet. Set an active position in the sidebar and close it to trigger auto-logging.</p>
                {:else}
                    <table class={styles.logsTable}>
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
                                    <td class={trade.outcome === 'WIN' ? styles.textGreen + ' font-bold' : styles.textRed}>{trade.outcome}</td>
                                    <td class={styles.mono}>1 : {trade.reward_multiplier.toFixed(2)}</td>
                                </tr>
                            {/each}
                        </tbody>
                    </table>
                {/if}
            </div>
        </div>
    {:else if activePerfTab === 'ai'}
        <!-- AI Recommendations Tab -->
        <div class={styles.perfGrid}>
            <div class="{styles.card} {styles.scoreCard}">
                <h3 class={styles.cardTitle}>AI Signal Hit Rate</h3>
                <div class={styles.hitRateGrid}>
                    <div class={styles.hitRateItem}>
                        <span class={styles.hitLabel}>1 Hour</span>
                        <span class={styles.hitValue}>{aiHitRate1h.toFixed(1)}%</span>
                        <span class={styles.hitSub}>({aiEvaluated1h.length} eval)</span>
                    </div>
                    <div class={styles.hitRateItem}>
                        <span class={styles.hitLabel}>4 Hours</span>
                        <span class={styles.hitValue}>{aiHitRate4h.toFixed(1)}%</span>
                        <span class={styles.hitSub}>({aiEvaluated4h.length} eval)</span>
                    </div>
                    <div class={styles.hitRateItem}>
                        <span class={styles.hitLabel}>24 Hours</span>
                        <span class={styles.hitValue}>{aiHitRate24h.toFixed(1)}%</span>
                        <span class={styles.hitSub}>({aiEvaluated24h.length} eval)</span>
                    </div>
                </div>
            </div>

            <div class="{styles.card} {styles.matrixCard}">
                <h3 class={styles.cardTitle}>Consensus Distribution</h3>
                <div class={styles.consensusBars}>
                    <div class={styles.consensusRow}>
                        <span class={styles.consensusLabel}>Bullish</span>
                        <div class={styles.consensusBarTrack}>
                            <div class="{styles.consensusBarFill} {styles.bullishFill}" style="width: {aiTotalRuns > 0 ? (aiBullishRuns / aiTotalRuns * 100) : 0}%"></div>
                        </div>
                        <span class={styles.consensusCount}>{aiBullishRuns}</span>
                    </div>
                    <div class={styles.consensusRow}>
                        <span class={styles.consensusLabel}>Bearish</span>
                        <div class={styles.consensusBarTrack}>
                            <div class="{styles.consensusBarFill} {styles.bearishFill}" style="width: {aiTotalRuns > 0 ? (aiBearishRuns / aiTotalRuns * 100) : 0}%"></div>
                        </div>
                        <span class={styles.consensusCount}>{aiBearishRuns}</span>
                    </div>
                    <div class={styles.consensusRow}>
                        <span class={styles.consensusLabel}>Sideways</span>
                        <div class={styles.consensusBarTrack}>
                            <div class="{styles.consensusBarFill} {styles.sidewaysFill}" style="width: {aiTotalRuns > 0 ? (aiSidewaysRuns / aiTotalRuns * 100) : 0}%"></div>
                        </div>
                        <span class={styles.consensusCount}>{aiSidewaysRuns}</span>
                    </div>
                </div>
                <p class="{styles.matrixInfo} {styles.matrixInfoSpacer}">Total automated evaluations: {aiTotalRuns}</p>
            </div>

            <div class="{styles.card} {styles.autoInfoCard}">
                <h3 class={styles.cardTitle}>How It Works</h3>
                <p class={styles.autoDescription}>Automated AI evaluations run independently for each trading pair at your configured interval:</p>
                <ol class={styles.autoSteps}>
                    <li>The scheduler gathers the last 100 candle closes and current indicator values.</li>
                    <li>Phase 1: Seven parallel indicator agents evaluate RSI, MACD, Squeeze, ADX, Bollinger/ATR, Volume/EMA, and VWAP.</li>
                    <li>Phase 2: The master orchestrator synthesizes findings and issues a recommendation.</li>
                    <li>Results are stored with trigger: <strong>"Automated"</strong> and tracked for accuracy over 1h, 4h, and 24h horizons.</li>
                </ol>
            </div>
        </div>

        <div class="{styles.card} {styles.logsCard}">
            <h3 class={styles.cardTitle}>Automated Run History</h3>
            <div class={styles.logsTableWrapper}>
                {#if aiLoading}
                    <p class={styles.emptyMsg}>Loading automated records...</p>
                {:else if aiRecords.length === 0}
                    <p class={styles.emptyMsg}>No automated AI evaluations recorded yet. Enable automation in Workspace Settings.</p>
                {:else}
                    <table class={styles.logsTable}>
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
                                    <td class={rec.trend_classification === 'UPWARD' ? styles.textGreen + ' font-bold' : rec.trend_classification === 'DOWNWARD' ? styles.textRed : styles.textAmber}>
                                        {rec.trend_classification}
                                    </td>
                                    <td>{rec.indicator_alignment}</td>
                                    <td class={rec.recommended_action === 'Open Long' || rec.recommended_action === 'Hold' ? styles.textGreen + ' font-bold' : rec.recommended_action === 'Close' ? styles.textRed : styles.textAmber}>
                                        {rec.recommended_action.substring(0, 10)}
                                    </td>
                                    <td class={styles.mono}>{rec.price_at_analysis.substring(0, 10)}</td>
                                    <td class="{styles.mono} {delta > 0 ? styles.deltaPositive : ''} {delta < 0 ? styles.deltaNegative : ''}">{delta.toFixed(2)}%</td>
                                </tr>
                            {/each}
                        </tbody>
                    </table>
                {/if}
            </div>
        </div>
    {:else if activePerfTab === 'paper'}
        <!-- Paper Trade Log Tab -->
        <div class={styles.perfGrid}>
            <div class="{styles.card} {styles.scoreCard}">
                <h3 class={styles.cardTitle}>Paper Trading Scorecard</h3>
                <div class={styles.hitRateGrid}>
                    <div class={styles.hitRateItem}>
                        <span class={styles.hitLabel}>Profit Factor</span>
                        <span class={styles.hitValue}>{paperProfitFactorDisplay}</span>
                    </div>
                    <div class={styles.hitRateItem}>
                        <span class={styles.hitLabel}>Win Rate</span>
                        <span class={styles.hitValue}>{paperWinRateDisplay}%</span>
                        <span class={styles.hitSub}>{paperPerfData.wins}W / {paperPerfData.losses}L</span>
                    </div>
                    <div class={styles.hitRateItem}>
                        <span class={styles.hitLabel}>Max Drawdown</span>
                        <span class={styles.hitValue} class:pnl-negative={paperPerfData.max_drawdown_pct > 0}>{paperPerfData.max_drawdown_pct.toFixed(2)}%</span>
                    </div>
                </div>
            </div>

            <div class="{styles.card} {styles.matrixCard}">
                <h3 class={styles.cardTitle}>Cumulative Metrics</h3>
                <div class="{styles.comparisonRow} {styles.comparisonCol}">
                    <div class="{styles.compareVal} {styles.compareFlexRow}">
                        <span class={styles.label}>Total P&L</span>
                        <span class={styles.value} class:pnl-positive={paperPerfData.total_pnl >= 0} class:pnl-negative={paperPerfData.total_pnl < 0}>
                            {paperPerfData.total_pnl >= 0 ? '+' : ''}${paperPerfData.total_pnl.toFixed(2)}
                        </span>
                    </div>
                    <div class="{styles.compareVal} {styles.compareFlexRow}">
                        <span class={styles.label}>Avg ROI / Trade</span>
                        <span class={styles.value}>{paperPerfData.avg_roi.toFixed(2)}%</span>
                    </div>
                    <div class="{styles.compareVal} {styles.compareFlexRow}">
                        <span class={styles.label}>Total Trades</span>
                        <span class={styles.value}>{paperPerfData.total_trades}</span>
                    </div>
                </div>
            </div>

            <div class="{styles.card} {styles.autoInfoCard}">
                <h3 class={styles.cardTitle}>About Paper Trading</h3>
                <p class={styles.autoDescription}>Paper trading simulates real trades using virtual capital without financial risk:</p>
                <ol class={styles.autoSteps}>
                    <li>Configure initial balance and per-trade allocation in Workspace Settings.</li>
                    <li>Open positions manually from the Positions tab or let automated AI signals execute them.</li>
                    <li>Track realized P&L, ROI, and performance metrics over time in this dashboard.</li>
                </ol>
            </div>
        </div>

        <div class="{styles.card} {styles.logsCard}">
            <h3 class={styles.cardTitle}>Paper Trade History</h3>
            <div class={styles.logsTableWrapper}>
                {#if paperLoading}
                    <p class={styles.emptyMsg}>Loading records...</p>
                {:else if !paperPerfData.trades || paperPerfData.trades.length === 0}
                    <p class={styles.emptyMsg}>No paper trades recorded yet. Open a position from the Positions tab.</p>
                {:else}
                    <table class={styles.logsTable}>
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
                                    <td class={trade.direction === 'LONG' ? styles.textGreen + ' font-bold' : styles.textRed}>{trade.direction}</td>
                                    <td class={styles.mono}>{trade.entry_price.toFixed(2)}</td>
                                    <td class={styles.mono}>{trade.exit_price.toFixed(2)}</td>
                                    <td class={styles.mono} class:pnl-positive={trade.realized_pnl >= 0} class:pnl-negative={trade.realized_pnl < 0}>
                                        {trade.realized_pnl >= 0 ? '+' : ''}{trade.realized_pnl.toFixed(2)}
                                    </td>
                                    <td class={styles.mono}>{trade.roi_pct.toFixed(2)}%</td>
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

