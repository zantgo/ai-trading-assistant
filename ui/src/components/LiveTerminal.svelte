<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import styles from './LiveTerminal.module.css';
    import ChartToggles from './ChartToggles.svelte';
    import PriceChart from './PriceChart.svelte';
    import RsiChart from './RsiChart.svelte';
    import MacdChart from './MacdChart.svelte';
    import AdxChart from './AdxChart.svelte';
    import AtrChart from './AtrChart.svelte';
    import SqueezeChart from './SqueezeChart.svelte';
    import BbwpChart from './BbwpChart.svelte';
    import VolumeChart from './VolumeChart.svelte';
    import RvolChart from './RvolChart.svelte';
    import StochasticChart from './StochasticChart.svelte';
    import ChandeMoChart from './ChandeMoChart.svelte';
    import ObvChart from './ObvChart.svelte';
    import CmfChart from './CmfChart.svelte';
    import MfiChart from './MfiChart.svelte';
    import HvChart from './HvChart.svelte';
    import AroonChart from './AroonChart.svelte';
    import ChoppinessChart from './ChoppinessChart.svelte';
    import LinRegSlopeChart from './LinRegSlopeChart.svelte';
    import ZScoreChart from './ZScoreChart.svelte';
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
    let indicatorsOpen = $state(true);
    let liquidityOpen = $state(true);

    const activeTfObj = $derived(
        (activeTf as string) === 'Fast' ? pair?.fastTerm :
        (activeTf as string) === 'Slow' ? pair?.slowTerm :
        (activeTf as string) === 'Macro' ? pair?.macroTerm :
        pair?.microTerm
    );

    const hasLiquidity = $derived(!!activeTfObj?.liquidity || !!activeTfObj?.cluster);

    type PaneDef = {
        key: string;
        label: string;
        component: any;
        visible: () => boolean;
    };

    const panes = $derived<PaneDef[]>([
        { key: 'rsi', label: 'RSI', component: RsiChart, visible: () => true },
        { key: 'macd', label: 'MACD', component: MacdChart, visible: () => true },
        { key: 'adx', label: 'ADX', component: AdxChart, visible: () => true },
        { key: 'atr', label: 'ATR', component: AtrChart, visible: () => true },
        { key: 'squeeze', label: 'Squeeze', component: SqueezeChart, visible: () => true },
        { key: 'bbwp', label: 'BBWP', component: BbwpChart, visible: () => true },
        { key: 'volume', label: 'Volume', component: VolumeChart, visible: () => true },
        { key: 'rvol', label: 'RVOL', component: RvolChart, visible: () => true },
        { key: 'stochastic', label: 'Stoch', component: StochasticChart, visible: () => true },
        { key: 'chandemo', label: 'ChandeMO', component: ChandeMoChart, visible: () => true },
        { key: 'obv', label: 'OBV', component: ObvChart, visible: () => true },
        { key: 'cmf', label: 'CMF', component: CmfChart, visible: () => true },
        { key: 'mfi', label: 'MFI', component: MfiChart, visible: () => true },
        { key: 'hv', label: 'HV', component: HvChart, visible: () => true },
        { key: 'aroon', label: 'Aroon', component: AroonChart, visible: () => true },
        { key: 'choppiness', label: 'Choppiness', component: ChoppinessChart, visible: () => true },
        { key: 'linreg', label: 'LinReg Slope', component: LinRegSlopeChart, visible: () => true },
        { key: 'zscore', label: 'Z-Score', component: ZScoreChart, visible: () => true },
    ]);

    let paneVisibility = $state<Record<string, boolean>>({});

    function togglePane(key: string) {
        paneVisibility = { ...paneVisibility, [key]: !paneVisibility[key] };
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
                    <div class={styles.priceChartContainer}>
                        <PriceChart pairKey={pairKey} timeframe={tfSecs} />
                    </div>
                {/if}
            </section>

            <section class={styles.collapsibleSection}>
                <button class={styles.collapsibleHeader} onclick={() => indicatorsOpen = !indicatorsOpen}>
                    <span class={styles.collapsibleCaret}>{indicatorsOpen ? '▼' : '▶'}</span>
                    <span>Indicators</span>
                    <span class={styles.tfBadge}>18 panes</span>
                </button>
                {#if indicatorsOpen}
                    <div class={styles.panesToolbar}>
                        {#each panes as pane (pane.key)}
                            <button
                                class={styles.paneToggle}
                                class:active={!paneVisibility[pane.key] !== false}
                                onclick={() => togglePane(pane.key)}
                            >
                                {pane.label}
                            </button>
                        {/each}
                    </div>
                    <div class={styles.panesGrid}>
                        {#each panes as pane (pane.key)}
                            {#if !paneVisibility[pane.key] !== false}
                                <div class={styles.paneWrapper}>
                                    <div class={styles.paneHeader}>{pane.label}</div>
                                    {#if pane.key === 'rsi'}
                                        <RsiChart pairKey={pairKey} timeframe={tfSecs} />
                                    {:else if pane.key === 'macd'}
                                        <MacdChart pairKey={pairKey} timeframe={tfSecs} />
                                    {:else if pane.key === 'adx'}
                                        <AdxChart pairKey={pairKey} timeframe={tfSecs} />
                                    {:else if pane.key === 'atr'}
                                        <AtrChart pairKey={pairKey} timeframe={tfSecs} />
                                    {:else if pane.key === 'squeeze'}
                                        <SqueezeChart pairKey={pairKey} timeframe={tfSecs} />
                                    {:else if pane.key === 'bbwp'}
                                        <BbwpChart pairKey={pairKey} timeframe={tfSecs} />
                                    {:else if pane.key === 'volume'}
                                        <VolumeChart pairKey={pairKey} timeframe={tfSecs} />
                                    {:else if pane.key === 'rvol'}
                                        <RvolChart pairKey={pairKey} timeframe={tfSecs} />
                                    {:else if pane.key === 'stochastic'}
                                        <StochasticChart pairKey={pairKey} timeframe={tfSecs} />
                                    {:else if pane.key === 'chandemo'}
                                        <ChandeMoChart pairKey={pairKey} timeframe={tfSecs} />
                                    {:else if pane.key === 'obv'}
                                        <ObvChart pairKey={pairKey} timeframe={tfSecs} />
                                    {:else if pane.key === 'cmf'}
                                        <CmfChart pairKey={pairKey} timeframe={tfSecs} />
                                    {:else if pane.key === 'mfi'}
                                        <MfiChart pairKey={pairKey} timeframe={tfSecs} />
                                    {:else if pane.key === 'hv'}
                                        <HvChart pairKey={pairKey} timeframe={tfSecs} />
                                    {:else if pane.key === 'aroon'}
                                        <AroonChart pairKey={pairKey} timeframe={tfSecs} />
                                    {:else if pane.key === 'choppiness'}
                                        <ChoppinessChart pairKey={pairKey} timeframe={tfSecs} />
                                    {:else if pane.key === 'linreg'}
                                        <LinRegSlopeChart pairKey={pairKey} timeframe={tfSecs} />
                                    {:else if pane.key === 'zscore'}
                                        <ZScoreChart pairKey={pairKey} timeframe={tfSecs} />
                                    {/if}
                                </div>
                            {/if}
                        {/each}
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
                <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="0.8" stroke-linecap="round" stroke-linejoin="round" opacity="0.3">
                    <polyline points="22 12 18 12 15 21 9 3 6 12 2 12"/>
                </svg>
                <p>Select a workspace to view live charts</p>
            </div>
        {/if}
    </div>
</div>
