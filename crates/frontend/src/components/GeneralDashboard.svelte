<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { useAppStore } from '../state.svelte';
    import type { DashboardStats, SystemHeartbeat, InstanceSummary } from '../types';
    import { createChart, LineSeries } from 'lightweight-charts';
    import type { IChartApi, ISeriesApi, Time } from 'lightweight-charts';
    import styles from './GeneralDashboard.module.css';

    const app = useAppStore();
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

    let ro: ResizeObserver;

    onMount(() => {
        ro = new ResizeObserver(() => {
            if (equityContainer) {
                const w = equityContainer.clientWidth, h = equityContainer.clientHeight;
                if (equityChart && w > 0 && h > 0) equityChart.resize(w, h);
            }
            if (compoundedContainer) {
                const w = compoundedContainer.clientWidth, h = compoundedContainer.clientHeight;
                if (compoundedChart && w > 0 && h > 0) compoundedChart.resize(w, h);
            }
        });
    });

    $effect(() => {
        if (ro) {
            if (equityContainer) ro.observe(equityContainer);
            if (compoundedContainer) ro.observe(compoundedContainer);
        }
    });

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

        if (!equityContainer) return;
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

        if (!compoundedContainer) return;
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
        ro?.disconnect();
        if (equityChart) { equityChart.remove(); equityChart = null; equitySeries = null; }
        if (compoundedChart) { compoundedChart.remove(); compoundedChart = null; compoundedSeries = null; }
    });
</script>

