<script lang="ts">
    // PAE L5 — Backtesting tab: live /api/backtest/run form + results.
    // The form uses the shared engine-dashboard field tokens; the equity
    // curve renders min/max/zero axes.
    import KpiStrip from './../KpiStrip.svelte';
    import ExportDataButton from './../ExportDataButton.svelte';
    import { buildEngineExport } from '../../lib/engineExport';
    import styles from '../../styles/engine-dashboard.module.css';
    import local from '../PerformanceDashboard.module.css';
    import { fmtNum, fmtPct, fmtSigned } from '../../lib/format';
    import type { StrategyAnalyticsRow } from '../../types/analytics';

    interface Props {
        btSymbols: string[];
        btSymbol: string;
        btTimeframe: number;
        btStartDate: string;
        btEndDate: string;
        btCapital: number;
        btRunning: boolean;
        btError: string;
        btResult: {
            backtest_id: number;
            summary: { total_trades: number; win_count: number; loss_count: number; win_rate: number; gross_profit: number; gross_loss: number; profit_factor: number | null; expectancy: number; max_drawdown_pct: number };
            stats: StrategyAnalyticsRow;
            trades: { timestamp: number; direction: string; entry_price: number; exit_price: number; size: number; pnl: number; exit_reason: string }[];
            equity_curve: [number, number][];
        } | null;
        runBacktest: () => Promise<void>;
        minTrades?: number;
    }

    let {
        btSymbols,
        btSymbol = $bindable(),
        btTimeframe = $bindable(),
        btStartDate = $bindable(),
        btEndDate = $bindable(),
        btCapital = $bindable(),
        btRunning,
        btError,
        btResult,
        runBacktest,
        minTrades = 30,
    }: Props = $props();

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

    function equityBounds(points: [number, number][]): { minY: number; maxY: number; startX: number; endX: number } {
        if (!points || points.length === 0) return { minY: 0, maxY: 0, startX: 0, endX: 0 };
        const ys = points.map((p) => p[1]);
        const xs = points.map((p) => p[0]);
        return { minY: Math.min(...ys), maxY: Math.max(...ys), startX: xs[0], endX: xs[xs.length - 1] };
    }

    function pnlClass(v: number): string {
        if (v > 0) return local.statPositive;
        if (v < 0) return local.statNegative;
        return local.statNeutral;
    }

    function buildExport(): string {
        return buildEngineExport('performance', 'backtesting', null, {
            params: {
                symbol: btSymbol,
                timeframe_secs: btTimeframe,
                start_date: btStartDate,
                end_date: btEndDate,
                capital: btCapital,
            },
            running: btRunning,
            error: btError || null,
            result: btResult ?? null,
        });
    }
</script>

