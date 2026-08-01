<script lang="ts">
    import type { RiskMatrix, RiskDimension, RiskLevel, RiskState, LiquidationClusterMatrix, LiquidityFlow, TimeframeTelemetry } from '../types';
    import { useAppStore } from '../state.svelte';
    import { buildRiskTabExport } from '../lib/exportBuilders/riskTab';
    import ExportDataButton from './ExportDataButton.svelte';
    import styles from './RiskPanel.module.css';

    const app = useAppStore();
    let { pairKey } = $props<{ pairKey: string }>();

    const instance = $derived(app.instancesMap[pairKey]);
    const risk = $derived<RiskMatrix | null>(instance?.risk ?? null);
    const microTerm = $derived<TimeframeTelemetry | undefined>(instance?.microTerm);
    const microSnap = $derived(microTerm?.latestSnapshot as Record<string, unknown> | undefined);
    const opportunity = $derived((microSnap?.opportunity ?? null) as any);
    const decisionContext = $derived((microSnap?.decision_context ?? null) as Record<string, unknown> | null);
    const markPrice = $derived(parseFloat(microTerm?.priceText ?? '0') || 0);
    const timestamp = $derived<number | null>(
        microSnap && typeof (microSnap as any).timestamp === 'number'
            ? (microSnap as any).timestamp
            : null
    );
    const registry = $derived(app.indicatorRegistry ?? []);

    function buildExport() {
        return buildRiskTabExport({
            risk,
            flow: (microTerm as any)?.liquidity ?? null,
            cluster: (microTerm as any)?.cluster ?? null,
            symbol: pairKey,
            tfSecs: microTerm?.barDurationSec ?? null,
            timestamp,
            markPrice,
            filterState: {
                activeOnly: false,
                confirmedPlusOnly: false,
                hideGates: false,
                hideOverlays: false,
            },
        });
    }

    const LEVELS: RiskLevel[] = ['VeryLow', 'Low', 'Moderate', 'High', 'Extreme'];

    function levelRank(l: RiskLevel): number {
        return LEVELS.indexOf(l);
    }

    function normalizeLevel(l: string): string {
        if (!l) return 'Moderate';
        const pascal = l.toLowerCase().replace(/_/g, '');
        return pascal.charAt(0).toUpperCase() + pascal.slice(1);
    }

    function levelClass(l: string): string {
        const n = normalizeLevel(l);
        switch (n) {
            case 'Verylow': return styles.riskVeryLow;
            case 'Low': return styles.riskLow;
            case 'Moderate': return styles.riskModerate;
            case 'High': return styles.riskHigh;
            case 'Extreme': return styles.riskExtreme;
            default: return styles.riskModerate;
        }
    }

    function labelClass(l: string): string {
        const n = normalizeLevel(l);
        switch (n) {
            case 'Verylow': return styles.labelVeryLow;
            case 'Low': return styles.labelLow;
            case 'Moderate': return styles.labelModerate;
            case 'High': return styles.labelHigh;
            case 'Extreme': return styles.labelExtreme;
            default: return styles.labelModerate;
        }
    }

    function fillClass(l: string): string {
        const n = normalizeLevel(l).toLowerCase();
        switch (n) {
            case 'verylow': return styles.fillVeryLow;
            case 'low': return styles.fillLow;
            case 'moderate': return styles.fillModerate;
            case 'high': return styles.fillHigh;
            case 'extreme': return styles.fillExtreme;
            default: return styles.fillModerate;
        }
    }

    function stateClass(s: string): string {
        const n = s ? s.toLowerCase().replace(/_/g, '') : 'stable';
        switch (n) {
            case 'stable': return styles.stateStable;
            case 'increasing': return styles.stateIncreasing;
            case 'elevated': return styles.stateElevated;
            case 'critical': return styles.stateCritical;
            case 'improving': return styles.stateImproving;
            default: return styles.stateStable;
        }
    }

    function stateArrow(s: RiskState | string): string {
        const n = typeof s === 'string' ? s.toLowerCase().replace(/_/g, '') : 'stable';
        switch (n) {
            case 'stable': return '\u2192';
            case 'increasing': return '\u2197';
            case 'elevated': return '\u2191';
            case 'critical': return '\u26A0';
            case 'improving': return '\u2198';
            default: return '\u2192';
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
        const counts: Record<string, number> = {
            verylow: 0, low: 0, moderate: 0, high: 0, extreme: 0,
        };
        for (const d of namedDims) {
            if (d.data) {
                const key = d.data.level;
                const nk = key ? key.toLowerCase().replace(/_/g, '') : 'moderate';
                if (nk in counts) counts[nk]++;
            }
        }
        return counts;
    });

    const headlineParts = $derived.by(() => {
        if (!risk) return 'Risk assessment data forming — all dimensions will populate once indicators stabilize';
        const c = dimCounts;
        const bits: string[] = [];
        if (c.extreme > 0) bits.push(`${c.extreme} extreme`);
        if (c.high > 0) bits.push(`${c.high} high`);
        if (c.moderate > 0) bits.push(`${c.moderate} moderate`);
        if (bits.length > 0) {
            return `${bits.join(' \u00B7 ')} \u00B7 overall ${risk.overall_risk.level.toLowerCase().replace(/_/g, ' ')}`;
        }
        return `all dimensions calm \u00B7 overall ${risk.overall_risk.level.toLowerCase().replace(/_/g, ' ')}`;
    });

    const topSeverity = $derived.by((): RiskLevel | null => {
        if (!risk) return null;
        const c = dimCounts;
        if (c.extreme > 0) return 'Extreme';
        if (c.high > 0) return 'High';
        if (c.moderate > 0) return 'Moderate';
        if (c.low > 0) return 'Low';
        return 'VeryLow';
    });

    // ── Cascade telemetry from per-TF snapshot ──
    const cascadeFlow = $derived(microSnap?.liquidity as LiquidityFlow | undefined);
    const cascadeCluster = $derived(microSnap?.cluster as LiquidationClusterMatrix | undefined);

    function cascadeStateLabel(state: string | undefined): string {
        if (!state || state === 'None') return '\u2014';
        return state;
    }

    function cascadeAsymmetryText(asym: number | undefined): string {
        if (asym == null || !isFinite(asym)) return '\u2014';
        const pct = (asym * 100).toFixed(1);
        if (asym > 0) return `\u2191${pct}% (short squeeze)`;
        if (asym < 0) return `\u2193${pct}% (long cascade)`;
        return `0.0% (balanced)`;
    }

    const ringRadius = 40;
    const ringCircumference = 2 * Math.PI * ringRadius;
