<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { getState } from '../state.svelte';
    import type { DashboardStats, SystemHeartbeat, InstanceSummary } from '../state.svelte';
    import { createChart, LineSeries } from 'lightweight-charts';
    import type { IChartApi, ISeriesApi, Time } from 'lightweight-charts';

    const app = getState();
    let stats = $state<DashboardStats | null>(null);
    let recommendations = $state<any[]>([]);
    let heartbeat = $state<SystemHeartbeat | null>(null);
    let instances = $state<InstanceSummary[]>([]);
    let loading = $state(true);

    interface PaperPosition {
        symbol: string;
        direction: string;
        entryPrice: number;
        size: number;
        unrealizedPnl: number;
        unrealizedRoi: number;
    }
    let paperPositions = $state<PaperPosition[]>([]);

    let equityContainer = $state<HTMLDivElement | null>(null);
    let equityChart: IChartApi | null = null;
    let equitySeries: ISeriesApi<'Line'> | null = null;

    let compoundedContainer = $state<HTMLDivElement | null>(null);
    let compoundedChart: IChartApi | null = null;
    let compoundedSeries: ISeriesApi<'Line'> | null = null;

    async function fetchAll() {
        loading = true;
        try {
            const [statsRes, recsRes, statusRes, instancesRes] = await Promise.all([
                fetch(`/api/dashboard/stats?initial_capital=${app.sessionCapital}`),
                fetch('/api/historical-recommendations'),
                fetch('/api/system/status'),
                fetch('/api/instances'),
            ]);
            if (statsRes.ok) stats = await statsRes.json();
            if (recsRes.ok) {
                const data = await recsRes.json();
                recommendations = data.recommendations || [];
            }
            if (statusRes.ok) heartbeat = await statusRes.json();
            if (instancesRes.ok) {
                const data = await instancesRes.json();
                instances = data.instances || [];

                // Fetch paper positions for each instance in parallel
                const positions: PaperPosition[] = [];
                const paperResults = await Promise.allSettled(
                    instances.map((inst: InstanceSummary) =>
                        fetch(`/api/paper/status?symbol=${encodeURIComponent(inst.symbol)}-USDT`)
                            .then(r => r.ok ? r.json() : null)
                    )
                );
                for (let i = 0; i < instances.length; i++) {
                    const result = paperResults[i];
                    if (result.status === 'fulfilled' && result.value?.active_position) {
                        const pos = result.value.active_position;
                        positions.push({
                            symbol: instances[i].symbol,
                            direction: pos.direction || '',
                            entryPrice: pos.entry_price ?? 0,
                            size: pos.size ?? 0,
                            unrealizedPnl: result.value.unrealized_pnl ?? 0,
                            unrealizedRoi: result.value.unrealized_roi_pct ?? 0,
                        });
                    }
                }
                paperPositions = positions;
            }
        } catch (_) {
        } finally {
            loading = false;
        }
    }

    function buildEquityChart() {
        if (!equityContainer || !stats?.equity_curve || stats.equity_curve.length === 0) return;
        if (equityChart) { equityChart.remove(); equityChart = null; }

        const data = stats.equity_curve.map(([ts, val]) => ({
            time: ts as Time,
            value: val,
        }));

        equityChart = createChart(equityContainer, {
            autoSize: true,
            layout: { background: { color: '#14142a' }, textColor: '#8f929d', fontSize: 10 },
            grid: { vertLines: { color: '#1e1e3a' }, horzLines: { color: '#1e1e3a' } },
            rightPriceScale: { borderColor: '#2a2a4a' },
            timeScale: { borderColor: '#2a2a4a', visible: true, timeVisible: false },
            handleScale: false,
            handleScroll: false,
        });
        equityChart.timeScale().fitContent();

        equitySeries = equityChart.addSeries(LineSeries, {
            color: '#5b7fff',
            lineWidth: 2,
            priceLineVisible: false,
            crosshairMarkerVisible: false,
        });
        equitySeries.setData(data);
    }

    function buildCompoundedChart() {
        if (!compoundedContainer || !stats?.compounded_curve || stats.compounded_curve.length < 2) return;
        if (compoundedChart) { compoundedChart.remove(); compoundedChart = null; }

        const data = stats.compounded_curve.map(([ts, val]) => ({
            time: ts as Time,
            value: val,
        }));

        compoundedChart = createChart(compoundedContainer, {
            autoSize: true,
            layout: { background: { color: '#14142a' }, textColor: '#8f929d', fontSize: 10 },
            grid: { vertLines: { color: '#1e1e3a' }, horzLines: { color: '#1e1e3a' } },
            rightPriceScale: { borderColor: '#2a2a4a' },
            timeScale: { borderColor: '#2a2a4a', visible: true, timeVisible: false },
            handleScale: false,
            handleScroll: false,
        });
        compoundedChart.timeScale().fitContent();

        compoundedSeries = compoundedChart.addSeries(LineSeries, {
            color: '#f59e0b',
            lineWidth: 2,
            priceLineVisible: false,
            crosshairMarkerVisible: false,
        });
        compoundedSeries.setData(data);
    }

    $effect(() => { fetchAll(); });

    $effect(() => {
        if (!loading && stats?.equity_curve) {
            requestAnimationFrame(() => buildEquityChart());
        }
        if (!loading && stats?.compounded_curve && stats.compounded_curve.length >= 2) {
            requestAnimationFrame(() => buildCompoundedChart());
        }
    });

    onDestroy(() => {
        if (equityChart) { equityChart.remove(); equityChart = null; }
        if (compoundedChart) { compoundedChart.remove(); compoundedChart = null; }
    });
