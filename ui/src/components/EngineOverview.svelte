<script lang="ts">
    let { engine }: { engine: string } = $props();

    const ENGINE_INFO: Record<string, { title: string; description: string; sections: string[] }> = {
        portfolio: {
            title: 'Portfolio Management',
            description: 'Tracks active positions, monitors margin utilization, calculates exposure concentration, and enforces the PME safety veto. The Portfolio engine is the single source of truth for account state — all position-size and capital decisions flow through it.',
            sections: ['Position tracking (live + paper)', 'Margin & exposure monitoring', 'Capital matrix & sizing protocol', 'Safety manager (drawdown veto, loss dropout, caution levels)'],
        },
        trade_automation: {
            title: 'Trade Automation',
            description: 'Evaluates execution policies against market conditions, runs the Position Sizing Protocol, and routes orders to the paper or live trading engine. The strategy layer is identical between paper and live modes — toggling the operational mode preserves strategy behavior.',
            sections: ['Execution policy evaluation', 'Position sizing ($S = E · R / (Dₛₗ / 100))', 'Paper trading engine (simulated matching)', 'Live trading adapter (Hyperliquid / Bitget — future)'],
        },
        performance: {
            title: 'Performance Analytics',
            description: 'Compiles dashboard statistics from closed trades, runs the strategy optimizer against regime-specific performance data, and computes risk-adjusted return metrics (Sharpe, Sortino, Ulcer Index, Calmar). The Monte Carlo significance tester uses sign-randomization (10,000 runs per policy).',
            sections: ['Dashboard stats (win rate, expectancy, profit factor, streaks)', 'Regime-strategy optimizer (regime → performance maps)', 'Monte Carlo sign-randomization (p_mc significance)', 'Portfolio equity logger (60s snapshots)'],
        },
        data_infra: {
            title: 'Data Infrastructure',
            description: 'The sole ingress point for external market data. Ingests WebSocket/REST feeds from Hyperliquid and Bitget, reconstructs candles on reconnect, enforces the NTP clock-monitor UTC drift budget, and tracks connection quality across rolling windows.',
            sections: ['WebSocket ingestion (Hyperliquid + Bitget)', 'Candle reconstruction on reconnect gaps', 'NTP clock monitor (≤50µs UTC drift budget)', 'Connection quality tracking (1h/6h/24h rolling scores)'],
        },
    };

    const info = $derived(ENGINE_INFO[engine] || { title: engine, description: 'No description available.', sections: [] });
</script>

<div style="display:flex; flex-direction:column; height:100%; background:#000; color:#fff; font-family:var(--mono); padding:2rem; gap:1rem;">
    <h2 style="font-size:1.2rem; text-transform:uppercase; letter-spacing:0.1em; color:#5a5f6e;">{info.title}</h2>
    <p style="color:#ccc; font-size:0.85rem; line-height:1.6; max-width:48rem;">{info.description}</p>

    <div style="border:1px solid #2a2e39; border-radius:8px; padding:1.25rem; max-width:48rem;">
        <h3 style="font-size:0.9rem; margin-bottom:0.75rem; color:#888;">Components</h3>
        <ul style="list-style:none; padding:0; margin:0; display:flex; flex-direction:column; gap:0.4rem;">
            {#each info.sections as section}
                <li style="color:#aaa; font-size:0.8rem; padding-left:1.2rem; position:relative;">
                    <span style="position:absolute; left:0; color:#4caf50;">▶</span>
                    {section}
                </li>
            {/each}
        </ul>
    </div>

    <p style="color:#5a5f6e; font-size:0.75rem; margin-top:auto;">
        Configure this engine in <strong>Settings</strong> → <code>config.toml</code> → <code>[workspace]</code>.
        {#if engine === 'trade_automation'}
            Operational mode: <strong>Advisory</strong> (market monitor only), <strong>PaperTrading</strong> (simulated orders), or <strong>LiveTrading</strong> (future).
        {/if}
    </p>
</div>
