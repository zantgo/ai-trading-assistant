<script lang="ts">
    // HeaderKpiStrip — six top-row KPIs answering the operator's
    // top-of-screen needs:
    //   Valid Trades, Best Opportunity, Avg R:R, Market Bias,
    //   Avg Risk, Coverage.
    import { useAppStore } from '../../state.svelte';
    import {
        aggregateRR,
        aggregateConfidence,
        aggregateRisk,
        collectActiveSetups,
        pickBestOpportunity,
    } from '../../lib/tradeAggregates';
    import { biasColor, rrColor, scoreColor, formatRR } from '../../lib/dashboardColors';
    import styles from './HeaderKpiStrip.module.css';

    const app = useAppStore();

    const kpis = $derived.by(() => {
        const instances = Object.values(app.instancesMap);
        const setups = collectActiveSetups(instances);
        const actionable = setups.filter(
            (s) => s.viability === 'Actionable' && s.readiness === 'READY',
        );
        const best = pickBestOpportunity(instances);
        const rr = aggregateRR(instances);
        const conf = aggregateConfidence(instances);
        const risk = aggregateRisk(instances);
        const overview = app.overviewMatrix;

        const totalCount = instances.length;
        const withOpportunity = instances.filter((i) => i.opportunity).length;
        const coverage = totalCount > 0 ? (withOpportunity / totalCount) * 100 : 0;

        return {
            validTrades: {
                label: 'VALID TRADES',
                value: actionable.length.toString(),
                sub: `of ${setups.length} candidates`,
                color: actionable.length > 0 ? '#22c55e' : 'rgba(255,255,255,0.55)',
            },
            bestOpportunity: {
                label: 'BEST OPPORTUNITY',
                value: best?.symbol ?? '—',
                sub: best ? `score ${best.opportunityScore.toFixed(0)} · ${best.direction}` : 'no qualifying setup',
                color: best ? scoreColor(best.opportunityScore) : 'rgba(255,255,255,0.35)',
            },
            avgRr: {
                label: 'AVG R:R',
                value: formatRR(rr.avg),
                sub: rr.count > 0 ? `best ${formatRR(rr.best)} · ${rr.count} pair${rr.count === 1 ? '' : 's'}` : 'no R:R data',
                color: rrColor(rr.avg),
            },
            marketBias: {
                label: 'MARKET BIAS',
                value: (overview?.global_market_bias ?? 'Neutral').toString(),
                sub: overview ? `${(overview.breadth_pct ?? 0).toFixed(0)}% breadth` : 'local aggregation',
                color: biasColor(overview?.global_market_bias ?? 'Neutral'),
            },
            avgRisk: {
                label: 'AVG RISK',
                value: risk.count > 0 ? risk.avg.toFixed(0) : '—',
                sub: risk.count > 0 ? `across ${risk.count} pair${risk.count === 1 ? '' : 's'}` : 'no risk data',
                color: risk.avg >= 60 ? '#ef4444' : risk.avg >= 40 ? '#f59e0b' : '#22c55e',
            },
            coverage: {
                label: 'COVERAGE',
                value: `${coverage.toFixed(0)}%`,
                sub: `${withOpportunity}/${totalCount} pairs have opportunity data`,
                color: coverage >= 80 ? '#22c55e' : coverage >= 50 ? '#f59e0b' : '#ef4444',
            },
        };
    });
</script>

<div class={styles.strip}>
    {#each Object.values(kpis) as kpi (kpi.label)}
        <div class={styles.kpi}>
            <div class={styles.label}>{kpi.label}</div>
            <div class={styles.value} style="color: {kpi.color}">{kpi.value}</div>
            <div class={styles.sub}>{kpi.sub}</div>
        </div>
    {/each}
</div>
