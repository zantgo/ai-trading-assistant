<script lang="ts">
    // TerminalMonitor — Market Monitoring → Metrics view.
    //
    // Redesigned layout (Phase C of the metrics-ia-rebuild):
    //   Row 1: MarketContextStrip       — per-TF LOCAL synthesis (5 dimensions + regime + overall)
    //   Row 2: GroupConfluenceGrid      — 8 functional-group cards with directional bias summary
    //   Row 3: FacetTabs                — 6-tab strip (Indicators / Signals / Divergences / Levels / Liquidity / MTF)
    //   Row 4: Facet body               — pivots the same data through the chosen facet
    //
    // Header includes the timeframe selector (4 buttons: Micro / Fast / Slow / Macro).
    // Cross-cutting controls (search, filter pills) live above the facet tabs.

    import { useAppStore } from '../state.svelte';
    import type {
        IndicatorMeta, IndicatorSignal, TimeframeTelemetry,
        SignalKind, MarketContext,
    } from '../types';
    import { defaultFilters, filterSignals, type FilterState } from '../lib/filtering';
    import MarketContextStrip from './MarketContextStrip.svelte';
    import GroupConfluenceGrid from './GroupConfluenceGrid.svelte';
    import FacetTabs, { type FacetId } from './facets/FacetTabs.svelte';
    import IndicatorsView from './facets/IndicatorsView.svelte';
    import SignalsView from './facets/SignalsView.svelte';
    import DivergencesView from './facets/DivergencesView.svelte';
    import LevelsView from './facets/LevelsView.svelte';
    import LiquidityView from './facets/LiquidityView.svelte';
    import MtfView from './facets/MtfView.svelte';
    import styles from './TerminalMonitor.module.css';
    import SvgIcon from '../lib/SvgIcon.svelte';
    import { formatTimeframeLabel } from '../lib/telemetry';

    const app = useAppStore();
    let { pairKey }: { pairKey: string } = $props();
    const pair = $derived(app.instancesMap[pairKey]);
    const registry = $derived<IndicatorMeta[]>((app.indicatorRegistry ?? []) as IndicatorMeta[]);

    type TfLabel = 'Micro' | 'Fast' | 'Slow' | 'Macro';
    let activeTf: TfLabel = $state('Micro');

    const DEFAULT_TF_SECS = { Micro: 60, Fast: 180, Slow: 300, Macro: 900 } as const;

    const TIMEFRAMES = $derived.by((): { key: TfLabel; label: string; tfKey: string; secs: number }[] => {
        const p = pair;
        return [
            { key: 'Micro', label: 'Micro', tfKey: 'microTerm', secs: p?.microTerm?.barDurationSec ?? DEFAULT_TF_SECS.Micro },
            { key: 'Fast',  label: 'Fast',  tfKey: 'fastTerm',  secs: p?.fastTerm?.barDurationSec ?? DEFAULT_TF_SECS.Fast },
            { key: 'Slow',  label: 'Slow',  tfKey: 'slowTerm',  secs: p?.slowTerm?.barDurationSec ?? DEFAULT_TF_SECS.Slow },
            { key: 'Macro', label: 'Macro', tfKey: 'macroTerm', secs: p?.macroTerm?.barDurationSec ?? DEFAULT_TF_SECS.Macro },
        ];
    });

    const activeTfEntry = $derived(TIMEFRAMES.find((t) => t.key === activeTf)!);
    const activeTfObj = $derived<TimeframeTelemetry | undefined>(
        (pair as any)?.[activeTfEntry.tfKey] as TimeframeTelemetry | undefined,
    );

    // ── Facet state ───────────────────────────────────────────────────
    let activeFacet: FacetId = $state('indicators');
    let focusGroup: string | null = $state(null);

    // ── Filters ───────────────────────────────────────────────────────
    let filters: FilterState = $state(defaultFilters());

    function toggleActiveOnly() { filters = { ...filters, activeOnly: !filters.activeOnly }; }
    function toggleConfirmed() { filters = { ...filters, confirmedPlusOnly: !filters.confirmedPlusOnly }; }
    function toggleHideGates() { filters = { ...filters, hideGates: !filters.hideGates }; }
    function clearFilters() { filters = defaultFilters(); }

    // ── Per-facet counts ──────────────────────────────────────────────
    function countActiveSignals(): number {
        if (!activeTfObj) return 0;
        let n = 0;
        for (const k in activeTfObj.indicators ?? {}) {
            n += (activeTfObj.indicators[k]?.signals ?? []).length;
        }
        return n;
    }

    function countActiveDivergences(): number {
        if (!activeTfObj) return 0;
        let n = 0;
        for (const k in activeTfObj.indicators ?? {}) {
            for (const s of (activeTfObj.indicators[k]?.signals ?? []) as IndicatorSignal[]) {
                if (s.kind === 'Divergence') n++;
            }
        }
        return n;
    }

    function countActiveLevels(): number {
        if (!activeTfObj) return 0;
        let n = 0;
        for (const k in activeTfObj.indicators ?? {}) {
            for (const s of (activeTfObj.indicators[k]?.signals ?? []) as IndicatorSignal[]) {
                if (s.kind === 'LevelTest') n++;
            }
        }
        return n;
    }

    const facets = $derived.by(() => {
        const out: { id: FacetId; label: string; count?: number }[] = [
            { id: 'indicators',  label: 'Indicators' },
            { id: 'signals',     label: 'Signals',    count: countActiveSignals() },
            { id: 'divergences', label: 'Divergences',count: countActiveDivergences() },
            { id: 'levels',      label: 'Levels',     count: countActiveLevels() },
        ];
        if (pair?.microTerm?.liquidity || pair?.microTerm?.cluster) {
            out.push({ id: 'liquidity', label: 'Liquidity' });
        }
        out.push({ id: 'mtf', label: 'MTF' });
        return out;
    });

    function handleGroupClick(group: string) {
        // When a group card is clicked, switch to the Indicators facet and
        // focus that group so it expands and scrolls into view.
        activeFacet = 'indicators';
        focusGroup = group;
        // Clear focus after a tick so the same group can be re-focused.
        setTimeout(() => { focusGroup = null; }, 50);
    }

    // ── Header context extraction ─────────────────────────────────────
    const context = $derived<MarketContext | null | undefined>(activeTfObj?.context);
    const snapshotTs = $derived<number | null>(
        activeTfObj?.latestSnapshot && typeof (activeTfObj.latestSnapshot as any).timestamp === 'number'
            ? (activeTfObj.latestSnapshot as any).timestamp
            : null,
    );
