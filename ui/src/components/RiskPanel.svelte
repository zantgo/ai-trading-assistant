<script lang="ts">
    import type { RiskMatrix, RiskDimension, RiskLevel, RiskState } from '../types';
    import { useAppStore } from '../state.svelte';
    import styles from './RiskPanel.module.css';

    const app = useAppStore();
    let { pairKey } = $props<{ pairKey: string }>();

    const instance = $derived(app.instancesMap[pairKey]);
    const risk = $derived<RiskMatrix | null>(instance?.risk ?? null);

    const LEVELS: RiskLevel[] = ['VeryLow', 'Low', 'Moderate', 'High', 'Extreme'];

    function levelRank(l: RiskLevel): number {
        return LEVELS.indexOf(l);
    }

    function levelClass(l: string): string {
        switch (l) {
            case 'VeryLow': return styles.riskVeryLow;
            case 'Low': return styles.riskLow;
            case 'Moderate': return styles.riskModerate;
            case 'High': return styles.riskHigh;
            case 'Extreme': return styles.riskExtreme;
            default: return styles.riskModerate;
        }
    }

    function labelClass(l: string): string {
        switch (l) {
            case 'VeryLow': return styles.labelVeryLow;
            case 'Low': return styles.labelLow;
            case 'Moderate': return styles.labelModerate;
            case 'High': return styles.labelHigh;
            case 'Extreme': return styles.labelExtreme;
            default: return styles.labelModerate;
        }
    }

    function fillClass(l: string): string {
        switch (l) {
            case 'VeryLow': return styles.fillVeryLow;
            case 'Low': return styles.fillLow;
            case 'Moderate': return styles.fillModerate;
            case 'High': return styles.fillHigh;
            case 'Extreme': return styles.fillExtreme;
            default: return styles.fillModerate;
        }
    }

    function stateClass(s: string): string {
        switch (s) {
            case 'Stable': return styles.stateStable;
            case 'Increasing': return styles.stateIncreasing;
            case 'Elevated': return styles.stateElevated;
            case 'Critical': return styles.stateCritical;
            case 'Improving': return styles.stateImproving;
            default: return styles.stateStable;
        }
    }

    function stateArrow(s: RiskState | string): string {
        switch (s) {
            case 'Stable': return '→';
            case 'Increasing': return '↗';
            case 'Elevated': return '↑';
            case 'Critical': return '⚠';
            case 'Improving': return '↘';
            default: return '→';
        }
    }

    type NamedDim = { name: string; key: string; weight: number; data: RiskDimension | undefined };

    const namedDims = $derived<NamedDim[]>(
        risk ? [
            { name: 'Market Risk', key: 'market_risk', weight: 0.14, data: risk.market_risk },
            { name: 'Volatility Risk', key: 'volatility_risk', weight: 0.14, data: risk.volatility_risk },
            { name: 'Exec Liquidity Risk', key: 'execution_liquidity_risk', weight: 0.14, data: risk.execution_liquidity_risk },
            { name: 'Structure Risk', key: 'structure_risk', weight: 0.10, data: risk.structure_risk },
            { name: 'Momentum Risk', key: 'momentum_risk', weight: 0.14, data: risk.momentum_risk },
            { name: 'Signal Risk', key: 'signal_risk', weight: 0.10, data: risk.signal_risk },
            { name: 'Execution Risk', key: 'execution_risk', weight: 0.10, data: risk.execution_risk },
            { name: 'Cascade Risk', key: 'cascade_risk', weight: 0.14, data: risk.cascade_risk },
        ] : []
    );

    const sortedDims = $derived(
        [...namedDims].sort((a, b) => {
            const sa = a.data?.score ?? -1;
            const sb = b.data?.score ?? -1;
            if (sb !== sa) return sb - sa;
            const la = a.data ? levelRank(a.data.level) : -1;
            const lb = b.data ? levelRank(b.data.level) : -1;
            return lb - la;
        })
    );

    const dimCounts = $derived.by(() => {
        const counts: Record<RiskLevel, number> = {
            VeryLow: 0, Low: 0, Moderate: 0, High: 0, Extreme: 0,
        };
        for (const d of namedDims) {
            if (d.data) counts[d.data.level]++;
        }
        return counts;
    });

    const headlineParts = $derived.by(() => {
        if (!risk) return '';
        const c = dimCounts;
        const bits: string[] = [];
        if (c.Extreme > 0) bits.push(`${c.Extreme} extreme`);
        if (c.High > 0) bits.push(`${c.High} high`);
        if (c.Moderate > 0) bits.push(`${c.Moderate} moderate`);
        if (bits.length === 0) bits.push('all dimensions calm');
        return `${bits.join(' · ')} · overall ${risk.overall_risk.level.toLowerCase()}`;
    });

    const topSeverity = $derived.by((): RiskLevel | null => {
        if (!risk) return null;
        const c = dimCounts;
        if (c.Extreme > 0) return 'Extreme';
        if (c.High > 0) return 'High';
        if (c.Moderate > 0) return 'Moderate';
        if (c.Low > 0) return 'Low';
        return 'VeryLow';
    });

    const ringRadius = 40;
    const ringCircumference = 2 * Math.PI * ringRadius;

    const symbolLabel = $derived.by(() => {
        if (!risk) return '';
        return instance?.symbol ?? pairKey;
    });

    const headTimestamp = $derived.by(() => {
        if (!instance?.microTerm?.latestSnapshot) return '';
        const snap = instance.microTerm.latestSnapshot as { timestamp?: number };
        const ts = snap?.timestamp;
        if (!ts) return '';
        const d = new Date(ts);
        if (isNaN(d.getTime())) return '';
        return d.toLocaleTimeString();
    });
