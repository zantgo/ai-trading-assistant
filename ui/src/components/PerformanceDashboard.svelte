<script lang="ts">
    // PerformanceDashboard — v7.2 mode-aware rewrite on the shared MME
    // design vocabulary. Personality by the launch session mode:
    //   observe → "Edge Validator" (backtesting only — validate the edge
    //             on recorded decisions before capital is deployed)
    //   paper   → "Backtest + Forward Test" (drift vs the paper record)
    //   live    → "Performance Truth" (drift vs the live record)
    import styles from '../styles/engine-dashboard.module.css';
    import local from './PerformanceDashboard.module.css';
    import { useAppStore } from '../state.svelte';
    import DashboardHeader from './DashboardHeader.svelte';
    import ModeChip from './ModeChip.svelte';
    import ModeBanner from './ModeBanner.svelte';
    import KpiStrip from './KpiStrip.svelte';
    import { isExecutionMode, type ExecutionMode } from '../lib/modePresentation';
    import type {
        StrategyAnalyticsRow, RiskAnalyticsRow, PerformanceMatrixRow,
        OptimizationReport, TradeAnalyticsRecord,
    } from '../types/analytics';

    const app = useAppStore();

    let { section = 'overview' }: { section?: string } = $props();
    let loading = $state(false);
    let errorMsg = $state<string | null>(null);

    let dashboardStats = $state<any>(null);
    let strategyRows = $state<StrategyAnalyticsRow[]>([]);
    let riskData = $state<RiskAnalyticsRow | null>(null);
    let performanceRows = $state<PerformanceMatrixRow[]>([]);
    let optimizationReport = $state<OptimizationReport | null>(null);
    let tradeRecords = $state<TradeAnalyticsRecord[]>([]);

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

    // v7.2: the system-wide launch mode drives PAE framing.
    const mode = $derived<ExecutionMode | undefined>(
        app.sessionMode && isExecutionMode(app.sessionMode) ? app.sessionMode : undefined,
    );
    const observe = $derived(mode === 'observe');

    // Observe collapses to backtesting only — every other surface has no
    // data source without recorded paper/live trades.
    const safeSection = $derived(observe ? 'backtesting' : section);

    const status = $derived<'live' | 'stale' | 'error' | 'loading'>(
        loading ? 'loading' : errorMsg ? 'error' : 'live',
    );

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
            StrongEdge: styles.badgeLong,
            ModerateEdge: styles.badgeLong,
            WeakMarginalEdge: styles.badgeNeutral,
            NoEdgeNegative: styles.badgeError,
            InsufficientData: styles.badgeEmpty,
        };
        return map[c] ?? styles.badgeEmpty;
    }

    function compatibilityBadge(c: string): string {
        const map: Record<string, string> = {
            Strong: local.labelStrong,
            Favorable: local.labelFavorable,
            Marginal: local.labelMarginal,
            Avoid: local.labelAvoid,
        };
        return map[c] ?? local.labelAvoid;
    }

    function sharpeClass(v: number | null): string {
        if (v == null) return local.statNeutral;
        if (v >= 1.0) return local.statPositive;
        if (v >= 0.5) return local.statNeutral;
        return local.statNegative;
    }

    function pnlClass(v: number): string {
        if (v > 0) return local.statPositive;
        if (v < 0) return local.statNegative;
        return local.statNeutral;
    }

    function headerTitle(s: string): string {
        const m: Record<string, string> = {
            overview: 'Performance Overview',
            strategy: 'Strategy Analytics',
            risk: 'Risk Metrics',
            regimes: 'Regime Map',
            trades: 'Trade Analytics',
            backtesting: 'Backtesting',
        };
        return m[s] ?? 'Performance';
    }

    function tabLabel(s: string): string {
        const m: Record<string, string> = {
            overview: 'Overview',
            strategy: 'Strategy',
            risk: 'Risk',
            regimes: 'Regimes',
            trades: 'Trades',
            backtesting: 'Backtesting',
        };
        return m[s] ?? 'Overview';
    }

    // ── Drift card (paper/live): execution record vs historical backtest
    const execRecord = $derived.by(() => {
        const cs = dashboardStats?.core_stats as Record<string, number> | undefined;
        const rd = riskData;
        if (!cs) return null;
        return {
            label: mode === 'live' ? 'Live record' : 'Paper record',
            winRate: cs.win_rate,
            profitFactor: cs.profit_factor,
            expectancy: cs.expectancy,
            maxDd: rd?.maximum_drawdown_pct ?? null,
            trades: cs.total_trades,
        };
    });

    const drift = $derived.by(() => {
        const exec = execRecord;
        const bt = btResult?.summary;
        if (!exec) return null;
        const rows = [
            { name: 'Win Rate', exec: exec.winRate != null ? fmtNum(exec.winRate) + '%' : '—', bt: bt ? fmtNum(bt.win_rate) + '%' : '—' },
            { name: 'Profit Factor', exec: exec.profitFactor != null ? fmtNum(exec.profitFactor) : '—', bt: bt && bt.profit_factor != null ? fmtNum(bt.profit_factor) : '—' },
            { name: 'Expectancy', exec: exec.expectancy != null ? fmtPnl(exec.expectancy) : '—', bt: bt ? fmtPnl(bt.expectancy) : '—' },
            { name: 'Max Drawdown', exec: exec.maxDd != null ? fmtPct(exec.maxDd) : '—', bt: bt ? '-' + fmtNum(bt.max_drawdown_pct) + '%' : '—' },
            { name: 'Trades', exec: exec.trades != null ? String(exec.trades) : '—', bt: bt ? String(bt.total_trades) : '—' },
        ];
        return { rows, trades: exec.trades ?? 0 };
    });
