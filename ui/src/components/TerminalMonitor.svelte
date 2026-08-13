<script lang="ts">
    // TerminalMonitor — Market Monitoring → Metrics view.
    //
    // Per-TF L1 exploration tool: indicators, signals, context, and
    // structural anchors pivoted through 6 facets. The Trade Plan
    // (L4/L6 synthesis) has been moved to the Decision tab where it
    // belongs architecturally — this tab is pure per-TF observation.
    //
    // Layout:
    //   Row 1: MarketContextStrip       — per-TF LOCAL synthesis (5 dimensions + regime + overall)
    //   Row 2: GroupConfluenceGrid      — 8 functional-group cards with directional bias summary
    //   Row 3: StructuralAnchorsStrip   — Volume Profile / Fibonacci / Liquidity ladder
    //   Row 4: FacetTabs + body         — 6-tab pivoted exploration
    //
    // Header includes the timeframe selector.
    // Cross-cutting controls (search, filter pills) live above the facet tabs.

    import { useAppStore } from '../state.svelte';
    import type {
        IndicatorMeta, IndicatorSignal, TimeframeTelemetry,
        SignalKind, MarketContext,
    } from '../types';
    import type { WsState } from '../lib/websocket.svelte';
    import { defaultFilters, filterSignals, type FilterState } from '../lib/filtering';
    import MarketContextStrip from './MarketContextStrip.svelte';
    import GroupConfluenceGrid from './GroupConfluenceGrid.svelte';
    import StructuralAnchorsStrip from './StructuralAnchorsStrip.svelte';
    import FacetTabs, { type FacetId } from './facets/FacetTabs.svelte';
    import IndicatorsView from './facets/IndicatorsView.svelte';
    import SignalsView from './facets/SignalsView.svelte';
    import DivergencesView from './facets/DivergencesView.svelte';
    import LevelsView from './facets/LevelsView.svelte';
    import MtfView from './facets/MtfView.svelte';
    import LayerHeader from './LayerHeader.svelte';
    import { buildL1MetricsHeader, buildL1MtfHeader, type LayerHeaderSpec } from '../lib/layerHeader';
    import styles from './TerminalMonitor.module.css';
    import SvgIcon from '../lib/SvgIcon.svelte';
    import { formatTimeframeLabel } from '../lib/telemetry';
    import { buildMetricsTabExport } from '../lib/exportBuilders/metricsTab';
    import { buildMtfExportJson } from '../lib/exportBuilders/mtfTab';
    import ExportDataButton from './ExportDataButton.svelte';

    const app = useAppStore();
    let { pairKey, wssState }: { pairKey: string; wssState?: WsState } = $props();
    const pair = $derived(app.instancesMap[pairKey]);
    const registry = $derived<IndicatorMeta[]>((app.indicatorRegistry ?? []) as IndicatorMeta[]);

    type TfLabel = 'Mtf' | 'Micro' | 'Fast' | 'Slow' | 'Macro';
    let activeTf = $state<TfLabel>('Mtf');

    // Phase 9: single source of truth — the backend's pipeline registry
    // (via WebSocket telemetry `barDurationSec`) is the canonical duration
    // for every timeframe. The fallback `??` guards only during the initial
    // boot interstice when the pair hasn't been streamed yet.
    //
    // v7.0-prod (D7): the sidebar order is `MTF · MICRO · FAST · SLOW · MACRO`.
    // MTF is the cross-timeframe synthesis view, mounted at top of the rail
    // so first paint shows the operator the consolidated picture. The MTF
    // entry is a sentinel (tfKey='', secs=null); activeTfObj resolves to
    // `undefined` and the body switches to `MtfView` further down.
    const TIMEFRAMES = $derived.by((): { key: TfLabel; label: string; tfKey: string; secs: number | null }[] => {
        const p = pair;
        return [
            { key: 'Mtf',   label: 'MTF',   tfKey: '',            secs: null },
            { key: 'Micro', label: 'Micro', tfKey: 'microTerm',  secs: p?.microTerm?.barDurationSec ?? null },
            { key: 'Fast',  label: 'Fast',  tfKey: 'fastTerm',   secs: p?.fastTerm?.barDurationSec ?? null },
            { key: 'Slow',  label: 'Slow',  tfKey: 'slowTerm',   secs: p?.slowTerm?.barDurationSec ?? null },
            { key: 'Macro', label: 'Macro', tfKey: 'macroTerm',  secs: p?.macroTerm?.barDurationSec ?? null },
        ];
    });

    const activeTfEntry = $derived(
        activeTf === 'Mtf'
            ? { key: 'Mtf' as TfLabel, label: 'MTF', tfKey: '', secs: null }
            : TIMEFRAMES.find((t) => t.key === activeTf && t.key !== 'Mtf')!
    );
    const activeTfObj = $derived<TimeframeTelemetry | undefined>(
        activeTf === 'Mtf'
            ? undefined
            : ((pair as any)?.[activeTfEntry.tfKey] as TimeframeTelemetry | undefined)
    );

    // ── Facet state ───────────────────────────────────────────────────
    let activeFacet = $state<FacetId>('indicators');
    let focusGroup: string | null = $state(null);

    // ── Filters ───────────────────────────────────────────────────────
    let filters: FilterState = $state(defaultFilters());

    function toggleActiveOnly() { filters = { ...filters, activeOnly: !filters.activeOnly }; }
    function toggleConfirmed() { filters = { ...filters, confirmedPlusOnly: !filters.confirmedPlusOnly }; }
    function toggleHideGates() { filters = { ...filters, hideGates: !filters.hideGates }; }
    function toggleHideOverlays() { filters = { ...filters, hideOverlays: !filters.hideOverlays }; }
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
        // LIQUIDITY is consolidated into the Structural Anchors LIQUIDITY tile
        // (see `StructuralAnchorsStrip.svelte`) — the indicators-table facet
        // tab was removed to avoid the same data rendered in two containers.
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
    /// Single-TF export (existing behaviour — used when activeTf is Micro /
    /// Fast / Slow / Macro).
    function buildMetricsExport() {
        if (!pair || !activeTfObj) return null;
        const markPrice = parseFloat(activeTfObj.priceText ?? '') || 0;
        return buildMetricsTabExport({
            tf: activeTfObj,
            registry,
            volumeProfile: (activeTfObj as any)?.volumeProfile ?? null,
            microVolumeProfile: (pair as any)?.microTerm?.volumeProfile ?? null,
            liquidity: (activeTfObj as any)?.liquidity ?? null,
            microLiquidity: (pair as any)?.microTerm?.liquidity ?? null,
            cluster: (activeTfObj as any)?.cluster ?? null,
            liquiditySignals: (activeTfObj as any)?.liquiditySignals ?? [],
            // `pairKey` is the FULL exchange-symbol (e.g. BTC-USDC) — never
            // the bare base. This is the canonical `meta.pair` for every
            // export payload.
            symbol: pairKey,
            tfSecs: activeTfEntry.secs ?? null,
            timestamp: snapshotTs,
            markPrice,
            headerSpec,
            // EMA ribbon periods — single source of truth with the dashboard
            // settings UI (state.svelte.ts:419-422). Drives the `period` field
            // on each line of the `body.ema` block in the export JSON.
            configuredEmaPeriods: {
                ema_fast:   activeTfObj.emaFastVal   ?? app.settings.globalIndicatorsConfig.ema_fast,
                ema_medium: activeTfObj.emaMediumVal ?? app.settings.globalIndicatorsConfig.ema_medium,
                ema_slow:   activeTfObj.emaSlowVal   ?? app.settings.globalIndicatorsConfig.ema_slow,
                ema_long:   activeTfObj.emaLongVal   ?? app.settings.globalIndicatorsConfig.ema_long,
            },
            terms: {
                microTerm: pair.microTerm,
                fastTerm: pair.fastTerm,
                slowTerm: pair.slowTerm,
                macroTerm: pair.macroTerm,
            },
        });
    }

    /// Cross-timeframe export — used when the MTF sidebar item is active.
    /// Returns the MtfView-shaped payload (4 × N grid + agreement labels).
    function buildMtfExport() {
        if (!pair) return null;
        return buildMtfExportJson({
            // `pairKey` is the FULL exchange-symbol (e.g. BTC-USDC).
            symbol: pairKey,
            pair: {
                microTerm: pair.microTerm,
                fastTerm:  pair.fastTerm,
                slowTerm:  pair.slowTerm,
                macroTerm: pair.macroTerm,
            },
            registry,
            filters,
            markPrice: parseFloat(pair.microTerm?.priceText ?? '') || 0,
            tfSecs: activeTfEntry?.secs ?? null,
            timestamp: snapshotTs,
            headerSpec,
            terms: {
                microTerm: pair.microTerm,
                fastTerm: pair.fastTerm,
                slowTerm: pair.slowTerm,
                macroTerm: pair.macroTerm,
            },
        });
    }

    /// Header EXPORT DATA button routes between the two builders based on
    /// the active TF (single-TF vs MTF). Returns null when nothing is loaded.
    function buildHeaderExport(): string | null {
        if (activeTf === 'Mtf') return buildMtfExport();
        return buildMetricsExport();
    }

    // LayerHeader spec — single-TF reads the per-timeframe `tf.context`,
    // MTF switches to the synthetic cross-TF header.
    const headerSpec = $derived<LayerHeaderSpec>(
        activeTf === 'Mtf'
            ? buildL1MtfHeader(pair?.alignment ?? null, pair?.analysis?.market_regime ?? null)
            : buildL1MetricsHeader(activeTfObj ?? null)
    );
