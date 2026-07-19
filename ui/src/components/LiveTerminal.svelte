<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import styles from './LiveTerminal.module.css';
    import { getIcon } from '../lib/icons';
    import ChartToggles from './ChartToggles.svelte';
    import PriceChart from './PriceChart.svelte';
    import RvolChart from './RvolChart.svelte';
    import RsiChart from './RsiChart.svelte';
    import MacdChart from './MacdChart.svelte';
    import AdxChart from './AdxChart.svelte';
    import AtrChart from './AtrChart.svelte';
    import LiquidityPanel from './LiquidityPanel.svelte';

    const app = useAppStore();
    let { pairKey }: { pairKey: string } = $props();
    const pair = $derived(app.instancesMap[pairKey]);

    type TfLabel = 'Micro' | 'Fast' | 'Slow' | 'Macro';
    let activeTf: TfLabel = $state('Micro');

    const TIMEFRAMES: { key: TfLabel; label: string; secs: number }[] = [
        { key: 'Micro', label: 'Micro', secs: 60 },
        { key: 'Fast',  label: 'Fast',  secs: 180 },
        { key: 'Slow',  label: 'Slow',  secs: 300 },
        { key: 'Macro', label: 'Macro', secs: 900 },
    ];

    const tfSecs = $derived(TIMEFRAMES.find(t => t.key === activeTf)!.secs);

    let priceOpen = $state(true);
    let rvolOpen = $state(true);
    let rsiOpen = $state(true);
    let macdOpen = $state(true);
    let adxOpen = $state(true);
    let atrOpen = $state(true);
    let liquidityOpen = $state(true);

    const activeTfObj = $derived(
        (activeTf as string) === 'Fast' ? pair?.fastTerm :
        (activeTf as string) === 'Slow' ? pair?.slowTerm :
        (activeTf as string) === 'Macro' ? pair?.macroTerm :
        pair?.microTerm
    );

    const hasLiquidity = $derived(!!pair?.microTerm?.liquidity || !!pair?.microTerm?.cluster);

    // ─── Resizable pane heights ────────────────────────────────────────
    const DEFAULT_PRICE = 420;
    const DEFAULT_INDICATOR = 160;
    const MIN_HEIGHT = 60;
    const MAX_HEIGHT = 800;

    let paneHeights = $state([DEFAULT_PRICE, DEFAULT_INDICATOR, DEFAULT_INDICATOR, DEFAULT_INDICATOR, DEFAULT_INDICATOR, DEFAULT_INDICATOR]);
    let draggingIdx = $state(-1);
    let dragStartY = $state(0);
    let dragStartHeights = $state<number[]>([]);

    function handleDragStart(idx: number, e: MouseEvent) {
        e.preventDefault();
        draggingIdx = idx;
        dragStartY = e.clientY;
        dragStartHeights = [...paneHeights];
        window.addEventListener('mousemove', handleDragMove);
        window.addEventListener('mouseup', handleDragEnd);
    }

    function handleDragMove(e: MouseEvent) {
        if (draggingIdx < 0) return;
        const delta = e.clientY - dragStartY;
        const current = [...dragStartHeights];
        const newTop = Math.max(MIN_HEIGHT, Math.min(MAX_HEIGHT, current[draggingIdx] + delta));
        const newBottom = Math.max(MIN_HEIGHT, Math.min(MAX_HEIGHT, current[draggingIdx + 1] - delta));
        const total = current[draggingIdx] + current[draggingIdx + 1];
        current[draggingIdx] = total > 0 ? (newTop / (newTop + newBottom)) * total : current[draggingIdx] + delta / 2;
        current[draggingIdx + 1] = total - current[draggingIdx];
        current[draggingIdx] = Math.max(MIN_HEIGHT, Math.min(MAX_HEIGHT, current[draggingIdx]));
        current[draggingIdx + 1] = Math.max(MIN_HEIGHT, Math.min(MAX_HEIGHT, current[draggingIdx + 1]));
        paneHeights = current;
    }

    function handleDragEnd() {
        draggingIdx = -1;
        window.removeEventListener('mousemove', handleDragMove);
        window.removeEventListener('mouseup', handleDragEnd);
    }

    function resetPanes(idx: number) {
        const current = [...paneHeights];
        current[idx] = idx === 0 ? DEFAULT_PRICE : DEFAULT_INDICATOR;
        current[idx + 1] = DEFAULT_INDICATOR;
        paneHeights = current;
    }
</script>

