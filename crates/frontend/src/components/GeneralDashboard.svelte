<script lang="ts">
    import { getState } from '../state.svelte';
    import type { DashboardStats } from '../state.svelte';

    const app = getState();
    let stats = $state<DashboardStats | null>(null);
    let recommendations = $state<any[]>([]);
    let loading = $state(true);

    async function fetchAll() {
        loading = true;
        try {
            const [statsRes, recsRes] = await Promise.all([
                fetch('/api/dashboard/stats'),
                fetch('/api/historical-recommendations'),
            ]);
            if (statsRes.ok) stats = await statsRes.json();
            if (recsRes.ok) {
                const data = await recsRes.json();
                recommendations = data.recommendations || [];
            }
        } catch (_) {
        } finally {
            loading = false;
        }
    }

    $effect(() => { fetchAll(); });
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