</script>

<div class={styles.monitor}>
    <div class={styles.tfSidebar}>
        <h3 class={styles.tfSidebarTitle}>TIMEFRAMES</h3>
        {#each TIMEFRAMES as tf (tf.key)}
            <button
                class={styles.tfSidebarItem}
                class:active={activeTf === tf.key}
                onclick={() => activeTf = tf.key}
            >
                <span class={styles.tfLabel}>{tf.label}</span>
                <span class={styles.tfSecs}>{formatTimeframeLabel(tf.secs)}</span>
            </button>
        {/each}
    </div>

    <div class={styles.contentArea}>
        {#if pair && registry.length > 0 && activeTfObj}
            <!-- HEADER -->
            <div class={styles.header}>
                <span class={styles.title}>METRICS</span>
                <span class={styles.symbol}>{app.pairDisplayFor(pair.symbol)}</span>
                <span class={styles.tfBadge}>{activeTfEntry.label} · {formatTimeframeLabel(activeTfEntry.secs)}</span>
            </div>

            <!-- ROW 1 — MarketContext -->
            <MarketContextStrip
                context={context ?? null}
                timestamp={snapshotTs}
                barDurationSec={activeTfEntry.secs}
            />

            <!-- ROW 2 — Group Confluence Grid -->
            <GroupConfluenceGrid
                registry={registry}
                indicators={activeTfObj.indicators ?? {}}
                activeGroup={focusGroup}
                onGroupClick={handleGroupClick}
            />

            <!-- SEARCH + FILTER PILLS -->
            <div class={styles.controls}>
                <div class={styles.pillBar}>
                    <button
                        class="{styles.pill} {filters.activeOnly ? styles.pillActive : ''}"
                        onclick={toggleActiveOnly}
                    >
                        Active only
                    </button>
                    <button
                        class="{styles.pill} {filters.confirmedPlusOnly ? styles.pillActive : ''}"
                        onclick={toggleConfirmed}
                    >
                        Confirmed+
                    </button>
                    <button
                        class="{styles.pill} {filters.hideGates ? styles.pillActive : ''}"
                        onclick={toggleHideGates}
                    >
                        Hide gates
                    </button>
                    {#if filters.activeOnly || filters.confirmedPlusOnly || filters.hideGates}
                        <button class={styles.pillClear} onclick={clearFilters}>Clear</button>
                    {/if}
                </div>
            </div>

            <!-- ROW 3 — Facet Tabs -->
            <FacetTabs active={activeFacet} facets={facets} onChange={(id) => activeFacet = id} />

            <!-- ROW 4 — Facet Body -->
            <div class={styles.facetBody}>
                {#if activeFacet === 'indicators'}
                    <IndicatorsView
                        tf={activeTfObj}
                        registry={registry}
                        filters={filters}
                        focusGroup={focusGroup}
                    />
                {:else if activeFacet === 'signals'}
                    <SignalsView tf={activeTfObj} registry={registry} filters={filters} />
                {:else if activeFacet === 'divergences'}
                    <DivergencesView tf={activeTfObj} registry={registry} filters={filters} />
                {:else if activeFacet === 'levels'}
                    <LevelsView tf={activeTfObj} registry={registry} filters={filters} />
                {:else if activeFacet === 'liquidity'}
                    <LiquidityView pairKey={pairKey} />
                {:else if activeFacet === 'mtf'}
                    <MtfView
                        pair={{
                            microTerm: pair.microTerm,
                            fastTerm:  pair.fastTerm,
                            slowTerm:  pair.slowTerm,
                            macroTerm: pair.macroTerm,
                        }}
                        registry={registry}
                        filters={filters}
                    />
                {/if}
            </div>
        {:else}
            <div class={styles.featurePlaceholder}>
                <SvgIcon name="tableChart" size={64} />
                <h2 class={styles.featurePlaceholderTitle}>Market Metrics</h2>
                <p class={styles.featurePlaceholderMsg}>
                    Awaiting indicator registry and market data…
                </p>
            </div>
        {/if}
    </div>
</div>