<div class={styles.terminalWorkspace}>
    <div class={styles.tfSidebar}>
        <h3 class={styles.tfSidebarTitle}>TIMEFRAMES</h3>
        {#each TIMEFRAMES as tf (tf.key)}
            <button
                class={styles.tfSidebarItem}
                class:active={activeTf === tf.key}
                onclick={() => activeTf = tf.key}
            >
                <span class={styles.tfLabel}>{tf.label}</span>
                <span class={styles.tfSecs}>{tf.secs}s</span>
            </button>
        {/each}
    </div>

    <div class={styles.chartArea}>
        {#if pair && tfSecs}
            <ChartToggles {pairKey} />

            <section class={styles.collapsibleSection}>
                <button class={styles.collapsibleHeader} onclick={() => priceOpen = !priceOpen}>
                    <span class={styles.collapsibleCaret}>{priceOpen ? '▼' : '▶'}</span>
                    <span>Price Chart</span>
                    <span class={styles.tfBadge}>{activeTf} · {tfSecs}s</span>
                </button>
                {#if priceOpen}
                    <div class={styles.resizablePane} style="height:{paneHeights[0]}px">
                        <PriceChart pairKey={pairKey} timeframe={tfSecs} />
                    </div>
                    <button class={styles.dragHandle} onmousedown={(e) => handleDragStart(0, e)}
                         ondblclick={() => resetPanes(0)}
                         title="Drag to resize · Double-click to reset"></button>
                {/if}
            </section>

            <section class={styles.collapsibleSection}>
                <button class={styles.collapsibleHeader} onclick={() => rvolOpen = !rvolOpen}>
                    <span class={styles.collapsibleCaret}>{rvolOpen ? '▼' : '▶'}</span>
                    <span>RVOL</span>
                    <span class={styles.tfBadge}>{activeTf} · {tfSecs}s</span>
                </button>
                {#if rvolOpen}
                    <div class={styles.resizablePane} style="height:{paneHeights[1]}px">
                        <RvolChart pairKey={pairKey} timeframe={tfSecs} />
                    </div>
                    <button class={styles.dragHandle} onmousedown={(e) => handleDragStart(1, e)}
                         ondblclick={() => resetPanes(1)}
                         title="Drag to resize · Double-click to reset"></button>
                {/if}
            </section>

            <section class={styles.collapsibleSection}>
                <button class={styles.collapsibleHeader} onclick={() => rsiOpen = !rsiOpen}>
                    <span class={styles.collapsibleCaret}>{rsiOpen ? '▼' : '▶'}</span>
                    <span>RSI 14</span>
                    <span class={styles.tfBadge}>{activeTf} · {tfSecs}s</span>
                </button>
                {#if rsiOpen}
                    <div class={styles.resizablePane} style="height:{paneHeights[2]}px">
                        <RsiChart pairKey={pairKey} timeframe={tfSecs} />
                    </div>
                    <button class={styles.dragHandle} onmousedown={(e) => handleDragStart(2, e)}
                         ondblclick={() => resetPanes(2)}
                         title="Drag to resize · Double-click to reset"></button>
                {/if}
            </section>

            <section class={styles.collapsibleSection}>
                <button class={styles.collapsibleHeader} onclick={() => macdOpen = !macdOpen}>
                    <span class={styles.collapsibleCaret}>{macdOpen ? '▼' : '▶'}</span>
                    <span>MACD</span>
                    <span class={styles.tfBadge}>{activeTf} · {tfSecs}s</span>
                </button>
                {#if macdOpen}
                    <div class={styles.resizablePane} style="height:{paneHeights[3]}px">
                        <MacdChart pairKey={pairKey} timeframe={tfSecs} />
                    </div>
                    <button class={styles.dragHandle} onmousedown={(e) => handleDragStart(3, e)}
                         ondblclick={() => resetPanes(3)}
                         title="Drag to resize · Double-click to reset"></button>
                {/if}
            </section>

            <section class={styles.collapsibleSection}>
                <button class={styles.collapsibleHeader} onclick={() => adxOpen = !adxOpen}>
                    <span class={styles.collapsibleCaret}>{adxOpen ? '▼' : '▶'}</span>
                    <span>ADX 14</span>
                    <span class={styles.tfBadge}>{activeTf} · {tfSecs}s</span>
                </button>
                {#if adxOpen}
                    <div class={styles.resizablePane} style="height:{paneHeights[4]}px">
                        <AdxChart pairKey={pairKey} timeframe={tfSecs} />
                    </div>
                    <button class={styles.dragHandle} onmousedown={(e) => handleDragStart(4, e)}
                         ondblclick={() => resetPanes(4)}
                         title="Drag to resize · Double-click to reset"></button>
                {/if}
            </section>

            <section class={styles.collapsibleSection}>
                <button class={styles.collapsibleHeader} onclick={() => atrOpen = !atrOpen}>
                    <span class={styles.collapsibleCaret}>{atrOpen ? '▼' : '▶'}</span>
                    <span>ATR 14</span>
                    <span class={styles.tfBadge}>{activeTf} · {tfSecs}s</span>
                </button>
                {#if atrOpen}
                    <div class={styles.resizablePane} style="height:{paneHeights[5]}px">
                        <AtrChart pairKey={pairKey} timeframe={tfSecs} />
                    </div>
                {/if}
            </section>

            {#if hasLiquidity}
                <section class={styles.collapsibleSection}>
                    <button class={styles.collapsibleHeader} onclick={() => liquidityOpen = !liquidityOpen}>
                        <span class={styles.collapsibleCaret}>{liquidityOpen ? '▼' : '▶'}</span>
                        <span>Liquidation Analysis</span>
                        <span class={styles.tfBadge}>cascade · cluster</span>
                    </button>
                    {#if liquidityOpen}
                        <div class={styles.liquidityContainer}>
                            <LiquidityPanel {pairKey} />
                        </div>
                    {/if}
                </section>
            {/if}
        {:else}
            <div class={styles.placeholderChart}>
                {@html getIcon('activity', 48)}
                <p>Select a workspace to view live charts</p>
            </div>
        {/if}
    </div>
</div>
