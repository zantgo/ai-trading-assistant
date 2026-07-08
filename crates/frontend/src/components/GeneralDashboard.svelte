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

    let totalRealizedPnl = $state(0);
    let totalUnrealizedPnl = $state(0);
    let totalPortfolioValue = $state(app.sessionCapital);
    let selectedTimeframe = $state<'1H' | '1D' | '1W' | '1M' | '1Y' | 'ALL'>('ALL');
    let lastTimeframe = $state('');

    let showModal = $state(false);
    let modalChartContainer = $state<HTMLDivElement | null>(null);
    let modalChart: IChartApi | null = null;
    let modalSeries: ISeriesApi<'Line'> | null = null;

    let utcTime = $state('');
    let clockInterval: ReturnType<typeof setInterval>;

    function updateUtcClock() {
        const now = new Date();
        utcTime = now.toISOString().replace('T', ' ').slice(0, 19) + ' UTC';
    }

    let compoundedContainer = $state<HTMLDivElement | null>(null);
    let compoundedChart: IChartApi | null = null;
    let compoundedSeries: ISeriesApi<'Line'> | null = null;

    let ro: ResizeObserver;
    let pollInterval: ReturnType<typeof setInterval>;

    async function fetchAll() {
        try {
            const [statsRes, recsRes, statusRes, instancesRes, paperPerfRes] = await Promise.all([
                fetch(`/api/dashboard/stats?initial_capital=${app.sessionCapital}`),
                fetch('/api/historical-recommendations'),
                fetch('/api/system/status'),
                fetch('/api/instances'),
                fetch('/api/paper/performance'),
            ]);

            if (statsRes.ok) stats = await statsRes.json();
            if (recsRes.ok) {
                const data = await recsRes.json();
                recommendations = data.recommendations || [];
            }
            if (statusRes.ok) heartbeat = await statusRes.json();

            let paperPnl = 0;
            if (paperPerfRes.ok) {
                const data = await paperPerfRes.json();
                paperPnl = data.total_pnl ?? 0;
            }
            totalRealizedPnl = paperPnl;

            if (instancesRes.ok) {
                const data = await instancesRes.json();
                instances = data.instances || [];

                const positions: PaperPosition[] = [];
                const paperResults = await Promise.allSettled(
                    instances.map((inst: InstanceSummary) =>
                        fetch(`/api/paper/status?symbol=${encodeURIComponent(inst.symbol)}`)
                            .then(r => r.ok ? r.json() : null)
                    )
                );

                let totalUnrealized = 0;
                for (let i = 0; i < instances.length; i++) {
                    const result = paperResults[i];
                    if (result.status === 'fulfilled' && result.value) {
                        const val = result.value;
                        totalUnrealized += val.unrealized_pnl ?? 0;
                        if (val.active_position) {
                            const pos = val.active_position;
                            positions.push({
                                symbol: instances[i].symbol,
                                direction: pos.direction || '',
                                entryPrice: pos.entry_price ?? 0,
                                size: pos.size ?? 0,
                                unrealizedPnl: val.unrealized_pnl ?? 0,
                                unrealizedRoi: val.unrealized_roi_pct ?? 0,
                            });
                        }
                    }
                }
                paperPositions = positions;
                totalUnrealizedPnl = totalUnrealized;
                totalPortfolioValue = app.sessionCapital + paperPnl + totalUnrealized;
            }
        } catch (e) {
            console.error('Error polling dashboard metrics:', e);
        } finally {
            loading = false;
        }
    }

    function filterCurveData(curve: [number, number][]) {
        if (!curve || curve.length === 0) {
            const nowMs = Date.now();
            let startMs = nowMs;
            if (selectedTimeframe === '1H') startMs = nowMs - 3600 * 1000;
            else if (selectedTimeframe === '1D') startMs = nowMs - 86400 * 1000;
            else if (selectedTimeframe === '1W') startMs = nowMs - 604800 * 1000;
            else if (selectedTimeframe === '1M') startMs = nowMs - 2592000 * 1000;
            else if (selectedTimeframe === '1Y') startMs = nowMs - 31536000 * 1000;
            else startMs = nowMs - 86400 * 1000;
            return [{ time: Math.floor(startMs / 1000) as Time, value: app.sessionCapital }];
        }

        const nowMs = Date.now();
        let cutoffMs = 0;

        if (selectedTimeframe === '1H') {
            cutoffMs = nowMs - 60 * 60 * 1000;
        } else if (selectedTimeframe === '1D') {
            cutoffMs = nowMs - 24 * 60 * 60 * 1000;
        } else if (selectedTimeframe === '1W') {
            cutoffMs = nowMs - 7 * 24 * 60 * 60 * 1000;
        } else if (selectedTimeframe === '1M') {
            cutoffMs = nowMs - 30 * 24 * 60 * 60 * 1000;
        } else if (selectedTimeframe === '1Y') {
            cutoffMs = nowMs - 365 * 24 * 60 * 60 * 1000;
        }

        const normalizedCurve = curve.map(([ts, val]) => {
            const normalizedTs = ts > 9_000_000_000 ? ts : ts * 1000;
            return [normalizedTs, val] as [number, number];
        });

        let filtered = selectedTimeframe === 'ALL'
            ? normalizedCurve
            : normalizedCurve.filter(([ts, _]) => ts >= cutoffMs);

        const beforeCutoff = normalizedCurve.filter(([ts, _]) => ts < cutoffMs);

        if (selectedTimeframe !== 'ALL') {
            if (beforeCutoff.length > 0) {
                const lastBefore = beforeCutoff[beforeCutoff.length - 1];
                filtered = [[cutoffMs, lastBefore[1]], ...filtered];
            } else if (filtered.length === 0) {
                filtered = [[cutoffMs, app.sessionCapital], [nowMs, app.sessionCapital]];
            } else {
                const first = filtered[0];
                filtered = [[cutoffMs, first[1]], ...filtered];
            }
        } else if (filtered.length > 0) {
            const first = filtered[0];
            filtered = [[first[0] - 60000, app.sessionCapital], ...filtered];
        }

        return filtered.map(([ts, val]) => ({
            time: Math.floor(ts / 1000) as Time,
            value: val,
        }));
    }

    function buildCompoundedChart() {
        if (!compoundedContainer) return;

        const curve = stats?.compounded_curve || [];
        const filteredData = filterCurveData(curve);
        const nowSec = Math.floor(Date.now() / 1000) as Time;
        filteredData.push({ time: nowSec, value: totalPortfolioValue });

        const timeframeChanged = lastTimeframe !== selectedTimeframe;
        lastTimeframe = selectedTimeframe;

        if (!compoundedChart) {
            compoundedChart = createChart(compoundedContainer, {
                autoSize: true,
                layout: { background: { color: 'transparent' }, textColor: '#94a3b8', fontSize: 10 },
                grid: { vertLines: { color: '#1c212e' }, horzLines: { color: '#1c212e' } },
                rightPriceScale: { borderColor: '#2d3448', scaleMargins: { top: 0.15, bottom: 0.15 } },
                timeScale: { borderColor: '#2d3448', visible: true, timeVisible: true, secondsVisible: false },
                handleScale: true,
                handleScroll: true,
            });

            compoundedSeries = compoundedChart.addSeries(LineSeries, {
                color: '#e2e8f0',
                lineWidth: 3,
                priceLineVisible: false,
                crosshairMarkerVisible: true,
            });

            compoundedSeries.setData(filteredData);
            compoundedChart.timeScale().fitContent();

            compoundedChart.subscribeDblClick(() => {
                showModal = true;
            });
        } else {
            if (compoundedSeries) {
                compoundedSeries.setData(filteredData);
            }
            if (timeframeChanged) {
                compoundedChart.timeScale().fitContent();
            }
        }
    }

    onMount(() => {
        fetchAll();
        pollInterval = setInterval(fetchAll, 5000);
        updateUtcClock();
        clockInterval = setInterval(updateUtcClock, 1000);

        ro = new ResizeObserver(() => {
            if (compoundedContainer && compoundedChart) {
                const w = compoundedContainer.clientWidth;
                const h = compoundedContainer.clientHeight;
                if (w > 0 && h > 0) compoundedChart.resize(w, h);
            }
            if (modalChartContainer && modalChart) {
                const w = modalChartContainer.clientWidth;
                const h = modalChartContainer.clientHeight;
                if (w > 0 && h > 0) modalChart.resize(w, h);
            }
        });

        if (compoundedContainer?.parentElement) {
            ro.observe(compoundedContainer.parentElement);
        }
    });

    onDestroy(() => {
        clearInterval(pollInterval);
        clearInterval(clockInterval);
        ro?.disconnect();
        if (compoundedChart) {
            compoundedChart.remove();
            compoundedChart = null;
            compoundedSeries = null;
        }
        if (modalChart) {
            modalChart.remove();
            modalChart = null;
            modalSeries = null;
        }
    });

    $effect(() => {
        if (!loading && stats?.compounded_curve) {
            requestAnimationFrame(() => buildCompoundedChart());
        }
    });

    $effect(() => {
        if (showModal && modalChartContainer && !modalChart) {
            requestAnimationFrame(() => {
                if (!modalChartContainer || modalChart) return;
                modalChart = createChart(modalChartContainer, {
                    autoSize: true,
                    layout: { background: { color: 'transparent' }, textColor: '#94a3b8', fontSize: 11 },
                    grid: { vertLines: { color: '#1c212e' }, horzLines: { color: '#1c212e' } },
                    rightPriceScale: { borderColor: '#2d3448', scaleMargins: { top: 0.15, bottom: 0.15 } },
                    timeScale: { borderColor: '#2d3448', visible: true, timeVisible: true, secondsVisible: false },
                    handleScale: true,
                    handleScroll: true,
                });

                modalSeries = modalChart.addSeries(LineSeries, {
                    color: '#e2e8f0',
                    lineWidth: 3,
                    priceLineVisible: false,
                    crosshairMarkerVisible: true,
                });

                const curve = stats?.compounded_curve || [];
                const filteredData = filterCurveData(curve);
                const nowSec = Math.floor(Date.now() / 1000) as Time;
                filteredData.push({ time: nowSec, value: totalPortfolioValue });

                modalSeries.setData(filteredData);
                modalChart.timeScale().fitContent();
            });
        }

        return () => {
            if (modalChart) {
                modalChart.remove();
                modalChart = null;
                modalSeries = null;
            }
        };
    });

    $effect(() => {
        if (showModal && modalSeries && stats?.compounded_curve) {
            const curve = stats.compounded_curve;
            const filteredData = filterCurveData(curve);
            const nowSec = Math.floor(Date.now() / 1000) as Time;
            filteredData.push({ time: nowSec, value: totalPortfolioValue });
            modalSeries.setData(filteredData);
        }
    });

    function handleTimeframeChange(tf: '1H' | '1D' | '1W' | '1M' | '1Y' | 'ALL') {
        selectedTimeframe = tf;
        buildCompoundedChart();
    }