<div class={styles.dashboardView}>
    <h2>General Dashboard</h2>

    {#if loading}
        <div class={styles.loadingRow}>Loading dashboard...</div>
    {:else}
        <!-- Overview Cards -->
        <div class={styles.statsGrid}>
            <div class={styles.statCard}>
                <span class={styles.statLabel}>Total P&L</span>
                <span class="{styles.statValue} {(stats?.core_stats?.total_pnl ?? 0) >= 0 ? styles.positive : ''} {(stats?.core_stats?.total_pnl ?? 0) < 0 ? styles.negative : ''}">
                    ${Math.abs(stats?.core_stats?.total_pnl ?? 0).toFixed(2)}
                </span>
            </div>
            <div class={styles.statCard}>
                <span class={styles.statLabel}>Win Rate</span>
                <span class={styles.statValue}>{((stats?.core_stats?.win_rate ?? 0) * 100).toFixed(1)}%</span>
            </div>
            <div class={styles.statCard}>
                <span class={styles.statLabel}>Total Trades</span>
                <span class={styles.statValue}>{stats?.core_stats?.total_trades ?? 0}</span>
            </div>
            <div class={styles.statCard}>
                <span class={styles.statLabel}>Expectancy</span>
                <span class="{styles.statValue} {(stats?.core_stats?.expectancy ?? 0) >= 0 ? styles.positive : ''} {(stats?.core_stats?.expectancy ?? 0) < 0 ? styles.negative : ''}">
                    {stats?.core_stats?.expectancy != null ? `$${Math.abs(stats!.core_stats.expectancy).toFixed(2)}` : '--'}
                </span>
            </div>
            <div class={styles.statCard}>
                <span class={styles.statLabel}>Profit Factor</span>
                <span class={styles.statValue}>{(stats?.core_stats?.profit_factor ?? 0).toFixed(2)}</span>
            </div>
            <div class={styles.statCard}>
                <span class={styles.statLabel}>Avg R:R Ratio</span>
                <span class={styles.statValue}>{(stats?.core_stats?.avg_risk_reward_ratio ?? 0).toFixed(2)}</span>
            </div>
        </div>

        <!-- Portfolio Overview -->
        {#if heartbeat || instances.length > 0}
            <div class={styles.sectionHeader}>
                <h3>Portfolio Overview</h3>
            </div>
            <div class={styles.portfolioGrid}>
                <div class="{styles.statCard} {styles.portfolioCard}">
                    <span class={styles.statLabel}>Session Capital</span>
                    <span class={styles.statValue}>{app.sessionCurrency} {app.sessionCapital.toLocaleString()}</span>
                </div>
                <div class="{styles.statCard} {styles.portfolioCard}">
                    <span class={styles.statLabel}>Active Pairs</span>
                    <span class={styles.statValue}>{heartbeat?.active_pairs_count ?? instances.length}</span>
                </div>
                <div class="{styles.statCard} {styles.portfolioCard}">
                    <span class={styles.statLabel}>Allocated Margin</span>
                    <span class={styles.statValue}>${(heartbeat?.total_allocated_margin ?? 0).toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}</span>
                </div>
            </div>

            <!-- Active Instances -->
            {#if instances.length > 0}
                <div class={styles.instancesTableWrapper}>
                    <table class={styles.instancesTable}>
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
                                    <td class={styles.colPair}>{inst.symbol}</td>
                                    <td>
                                        <span class="{styles.statusBadge} {inst.status === 'running' ? styles.statusRunning : ''} {inst.status === 'paused' ? styles.statusPaused : ''} {inst.status === 'stopped' ? styles.statusStopped : ''}">
                                            {inst.status}
                                        </span>
                                    </td>
                                    <td class={styles.colMono}>${inst.initial_capital.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}</td>
                                    <td class="{styles.colMono} {inst.current_equity >= 0 ? styles.positive : ''} {inst.current_equity < 0 ? styles.negative : ''}">${inst.current_equity.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}</td>
                                    <td class={styles.colMono}>{inst.consecutive_losses}</td>
                                    <td>
                                        <span class="{styles.cautionBadge} {inst.caution_level === 'normal' ? styles.cautionNormal : ''} {inst.caution_level === 'cautious' ? styles.cautionCautious : ''} {(inst.caution_level === 'suspended' || inst.caution_level === 'drawdown_stop') ? styles.cautionWarn : ''}">
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
                <div class={styles.sectionHeader} style="margin-top: 1.5rem;">
                    <h3>Open Positions</h3>
                </div>
                <div class={styles.instancesTableWrapper}>
                    <table class={styles.instancesTable}>
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
                                    <td class={styles.colPair}>{pos.symbol}</td>
                                    <td>
                                        <span class="{styles.statusBadge} {pos.direction === 'LONG' ? styles.statusRunning : ''} {pos.direction === 'SHORT' ? styles.statusStopped : ''}">
                                            {pos.direction}
                                        </span>
                                    </td>
                                    <td class={styles.colMono}>${pos.entryPrice.toFixed(2)}</td>
                                    <td class={styles.colMono}>{pos.size.toFixed(4)}</td>
                                    <td class="{styles.colMono} {pos.unrealizedPnl >= 0 ? styles.positive : ''} {pos.unrealizedPnl < 0 ? styles.negative : ''}">
                                        {pos.unrealizedPnl >= 0 ? '+' : ''}${pos.unrealizedPnl.toFixed(2)}
                                    </td>
                                    <td class="{styles.colMono} {pos.unrealizedRoi >= 0 ? styles.positive : ''} {pos.unrealizedRoi < 0 ? styles.negative : ''}">
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
                <div class={styles.sectionHeader} style="margin-top: 1.5rem;">
                    <h3>Equity Curve</h3>
                </div>
                <div class={styles.equityChartWrapper}>
                    <div class={styles.equityChartContainer} bind:this={equityContainer}></div>
                </div>
            {/if}

            <!-- Compounded Balance Curve -->
            {#if stats?.compounded_curve && stats.compounded_curve.length >= 2}
                <div class={styles.sectionHeader} style="margin-top: 1.5rem;">
                    <h3>Compounded Balance Curve</h3>
                </div>
                <div class={styles.equityChartWrapper}>
                    <div class={styles.equityChartContainer} bind:this={compoundedContainer}></div>
                </div>
            {/if}
        {/if}

        <!-- Detailed Stats -->
        {#if stats?.core_stats}
            <div class={styles.detailGrid}>
                <div class={styles.detailCard}>
                    <h4>Trade Outcomes</h4>
                    <div class={styles.detailRow}>
                        <span>Wins</span>
                        <span class={styles.positive}>{stats.core_stats.wins}</span>
                    </div>
                    <div class={styles.detailRow}>
                        <span>Losses</span>
                        <span class={styles.negative}>{stats.core_stats.losses}</span>
                    </div>
                    <div class={styles.detailRow}>
                        <span>Avg Win</span>
                        <span>${stats.core_stats.avg_gain.toFixed(2)}</span>
                    </div>
                    <div class={styles.detailRow}>
                        <span>Avg Loss</span>
                        <span>${Math.abs(stats.core_stats.avg_loss).toFixed(2)}</span>
                    </div>
                    <div class={styles.detailRow}>
                        <span>Largest Win</span>
                        <span class={styles.positive}>${stats.core_stats.largest_gain.toFixed(2)}</span>
                    </div>
                    <div class={styles.detailRow}>
                        <span>Largest Loss</span>
                        <span class={styles.negative}>${Math.abs(stats.core_stats.largest_loss).toFixed(2)}</span>
                    </div>
                </div>

                <div class={styles.detailCard}>
                    <h4>Direction Breakdown</h4>
                    {#if stats.direction_breakdown}
                        <div class={styles.detailRow}>
                            <span>Longs</span>
                            <span>{stats.direction_breakdown.longs}</span>
                        </div>
                        <div class={styles.detailRow}>
                            <span>Shorts</span>
                            <span>{stats.direction_breakdown.shorts}</span>
                        </div>
                        <div class={styles.detailRow}>
                            <span>Long Expectancy</span>
                            <span>${stats.direction_breakdown.long_expectancy.toFixed(2)}</span>
                        </div>
                        <div class={styles.detailRow}>
                            <span>Short Expectancy</span>
                            <span>${stats.direction_breakdown.short_expectancy.toFixed(2)}</span>
                        </div>
                        <div class={styles.dirSubTable}>
                            <table class={styles.dirTable}>
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
                                        <td class={styles.positive}>LONG</td>
                                        <td class={styles.positive}>{stats.direction_breakdown.long_wins}</td>
                                        <td class={styles.negative}>{stats.direction_breakdown.long_losses}</td>
                                        <td>{stats.direction_breakdown.long_win_rate.toFixed(1)}%</td>
                                        <td class={styles.positive}>{stats.direction_breakdown.long_avg_gain.toFixed(2)}%</td>
                                        <td class={styles.negative}>-{stats.direction_breakdown.long_avg_loss.toFixed(2)}%</td>
                                    </tr>
                                    <tr>
                                        <td class={styles.negative}>SHORT</td>
                                        <td class={styles.positive}>{stats.direction_breakdown.short_wins}</td>
                                        <td class={styles.negative}>{stats.direction_breakdown.short_losses}</td>
                                        <td>{stats.direction_breakdown.short_win_rate.toFixed(1)}%</td>
                                        <td class={styles.positive}>{stats.direction_breakdown.short_avg_gain.toFixed(2)}%</td>
                                        <td class={styles.negative}>-{stats.direction_breakdown.short_avg_loss.toFixed(2)}%</td>
                                    </tr>
                                </tbody>
                            </table>
                        </div>
                    {/if}
                </div>

                <div class={styles.detailCard}>
                    <h4>Streaks</h4>
                    {#if stats.winning_streaks}
                        <div class={styles.detailRow}>
                            <span>Max Wins</span>
                            <span class={styles.positive}>{stats.winning_streaks.max_streak_length}</span>
                        </div>
                        <div class={styles.detailRow}>
                            <span>Max Losses</span>
                            <span class={styles.negative}>{stats.losing_streaks.max_streak_length}</span>
                        </div>
                        <div class={styles.detailRow}>
                            <span>Post-Loss Recovery</span>
                            <span>{(stats.post_loss_recovery_pct * 100).toFixed(0)}%</span>
                        </div>
                    {/if}
                </div>
            </div>
        {/if}

        <!-- Historical Analyst Recommendations -->
        {#if recommendations.length > 0}
            <div class={styles.sectionHeader}>
                <h3>📊 Historical Analyst Recommendations</h3>
            </div>
            <div class={styles.recommendationsList}>
                {#each recommendations.slice(0, 5) as rec}
                    <div class={styles.recCard}>
                        <div class={styles.recHeader}>
                            <span class={styles.recSymbol}>{rec.symbol || rec.pair_key}</span>
                            <span class={styles.recDate}>{rec.generated_at?.substring(0, 10)}</span>
                            <span class={styles.recStats}>
                                WR: {(rec.win_rate * 100).toFixed(0)}% |
                                PF: {rec.profit_factor?.toFixed(2)} |
                                R:R: {rec.avg_risk_reward?.toFixed(2)}
                            </span>
                        </div>
                        {#if rec.key_improvements}
                            <p class={styles.recText}><strong>Improvements:</strong> {rec.key_improvements}</p>
                        {/if}
                        {#if rec.risk_recommendation}
                            <p class={styles.recText}><strong>Risk:</strong> {rec.risk_recommendation}</p>
                        {/if}
                        {#if rec.regime_analysis}
                            <p class={styles.recText}><strong>Regimes:</strong> {rec.regime_analysis}</p>
                        {/if}
                    </div>
                {/each}
            </div>
        {:else}
            <div class={styles.sectionHeader}>
                <p class={styles.noData}>No historical analyst recommendations yet. They appear after enough trades accumulate.</p>
            </div>
        {/if}
    {/if}
</div>