</script>

<div class={styles.panel}>
    {#if !risk}
        <div class={styles.placeholder}>Awaiting risk assessment data...</div>
    {:else}
        <header class={styles.head}>
            <div class={styles.headTitleBlock}>
                <h2 class={styles.title}>Risk Assessment</h2>
                <div class={styles.headMeta}>
                    <span class={styles.headSymbol}>{symbolLabel}</span>
                    {#if headTimestamp}
                        <span class={styles.headDot}>·</span>
                        <span class={styles.headTime}>{headTimestamp}</span>
                    {/if}
                </div>
            </div>
            <div class={styles.headHeadline}>{headlineParts}</div>
        </header>

        <section class={styles.hero}>
            <div class={styles.ring}>
                <svg viewBox="0 0 96 96" class={styles.ringSvg}
                     role="img"
                     aria-label="Overall risk {risk.overall_risk.score.toFixed(0)} out of 100, {risk.overall_risk.level}">
                    <circle cx="48" cy="48" r={ringRadius} class={styles.ringTrack} />
                    <circle cx="48" cy="48" r={ringRadius}
                            class="{styles.ringProgress} {levelClass(risk.overall_risk.level)}"
                            stroke-dasharray={ringCircumference}
                            stroke-dashoffset={ringCircumference * (1 - Math.min(risk.overall_risk.score, 100) / 100)} />
                </svg>
                <div class={styles.ringCenter}>
                    <span class={styles.ringScore}>{risk.overall_risk.score.toFixed(0)}</span>
                    <span class={styles.ringUnit}>/ 100</span>
                </div>
            </div>
            <div class={styles.heroInfo}>
                <div class={styles.heroLevelRow}>
                    <span class="{styles.heroLevel} {labelClass(risk.overall_risk.level)}">
                        {risk.overall_risk.level.toUpperCase()}
                    </span>
                    {#if topSeverity && topSeverity !== risk.overall_risk.level}
                        <span class={styles.heroPeak}>
                            peak: <span class="{styles.heroPeakVal} {labelClass(topSeverity)}">{topSeverity}</span>
                        </span>
                    {/if}
                </div>
                <div class={styles.heroConf}>
                    <span class={styles.confLabel}>Confidence</span>
                    <div class={styles.confBar}>
                        <div class="{styles.confFill} {levelClass(risk.overall_risk.level)}"
                             style="width: {Math.min(risk.overall_risk.confidence, 100).toFixed(1)}%"></div>
                    </div>
                    <span class={styles.confVal}>{risk.overall_risk.confidence.toFixed(0)}%</span>
                </div>
                <p class={styles.heroHint}>
                    Lower is safer. State modifiers adjust each dimension's contribution but not the headline score.
                </p>
            </div>
        </section>

        <section class={styles.summary} aria-label="Dimension severity distribution">
            {#each LEVELS as l}
                {@const cls = labelClass(l)}
                {@const active = dimCounts[l] > 0}
                <div class="{styles.summaryTile} {cls} {active ? styles.summaryTileActive : ''}">
                    <span class={styles.summaryCount}>{dimCounts[l]}</span>
                    <span class={styles.summaryLabel}>{l === 'VeryLow' ? 'Very Low' : l}</span>
                </div>
            {/each}
        </section>

        <section class={styles.dimsSection}>
            <div class={styles.sectionTitle}>
                Risk Dimensions
                <span class={styles.sectionMeta}>sorted by severity</span>
            </div>
            <div class={styles.dimCards}>
                {#each sortedDims as dim (dim.key)}
                    {#if dim.data}
                        {@const sevClass = levelClass(dim.data.level)}
                        {@const levelFillCls = fillClass(dim.data.level)}
                        <article class="{styles.dimCard} {sevClass}" aria-label="{dim.name}: {dim.data.level}, score {dim.data.score}">
                            <header class={styles.dimHead}>
                                <div class={styles.dimNameBlock}>
                                    <span class={styles.dimName}>{dim.name}</span>
                                    <span class={styles.dimWeight}>{Math.round(dim.weight * 100)}% wt</span>
                                </div>
                                <div class={styles.dimBadges}>
                                    <span class="{styles.dimLevel} {labelClass(dim.data.level)}">{dim.data.level}</span>
                                    <span class="{styles.dimState} {stateClass(dim.data.state)}">
                                        <span class={styles.dimStateArrow}>{stateArrow(dim.data.state)}</span>
                                        <span>{dim.data.state.toUpperCase()}</span>
                                    </span>
                                </div>
                            </header>
                            <div class={styles.dimBarRow}>
                                <div class={styles.dimBar}>
                                    <div class="{styles.dimFill} {levelFillCls}"
                                         style="width: {Math.min(dim.data.score, 100).toFixed(1)}%"></div>
                                    <div class={styles.dimWeightMark}
                                         style="left: {(dim.weight * 100).toFixed(1)}%"
                                         title="Weight: {Math.round(dim.weight * 100)}%"></div>
                                </div>
                                <span class={styles.dimScore}>{dim.data.score.toFixed(0)}</span>
                                <span class={styles.dimConf}>{dim.data.confidence.toFixed(0)}%</span>
                            </div>
                            {#if dim.data.evidence && dim.data.evidence.length > 0}
                                <div class={styles.dimEvidence}>
                                    {#each dim.data.evidence as ev}
                                        <span class="{styles.evidenceChip} {sevClass}">{ev}</span>
                                    {/each}
                                </div>
                            {/if}
                        </article>
                    {:else}
                        <article class="{styles.dimCard} {styles.dimCardMissing}">
                            <header class={styles.dimHead}>
                                <div class={styles.dimNameBlock}>
                                    <span class={styles.dimName}>{dim.name}</span>
                                    <span class={styles.dimWeight}>{Math.round(dim.weight * 100)}% wt</span>
                                </div>
                                <span class="{styles.dimLevel} {styles.dimMissingBadge}">NOT ACTIVE</span>
                            </header>
                            <p class={styles.dimMissing}>Data feed inactive for this dimension.</p>
                        </article>
                    {/if}
                {/each}
            </div>
        </section>

        <details class={styles.disclosure}>
            <summary class={styles.disclosureSummary}>
                <span>How is overall risk computed?</span>
                <span class={styles.disclosureChevron}>›</span>
            </summary>
            <div class={styles.disclosureBody}>
                <div class={styles.weightGrid}>
                    {#each namedDims as d}
                        <div class={styles.weightChip}>
                            <span class={styles.weightLabel}>{d.name.replace(/ Risk$/, '')}</span>
                            <span class={styles.weightPct}>{(d.weight * 100).toFixed(0)}%</span>
                        </div>
                    {/each}
                </div>
                <p class={styles.disclosureNote}>
                    Overall risk is a weighted sum of the 8 dimension scores. State and confidence modify each
                    dimension's contribution, but do not alter the headline score directly.
                </p>
            </div>
        </details>
    {/if}
</div>
