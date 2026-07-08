<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import type { CoreStats, StreakMetrics, CalendarDay, StyleSegment, PairStat } from '../types';
    import styles from './AnalyticsDashboard.module.css';

    const app = useAppStore();

    $effect(() => {
        const _period = app.dashboardPeriod;
        const _origin = app.dashboardOrigin;
        const _filter = app.dashboardActiveFilter;
        app.fetchDashboardStats();
        app.fetchTradeLedger();
    });

    function formatUsd(v: number): string {
        if (Math.abs(v) >= 1000) return '$' + v.toLocaleString('en-US', { minimumFractionDigits: 0, maximumFractionDigits: 0 });
        if (Math.abs(v) >= 1) return '$' + v.toFixed(2);
        return '$' + v.toFixed(4);
    }
    function formatPct(v: number): string { return v.toFixed(2) + '%'; }
    function formatCount(v: number): string { return v.toString(); }

    const FILTERS = [
        { key: 'summary', label: 'Summary' },
        { key: 'performance', label: 'Performance' },
        { key: 'behavior', label: 'Behavior' },
        { key: 'streaks', label: 'Streaks' },
        { key: 'pairs', label: 'Pairs' },
        { key: 'commissions', label: 'Commissions' },
    ];

    let currentFilter = $state('');

    $effect(() => {
        currentFilter = app.dashboardActiveFilter;
    });
</script>

