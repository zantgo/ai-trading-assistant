<script lang="ts">
    import { onMount } from 'svelte';
    import { useAppStore } from '../state.svelte';
    import { useEdgeStore } from '../stores/edges.svelte';
    import styles from './EdgeAnalyzer.module.css';

    let { paradigm = 'rule' }: { paradigm?: 'rule' | 'ai' } = $props();

    const app = useAppStore();
    const edge = useEdgeStore();

    const pair = app.activeInstance();
    let selectedTimeframe: number = $state(60);
    let analyzedSomething = $state(false);

    async function runAnalysis() {
        if (!edge.activeEdgeId) {
            edge.error = 'No edge selected. Go to Edge Builder and save a strategy first.';
            return;
        }
        await edge.runAnalysis(pair.symbol, selectedTimeframe);
        if (edge.simulationResults) {
            analyzedSomething = true;
        }
    }

    onMount(() => {
        edge.fetchEdges(app.pairKeyFor(pair.symbol));
    });

    function formatPct(v: number): string {
        return (v >= 0 ? '+' : '') + v.toFixed(2) + '%';
    }

    function formatNum(v: number, decimals: number = 2): string {
        return v.toFixed(decimals);
    }

    function getPValueColor(p: number): string {
        if (p < 0.01) return '#4caf50';
        if (p < 0.05) return '#8bc34a';
        if (p < 0.10) return '#ffc107';
        return '#f44336';
    }

    function getMaxDrawdownFreq(buckets: { bucket_pct: number; frequency: number }[]): number {
        return Math.max(...buckets.map(d => d.frequency), 1);
    }

    function getSharpeColor(s: number): string {
        if (s >= 2.0) return '#4caf50';
        if (s >= 1.0) return '#8bc34a';
        if (s >= 0.5) return '#ffc107';
        return '#f44336';
    }

    // Map a market regime to its equity-curve segment color.
    function regimeColor(regime: string): string {
        switch (regime) {
            case 'expansion': return '#4caf50';   // green — volatility breakouts
            case 'trending': return '#4fc3f7';    // blue — established trends
            case 'range': return '#94a3b8';       // range-reversion
            case 'compression': return '#9e9e9e'; // grey — coiling
            default: return '#607d8b';            // seed / unknown
        }
    }

    type EquityPt = { trade_index: number; cumulative_return_pct: number; regime: string };

    // Build per-segment SVG paths colored by the regime of each trade, so the
    // equity curve visually segments across market regimes.
    function buildEquitySegments(cb: EquityPt[]): { d: string; color: string }[] {
        if (cb.length < 2) return [];
        const maxVal = Math.max(...cb.map(p => p.cumulative_return_pct), 1);
        const minVal = Math.min(...cb.map(p => p.cumulative_return_pct), -1);
        const range = maxVal - minVal || 1;
        const x = (i: number) => (i / (cb.length - 1)) * 600;
        const y = (v: number) => 200 - ((v - minVal) / range) * 200;
        const segs: { d: string; color: string }[] = [];
        for (let i = 1; i < cb.length; i++) {
            const d = `M ${x(i - 1)} ${y(cb[i - 1].cumulative_return_pct)} L ${x(i)} ${y(cb[i].cumulative_return_pct)}`;
            segs.push({ d, color: regimeColor(cb[i].regime) });
        }
        return segs;
    }

    function buildMcPaths(paths: { equity_points: number[]; path_index: number }[]): string[] {
        if (paths.length === 0) return [];
        const allReturns = paths.flatMap(p => p.equity_points);
        const maxVal = Math.max(...allReturns, 10);
        const minVal = Math.min(...allReturns, -10);
        const range = maxVal - minVal || 1;
        return paths.map(path => {
            const pts = path.equity_points;
            let d = '';
            for (let i = 0; i < pts.length; i++) {
                const x = (i / (pts.length - 1)) * 600;
                const y = 200 - ((pts[i] - minVal) / range) * 200;
                if (i === 0) {
                    d += `M ${x} ${y}`;
                } else {
                    d += ` L ${x} ${y}`;
                }
            }
            return d;
        });
    }
</script>

