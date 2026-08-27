<script lang="ts">
    // BtePortfolioTab (PME) — the simulated capital journey: equity curve,
    // cash/margin/exposure/drawdown samples from the DS portfolio table.
    import styles from '../../styles/engine-dashboard.module.css';
    import KpiStrip from '../KpiStrip.svelte';
    import { fmtNum } from '../../lib/format';
    import { linePath } from '../../lib/studyCharts';

    interface Props {
        portfolio: any[];
        equity: [number, number][];
        capital: number;
    }

    let { portfolio, equity, capital }: Props = $props();

    const last = $derived(portfolio.length > 0 ? portfolio[portfolio.length - 1] : null);
    const finalEquity = $derived(equity.length > 0 ? equity[equity.length - 1][1] : capital);

    const kpis = $derived.by(() => {
        if (!last && equity.length === 0) return [];
        return [
            { label: 'Final Equity', value: '$' + fmtNum(finalEquity), sub: 'from $' + fmtNum(capital), color: finalEquity >= capital ? '#22c55e' : '#ef4444' },
            { label: 'Net Return', value: (((finalEquity - capital) / capital) * 100).toFixed(1) + '%', sub: 'over the window', color: finalEquity >= capital ? '#22c55e' : '#ef4444' },
            { label: 'Peak Margin Used', value: '$' + fmtNum(last ? Math.max(...portfolio.map((p) => p.margin_used)) : 0), sub: 'position notional' },
            { label: 'Peak Exposure', value: fmtNum(last ? Math.max(...portfolio.map((p) => p.exposure_pct)) : 0) + '%', sub: 'of initial capital' },
            { label: 'Max Drawdown', value: '-' + fmtNum(last ? Math.max(...portfolio.map((p) => p.drawdown_pct)) : 0) + '%', sub: 'from equity peak' },
            { label: 'Max Open Positions', value: String(last ? Math.max(...portfolio.map((p) => p.positions_open)) : 0), sub: 'simultaneous' },
        ];
    });

    const equityLine = $derived(linePath(equity, 600, 180));
</script>

<div class={styles.card}>
    <h3 class={styles.cardTitle} style="margin-top:0">PME · Simulated Portfolio</h3>
    <p class={styles.infoLine}>
        The capital/margin ledger of the replayed session — the same
        ExecutionEngine paper backend the live paper mode runs.
    </p>

    {#if portfolio.length === 0 && equity.length === 0}
        <div class={styles.empty} style="height:120px; display:flex; align-items:center; justify-content:center">
            No portfolio samples. Run a backtest first (Overview tab).
        </div>
    {:else}
        <KpiStrip items={kpis} />
        <h4 class={styles.cardTitle} style="margin-top:16px">Equity Journey</h4>
        {#if equity.length >= 2}
            <svg viewBox="0 0 640 200" width="100%" height="200" style="background:#0b0d12; border:1px solid rgba(255,255,255,0.08); border-radius:6px" role="img" aria-label="Equity journey">
                <polyline points={equityLine.path} fill="none" stroke="#22c55e" stroke-width="1.5" />
            </svg>
        {:else}
            <div class={styles.empty}>Not enough samples for a curve.</div>
        {/if}
    {/if}
</div>