<div class={styles.adbLayout}>
    <!-- Filter Ribbon -->
    <div class={styles.adbRibbon}>
        <div class={styles.adbFiltersLeft}>
            <select class={styles.adbSelect} bind:value={app.dashboardPeriod}>
                <option>All</option>
                <option>7d</option>
                <option>30d</option>
                <option>90d</option>
            </select>
            <select class={styles.adbSelect} bind:value={app.dashboardOrigin}>
                <option>All</option>
                <option>MANUAL</option>
                <option>AUTOMATED</option>
            </select>
        </div>
        <div class={styles.adbFiltersCenter}>
            <button class="{styles.adbFilterBtn} {currentFilter === '' ? styles.active : ''}" onclick={() => { currentFilter = ''; app.dashboardActiveFilter = ''; }}>All</button>
            {#each FILTERS as f (f.key)}
                <button class="{styles.adbFilterBtn} {currentFilter === f.key ? styles.active : ''}"
                    onclick={() => { currentFilter = f.key; app.dashboardActiveFilter = f.key; }}>{f.label}</button>
            {/each}
        </div>
    </div>

    {#if !app.dashboardStats}
        <div class={styles.adbEmpty}>No trade data available. Execute trades to populate the dashboard.</div>
    {:else}
        {@const stats = app.dashboardStats}

        <!-- Resumen / Rendimiento / Comportamiento -->
        {#if currentFilter === '' || currentFilter === 'summary' || currentFilter === 'performance' || currentFilter === 'behavior'}
            <div class={styles.adbSectionTitle}>YOUR STATISTICS</div>
            <div class={styles.adbStatsGrid}>
                <div class={styles.adbStatCard}>
                    <span class={styles.adbStatLabel}>Total PnL</span>
                    <span class="{styles.adbStatValue} {stats.core_stats.total_pnl >= 0 ? styles.adbPos : ''} {stats.core_stats.total_pnl < 0 ? styles.adbNeg : ''}">
                        {formatUsd(stats.core_stats.total_pnl)}
                    </span>
                </div>
                <div class={styles.adbStatCard}>
                    <span class={styles.adbStatLabel}>Win Rate</span>
                    <span class={styles.adbStatValue}>{formatPct(stats.core_stats.win_rate * 100)}</span>
                </div>
                <div class={styles.adbStatCard}>
                    <span class={styles.adbStatLabel}>Average Loss</span>
                    <span class="{styles.adbStatValue} {styles.adbNeg}">{formatUsd(stats.core_stats.avg_loss)}</span>
                </div>
                <div class={styles.adbStatCard}>
                    <span class={styles.adbStatLabel}>Average Gain</span>
                    <span class="{styles.adbStatValue} {styles.adbPos}">{formatUsd(stats.core_stats.avg_gain)}</span>
                </div>
                <div class={styles.adbStatCard}>
                    <span class={styles.adbStatLabel}>Expectancy</span>
                    <span class={styles.adbStatValue}>{formatUsd(stats.core_stats.expectancy)}</span>
                </div>
                <div class={styles.adbStatCard}>
                    <span class={styles.adbStatLabel}>Average R:R</span>
                    <span class={styles.adbStatValue}>1:{stats.core_stats.avg_risk_reward_ratio.toFixed(2)}</span>
                </div>
                <div class={styles.adbStatCard}>
                    <span class={styles.adbStatLabel}>Largest Loss</span>
                    <span class="{styles.adbStatValue} {styles.adbNeg}">{formatUsd(stats.core_stats.largest_loss)}</span>
                </div>
                <div class={styles.adbStatCard}>
                    <span class={styles.adbStatLabel}>Largest Gain</span>
                    <span class="{styles.adbStatValue} {styles.adbPos}">{formatUsd(stats.core_stats.largest_gain)}</span>
                </div>
                <div class={styles.adbStatCard}>
                    <span class={styles.adbStatLabel}>Sharpe Ratio</span>
                    <span class={styles.adbStatValue}>{((stats as any).sharpe_ratio ?? 0).toFixed(2)}</span>
                </div>
                <div class={styles.adbStatCard}>
                    <span class={styles.adbStatLabel}>Recovery Factor</span>
                    <span class={styles.adbStatValue}>{((stats as any).recovery_factor ?? 0).toFixed(2)}</span>
                </div>
            </div>

            <!-- Equity Curve -->
            <div class={styles.adbSectionTitle}>CUMULATIVE GAIN</div>
            <div class={styles.adbChartBox}>
                <div class={styles.adbMiniChart}>
                    {#each stats.equity_curve.slice(-50) as [ts, val], i}
                        <div class={styles.adbBarLine} style="left: {(i / Math.max(stats.equity_curve.length - 1, 1)) * 100}%; bottom: 0; height: {val === 0 ? 0 : Math.min(Math.abs(val) / Math.max(Math.abs(stats.core_stats.total_pnl), 1) * 100, 100)}%; background: {val >= 0 ? '#10b981' : '#ef4444'}">
                        </div>
                    {/each}
                </div>
            </div>

            <!-- Compounded Balance Curve -->
            {#if stats.compounded_curve.length > 0}
                {@const compValues = stats.compounded_curve.map(c => c[1])}
                {@const compMin = Math.min(...compValues)}
                {@const compMax = Math.max(...compValues)}
                {@const compRange = Math.max(compMax - compMin, 0.01)}
                <div class={styles.adbSectionTitle}>COMPOUNDED BALANCE</div>
                <div class={styles.adbChartBox}>
                    <div class={styles.adbMiniChart}>
                        {#each stats.compounded_curve.slice(-50) as [ts, val], i}
                            <div class={styles.adbBarLine} style="left: {(i / Math.max(stats.compounded_curve.length - 1, 1)) * 100}%;
                                bottom: {((val - compMin) / compRange) * 100}%;
                                height: 2px;
                                background: {val >= 10000 ? '#10b981' : '#ef4444'};">
                            </div>
                        {/each}
                    </div>
                </div>
            {/if}

            <!-- Daily Activity -->
            <div class={styles.adbSectionTitle}>ACTIVITY</div>
            <div class={styles.adbChartBox}>
                <div class={styles.adbMiniChart}>
                    {#each stats.daily_activity.slice(-30) as day, i}
                        <div class={styles.adbStackedBar} style="left: {(i / Math.max(stats.daily_activity.length - 1, 1)) * 100}%">
                            <div class={styles.adbStackLong} style="height: {day.longs / Math.max(day.longs + day.shorts, 1) * 100}%; background: #10b981;"></div>
                            <div class={styles.adbStackShort} style="height: {day.shorts / Math.max(day.longs + day.shorts, 1) * 100}%; background: #ef4444;"></div>
                        </div>
                    {/each}
                </div>
            </div>

            <!-- Daily PnL -->
            <div class={styles.adbSectionTitle}>PNL PER DAY</div>
            <div class={styles.adbChartBox}>
                <div class={styles.adbMiniChart}>
                    {#each stats.daily_pnl.slice(-30) as day, i}
                        {@const maxVal = Math.max(...stats.daily_pnl.map(d => Math.abs(d.pnl)), 1)}
                        <div class={styles.adbBarLine} style="left: {(i / Math.max(stats.daily_pnl.length - 1, 1)) * 100}%;
                            height: {Math.abs(day.pnl) / maxVal * 100}%;
                            background: {day.pnl >= 0 ? '#10b981' : '#ef4444'}; bottom: 0;">
                        </div>
                    {/each}
                </div>
            </div>

            <!-- Win Rate by Hour -->
            <div class={styles.adbSectionTitle}>WIN RATE BY HOUR</div>
            <div class={styles.adbChartBox}>
                <div class={styles.adbMiniChart}>
                    {#each stats.win_rate_by_hour as h, i}
                        <div class={styles.adbBarLine} style="left: {(i / 23) * 100}%; bottom: 0; height: {h.win_rate * 100}%; background: #94a3b8; width: 3px;">
                        </div>
                    {/each}
                </div>
                <div class={styles.adbAxisLabels}>
                    <span>00</span><span>06</span><span>12</span><span>18</span><span>23</span>
                </div>
            </div>

            <!-- Win Rate by Weekday -->
            <div class={styles.adbSectionTitle}>WIN RATE BY DAY</div>
            <div class={styles.adbChartBox}>
                <div class={styles.adbMiniChart}>
                    {#each stats.win_rate_by_weekday as day, i}
                        <div class={styles.adbBarLine} style="left: {(i / 6) * 100}%; bottom: 0; height: {day.win_rate * 100}%; background: #94a3b8; width: 12px;">
                        </div>
                    {/each}
                </div>
                <div class={styles.adbAxisLabels}>
                    <span>Sun</span><span>Mon</span><span>Tue</span><span>Wed</span><span>Thu</span><span>Fri</span><span>Sat</span>
                </div>
            </div>

            <!-- Direction Breakdown -->
            <div class={styles.adbSectionTitle}>TRADE DIRECTION</div>
            <div class={styles.adbTwoCol}>
                <div class={styles.adbDonutContainer}>
                    <svg viewBox="0 0 100 100" class={styles.adbDonut}>
                        <circle cx="50" cy="50" r="35" fill="none" stroke="#1e293b" stroke-width="12" />
                        <circle cx="50" cy="50" r="35" fill="none" stroke="#10b981" stroke-width="12"
                            stroke-dasharray="{(stats.direction_breakdown.longs / (stats.direction_breakdown.longs + stats.direction_breakdown.shorts || 1)) * 220} 220"
                            stroke-dashoffset="0" transform="rotate(-90 50 50)" />
                        <circle cx="50" cy="50" r="35" fill="none" stroke="#ef4444" stroke-width="12"
                            stroke-dasharray="{(stats.direction_breakdown.shorts / (stats.direction_breakdown.longs + stats.direction_breakdown.shorts || 1)) * 220} 220"
                            stroke-dashoffset="-{(stats.direction_breakdown.longs / (stats.direction_breakdown.longs + stats.direction_breakdown.shorts || 1)) * 220}"
                            transform="rotate(-90 50 50)" />
                    </svg>
                    <div class={styles.adbDonutLegend}>
                        <span class={styles.adbLegendLong}>Long: {stats.direction_breakdown.longs}</span>
                        <span class={styles.adbLegendShort}>Short: {stats.direction_breakdown.shorts}</span>
                    </div>
                </div>
                <div class={styles.adbExpectancyBox}>
                    <span class={styles.adbExpLabel}>Long Expectancy</span>
                    <span class={styles.adbExpVal}>{formatUsd(stats.direction_breakdown.long_expectancy)}</span>
                    <span class={styles.adbExpLabel}>Short Expectancy</span>
                    <span class={styles.adbExpVal}>{formatUsd(stats.direction_breakdown.short_expectancy)}</span>
                </div>
            </div>

            <!-- Directional Performance Table -->
            <div class={styles.adbSectionTitle}>DIRECTIONAL PERFORMANCE (LONG vs SHORT)</div>
            <div class={styles.adbTableWrap}>
                <table class={styles.adbTable}>
                    <thead>
                        <tr>
                            <th>Direction</th>
                            <th>Wins (Profit)</th>
                            <th>Losses (Loss)</th>
                            <th>Total Trades</th>
                            <th>Win Rate %</th>
                            <th>Avg Gain %</th>
                            <th>Avg Loss %</th>
                        </tr>
                    </thead>
                    <tbody>
                        <tr>
                            <td class={styles.adbDirLong}>LONG</td>
                            <td>{stats.direction_breakdown.long_wins}</td>
                            <td>{stats.direction_breakdown.long_losses}</td>
                            <td>{stats.direction_breakdown.longs}</td>
                            <td>{stats.direction_breakdown.long_win_rate.toFixed(2)}%</td>
                            <td class={styles.adbPos}>{stats.direction_breakdown.long_avg_gain.toFixed(2)}%</td>
                            <td class={styles.adbNeg}>-{stats.direction_breakdown.long_avg_loss.toFixed(2)}%</td>
                        </tr>
                        <tr>
                            <td class={styles.adbDirShort}>SHORT</td>
                            <td>{stats.direction_breakdown.short_wins}</td>
                            <td>{stats.direction_breakdown.short_losses}</td>
                            <td>{stats.direction_breakdown.shorts}</td>
                            <td>{stats.direction_breakdown.short_win_rate.toFixed(2)}%</td>
                            <td class={styles.adbPos}>{stats.direction_breakdown.short_avg_gain.toFixed(2)}%</td>
                            <td class={styles.adbNeg}>-{stats.direction_breakdown.short_avg_loss.toFixed(2)}%</td>
                        </tr>
                    </tbody>
                </table>
            </div>

            <!-- Trader Style -->
            <div class={styles.adbSectionTitle}>TRADER PROFILE</div>
            <div class={styles.adbStyleGrid}>
                <div class={styles.adbStyleCard}>
                    <span class={styles.adbStyleName}>Scalper</span>
                    <span class={styles.adbStyleCount}>{stats.trader_style.scalper.count} trades</span>
                    <span class={styles.adbStyleDur}>{(stats.trader_style.scalper.avg_duration_minutes / 60).toFixed(1)}h avg</span>
                    <span class={styles.adbStyleWr}>{formatPct(stats.trader_style.scalper.win_rate * 100)} WR</span>
                </div>
                <div class={styles.adbStyleCard}>
                    <span class={styles.adbStyleName}>Day Trader</span>
                    <span class={styles.adbStyleCount}>{stats.trader_style.day_trader.count} trades</span>
                    <span class={styles.adbStyleDur}>{(stats.trader_style.day_trader.avg_duration_minutes / 60).toFixed(1)}h avg</span>
                    <span class={styles.adbStyleWr}>{formatPct(stats.trader_style.day_trader.win_rate * 100)} WR</span>
                </div>
                <div class={styles.adbStyleCard}>
                    <span class={styles.adbStyleName}>Swing</span>
                    <span class={styles.adbStyleCount}>{stats.trader_style.swing_trader.count} trades</span>
                    <span class={styles.adbStyleDur}>{(stats.trader_style.swing_trader.avg_duration_minutes / 60).toFixed(1)}h avg</span>
                    <span class={styles.adbStyleWr}>{formatPct(stats.trader_style.swing_trader.win_rate * 100)} WR</span>
                </div>
            </div>
        {/if}

        <!-- Streaks -->
        {#if currentFilter === '' || currentFilter === 'streaks'}
            <div class={styles.adbSectionTitle}>WINNING STREAKS</div>
            <div class={styles.adbStatsGrid}>
                <div class={styles.adbStatCard}>
                    <span class={styles.adbStatLabel}>Average Streak</span>
                    <span class={styles.adbStatValue}>{stats.winning_streaks.avg_streak_length.toFixed(1)}</span>
                </div>
                <div class={styles.adbStatCard}>
                    <span class={styles.adbStatLabel}>Max Streak</span>
                    <span class={styles.adbStatValue}>{stats.winning_streaks.max_streak_length}</span>
                </div>
                <div class={styles.adbStatCard}>
                    <span class={styles.adbStatLabel}>Max Value</span>
                    <span class="{styles.adbStatValue} {styles.adbPos}">{formatUsd(stats.winning_streaks.max_consecutive_value)}</span>
                </div>
            </div>

            <div class={styles.adbSectionTitle}>LOSING STREAKS</div>
            <div class={styles.adbStatsGrid}>
                <div class={styles.adbStatCard}>
                    <span class={styles.adbStatLabel}>Average Streak</span>
                    <span class={styles.adbStatValue}>{stats.losing_streaks.avg_streak_length.toFixed(1)}</span>
                </div>
                <div class={styles.adbStatCard}>
                    <span class={styles.adbStatLabel}>Max Streak</span>
                    <span class={styles.adbStatValue}>{stats.losing_streaks.max_streak_length}</span>
                </div>
                <div class={styles.adbStatCard}>
                    <span class={styles.adbStatLabel}>Max Value</span>
                    <span class="{styles.adbStatValue} {styles.adbNeg}">{formatUsd(stats.losing_streaks.max_consecutive_value)}</span>
                </div>
            </div>

            <div class={styles.adbSectionTitle}>POST-LOSS RECOVERY</div>
            <div class={styles.adbStatCard}>
                <span class={styles.adbStatValue}>{formatPct(stats.post_loss_recovery_pct)}</span>
                <span class={styles.adbStatLabel}>winning trades after loss</span>
            </div>
        {/if}

        <!-- Pairs -->
        {#if currentFilter === '' || currentFilter === 'pairs'}
            <div class={styles.adbSectionTitle}>MOST TRADED PAIRS</div>
            <div class={styles.adbPairList}>
                {#each stats.pair_volume as pair}
                    <div class={styles.adbPairRow}>
                        <span class={styles.adbPairSymbol}>{pair.symbol}</span>
                        <div class={styles.adbPairBarBg}>
                            <div class={styles.adbPairBarFill} style="width: {(pair.value / Math.max(...stats.pair_volume.map(p => p.value), 1)) * 100}%"></div>
                        </div>
                        <span class={styles.adbPairVal}>{pair.value.toFixed(0)}</span>
                    </div>
                {/each}
            </div>

            <div class={styles.adbSectionTitle}>MOST PROFITABLE PAIRS</div>
            <div class={styles.adbPairList}>
                {#each stats.top_pairs_profitability as pair}
                    <div class={styles.adbPairRow}>
                        <span class={styles.adbPairSymbol}>{pair.symbol}</span>
                        <div class={styles.adbPairBarBg}>
                            <div class="{styles.adbPairBarFill} {styles.adbGreen}" style="width: {(pair.value / Math.max(...stats.top_pairs_profitability.map(p => Math.abs(p.value)), 1)) * 100}%"></div>
                        </div>
                        <span class="{styles.adbPairVal} {styles.adbPos}">{formatUsd(pair.value)}</span>
                    </div>
                {/each}
            </div>

            <div class={styles.adbSectionTitle}>LEAST PROFITABLE PAIRS</div>
            <div class={styles.adbPairList}>
                {#each stats.bottom_pairs_profitability as pair}
                    <div class={styles.adbPairRow}>
                        <span class={styles.adbPairSymbol}>{pair.symbol}</span>
                        <div class={styles.adbPairBarBg}>
                            <div class="{styles.adbPairBarFill} {styles.adbRed}" style="width: {(Math.abs(pair.value) / Math.max(...stats.bottom_pairs_profitability.map(p => Math.abs(p.value)), 1)) * 100}%"></div>
                        </div>
                        <span class="{styles.adbPairVal} {styles.adbNeg}">{formatUsd(pair.value)}</span>
                    </div>
                {/each}
            </div>
        {/if}

        <!-- Commissions -->
        {#if currentFilter === '' || currentFilter === 'commissions'}
            <div class={styles.adbSectionTitle}>COMMISSIONS BY DAY</div>
            <div class={styles.adbChartBox}>
                <div class={styles.adbMiniChart}>
                    {#each stats.daily_commissions.slice(-30) as day, i}
                        {@const maxFee = Math.max(...stats.daily_commissions.map(d => d.fees), 0.01)}
                        <div class={styles.adbBarLine} style="left: {(i / Math.max(stats.daily_commissions.length - 1, 1)) * 100}%; height: {day.fees / maxFee * 100}%; background: #64748b; bottom: 0;">
                        </div>
                    {/each}
                </div>
            </div>

            <div class={styles.adbSectionTitle}>CUMULATIVE COMMISSIONS</div>
            <div class={styles.adbChartBox}>
                <div class={styles.adbMiniChart}>
                    {#each stats.cumulative_commissions.slice(-50) as [ts, val], i}
                        {@const maxCum = stats.cumulative_commissions.length > 0 ? Math.max(...stats.cumulative_commissions.map(c => c[1]), 0.01) : 1}
                        <div class={styles.adbBarLine} style="left: {(i / Math.max(stats.cumulative_commissions.length - 1, 1)) * 100}%; height: {val / maxCum * 100}%; background: #64748b; bottom: 0; width: 2px;">
                        </div>
                    {/each}
                </div>
            </div>

            <div class={styles.adbSectionTitle}>COMMISSIONS / PNL RATIO</div>
            <div class={styles.adbChartBox}>
                <div class={styles.adbMiniChart}>
                    {#each stats.fee_pnl_ratio.slice(-30) as day, i}
                        {@const maxRat = Math.max(...stats.fee_pnl_ratio.map(d => d.ratio), 0.01)}
                        <div class={styles.adbBarLine} style="left: {(i / Math.max(stats.fee_pnl_ratio.length - 1, 1)) * 100}%; height: {Math.min(day.ratio / maxRat * 100, 100)}%; background: '#a855f7'; bottom: 0;">
                        </div>
                    {/each}
                </div>
            </div>
        {/if}

        <!-- PnL Calendar -->
        {#if currentFilter === '' || currentFilter === 'summary'}
            <div class={styles.adbSectionTitle}>CALENDAR</div>
            <div class={styles.adbCalendar}>
                {#each stats.pnl_calendar.slice(-42) as day}
                    {@const intensity = Math.min(Math.abs(day.pnl) / Math.max(...stats.pnl_calendar.map(d => Math.abs(d.pnl)), 0.01), 1)}
                    <div class={styles.adbCalDay}
                        style="background: {day.pnl >= 0 ? `rgba(16,185,129,${0.15 + intensity * 0.7})` : `rgba(239,68,68,${0.15 + intensity * 0.7})`}"
                        title="{day.date}: {formatUsd(day.pnl)}">
                        <span class={styles.adbCalNum}>{day.day}</span>
                    </div>
                {/each}
            </div>
        {/if}

        <!-- Monthly Summary -->
        {#if currentFilter === '' || currentFilter === 'summary'}
            <div class={styles.adbSectionTitle}>MONTHLY SUMMARY</div>
            <div class={styles.adbChartBox}>
                <div class={styles.adbMiniChart}>
                    {#each stats.monthly_summary as month, i}
                        {@const maxPnL = Math.max(...stats.monthly_summary.map(m => Math.abs(m.net_pnl)), 1)}
                        <div class={styles.adbBarLine} style="left: {(i / Math.max(stats.monthly_summary.length - 1, 1)) * 100}%; bottom: 0;
                            height: {Math.abs(month.net_pnl) / maxPnL * 100}%;
                            background: {month.net_pnl >= 0 ? '#10b981' : '#ef4444'}; width: 14px;">
                        </div>
                    {/each}
                </div>
                <div class={styles.adbAxisLabels}>
                    {#each stats.monthly_summary as month}
                        <span>{month.month}</span>
                    {/each}
                </div>
            </div>
        {/if}
    {/if}
</div>


