<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import type { WsState } from '../lib/websocket.svelte';
    import styles from './GeneralDashboard.module.css';
    import SvgIcon from '../lib/SvgIcon.svelte';
    import WatchlistRunnerButton from './WatchlistRunnerButton.svelte';

    interface Props {
        wssMap: Record<string, WsState>;
    }

    let { wssMap }: Props = $props();

    const app = useAppStore();

    const instances = $derived(Object.entries(app.instancesMap));
    const activeCount = $derived(instances.filter(([_, p]) => p.isConnected).length);
    const totalCount = $derived(instances.length);

    const marketBiasSummary = $derived.by(() => {
        const entries = instances.filter(([_, p]) => p.analysis?.bias);
        const biasCounts: Record<string, number> = {};
        for (const [_, p] of entries) {
            const b = p.analysis?.bias ?? 'NEUTRAL';
            biasCounts[b] = (biasCounts[b] || 0) + 1;
        }
        const total = entries.length;
        return { biasCounts, total };
    });

    const aggregateRisk = $derived.by(() => {
        const entries = instances.filter(([_, p]) => p.risk?.overall_risk);
        if (entries.length === 0) return null;
        let totalRisk = 0;
        let totalCascade = 0;
        for (const [_, p] of entries) {
            totalRisk += p.risk?.overall_risk?.score ?? 0;
            totalCascade += p.risk?.cascade_risk?.score ?? 0;
        }
        return {
            avgRisk: totalRisk / entries.length,
            avgCascade: totalCascade / entries.length,
            count: entries.length,
        };
    });

    const regimeSummary = $derived.by(() => {
        const counts: Record<string, number> = {};
        for (const [_, p] of instances) {
            const r = p.analysis?.market_regime ?? 'UNKNOWN';
            counts[r] = (counts[r] || 0) + 1;
        }
        return Object.entries(counts).sort((a, b) => b[1] - a[1]);
    });

    const advisorySummary = $derived.by(() => {
        const entries = instances.filter(([_, p]) => p.advisory);
        const stanceCounts: Record<string, number> = {};
        for (const [_, p] of entries) {
            const s = p.advisory?.directional_guidance ?? 'NEUTRAL';
            stanceCounts[s] = (stanceCounts[s] || 0) + 1;
        }
        return { stanceCounts, total: entries.length };
    });

    const sortedInstances = $derived(
        instances
            .map(([key, p]) => ({
                key,
                symbol: p.symbol,
                price: p.microTerm?.priceText ?? '--',
                bias: p.analysis?.bias ?? '--',
                regime: p.analysis?.market_regime ?? '--',
                confidence: Math.round(((p.analysis?.confidence ?? 0) * 100)),
                risk: Math.round(p.risk?.overall_risk?.score ?? 50),
                connected: p.isConnected,
            }))
            .sort((a, b) => b.confidence - a.confidence)
    );

    function biasColor(bias: string): string {
        if (bias.includes('STRONG_BULLISH') || bias.includes('BULLISH')) return '#22c55e';
        if (bias.includes('STRONG_BEARISH') || bias.includes('BEARISH')) return '#ef4444';
        return '#f59e0b';
    }

    function riskColor(risk: number): string {
        if (risk >= 60) return '#ef4444';
        if (risk >= 40) return '#f59e0b';
        if (risk >= 20) return '#4ade80';
        return '#22c55e';
    }

    function regimeLabel(r: string): string {
        return r.replace(/_/g, ' ').replace(/\b\w/g, c => c.toUpperCase());
    }
</script>