</script>

<div class={styles.dashboardView}>
    <div class={styles.headerRow}>
        <h2 class={styles.dashboardTitle}>Dashboard</h2>
        <div class={styles.utcClock}>{utcTime}</div>
    </div>

    {#if loading}
        <div class={styles.loadingRow}>Loading dashboard metrics...</div>
    {:else}
        <div class={styles.portfolioHeaderPanel}>
            <div class={styles.portfolioSummaryDetails}>
                <div class={styles.hudHeader}>
                    <span class={styles.hudLabel}>Total Account Value ({app.quote})</span>
                </div>
                <div class={styles.hudValue}>
                    ${totalPortfolioValue.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
                </div>
                <div class={styles.hudSubRow}>
                    <div class={styles.hudSubItem}>
                        <span class={styles.hudSubLabel}>Realized P&L</span>
                        <span class="{styles.hudSubValue} {totalRealizedPnl >= 0 ? styles.positive : styles.negative}">
                            {totalRealizedPnl >= 0 ? '+' : ''}${totalRealizedPnl.toFixed(2)}
                            ({totalRealizedPnl >= 0 ? '+' : ''}{(totalRealizedPnl / app.sessionCapital * 100).toFixed(2)}%)
                        </span>
                    </div>
                    <div class={styles.hudSubItem}>
                        <span class={styles.hudSubLabel}>Unrealized P&L</span>
                        <span class="{styles.hudSubValue} {totalUnrealizedPnl >= 0 ? styles.positive : styles.negative}">
                            {totalUnrealizedPnl >= 0 ? '+' : ''}${totalUnrealizedPnl.toFixed(2)}
                            ({totalUnrealizedPnl >= 0 ? '+' : ''}{(totalUnrealizedPnl / app.sessionCapital * 100).toFixed(2)}%)
                        </span>
                    </div>
                </div>
            </div>

            <div class={styles.portfolioChartContainer}>
                <div class={styles.chartControlBar}>
                    <span class={styles.chartTitle}>Portfolio Performance Curve</span>
                    <div class={styles.timeframeTabs}>
                        <button class="{styles.timeframeBtn} {selectedTimeframe === '1H' ? styles.active : ''}" onclick={() => handleTimeframeChange('1H')}>1H</button>
                        <button class="{styles.timeframeBtn} {selectedTimeframe === '1D' ? styles.active : ''}" onclick={() => handleTimeframeChange('1D')}>1D</button>
                        <button class="{styles.timeframeBtn} {selectedTimeframe === '1W' ? styles.active : ''}" onclick={() => handleTimeframeChange('1W')}>1W</button>
                        <button class="{styles.timeframeBtn} {selectedTimeframe === '1M' ? styles.active : ''}" onclick={() => handleTimeframeChange('1M')}>1M</button>
                        <button class="{styles.timeframeBtn} {selectedTimeframe === '1Y' ? styles.active : ''}" onclick={() => handleTimeframeChange('1Y')}>1Y</button>
                        <button class="{styles.timeframeBtn} {selectedTimeframe === 'ALL' ? styles.active : ''}" onclick={() => handleTimeframeChange('ALL')}>ALL</button>
                    </div>
                </div>
                <div class={styles.equityChartContainer} bind:this={compoundedContainer}></div>
            </div>
        </div>

        <!-- Overview Cards -->
        <div class={styles.statsGrid}>
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
                <div class="{styles.sectionHeader} {styles.sectionHeaderSpacer}">
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
                <h3>Historical Analyst Recommendations</h3>
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

{#if showModal}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div class={styles.modalOverlay} onclick={() => showModal = false} role="presentation">
        <div class={styles.modalContent} onclick={e => e.stopPropagation()} role="dialog" tabindex="-1">
            <div class={styles.modalHeader}>
                <span class={styles.modalTitle}>Portfolio Performance Curve — Expanded View</span>
                <button class={styles.modalCloseBtn} onclick={() => showModal = false}>✕</button>
            </div>
            <div class={styles.modalChartContainer} bind:this={modalChartContainer}></div>
        </div>
    </div>
{/if}
