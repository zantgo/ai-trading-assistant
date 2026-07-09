<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import type { IndicatorMap, IndicatorMeta } from '../types';
    import type { FineCategory } from '../lib/decisionStages';
    import { stageForKey, categoryForKey } from '../lib/decisionStages';
    import GroupSummaryPanel from './state/GroupSummaryPanel.svelte';
    import GateStatusStrip from './state/GateStatusStrip.svelte';
    import IndicatorCard from './state/IndicatorCard.svelte';
    import styles from './WorkflowMetrics.module.css';

    const app = useAppStore();
    let { pairKey }: { pairKey: string } = $props();
    const pair = $derived(app.instancesMap[pairKey]);
    const tf = $derived(pair?.microTerm);
    const snap = $derived(tf?.latestSnapshot);
    const indicators = $derived((snap?.indicators ?? {}) as IndicatorMap);
    const registry = $derived((app.indicatorRegistry ?? []) as IndicatorMeta[]);
    const priceRef = $derived((snap?.current_price ?? 0) as number);

    // Group registry by category
    const groups: string[] = ['Trend', 'Momentum', 'Volume', 'Volatility', 'Structure', 'Regime', 'Institutional'];

    interface GroupSection {
        name: string;
        metas: IndicatorMeta[];
    }

    const groupSections = $derived<GroupSection[]>(
        groups.map((g) => {
            const metas = registry.filter((m) => m.group === g);
            return { name: g, metas };
        }).filter((s) => s.metas.length > 0),
    );

    let expandedGroups: Record<string, boolean> = $state({});
    function toggleGroup(name: string) {
        expandedGroups = { ...expandedGroups, [name]: !expandedGroups[name] };
    }
</script>

<div class={styles.container}>
    <!-- Gate Status Strip -->
    <div class={styles.gateSection}>
        <GateStatusStrip {pairKey} />
    </div>

    <!-- Group Summary -->
    <div class={styles.summarySection}>
        <GroupSummaryPanel {pairKey} />
    </div>

    <!-- Indicator Groups -->
    <div class={styles.groupsSection}>
        <div class={styles.sectionTitle}>INDICATORS BY CATEGORY</div>
        {#each groupSections as group}
            <div class={styles.groupBlock} data-expanded={expandedGroups[group.name] ?? false}>
                <button class={styles.groupHeader} onclick={() => toggleGroup(group.name)}>
                    <span class={styles.groupArrow}>{expandedGroups[group.name] ? '▼' : '▶'}</span>
                    <span class={styles.groupName}>{group.name}</span>
                    <span class={styles.groupCount}>{group.metas.length} indicators</span>
                </button>
                {#if expandedGroups[group.name]}
                    <div class={styles.groupCards}>
                        {#each group.metas as meta (meta.key)}
                            <IndicatorCard
                                {meta}
                                map={indicators}
                                category={categoryForKey(meta.key, meta.group) as FineCategory}
                                {priceRef}
                            />
                        {/each}
                    </div>
                {/if}
            </div>
        {/each}
    </div>
</div>
