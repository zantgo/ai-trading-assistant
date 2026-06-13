<script lang="ts">
    import { getState } from '../state.svelte';
    import type { CoreStats, StreakMetrics, CalendarDay, StyleSegment, PairStat } from '../state.svelte';

    const app = getState();

    $effect(() => {
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

<div class="adb-layout">
    <!-- Filter Ribbon -->
    <div class="adb-ribbon">
        <div class="adb-filters-left">
            <select class="adb-select" bind:value={app.dashboardPeriod}>
                <option>All</option>
                <option>7d</option>
                <option>30d</option>
                <option>90d</option>
            </select>
            <select class="adb-select" bind:value={app.dashboardOrigin}>
                <option>All</option>
                <option>MANUAL</option>
                <option>AUTOMATED</option>
            </select>
        </div>
        <div class="adb-filters-center">
            <button class="adb-filter-btn" class:active={currentFilter === ''} onclick={() => { currentFilter = ''; currentFilter = ''; }}>All</button>
            {#each FILTERS as f (f.key)}
                <button class="adb-filter-btn" class:active={currentFilter === f.key}
                    onclick={() => { currentFilter = f.key; currentFilter = f.key; }}>{f.label}</button>
            {/each}
        </div>
    </div>

    {#if !app.dashboardStats}
        <div class="adb-empty">No trade data available. Execute trades to populate the dashboard.</div>
    {:else}
        {@const stats = app.dashboardStats}

        <!-- Resumen / Rendimiento / Comportamiento -->
        {#if currentFilter === '' || currentFilter === 'summary' || currentFilter === 'performance' || currentFilter === 'behavior'}
            <div class="adb-section-title">YOUR STATISTICS</div>
            <div class="adb-stats-grid">
                <div class="adb-stat-card">
                    <span class="adb-stat-label">Total PnL</span>
                    <span class="adb-stat-value" class:adb-pos={stats.core_stats.total_pnl >= 0} class:adb-neg={stats.core_stats.total_pnl < 0}>
                        {formatUsd(stats.core_stats.total_pnl)}
                    </span>
                </div>
                <div class="adb-stat-card">
                    <span class="adb-stat-label">Win Rate</span>
                    <span class="adb-stat-value">{formatPct(stats.core_stats.win_rate * 100)}</span>
                </div>
                <div class="adb-stat-card">
                    <span class="adb-stat-label">Average Loss</span>
                    <span class="adb-stat-value adb-neg">{formatUsd(stats.core_stats.avg_loss)}</span>
                </div>
                <div class="adb-stat-card">
                    <span class="adb-stat-label">Average Gain</span>
                    <span class="adb-stat-value adb-pos">{formatUsd(stats.core_stats.avg_gain)}</span>
                </div>
                <div class="adb-stat-card">
                    <span class="adb-stat-label">Expectancy</span>
                    <span class="adb-stat-value">{formatUsd(stats.core_stats.expectancy)}</span>
                </div>
                <div class="adb-stat-card">
                    <span class="adb-stat-label">Average R:R</span>
                    <span class="adb-stat-value">1:{stats.core_stats.avg_risk_reward_ratio.toFixed(2)}</span>
                </div>
                <div class="adb-stat-card">
                    <span class="adb-stat-label">Largest Loss</span>
                    <span class="adb-stat-value adb-neg">{formatUsd(stats.core_stats.largest_loss)}</span>
                </div>
                <div class="adb-stat-card">
                    <span class="adb-stat-label">Largest Gain</span>
                    <span class="adb-stat-value adb-pos">{formatUsd(stats.core_stats.largest_gain)}</span>
                </div>
            </div>

            <!-- Equity Curve -->
            <div class="adb-section-title">CUMULATIVE GAIN</div>
            <div class="adb-chart-box">
                <div class="adb-mini-chart">
                    {#each stats.equity_curve.slice(-50) as [ts, val], i}
                        <div class="adb-bar-line" style="left: {(i / Math.max(stats.equity_curve.length - 1, 1)) * 100}%; bottom: 0; height: {val === 0 ? 0 : Math.min(Math.abs(val) / Math.max(Math.abs(stats.core_stats.total_pnl), 1) * 100, 100)}%; background: {val >= 0 ? '#10b981' : '#ef4444'}">
                        </div>
                    {/each}
                </div>
            </div>

            <!-- Daily Activity -->
            <div class="adb-section-title">ACTIVITY</div>
            <div class="adb-chart-box">
                <div class="adb-mini-chart">
                    {#each stats.daily_activity.slice(-30) as day, i}
                        <div class="adb-stacked-bar" style="left: {(i / Math.max(stats.daily_activity.length - 1, 1)) * 100}%">
                            <div class="adb-stack-long" style="height: {day.longs / Math.max(day.longs + day.shorts, 1) * 100}%; background: #10b981;"></div>
                            <div class="adb-stack-short" style="height: {day.shorts / Math.max(day.longs + day.shorts, 1) * 100}%; background: #ef4444;"></div>
                        </div>
                    {/each}
                </div>
            </div>

            <!-- Daily PnL -->
            <div class="adb-section-title">PNL PER DAY</div>
            <div class="adb-chart-box">
                <div class="adb-mini-chart">
                    {#each stats.daily_pnl.slice(-30) as day, i}
                        {@const maxVal = Math.max(...stats.daily_pnl.map(d => Math.abs(d.pnl)), 1)}
                        <div class="adb-bar-line" style="left: {(i / Math.max(stats.daily_pnl.length - 1, 1)) * 100}%;
                            height: {Math.abs(day.pnl) / maxVal * 100}%;
                            background: {day.pnl >= 0 ? '#10b981' : '#ef4444'}; bottom: 0;">
                        </div>
                    {/each}
                </div>
            </div>

            <!-- Win Rate by Hour -->
            <div class="adb-section-title">WIN RATE BY HOUR</div>
            <div class="adb-chart-box">
                <div class="adb-mini-chart">
                    {#each stats.win_rate_by_hour as h, i}
                        <div class="adb-bar-line" style="left: {(i / 23) * 100}%; bottom: 0; height: {h.win_rate * 100}%; background: #3b82f6; width: 3px;">
                        </div>
                    {/each}
                </div>
                <div class="adb-axis-labels">
                    <span>00</span><span>06</span><span>12</span><span>18</span><span>23</span>
                </div>
            </div>

            <!-- Win Rate by Weekday -->
            <div class="adb-section-title">WIN RATE BY DAY</div>
            <div class="adb-chart-box">
                <div class="adb-mini-chart">
                    {#each stats.win_rate_by_weekday as day, i}
                        <div class="adb-bar-line" style="left: {(i / 6) * 100}%; bottom: 0; height: {day.win_rate * 100}%; background: #8b5cf6; width: 12px;">
                        </div>
                    {/each}
                </div>
                <div class="adb-axis-labels">
                    <span>Sun</span><span>Mon</span><span>Tue</span><span>Wed</span><span>Thu</span><span>Fri</span><span>Sat</span>
                </div>
            </div>

            <!-- Direction Breakdown -->
            <div class="adb-section-title">TRADE DIRECTION</div>
            <div class="adb-two-col">
                <div class="adb-donut-container">
                    <svg viewBox="0 0 100 100" class="adb-donut">
                        <circle cx="50" cy="50" r="35" fill="none" stroke="#1e293b" stroke-width="12" />
                        <circle cx="50" cy="50" r="35" fill="none" stroke="#10b981" stroke-width="12"
                            stroke-dasharray="{(stats.direction_breakdown.longs / (stats.direction_breakdown.longs + stats.direction_breakdown.shorts || 1)) * 220} 220"
                            stroke-dashoffset="0" transform="rotate(-90 50 50)" />
                        <circle cx="50" cy="50" r="35" fill="none" stroke="#ef4444" stroke-width="12"
                            stroke-dasharray="{(stats.direction_breakdown.shorts / (stats.direction_breakdown.longs + stats.direction_breakdown.shorts || 1)) * 220} 220"
                            stroke-dashoffset="-{(stats.direction_breakdown.longs / (stats.direction_breakdown.longs + stats.direction_breakdown.shorts || 1)) * 220}"
                            transform="rotate(-90 50 50)" />
                    </svg>
                    <div class="adb-donut-legend">
                        <span class="adb-legend-long">Long: {stats.direction_breakdown.longs}</span>
                        <span class="adb-legend-short">Short: {stats.direction_breakdown.shorts}</span>
                    </div>
                </div>
                <div class="adb-expectancy-box">
                    <span class="adb-exp-label">Long Expectancy</span>
                    <span class="adb-exp-val">{formatUsd(stats.direction_breakdown.long_expectancy)}</span>
                    <span class="adb-exp-label">Short Expectancy</span>
                    <span class="adb-exp-val">{formatUsd(stats.direction_breakdown.short_expectancy)}</span>
                </div>
            </div>

            <!-- Trader Style -->
            <div class="adb-section-title">TRADER PROFILE</div>
            <div class="adb-style-grid">
                <div class="adb-style-card">
                    <span class="adb-style-name">Scalper</span>
                    <span class="adb-style-count">{stats.trader_style.scalper.count} trades</span>
                    <span class="adb-style-dur">{(stats.trader_style.scalper.avg_duration_minutes / 60).toFixed(1)}h avg</span>
                    <span class="adb-style-wr">{formatPct(stats.trader_style.scalper.win_rate * 100)} WR</span>
                </div>
                <div class="adb-style-card">
                    <span class="adb-style-name">Day Trader</span>
                    <span class="adb-style-count">{stats.trader_style.day_trader.count} trades</span>
                    <span class="adb-style-dur">{(stats.trader_style.day_trader.avg_duration_minutes / 60).toFixed(1)}h avg</span>
                    <span class="adb-style-wr">{formatPct(stats.trader_style.day_trader.win_rate * 100)} WR</span>
                </div>
                <div class="adb-style-card">
                    <span class="adb-style-name">Swing</span>
                    <span class="adb-style-count">{stats.trader_style.swing_trader.count} trades</span>
                    <span class="adb-style-dur">{(stats.trader_style.swing_trader.avg_duration_minutes / 60).toFixed(1)}h avg</span>
                    <span class="adb-style-wr">{formatPct(stats.trader_style.swing_trader.win_rate * 100)} WR</span>
                </div>
            </div>
        {/if}

        <!-- Streaks -->
        {#if currentFilter === '' || currentFilter === 'streaks'}
            <div class="adb-section-title">WINNING STREAKS</div>
            <div class="adb-stats-grid">
                <div class="adb-stat-card">
                    <span class="adb-stat-label">Average Streak</span>
                    <span class="adb-stat-value">{stats.winning_streaks.avg_streak_length.toFixed(1)}</span>
                </div>
                <div class="adb-stat-card">
                    <span class="adb-stat-label">Max Streak</span>
                    <span class="adb-stat-value">{stats.winning_streaks.max_streak_length}</span>
                </div>
                <div class="adb-stat-card">
                    <span class="adb-stat-label">Max Value</span>
                    <span class="adb-stat-value adb-pos">{formatUsd(stats.winning_streaks.max_consecutive_value)}</span>
                </div>
            </div>

            <div class="adb-section-title">LOSING STREAKS</div>
            <div class="adb-stats-grid">
                <div class="adb-stat-card">
                    <span class="adb-stat-label">Average Streak</span>
                    <span class="adb-stat-value">{stats.losing_streaks.avg_streak_length.toFixed(1)}</span>
                </div>
                <div class="adb-stat-card">
                    <span class="adb-stat-label">Max Streak</span>
                    <span class="adb-stat-value">{stats.losing_streaks.max_streak_length}</span>
                </div>
                <div class="adb-stat-card">
                    <span class="adb-stat-label">Max Value</span>
                    <span class="adb-stat-value adb-neg">{formatUsd(stats.losing_streaks.max_consecutive_value)}</span>
                </div>
            </div>

            <div class="adb-section-title">POST-LOSS RECOVERY</div>
            <div class="adb-stat-card">
                <span class="adb-stat-value">{formatPct(stats.post_loss_recovery_pct)}</span>
                <span class="adb-stat-label">winning trades after loss</span>
            </div>
        {/if}

        <!-- Pairs -->
        {#if currentFilter === '' || currentFilter === 'pairs'}
            <div class="adb-section-title">MOST TRADED PAIRS</div>
            <div class="adb-pair-list">
                {#each stats.pair_volume as pair}
                    <div class="adb-pair-row">
                        <span class="adb-pair-symbol">{pair.symbol}</span>
                        <div class="adb-pair-bar-bg">
                            <div class="adb-pair-bar-fill" style="width: {(pair.value / Math.max(...stats.pair_volume.map(p => p.value), 1)) * 100}%"></div>
                        </div>
                        <span class="adb-pair-val">{pair.value.toFixed(0)}</span>
                    </div>
                {/each}
            </div>

            <div class="adb-section-title">MOST PROFITABLE PAIRS</div>
            <div class="adb-pair-list">
                {#each stats.top_pairs_profitability as pair}
                    <div class="adb-pair-row">
                        <span class="adb-pair-symbol">{pair.symbol}</span>
                        <div class="adb-pair-bar-bg">
                            <div class="adb-pair-bar-fill adb-green" style="width: {(pair.value / Math.max(...stats.top_pairs_profitability.map(p => Math.abs(p.value)), 1)) * 100}%"></div>
                        </div>
                        <span class="adb-pair-val adb-pos">{formatUsd(pair.value)}</span>
                    </div>
                {/each}
            </div>

            <div class="adb-section-title">LEAST PROFITABLE PAIRS</div>
            <div class="adb-pair-list">
                {#each stats.bottom_pairs_profitability as pair}
                    <div class="adb-pair-row">
                        <span class="adb-pair-symbol">{pair.symbol}</span>
                        <div class="adb-pair-bar-bg">
                            <div class="adb-pair-bar-fill adb-red" style="width: {(Math.abs(pair.value) / Math.max(...stats.bottom_pairs_profitability.map(p => Math.abs(p.value)), 1)) * 100}%"></div>
                        </div>
                        <span class="adb-pair-val adb-neg">{formatUsd(pair.value)}</span>
                    </div>
                {/each}
            </div>
        {/if}

        <!-- Commissions -->
        {#if currentFilter === '' || currentFilter === 'commissions'}
            <div class="adb-section-title">COMMISSIONS BY DAY</div>
            <div class="adb-chart-box">
                <div class="adb-mini-chart">
                    {#each stats.daily_commissions.slice(-30) as day, i}
                        {@const maxFee = Math.max(...stats.daily_commissions.map(d => d.fees), 0.01)}
                        <div class="adb-bar-line" style="left: {(i / Math.max(stats.daily_commissions.length - 1, 1)) * 100}%; height: {day.fees / maxFee * 100}%; background: #f59e0b; bottom: 0;">
                        </div>
                    {/each}
                </div>
            </div>

            <div class="adb-section-title">CUMULATIVE COMMISSIONS</div>
            <div class="adb-chart-box">
                <div class="adb-mini-chart">
                    {#each stats.cumulative_commissions.slice(-50) as [ts, val], i}
                        {@const maxCum = stats.cumulative_commissions.length > 0 ? Math.max(...stats.cumulative_commissions.map(c => c[1]), 0.01) : 1}
                        <div class="adb-bar-line" style="left: {(i / Math.max(stats.cumulative_commissions.length - 1, 1)) * 100}%; height: {val / maxCum * 100}%; background: #f59e0b; bottom: 0; width: 2px;">
                        </div>
                    {/each}
                </div>
            </div>

            <div class="adb-section-title">COMMISSIONS / PNL RATIO</div>
            <div class="adb-chart-box">
                <div class="adb-mini-chart">
                    {#each stats.fee_pnl_ratio.slice(-30) as day, i}
                        {@const maxRat = Math.max(...stats.fee_pnl_ratio.map(d => d.ratio), 0.01)}
                        <div class="adb-bar-line" style="left: {(i / Math.max(stats.fee_pnl_ratio.length - 1, 1)) * 100}%; height: {Math.min(day.ratio / maxRat * 100, 100)}%; background: '#a855f7'; bottom: 0;">
                        </div>
                    {/each}
                </div>
            </div>
        {/if}

        <!-- PnL Calendar -->
        {#if currentFilter === '' || currentFilter === 'summary'}
            <div class="adb-section-title">CALENDAR</div>
            <div class="adb-calendar">
                {#each stats.pnl_calendar.slice(-42) as day}
                    {@const intensity = Math.min(Math.abs(day.pnl) / Math.max(...stats.pnl_calendar.map(d => Math.abs(d.pnl)), 0.01), 1)}
                    <div class="adb-cal-day"
                        style="background: {day.pnl >= 0 ? `rgba(16,185,129,${0.15 + intensity * 0.7})` : `rgba(239,68,68,${0.15 + intensity * 0.7})`}"
                        title="{day.date}: {formatUsd(day.pnl)}">
                        <span class="adb-cal-num">{day.day}</span>
                    </div>
                {/each}
            </div>
        {/if}

        <!-- Monthly Summary -->
        {#if currentFilter === '' || currentFilter === 'summary'}
            <div class="adb-section-title">MONTHLY SUMMARY</div>
            <div class="adb-chart-box">
                <div class="adb-mini-chart">
                    {#each stats.monthly_summary as month, i}
                        {@const maxPnL = Math.max(...stats.monthly_summary.map(m => Math.abs(m.net_pnl)), 1)}
                        <div class="adb-bar-line" style="left: {(i / Math.max(stats.monthly_summary.length - 1, 1)) * 100}%; bottom: 0;
                            height: {Math.abs(month.net_pnl) / maxPnL * 100}%;
                            background: {month.net_pnl >= 0 ? '#10b981' : '#ef4444'}; width: 14px;">
                        </div>
                    {/each}
                </div>
                <div class="adb-axis-labels">
                    {#each stats.monthly_summary as month}
                        <span>{month.month}</span>
                    {/each}
                </div>
            </div>
        {/if}
    {/if}
</div>

<style>
    .adb-layout { max-width: 1400px; margin: 0 auto; width: 100%; padding: 16px; box-sizing: border-box; }
    .adb-ribbon {
        display: flex; justify-content: space-between; align-items: center; gap: 12px; margin-bottom: 16px;
        padding: 10px 16px; background: #131722; border: 1px solid #2a2e39; border-radius: 8px; flex-wrap: wrap;
    }
    .adb-filters-left { display: flex; gap: 8px; }
    .adb-filters-center { display: flex; gap: 4px; flex-wrap: wrap; }
    .adb-select {
        background: #0f131c; border: 1px solid #2a2e39; color: #e2e8f0; padding: 6px 10px;
        border-radius: 4px; font-size: 11px; outline: none; cursor: pointer;
    }
    .adb-filter-btn {
        background: #0f131c; border: 1px solid #2a2e39; color: #64748b; padding: 6px 12px;
        border-radius: 4px; font-size: 10px; font-weight: 600; cursor: pointer; text-transform: uppercase;
    }
    .adb-filter-btn.active { background: rgba(59,130,246,0.12); border-color: #3b82f6; color: #3b82f6; }
    .adb-filter-btn:hover { border-color: #3b82f6; }

    .adb-empty { font-size: 12px; color: #64748b; text-align: center; padding: 40px; font-style: italic; }

    .adb-section-title {
        font-size: 11px; font-weight: 700; color: #64748b; text-transform: uppercase;
        letter-spacing: 0.06em; padding: 12px 0 8px; border-bottom: 1px solid #1e293b; margin-bottom: 10px;
    }
    .adb-stats-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(150px, 1fr)); gap: 8px; margin-bottom: 12px; }
    .adb-stat-card {
        background: #131722; border: 1px solid #1e293b; border-radius: 8px; padding: 12px;
        display: flex; flex-direction: column; gap: 4px;
    }
    .adb-stat-label { font-size: 9px; color: #64748b; text-transform: uppercase; font-weight: 600; }
    .adb-stat-value { font-size: 16px; font-weight: 800; color: #e2e8f0; font-family: monospace; }
    .adb-pos { color: #10b981; }
    .adb-neg { color: #ef4444; }

    .adb-chart-box { background: #131722; border: 1px solid #1e293b; border-radius: 8px; padding: 12px; margin-bottom: 12px; height: 120px; position: relative; }
    .adb-mini-chart { height: 100%; position: relative; }
    .adb-bar-line { position: absolute; width: 2px; min-height: 1px; border-radius: 1px 1px 0 0; transition: height 0.3s; }
    .adb-stacked-bar { position: absolute; width: 6px; height: 100%; display: flex; flex-direction: column; justify-content: flex-end; }
    .adb-stack-long { width: 100%; }
    .adb-stack-short { width: 100%; }
    .adb-axis-labels { display: flex; justify-content: space-between; font-size: 8px; color: #64748b; padding: 4px 0; }
    .adb-two-col { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; margin-bottom: 12px; }
    .adb-donut-container { display: flex; align-items: center; gap: 12px; }
    .adb-donut { width: 80px; height: 80px; }
    .adb-donut-legend { display: flex; flex-direction: column; gap: 4px; font-size: 10px; }
    .adb-legend-long { color: #10b981; font-weight: 600; }
    .adb-legend-short { color: #ef4444; font-weight: 600; }
    .adb-expectancy-box { display: flex; flex-direction: column; gap: 4px; justify-content: center; }
    .adb-exp-label { font-size: 10px; color: #64748b; }
    .adb-exp-val { font-size: 14px; color: #e2e8f0; font-weight: 700; font-family: monospace; }

    .adb-style-grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 8px; margin-bottom: 12px; }
    .adb-style-card {
        background: #131722; border: 1px solid #1e293b; border-radius: 8px; padding: 12px;
        display: flex; flex-direction: column; gap: 3px; text-align: center;
    }
    .adb-style-name { font-size: 11px; font-weight: 700; color: #cbd5e1; }
    .adb-style-count { font-size: 10px; color: #64748b; }
    .adb-style-dur { font-size: 10px; color: #94a3b8; }
    .adb-style-wr { font-size: 12px; font-weight: 700; color: #60a5fa; }

    .adb-pair-list { display: flex; flex-direction: column; gap: 6px; margin-bottom: 12px; }
    .adb-pair-row { display: flex; align-items: center; gap: 8px; }
    .adb-pair-symbol { font-size: 10px; color: #cbd5e1; font-weight: 600; width: 60px; text-align: right; }
    .adb-pair-bar-bg { flex: 1; height: 8px; background: #1e293b; border-radius: 4px; overflow: hidden; }
    .adb-pair-bar-fill { height: 100%; background: #3b82f6; border-radius: 4px; transition: width 0.5s; }
    .adb-green { background: #10b981; }
    .adb-red { background: #ef4444; }
    .adb-pair-val { font-size: 10px; color: #94a3b8; font-family: monospace; width: 70px; text-align: right; }

    .adb-calendar { display: grid; grid-template-columns: repeat(7, 1fr); gap: 3px; margin-bottom: 12px; }
    .adb-cal-day {
        aspect-ratio: 1; display: flex; align-items: center; justify-content: center; border-radius: 4px;
        font-size: 9px; font-weight: 600; cursor: default;
    }
    .adb-cal-num { color: #cbd5e1; }

    @media (max-width: 768px) {
        .adb-stats-grid { grid-template-columns: repeat(2, 1fr); }
        .adb-style-grid { grid-template-columns: 1fr; }
        .adb-two-col { grid-template-columns: 1fr; }
        .adb-ribbon { flex-direction: column; align-items: flex-start; }
    }
</style>