<div class={styles.dashboardView}>
    <div class={styles.content}>
        <div class={styles.header}>
            <h2 class={styles.title}>MARKET OVERVIEW</h2>
            <span class={styles.subtitle}>
                {activeCount}/{totalCount} instances active
            </span>
        </div>

        {#if totalCount === 0}
            <div class={styles.featurePlaceholder}>
                <SvgIcon name="layoutDashboard" size={64} />
                <h2 class={styles.featurePlaceholderTitle}>Market Overview</h2>
                <p class={styles.featurePlaceholderMsg}>
                    Add workspaces to see system-wide market intelligence across all monitored pairs.
                </p>
            </div>
        {:else}
            <div class={styles.grid}>
                <div class={styles.card}>
                    <div class={styles.cardHeader}>
                        <span class={styles.cardTitle}>Market Bias</span>
                        <span class={styles.cardCount}>{marketBiasSummary.total} pairs</span>
                    </div>
                    <div class={styles.barList}>
                        {#each Object.entries(marketBiasSummary.biasCounts) as [bias, count]}
                            {#if count > 0}
                                <div class={styles.barRow}>
                                    <span class={styles.barLabel} style="color: {biasColor(bias)}">
                                        {bias.replace('STRONG_', '')}
                                    </span>
                                    <div class={styles.barTrack}>
                                        <div class={styles.barFill}
                                             style="width: {((count / marketBiasSummary.total) * 100).toFixed(0)}%; background: {biasColor(bias)}">
                                        </div>
                                    </div>
                                    <span class={styles.barVal}>{count}</span>
                                </div>
                            {/if}
                        {/each}
                    </div>
                </div>

                <div class={styles.card}>
                    <div class={styles.cardHeader}>
                        <span class={styles.cardTitle}>Directional Guidance</span>
                        <span class={styles.cardCount}>{advisorySummary.total} pairs</span>
                    </div>
                    <div class={styles.barList}>
                        {#each Object.entries(advisorySummary.stanceCounts) as [guidance, count]}
                            {#if count > 0}
                                <div class={styles.barRow}>
                                    <span class={styles.barLabel}>{guidance}</span>
                                    <div class={styles.barTrack}>
                                        <div class={styles.barFill}
                                             style="width: {((count / advisorySummary.total) * 100).toFixed(0)}%">
                                        </div>
                                    </div>
                                    <span class={styles.barVal}>{count}</span>
                                </div>
                            {/if}
                        {/each}
                    </div>
                </div>

                <div class={styles.card}>
                    <div class={styles.cardHeader}>
                        <span class={styles.cardTitle}>Risk Aggregate</span>
                    </div>
                    {#if aggregateRisk}
                        <div class={styles.riskStats}>
                            <div class={styles.riskMetric}>
                                <span class={styles.riskLabel}>Avg Risk</span>
                                <span class={styles.riskVal} style="color: {riskColor(aggregateRisk.avgRisk)}">
                                    {aggregateRisk.avgRisk.toFixed(0)}
                                </span>
                            </div>
                            <div class={styles.riskMetric}>
                                <span class={styles.riskLabel}>Avg Cascade</span>
                                <span class={styles.riskVal} style="color: {riskColor(aggregateRisk.avgCascade)}">
                                    {aggregateRisk.avgCascade.toFixed(0)}
                                </span>
                            </div>
                            <div class={styles.riskMetric}>
                                <span class={styles.riskLabel}>Pairs w/ Risk</span>
                                <span class={styles.riskVal}>{aggregateRisk.count}</span>
                            </div>
                        </div>
                    {:else}
                        <div class={styles.emptyRow}>Awaiting risk data...</div>
                    {/if}
                </div>

                <div class={styles.card}>
                    <div class={styles.cardHeader}>
                        <span class={styles.cardTitle}>Regime Distribution</span>
                    </div>
                    <div class={styles.barList}>
                        {#each regimeSummary as [regime, count]}
                            <div class={styles.barRow}>
                                <span class={styles.barLabel}>{regimeLabel(regime)}</span>
                                <span class={styles.barVal}>{count}</span>
                            </div>
                        {/each}
                    </div>
                </div>
            </div>

            <div class={styles.tableSection}>
                <h3 class={styles.sectionTitle}>Asset Rankings</h3>
                <table class={styles.table}>
                    <thead>
                        <tr>
                            <th class={styles.th}>Symbol</th>
                            <th class={styles.th}>Price</th>
                            <th class={styles.th}>Bias</th>
                            <th class={styles.th}>Regime</th>
                            <th class={styles.th}>Confidence</th>
                            <th class={styles.th}>Risk</th>
                        </tr>
                    </thead>
                    <tbody>
                        {#each sortedInstances as inst (inst.key)}
                            <tr class={styles.tr}>
                                <td class={styles.tdSymbol}>
                                    <span class={styles.statusDot} class:active={inst.connected}></span>
                                    {inst.symbol}
                                </td>
                                <td class={styles.tdMono}>{inst.price}</td>
                                <td class={styles.td} style="color: {biasColor(inst.bias)}">{inst.bias}</td>
                                <td class={styles.td}>{inst.regime}</td>
                                <td class={styles.td}>{inst.confidence}%</td>
                                <td class={styles.td} style="color: {riskColor(inst.risk)}">{inst.risk}</td>
                            </tr>
                        {/each}
                    </tbody>
                </table>
            </div>
        {/if}

        <WatchlistRunnerButton {wssMap} />
    </div>
</div>
