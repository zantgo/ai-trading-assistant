<script lang="ts">
    import styles from './PerformanceDashboard.module.css';
    import { useAppStore } from '../state.svelte';
    import SvgIcon from '../lib/SvgIcon.svelte';
    import type {
        StrategyAnalyticsRow, RiskAnalyticsRow, PerformanceMatrixRow,
        OptimizationReport, TradeAnalyticsRecord,
    } from '../types/analytics';

    const app = useAppStore();

    type Panel = 'overview' | 'strategy' | 'risk' | 'regimes' | 'trades' | 'backtesting';

    let activePanel = $state<Panel>('overview');
    let loading = $state(false);

    let dashboardStats = $state<any>(null);
    let strategyRows = $state<StrategyAnalyticsRow[]>([]);
    let riskData = $state<RiskAnalyticsRow | null>(null);
    let performanceRows = $state<PerformanceMatrixRow[]>([]);
    let optimizationReport = $state<OptimizationReport | null>(null);
    let tradeRecords = $state<TradeAnalyticsRecord[]>([]);
    let errorMsg = $state<string | null>(null);

    // ── Backtesting state (v7: live /api/backtest/run) ────────────────
    const btSymbols = $derived(Object.keys(app.instancesMap).length > 0 ? Object.keys(app.instancesMap) : ['BTC-USDC']);
    let btSymbol = $state('BTC-USDC');
    let btTimeframe = $state(60);
    let btStartDate = $state(new Date(Date.now() - 30 * 864e5).toISOString().slice(0, 10));
    let btEndDate = $state(new Date().toISOString().slice(0, 10));
    let btCapital = $state(1000);
    let btRunning = $state(false);
    let btError = $state('');
    let btResult = $state<{
        backtest_id: number;
        summary: { total_trades: number; win_count: number; loss_count: number; win_rate: number; gross_profit: number; gross_loss: number; profit_factor: number | null; expectancy: number; max_drawdown_pct: number };
        stats: StrategyAnalyticsRow;
        trades: { timestamp: number; direction: string; entry_price: number; exit_price: number; size: number; pnl: number; exit_reason: string }[];
        equity_curve: [number, number][];
    } | null>(null);

    async function runBacktest() {
        btRunning = true;
        btError = '';
        btResult = null;
        try {
            const fromMs = Date.parse(btStartDate);
            const toMs = Date.parse(btEndDate) + 864e5 - 1;
            if (!isFinite(fromMs) || !isFinite(toMs)) throw new Error('Invalid date range');
            const res = await fetch('/api/backtest/run', {
                method: 'POST',
                headers: { 'content-type': 'application/json' },
                body: JSON.stringify({
                    symbol: btSymbol,
                    timeframe_secs: Number(btTimeframe),
                    from_ms: fromMs,
                    to_ms: toMs,
                    initial_capital: Number(btCapital),
                }),
            });
            if (!res.ok) throw new Error('Backtest failed: HTTP ' + res.status);
            btResult = await res.json();
        } catch (e: any) {
            btError = e?.message ?? 'Backtest failed';
        } finally {
            btRunning = false;
        }
    }

    function equityPath(points: [number, number][]): string {
        if (!points || points.length < 2) return '';
        const xs = points.map((p) => p[0]);
        const ys = points.map((p) => p[1]);
        const minX = Math.min(...xs), maxX = Math.max(...xs);
        const minY = Math.min(...ys), maxY = Math.max(...ys);
        const w = 600, h = 180;
        const spanX = maxX - minX || 1, spanY = maxY - minY || 1;
        return points.map(([x, y], i) => {
            const px = ((x - minX) / spanX) * w + 20;
            const py = h - ((y - minY) / spanY) * h + 10;
            return (i === 0 ? 'M' : 'L') + px.toFixed(1) + ',' + py.toFixed(1);
        }).join(' ');
    }

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
        <button class="{styles.sidebarBtn} {activePanel === 'overview' ? styles.sidebarBtnActive : ''}" onclick={() => activePanel = 'overview'}>Overview</button>
        <button class="{styles.sidebarBtn} {activePanel === 'strategy' ? styles.sidebarBtnActive : ''}" onclick={() => activePanel = 'strategy'}>Strategy</button>
        <button class="{styles.sidebarBtn} {activePanel === 'risk' ? styles.sidebarBtnActive : ''}" onclick={() => activePanel = 'risk'}>Risk Metrics</button>
        <button class="{styles.sidebarBtn} {activePanel === 'regimes' ? styles.sidebarBtnActive : ''}" onclick={() => activePanel = 'regimes'}>Regime Map</button>
        <button class="{styles.sidebarBtn} {activePanel === 'trades' ? styles.sidebarBtnActive : ''}" onclick={() => activePanel = 'trades'}>Trade Analytics</button>
        <button class="{styles.sidebarBtn} {activePanel === 'backtesting' ? styles.sidebarBtnActive : ''}" onclick={() => activePanel = 'backtesting'}>Backtesting</button>
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
                <h3 class={styles.sectionTitle}>Strategy Analytics</h3>
                <p class={styles.sectionDesc}>Null Hypothesis Significance Testing — determines whether each setup type generates a statistically significant positive edge (H₀: μ ≤ 0 vs H₁: μ > 0). An edge is significant at α = {fmtNum(strategyRows[0]?.alpha ?? 0.05, 2)} when both p-values are below it; p_mc comes from 10,000 Monte Carlo sign-randomization runs.</p>
                {#if strategyRows.length === 0}
                    <div class={styles.equityPlaceholder}>No strategy data available. Trades must be closed for NHST analysis.</div>
                {:else}
                    <table class={styles.table}>
                        <thead>
                            <tr>
                                <th>Setup Type</th>
                                <th>Trades</th>
                                <th>Win Rate</th>
                                <th>Profit Factor</th>
                                <th>Expectancy</th>
                                <th>T-Stat</th>
                                <th>P-Value</th>
                                <th>P_MC (10k)</th>
                                <th>Edge</th>
                            </tr>
                        </thead>
                        <tbody>
                            {#each strategyRows as row}
                                <tr>
                                    <td>{row.setup_type}</td>
                                    <td>{row.total_trades}</td>
                                    <td class={pnlClass(row.win_rate - 50)}>{fmtNum(row.win_rate)}%</td>
                                    <td>{fmtNum(row.profit_factor)}</td>
                                    <td class={pnlClass(row.expectancy)}>{fmtPnl(row.expectancy)}</td>
                                    <td>{fmtNum(row.t_statistic)}</td>
                                    <td>{fmtNum(row.p_value, 4)}</td>
                                    <td>{fmtNum(row.p_mc, 4)}</td>
                                    <td>
                                        <span class="{styles.badge} {classificationBadge(row.classification)}">{row.classification.replace(/([A-Z])/g, ' $1').trim()}</span>
                                        {#if row.is_significant}
                                            <span class={styles.badgeSig}>sig @ α={fmtNum(row.alpha, 2)}</span>
                                        {/if}
                                    </td>
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
            {:else if activePanel === 'backtesting'}
                <h3 class={styles.sectionTitle}>Strategy Backtesting</h3>
                <p class={styles.sectionDesc}>
                    Simulate strategy performance over historical data. Configure parameters and run
                    to evaluate edge, drawdown, and risk-adjusted returns on past market conditions.
                </p>

                <div style="display:flex; gap:0.75rem; flex-wrap:wrap; margin-bottom:1.25rem">
                    <div style="display:flex; flex-direction:column; gap:0.25rem">
                        <label for="bt-symbol" style="font-size:0.7rem; color:#5a5f6e; text-transform:uppercase">Symbol</label>
                        <select id="bt-symbol" bind:value={btSymbol} style="padding:0.4rem 0.6rem; background:#080808; border:1px solid #2a2e39; border-radius:4px; color:#ccc; font-family:var(--mono); font-size:0.78rem">
                            {#each btSymbols as s (s)}
                                <option value={s}>{s}</option>
                            {/each}
                        </select>
                    </div>
                    <div style="display:flex; flex-direction:column; gap:0.25rem">
                        <label for="bt-tf" style="font-size:0.7rem; color:#5a5f6e; text-transform:uppercase">Timeframe</label>
                        <select id="bt-tf" bind:value={btTimeframe} style="padding:0.4rem 0.6rem; background:#080808; border:1px solid #2a2e39; border-radius:4px; color:#ccc; font-family:var(--mono); font-size:0.78rem">
                            <option value={60}>1m</option>
                            <option value={300}>5m</option>
                            <option value={900}>15m</option>
                            <option value={3600}>1h</option>
                        </select>
                    </div>
                    <div style="display:flex; flex-direction:column; gap:0.25rem">
                        <label for="bt-start" style="font-size:0.7rem; color:#5a5f6e; text-transform:uppercase">Start Date</label>
                        <input id="bt-start" type="date" bind:value={btStartDate} style="padding:0.4rem 0.6rem; background:#080808; border:1px solid #2a2e39; border-radius:4px; color:#ccc; font-family:var(--mono); font-size:0.78rem" />
                    </div>
                    <div style="display:flex; flex-direction:column; gap:0.25rem">
                        <label for="bt-end" style="font-size:0.7rem; color:#5a5f6e; text-transform:uppercase">End Date</label>
                        <input id="bt-end" type="date" bind:value={btEndDate} style="padding:0.4rem 0.6rem; background:#080808; border:1px solid #2a2e39; border-radius:4px; color:#ccc; font-family:var(--mono); font-size:0.78rem" />
                    </div>
                    <div style="display:flex; flex-direction:column; gap:0.25rem">
                        <label for="bt-capital" style="font-size:0.7rem; color:#5a5f6e; text-transform:uppercase">Capital ($)</label>
                        <input id="bt-capital" type="number" bind:value={btCapital} min="100" step="1000" style="padding:0.4rem 0.6rem; background:#080808; border:1px solid #2a2e39; border-radius:4px; color:#ccc; font-family:var(--mono); font-size:0.78rem; width:100px" />
                    </div>
                    <div style="display:flex; align-items:flex-end">
                        <button onclick={runBacktest} disabled={btRunning}
                            style="padding:0.45rem 1rem; background:#fff; border:none; border-radius:4px; color:#000; cursor:pointer; font-family:var(--mono); font-size:0.78rem; text-transform:uppercase; letter-spacing:0.05em; transition:opacity 0.15s; opacity:{btRunning ? '0.5' : '1'}; font-weight:700">
                            {btRunning ? 'Running...' : 'Run Backtest'}
                        </button>
                    </div>
                </div>

                {#if btError}
                    <div style="margin-top:1rem; padding:0.6rem; background:#2a0d0d; border:1px solid #5c1f1f; border-radius:6px; color:#ff7b7b; font-size:0.78rem">{btError}</div>
                {/if}

                {#if !btResult}
                    <div class={styles.equityPlaceholder} style="margin-top:1rem">
                        Choose a symbol + timeframe and date range, then run the backtest. Results replay the recorded MME decisions through the setup executor (paper only) and apply the full significance treatment.
                    </div>
                {:else}
                    {@const s = btResult.summary}
                    {@const st = btResult.stats}
                    <h3 class={styles.sectionTitle} style="margin-top:1.25rem">Results — {btSymbol} · {btTimeframe}s</h3>
                    <p class={styles.sectionDesc}>{btStartDate} → {btEndDate} · Capital: ${btCapital.toLocaleString()} · backtest #{btResult.backtest_id}</p>

                    <div class={styles.statsGrid}>
                        <div class={styles.statCard}>
                            <div class={styles.statLabel}>Total Trades</div>
                            <div class={styles.statValue}>{s.total_trades}</div>
                        </div>
                        <div class={styles.statCard}>
                            <div class={styles.statLabel}>Win Rate</div>
                            <div class={styles.statValue} style="color:{s.win_rate >= 50 ? '#4caf50' : '#ef5350'}">{fmtNum(s.win_rate)}%</div>
                            <div class={styles.statSub}>{s.win_count}W / {s.loss_count}L</div>
                        </div>
                        <div class={styles.statCard}>
                            <div class={styles.statLabel}>Profit Factor</div>
                            <div class={styles.statValue} style="color:{s.profit_factor != null && s.profit_factor >= 1 ? '#4caf50' : '#ef5350'}">{fmtNum(s.profit_factor)}</div>
                        </div>
                        <div class={styles.statCard}>
                            <div class={styles.statLabel}>Net P&L</div>
                            <div class={styles.statValue} style="color:{s.gross_profit - s.gross_loss >= 0 ? '#4caf50' : '#ef5350'}">{fmtPnl(s.gross_profit - s.gross_loss)}</div>
                            <div class={styles.statSub}>gross {fmtPnl(s.gross_profit)} / {fmtPnl(-s.gross_loss)}</div>
                        </div>
                        <div class={styles.statCard}>
                            <div class={styles.statLabel}>Max Drawdown</div>
                            <div class={styles.statValue} style="color:#ffb74d">-{fmtNum(s.max_drawdown_pct)}%</div>
                        </div>
                        <div class={styles.statCard}>
                            <div class={styles.statLabel}>Expectancy</div>
                            <div class="{styles.statValue} {pnlClass(s.expectancy)}">{fmtPnl(s.expectancy)}</div>
                        </div>
                    </div>

                    <div class="{styles.edgeCard} {st.is_significant ? styles.edgeSig : styles.edgeNo}">
                        <span class={styles.edgeLabel}>EDGE VERDICT</span>
                        <span class={styles.edgeTitle}>{st.classification.replace(/([A-Z])/g, ' $1').trim()}</span>
                        <span class={styles.edgeDetail}>
                            {#if st.total_trades < 30}
                                insufficient data — need at least 30 simulated trades for a verdict
                            {:else if st.is_significant}
                                statistically significant at α = {fmtNum(st.alpha, 2)} — t-test p = {fmtNum(st.p_value, 4)}, Monte Carlo p = {fmtNum(st.p_mc, 4)} ({st.monte_carlo_runs.toLocaleString()} runs)
                            {:else}
                                not significant at α = {fmtNum(st.alpha, 2)} — t-test p = {fmtNum(st.p_value, 4)}, Monte Carlo p = {fmtNum(st.p_mc, 4)} ({st.monte_carlo_runs.toLocaleString()} runs) — this result could be luck
                            {/if}
                        </span>
                    </div>

                    <h3 class={styles.sectionTitle} style="margin-top:1.5rem">Equity Curve</h3>
                    {#if btResult.equity_curve.length >= 2}
                        <svg viewBox="0 0 640 200" width="100%" height="200" style="background:#0b0d12; border:1px solid #23262e; border-radius:6px">
                            <polyline points={equityPath(btResult.equity_curve)} fill="none" stroke="#4caf50" stroke-width="1.5" />
                        </svg>
                    {:else}
                        <div class={styles.equityPlaceholder} style="height:120px; display:flex; align-items:center; justify-content:center; color:#5a5f6e">Not enough data for an equity curve.</div>
                    {/if}

                    <h3 class={styles.sectionTitle} style="margin-top:1.5rem">Trade Log</h3>
                    {#if btResult.trades.length === 0}
                        <div class={styles.equityPlaceholder}>No trades in this window.</div>
                    {:else}
                        <table class={styles.table}>
                            <thead>
                                <tr>
                                    <th>Time</th>
                                    <th>Dir</th>
                                    <th>Entry</th>
                                    <th>Exit</th>
                                    <th>Size</th>
                                    <th>P&L</th>
                                    <th>Exit Reason</th>
                                </tr>
                            </thead>
                            <tbody>
                                {#each btResult.trades as tr, i (i)}
                                    <tr>
                                        <td>{new Date(tr.timestamp).toLocaleString()}</td>
                                        <td style="color:{tr.direction === 'LONG' ? '#4caf50' : '#ef5350'}">{tr.direction}</td>
                                        <td style="font-variant-numeric:tabular-nums">${fmtNum(tr.entry_price)}</td>
                                        <td style="font-variant-numeric:tabular-nums">${fmtNum(tr.exit_price)}</td>
                                        <td>{fmtNum(tr.size, 4)}</td>
                                        <td style="color:{tr.pnl >= 0 ? '#4caf50' : '#ef5350'}">{fmtPnl(tr.pnl)}</td>
                                        <td>{tr.exit_reason}</td>
                                    </tr>
                                {/each}
                            </tbody>
                        </table>
                    {/if}
                {/if}
            {/if}
        {/if}
    </div>
</div>