</script>

<div class="dashboard-view">
    <h2>General Dashboard</h2>

    {#if loading}
        <div class="loading-row">Loading dashboard...</div>
    {:else}
        <!-- Overview Cards -->
        <div class="stats-grid">
            <div class="stat-card">
                <span class="stat-label">Total P&L</span>
                <span class="stat-value" class:positive={(stats?.core_stats?.total_pnl ?? 0) >= 0} class:negative={(stats?.core_stats?.total_pnl ?? 0) < 0}>
                    ${Math.abs(stats?.core_stats?.total_pnl ?? 0).toFixed(2)}
                </span>
            </div>
            <div class="stat-card">
                <span class="stat-label">Win Rate</span>
                <span class="stat-value">{((stats?.core_stats?.win_rate ?? 0) * 100).toFixed(1)}%</span>
            </div>
            <div class="stat-card">
                <span class="stat-label">Total Trades</span>
                <span class="stat-value">{stats?.core_stats?.total_trades ?? 0}</span>
            </div>
            <div class="stat-card">
                <span class="stat-label">Expectancy</span>
                <span class="stat-value" class:positive={(stats?.core_stats?.expectancy ?? 0) >= 0} class:negative={(stats?.core_stats?.expectancy ?? 0) < 0}>
                    {stats?.core_stats?.expectancy != null ? `$${Math.abs(stats!.core_stats.expectancy).toFixed(2)}` : '--'}
                </span>
            </div>
            <div class="stat-card">
                <span class="stat-label">Profit Factor</span>
                <span class="stat-value">{(stats?.core_stats?.profit_factor ?? 0).toFixed(2)}</span>
            </div>
            <div class="stat-card">
                <span class="stat-label">Avg R:R Ratio</span>
                <span class="stat-value">{(stats?.core_stats?.avg_risk_reward_ratio ?? 0).toFixed(2)}</span>
            </div>
        </div>

        <!-- Portfolio Overview -->
        {#if heartbeat || instances.length > 0}
            <div class="section-header">
                <h3>Portfolio Overview</h3>
            </div>
            <div class="portfolio-grid">
                <div class="stat-card portfolio-card">
                    <span class="stat-label">Session Capital</span>
                    <span class="stat-value">{app.sessionCurrency} {app.sessionCapital.toLocaleString()}</span>
                </div>
                <div class="stat-card portfolio-card">
                    <span class="stat-label">Active Pairs</span>
                    <span class="stat-value">{heartbeat?.active_pairs_count ?? instances.length}</span>
                </div>
                <div class="stat-card portfolio-card">
                    <span class="stat-label">Allocated Margin</span>
                    <span class="stat-value">${(heartbeat?.total_allocated_margin ?? 0).toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}</span>
                </div>
            </div>

            <!-- Active Instances -->
            {#if instances.length > 0}
                <div class="instances-table-wrapper">
                    <table class="instances-table">
                        <thead>
                            <tr>
                                <th>Pair</th>
                                <th>Status</th>
                                <th>Initial Capital</th>
                                <th>Equity</th>
                                <th>Consec. Losses</th>
                                <th>Caution</th>
                            </tr>
                        </thead>
                        <tbody>
                            {#each instances as inst}
                                <tr>
                                    <td class="col-pair">{inst.symbol}</td>
                                    <td>
                                        <span class="status-badge" class:status-running={inst.status === 'running'} class:status-paused={inst.status === 'paused'} class:status-stopped={inst.status === 'stopped'}>
                                            {inst.status}
                                        </span>
                                    </td>
                                    <td class="col-mono">${inst.initial_capital.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}</td>
                                    <td class="col-mono" class:positive={inst.current_equity >= 0} class:negative={inst.current_equity < 0}>${inst.current_equity.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}</td>
                                    <td class="col-mono">{inst.consecutive_losses}</td>
                                    <td>
                                        <span class="caution-badge" class:caution-normal={inst.caution_level === 'normal'} class:caution-cautious={inst.caution_level === 'cautious'} class:caution-warn={(inst.caution_level === 'suspended' || inst.caution_level === 'drawdown_stop')}>
                                            {inst.caution_level}
                                        </span>
                                    </td>
                                </tr>
                            {/each}
                        </tbody>
                    </table>
                </div>
            {/if}

            <!-- Open Positions -->
            {#if paperPositions.length > 0}
                <div class="section-header" style="margin-top: 1.5rem;">
                    <h3>Open Positions</h3>
                </div>
                <div class="instances-table-wrapper">
                    <table class="instances-table">
                        <thead>
                            <tr>
                                <th>Symbol</th>
                                <th>Direction</th>
                                <th>Entry Price</th>
                                <th>Size</th>
                                <th>Unrealized P&L</th>
                                <th>ROI</th>
                            </tr>
                        </thead>
                        <tbody>
                            {#each paperPositions as pos}
                                <tr>
                                    <td class="col-pair">{pos.symbol}</td>
                                    <td>
                                        <span class="status-badge" class:status-running={pos.direction === 'LONG'} class:status-stopped={pos.direction === 'SHORT'}>
                                            {pos.direction}
                                        </span>
                                    </td>
                                    <td class="col-mono">${pos.entryPrice.toFixed(2)}</td>
                                    <td class="col-mono">{pos.size.toFixed(4)}</td>
                                    <td class="col-mono" class:positive={pos.unrealizedPnl >= 0} class:negative={pos.unrealizedPnl < 0}>
                                        {pos.unrealizedPnl >= 0 ? '+' : ''}${pos.unrealizedPnl.toFixed(2)}
                                    </td>
                                    <td class="col-mono" class:positive={pos.unrealizedRoi >= 0} class:negative={pos.unrealizedRoi < 0}>
                                        {pos.unrealizedRoi >= 0 ? '+' : ''}{pos.unrealizedRoi.toFixed(2)}%
                                    </td>
                                </tr>
                            {/each}
                        </tbody>
                    </table>
                </div>
            {/if}

            <!-- Equity Curve -->
            {#if stats?.equity_curve && stats.equity_curve.length > 0}
                <div class="section-header" style="margin-top: 1.5rem;">
                    <h3>Equity Curve</h3>
                </div>
                <div class="equity-chart-wrapper">
                    <div class="equity-chart-container" bind:this={equityContainer}></div>
                </div>
            {/if}

            <!-- Compounded Balance Curve -->
            {#if stats?.compounded_curve && stats.compounded_curve.length >= 2}
                <div class="section-header" style="margin-top: 1.5rem;">
                    <h3>Compounded Balance Curve</h3>
                </div>
                <div class="equity-chart-wrapper">
                    <div class="equity-chart-container" bind:this={compoundedContainer}></div>
                </div>
            {/if}
        {/if}

        <!-- Detailed Stats -->
        {#if stats?.core_stats}
            <div class="detail-grid">
                <div class="detail-card">
                    <h4>Trade Outcomes</h4>
                    <div class="detail-row">
                        <span>Wins</span>
                        <span class="positive">{stats.core_stats.wins}</span>
                    </div>
                    <div class="detail-row">
                        <span>Losses</span>
                        <span class="negative">{stats.core_stats.losses}</span>
                    </div>
                    <div class="detail-row">
                        <span>Avg Win</span>
                        <span>${stats.core_stats.avg_gain.toFixed(2)}</span>
                    </div>
                    <div class="detail-row">
                        <span>Avg Loss</span>
                        <span>${Math.abs(stats.core_stats.avg_loss).toFixed(2)}</span>
                    </div>
                    <div class="detail-row">
                        <span>Largest Win</span>
                        <span class="positive">${stats.core_stats.largest_gain.toFixed(2)}</span>
                    </div>
                    <div class="detail-row">
                        <span>Largest Loss</span>
                        <span class="negative">${Math.abs(stats.core_stats.largest_loss).toFixed(2)}</span>
                    </div>
                </div>

                <div class="detail-card">
                    <h4>Direction Breakdown</h4>
                    {#if stats.direction_breakdown}
                        <div class="detail-row">
                            <span>Longs</span>
                            <span>{stats.direction_breakdown.longs}</span>
                        </div>
                        <div class="detail-row">
                            <span>Shorts</span>
                            <span>{stats.direction_breakdown.shorts}</span>
                        </div>
                        <div class="detail-row">
                            <span>Long Expectancy</span>
                            <span>${stats.direction_breakdown.long_expectancy.toFixed(2)}</span>
                        </div>
                        <div class="detail-row">
                            <span>Short Expectancy</span>
                            <span>${stats.direction_breakdown.short_expectancy.toFixed(2)}</span>
                        </div>
                        <div class="dir-sub-table">
                            <table class="dir-table">
                                <thead>
                                    <tr>
                                        <th>Dir</th>
                                        <th>Wins</th>
                                        <th>Losses</th>
                                        <th>WR%</th>
                                        <th>AvgG%</th>
                                        <th>AvgL%</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    <tr>
                                        <td class="positive">LONG</td>
                                        <td class="positive">{stats.direction_breakdown.long_wins}</td>
                                        <td class="negative">{stats.direction_breakdown.long_losses}</td>
                                        <td>{stats.direction_breakdown.long_win_rate.toFixed(1)}%</td>
                                        <td class="positive">{stats.direction_breakdown.long_avg_gain.toFixed(2)}%</td>
                                        <td class="negative">-{stats.direction_breakdown.long_avg_loss.toFixed(2)}%</td>
                                    </tr>
                                    <tr>
                                        <td class="negative">SHORT</td>
                                        <td class="positive">{stats.direction_breakdown.short_wins}</td>
                                        <td class="negative">{stats.direction_breakdown.short_losses}</td>
                                        <td>{stats.direction_breakdown.short_win_rate.toFixed(1)}%</td>
                                        <td class="positive">{stats.direction_breakdown.short_avg_gain.toFixed(2)}%</td>
                                        <td class="negative">-{stats.direction_breakdown.short_avg_loss.toFixed(2)}%</td>
                                    </tr>
                                </tbody>
                            </table>
                        </div>
                    {/if}
                </div>

                <div class="detail-card">
                    <h4>Streaks</h4>
                    {#if stats.winning_streaks}
                        <div class="detail-row">
                            <span>Max Wins</span>
                            <span class="positive">{stats.winning_streaks.max_streak_length}</span>
                        </div>
                        <div class="detail-row">
                            <span>Max Losses</span>
                            <span class="negative">{stats.losing_streaks.max_streak_length}</span>
                        </div>
                        <div class="detail-row">
                            <span>Post-Loss Recovery</span>
                            <span>{(stats.post_loss_recovery_pct * 100).toFixed(0)}%</span>
                        </div>
                    {/if}
                </div>
            </div>
        {/if}

        <!-- Historical Analyst Recommendations -->
        {#if recommendations.length > 0}
            <div class="section-header">
                <h3>📊 Historical Analyst Recommendations</h3>
            </div>
            <div class="recommendations-list">
                {#each recommendations.slice(0, 5) as rec}
                    <div class="rec-card">
                        <div class="rec-header">
                            <span class="rec-symbol">{rec.symbol || rec.pair_key}</span>
                            <span class="rec-date">{rec.generated_at?.substring(0, 10)}</span>
                            <span class="rec-stats">
                                WR: {(rec.win_rate * 100).toFixed(0)}% |
                                PF: {rec.profit_factor?.toFixed(2)} |
                                R:R: {rec.avg_risk_reward?.toFixed(2)}
                            </span>
                        </div>
                        {#if rec.key_improvements}
                            <p class="rec-text"><strong>Improvements:</strong> {rec.key_improvements}</p>
                        {/if}
                        {#if rec.risk_recommendation}
                            <p class="rec-text"><strong>Risk:</strong> {rec.risk_recommendation}</p>
                        {/if}
                        {#if rec.regime_analysis}
                            <p class="rec-text"><strong>Regimes:</strong> {rec.regime_analysis}</p>
                        {/if}
                    </div>
                {/each}
            </div>
        {:else}
            <div class="section-header">
                <p class="no-data">No historical analyst recommendations yet. They appear after enough trades accumulate.</p>
            </div>
        {/if}
    {/if}
</div>

<style>
    .dashboard-view {
        padding: 1.5rem;
        color: #cbd5e1;
        max-width: 1100px;
        margin: 0 auto;
    }
    .dashboard-view h2 { margin: 0 0 1rem 0; color: #e0e0ff; font-size: 1.2rem; }
    .stats-grid {
        display: grid;
        grid-template-columns: repeat(3, 1fr);
        gap: 0.75rem;
        margin-bottom: 1.5rem;
    }
    .stat-card {
        background: #14142a;
        border: 1px solid #2a2a4a;
        border-radius: 8px;
        padding: 1rem;
        text-align: center;
    }
    .stat-label {
        display: block;
        font-size: 0.7rem;
        color: #64748b;
        text-transform: uppercase;
        letter-spacing: 0.5px;
        margin-bottom: 0.25rem;
    }
    .stat-value {
        display: block;
        font-size: 1.2rem;
        font-weight: 700;
        color: #e0e0ff;
    }
    .positive { color: #22c55e !important; }
    .negative { color: #ef4444 !important; }
    .portfolio-grid {
        display: grid;
        grid-template-columns: repeat(3, 1fr);
        gap: 0.75rem;
        margin-bottom: 1.5rem;
    }
    .portfolio-card {
        border-color: #2e2e5e;
    }
    .instances-table-wrapper {
        overflow-x: auto;
        margin-bottom: 1.5rem;
        border: 1px solid #2a2a4a;
        border-radius: 8px;
        background: #14142a;
    }
    .instances-table {
        width: 100%;
        border-collapse: collapse;
        font-size: 0.75rem;
    }
    .instances-table thead {
        background: #0e0e24;
    }
    .instances-table th {
        text-align: left;
        padding: 0.5rem 0.75rem;
        font-size: 0.65rem;
        font-weight: 700;
        color: #64748b;
        text-transform: uppercase;
        letter-spacing: 0.5px;
        border-bottom: 1px solid #2a2a4a;
    }
    .instances-table td {
        padding: 0.5rem 0.75rem;
        border-bottom: 1px solid #1e1e3a;
    }
    .instances-table tbody tr:hover {
        background: rgba(91, 127, 255, 0.04);
    }
    .col-pair {
        font-weight: 700;
        color: #5b7fff;
    }
    .col-mono {
        font-family: ui-monospace, monospace;
        font-size: 0.7rem;
    }
    .status-badge {
        display: inline-block;
        padding: 2px 8px;
        border-radius: 3px;
        font-size: 0.6rem;
        font-weight: 700;
        text-transform: uppercase;
        letter-spacing: 0.5px;
        background: #1e1e3a;
        color: #64748b;
    }
    .status-running {
        background: rgba(34, 197, 94, 0.12);
        color: #22c55e;
    }
    .status-paused {
        background: rgba(251, 191, 36, 0.12);
        color: #f59e0b;
    }
    .status-stopped {
        background: rgba(239, 68, 68, 0.12);
        color: #ef4444;
    }
    .caution-badge {
        display: inline-block;
        padding: 2px 8px;
        border-radius: 3px;
        font-size: 0.6rem;
        font-weight: 700;
        text-transform: uppercase;
        letter-spacing: 0.5px;
        background: #1e1e3a;
        color: #64748b;
    }
    .caution-normal {
        background: rgba(34, 197, 94, 0.12);
        color: #22c55e;
    }
    .caution-cautious {
        background: rgba(251, 191, 36, 0.12);
        color: #f59e0b;
    }
    .caution-warn {
        background: rgba(239, 68, 68, 0.12);
        color: #ef4444;
    }
    .equity-chart-wrapper {
        border: 1px solid #2a2a4a;
        border-radius: 8px;
        background: #14142a;
        overflow: hidden;
        margin-bottom: 1.5rem;
    }
    .equity-chart-container {
        width: 100%;
        height: 260px;
    }
    .detail-grid {
        display: grid;
        grid-template-columns: repeat(3, 1fr);
        gap: 0.75rem;
        margin-bottom: 1.5rem;
    }
    .detail-card {
        background: #14142a;
        border: 1px solid #2a2a4a;
        border-radius: 8px;
        padding: 0.75rem 1rem;
    }
    .detail-card h4 {
        margin: 0 0 0.5rem 0;
        font-size: 0.8rem;
        color: #8888aa;
        text-transform: uppercase;
        letter-spacing: 0.5px;
    }
    .detail-row {
        display: flex;
        justify-content: space-between;
        padding: 0.2rem 0;
        font-size: 0.8rem;
        border-bottom: 1px solid #1e1e3a;
    }
    .detail-row:last-child { border-bottom: none; }
    .dir-sub-table { margin-top: 0.5rem; padding-top: 0.5rem; border-top: 1px solid #2a2a4a; }
    .dir-table { width: 100%; border-collapse: collapse; font-size: 0.7rem; font-family: ui-monospace, monospace; }
    .dir-table th { text-align: left; padding: 3px 4px; border-bottom: 1px solid #2a2a4a; color: #64748b; font-weight: 600; font-size: 0.6rem; text-transform: uppercase; }
    .dir-table td { padding: 3px 4px; border-bottom: 1px solid #1e1e3a; color: #cbd5e1; }
    .section-header {
        margin-bottom: 0.75rem;
    }
    .section-header h3 {
        margin: 0 0 0.5rem 0;
        font-size: 0.95rem;
        color: #e0e0ff;
    }
    .no-data { color: #64748b; font-size: 0.8rem; }
    .recommendations-list {
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
    }
    .rec-card {
        background: #14142a;
        border: 1px solid #2a2a4a;
        border-radius: 8px;
        padding: 0.75rem 1rem;
    }
    .rec-header {
        display: flex;
        gap: 0.75rem;
        margin-bottom: 0.35rem;
        font-size: 0.75rem;
    }
    .rec-symbol { font-weight: 700; color: #5b7fff; }
    .rec-date { color: #64748b; }
    .rec-stats { color: #8888aa; }
    .rec-text {
        margin: 0.25rem 0;
        font-size: 0.78rem;
        color: #94a3b8;
        line-height: 1.4;
    }
    .rec-text strong { color: #cbd5e1; }
    .loading-row { text-align: center; padding: 2rem; color: #64748b; }
</style>