<div class={styles.edgeAnalyzer}>
    <div class={styles.header}>
        <h2>Edge Analyzer <span class={styles.paradigmBadge}>{paradigm === 'ai' ? 'AI-Driven' : 'Rule-Based'}</span></h2>
        <div class={styles.headerControls}>
            <select bind:value={selectedTimeframe} class={styles.tfSelect}>
                <option value={60}>1m Timeframe</option>
                <option value={180}>3m Timeframe</option>
                <option value={300}>5m Timeframe</option>
                <option value={900}>15m Timeframe</option>
                <option value={3600}>1h Timeframe</option>
            </select>
            <button class={styles.btnPrimary} onclick={runAnalysis} disabled={edge.isSimulating || !edge.activeEdgeId}>
                {edge.isSimulating ? 'Analyzing...' : 'Run Analysis'}
            </button>
        </div>
    </div>

    {#if edge.error}
        <div class={styles.errorBanner}>{edge.error}</div>
    {/if}

    {#if !edge.activeEdgeId}
        <div class={styles.emptyState}>
            <p>No strategy selected. Go to <strong>Edge Builder</strong> to create and save a strategy first.</p>
        </div>
    {:else if !analyzedSomething && !edge.isSimulating}
        <div class={styles.emptyState}>
            <p>Strategy <strong>{edge.draftName || 'Unknown'}</strong> is selected. Press <strong>Run Analysis</strong> to start simulation.</p>
        </div>
    {:else if edge.isSimulating}
        <div class={styles.loadingState}>
            <div class={styles.spinner}></div>
            <p>Running backtest, bootstrap, and Monte Carlo simulations...</p>
        </div>
    {:else if edge.simulationResults}
        {@const r = edge.simulationResults}
        {@const m = r.historical_metrics}

        {#if r.cached}
            <div class={styles.cachedBanner}>Results loaded from cache</div>
        {/if}

        <!-- Top Stats Row -->
        <div class={styles.statsRow}>
            <div class={styles.statCard}>
                <div class={styles.statLabel}>Net Sharpe</div>
                <div class={styles.statValue} style="color: {getSharpeColor(m.net_sharpe_ratio)}">{formatNum(m.net_sharpe_ratio)}</div>
            </div>
            <div class={styles.statCard}>
                <div class={styles.statLabel}>Profit Factor</div>
                <div class={styles.statValue} style="color: {m.profit_factor >= 1.5 ? '#4caf50' : m.profit_factor >= 1.0 ? '#ffc107' : '#f44336'}">{formatNum(m.profit_factor)}</div>
            </div>
            <div class={styles.statCard}>
                <div class={styles.statLabel}>Win Rate</div>
                <div class={styles.statValue}>{formatNum(m.win_rate * 100, 1)}%</div>
            </div>
            <div class={styles.statCard}>
                <div class={styles.statLabel}>Max Drawdown</div>
                <div class={styles.statValue} style="color: {m.max_drawdown_pct <= 15 ? '#4caf50' : m.max_drawdown_pct <= 30 ? '#ffc107' : '#f44336'}">{formatNum(m.max_drawdown_pct)}%</div>
            </div>
            <div class={styles.statCard}>
                <div class={styles.statLabel}>Total Return</div>
                <div class={styles.statValue} style="color: {m.total_return_pct >= 0 ? '#4caf50' : '#f44336'}">{formatPct(m.total_return_pct)}</div>
            </div>
            <div class={styles.statCard}>
                <div class={styles.statLabel}>Total Trades</div>
                <div class={styles.statValue}>{m.total_trades}</div>
            </div>
        </div>

        <div class={styles.mainGrid}>
            <!-- Left Column: Backtest -->
            <div class={styles.mainColumn}>
                <h3 class={styles.colTitle}>Historical Backtest</h3>

                <div class={styles.metricsExpand}>
                    <div class={styles.metricRow}>
                        <span>Avg Trade Return</span>
                        <span class:positive={m.avg_trade_return_pct > 0} class:negative={m.avg_trade_return_pct < 0}>{formatPct(m.avg_trade_return_pct)}</span>
                    </div>
                    <div class={styles.metricRow}>
                        <span>Avg Win</span>
                        <span class="positive">{formatPct(m.avg_win_pct)}</span>
                    </div>
                    <div class={styles.metricRow}>
                        <span>Avg Loss</span>
                        <span class="negative">{formatPct(m.avg_loss_pct)}</span>
                    </div>
                    <div class={styles.metricRow}>
                        <span>Max DD Duration</span>
                        <span>{m.max_drawdown_duration} trades</span>
                    </div>
                    <div class={styles.metricRow}>
                        <span>Backtest Depth</span>
                        <span>{r.backtest_depth.toLocaleString()} candles</span>
                    </div>
                    <div class={styles.metricRow}>
                        <span>Timeframe</span>
                        <span>{r.timeframe_secs}s</span>
                    </div>
                </div>

                {#if r.backtest_curve.combined.length > 0}
                    <div class={styles.chartContainer}>
                        <div class={styles.chartLabel}>Regime-Colored Equity Curve</div>
                        <div class={styles.miniChart}>
                            <svg viewBox="0 0 600 200" class={styles.equitySvg}>
                                <line x1="0" y1={100} x2="600" y2={100} stroke="#333" stroke-dasharray="4" />
                                {#each buildEquitySegments(r.backtest_curve.combined) as seg}
                                    <path d={seg.d} fill="none" stroke={seg.color} stroke-width="1.5" />
                                {/each}
                            </svg>
                        </div>
                        <div class={styles.regimeLegend}>
                            <span class={styles.legendItem}><i class={styles.legendSwatch} style="background:#4fc3f7"></i>Trending</span>
                            <span class={styles.legendItem}><i class={styles.legendSwatch} style="background:#4caf50"></i>Expansion</span>
                            <span class={styles.legendItem}><i class={styles.legendSwatch} style="background:#ffc107"></i>Range</span>
                            <span class={styles.legendItem}><i class={styles.legendSwatch} style="background:#9e9e9e"></i>Compression</span>
                        </div>
                    </div>
                {/if}
            </div>

            <!-- Right Column: Forward Projections -->
            <div class={styles.mainColumn}>
                <h3 class={styles.colTitle}>Forward Probabilistic</h3>

                {#if r.monte_carlo_paths.length > 0}
                    <div class={styles.chartContainer}>
                        <div class={styles.chartLabel}>Monte Carlo Paths ({r.monte_carlo_paths.length} simulations)</div>
                        <div class={styles.miniChart}>
                            <svg viewBox="0 0 600 200" class={styles.equitySvg}>
                                {#each buildMcPaths(r.monte_carlo_paths.slice(0, 30)) as pathD}
                                    <path d={pathD} fill="none" stroke="#4fc3f7" stroke-width="0.5" opacity="0.3" />
                                {/each}
                            </svg>
                        </div>
                    </div>
                {/if}

                {#if r.drawdown_distribution.length > 0}
                    <div class={styles.chartContainer}>
                        <div class={styles.chartLabel}>
                            Drawdown Distribution (95% CI: {formatNum(r.confidence_95_drawdown_pct)}%)
                        </div>
                        <div class={styles.drawdownDist}>
                            {#each r.drawdown_distribution as bucket}
                                <div class={styles.ddBar}>
                                    <span class={styles.ddLabel}>{formatNum(bucket.bucket_pct)}%</span>
                                    <div
                                        class={styles.ddFill}
                                        style="width: {(bucket.frequency / getMaxDrawdownFreq(r.drawdown_distribution)) * 100}%"
                                    ></div>
                                    <span class={styles.ddCount}>{bucket.frequency}</span>
                                </div>
                            {/each}
                        </div>
                        <div class={styles.ddSummary}>
                            Median DD: {formatNum(r.monte_carlo_paths[0] ? 0 : 0)}% | Ruin Risk: {formatNum(r.probability_of_ruin_pct)}%
                        </div>
                    </div>
                {/if}
            </div>
        </div>

        <!-- Bottom Panel: Robustness -->
        <div class={styles.robustnessPanel}>
            <h3 class={styles.colTitle}>System Robustness Signals</h3>
            <div class={styles.robustnessGrid}>
                <div class={styles.robustCard}>
                    <div class={styles.robustLabel}>Aronson Bootstrap p-Value</div>
                    <div class={styles.robustValue} style="color: {getPValueColor(r.bootstrap_p_value)}">
                        {formatNum(r.bootstrap_p_value, 4)}
                    </div>
                    <div class={styles.robustStatus}>
                        {#if r.bootstrap_significant}
                            <span class={styles.statusGreen}>Statistically Significant (p &lt; 0.05)</span>
                        {:else}
                            <span class={styles.statusRed}>Not Significant — potential data-mining bias</span>
                        {/if}
                    </div>
                </div>

                <div class={styles.robustCard}>
                    <div class={styles.robustLabel}>Covel Return Skewness</div>
                    <div class={styles.robustValue} style="color: {r.skewness > 0 ? '#4caf50' : '#f44336'}">
                        {formatNum(r.skewness, 3)}
                    </div>
                    <div class={styles.robustStatus}>
                        {#if r.skewness > 0.5}
                            <span class={styles.statusGreen}>Positively Skewed — healthy trend-following profile</span>
                        {:else if r.skewness > 0}
                            <span class={styles.statusYellow}>Slightly Positive</span>
                        {:else}
                            <span class={styles.statusRed}>Negative Skew — large loss risk profile</span>
                        {/if}
                    </div>
                </div>

                <div class={styles.robustCard}>
                    <div class={styles.robustLabel}>Breiman Stopped Walk Ruin %</div>
                    <div class={styles.robustValue} style="color: {r.probability_of_ruin_pct < 10 ? '#4caf50' : r.probability_of_ruin_pct < 30 ? '#ffc107' : '#f44336'}">
                        {formatNum(r.probability_of_ruin_pct)}%
                    </div>
                    <div class={styles.robustStatus}>
                        {#if r.probability_of_ruin_pct < 5}
                            <span class={styles.statusGreen}>Very Low Ruin Risk</span>
                        {:else if r.probability_of_ruin_pct < 20}
                            <span class={styles.statusYellow}>Moderate — consider tighter drawdown limits</span>
                        {:else}
                            <span class={styles.statusRed}>High Ruin Risk — reduce size or widen stops</span>
                        {/if}
                    </div>
                </div>

                <div class={styles.robustCard}>
                    <div class={styles.robustLabel}>Win Rate</div>
                    <div class={styles.robustValue} style="color: {m.win_rate >= 0.5 ? '#4caf50' : m.win_rate >= 0.35 ? '#ffc107' : '#f44336'}">
                        {formatNum(m.win_rate * 100, 1)}%
                    </div>
                    <div class={styles.robustStatus}>
                        {m.total_trades >= 30 ? `${m.total_trades} trades — statistically meaningful` : `${m.total_trades} trades — low sample size`}
                    </div>
                </div>
            </div>
        </div>
    {/if}
</div>
