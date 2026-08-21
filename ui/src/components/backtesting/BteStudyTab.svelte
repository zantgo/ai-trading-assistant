<script lang="ts">
    // BteStudyTab — the finished data-science presentation of a run:
    // KPI strip, equity curve, drawdown chart, rolling win-rate, trade
    // PnL histogram, per-exit-reason table, and the edge verdict.
    import styles from '../../styles/engine-dashboard.module.css';
    import KpiStrip from '../KpiStrip.svelte';
    import { fmtNum, fmtSigned } from '../../lib/format';
    import {
        linePath, areaPath, rollingWinRate, pnlHistogram, drawdownSeries,
    } from '../../lib/studyCharts';
    import type { BteResult } from './BacktestingDashboard.svelte';

    interface Props {
        result: BteResult | null;
        portfolio: any[];
        signals: any[];
    }

    let { result, portfolio, signals }: Props = $props();

    const summary = $derived(result?.summary ?? null);
    const stats = $derived(result?.stats ?? null);

    const kpis = $derived.by(() => {
        const s = summary;
        if (!s) return [];
        return [
            { label: 'Total Trades', value: String(s.total_trades), sub: 'simulated closes' },
            { label: 'Win Rate', value: fmtNum(s.win_rate) + '%', sub: `${s.win_count}W / ${s.loss_count}L`, color: s.win_rate >= 50 ? '#22c55e' : '#ef4444' },
            { label: 'Profit Factor', value: fmtNum(s.profit_factor), sub: 'gross win / loss', color: s.profit_factor != null && s.profit_factor >= 1 ? '#22c55e' : '#ef4444' },
            { label: 'Net P&L', value: fmtSigned(s.gross_profit - s.gross_loss), sub: 'total realized', color: s.gross_profit - s.gross_loss >= 0 ? '#22c55e' : '#ef4444' },
            { label: 'Max Drawdown', value: '-' + fmtNum(s.max_drawdown_pct) + '%', sub: 'from equity peak' },
            { label: 'Expectancy', value: fmtSigned(s.expectancy), sub: 'avg per trade', color: s.expectancy >= 0 ? '#22c55e' : '#ef4444' },
        ];
    });

    const equity = $derived(result?.equity_curve ?? []);
    const equityLine = $derived(linePath(equity, 600, 180));
    const dd = $derived(drawdownSeries(equity));
    const ddArea = $derived(areaPath(dd, 600, 120, 0));
    const pnls = $derived((result?.trades ?? []).map((t) => t.pnl));
    const rollWin = $derived(rollingWinRate(pnls, 10));
    const rollLine = $derived(linePath(rollWin, 600, 120, 30));
    const hist = $derived(pnlHistogram(pnls));
    const maxHist = $derived(Math.max(1, ...hist.map((h) => h.count)));

    const exitReasons = $derived.by(() => {
        const map = new Map<string, { count: number; pnl: number }>();
        for (const t of result?.trades ?? []) {
            const e = map.get(t.exit_reason) ?? { count: 0, pnl: 0 };
            e.count++;
            e.pnl += t.pnl;
            map.set(t.exit_reason, e);
        }
        return [...map.entries()].sort((a, b) => b[1].count - a[1].count);
    });

    const minTrades = $derived.by(() => {
        // The verdict floor mirrors the backend default; the server-side
        // classification already honors the configured value.
        const classification = stats?.classification;
        if (classification === 'InsufficientData') return 999_999;
        return 0;
    });

    function dateTs(ts: number): string {
        return new Date(ts).toLocaleDateString();
    }
</script>

