<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import styles from './LiveTerminal.module.css';
    import type { TimeframeTelemetry } from '../types';

    const app = useAppStore();
    let { pairKey }: { pairKey: string } = $props();
    const pair = $derived(app.instancesMap[pairKey]);

    // ─── Timeframe selection ────────────────────────────────────────────
    type TfLabel = 'Micro' | 'Fast' | 'Slow' | 'Macro';
    let activeTf: TfLabel = $state('Micro');

    const TIMEFRAMES: { key: TfLabel; label: string; secs: number }[] = [
        { key: 'Micro', label: 'Micro',   secs: 60 },
        { key: 'Fast',  label: 'Fast',    secs: 180 },
        { key: 'Slow',  label: 'Slow',    secs: 300 },
        { key: 'Macro', label: 'Macro',   secs: 900 },
    ];

    // ─── Collapsible section state ──────────────────────────────────────
    let priceOpen = $state(true);
    let indicatorsOpen = $state(true);
    let liquidityOpen = $state(true);

    const tfInfo = $derived(TIMEFRAMES.find(t => t.key === activeTf)!);
</script>

<div class={styles.terminalWorkspace}>
    <!-- Left panel: timeframe selector -->
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

    <!-- Right content: charts area -->
    <div class={styles.chartArea}>
        <!-- 1. Price Chart -->
        <section class={styles.collapsibleSection}>
            <button class={styles.collapsibleHeader} onclick={() => priceOpen = !priceOpen}>
                <span class={styles.collapsibleCaret}>{priceOpen ? '▼' : '▶'}</span>
                <span>Price Chart</span>
                <span class={styles.tfBadge}>{tfInfo.label} · {tfInfo.secs}s</span>
            </button>
            {#if priceOpen}
                <div class={styles.collapsibleBody}>
                    <div class={styles.placeholderChart}>
                        <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="0.8" stroke-linecap="round" stroke-linejoin="round" opacity="0.3">
                            <polyline points="22 12 18 12 15 21 9 3 6 12 2 12"/>
                        </svg>
                        <p>Candlestick chart + support/resistance levels + EMA overlays</p>
                        <span class={styles.comingSoon}>Coming soon</span>
                    </div>
                </div>
            {/if}
        </section>

        <!-- 2. Indicators -->
        <section class={styles.collapsibleSection}>
            <button class={styles.collapsibleHeader} onclick={() => indicatorsOpen = !indicatorsOpen}>
                <span class={styles.collapsibleCaret}>{indicatorsOpen ? '▼' : '▶'}</span>
                <span>Indicators</span>
                <span class={styles.tfBadge}>{tfInfo.label} · {tfInfo.secs}s</span>
            </button>
            {#if indicatorsOpen}
                <div class={styles.collapsibleBody}>
                    <div class={styles.placeholderChart}>
                        <p>50 indicators across 8 functional groups (Trend, Momentum, Volume, Volatility, Structure, Regime, Institutional, Derivatives Data)</p>
                        <span class={styles.comingSoon}>Coming soon</span>
                    </div>
                </div>
            {/if}
        </section>

        <!-- 3. Liquidity Heatmap -->
        <section class={styles.collapsibleSection}>
            <button class={styles.collapsibleHeader} onclick={() => liquidityOpen = !liquidityOpen}>
                <span class={styles.collapsibleCaret}>{liquidityOpen ? '▼' : '▶'}</span>
                <span>Liquidation Heatmap</span>
                <span class={styles.tfBadge}>cluster · cascade</span>
            </button>
            {#if liquidityOpen}
                <div class={styles.collapsibleBody}>
                    <div class={styles.placeholderChart}>
                        <p>Estimated liquidation cluster distribution (vertical heatmap) + cascade risk intensity</p>
                        <p style="color:#888; font-size:0.75rem; margin-top:0.5rem;">Data source: <code>MarketSnapshot.liquidity</code> and <code>MarketSnapshot.cluster</code> fields (Phase 0-4 Liquidity Intelligence extension). The heatmap is part of the Market Monitor Metrics Layer (L1), not a separate matrix or engine.</p>
                        <span class={styles.comingSoon}>Coming soon</span>
                    </div>
                </div>
            {/if}
        </section>
    </div>
</div>
