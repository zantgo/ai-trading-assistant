<script lang="ts">
    import styles from './PerformanceDashboard.module.css';
    import { useAppStore } from '../state.svelte';
    import type {
        StrategyAnalyticsRow, RiskAnalyticsRow, PerformanceMatrixRow,
        OptimizationReport, TradeAnalyticsRecord,
    } from '../types/analytics';

    const app = useAppStore();

    type Panel = 'overview' | 'strategy' | 'risk' | 'regimes' | 'trades';

    let activePanel = $state<Panel>('overview');
    let loading = $state(false);

    let dashboardStats = $state<any>(null);
    let strategyRows = $state<StrategyAnalyticsRow[]>([]);
    let riskData = $state<RiskAnalyticsRow | null>(null);
    let performanceRows = $state<PerformanceMatrixRow[]>([]);
    let optimizationReport = $state<OptimizationReport | null>(null);
    let tradeRecords = $state<TradeAnalyticsRecord[]>([]);
    let errorMsg = $state<string | null>(null);

    const sessionCapital = $derived(app.sessionCapital ?? 10000);

    async function fetchPanelData() {
        loading = true; errorMsg = null;
        try {
            const [statsRes, strategyRes, riskRes, perfRes, optRes, tradesRes] = await Promise.all([
                fetch(`/api/dashboard/stats?initial_capital=${sessionCapital}`),
                fetch('/api/analytics/strategy'),
                fetch('/api/analytics/risk'),
                fetch('/api/analytics/performance'),
                fetch('/api/analytics/optimization'),
                fetch('/api/analytics/trades?limit=200'),
            ]);
            if (statsRes.ok) dashboardStats = await statsRes.json();
            if (strategyRes.ok) strategyRows = await strategyRes.json();
            if (riskRes.ok) riskData = await riskRes.json();
            if (perfRes.ok) performanceRows = await perfRes.json();
            if (optRes.ok) optimizationReport = await optRes.json();
            if (tradesRes.ok) tradeRecords = await tradesRes.json();
        } catch (e: any) {
            errorMsg = e?.message ?? 'Failed to fetch analytics data';
        } finally {
            loading = false;
        }
    }

    $effect(() => { fetchPanelData(); });

    function fmtNum(n: number | null | undefined, decimals: number = 2): string {
        if (n == null) return '--';
        if (!isFinite(n)) return n > 0 ? '∞' : '-∞';
        return n.toFixed(decimals);
    }

    function fmtPnl(n: number): string {
        const prefix = n > 0 ? '+' : '';
        return prefix + fmtNum(n);
    }

    function fmtPct(n: number): string {
        return fmtNum(n) + '%';
    }

    function classificationBadge(c: string): string {
        const map: Record<string, string> = {
            StrongEdge: styles.badgeStrong,
            ModerateEdge: styles.badgeModerate,
            WeakMarginalEdge: styles.badgeWeak,
            NoEdgeNegative: styles.badgeNone,
            InsufficientData: styles.badgeInsufficient,
        };
        return map[c] ?? styles.badgeInsufficient;
    }

    function compatibilityBadge(c: string): string {
        const map: Record<string, string> = {
            Strong: styles.labelStrong,
            Favorable: styles.labelFavorable,
            Marginal: styles.labelMarginal,
            Avoid: styles.labelAvoid,
        };
        return map[c] ?? styles.labelAvoid;
    }

    function sharpeClass(v: number | null): string {
        if (v == null) return styles.statNeutral;
        if (v > 2.0) return styles.statPositive;
        if (v >= 1.0) return styles.statPositive;
        if (v >= 0.5) return styles.statNeutral;
        return styles.statNegative;
    }

    function pnlClass(v: number): string {
        if (v > 0) return styles.statPositive;
        if (v < 0) return styles.statNegative;
        return styles.statNeutral;
    }
</script>