<div class={styles.card}>
    <div style="display:flex; align-items:center; justify-content:space-between; flex-wrap:wrap; gap:8px">
        <h3 class={styles.cardTitle} style="margin:0">Strategy Backtesting</h3>
        <ExportDataButton onExport={buildExport} title="Copy the backtest form + result as JSON" />
    </div>
    <p class={styles.infoLine}>
        Simulate strategy performance over historical data. Results replay the recorded MME
        decisions through the setup executor (paper engine) and apply the full significance
        treatment.
    </p>

    <div class={styles.formRow}>
        <div class={styles.field}>
            <label for="bt-symbol" class={styles.fieldLabel}>Symbol</label>
            <select id="bt-symbol" bind:value={btSymbol} class={styles.fieldInput}>
                {#each btSymbols as s (s)}
                    <option value={s}>{s}</option>
                {/each}
            </select>
        </div>
        <div class={styles.field}>
            <label for="bt-tf" class={styles.fieldLabel}>Timeframe</label>
            <select id="bt-tf" bind:value={btTimeframe} class={styles.fieldInput}>
                <option value={60}>1m</option>
                <option value={180}>3m</option>
                <option value={300}>5m</option>
                <option value={900}>15m</option>
                <option value={3600}>1h</option>
            </select>
        </div>
        <div class={styles.field}>
            <label for="bt-start" class={styles.fieldLabel}>Start Date</label>
            <input id="bt-start" type="date" bind:value={btStartDate} class={styles.fieldInput} />
        </div>
        <div class={styles.field}>
            <label for="bt-end" class={styles.fieldLabel}>End Date</label>
            <input id="bt-end" type="date" bind:value={btEndDate} class={styles.fieldInput} />
        </div>
        <div class={styles.field}>
            <label for="bt-capital" class={styles.fieldLabel}>Capital ($)</label>
            <input id="bt-capital" type="number" bind:value={btCapital} min="100" step="1000" class={styles.fieldInput} style="width:110px" />
        </div>
        <div class={styles.field} style="justify-content:flex-end">
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
            Choose a symbol + timeframe and date range, then run the backtest. Results replay the
            recorded MME decisions through the setup executor (paper only) and apply the full
            significance treatment.
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
            { label: 'Net P&L', value: fmtSigned(s.gross_profit - s.gross_loss), sub: `gross ${fmtSigned(s.gross_profit)} / ${fmtSigned(-s.gross_loss)}`, color: s.gross_profit - s.gross_loss >= 0 ? local.statPositive : local.statNegative },
            { label: 'Max Drawdown', value: '-' + fmtNum(s.max_drawdown_pct) + '%', sub: 'from peak', color: local.statNeutral },
            { label: 'Expectancy', value: fmtSigned(s.expectancy), sub: 'avg per trade', color: pnlClass(s.expectancy) },
        ]} />

        <div class="{local.edgeCard} {st.is_significant ? local.edgeSig : local.edgeNo}">
            <span class={local.edgeLabel}>EDGE VERDICT</span>
            <span class={local.edgeTitle}>{st.classification.replace(/([A-Z])/g, ' $1').trim()}</span>
            <span class={local.edgeDetail}>
                {#if st.total_trades < minTrades}
                    insufficient data — need at least {minTrades} simulated trades for a verdict
                {:else if st.is_significant}
                    statistically significant at α = {fmtNum(st.alpha, 2)} — t-test p = {fmtNum(st.p_value, 4)}, Monte Carlo p = {fmtNum(st.p_mc, 4)} ({st.monte_carlo_runs.toLocaleString()} runs)
                {:else}
                    not significant at α = {fmtNum(st.alpha, 2)} — t-test p = {fmtNum(st.p_value, 4)}, Monte Carlo p = {fmtNum(st.p_mc, 4)} ({st.monte_carlo_runs.toLocaleString()} runs) — this result could be luck
                {/if}
            </span>
        </div>

        <h3 class={styles.cardTitle} style="margin-top:16px">Equity Curve</h3>
        {#if btResult.equity_curve.length >= 2}
            {@const b = equityBounds(btResult.equity_curve)}
            <svg viewBox="0 0 640 200" width="100%" height="200" style="background:#0b0d12; border:1px solid rgba(255,255,255,0.08); border-radius:6px" role="img" aria-label="Equity curve">
                <line x1="20" y1="190" x2="620" y2="190" stroke="rgba(255,255,255,0.15)" stroke-width="1" />
                <line x1="20" y1="10" x2="20" y2="190" stroke="rgba(255,255,255,0.15)" stroke-width="1" />
                <polyline points={equityPath(btResult.equity_curve)} fill="none" stroke="#22c55e" stroke-width="1.5" />
                <text x="24" y="184" fill="rgba(255,255,255,0.4)" font-size="9" font-family="monospace">min {b.minY.toFixed(0)}</text>
                <text x="24" y="16" fill="rgba(255,255,255,0.4)" font-size="9" font-family="monospace">max {b.maxY.toFixed(0)}</text>
                <text x="600" y="184" fill="rgba(255,255,255,0.4)" font-size="9" font-family="monospace" text-anchor="end">{new Date(b.endX).toLocaleDateString()}</text>
                <text x="24" y="200" fill="rgba(255,255,255,0.4)" font-size="9" font-family="monospace">{new Date(b.startX).toLocaleDateString()}</text>
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
                    <tr><th>Time</th><th>Dir</th><th class={styles.tdRight}>Entry</th><th class={styles.tdRight}>Exit</th><th class={styles.tdRight}>Size</th><th class={styles.tdRight}>P&L</th><th>Exit Reason</th></tr>
                </thead>
                <tbody>
                    {#each btResult.trades as tr, i (i)}
                        <tr>
                            <td>{new Date(tr.timestamp).toLocaleString()}</td>
                            <td style="color:{tr.direction === 'LONG' ? '#22c55e' : '#ef4444'}">{tr.direction}</td>
                            <td class={styles.tdRight}>${fmtNum(tr.entry_price)}</td>
                            <td class={styles.tdRight}>${fmtNum(tr.exit_price)}</td>
                            <td class={styles.tdRight}>{fmtNum(tr.size, 4)}</td>
                            <td class={styles.tdRight} style="color:{tr.pnl >= 0 ? '#22c55e' : '#ef4444'}">{fmtSigned(tr.pnl)}</td>
                            <td class={styles.tdMono}>{tr.exit_reason}</td>
                        </tr>
                    {/each}
                </tbody>
            </table>
        {/if}
    {/if}
</div>