</script>

<div class={styles.monitor}>
    <div class={styles.tfSidebar}>
        <h3 class={styles.tfSidebarTitle}>TIMEFRAMES</h3>
        <!-- v7.0-prod (D7): MTF first, then MICRO · FAST · SLOW · MACRO.
             The MTF entry is part of TIMEFRAMES itself (sentinel with
             empty tfKey + secs=null); the body switches to MtfView when
             activeTf === 'Mtf'. -->
        {#each TIMEFRAMES as tf (tf.key)}
            <button
                class="{styles.tfSidebarItem} {activeTf === tf.key ? styles.active : ''}"
                onclick={() => activeTf = tf.key}
            >
                <span class={styles.tfLabel}>{tf.label}</span>
                <span class={styles.tfSecs}>{tf.secs != null ? formatTimeframeLabel(tf.secs) : (tf.key === 'Mtf' ? 'Multi-TF' : '—')}</span>
            </button>
        {/each}
    </div>

    <div class={styles.contentArea}>
        {#if pair && registry.length > 0 && (activeTf === 'Mtf' || activeTfObj)}
            <!-- L1 HEADER (v7.0-prod — shared chrome across all MME tabs) -->
            <LayerHeader spec={headerSpec}>
                {#snippet trailing()}
                    <span class={styles.symbol}>{app.pairDisplayFor(pair.symbol)}</span>
                    <span class={styles.tfBadge}>
                        {activeTf === 'Mtf'
                            ? 'MULTI-TIMEFRAME'
                            : `${activeTfEntry.label} · ${activeTfEntry.secs != null ? formatTimeframeLabel(activeTfEntry.secs) : '—'}`}
                    </span>
                    <ExportDataButton
                        onExport={buildHeaderExport}
                        title={activeTf === 'Mtf'
                            ? 'Copy the cross-timeframe grid as JSON'
                            : "Copy current timeframe's indicators + signals as JSON"}
                    />
                {/snippet}
            </LayerHeader>

            {#if activeTf === 'Mtf'}
                <!-- SEARCH + FILTER PILLS (Directly applied to the MTF Grid) -->
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
                        <button
                            class="{styles.pill} {filters.hideOverlays ? styles.pillActive : ''}"
                            onclick={toggleHideOverlays}
                            title="Hide price overlays / price levels / marker rows — they live on the chart and in the Structural Anchors Strip"
                        >
                            Hide overlays
                        </button>
                        {#if filters.activeOnly || filters.confirmedPlusOnly || filters.hideGates || filters.hideOverlays}
                            <button class={styles.pillClear} onclick={clearFilters}>Clear</button>
                        {/if}
                    </div>
                </div>

                <!-- Dedicated Cross-Timeframe Grid Workspace -->
                <div class={styles.facetBody}>
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
                </div>
            {:else if activeTfObj}
                <!-- SINGLE TIMEFRAME WORKSPACE -->

                <!-- ROW 1 — MarketContext -->
                <MarketContextStrip
                    context={context ?? null}
                    timestamp={snapshotTs}
                    barDurationSec={activeTfEntry.secs ?? undefined}
                    signalCount={countActiveSignals()}
                />

                <!-- ROW 2 — Group Confluence Grid -->
                <GroupConfluenceGrid
                    registry={registry}
                    indicators={activeTfObj.indicators ?? {}}
                    activeGroup={focusGroup}
                    onGroupClick={handleGroupClick}
                />

                <!-- ROW 2.5 — Tier-1 Cascade Alert (conditional: only when SUSTAINED / DETECTED) -->
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

                <!-- ROW 3 — Structural Anchors Strip -->
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
                        <button
                            class="{styles.pill} {filters.hideOverlays ? styles.pillActive : ''}"
                            onclick={toggleHideOverlays}
                            title="Hide price overlays / price levels / marker rows — they live on the chart and in the Structural Anchors Strip"
                        >
                            Hide overlays
                        </button>
                        {#if filters.activeOnly || filters.confirmedPlusOnly || filters.hideGates || filters.hideOverlays}
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
                    {/if}
                </div>
            {/if}
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