<div class={styles.dashboard}>
    <div class={styles.sidebar}>
        <h2 class={styles.sidebarTitle}>PERFORMANCE ANALYTICS</h2>
        <button class="{styles.sidebarBtn} {activePanel === 'overview' ? styles.sidebarBtnActive : ''}" onclick={() => activePanel = 'overview'}>📊 Overview</button>
        <button class="{styles.sidebarBtn} {activePanel === 'strategy' ? styles.sidebarBtnActive : ''}" onclick={() => activePanel = 'strategy'}>🎯 Strategy (NHST)</button>
        <button class="{styles.sidebarBtn} {activePanel === 'risk' ? styles.sidebarBtnActive : ''}" onclick={() => activePanel = 'risk'}>⚠ Risk Metrics</button>
        <button class="{styles.sidebarBtn} {activePanel === 'regimes' ? styles.sidebarBtnActive : ''}" onclick={() => activePanel = 'regimes'}>🗺 Regime Map</button>
        <button class="{styles.sidebarBtn} {activePanel === 'trades' ? styles.sidebarBtnActive : ''}" onclick={() => activePanel = 'trades'}>📋 Trade Ledger</button>
    </div>

    <div class={styles.content}>
        {#if loading}
            <div class={styles.loading}>Loading analytics data...</div>
        {:else if errorMsg}
            <div class={styles.loading} style="color:#ef5350">{errorMsg}</div>
        {:else}
            {#if activePanel === 'overview'}
                <h3 class={styles.sectionTitle}>Performance Overview</h3>
                <p class={styles.sectionDesc}>Realized trading performance across all closed trades.</p>
                {#if dashboardStats?.core_stats}
                    {@const cs = dashboardStats.core_stats}
                    <div class={styles.statsGrid}>
                        <div class={styles.statCard}>
                            <div class={styles.statLabel}>Total P&L</div>
                            <div class="{styles.statValue} {pnlClass(cs.total_pnl)}">{fmtPnl(cs.total_pnl)}</div>
                        </div>
                        <div class={styles.statCard}>
                            <div class={styles.statLabel}>Win Rate</div>
                            <div class="{styles.statValue} {cs.win_rate >= 50 ? styles.statPositive : styles.statNegative}">{fmtNum(cs.win_rate)}%</div>
                            <div class={styles.statSub}>{cs.wins}W / {cs.losses}L / {cs.total_trades}T</div>
                        </div>
                        <div class={styles.statCard}>
                            <div class={styles.statLabel}>Profit Factor</div>
                            <div class="{styles.statValue} {cs.profit_factor >= 1.5 ? styles.statPositive : pnlClass(cs.profit_factor - 1)}">{fmtNum(cs.profit_factor)}</div>
                        </div>
                        <div class={styles.statCard}>
                            <div class={styles.statLabel}>Expectancy</div>
                            <div class="{styles.statValue} {pnlClass(cs.expectancy)}">{fmtPnl(cs.expectancy)}</div>
                        </div>
                        <div class={styles.statCard}>
                            <div class={styles.statLabel}>Avg R:R</div>
                            <div class="{styles.statValue} {cs.avg_risk_reward_ratio >= 1 ? styles.statPositive : styles.statNeutral}">{fmtNum(cs.avg_risk_reward_ratio)}</div>
                        </div>
                        <div class={styles.statCard}>
                            <div class={styles.statLabel}>Largest Gain</div>
                            <div class={styles.statValue + ' ' + styles.statPositive}>+{fmtNum(cs.largest_gain)}</div>
                        </div>
                        <div class={styles.statCard}>
                            <div class={styles.statLabel}>Largest Loss</div>
                            <div class={styles.statValue + ' ' + styles.statNegative}>{fmtNum(cs.largest_loss)}</div>
                        </div>
                        <div class={styles.statCard}>
                            <div class={styles.statLabel}>Avg Gain / Loss</div>
                            <div class={styles.statValue}>{fmtNum(cs.avg_gain)} / {fmtNum(cs.avg_loss)}</div>
                        </div>
                    </div>
                {/if}

                {#if riskData}
                    {@const rd = riskData}
                    <h3 class={styles.sectionTitle} style="margin-top:1.5rem">Risk-Adjusted Metrics</h3>
                    <div class={styles.statsGrid}>
                        <div class={styles.statCard}>
                            <div class={styles.statLabel}>Sharpe Ratio</div>
                            <div class="{styles.statValue} {sharpeClass(rd.sharpe_ratio)}">{fmtNum(rd.sharpe_ratio)}</div>
                            <div class={styles.statSub}>
                                <div class={styles.gaugeBar}><div class="{styles.gaugeFill} {rd.sharpe_ratio != null && rd.sharpe_ratio > 2 ? styles.gaugeGreen : rd.sharpe_ratio != null && rd.sharpe_ratio >= 1 ? styles.gaugeBlue : rd.sharpe_ratio != null && rd.sharpe_ratio >= 0.5 ? styles.gaugeOrange : styles.gaugeRed}" style="width: {rd.sharpe_ratio != null ? Math.min(Math.max((rd.sharpe_ratio / 3) * 100, 5), 100) : 5}%"></div></div>
                            </div>
                        </div>
                        <div class={styles.statCard}>
                            <div class={styles.statLabel}>Sortino Ratio</div>
                            <div class="{styles.statValue} {sharpeClass(rd.sortino_ratio)}">{fmtNum(rd.sortino_ratio)}</div>
                        </div>
                        <div class={styles.statCard}>
                            <div class={styles.statLabel}>Max Drawdown</div>
                            <div class="{styles.statValue} {rd.maximum_drawdown_pct > 20 ? styles.statNegative : styles.statNeutral}">{fmtNum(rd.maximum_drawdown_pct)}%</div>
                        </div>
                        <div class={styles.statCard}>
                            <div class={styles.statLabel}>Calmar Ratio</div>
                            <div class="{styles.statValue} {sharpeClass(rd.calmar_ratio)}">{fmtNum(rd.calmar_ratio)}</div>
                        </div>
                        <div class={styles.statCard}>
                            <div class={styles.statLabel}>Ulcer Index</div>
                            <div class={styles.statValue}>{fmtNum(rd.ulcer_index)}</div>
                        </div>
                        <div class={styles.statCard}>
                            <div class={styles.statLabel}>Daily Volatility</div>
                            <div class={styles.statValue}>{fmtNum(rd.daily_volatility * 100)}%</div>
                        </div>
                        <div class={styles.statCard}>
                            <div class={styles.statLabel}>VaR 95%</div>
                            <div class={styles.statValue}>{fmtNum(rd.value_at_risk_95 * 100)}%</div>
                        </div>
                        <div class={styles.statCard}>
                            <div class={styles.statLabel}>Expected Shortfall 95%</div>
                            <div class={styles.statValue}>{fmtNum(rd.expected_shortfall_95 * 100)}%</div>
                        </div>
                    </div>
                {/if}

            {:else if activePanel === 'strategy'}
                <h3 class={styles.sectionTitle}>Strategy Analytics (NHST)</h3>
                <p class={styles.sectionDesc}>Null Hypothesis Significance Testing — determines whether each policy generates a statistically significant positive edge (H₀: μ ≤ 0 vs H₁: μ > 0).</p>
                {#if strategyRows.length === 0}
                    <div class={styles.equityPlaceholder}>No strategy data available. Trades must be closed for NHST analysis.</div>
                {:else}
                    <table class={styles.table}>
                        <thead>
                            <tr>
                                <th>Policy</th>
                                <th>Trades</th>
                                <th>Win Rate</th>
                                <th>Profit Factor</th>
                                <th>Expectancy</th>
                                <th>T-Stat</th>
                                <th>P-Value</th>
                                <th>P_MC</th>
                                <th>Classification</th>
                            </tr>
                        </thead>
                        <tbody>
                            {#each strategyRows as row}
                                <tr>
                                    <td>{row.policy_id}</td>
                                    <td>{row.total_trades}</td>
                                    <td class={pnlClass(row.win_rate - 50)}>{fmtNum(row.win_rate)}%</td>
                                    <td>{fmtNum(row.profit_factor)}</td>
                                    <td class={pnlClass(row.expectancy)}>{fmtPnl(row.expectancy)}</td>
                                    <td>{fmtNum(row.t_statistic)}</td>
                                    <td>{fmtNum(row.p_value, 4)}</td>
                                    <td>{fmtNum(row.p_mc, 4)}</td>
                                    <td><span class="{styles.badge} {classificationBadge(row.classification)}">{row.classification.replace(/([A-Z])/g, ' $1').trim()}</span></td>
                                </tr>
                            {/each}
                        </tbody>
                    </table>
                {/if}

            {:else if activePanel === 'risk'}
                <h3 class={styles.sectionTitle}>Risk Analytics</h3>
                <p class={styles.sectionDesc}>Risk-adjusted return metrics computed from portfolio equity history. Sharpe/Sortino annualized assuming 365 trading days for crypto.</p>
                {#if !riskData}
                    <div class={styles.equityPlaceholder}>No risk data available. Requires portfolio equity history.</div>
                {:else}
                    {@const rd = riskData}
                    <div class={styles.statsGrid}>
                        <div class={styles.statCard}>
                            <div class={styles.statLabel}>Sharpe Ratio</div>
                            <div class="{styles.statValue} {sharpeClass(rd.sharpe_ratio)}">
                                {fmtNum(rd.sharpe_ratio)}
                            </div>
                            <div class={styles.statSub}>{rd.sharpe_ratio != null && rd.sharpe_ratio > 2 ? 'Excellent' : rd.sharpe_ratio != null && rd.sharpe_ratio >= 1 ? 'Good' : rd.sharpe_ratio != null && rd.sharpe_ratio >= 0.5 ? 'Acceptable' : rd.sharpe_ratio != null && rd.sharpe_ratio >= 0 ? 'Poor' : 'Negative'}</div>
                            <div class={styles.gaugeBar}><div class="{styles.gaugeFill} {rd.sharpe_ratio != null && rd.sharpe_ratio > 2 ? styles.gaugeGreen : rd.sharpe_ratio != null && rd.sharpe_ratio >= 1 ? styles.gaugeBlue : rd.sharpe_ratio != null && rd.sharpe_ratio >= 0.5 ? styles.gaugeOrange : styles.gaugeRed}" style="width: {rd.sharpe_ratio != null ? Math.min(Math.max((rd.sharpe_ratio / 3) * 100, 3), 100) : 3}%"></div></div>
                        </div>
                        <div class={styles.statCard}>
                            <div class={styles.statLabel}>Sortino Ratio</div>
                            <div class="{styles.statValue} {sharpeClass(rd.sortino_ratio)}">{fmtNum(rd.sortino_ratio)}</div>
                        </div>
                        <div class={styles.statCard}>
                            <div class={styles.statLabel}>Calmar Ratio</div>
                            <div class="{styles.statValue} {sharpeClass(rd.calmar_ratio)}">{fmtNum(rd.calmar_ratio)}</div>
                        </div>
                        <div class={styles.statCard}>
                            <div class={styles.statLabel}>Ulcer Index</div>
                            <div class={styles.statValue}>{fmtNum(rd.ulcer_index)}</div>
                        </div>
                        <div class={styles.statCard}>
                            <div class={styles.statLabel}>Max Drawdown</div>
                            <div class="{styles.statValue} {rd.maximum_drawdown_pct > 20 ? styles.statNegative : styles.statNeutral}">{fmtPct(rd.maximum_drawdown_pct)}</div>
                            <div class={styles.statSub}>{rd.drawdown_count} drawdown events</div>
                        </div>
                        <div class={styles.statCard}>
                            <div class={styles.statLabel}>Avg Drawdown</div>
                            <div class="{styles.statValue} {rd.average_drawdown_pct > 10 ? styles.statNegative : styles.statNeutral}">{fmtPct(rd.average_drawdown_pct)}</div>
                        </div>
                        <div class={styles.statCard}>
                            <div class={styles.statLabel}>Daily Volatility</div>
                            <div class={styles.statValue}>{fmtPct(rd.daily_volatility * 100)}</div>
                        </div>
                        <div class={styles.statCard}>
                            <div class={styles.statLabel}>Downside Deviation</div>
                            <div class={styles.statValue}>{fmtPct(rd.downside_deviation * 100)}</div>
                        </div>
                        <div class={styles.statCard}>
                            <div class={styles.statLabel}>VaR 95%</div>
                            <div class={styles.statValue}>{fmtPct(rd.value_at_risk_95 * 100)}</div>
                            <div class={styles.statSub}>Worst expected daily loss in 95% of days</div>
                        </div>
                        <div class={styles.statCard}>
                            <div class={styles.statLabel}>Expected Shortfall 95%</div>
                            <div class={styles.statValue}>{fmtPct(rd.expected_shortfall_95 * 100)}</div>
                            <div class={styles.statSub}>Avg loss beyond VaR 95%</div>
                        </div>
                    </div>
                {/if}

            {:else if activePanel === 'regimes'}
                <h3 class={styles.sectionTitle}>Regime-Performance Map</h3>
                <p class={styles.sectionDesc}>Strategy performance segmented by market regime at trade entry. Regimes resolved from market_snapshots.</p>
                {#if performanceRows.length === 0}
                    <div class={styles.equityPlaceholder}>No regime-performance data available. Requires closed trades with market regime context.</div>
                {:else}
                    <div class={styles.regimeGrid}>
                        {#each performanceRows as row}
                            <div class={styles.regimeCard}>
                                <div class={styles.regimeName}>
                                    {row.regime}
                                    <span class="{styles.regimeLabel} {compatibilityBadge(row.compatibility_label)}" style="float:right">{row.compatibility_label}</span>
                                </div>
                                <div class={styles.regimeStats}>
                                    <span>{row.trade_count} trades</span>
                                    <span>WR: {fmtNum(row.win_rate)}%</span>
                                    <span>PF: {fmtNum(row.profit_factor)}</span>
                                </div>
                                <div class={styles.regimeStats}>
                                    <span>Avg R: {fmtNum(row.avg_r_multiple)}</span>
                                    <span class={pnlClass(row.total_pnl)}>P&L: {fmtPnl(row.total_pnl)}</span>
                                </div>
                            </div>
                        {/each}
                    </div>
                {/if}

                {#if optimizationReport?.recommendations?.length}
                    <h3 class={styles.sectionTitle} style="margin-top:1.5rem">Recommendations</h3>
                    {#each optimizationReport.recommendations as rec}
                        <div class={styles.recommendation}>{rec}</div>
                    {/each}
                {/if}

            {:else if activePanel === 'trades'}
                <h3 class={styles.sectionTitle}>Trade Analytics</h3>
                <p class={styles.sectionDesc}>Reconstructed closed trades with execution efficiency metrics.</p>
                {#if tradeRecords.length === 0}
                    <div class={styles.equityPlaceholder}>No trade data available.</div>
                {:else}
                    <table class={styles.table}>
                        <thead>
                            <tr>
                                <th>Trade ID</th>
                                <th>Symbol</th>
                                <th>Dir</th>
                                <th>Hold</th>
                                <th>Gross P&L</th>
                                <th>Net P&L</th>
                                <th>ROI</th>
                                <th>MFE</th>
                                <th>MAE</th>
                                <th>Flat</th>
                            </tr>
                        </thead>
                        <tbody>
                            {#each tradeRecords as t}
                                <tr>
                                    <td>{t.trade_id}</td>
                                    <td>{t.symbol}</td>
                                    <td>{t.direction}</td>
                                    <td>{t.hold_time_seconds < 3600 ? Math.round(t.hold_time_seconds / 60) + 'm' : Math.round(t.hold_time_seconds / 3600) + 'h'}</td>
                                    <td class={pnlClass(t.gross_pnl)}>{fmtPnl(t.gross_pnl)}</td>
                                    <td class={pnlClass(t.net_pnl)}>{fmtPnl(t.net_pnl)}</td>
                                    <td class={pnlClass(t.roi_pct)}>{fmtPct(t.roi_pct)}</td>
                                    <td class={styles.statPositive}>{fmtNum(t.mfe)}</td>
                                    <td class={styles.statNegative}>{fmtNum(t.mae)}</td>
                                    <td>{t.flat_trade ? 'Yes' : ''}</td>
                                </tr>
                            {/each}
                        </tbody>
                    </table>
                {/if}
            {/if}
        {/if}
    </div>
</div>