</script>

<div class={styles.dashboard}>
    <div class={styles.content}>
        <DashboardHeader
            title={headerTitle(safeSection)}
            tabLabel={tabLabel(safeSection)}
            {status}
        >
            {#snippet trailing()}
                {#if mode}
                    <ModeChip {mode} />
                {/if}
            {/snippet}
        </DashboardHeader>

        <ModeBanner engine="performance" {mode} />

        {#if observe}
            <div class={styles.card}>
                <h3 class={styles.cardTitle}>Edge Validation</h3>
                <p class={styles.infoLine}>
                    No capital deployed — validate the strategy on recorded decisions first. Run a backtest below;
                    the significance treatment (t-test, 10k Monte Carlo, α = 0.05) tells you whether any edge is real
                    or luck. The other analytics surfaces activate once paper/live trades are recorded.
                </p>
            </div>
        {/if}

        {#if loading}
            <div class={styles.empty}>Loading analytics data…</div>
        {:else if errorMsg && !dashboardStats && safeSection !== 'backtesting'}
            <div class={styles.empty}>{errorMsg}</div>
        {:else}
            {#if safeSection === 'overview'}
                <!-- ── Drift: execution record vs historical backtest ── -->
                {#if !observe}
                    <div class={styles.card}>
                        <h3 class={styles.cardTitle}>Forward Test — {mode === 'live' ? 'Live' : 'Paper'} Record vs Historical Backtest</h3>
                        {#if drift}
                            <table class={styles.table}>
                                <thead>
                                    <tr>
                                        <th>Metric</th>
                                        <th class={styles.tdRight}>{drift.trades > 0 ? `${mode === 'live' ? 'LIVE' : 'PAPER'} RECORD` : 'RECORD'}</th>
                                        <th class={styles.tdRight}>HISTORICAL BACKTEST</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {#each drift.rows as row (row.name)}
                                        <tr>
                                            <td>{row.name}</td>
                                            <td class={styles.tdRight}>{row.exec}</td>
                                            <td class={styles.tdRight}>{row.bt}</td>
                                        </tr>
                                    {/each}
                                </tbody>
                            </table>
                            {#if !btResult}
                                <p class={styles.infoLine} style="margin-top:8px">
                                    Run a backtest on the Backtesting tab to compare the {mode === 'live' ? 'live' : 'paper'} record against historical expectations.
                                </p>
                            {/if}
                        {:else}
                            <div class={styles.empty}>No execution record yet. Closed trades appear here once recorded.</div>
                        {/if}
                    </div>
                {/if}

                <div class={styles.card}>
                    <h3 class={styles.cardTitle}>Performance Overview</h3>
                    <p class={styles.infoLine}>Realized trading performance across all closed trades.</p>
                    {#if dashboardStats?.core_stats}
                        {@const cs = dashboardStats.core_stats}
                        <KpiStrip items={[
                            { label: 'Total P&L', value: fmtPnl(cs.total_pnl), sub: 'net of fees', color: pnlClass(cs.total_pnl) },
                            { label: 'Win Rate', value: fmtNum(cs.win_rate) + '%', sub: `${cs.wins}W / ${cs.losses}L / ${cs.total_trades}T`, color: cs.win_rate >= 50 ? local.statPositive : local.statNegative },
                            { label: 'Profit Factor', value: fmtNum(cs.profit_factor), sub: 'gross win / gross loss', color: cs.profit_factor >= 1.5 ? local.statPositive : pnlClass(cs.profit_factor - 1) },
                            { label: 'Expectancy', value: fmtPnl(cs.expectancy), sub: 'avg per trade', color: pnlClass(cs.expectancy) },
                            { label: 'Avg R:R', value: fmtNum(cs.avg_risk_reward_ratio), sub: 'avg reward multiple', color: cs.avg_risk_reward_ratio >= 1 ? local.statPositive : local.statNeutral },
                            { label: 'Largest Gain', value: '+' + fmtNum(cs.largest_gain), sub: 'best trade', color: local.statPositive },
                            { label: 'Largest Loss', value: fmtNum(cs.largest_loss), sub: 'worst trade', color: local.statNegative },
                            { label: 'Avg Gain / Loss', value: `${fmtNum(cs.avg_gain)} / ${fmtNum(cs.avg_loss)}`, sub: 'per side', color: undefined },
                        ]} />
                    {:else}
                        <div class={styles.empty}>No closed trades yet — performance stats appear once trades are recorded.</div>
                    {/if}
                </div>

                {#if riskData}
                    {@const rd = riskData}
                    <div class={styles.card}>
                        <h3 class={styles.cardTitle}>Risk-Adjusted Metrics</h3>
                        <KpiStrip items={[
                            { label: 'Sharpe Ratio', value: fmtNum(rd.sharpe_ratio), sub: 'annualized', color: sharpeClass(rd.sharpe_ratio) },
                            { label: 'Sortino Ratio', value: fmtNum(rd.sortino_ratio), sub: 'downside only', color: sharpeClass(rd.sortino_ratio) },
                            { label: 'Max Drawdown', value: fmtNum(rd.maximum_drawdown_pct) + '%', sub: 'from peak', color: rd.maximum_drawdown_pct > 20 ? local.statNegative : local.statNeutral },
                            { label: 'Calmar Ratio', value: fmtNum(rd.calmar_ratio), sub: 'return / DD', color: sharpeClass(rd.calmar_ratio) },
                            { label: 'Ulcer Index', value: fmtNum(rd.ulcer_index), sub: 'drawdown depth', color: undefined },
                            { label: 'Daily Volatility', value: fmtNum(rd.daily_volatility * 100) + '%', sub: 'per day', color: undefined },
                            { label: 'VaR 95%', value: fmtNum(rd.value_at_risk_95 * 100) + '%', sub: 'worst daily in 95%', color: undefined },
                            { label: 'Exp. Shortfall 95%', value: fmtNum(rd.expected_shortfall_95 * 100) + '%', sub: 'beyond VaR', color: undefined },
                        ]} />
                    </div>
                {/if}

            {:else if safeSection === 'strategy'}
                <div class={styles.card}>
                    <h3 class={styles.cardTitle}>Strategy Analytics</h3>
                    <p class={styles.infoLine}>Null Hypothesis Significance Testing — determines whether each setup type generates a statistically significant positive edge (H₀: μ ≤ 0 vs H₁: μ > 0). An edge is significant at α = {fmtNum(strategyRows[0]?.alpha ?? 0.05, 2)} when both p-values are below it; p_mc comes from 10,000 Monte Carlo sign-randomization runs.</p>
                    {#if strategyRows.length === 0}
                        <div class={styles.empty}>No strategy data available. Trades must be closed for NHST analysis.</div>
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
                                                <span class={styles.badgeLong} style="margin-left:6px; padding:2px 8px; font-size:10px; border-style:solid">sig @ α={fmtNum(row.alpha, 2)}</span>
                                            {/if}
                                        </td>
                                    </tr>
                                {/each}
                            </tbody>
                        </table>
                    {/if}
                </div>

            {:else if safeSection === 'risk'}
                <div class={styles.card}>
                    <h3 class={styles.cardTitle}>Risk Analytics</h3>
                    <p class={styles.infoLine}>Risk-adjusted return metrics computed from portfolio equity history. Sharpe/Sortino annualized assuming 365 trading days for crypto.</p>
                    {#if !riskData}
                        <div class={styles.empty}>No risk data available. Requires portfolio equity history.</div>
                    {:else}
                        {@const rd = riskData}
                        <KpiStrip items={[
                            { label: 'Sharpe Ratio', value: fmtNum(rd.sharpe_ratio), sub: rd.sharpe_ratio != null && rd.sharpe_ratio > 2 ? 'Excellent' : rd.sharpe_ratio != null && rd.sharpe_ratio >= 1 ? 'Good' : rd.sharpe_ratio != null && rd.sharpe_ratio >= 0.5 ? 'Acceptable' : 'Poor', color: sharpeClass(rd.sharpe_ratio) },
                            { label: 'Sortino Ratio', value: fmtNum(rd.sortino_ratio), sub: 'downside only', color: sharpeClass(rd.sortino_ratio) },
                            { label: 'Calmar Ratio', value: fmtNum(rd.calmar_ratio), sub: 'return / DD', color: sharpeClass(rd.calmar_ratio) },
                            { label: 'Ulcer Index', value: fmtNum(rd.ulcer_index), sub: 'drawdown depth', color: undefined },
                            { label: 'Max Drawdown', value: fmtPct(rd.maximum_drawdown_pct), sub: `${rd.drawdown_count} drawdown events`, color: rd.maximum_drawdown_pct > 20 ? local.statNegative : local.statNeutral },
                            { label: 'Avg Drawdown', value: fmtPct(rd.average_drawdown_pct), sub: 'per event', color: rd.average_drawdown_pct > 10 ? local.statNegative : local.statNeutral },
                            { label: 'Daily Volatility', value: fmtPct(rd.daily_volatility * 100), sub: 'per day', color: undefined },
                            { label: 'Downside Deviation', value: fmtPct(rd.downside_deviation * 100), sub: 'negative moves', color: undefined },
                            { label: 'VaR 95%', value: fmtPct(rd.value_at_risk_95 * 100), sub: 'worst daily in 95%', color: undefined },
                            { label: 'Exp. Shortfall 95%', value: fmtPct(rd.expected_shortfall_95 * 100), sub: 'avg loss beyond VaR', color: undefined },
                        ]} />
                    {/if}
                </div>

            {:else if safeSection === 'regimes'}
                <div class={styles.card}>
                    <h3 class={styles.cardTitle}>Regime-Performance Map</h3>
                    <p class={styles.infoLine}>Strategy performance segmented by market regime at trade entry. Regimes resolved from market_snapshots.</p>
                    {#if performanceRows.length === 0}
                        <div class={styles.empty}>No regime-performance data available. Requires closed trades with market regime context.</div>
                    {:else}
                        <div class={local.regimeGrid}>
                            {#each performanceRows as row}
                                <div class={local.regimeCard}>
                                    <div class={local.regimeName}>
                                        {row.regime}
                                        <span class="{local.regimeLabel} {compatibilityBadge(row.compatibility_label)}" style="float:right">{row.compatibility_label}</span>
                                    </div>
                                    <div class={local.regimeStats}>
                                        <span>{row.trade_count} trades</span>
                                        <span>WR: {fmtNum(row.win_rate)}%</span>
                                        <span>PF: {fmtNum(row.profit_factor)}</span>
                                    </div>
                                    <div class={local.regimeStats}>
                                        <span>Avg R: {fmtNum(row.avg_r_multiple)}</span>
                                        <span class={pnlClass(row.total_pnl)}>P&L: {fmtPnl(row.total_pnl)}</span>
                                    </div>
                                </div>
                            {/each}
                        </div>
                    {/if}

                    {#if optimizationReport?.recommendations?.length}
                        <h3 class={styles.cardTitle} style="margin-top:16px">Recommendations</h3>
                        {#each optimizationReport.recommendations as rec}
                            <div class={local.recommendation}>{rec}</div>
                        {/each}
                    {/if}
                </div>

            {:else if safeSection === 'trades'}
                <div class={styles.card}>
                    <h3 class={styles.cardTitle}>Trade Analytics</h3>
                    <p class={styles.infoLine}>Reconstructed closed trades with execution efficiency metrics.</p>
                    {#if tradeRecords.length === 0}
                        <div class={styles.empty}>No trade data available.</div>
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
                                        <td class={local.statPositive}>{fmtNum(t.mfe)}</td>
                                        <td class={local.statNegative}>{fmtNum(t.mae)}</td>
                                        <td>{t.flat_trade ? 'Yes' : ''}</td>
                                    </tr>
                                {/each}
                            </tbody>
                        </table>
                    {/if}
                </div>

            {:else if safeSection === 'backtesting'}
                <div class={styles.card}>
                    <h3 class={styles.cardTitle}>Strategy Backtesting</h3>
                    <p class={styles.infoLine}>
                        Simulate strategy performance over historical data. Results replay the recorded MME decisions through the setup executor (paper engine) and apply the full significance treatment.
                    </p>

                    <div style="display:flex; gap:12px; flex-wrap:wrap; margin-bottom:16px">
                        <div style="display:flex; flex-direction:column; gap:4px">
                            <label for="bt-symbol" style="font-size:10px; color:rgba(255,255,255,0.45); text-transform:uppercase">Symbol</label>
                            <select id="bt-symbol" bind:value={btSymbol} class={styles.select}>
                                {#each btSymbols as s (s)}
                                    <option value={s}>{s}</option>
                                {/each}
                            </select>
                        </div>
                        <div style="display:flex; flex-direction:column; gap:4px">
                            <label for="bt-tf" style="font-size:10px; color:rgba(255,255,255,0.45); text-transform:uppercase">Timeframe</label>
                            <select id="bt-tf" bind:value={btTimeframe} class={styles.select}>
                                <option value={60}>1m</option>
                                <option value={300}>5m</option>
                                <option value={900}>15m</option>
                                <option value={3600}>1h</option>
                            </select>
                        </div>
                        <div style="display:flex; flex-direction:column; gap:4px">
                            <label for="bt-start" style="font-size:10px; color:rgba(255,255,255,0.45); text-transform:uppercase">Start Date</label>
                            <input id="bt-start" type="date" bind:value={btStartDate} class={styles.select} />
                        </div>
                        <div style="display:flex; flex-direction:column; gap:4px">
                            <label for="bt-end" style="font-size:10px; color:rgba(255,255,255,0.45); text-transform:uppercase">End Date</label>
                            <input id="bt-end" type="date" bind:value={btEndDate} class={styles.select} />
                        </div>
                        <div style="display:flex; flex-direction:column; gap:4px">
                            <label for="bt-capital" style="font-size:10px; color:rgba(255,255,255,0.45); text-transform:uppercase">Capital ($)</label>
                            <input id="bt-capital" type="number" bind:value={btCapital} min="100" step="1000" class={styles.select} style="width:100px" />
                        </div>
                        <div style="display:flex; align-items:flex-end">
                            <button class="{styles.btn} {styles.btnPrimary}" onclick={runBacktest} disabled={btRunning}>
                                {btRunning ? 'Running...' : 'Run Backtest'}
                            </button>
                        </div>
                    </div>

                    {#if btError}
                        <div class="{styles.alertBanner} {styles.alertError}" style="margin-bottom:12px">{btError}</div>
                    {/if}

                    {#if !btResult}
                        <div class={styles.empty} style="margin-top:12px">
                            Choose a symbol + timeframe and date range, then run the backtest. Results replay the recorded MME decisions through the setup executor (paper only) and apply the full significance treatment.
                        </div>
                    {:else}
                        {@const s = btResult.summary}
                        {@const st = btResult.stats}
                        <h3 class={styles.cardTitle} style="margin-top:16px">Results — {btSymbol} · {btTimeframe}s</h3>
                        <p class={styles.infoLine}>{btStartDate} → {btEndDate} · Capital: ${btCapital.toLocaleString()} · backtest #{btResult.backtest_id}</p>

                        <KpiStrip items={[
                            { label: 'Total Trades', value: String(s.total_trades), sub: 'simulated', color: undefined },
                            { label: 'Win Rate', value: fmtNum(s.win_rate) + '%', sub: `${s.win_count}W / ${s.loss_count}L`, color: s.win_rate >= 50 ? local.statPositive : local.statNegative },
                            { label: 'Profit Factor', value: fmtNum(s.profit_factor), sub: 'gross win / loss', color: s.profit_factor != null && s.profit_factor >= 1 ? local.statPositive : local.statNegative },
                            { label: 'Net P&L', value: fmtPnl(s.gross_profit - s.gross_loss), sub: `gross ${fmtPnl(s.gross_profit)} / ${fmtPnl(-s.gross_loss)}`, color: s.gross_profit - s.gross_loss >= 0 ? local.statPositive : local.statNegative },
                            { label: 'Max Drawdown', value: '-' + fmtNum(s.max_drawdown_pct) + '%', sub: 'from peak', color: local.statNeutral },
                            { label: 'Expectancy', value: fmtPnl(s.expectancy), sub: 'avg per trade', color: pnlClass(s.expectancy) },
                        ]} />

                        <div class="{local.edgeCard} {st.is_significant ? local.edgeSig : local.edgeNo}">
                            <span class={local.edgeLabel}>EDGE VERDICT</span>
                            <span class={local.edgeTitle}>{st.classification.replace(/([A-Z])/g, ' $1').trim()}</span>
                            <span class={local.edgeDetail}>
                                {#if st.total_trades < 30}
                                    insufficient data — need at least 30 simulated trades for a verdict
                                {:else if st.is_significant}
                                    statistically significant at α = {fmtNum(st.alpha, 2)} — t-test p = {fmtNum(st.p_value, 4)}, Monte Carlo p = {fmtNum(st.p_mc, 4)} ({st.monte_carlo_runs.toLocaleString()} runs)
                                {:else}
                                    not significant at α = {fmtNum(st.alpha, 2)} — t-test p = {fmtNum(st.p_value, 4)}, Monte Carlo p = {fmtNum(st.p_mc, 4)} ({st.monte_carlo_runs.toLocaleString()} runs) — this result could be luck
                                {/if}
                            </span>
                        </div>

                        <h3 class={styles.cardTitle} style="margin-top:16px">Equity Curve</h3>
                        {#if btResult.equity_curve.length >= 2}
                            <svg viewBox="0 0 640 200" width="100%" height="200" style="background:#0b0d12; border:1px solid rgba(255,255,255,0.08); border-radius:6px">
                                <polyline points={equityPath(btResult.equity_curve)} fill="none" stroke="#22c55e" stroke-width="1.5" />
                            </svg>
                        {:else}
                            <div class={styles.empty} style="height:120px; display:flex; align-items:center; justify-content:center">Not enough data for an equity curve.</div>
                        {/if}

                        <h3 class={styles.cardTitle} style="margin-top:16px">Trade Log</h3>
                        {#if btResult.trades.length === 0}
                            <div class={styles.empty}>No trades in this window.</div>
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
                                            <td style="color:{tr.direction === 'LONG' ? '#22c55e' : '#ef4444'}">{tr.direction}</td>
                                            <td>${fmtNum(tr.entry_price)}</td>
                                            <td>${fmtNum(tr.exit_price)}</td>
                                            <td>{fmtNum(tr.size, 4)}</td>
                                            <td style="color:{tr.pnl >= 0 ? '#22c55e' : '#ef4444'}">{fmtPnl(tr.pnl)}</td>
                                            <td>{tr.exit_reason}</td>
                                        </tr>
                                    {/each}
                                </tbody>
                            </table>
                        {/if}
                    {/if}
                </div>
            {/if}
        {/if}
    </div>
</div>
