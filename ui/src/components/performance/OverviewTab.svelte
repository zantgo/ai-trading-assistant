<script lang="ts">
    // PAE Overview tab — "Edge Validator" in observe mode (data coverage +
    // latest verdict + significance summary) and the forward-test drift +
    // performance overview in paper/live.
    import { onMount } from 'svelte';
    import KpiStrip from './../KpiStrip.svelte';
    import styles from '../../styles/engine-dashboard.module.css';
    import local from '../PerformanceDashboard.module.css';
    import { fmtNum, fmtPct, fmtSigned } from '../../lib/format';
    import { linePath } from '../../lib/studyCharts';
    import type { ExecutionMode } from '../../lib/modePresentation';
    import type { RiskAnalyticsRow, StrategyAnalyticsRow } from '../../types/analytics';

    interface Props {
        mode: ExecutionMode | undefined;
        observe: boolean;
        dashboardStats: any;
        riskData: RiskAnalyticsRow | null;
        btResult: {
            backtest_id: number;
            summary: { total_trades: number; win_count: number; loss_count: number; win_rate: number; gross_profit: number; gross_loss: number; profit_factor: number | null; expectancy: number; max_drawdown_pct: number };
            stats: StrategyAnalyticsRow;
        } | null;
    }

    let { mode, observe, dashboardStats, riskData, btResult }: Props = $props();

    let coverage: { symbol: string; timeframe_secs: number; snapshot_count: number; earliest_secs: number; latest_secs: number }[] = $state([]);
    interface AnalyticsCfg { alpha?: number; monte_carlo_runs?: number; min_trades_for_verdict?: number }
    let analyticsCfg = $state<AnalyticsCfg>({});
    let latestRun: { id: number; created_at: number; params: any; summary: any } | null = $state(null);

    onMount(() => {
        void fetch('/api/backtest/coverage')
            .then((r) => (r.ok ? r.json() : {}))
            .then((d: any) => {
                // v8 BTE shape: { snapshots, archive, ... }.
                const arr = Array.isArray(d?.snapshots) ? d.snapshots : Array.isArray(d) ? d : [];
                coverage = arr;
            })
            .catch(() => {});
        void fetch('/api/backtest/list?limit=1')
            .then((r) => (r.ok ? r.json() : []))
            .then((d: unknown) => {
                const arr = Array.isArray(d) ? d : [];
                if (arr.length > 0) {
                    const row = arr[0] as { id: number; created_at: number; params?: unknown; summary?: unknown };
                    latestRun = {
                        id: row.id,
                        created_at: row.created_at,
                        params: row.params ?? {},
                        summary: row.summary ?? {},
                    };
                }
            })
            .catch(() => {});
        void fetch('/api/config')
            .then((r) => (r.ok ? r.json() : {}))
            .then((c: unknown) => {
                const a = (c as { analytics?: AnalyticsCfg })?.analytics;
                if (a) analyticsCfg = a;
            })
            .catch(() => {});
    });

    // Verdict derived from the persisted summary headline numbers (the
    // History tab uses the same rule).
    function latestVerdict(): string {
        const s = latestRun?.summary ?? {};
        const trades = s.total_trades ?? 0;
        if (trades < 30) return 'InsufficientData';
        const pf = s.profit_factor ?? 0;
        const wr = s.win_rate ?? 0;
        if (pf > 1.2 && wr > 50) return 'StrongEdge';
        if (pf >= 1.0) return 'WeakMarginalEdge';
        return 'NoEdgeNegative';
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

    function sharpeClass(v: number | null | undefined): string {
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

    // ── Drift card (paper/live): execution record vs historical backtest
    const drift = $derived.by(() => {
        const cs = dashboardStats?.core_stats as Record<string, number> | undefined;
        const rd = riskData;
        if (!cs) return null;
        const exec = {
            winRate: cs.win_rate,
            profitFactor: cs.profit_factor,
            expectancy: cs.expectancy,
            maxDd: rd?.maximum_drawdown_pct ?? null,
            trades: cs.total_trades,
        };
        const bt = btResult?.summary;
        const rows = [
            { name: 'Win Rate', exec: exec.winRate != null ? fmtNum(exec.winRate) + '%' : '—', bt: bt ? fmtNum(bt.win_rate) + '%' : '—' },
            { name: 'Profit Factor', exec: exec.profitFactor != null ? fmtNum(exec.profitFactor) : '—', bt: bt && bt.profit_factor != null ? fmtNum(bt.profit_factor) : '—' },
            { name: 'Expectancy', exec: exec.expectancy != null ? fmtSigned(exec.expectancy) : '—', bt: bt ? fmtSigned(bt.expectancy) : '—' },
            { name: 'Max Drawdown', exec: exec.maxDd != null ? fmtPct(exec.maxDd) : '—', bt: bt ? '-' + fmtNum(bt.max_drawdown_pct) + '%' : '—' },
            { name: 'Trades', exec: exec.trades != null ? String(exec.trades) : '—', bt: bt ? String(bt.total_trades) : '—' },
        ];
        return { rows, trades: exec.trades ?? 0 };
    });
</script>

{#if observe}
    <div class={styles.card}>
        <h3 class={styles.cardTitle}>Edge Validation</h3>
        <p class={styles.infoLine}>
            No capital deployed — validate the strategy on recorded decisions first. Run a backtest below;
            the significance treatment (t-test, {analyticsCfg.monte_carlo_runs ?? '10,000'} Monte Carlo runs,
            α = {analyticsCfg.alpha ?? 0.05}) tells you whether any edge is real or luck. The other
            analytics surfaces activate once paper/live trades are recorded.
        </p>
    </div>

    <div class={styles.card}>
        <h3 class={styles.cardTitle}>Data Coverage — Recorded Decisions</h3>
        <p class={styles.infoLine}>
            The backtest replays recorded MME decisions from the snapshot store. These rows show how much
            replayable data exists per symbol × timeframe.
        </p>
        {#if coverage.length === 0}
            <div class={styles.empty}>No recorded snapshots yet — backtests need recorded completed candles with decision matrices.</div>
        {:else}
            <table class={styles.table}>
                <thead><tr><th>Symbol</th><th class={styles.tdRight}>TF</th><th class={styles.tdRight}>Snapshots</th><th class={styles.tdRight}>Earliest</th><th class={styles.tdRight}>Latest</th></tr></thead>
                <tbody>
                    {#each coverage as c (c.symbol + c.timeframe_secs)}
                        <tr>
                            <td class={styles.tdMono}>{c.symbol}</td>
                            <td class={styles.tdRight}>{c.timeframe_secs}s</td>
                            <td class={styles.tdRight}>{c.snapshot_count.toLocaleString()}</td>
                            <td class={styles.tdRight}>{new Date(c.earliest_secs * 1000).toLocaleDateString()}</td>
                            <td class={styles.tdRight}>{new Date(c.latest_secs * 1000).toLocaleDateString()}</td>
                        </tr>
                    {/each}
                </tbody>
            </table>
        {/if}
    </div>

    <div class={styles.card}>
        <h3 class={styles.cardTitle}>Latest Backtest Verdict</h3>
        {#if latestRun}
            {@const verdict = latestVerdict()}
            <div class={styles.inlineGroup}>
                <span class={styles.badgeNeutral} style="padding:3px 10px; border-radius:6px; border:1px solid">RUN #{latestRun.id}</span>
                <span class="{styles.badge} {classificationBadge(verdict)}">
                    {verdict.replace(/([A-Z])/g, ' $1').trim()}
                </span>
                <span class={styles.metaChip}><span class={styles.metaChipLabel}>trades</span><span class={styles.metaChipValue}>{latestRun.summary?.total_trades ?? '—'}</span></span>
                <span class={styles.metaChip}><span class={styles.metaChipLabel}>win rate</span><span class={styles.metaChipValue}>{latestRun.summary?.win_rate != null ? fmtNum(latestRun.summary.win_rate) + '%' : '—'}</span></span>
            </div>
            <p class={styles.infoLine} style="margin-top:8px">
                {latestRun.params?.symbol ?? '—'} · {latestRun.params?.timeframe_secs ?? '—'}s ·
                {latestRun.params?.from_ms ? new Date(latestRun.params.from_ms).toLocaleDateString() : '—'} → {latestRun.params?.to_ms ? new Date(latestRun.params.to_ms).toLocaleDateString() : '—'} ·
                run {new Date(latestRun.created_at).toLocaleString()}
            </p>
        {:else}
            <div class={styles.empty}>No backtest has been run yet — run one on the Backtesting tab.</div>
        {/if}
    </div>

    <div class={styles.card}>
        <h3 class={styles.cardTitle}>Significance Summary</h3>
        <KpiStrip items={[
            { label: 'Alpha (α)', value: fmtNum(analyticsCfg.alpha ?? 0.05, 2), sub: 'significance level' },
            { label: 'Monte Carlo Runs', value: (analyticsCfg.monte_carlo_runs ?? 10000).toLocaleString(), sub: 'sign randomization' },
            { label: 'Min Trades', value: String(analyticsCfg.min_trades_for_verdict ?? 30), sub: 'verdict floor' },
        ]} />
    </div>
{:else}
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
        {:else}
            <div class={styles.empty}>No execution record yet. Closed trades appear here once recorded.</div>
        {/if}
        {#if drift && !btResult}
            <p class={styles.infoLine} style="margin-top:8px">
                Run a backtest on the Backtesting tab to compare the {mode === 'live' ? 'live' : 'paper'} record against historical expectations.
            </p>
        {/if}
    </div>

    <div class={styles.card}>
        <h3 class={styles.cardTitle}>Performance Overview</h3>
        <p class={styles.infoLine}>Realized trading performance across all closed trades.</p>
        {#if dashboardStats?.core_stats}
            {@const cs = dashboardStats.core_stats}
            <KpiStrip items={[
                { label: 'Total P&L', value: fmtSigned(cs.total_pnl), sub: 'net of fees', color: pnlClass(cs.total_pnl) },
                { label: 'Win Rate', value: fmtNum(cs.win_rate) + '%', sub: `${cs.wins}W / ${cs.losses}L / ${cs.total_trades}T`, color: cs.win_rate >= 50 ? local.statPositive : local.statNegative },
                { label: 'Profit Factor', value: fmtNum(cs.profit_factor), sub: 'gross win / gross loss', color: cs.profit_factor >= 1.5 ? local.statPositive : pnlClass(cs.profit_factor - 1) },
                { label: 'Expectancy', value: fmtSigned(cs.expectancy), sub: 'avg per trade', color: pnlClass(cs.expectancy) },
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
                { label: 'Log-Return Sharpe', value: fmtNum(rd.sharpe_ratio_log), sub: 'log daily returns', color: sharpeClass(rd.sharpe_ratio_log) },
            ]} />
        </div>
    {/if}

    {@const symmetry = dashboardStats?.direction_symmetry as {
        long_count: number; short_count: number; long_expectancy_usd: number;
        short_expectancy_usd: number; long_win_rate: number; short_win_rate: number;
        t_statistic: number; degrees_of_freedom: number; p_value: number;
        significant: boolean; verdict: string;
    } | null | undefined}
    <div class={styles.card}>
        <h3 class={styles.cardTitle}>Long / Short Symmetry</h3>
        {#if symmetry}
            <p class={styles.infoLine}>
                Welch two-sample t-test on per-trade returns — H0: longs and shorts are statistically equal.
            </p>
            <table class={styles.table}>
                <thead>
                    <tr><th>Direction</th><th class={styles.tdRight}>Trades</th><th class={styles.tdRight}>Expectancy</th><th class={styles.tdRight}>Win Rate</th></tr>
                </thead>
                <tbody>
                    <tr>
                        <td>LONG</td>
                        <td class={styles.tdRight}>{symmetry.long_count}</td>
                        <td class={styles.tdRight}>{fmtSigned(symmetry.long_expectancy_usd)}</td>
                        <td class={styles.tdRight}>{fmtNum(symmetry.long_win_rate)}%</td>
                    </tr>
                    <tr>
                        <td>SHORT</td>
                        <td class={styles.tdRight}>{symmetry.short_count}</td>
                        <td class={styles.tdRight}>{fmtSigned(symmetry.short_expectancy_usd)}</td>
                        <td class={styles.tdRight}>{fmtNum(symmetry.short_win_rate)}%</td>
                    </tr>
                </tbody>
            </table>
            <div class={styles.inlineGroup} style="margin-top:8px">
                <span class={styles.metaChip}><span class={styles.metaChipLabel}>t</span><span class={styles.metaChipValue}>{fmtNum(symmetry.t_statistic, 2)}</span></span>
                <span class={styles.metaChip}><span class={styles.metaChipLabel}>df</span><span class={styles.metaChipValue}>{fmtNum(symmetry.degrees_of_freedom, 1)}</span></span>
                <span class={styles.metaChip}><span class={styles.metaChipLabel}>p</span><span class={styles.metaChipValue}>{fmtNum(symmetry.p_value, 4)}</span></span>
                <span class="{styles.badge} {symmetry.verdict === 'SYMMETRIC' || !symmetry.significant ? styles.badgeNeutral : symmetry.verdict === 'LONG_BETTER' ? styles.badgeLong : styles.badgeError}">
                    {symmetry.verdict.replaceAll('_', ' ')}
                </span>
            </div>
        {:else}
            <div class={styles.empty}>Insufficient data — the symmetry test needs at least 10 closed trades per direction.</div>
        {/if}
    </div>

    {@const compounded = (dashboardStats?.compounded_curve ?? []) as [number, number][]}
    {#if compounded.length >= 2}
        {@const eq = linePath(compounded, 600, 160)}
        <div class={styles.card}>
            <h3 class={styles.cardTitle}>Live Equity Curve</h3>
            <svg viewBox="0 0 640 180" width="100%" height="180" style="background:#0b0d12; border:1px solid rgba(255,255,255,0.08); border-radius:6px" role="img" aria-label="Live equity curve">
                <line x1="10" y1="170" x2="630" y2="170" stroke="rgba(255,255,255,0.15)" stroke-width="1" />
                <polyline points={eq.path} fill="none" stroke="#22c55e" stroke-width="1.5" />
                <text x="16" y="16" fill="rgba(255,255,255,0.4)" font-size="9" font-family="monospace">max {eq.bounds.maxY.toFixed(0)}</text>
                <text x="16" y="174" fill="rgba(255,255,255,0.4)" font-size="9" font-family="monospace">min {eq.bounds.minY.toFixed(0)}</text>
            </svg>
        </div>
    {/if}
{/if}
