<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { useAppStore } from '../state.svelte';
    import styles from './StatePanel.module.css';
    import StageSection from './state/StageSection.svelte';
    import ConfluenceHero from './state/ConfluenceHero.svelte';
    import DecisionScorecard from './state/DecisionScorecard.svelte';
    import MtfMatrix from './state/MtfMatrix.svelte';
    import GateStatusStrip from './state/GateStatusStrip.svelte';
    import GroupSummaryPanel from './state/GroupSummaryPanel.svelte';
    import LevelHierarchy from './state/LevelHierarchy.svelte';
    import RegimeStrategyGates from './state/RegimeStrategyGates.svelte';
    import TelemetryTable from './TelemetryTable.svelte';
    import { groupIndicatorsByStage } from '../lib/decisionStages';
    import type { MonitorResponse, IndicatorMeta, TimeframeTelemetry } from '../types';

    const app = useAppStore();
    let { pairKey }: { pairKey: string } = $props();
    const pair = $derived(app.instancesMap[pairKey]);
    const registry = $derived<IndicatorMeta[]>((app.indicatorRegistry ?? []) as IndicatorMeta[]);
    const stageBuckets = $derived(groupIndicatorsByStage(registry));

    let monitor = $state<MonitorResponse | null>(null);
    let loading = $state(false);
    let timer: ReturnType<typeof setInterval> | null = null;

    async function fetchMonitor() {
        if (loading) return;
        loading = true;
        try {
            const res = await fetch(`/api/monitor?symbol=${encodeURIComponent(pairKey)}`);
            if (res.ok) monitor = await res.json();
        } catch (_) { /* transient */ }
        loading = false;
    }
    onMount(() => { fetchMonitor(); timer = setInterval(fetchMonitor, 5000); });
    onDestroy(() => { if (timer) clearInterval(timer); });

    type TfTab = { key: 'microTerm' | 'fastTerm' | 'slowTerm' | 'macroTerm'; label: string; mi: number };
    const TF_TABS: TfTab[] = [
        { key: 'microTerm', label: 'MICRO', mi: 0 },
        { key: 'fastTerm', label: 'FAST', mi: 1 },
        { key: 'slowTerm', label: 'SLOW', mi: 2 },
        { key: 'macroTerm', label: 'MACRO', mi: 3 },
    ];

    type ActiveTab = TfTab['key'] | 'confluence';
    let activeTab = $state<ActiveTab>('microTerm');
    let viewMode = $state<'stages' | 'matrix'>('stages');

    const activeTf = $derived<TfTab | null>(TF_TABS.find((t) => t.key === activeTab) ?? null);
    const activeMonitorTf = $derived(activeTf ? (monitor?.timeframes?.[activeTf.mi] ?? null) : null);
    const activeTelemetry = $derived<TimeframeTelemetry | null>(
        activeTf && pair ? ((pair as any)[activeTf.key] as TimeframeTelemetry) : null,
    );
    const priceRef = $derived(parseFloat(activeTelemetry?.priceText ?? '0') || 0);

    function scoreColor(score: number): string {
        const mag = Math.min(Math.abs(score) / 100, 1);
        if (mag >= 0.9) return '#a855f7';
        if (score > 5) return `rgb(16, ${Math.round(120 + 135 * mag)}, 129)`;
        if (score < -5) return `rgb(${Math.round(180 + 59 * mag)}, 68, 68)`;
        return '#94a3b8';
    }
    function verdictFor(mi: number) { return monitor?.timeframes?.[mi] ?? null; }
</script>

