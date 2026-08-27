<script lang="ts">
    // PAE Methodology tab — the significance treatment, config-driven:
    // α, Monte Carlo runs and the min-trade verdict floor come from
    // `[workspace.analytics]` via /api/config.
    import { onMount } from 'svelte';
    import KpiStrip from './../KpiStrip.svelte';
    import styles from '../../styles/engine-dashboard.module.css';

    interface AnalyticsCfg { alpha?: number; monte_carlo_runs?: number; min_trades_for_verdict?: number }
    let analytics = $state<AnalyticsCfg>({});

    onMount(() => {
        void fetch('/api/config')
            .then((r) => (r.ok ? r.json() : {}))
            .then((c: unknown) => {
                const a = (c as { analytics?: AnalyticsCfg })?.analytics;
                if (a) analytics = a;
            })
            .catch(() => {});
    });

    const alpha = $derived(analytics.alpha ?? 0.05);
    const runs = $derived(analytics.monte_carlo_runs ?? 10000);
    const minTrades = $derived(analytics.min_trades_for_verdict ?? 30);

    const VERDICT_CLASSES = $derived([
        { name: 'StrongEdge', rule: 'PF > 1.2 · WR > 50% · p & p_mc < 0.01' },
        { name: 'ModerateEdge', rule: `PF > 1.5 · WR > 45% · p & p_mc < α = ${alpha}` },
        { name: 'WeakMarginalEdge', rule: 'PF ≥ 1.0 · p ≤ 0.10' },
        { name: 'NoEdgeNegative', rule: 'everything else' },
        { name: 'InsufficientData', rule: `fewer than ${minTrades} trades` },
    ]);
</script>

<div class={styles.card}>
    <h3 class={styles.cardTitle}>Significance Treatment</h3>
    <p class={styles.infoLine}>
        Every PAE verdict (live strategy analytics and backtests) uses Null Hypothesis
        Significance Testing: H₀ = mean net P&L ≤ 0 vs H₁ &gt; 0. An edge is declared significant
        only when BOTH the one-tailed t-test p-value and the Monte Carlo sign-randomization
        p-value fall below α. The parameters are operator-tunable in
        <code>config.toml → [workspace.analytics]</code> — the values below are what the engine
        actually runs with.
    </p>
    <KpiStrip items={[
        { label: 'Alpha (α)', value: String(alpha), sub: 'significance level', color: '#f59e0b' },
        { label: 'Monte Carlo Runs', value: runs.toLocaleString(), sub: 'sign randomization' },
        { label: 'Min Trades', value: String(minTrades), sub: 'verdict floor' },
        { label: 'Tests', value: '2', sub: 't-test + Monte Carlo', color: '#22c55e' },
    ]} />
</div>

<div class={styles.card}>
    <h3 class={styles.cardTitle}>Verdict Classes</h3>
    <table class={styles.table}>
        <thead><tr><th>Classification</th><th>Rule</th></tr></thead>
        <tbody>
            {#each VERDICT_CLASSES as v (v.name)}
                <tr>
                    <td><span class="{styles.badge} {v.name === 'StrongEdge' || v.name === 'ModerateEdge' ? styles.badgeLong : v.name === 'InsufficientData' ? styles.badgeEmpty : v.name === 'NoEdgeNegative' ? styles.badgeError : styles.badgeNeutral}">{v.name}</span></td>
                    <td>{v.rule}</td>
                </tr>
            {/each}
        </tbody>
    </table>
</div>

<div class={styles.card}>
    <h3 class={styles.cardTitle}>Interpretation Guide</h3>
    <p class={styles.infoLine}>
        <strong>Significant:</strong> both p-values below α — the edge is unlikely to be luck and is
        worth forward-testing in paper mode.<br />
        <strong>Not significant:</strong> the sample cannot rule out zero expectancy — treat the
        result as no edge regardless of the win rate.<br />
        <strong>Insufficient data:</strong> fewer than {minTrades} simulated/recorded trades — no
        verdict is issued.<br />
        The recorded-decision backtest replays the exact MME decision matrices, so the same verdict
        machinery evaluates both historical replay and live performance.
    </p>
</div>
