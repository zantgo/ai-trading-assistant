<script lang="ts">
    // LiveTerminal — single-column chart stack driven by a typed `LIVE_PANES`
    // descriptor. The 8 always-visible panes (Price, Volume, MACD, RSI, ADX,
    // Squeeze, BBWP, ATR) match the institutional-quant workflow; secondary
    // oscillators (Stochastic, ChandeMO, OBV, CMF, MFI, Williams %R, CCI,
    // Force Index) and derivative panels (Funding, Open Interest, OI Delta,
    // Order-Flow+Depth) are grouped under collapsible accordion headers
    // (MOMENTUM OSCILLATORS / VOLUME FLOW / REGIME / DERIVATIVES DETAIL) so
    // the default screen stays readable.
    //
    // Layout:
    //   - ChartToggles (overlay pills)
    //   - Price pane (200px)
    //   - Derivative ribbon (if showDerivativeRibbon)
    //   - Volume pane (RVOL kept as color annotation, RVOL pane removed)
    //   - 7 always-on oscillator panes
    //   - Accordion groups
    import { useAppStore } from '../state.svelte';
    import styles from './LiveTerminal.module.css';
    import type { TimeframeTelemetry } from '../types';
    import ChartToggles from './ChartToggles.svelte';
    import PriceChart from './PriceChart.svelte';
    import VolumeChart from './VolumeChart.svelte';
    import RvolChart from './RvolChart.svelte';
    import MacdChart from './MacdChart.svelte';
    import SqueezeChart from './SqueezeChart.svelte';
    import RsiChart from './RsiChart.svelte';
    import AdxChart from './AdxChart.svelte';
    import BbwpChart from './BbwpChart.svelte';
    import AtrChart from './AtrChart.svelte';
    import StochasticChart from './StochasticChart.svelte';
    import ChandeMoChart from './ChandeMoChart.svelte';
    import WilliamsRChart from './WilliamsRChart.svelte';
    import CciChart from './CciChart.svelte';
    import ForceIndexChart from './ForceIndexChart.svelte';
    import ObvChart from './ObvChart.svelte';
    import CmfChart from './CmfChart.svelte';
    import MfiChart from './MfiChart.svelte';
    import HvChart from './HvChart.svelte';
    import AroonChart from './AroonChart.svelte';
    import ChoppinessChart from './ChoppinessChart.svelte';
    import LinRegSlopeChart from './LinRegSlopeChart.svelte';
    import ZScoreChart from './ZScoreChart.svelte';
    import FundingChart from './FundingChart.svelte';
    import OpenInterestChart from './OpenInterestChart.svelte';
    import OiDeltaChart from './OiDeltaChart.svelte';
    import OrderFlowDepthChart from './OrderFlowDepthChart.svelte';
    import DerivativeRibbon from './DerivativeRibbon.svelte';
    import PaneGroupHeader from './PaneGroupHeader.svelte';
    import FullscreenToolbar from './FullscreenToolbar.svelte';
    import { chartsWithin } from '../chartRegistry.svelte';
    import { composeChartScreenshots } from '../lib/chartScreenshot';

    const app = useAppStore();
    let { pairKey }: { pairKey: string } = $props();

    type TfKey = 'micro' | 'fast' | 'slow' | 'macro';
    type TfLabel = 'MICRO' | 'FAST' | 'SLOW' | 'MACRO';
    let activeTf: TfKey = $state('micro');

    let expandedTf = $state<string | null>(null);
    let expandedColumnEl = $state<HTMLDivElement | null>(null);

    function handleExpandedKeydown(e: KeyboardEvent) {
        if (e.key === 'Escape') {
            expandedTf = null;
        }
    }

    $effect(() => {
        if (expandedTf === null) return;
        window.addEventListener('keydown', handleExpandedKeydown);
        return () => window.removeEventListener('keydown', handleExpandedKeydown);
    });

    /// Format the `(suffix)` portion of a column header. Always pairs with the
    /// positional MICRO/FAST/SLOW/MACRO label from the column's slot.
    function durationSuffix(sec: number): string {
        if (sec >= 86400) return `${sec / 86400}d`;
        if (sec >= 3600) return `${sec / 3600}h`;
        if (sec >= 60) return `${sec / 60}m`;
        return `${sec}s`;
    }

    /// Column label = positional slot name + the duration suffix. The name
    /// is derived from `tf.slot`, never from duration bands, so the four
    /// columns always read MICRO/FAST/SLOW/MACRO left-to-right regardless of
    /// the user's chosen durations.
    function termLabel(name: 'MICRO' | 'FAST' | 'SLOW' | 'MACRO', tf: TimeframeTelemetry): string {
        return `${name} (${durationSuffix(tf.barDurationSec)})`;
    }

    function toggleExpand(key: string) {
        expandedTf = expandedTf === key ? null : key;
    }

    function handleChartDblClick(chartType: string, slot: string, _timeframe: number) {
        app.openFullscreenChart(chartType, slot as 'micro' | 'fast' | 'slow' | 'macro', pairKey);
    }

    function chartKey(t: TimeframeTelemetry, chartType: string): string {
        // Remount when slot, duration or any EMA param flips so the chart
        // picks up new config without leaking stale series.
        return `${pairKey}-${t.slot}-${chartType}-${t.barDurationSec}-${t.emaFastVal}-${t.emaMediumVal}-${t.emaSlowVal}-${t.emaLongVal}`;
    }

    /// Type-safe chart-type union — used by `LIVE_PANES`, `GROUPS`, and the
    /// dblClick dispatch in `FullscreenChartModal`. Adding a new pane = add
    /// one row here + one row in `LIVE_PANES` (or one of `GROUPS`).
    type ChartType =
        | 'price' | 'volume' | 'rvol'
        | 'macd' | 'rsi' | 'squeeze' | 'adx' | 'bbwp' | 'atr'
        | 'stochastic' | 'chandemo' | 'williams_r' | 'cci' | 'force_index'
        | 'obv' | 'cmf' | 'mfi' | 'hv'
        | 'aroon' | 'choppiness' | 'linreg' | 'zscore'
        | 'funding' | 'open_interest' | 'oi_delta' | 'order_flow_depth';

    interface PaneDescriptor {
        chartType: ChartType;
        box: string;
        component:
            | typeof PriceChart | typeof VolumeChart | typeof RvolChart
            | typeof MacdChart | typeof RsiChart | typeof SqueezeChart
            | typeof AdxChart | typeof BbwpChart | typeof AtrChart
            | typeof StochasticChart | typeof ChandeMoChart
            | typeof WilliamsRChart | typeof CciChart | typeof ForceIndexChart
            | typeof ObvChart | typeof CmfChart | typeof MfiChart | typeof HvChart
            | typeof AroonChart | typeof ChoppinessChart
            | typeof LinRegSlopeChart | typeof ZScoreChart
            | typeof FundingChart | typeof OpenInterestChart
            | typeof OiDeltaChart | typeof OrderFlowDepthChart;
    }

    // 8 always-on panes (compact default). RVOL intentionally omitted as a
    // dedicated pane — RVOL values still color Volume bars and are surfaced
    // via the inline RVOL numeric badge inside VolumeChart's snapshot.
    const LIVE_PANES: PaneDescriptor[] = [
        { chartType: 'price',   box: 'panePrice',   component: PriceChart },
        { chartType: 'volume',  box: 'paneVol',     component: VolumeChart },
        { chartType: 'macd',    box: 'paneMacd',    component: MacdChart },
        { chartType: 'rsi',     box: 'paneRsi',     component: RsiChart },
        { chartType: 'adx',     box: 'paneAdx',     component: AdxChart },
        { chartType: 'squeeze', box: 'paneSqueeze', component: SqueezeChart },
        { chartType: 'bbwp',    box: 'paneBbwp',    component: BbwpChart },
        { chartType: 'atr',     box: 'paneAtr',     component: AtrChart },
    ];

    interface PaneGroup {
        title: string;
        panes: PaneDescriptor[];
        defaultOpen?: boolean;
    }

    const MOMENTUM_GROUP: PaneGroup = {
        title: 'MOMENTUM OSCILLATORS',
        defaultOpen: false,
        panes: [
            { chartType: 'stochastic', box: 'paneStoch', component: StochasticChart },
            { chartType: 'chandemo',   box: 'paneChandeMo', component: ChandeMoChart },
            { chartType: 'williams_r', box: 'paneWilliamsR', component: WilliamsRChart },
            { chartType: 'cci',        box: 'paneCci', component: CciChart },
            { chartType: 'force_index', box: 'paneForceIndex', component: ForceIndexChart },
        ],
    };

    const VOLUME_GROUP: PaneGroup = {
        title: 'VOLUME FLOW',
        defaultOpen: false,
        panes: [
            { chartType: 'obv', box: 'paneObv', component: ObvChart },
            { chartType: 'cmf', box: 'paneCmf', component: CmfChart },
            { chartType: 'mfi', box: 'paneMfi', component: MfiChart },
            { chartType: 'hv',  box: 'paneHv',  component: HvChart },
        ],
    };

    const REGIME_GROUP: PaneGroup = {
        title: 'REGIME / CHOPPINESS',
        defaultOpen: false,
        panes: [
            { chartType: 'aroon',      box: 'paneAroon', component: AroonChart },
            { chartType: 'choppiness', box: 'paneChoppiness', component: ChoppinessChart },
            { chartType: 'linreg',     box: 'paneLinReg', component: LinRegSlopeChart },
            { chartType: 'zscore',     box: 'paneZScore', component: ZScoreChart },
        ],
    };

    const DERIVATIVES_GROUP: PaneGroup = {
        title: 'DERIVATIVES DETAIL',
        defaultOpen: false,
        panes: [
            { chartType: 'funding',         box: 'paneFunding', component: FundingChart },
            { chartType: 'open_interest',   box: 'paneOi',      component: OpenInterestChart },
            { chartType: 'oi_delta',        box: 'paneOiDelta', component: OiDeltaChart },
            { chartType: 'order_flow_depth', box: 'paneOfiDepth', component: OrderFlowDepthChart },
        ],
    };

    const COLLAPSED_GROUPS: PaneGroup[] = [
        MOMENTUM_GROUP,
        VOLUME_GROUP,
        REGIME_GROUP,
        DERIVATIVES_GROUP,
    ];

    /// Static descriptor for each sidebar entry. The label feeds the header
    /// (`termLabel`) and the secsFn keeps the duration live against the
    /// currently-streaming pair state.
    const TERMS: { key: TfKey; label: TfLabel; secsFn: (p: any) => number }[] = [
        { key: 'micro', label: 'MICRO', secsFn: (p) => p.microTerm.barDurationSec },
        { key: 'fast',  label: 'FAST',  secsFn: (p) => p.fastTerm.barDurationSec  },
        { key: 'slow',  label: 'SLOW',  secsFn: (p) => p.slowTerm.barDurationSec  },
        { key: 'macro', label: 'MACRO', secsFn: (p) => p.macroTerm.barDurationSec },
    ];

    function activeTermFor(p: any): TimeframeTelemetry {
        return p[activeTf === 'micro' ? 'microTerm'
              : activeTf === 'fast'  ? 'fastTerm'
              : activeTf === 'slow'  ? 'slowTerm'
              : 'macroTerm'];
    }

    function activeLabelFor(k: TfKey): TfLabel {
        return k === 'micro' ? 'MICRO'
             : k === 'fast'  ? 'FAST'
             : k === 'slow'  ? 'SLOW'
             : 'MACRO';
    }

    function takeColumnScreenshot() {
        if (!expandedColumnEl) return;
        const entries = chartsWithin(expandedColumnEl);
        if (entries.length === 0) return;
        const ordered = entries.map((e, idx) => ({
            label: `${idx + 1}. ${(e.container.getAttribute('data-pane-type') ?? 'chart').toUpperCase()}`,
            chart: e.chart,
        }));
        const tf = activeTermFor(app.instancesMap[pairKey]);
        const slot = activeLabelFor(activeTf);
        composeChartScreenshots(ordered, `${pairKey}_${slot.toLowerCase()}_${durationSuffix(tf.barDurationSec)}_column`);
    }

    function closeExpanded() {
        expandedTf = null;
    }