{#if pair}
<div class={styles.panel}>
    <!-- Persistent 4-timeframe verdict strip -->
    <div class={styles.verdictStrip}>
        {#each TF_TABS as t}
            {@const v = verdictFor(t.mi)}
            <button
                class="{styles.verdict} {activeTab === t.key ? styles.verdictActive : ''}"
                onclick={() => (activeTab = t.key)}
            >
                <span class={styles.verdictLabel}>{t.label}</span>
                <span class={styles.verdictScore} style="color:{scoreColor(v?.confluence_score ?? 0)}">
                    {(v?.confluence_score ?? 0) > 0 ? '+' : ''}{v?.confluence_score ?? 0}
                </span>
                <span class={styles.verdictRegime}>{v?.regime ?? '—'}</span>
            </button>
        {/each}
        <button
            class="{styles.verdict} {styles.verdictConfluence} {activeTab === 'confluence' ? styles.verdictActive : ''}"
            onclick={() => (activeTab = 'confluence')}
        >
            <span class={styles.verdictLabel}>CONFLUENCE</span>
            <span class={styles.verdictScore} style="color:{scoreColor(monitor?.mtf?.trend_agreement_pct ?? 0)}">
                {Math.round(monitor?.mtf?.trend_agreement_pct ?? 0)}%
            </span>
            <span class={styles.verdictRegime}>MTF AGREE</span>
        </button>
        <button class={styles.refreshBtn} onclick={fetchMonitor} title="Refresh">{loading ? '…' : '⟳'}</button>
    </div>

    {#if activeTab === 'confluence'}
        <!-- Cross-timeframe synthesis -->
        <div class={styles.confluenceView}>
            <MtfMatrix mtf={monitor?.mtf} />
            <GateStatusStrip {pairKey} />
            <RegimeStrategyGates {pairKey} regime={monitor?.timeframes?.[0]?.regime} confidence={activeTelemetry?.decisionContext?.regime_confidence} />
            <div class={styles.heroGrid}>
                {#each TF_TABS as t}
                    <div class={styles.heroCell}>
                        <div class={styles.heroCellLabel}>{t.label}</div>
                        <ConfluenceHero tf={verdictFor(t.mi)} position={pair.currentPosition} topN={3} />
                    </div>
                {/each}
            </div>
        </div>
    {:else}
        <!-- Per-timeframe decision lifecycle -->
        <div class={styles.tfHead}>
            <span class={styles.tfTitle}>{activeTf?.label} DECISION FLOW</span>
            <div class={styles.modeToggle}>
                <button class={viewMode === 'stages' ? styles.modeActive : ''} onclick={() => (viewMode = 'stages')}>STAGES</button>
                <button class={viewMode === 'matrix' ? styles.modeActive : ''} onclick={() => (viewMode = 'matrix')}>MATRIX</button>
            </div>
        </div>

        {#if viewMode === 'stages'}
            <div class={styles.stagesView}>
                <GroupSummaryPanel {pairKey} />
                <RegimeStrategyGates {pairKey} regime={activeMonitorTf?.regime} confidence={activeTelemetry?.decisionContext?.regime_confidence} />
                <LevelHierarchy {pairKey} />
                <!-- EXECUTION stage: synthesis (confluence + decision context) -->
                <div class={styles.entryStage}>
                    <div class={styles.entryLabel}>④ EXECUTION · SYNTHESIS</div>
                    <div class={styles.entryGrid}>
                        <ConfluenceHero tf={activeMonitorTf} position={pair.currentPosition} />
                        <DecisionScorecard dc={activeTelemetry?.decisionContext} />
                    </div>
                </div>

                <!-- Indicator stages: Setup → Trigger → Confirmation -->
                {#each stageBuckets as [stage, metas] (stage)}
                    <StageSection
                        {stage}
                        {metas}
                        map={activeTelemetry?.indicators}
                        {priceRef}
                        startOpen={stage === 'Setup' || stage === 'Trigger'}
                    />
                {/each}

                <!-- MONITORING stage: active trade surveillance -->
                <div class={styles.monitoringStage}>
                    <div class={styles.monitoringHeader}>
                        <span class={styles.monitoringStageTitle}>⑤ MONITORING</span>
                        <span class={styles.monitoringStageSubtitle}>Trade Management · Scale · Trailing · Exit</span>
                        <button
                            class={styles.monitoringJumpBtn}
                            onclick={() => {
                                const p = pair;
                                if (p) {
                                    p.currentView = 'monitoring';
                                    p.modeViews[p.currentLevel2Mode] = 'monitoring';
                                }
                            }}
                            title="Open full Monitoring panel"
                        >View Full Monitor ↗</button>
                    </div>
                </div>
            </div>
        {:else}
            <div class={styles.matrixView}>
                <TelemetryTable {pairKey} only={activeTf?.key} />
            </div>
        {/if}
    {/if}
</div>
{/if}