</script>

<div class={styles.panel}>
    <header class={styles.head}>
        <div class={styles.headTitleBlock}>
            <h2 class={styles.title}>Risk Assessment</h2>
            <div class={styles.headHeadline}>{headlineParts}</div>
        </div>
        <ExportDataButton onExport={buildExport} title="Copy all Risk data as JSON" />
    </header>

    {#if !risk}
        <div class={styles.noDataBanner}>Risk assessment engine initializing — the dashboard skeleton shows all dimensions that will populate once market data stabilizes.</div>
    {/if}

    <!-- ── Hero: ring + info ── -->
    <section class={styles.hero}>
        <div class={styles.ring}>
            <svg viewBox="0 0 96 96" class={styles.ringSvg}
                 role="img"
                 aria-label="Overall risk {risk ? risk.overall_risk.score.toFixed(0) : '0'} out of 100">
                <circle cx="48" cy="48" r={ringRadius} class={styles.ringTrack} />
                <circle cx="48" cy="48" r={ringRadius}
                        class="{styles.ringProgress} {risk ? levelClass(risk.overall_risk.level) : ''}"
                        stroke-dasharray={ringCircumference}
                        stroke-dashoffset={ringCircumference * (1 - Math.min(risk?.overall_risk?.score ?? 0, 100) / 100)} />
            </svg>
            <div class={styles.ringCenter}>
                <span class={styles.ringScore}>{risk ? risk.overall_risk.score.toFixed(0) : '\u2014'}</span>
                <span class={styles.ringUnit}>{risk ? '/ 100' : ''}</span>
            </div>
        </div>
        <div class={styles.heroInfo}>
            <div class={styles.heroLevelRow}>
                <span class="{styles.heroLevel} {risk ? labelClass(risk.overall_risk.level) : styles.labelNoData}">
                    {risk ? risk.overall_risk.level.replace(/_/g, ' ').toUpperCase().replace(/\b\w/g, (c: string) => c.toUpperCase()) : 'NO DATA'}
                </span>
                {#if topSeverity && risk && topSeverity !== risk.overall_risk.level}
                    <span class={styles.heroPeak}>
                        peak: <span class="{styles.heroPeakVal} {labelClass(topSeverity)}">{topSeverity}</span>
                    </span>
                {/if}
            </div>
            <div class={styles.heroConf}>
                <span class={styles.confLabel}>Confidence</span>
                <div class={styles.confBar}>
                    <div class="{styles.confFill} {risk ? levelClass(risk.overall_risk.level) : ''}"
                         style="width: {risk ? Math.min(risk.overall_risk.confidence, 100).toFixed(1) : '0'}%"></div>
                </div>
                <span class={styles.confVal}>{risk ? risk.overall_risk.confidence.toFixed(0) : '\u2014'}%</span>
            </div>
            <p class={styles.heroHint}>
                Lower is safer. State modifiers adjust each dimension's contribution but not the headline score.
            </p>
        </div>
    </section>

    <!-- ── Summary tiles ── -->
    <section class={styles.summary} aria-label="Dimension severity distribution">
        {#each LEVELS as l}
            {@const lk = l.toLowerCase().replace(/_/g, '')}
            {@const active = dimCounts[lk] > 0}
            <div class="{styles.summaryTile} {active ? styles.summaryTileActive : ''}">
                <span class={styles.summaryCount}>{dimCounts[lk]}</span>
                <span class={styles.summaryLabel}>{l === 'VeryLow' ? 'Very Low' : l}</span>
            </div>
        {/each}
    </section>

    <!-- ── Dimension cards ── -->
    <section class={styles.dimsSection}>
        <div class={styles.sectionTitle}>
            Risk Dimensions
            {#if sortedDims.length > 0}
                <span class={styles.sectionMeta}>sorted by severity</span>
            {/if}
        </div>
        {#if sortedDims.length > 0}
            <div class={styles.dimCards}>
                {#each sortedDims as dim (dim.key)}
                    {#if dim.data}
                        {@const levelFillCls = fillClass(dim.data.level)}
                        <article class={styles.dimCard} aria-label="{dim.name}: {dim.data.level}, score {dim.data.score}">
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
                                        <span class={styles.evidenceChip}>{ev}</span>
                                    {/each}
                                </div>
                            {:else if dim.data.level === 'High' || dim.data.level === 'Extreme' || dim.data.level === 'HIGH' || dim.data.level === 'EXTREME'}
                                <div class={styles.dimEvidence}>
                                    <span class={styles.evidenceChip}>No evidence recorded</span>
                                </div>
                            {/if}

                            {#if dim.key === 'cascade_risk' && (cascadeFlow || cascadeCluster)}
                                <div class={styles.cascadeExtra}>
                                    {#if cascadeFlow?.cascade_state}
                                        <span class={styles.cascadeField}>
                                            <span class={styles.cascadeFieldLabel}>State</span>
                                            <span class={styles.cascadeFieldValue}>{cascadeStateLabel((cascadeFlow as any)?.cascade_state)}</span>
                                        </span>
                                    {/if}
                                    {#if cascadeFlow?.cascade_intensity != null}
                                        <span class={styles.cascadeField}>
                                            <span class={styles.cascadeFieldLabel}>Intensity</span>
                                            <span class={styles.cascadeFieldValue}>{(cascadeFlow.cascade_intensity).toFixed(1)}</span>
                                        </span>
                                    {/if}
                                    {#if cascadeCluster?.cascade_asymmetry != null}
                                        <span class={styles.cascadeField}>
                                            <span class={styles.cascadeFieldLabel}>Asymmetry</span>
                                            <span class={styles.cascadeFieldValue}>{cascadeAsymmetryText(cascadeCluster.cascade_asymmetry)}</span>
                                        </span>
                                    {/if}
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
        {:else}
            <div class={styles.dimCards}>
                {#each [
                    { name: 'Market Risk', weight: 0.14 },
                    { name: 'Volatility Risk', weight: 0.14 },
                    { name: 'Exec Liquidity Risk', weight: 0.14 },
                    { name: 'Structure Risk', weight: 0.10 },
                    { name: 'Momentum Risk', weight: 0.14 },
                    { name: 'Signal Risk', weight: 0.10 },
                    { name: 'Execution Risk', weight: 0.10 },
                    { name: 'Cascade Risk', weight: 0.14 },
                ] as dim}
                    <article class="{styles.dimCard} {styles.dimCardMissing}">
                        <header class={styles.dimHead}>
                            <div class={styles.dimNameBlock}>
                                <span class={styles.dimName}>{dim.name}</span>
                                <span class={styles.dimWeight}>{Math.round(dim.weight * 100)}% wt</span>
                            </div>
                            <span class="{styles.dimLevel} {styles.dimMissingBadge}">AWAITING</span>
                        </header>
                        <p class={styles.dimMissing}>Awaiting risk assessment — this dimension will populate once market data stabilizes.</p>
                    </article>
                {/each}
            </div>
        {/if}
    </section>

    <!-- ── Interpretation ── -->
    <div class={styles.section}>
        <div class={styles.sectionTitle}>Risk Summary</div>
        <div class={styles.interpretation}>
            {#if risk}
                {#if dimCounts.extreme > 0 || dimCounts.high > 0}
                    <strong>Elevated risk environment.</strong>
                    {dimCounts.extreme > 0 ? ` ${dimCounts.extreme} dimension${dimCounts.extreme > 1 ? 's' : ''} at extreme levels.` : ''}
                    {dimCounts.high > 0 ? ` ${dimCounts.high} dimension${dimCounts.high > 1 ? 's' : ''} at high levels.` : ''}
                    {' '}Consider reduced position sizing and wider stops. Monitor the highest-severity dimensions for evidence of improvement before committing capital.
                {:else if dimCounts.moderate > 0}
                    <strong>Moderate risk environment.</strong> {dimCounts.moderate} dimension{dimCounts.moderate > 1 ? 's' : ''} at moderate levels.
                    {' '}Standard position sizing applies, but stay alert to dimensions trending toward higher severity.
                {:else}
                    <strong>Low risk environment.</strong> All dimensions are within acceptable bounds.
                    {' '}Favorable conditions for disciplined execution with standard risk parameters.
                {/if}
                {' '}Overall composite score is <strong>{risk.overall_risk.level.toLowerCase().replace(/_/g, ' ')}</strong> at {risk.overall_risk.confidence.toFixed(0)}% confidence.
            {:else}
                Risk synthesis is initializing — this section will provide a human-readable summary of the overall risk environment, highlighting which dimensions require attention and suggesting position-sizing guidance.
            {/if}
        </div>
    </div>

    <!-- ── Disclosure ── -->
    <details class={styles.disclosure}>
        <summary class={styles.disclosureSummary}>
            <span>How is overall risk computed?</span>
            <span class={styles.disclosureChevron}>{(v) => '\u203A'}</span>
        </summary>
        <div class={styles.disclosureBody}>
            <div class={styles.weightGrid}>
                {#each [
                    { label: 'Market', pct: 14 },
                    { label: 'Volatility', pct: 14 },
                    { label: 'ExecLiq', pct: 14 },
                    { label: 'Structure', pct: 10 },
                    { label: 'Momentum', pct: 14 },
                    { label: 'Signal', pct: 10 },
                    { label: 'Execution', pct: 10 },
                    { label: 'Cascade', pct: 14 },
                ] as d}
                    <div class={styles.weightChip}>
                        <span class={styles.weightLabel}>{d.label}</span>
                        <span class={styles.weightPct}>{d.pct}%</span>
                    </div>
                {/each}
            </div>
            <p class={styles.disclosureNote}>
                Overall risk is a weighted sum of the 8 dimension scores. State and confidence modify each
                dimension's contribution, but do not alter the headline score directly.
            </p>
        </div>
    </details>
</div>