</script>

<div class={styles.terminalWorkspace}>
    {#if app.instancesMap[pairKey]}
        {@const pair = app.instancesMap[pairKey]}
        {@const activeTerm = activeTermFor(pair)}
        {@const activeLabel = activeLabelFor(activeTf)}

        <ChartToggles {pairKey} />
        <div class={styles.workspaceSidebar}>
            <aside class={styles.tfSidebar}>
                <h3 class={styles.tfSidebarTitle}>TIMEFRAMES</h3>
                {#each TERMS as t (t.key)}
                    <button
                        class="{styles.tfSidebarItem} {activeTf === t.key ? styles.tfSidebarItemActive : ''}"
                        onclick={() => activeTf = t.key}
                    >
                        <span class={styles.tfLabel}>{t.label}</span>
                        <span class={styles.tfSecs}>{durationSuffix(t.secsFn(pair))}</span>
                    </button>
                {/each}
            </aside>

            <div class={styles.singleColumn}>
                <div bind:this={expandedColumnEl} class="{styles.timescaleColumn} {expandedTf === activeTf ? styles.expandedTfColumn : ''}">
                    <div class={styles.timescaleHeader} class:styles.tfHeaderHidden={expandedTf === activeTf}>
                        <span class={styles.timescaleTitle}>{termLabel(activeLabel, activeTerm)}</span>
                        <div class={styles.headerActions}>
                            <span class={styles.timescalePrice}>{activeTerm.priceText}</span>
                            <button class={styles.expandBtn} onclick={() => toggleExpand(activeTf)} title={expandedTf === activeTf ? 'Collapse' : 'Expand'}>
                                {expandedTf === activeTf ? '✕' : '⛶'}
                            </button>
                        </div>
                    </div>
                    <div class={styles.timescaleCharts}>
                        {#each LIVE_PANES as pane (pane.chartType)}
                            <div class="{styles.panelBox} {styles[pane.box]}" data-pane-type={pane.chartType}>
                                <div class={styles.panelLabel}>{pane.chartType.toUpperCase()}</div>
                                {#key chartKey(activeTerm, pane.chartType)}
                                    {@const C = pane.component}
                                    <C
                                        {pairKey}
                                        slot={activeTerm.slot}
                                        onDoubleClick={() => handleChartDblClick(pane.chartType, activeTerm.slot, activeTerm.barDurationSec)}
                                    />
                                {/key}
                            </div>
                        {/each}

                        {#if activeTerm.showDerivativeRibbon}
                            {#key chartKey(activeTerm, 'derivative-ribbon')}
                                <DerivativeRibbon slot={activeTerm.slot} />
                            {/key}
                        {/if}

                        {#each COLLAPSED_GROUPS as group (group.title)}
                            <PaneGroupHeader title={group.title} count={group.panes.length} defaultOpen={group.defaultOpen ?? false}>
                                {#each group.panes as pane (pane.chartType)}
                                    <div class="{styles.panelBox} {styles[pane.box]} {styles.groupedPaneBox}" data-pane-type={pane.chartType}>
                                        <div class={styles.panelLabel}>{pane.chartType.toUpperCase()}</div>
                                        {#key chartKey(activeTerm, pane.chartType)}
                                            {@const C = pane.component}
                                            <C
                                                {pairKey}
                                                slot={activeTerm.slot}
                                                onDoubleClick={() => handleChartDblClick(pane.chartType, activeTerm.slot, activeTerm.barDurationSec)}
                                            />
                                        {/key}
                                    </div>
                                {/each}
                            </PaneGroupHeader>
                        {/each}

                        <!--
                            Hidden RVOL pane retained (off by default) so the
                            fullscreen modal can still open the RVOL chart by
                            URL/shortcut, preserving the legacy URL contract.
                        -->
                        <div class="{styles.panelBox} {styles.paneRvol} {styles.hiddenPane}" data-pane-type="rvol" aria-hidden="true" hidden>
                            <div class={styles.panelLabel}>RVOL</div>
                            {#key chartKey(activeTerm, 'rvol')}
                                <RvolChart {pairKey} slot={activeTerm.slot} />
                            {/key}
                        </div>
                    </div>
                </div>
            </div>
        </div>

        {#if expandedTf === activeTf}
            <FullscreenToolbar onScreenshot={takeColumnScreenshot} onClose={closeExpanded} />
        {/if}
    {/if}
</div>
