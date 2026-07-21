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

    const app = useAppStore();
    let { pairKey }: { pairKey: string } = $props();
    let showMicro = $state(true);
    let showFast = $state(true);
    let showSlow = $state(true);
    let showMacro = $state(true);

    let expandedTf = $state<string | null>(null);

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
</script>

<div class={styles.terminalWorkspace}>
    {#if app.instancesMap[pairKey]}
        {@const pair = app.instancesMap[pairKey]}

        <ChartToggles {pairKey} />
        <div class={styles.mtfGrid}>
        <!-- Micro-Term Column -->
        <div class="{styles.timescaleColumn} {!showMicro ? styles.hiddenPane : ''} {expandedTf === 'micro' ? styles.expandedTfColumn : ''}">
            <div class={styles.timescaleHeader} class:styles.tfHeaderHidden={expandedTf === 'micro'}>
                <span class={styles.timescaleTitle}>{termLabel('MICRO', pair.microTerm)}</span>
                <div class={styles.headerActions}>
                    <span class={styles.timescalePrice}>{pair.microTerm.priceText}</span>
                    <button class={styles.expandBtn} onclick={() => toggleExpand('micro')} title={expandedTf === 'micro' ? 'Collapse' : 'Expand'}>
                        {expandedTf === 'micro' ? '✕' : '⛶'}
                    </button>
                </div>
            </div>
            <div class={styles.timescaleCharts}>
                {#each paneStack(pair.microTerm) as pane (pane.chartType)}
                    <div class="{styles.panelBox} {styles[pane.box]}">
                        <div class={styles.panelLabel}>{pane.chartType.toUpperCase()}</div>
                        {#key chartKey(pair.microTerm, pane.chartType)}
                            {#if pane.component === 'Price'}
                                <PriceChart pairKey={pairKey} slot={pair.microTerm.slot} onDoubleClick={() => handleChartDblClick('price', pair.microTerm.slot, pair.microTerm.barDurationSec)} />
                            {:else if pane.component === 'Volume'}
                                <VolumeChart pairKey={pairKey} slot={pair.microTerm.slot} onDoubleClick={() => handleChartDblClick('volume', pair.microTerm.slot, pair.microTerm.barDurationSec)} />
                            {:else if pane.component === 'Rvol'}
                                <RvolChart pairKey={pairKey} slot={pair.microTerm.slot} onDoubleClick={() => handleChartDblClick('rvol', pair.microTerm.slot, pair.microTerm.barDurationSec)} />
                            {:else if pane.component === 'Macd'}
                                <MacdChart pairKey={pairKey} slot={pair.microTerm.slot} onDoubleClick={() => handleChartDblClick('macd', pair.microTerm.slot, pair.microTerm.barDurationSec)} />
                            {:else if pane.component === 'Squeeze'}
                                <SqueezeChart pairKey={pairKey} slot={pair.microTerm.slot} onDoubleClick={() => handleChartDblClick('squeeze', pair.microTerm.slot, pair.microTerm.barDurationSec)} />
                            {:else if pane.component === 'Rsi'}
                                <RsiChart pairKey={pairKey} slot={pair.microTerm.slot} onDoubleClick={() => handleChartDblClick('rsi', pair.microTerm.slot, pair.microTerm.barDurationSec)} />
                            {:else if pane.component === 'Adx'}
                                <AdxChart pairKey={pairKey} slot={pair.microTerm.slot} onDoubleClick={() => handleChartDblClick('adx', pair.microTerm.slot, pair.microTerm.barDurationSec)} />
                            {:else if pane.component === 'Bbwp'}
                                <BbwpChart pairKey={pairKey} slot={pair.microTerm.slot} onDoubleClick={() => handleChartDblClick('bbwp', pair.microTerm.slot, pair.microTerm.barDurationSec)} />
                            {:else if pane.component === 'Atr'}
                                <AtrChart pairKey={pairKey} slot={pair.microTerm.slot} onDoubleClick={() => handleChartDblClick('atr', pair.microTerm.slot, pair.microTerm.barDurationSec)} />
                            {/if}
                        {/key}
                    </div>
                {/each}
            </div>
        </div>

        <!-- Small-Term Column -->
        <div class="{styles.timescaleColumn} {!showFast ? styles.hiddenPane : ''} {expandedTf === 'fast' ? styles.expandedTfColumn : ''}">
            <div class={styles.timescaleHeader} class:styles.tfHeaderHidden={expandedTf === 'fast'}>
                <span class={styles.timescaleTitle}>{termLabel('FAST', pair.fastTerm)}</span>
                <div class={styles.headerActions}>
                    <span class={styles.timescalePrice}>{pair.fastTerm.priceText}</span>
                    <button class={styles.expandBtn} onclick={() => toggleExpand('fast')} title={expandedTf === 'fast' ? 'Collapse' : 'Expand'}>
                        {expandedTf === 'fast' ? '✕' : '⛶'}
                    </button>
                </div>
            </div>
            <div class={styles.timescaleCharts}>
                {#each paneStack(pair.fastTerm) as pane (pane.chartType)}
                    <div class="{styles.panelBox} {styles[pane.box]}">
                        <div class={styles.panelLabel}>{pane.chartType.toUpperCase()}</div>
                        {#key chartKey(pair.fastTerm, pane.chartType)}
                            {#if pane.component === 'Price'}
                                <PriceChart pairKey={pairKey} slot={pair.fastTerm.slot} onDoubleClick={() => handleChartDblClick('price', pair.fastTerm.slot, pair.fastTerm.barDurationSec)} />
                            {:else if pane.component === 'Volume'}
                                <VolumeChart pairKey={pairKey} slot={pair.fastTerm.slot} onDoubleClick={() => handleChartDblClick('volume', pair.fastTerm.slot, pair.fastTerm.barDurationSec)} />
                            {:else if pane.component === 'Rvol'}
                                <RvolChart pairKey={pairKey} slot={pair.fastTerm.slot} onDoubleClick={() => handleChartDblClick('rvol', pair.fastTerm.slot, pair.fastTerm.barDurationSec)} />
                            {:else if pane.component === 'Macd'}
                                <MacdChart pairKey={pairKey} slot={pair.fastTerm.slot} onDoubleClick={() => handleChartDblClick('macd', pair.fastTerm.slot, pair.fastTerm.barDurationSec)} />
                            {:else if pane.component === 'Squeeze'}
                                <SqueezeChart pairKey={pairKey} slot={pair.fastTerm.slot} onDoubleClick={() => handleChartDblClick('squeeze', pair.fastTerm.slot, pair.fastTerm.barDurationSec)} />
                            {:else if pane.component === 'Rsi'}
                                <RsiChart pairKey={pairKey} slot={pair.fastTerm.slot} onDoubleClick={() => handleChartDblClick('rsi', pair.fastTerm.slot, pair.fastTerm.barDurationSec)} />
                            {:else if pane.component === 'Adx'}
                                <AdxChart pairKey={pairKey} slot={pair.fastTerm.slot} onDoubleClick={() => handleChartDblClick('adx', pair.fastTerm.slot, pair.fastTerm.barDurationSec)} />
                            {:else if pane.component === 'Bbwp'}
                                <BbwpChart pairKey={pairKey} slot={pair.fastTerm.slot} onDoubleClick={() => handleChartDblClick('bbwp', pair.fastTerm.slot, pair.fastTerm.barDurationSec)} />
                            {:else if pane.component === 'Atr'}
                                <AtrChart pairKey={pairKey} slot={pair.fastTerm.slot} onDoubleClick={() => handleChartDblClick('atr', pair.fastTerm.slot, pair.fastTerm.barDurationSec)} />
                            {/if}
                        {/key}
                    </div>
                {/each}
            </div>
        </div>

        <!-- Medium-Term Column -->
        <div class="{styles.timescaleColumn} {!showSlow ? styles.hiddenPane : ''} {expandedTf === 'slow' ? styles.expandedTfColumn : ''}">
            <div class={styles.timescaleHeader} class:styles.tfHeaderHidden={expandedTf === 'slow'}>
                <span class={styles.timescaleTitle}>{termLabel('SLOW', pair.slowTerm)}</span>
                <div class={styles.headerActions}>
                    <span class={styles.timescalePrice}>{pair.slowTerm.priceText}</span>
                    <button class={styles.expandBtn} onclick={() => toggleExpand('slow')} title={expandedTf === 'slow' ? 'Collapse' : 'Expand'}>
                        {expandedTf === 'slow' ? '✕' : '⛶'}
                    </button>
                </div>
            </div>
            <div class={styles.timescaleCharts}>
                {#each paneStack(pair.slowTerm) as pane (pane.chartType)}
                    <div class="{styles.panelBox} {styles[pane.box]}">
                        <div class={styles.panelLabel}>{pane.chartType.toUpperCase()}</div>
                        {#key chartKey(pair.slowTerm, pane.chartType)}
                            {#if pane.component === 'Price'}
                                <PriceChart pairKey={pairKey} slot={pair.slowTerm.slot} onDoubleClick={() => handleChartDblClick('price', pair.slowTerm.slot, pair.slowTerm.barDurationSec)} />
                            {:else if pane.component === 'Volume'}
                                <VolumeChart pairKey={pairKey} slot={pair.slowTerm.slot} onDoubleClick={() => handleChartDblClick('volume', pair.slowTerm.slot, pair.slowTerm.barDurationSec)} />
                            {:else if pane.component === 'Rvol'}
                                <RvolChart pairKey={pairKey} slot={pair.slowTerm.slot} onDoubleClick={() => handleChartDblClick('rvol', pair.slowTerm.slot, pair.slowTerm.barDurationSec)} />
                            {:else if pane.component === 'Macd'}
                                <MacdChart pairKey={pairKey} slot={pair.slowTerm.slot} onDoubleClick={() => handleChartDblClick('macd', pair.slowTerm.slot, pair.slowTerm.barDurationSec)} />
                            {:else if pane.component === 'Squeeze'}
                                <SqueezeChart pairKey={pairKey} slot={pair.slowTerm.slot} onDoubleClick={() => handleChartDblClick('squeeze', pair.slowTerm.slot, pair.slowTerm.barDurationSec)} />
                            {:else if pane.component === 'Rsi'}
                                <RsiChart pairKey={pairKey} slot={pair.slowTerm.slot} onDoubleClick={() => handleChartDblClick('rsi', pair.slowTerm.slot, pair.slowTerm.barDurationSec)} />
                            {:else if pane.component === 'Adx'}
                                <AdxChart pairKey={pairKey} slot={pair.slowTerm.slot} onDoubleClick={() => handleChartDblClick('adx', pair.slowTerm.slot, pair.slowTerm.barDurationSec)} />
                            {:else if pane.component === 'Bbwp'}
                                <BbwpChart pairKey={pairKey} slot={pair.slowTerm.slot} onDoubleClick={() => handleChartDblClick('bbwp', pair.slowTerm.slot, pair.slowTerm.barDurationSec)} />
                            {:else if pane.component === 'Atr'}
                                <AtrChart pairKey={pairKey} slot={pair.slowTerm.slot} onDoubleClick={() => handleChartDblClick('atr', pair.slowTerm.slot, pair.slowTerm.barDurationSec)} />
                            {/if}
                        {/key}
                    </div>
                {/each}
            </div>
        </div>

        <!-- Large-Term Column -->
        <div class="{styles.timescaleColumn} {!showMacro ? styles.hiddenPane : ''} {expandedTf === 'macro' ? styles.expandedTfColumn : ''}">
            <div class={styles.timescaleHeader} class:styles.tfHeaderHidden={expandedTf === 'macro'}>
                <span class={styles.timescaleTitle}>{termLabel('MACRO', pair.macroTerm)}</span>
                <div class={styles.headerActions}>
                    <span class={styles.timescalePrice}>{pair.macroTerm.priceText}</span>
                    <button class={styles.expandBtn} onclick={() => toggleExpand('macro')} title={expandedTf === 'macro' ? 'Collapse' : 'Expand'}>
                        {expandedTf === 'macro' ? '✕' : '⛶'}
                    </button>
                </div>
            </div>
            <div class={styles.timescaleCharts}>
                {#each paneStack(pair.macroTerm) as pane (pane.chartType)}
                    <div class="{styles.panelBox} {styles[pane.box]}">
                        <div class={styles.panelLabel}>{pane.chartType.toUpperCase()}</div>
                        {#key chartKey(pair.macroTerm, pane.chartType)}
                            {#if pane.component === 'Price'}
                                <PriceChart pairKey={pairKey} slot={pair.macroTerm.slot} onDoubleClick={() => handleChartDblClick('price', pair.macroTerm.slot, pair.macroTerm.barDurationSec)} />
                            {:else if pane.component === 'Volume'}
                                <VolumeChart pairKey={pairKey} slot={pair.macroTerm.slot} onDoubleClick={() => handleChartDblClick('volume', pair.macroTerm.slot, pair.macroTerm.barDurationSec)} />
                            {:else if pane.component === 'Rvol'}
                                <RvolChart pairKey={pairKey} slot={pair.macroTerm.slot} onDoubleClick={() => handleChartDblClick('rvol', pair.macroTerm.slot, pair.macroTerm.barDurationSec)} />
                            {:else if pane.component === 'Macd'}
                                <MacdChart pairKey={pairKey} slot={pair.macroTerm.slot} onDoubleClick={() => handleChartDblClick('macd', pair.macroTerm.slot, pair.macroTerm.barDurationSec)} />
                            {:else if pane.component === 'Squeeze'}
                                <SqueezeChart pairKey={pairKey} slot={pair.macroTerm.slot} onDoubleClick={() => handleChartDblClick('squeeze', pair.macroTerm.slot, pair.macroTerm.barDurationSec)} />
                            {:else if pane.component === 'Rsi'}
                                <RsiChart pairKey={pairKey} slot={pair.macroTerm.slot} onDoubleClick={() => handleChartDblClick('rsi', pair.macroTerm.slot, pair.macroTerm.barDurationSec)} />
                            {:else if pane.component === 'Adx'}
                                <AdxChart pairKey={pairKey} slot={pair.macroTerm.slot} onDoubleClick={() => handleChartDblClick('adx', pair.macroTerm.slot, pair.macroTerm.barDurationSec)} />
                            {:else if pane.component === 'Bbwp'}
                                <BbwpChart pairKey={pairKey} slot={pair.macroTerm.slot} onDoubleClick={() => handleChartDblClick('bbwp', pair.macroTerm.slot, pair.macroTerm.barDurationSec)} />
                            {:else if pane.component === 'Atr'}
                                <AtrChart pairKey={pairKey} slot={pair.macroTerm.slot} onDoubleClick={() => handleChartDblClick('atr', pair.macroTerm.slot, pair.macroTerm.barDurationSec)} />
                            {/if}
                        {/key}
                    </div>
                {/each}
            </div>
        </div>
        </div>
    {/if}
</div>