{#if !result}
    <div class={styles.card}>
        <div class={styles.empty} style="height:160px; display:flex; align-items:center; justify-content:center">
            No study yet — run a backtest from the Overview tab, or load one from History.
        </div>
    </div>
{:else}
    <div class={styles.card}>
        <div style="display:flex; align-items:center; justify-content:space-between; flex-wrap:wrap; gap:8px">
            <div>
                <h3 class={styles.cardTitle} style="margin:0">Study Report — #{result.backtest_id}</h3>
                <p class={styles.infoLine}>
                    {result.params.symbol} · {result.params.timeframe_secs}s ·
                    {dateTs(result.params.from_secs * 1000)} → {dateTs(result.params.to_secs * 1000)} ·
                    {result.mode ?? 'recorded'} mode · ${result.params.initial_capital.toLocaleString()} capital
                </p>
            </div>
            {#if stats}
                <div class="{styles.badge} {stats.is_significant ? styles.badgeLong : styles.badgeError}">
                    {String(stats.classification).replace(/([A-Z])/g, ' $1').trim().toUpperCase()}
                </div>
            {/if}
        </div>

        <KpiStrip items={kpis} />

        {#if stats}
            <div class="{styles.alertBanner} {stats.is_significant ? styles.alertWarn : styles.alertError}" style="margin-top:12px">
                {#if minTrades > 0}
                    Insufficient data — need at least {stats.min_trades ?? 30} simulated trades for a verdict.
                {:else if stats.is_significant}
                    Statistically significant at α = {fmtNum(stats.alpha, 2)} — t-test p = {fmtNum(stats.p_value, 4)},
                    Monte Carlo p = {fmtNum(stats.p_mc, 4)} ({stats.monte_carlo_runs?.toLocaleString() ?? '10,000'} runs).
                {:else}
                    Not significant at α = {fmtNum(stats.alpha, 2)} — t-test p = {fmtNum(stats.p_value, 4)},
                    Monte Carlo p = {fmtNum(stats.p_mc, 4)} ({stats.monte_carlo_runs?.toLocaleString() ?? '10,000'} runs) — this result could be luck.
                {/if}
            </div>
        {/if}

        <h4 class={styles.cardTitle} style="margin-top:16px">Equity Curve</h4>
        {#if equity.length >= 2}
            <svg viewBox="0 0 640 200" width="100%" height="200" style="background:#0b0d12; border:1px solid rgba(255,255,255,0.08); border-radius:6px" role="img" aria-label="Equity curve">
                <line x1="10" y1="190" x2="630" y2="190" stroke="rgba(255,255,255,0.15)" stroke-width="1" />
                <line x1="10" y1="10" x2="10" y2="190" stroke="rgba(255,255,255,0.15)" stroke-width="1" />
                <polyline points={equityLine.path} fill="none" stroke="#22c55e" stroke-width="1.5" />
                <text x="16" y="184" fill="rgba(255,255,255,0.4)" font-size="9" font-family="monospace">min {equityLine.bounds.minY.toFixed(0)}</text>
                <text x="16" y="16" fill="rgba(255,255,255,0.4)" font-size="9" font-family="monospace">max {equityLine.bounds.maxY.toFixed(0)}</text>
            </svg>
        {:else}
            <div class={styles.empty} style="height:100px; display:flex; align-items:center; justify-content:center">Not enough data for an equity curve.</div>
        {/if}

        <div style="display:grid; grid-template-columns:repeat(auto-fit, minmax(280px, 1fr)); gap:16px; margin-top:16px">
            <div>
                <h4 class={styles.cardTitle}>Drawdown (%)</h4>
                {#if dd.length >= 2}
                    <svg viewBox="0 0 640 140" width="100%" height="140" style="background:#0b0d12; border:1px solid rgba(255,255,255,0.08); border-radius:6px" role="img" aria-label="Drawdown">
                        <path d={ddArea} fill="rgba(239,68,68,0.25)" stroke="#ef4444" stroke-width="1" />
                    </svg>
                {:else}
                    <div class={styles.empty}>No drawdown data.</div>
                {/if}
            </div>
            <div>
                <h4 class={styles.cardTitle}>Rolling Win Rate (10 trades)</h4>
                {#if rollWin.length >= 2}
                    <svg viewBox="0 0 640 140" width="100%" height="140" style="background:#0b0d12; border:1px solid rgba(255,255,255,0.08); border-radius:6px" role="img" aria-label="Rolling win rate">
                        <line x1="10" y1="70" x2="630" y2="70" stroke="rgba(255,255,255,0.15)" stroke-width="1" stroke-dasharray="3 3" />
                        <polyline points={rollLine.path} fill="none" stroke="#3b82f6" stroke-width="1.5" />
                    </svg>
                {:else}
                    <div class={styles.empty}>Needs ≥ 10 trades for a rolling window.</div>
                {/if}
            </div>
        </div>

        <h4 class={styles.cardTitle} style="margin-top:16px">Trade P&L Distribution</h4>
        {#if hist.length > 0 && pnls.length > 0}
            <div style="display:flex; align-items:flex-end; gap:4px; height:120px; padding:8px; background:#0b0d12; border:1px solid rgba(255,255,255,0.08); border-radius:6px" role="img" aria-label="PnL histogram">
                {#each hist as h (h.label)}
                    <div style="flex:1; display:flex; flex-direction:column; align-items:center; justify-content:flex-end; height:100%" title={`${h.label}: ${h.count} trades`}>
                        <span style="font-size:9px; color:rgba(255,255,255,0.5); font-family:monospace">{h.count}</span>
                        <div style="width:100%; height:{Math.max(2, (h.count / maxHist) * 100)}%; background:{h.min >= 0 ? '#22c55e' : '#ef4444'}; opacity:0.75; border-radius:2px 2px 0 0"></div>
                    </div>
                {/each}
            </div>
        {:else}
            <div class={styles.empty}>No trades in this run.</div>
        {/if}

        <h4 class={styles.cardTitle} style="margin-top:16px">Exit Reasons</h4>
        {#if exitReasons.length === 0}
            <div class={styles.empty}>No trades in this run.</div>
        {:else}
            <table class={styles.table}>
                <thead>
                    <tr><th>Exit Reason</th><th class={styles.tdRight}>Trades</th><th class={styles.tdRight}>P&L</th></tr>
                </thead>
                <tbody>
                    {#each exitReasons as [reason, agg] (reason)}
                        <tr>
                            <td class={styles.tdMono}>{reason}</td>
                            <td class={styles.tdRight}>{agg.count}</td>
                            <td class={styles.tdRight} style="color:{agg.pnl >= 0 ? '#22c55e' : '#ef4444'}">{fmtSigned(agg.pnl)}</td>
                        </tr>
                    {/each}
                </tbody>
            </table>
        {/if}

        <p class={styles.infoLine} style="margin-top:12px">
            Backed by {signals.length.toLocaleString()} persisted decision snapshots and
            {portfolio.length.toLocaleString()} portfolio samples in the data-science tables
            (see DIE / MME / PME tabs for the per-engine breakdowns).
        </p>
    </div>
{/if}
