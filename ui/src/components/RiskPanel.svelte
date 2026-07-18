<script lang="ts">
    import type { RiskMatrix, RiskDimension } from '../types';
    import { useAppStore } from '../state.svelte';
    import styles from './RiskPanel.module.css';

    const app = useAppStore();
    let { pairKey } = $props<{ pairKey: string }>();

    const instance = $derived(app.instancesMap[pairKey]);
    const risk = $derived<RiskMatrix | null>(instance?.risk ?? null);

    function ringClass(l: string): string {
        switch (l) {
            case 'VERY_LOW': return styles.riskVeryLow;
            case 'LOW': return styles.riskLow;
            case 'MODERATE': return styles.riskModerate;
            case 'HIGH': return styles.riskHigh;
            case 'EXTREME': return styles.riskExtreme;
            default: return styles.riskModerate;
        }
    }
    function labelClass(l: string): string {
        switch (l) {
            case 'VERY_LOW': return styles.labelVeryLow;
            case 'LOW': return styles.labelLow;
            case 'MODERATE': return styles.labelModerate;
            case 'HIGH': return styles.labelHigh;
            case 'EXTREME': return styles.labelExtreme;
            default: return styles.labelModerate;
        }
    }
    function dimFillClass(score: number): string {
        if (score >= 80) return '#ef4444';
        if (score >= 60) return '#f87171';
        if (score >= 40) return '#f59e0b';
        if (score >= 20) return '#4ade80';
        return '#22c55e';
    }
    function dimLvlClass(l: string): string {
        return labelClass(l);
    }
    function shortName(full: string): string {
        return full.replace(/_/g, ' ').replace(/risk/i, '').trim();
    }

    const dimensions = $derived<{ name: string; data: RiskDimension }[]>(
        risk ? [
            { name: 'Market Risk', data: risk.market_risk },
            { name: 'Volatility Risk', data: risk.volatility_risk },
            { name: 'Exec Liquidity Risk', data: risk.execution_liquidity_risk ?? risk.market_risk },
            { name: 'Structure Risk', data: risk.structure_risk },
            { name: 'Momentum Risk', data: risk.momentum_risk },
            { name: 'Signal Risk', data: risk.signal_risk },
            { name: 'Execution Risk', data: risk.execution_risk },
            { name: 'Cascade Risk', data: risk.cascade_risk ?? risk.market_risk },
        ] : []
    );
</script>

<div class={styles.panel}>
    {#if !risk}
        <div class={styles.placeholder}>Awaiting risk assessment data...</div>
    {:else}
        <h2 class={styles.title}>Risk Assessment</h2>

        <div class={styles.section}>
            <div class={styles.overallRow}>
                <div class="{styles.overallMeter} {ringClass(risk.overall_risk.level)}">
                    <span class={styles.overallScore}>{risk.overall_risk.score.toFixed(0)}</span>
                </div>
                <div>
                    <span class="{styles.overallLabel} {labelClass(risk.overall_risk.level)}">
                        {risk.overall_risk.level}
                    </span>
                    <div style="margin-top:4px;font-size:10px;color:#64748b;">
                        Confidence: {(risk.overall_risk.confidence).toFixed(0)}%
                    </div>
                </div>
            </div>
        </div>

        <div class={styles.section}>
            <div class={styles.sectionTitle}>Risk Dimensions (0-100)</div>
            <div class={styles.dimList}>
                {#each dimensions as dim}
                    <div class={styles.dimRow}>
                        <span class={styles.dimName}>{dim.name}</span>
                        <div class={styles.dimBar}>
                            <div class={styles.dimFill}
                                 style="width: {dim.data.score.toFixed(1)}%; background: {dimFillClass(dim.data.score)}"></div>
                        </div>
                        <span class={styles.dimScore}>{dim.data.score.toFixed(0)}</span>
                        <span class="{styles.dimLevel} {dimLvlClass(dim.data.level)}">{dim.data.level}</span>
                    </div>
                    {#if dim.data.evidence && dim.data.evidence.length > 0}
                        <div class={styles.evidence}>
                            {#each dim.data.evidence as ev}
                                &bull; {ev}
                            {/each}
                        </div>
                    {/if}
                {/each}
            </div>
            <div class={styles.weights}>
                W: 0.14 Market + 0.14 Volatility + 0.14 ExecLiquid + 0.10 Structure + 0.14 Momentum + 0.10 Signal + 0.10 Execution + 0.14 Cascade
            </div>
        </div>
    {/if}
</div>