<script lang="ts">
    // LiveTerminal — single-column chart stack. By user request, the
    // default state is "only the price chart visible": one PriceChart
    // (with its overlay toggles above) plus the always-on Derivative
    // Ribbon directly below it. All 27 indicator chart panes are
    // surfaced through 8 collapsible groups (PaneGroupHeader
    // accordions), default collapsed, ordered within each group by
    // importance to a quant trader. Selecting a group reveals the
    // panes inside that group; clicking the price chart's expand
    // button (⛶) still maximises the column for any pane.
    //
    // Order within each group is the top-of-list indicator first.
    // Charts inside an opened group are 90 px tall and share the
    // column with the price chart when that pane is pinned.
    import { useAppStore } from '../state.svelte';
    import styles from './LiveTerminal.module.css';
    import type { TimeframeTelemetry } from '../types';
    import ChartToggles from './ChartToggles.svelte';
    import PriceChart from './PriceChart.svelte';
    import VolumeChart from './VolumeChart.svelte';
    import RvolChart from './RvolChart.svelte';
    import AdxChart from './AdxChart.svelte';
    import SupertrendChart from './SupertrendChart.svelte';
    import IchimokuChart from './IchimokuChart.svelte';
    import HullMaChart from './HullMaChart.svelte';
    import AroonChart from './AroonChart.svelte';
    import PsarChart from './PsarChart.svelte';
    import DonchianChart from './DonchianChart.svelte';
    import RsiChart from './RsiChart.svelte';
    import MacdChart from './MacdChart.svelte';
    import StochasticChart from './StochasticChart.svelte';
    import ChandeMoChart from './ChandeMoChart.svelte';
    import WilliamsRChart from './WilliamsRChart.svelte';
    import CciChart from './CciChart.svelte';
    import AwesomeOscillatorChart from './AwesomeOscillatorChart.svelte';
    import ObvChart from './ObvChart.svelte';
    import CmfChart from './CmfChart.svelte';
    import MfiChart from './MfiChart.svelte';
    import ForceIndexChart from './ForceIndexChart.svelte';
    import AtrChart from './AtrChart.svelte';
    import BbwpChart from './BbwpChart.svelte';
    import SqueezeChart from './SqueezeChart.svelte';
    import HvChart from './HvChart.svelte';
    import StdDevChannelChart from './StdDevChannelChart.svelte';
    import ChoppinessChart from './ChoppinessChart.svelte';
    import LinRegSlopeChart from './LinRegSlopeChart.svelte';
    import ZScoreChart from './ZScoreChart.svelte';
    import KeltnerChart from './KeltnerChart.svelte';
    import FundingChart from './FundingChart.svelte';
    import OpenInterestChart from './OpenInterestChart.svelte';
    import OiDeltaChart from './OiDeltaChart.svelte';
    import OrderFlowDepthChart from './OrderFlowDepthChart.svelte';
    import SpreadChart from './SpreadChart.svelte';
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
        return `${pairKey}-${t.slot}-${chartType}-${t.barDurationSec}-${t.emaFastVal}-${t.emaMediumVal}-${t.emaSlowVal}-${t.emaLongVal}`;
    }

    /// Type-safe chart-type union. Adding a new pane = add one row here
    /// + add its entry to one of the groups below + add a branch to
    /// `FullscreenChartModal.svelte`.
    type ChartType =
        | 'price' | 'rvol' | 'volume'
        | 'adx' | 'supertrend' | 'ichimoku' | 'hull_ma' | 'aroon' | 'psar' | 'donchian'
        | 'rsi' | 'macd' | 'stochastic' | 'chandemo' | 'williams_r' | 'cci' | 'awesome'
        | 'obv' | 'cmf' | 'mfi' | 'force_index'
        | 'atr' | 'bbwp' | 'squeeze' | 'hv' | 'stddev_channel'
        | 'choppiness' | 'linreg' | 'zscore' | 'keltner'
        | 'funding' | 'open_interest' | 'oi_delta' | 'order_flow_depth' | 'spread';

    interface PaneDescriptor {
        chartType: ChartType;
        box: string;
        showFlag?: keyof TimeframeTelemetry;
        component: any;
    }

    /// 8 collapsible groups, ordered top-to-bottom in the column.
    /// All `defaultOpen: false` so the first paint is PriceChart-only.
    ///
    /// Within each group, panes are listed by quant-trader importance —
    /// the first entry is the most-cited / most-general indicator for
    /// that category. The previously-always-on panes (ADX / MACD / RSI
    /// / Squeeze / BBWP / ATR / RVOL / Volume) are distributed across
    /// groups so every pane is still one click away, but nothing
    /// crowds the first paint.
    const TREND_GROUP: PaneGroup = {
        title: 'TREND STRENGTH',
        panes: [
            { chartType: 'adx',        box: 'paneAdx',        component: AdxChart },
            { chartType: 'supertrend', box: 'paneSupertrend', component: SupertrendChart },
            { chartType: 'ichimoku',   box: 'paneIchimoku',   component: IchimokuChart },
            { chartType: 'hull_ma',    box: 'paneHullMa',     component: HullMaChart },
            { chartType: 'aroon',      box: 'paneAroon',      component: AroonChart },
            { chartType: 'psar',       box: 'panePsar',       component: PsarChart },
            { chartType: 'donchian',   box: 'paneDonchian',   component: DonchianChart },
        ],
    };

    const MOMENTUM_GROUP: PaneGroup = {
        title: 'MOMENTUM OSCILLATORS',
        panes: [
            { chartType: 'rsi',         box: 'paneRsi',         component: RsiChart },
            { chartType: 'macd',       box: 'paneMacd',        component: MacdChart },
            { chartType: 'stochastic',  box: 'paneStoch',       component: StochasticChart },
            { chartType: 'chandemo',    box: 'paneChandeMo',    component: ChandeMoChart },
            { chartType: 'williams_r',  box: 'paneWilliamsR',   component: WilliamsRChart },
            { chartType: 'cci',         box: 'paneCci',         component: CciChart },
            { chartType: 'awesome',     box: 'paneAwesome',     component: AwesomeOscillatorChart },
        ],
    };

    const VOLUME_GROUP: PaneGroup = {
        title: 'VOLUME FLOW',
        panes: [
            // RVOL replaces Volume at top of group — Volume is now reachable
            // only through its fullscreen modal / legacy URL.
            { chartType: 'rvol',        box: 'paneRvol',        component: RvolChart },
            { chartType: 'obv',         box: 'paneObv',         component: ObvChart },
            { chartType: 'cmf',         box: 'paneCmf',         component: CmfChart },
            { chartType: 'mfi',         box: 'paneMfi',         component: MfiChart },
            { chartType: 'force_index', box: 'paneForceIndex',  component: ForceIndexChart },
        ],
    };

    const VOLATILITY_GROUP: PaneGroup = {
        title: 'VOLATILITY',
        panes: [
            { chartType: 'atr',           box: 'paneAtr',          component: AtrChart },
            { chartType: 'bbwp',          box: 'paneBbwp',         component: BbwpChart },
            { chartType: 'squeeze',       box: 'paneSqueeze',      component: SqueezeChart },
            { chartType: 'hv',            box: 'paneHv',           component: HvChart },
            { chartType: 'stddev_channel', box: 'paneStdDevChannel', component: StdDevChannelChart },
        ],
    };

    const CONTEXT_GROUP: PaneGroup = {
        title: 'MARKET CONTEXT',
        panes: [
            { chartType: 'choppiness', box: 'paneChoppiness', component: ChoppinessChart },
            { chartType: 'linreg',     box: 'paneLinReg',     component: LinRegSlopeChart },
            { chartType: 'zscore',     box: 'paneZScore',     component: ZScoreChart },
            { chartType: 'keltner',    box: 'paneKeltner',    component: KeltnerChart },
        ],
    };

    const DERIVATIVES_GROUP: PaneGroup = {
        title: 'DERIVATIVES & DEPTH',
        panes: [
            { chartType: 'open_interest',   box: 'paneOi',         component: OpenInterestChart },
            { chartType: 'oi_delta',        box: 'paneOiDelta',   component: OiDeltaChart },
            { chartType: 'funding',         box: 'paneFunding',   component: FundingChart },
            { chartType: 'order_flow_depth', box: 'paneOfiDepth', component: OrderFlowDepthChart },
            { chartType: 'spread',          box: 'paneSpread',    component: SpreadChart },
        ],
    };

    interface PaneGroup {
        title: string;
        panes: PaneDescriptor[];
        defaultOpen?: boolean;
    }

    const COLLAPSED_GROUPS: PaneGroup[] = [
        TREND_GROUP,
        MOMENTUM_GROUP,
        VOLUME_GROUP,
        VOLATILITY_GROUP,
        CONTEXT_GROUP,
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
                        class="{styles.tfSidebarItem} {activeTf === t.key ? styles.active : ''}"
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
                        <!--
                            Price chart (always visible). All other panes
                            are inside collapsed accordion groups below.
                        -->
                        <div class="{styles.panelBox} {styles['panePrice']}" data-pane-type="price">
                            <div class={styles.panelLabel}>PRICE</div>
                            {#key chartKey(activeTerm, 'price')}
                                <PriceChart
                                    {pairKey}
                                    slot={activeTerm.slot}
                                    onDoubleClick={() => handleChartDblClick('price', activeTerm.slot, activeTerm.barDurationSec)}
                                />
                            {/key}
                        </div>

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
                            Volume (legacy) retained as a hidden mount so the
                            fullscreen modal and any URL shortcuts that call
                            `chartType === 'volume'` continue to work.
                        -->
                        <div class="{styles.panelBox} {styles.paneVol} {styles.hiddenPane}" data-pane-type="volume" aria-hidden="true" hidden>
                            <div class={styles.panelLabel}>VOLUME</div>
                            {#key chartKey(activeTerm, 'volume')}
                                <VolumeChart {pairKey} slot={activeTerm.slot} />
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
