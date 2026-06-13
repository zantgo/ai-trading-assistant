<script lang="ts">
    import { getState } from '../state.svelte';
    import type { TradeLedgerRecord } from '../state.svelte';

    const app = getState();

    $effect(() => {
        app.fetchTradeLedger();
    });

    function formatUsd(v: number): string {
        if (Math.abs(v) >= 1000) return '$' + v.toLocaleString('en-US', { maximumFractionDigits: 0 });
        return '$' + v.toFixed(2);
    }
    function formatPct(v: number): string { return v.toFixed(2) + '%'; }
    function formatTs(ts: number): string {
        const d = new Date(ts > 9_000_000_000 ? ts : ts * 1000);
        return d.toISOString().substring(0, 19).replace('T', ' ');
    }
</script>

<div class="tl-layout">
    <div class="tl-header">
        <h3 class="tl-title">TRADE LIST LEDGER</h3>
        <span class="tl-count">{app.tradeLedgerRecords.length} trades</span>
    </div>
    <div class="tl-table-wrap">
        <table class="tl-table">
            <thead>
                <tr>
                    <th>ID</th>
                    <th>Timestamp</th>
                    <th>Symbol</th>
                    <th>Dir</th>
                    <th>Entry</th>
                    <th>Exit</th>
                    <th>Size</th>
                    <th>PnL</th>
                    <th>Fees</th>
                    <th>ROI%</th>
                    <th>Source</th>
                </tr>
            </thead>
            <tbody>
                {#each app.tradeLedgerRecords as trade (trade.id)}
                    <tr>
                        <td class="tl-mono tl-dim">{trade.id}</td>
                        <td class="tl-mono tl-dim">{formatTs(trade.entry_timestamp)}</td>
                        <td class="tl-symbol">{trade.symbol}</td>
                        <td class="tl-dir" class:tl-long={trade.direction === 'LONG'} class:tl-short={trade.direction === 'SHORT'}>
                            {trade.direction}
                        </td>
                        <td class="tl-mono">{formatUsd(trade.entry_price)}</td>
                        <td class="tl-mono">{formatUsd(trade.exit_price)}</td>
                        <td class="tl-mono">{trade.size.toFixed(6)}</td>
                        <td class="tl-mono" class:tl-pnl-pos={trade.realized_pnl > 0} class:tl-pnl-neg={trade.realized_pnl < 0}>
                            {formatUsd(trade.realized_pnl)}
                        </td>
                        <td class="tl-mono tl-dim">{formatUsd(trade.commission_fees)}</td>
                        <td class="tl-mono" class:tl-roi-pos={trade.roi_percentage > 0} class:tl-roi-neg={trade.roi_percentage < 0}>
                            {formatPct(trade.roi_percentage)}
                        </td>
                        <td>
                            <span class="tl-source" class:tl-src-auto={trade.trigger_source === 'AUTOMATED'}>
                                {trade.trigger_source}
                            </span>
                        </td>
                    </tr>
                {/each}
            </tbody>
        </table>
        {#if app.tradeLedgerRecords.length === 0}
            <div class="tl-empty">No trade records found. Trades will appear here once executed.</div>
        {/if}
    </div>
</div>

<style>
    .tl-layout { max-width: 1400px; margin: 0 auto; width: 100%; padding: 16px; box-sizing: border-box; }
    .tl-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px; }
    .tl-title { font-size: 12px; font-weight: 700; color: #64748b; text-transform: uppercase; letter-spacing: 0.05em; margin: 0; }
    .tl-count { font-size: 10px; color: #64748b; font-weight: 600; }
    .tl-table-wrap { overflow-x: auto; } /* max-height: 70vh; overflow: auto; */
    .tl-table {
        width: 100%; border-collapse: collapse; font-size: 10px; color: #94a3b8;
        background: #131722; border: 1px solid #2a2e39; border-radius: 8px;
    }
    .tl-table thead { position: sticky; top: 0; background: #131722; z-index: 1; }
    .tl-table th {
        text-align: left; padding: 8px 10px; font-weight: 700; color: #64748b; text-transform: uppercase;
        letter-spacing: 0.04em; border-bottom: 2px solid #1e293b; font-size: 9px;
    }
    .tl-table td { padding: 6px 10px; border-bottom: 1px solid #0f131c; white-space: nowrap; }
    .tl-table tbody tr:hover { background: #1a1f2e; }
    .tl-mono { font-family: ui-monospace, monospace; font-size: 9px; }
    .tl-dim { color: #64748b; }
    .tl-symbol { font-weight: 700; color: #cbd5e1; }
    .tl-dir { font-weight: 700; text-transform: uppercase; }
    .tl-long { color: #10b981; }
    .tl-short { color: #ef4444; }
    .tl-pnl-pos { color: #10b981; font-weight: 700; }
    .tl-pnl-neg { color: #ef4444; font-weight: 700; }
    .tl-roi-pos { color: #10b981; }
    .tl-roi-neg { color: #ef4444; }
    .tl-source {
        font-size: 9px; padding: 2px 6px; border-radius: 3px; font-weight: 600; text-transform: uppercase;
        background: rgba(59,130,246,0.08); color: #60a5fa;
    }
    .tl-src-auto { background: rgba(168,85,247,0.08); color: #a78bfa; }
    .tl-empty { font-size: 11px; color: #64748b; text-align: center; padding: 40px; font-style: italic; }
</style>
