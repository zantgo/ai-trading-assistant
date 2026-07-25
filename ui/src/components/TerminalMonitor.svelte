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
    import StructuralAnchorsStrip from './StructuralAnchorsStrip.svelte';
    import TradePlanStrip from './TradePlanStrip.svelte';
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
    import { buildMetricsExportJson, copyJsonToClipboard } from '../lib/metricsExport';
    import { deriveTradePlan } from '../lib/tradePlan';

    const app = useAppStore();
    let { pairKey }: { pairKey: string } = $props();
    const pair = $derived(app.instancesMap[pairKey]);
    const registry = $derived<IndicatorMeta[]>((app.indicatorRegistry ?? []) as IndicatorMeta[]);

    type TfLabel = 'Micro' | 'Fast' | 'Slow' | 'Macro';
    let activeTf: TfLabel = $state('Micro');

    // Phase 9: single source of truth — the backend's pipeline registry
    // (via WebSocket telemetry `barDurationSec`) is the canonical duration
    // for every timeframe. The fallback `??` guards only during the initial
    // boot interstice when the pair hasn't been streamed yet.
    const TIMEFRAMES = $derived.by((): { key: TfLabel; label: string; tfKey: string; secs: number | null }[] => {
        const p = pair;
        return [
            { key: 'Micro', label: 'Micro', tfKey: 'microTerm', secs: p?.microTerm?.barDurationSec ?? null },
            { key: 'Fast',  label: 'Fast',  tfKey: 'fastTerm',  secs: p?.fastTerm?.barDurationSec ?? null },
            { key: 'Slow',  label: 'Slow',  tfKey: 'slowTerm',  secs: p?.slowTerm?.barDurationSec ?? null },
            { key: 'Macro', label: 'Macro', tfKey: 'macroTerm', secs: p?.macroTerm?.barDurationSec ?? null },
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

    // ── Export JSON ──────────────────────────────────────────────────
    let exportLabel = $state('Export JSON');
    let exportTimer: ReturnType<typeof setTimeout> | null = null;

    async function handleExportJson() {
        if (!pair || !activeTfObj) return;
        const markPrice = parseFloat(activeTfObj.priceText ?? '') || 0;
        const microSnap = (pair as any)?.microTerm?.latestSnapshot ?? {};
        const opportunity = (microSnap as any)?.opportunity ?? null;
        const decisionContext = (microSnap as any)?.decision_context ?? null;
        const tradePlanVal = $derived.by(() => deriveTradePlan({
            symbol: pair.symbol,
            markPrice,
            opportunity,
            advisory: pair.advisory ?? null,
            analysis: pair.analysis ?? null,
            decisionContext,
            tf: activeTfObj,
            microTf: (pair as any)?.microTerm,
            overallRisk: pair.risk?.overall_risk?.score,
        }));
        const text = buildMetricsExportJson({
            symbol: pair.symbol,
            tfLabel: activeTfEntry.label,
            tfSecs: activeTfEntry.secs ?? 0,
            timestamp: snapshotTs,
            markPrice,
            registry,
            tf: activeTfObj,
            filters: {
                activeOnly: filters.activeOnly,
                confirmedPlusOnly: filters.confirmedPlusOnly,
                hideGates: filters.hideGates,
            },
            analysis: pair.analysis ?? null,
            risk: pair.risk ?? null,
            alignment: (pair.alignment as unknown as Record<string, unknown>) ?? null,
            opportunity,
            advisory: pair.advisory ?? null,
            volumeProfile: (pair as any)?.microTerm?.volumeProfile ?? null,
            liquidity: (pair as any)?.microTerm?.liquidity ?? null,
            cluster: (pair as any)?.microTerm?.cluster ?? null,
            liquiditySignals: ((pair as any)?.microTerm?.liquiditySignals ?? []) as any[],
            tradePlan: tradePlanVal,
            decisionContext,
        });
        const ok = await copyJsonToClipboard(text);
        exportLabel = ok ? 'Copied!' : 'Copy failed';
        if (exportTimer) clearTimeout(exportTimer);
        exportTimer = setTimeout(() => { exportLabel = 'Export JSON'; }, 2000);
    }
</script>

<div class={styles.monitor}>
    <div class={styles.tfSidebar}>
        <h3 class={styles.tfSidebarTitle}>TIMEFRAMES</h3>
        {#each TIMEFRAMES as tf (tf.key)}
            <button
                class="{styles.tfSidebarItem} {activeTf === tf.key ? styles.active : ''}"
                onclick={() => activeTf = tf.key}
            >
                <span class={styles.tfLabel}>{tf.label}</span>
                <span class={styles.tfSecs}>{tf.secs != null ? formatTimeframeLabel(tf.secs) : '—'}</span>
            </button>
        {/each}
    </div>

    <div class={styles.contentArea}>
        {#if pair && registry.length > 0 && activeTfObj}
            <!-- HEADER -->
            <div class={styles.header}>
                <span class={styles.title}>METRICS</span>
                <span class={styles.symbol}>{app.pairDisplayFor(pair.symbol)}</span>
                <span class={styles.tfBadge}>{activeTfEntry.label} · {activeTfEntry.secs != null ? formatTimeframeLabel(activeTfEntry.secs) : '—'}</span>
                <button
                    class={styles.exportBtn}
                    onclick={handleExportJson}
                    title="Copy current timeframe's indicators + signals as JSON"
                >
                    {exportLabel}
                </button>
            </div>

            <!-- ROW 1 — MarketContext -->
            <MarketContextStrip
                context={context ?? null}
                timestamp={snapshotTs}
                barDurationSec={activeTfEntry.secs ?? undefined}
            />

            <!-- ROW 2 — Trade Plan Strip (entry / TP1/TP2/TP3 / SL / R:R / confidence) -->
            <TradePlanStrip
                pair={pair}
                tf={activeTfObj}
                microTf={(pair as any)?.microTerm}
                risk={pair?.risk ?? null}
                markPrice={parseFloat(activeTfObj.priceText ?? '') || 0}
            />

            <!-- ROW 3 — Group Confluence Grid -->
            <GroupConfluenceGrid
                registry={registry}
                indicators={activeTfObj.indicators ?? {}}
                activeGroup={focusGroup}
                onGroupClick={handleGroupClick}
            />

            <!-- ROW 3.5 — Tier-1 Cascade Alert (conditional: only when SUSTAINED / DETECTED) -->
            {@const microFlow = (pair as any)?.microTerm?.liquidity ?? null}
            {#if microFlow && (microFlow.cascade_state === 'SUSTAINED' || microFlow.cascade_state === 'DETECTED')}
                <button
                    class="{styles.cascadeAlert} {microFlow.cascade_state === 'SUSTAINED' ? styles.cascadeAlertSustained : styles.cascadeAlertDetected}"
                    onclick={() => activeFacet = 'liquidity'}
                    title="Click to inspect the Liquidity facet"
                >
                    <span class={styles.cascadeAlertIcon}>⚠</span>
                    <span class={styles.cascadeAlertLabel}>
                        CASCADE {microFlow.cascade_state} · intensity {microFlow.cascade_intensity.toFixed(0)}/100 · click for Liquidity facet
                    </span>
                </button>
            {/if}

            <!-- ROW 4 — Structural Anchors Strip (Volume Profile / Fibonacci / Liquidity ladder) -->
            <StructuralAnchorsStrip
                tf={activeTfObj}
                microTf={(pair as any)?.microTerm}
                markPrice={parseFloat(activeTfObj.priceText ?? '') || 0}
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
