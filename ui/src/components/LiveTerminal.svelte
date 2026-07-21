<script lang="ts">
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

    function tfKey(pairKey: string, tf: TimeframeTelemetry): string {
        return `${pairKey}-${tf.barDurationSec}-${tf.emaFastVal}-${tf.emaMediumVal}-${tf.emaSlowVal}-${tf.emaLongVal}-${tf.slot}`;
    }

    function toggleExpand(key: string) {
        expandedTf = expandedTf === key ? null : key;
    }

    function handleChartDblClick(chartType: string, slot: string, _timeframe: number) {
        app.openFullscreenChart(chartType, slot as 'micro' | 'fast' | 'slow' | 'macro', pairKey);
    }

    type SlotName = 'micro' | 'fast' | 'slow' | 'macro';

    function chartKey(t: TimeframeTelemetry, chartType: string): string {
        // Remount when slot, duration or any EMA param flips so the chart
        // picks up new config without leaking stale series.
        return `${pairKey}-${t.slot}-${chartType}-${t.barDurationSec}-${t.emaFastVal}-${t.emaMediumVal}-${t.emaSlowVal}-${t.emaLongVal}`;
    }

    /// One column of panes for a single slot. Each chart inside binds to the
    /// same `tf` reference so the WS stream and the chart components stay in
    /// lockstep without re-deriving slot from duration.
    function paneStack(t: TimeframeTelemetry): { chartType: string; box: string; component: 'Price' | 'Volume' | 'Rvol' | 'Macd' | 'Squeeze' | 'Rsi' | 'Adx' | 'Bbwp' | 'Atr' }[] {
        return [
            { chartType: 'price',   box: 'panePrice',   component: 'Price' },
            { chartType: 'volume',  box: 'paneVol',     component: 'Volume' },
            { chartType: 'rvol',    box: 'paneRvol',    component: 'Rvol' },
            { chartType: 'macd',    box: 'paneMacd',    component: 'Macd' },
            { chartType: 'squeeze', box: 'paneSqueeze', component: 'Squeeze' },
            { chartType: 'rsi',     box: 'paneRsi',     component: 'Rsi' },
            { chartType: 'adx',     box: 'paneAdx',     component: 'Adx' },
            { chartType: 'bbwp',    box: 'paneBbwp',    component: 'Bbwp' },
            { chartType: 'atr',     box: 'paneAtr',     component: 'Atr' },
        ];
    }

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
                        {#each paneStack(activeTerm) as pane (pane.chartType)}
                            <div class="{styles.panelBox} {styles[pane.box]}" data-pane-type={pane.chartType}>
                                <div class={styles.panelLabel}>{pane.chartType.toUpperCase()}</div>
                                {#key chartKey(activeTerm, pane.chartType)}
                                    {#if pane.component === 'Price'}
                                        <PriceChart pairKey={pairKey} slot={activeTerm.slot} onDoubleClick={() => handleChartDblClick('price', activeTerm.slot, activeTerm.barDurationSec)} />
                                    {:else if pane.component === 'Volume'}
                                        <VolumeChart pairKey={pairKey} slot={activeTerm.slot} onDoubleClick={() => handleChartDblClick('volume', activeTerm.slot, activeTerm.barDurationSec)} />
                                    {:else if pane.component === 'Rvol'}
                                        <RvolChart pairKey={pairKey} slot={activeTerm.slot} onDoubleClick={() => handleChartDblClick('rvol', activeTerm.slot, activeTerm.barDurationSec)} />
                                    {:else if pane.component === 'Macd'}
                                        <MacdChart pairKey={pairKey} slot={activeTerm.slot} onDoubleClick={() => handleChartDblClick('macd', activeTerm.slot, activeTerm.barDurationSec)} />
                                    {:else if pane.component === 'Squeeze'}
                                        <SqueezeChart pairKey={pairKey} slot={activeTerm.slot} onDoubleClick={() => handleChartDblClick('squeeze', activeTerm.slot, activeTerm.barDurationSec)} />
                                    {:else if pane.component === 'Rsi'}
                                        <RsiChart pairKey={pairKey} slot={activeTerm.slot} onDoubleClick={() => handleChartDblClick('rsi', activeTerm.slot, activeTerm.barDurationSec)} />
                                    {:else if pane.component === 'Adx'}
                                        <AdxChart pairKey={pairKey} slot={activeTerm.slot} onDoubleClick={() => handleChartDblClick('adx', activeTerm.slot, activeTerm.barDurationSec)} />
                                    {:else if pane.component === 'Bbwp'}
                                        <BbwpChart pairKey={pairKey} slot={activeTerm.slot} onDoubleClick={() => handleChartDblClick('bbwp', activeTerm.slot, activeTerm.barDurationSec)} />
                                    {:else if pane.component === 'Atr'}
                                        <AtrChart pairKey={pairKey} slot={activeTerm.slot} onDoubleClick={() => handleChartDblClick('atr', activeTerm.slot, activeTerm.barDurationSec)} />
                                    {/if}
                                {/key}
                            </div>
                        {/each}
                    </div>
                </div>
            </div>
        </div>

        {#if expandedTf === activeTf}
            <FullscreenToolbar onScreenshot={takeColumnScreenshot} onClose={closeExpanded} />
        {/if}
    {/if}
</div>
