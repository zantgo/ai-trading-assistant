<script lang="ts">
    // Phase 4: Liquidity Intelligence Panel
    //
    // Renders the three views into the liquidity intelligence subsystem:
    //   1. Flow    — per-candle real liquidation events (Phase 1)
    //   2. Cluster — estimated liquidation heatmap (Phase 2)
    //   3. Context — funding, OI, mark/index, leverage assumptions
    //
    // Data sources:
    //   - instance.microTerm.liquidity (per-candle flow)
    //   - instance.microTerm.cluster   (per-timeframe matrix; refreshed at
    //                                   TF candle cadence; each TF owns its own)
    //   - instance.microTerm.liquiditySignals (computed by server)
    import type { LiquidityFlow, LiquidationClusterMatrix, LiquiditySignal, TimeframeTelemetry } from '../types';
    import { useAppStore } from '../state.svelte';
    import styles from './LiquidityPanel.module.css';

    const app = useAppStore();
    let { pairKey } = $props<{ pairKey: string }>();

    let activeView = $state<'flow' | 'cluster' | 'context'>('flow');
    let flowTf = $state<'micro' | 'fast' | 'slow' | 'macro'>('micro');

    const instance = $derived(app && pairKey ? app.instancesMap[pairKey] : undefined);
    const micro = $derived(instance?.microTerm);

    const flowTerm = $derived<TimeframeTelemetry | undefined>(
        (instance as any)?.[`${flowTf}Term`] as TimeframeTelemetry | undefined,
    );

    const flow = $derived<LiquidityFlow | null>(flowTerm?.liquidity ?? null);
    const cluster = $derived<LiquidationClusterMatrix | null>(micro?.cluster ?? null);
    const signals = $derived<LiquiditySignal[]>(micro?.liquiditySignals ?? []);

    function fmtUsd(n: number): string {
        if (n === 0) return '$0';
        if (Math.abs(n) >= 1_000_000) return `$${(n / 1_000_000).toFixed(2)}M`;
        if (Math.abs(n) >= 1_000) return `$${(n / 1_000).toFixed(1)}K`;
        return `$${n.toFixed(0)}`;
    }

    function fmtPrice(n: number | undefined): string {
        if (n === undefined || n === null) return '--';
        if (n >= 1000) return `$${n.toFixed(0)}`;
        return `$${n.toFixed(2)}`;
    }

    function fmtPct(n: number): string {
        return `${n.toFixed(2)}%`;
    }
</script>

<div class={styles.panel}>
    <div class={styles.tabBar}>
        <button class="{styles.tab} {activeView === 'flow' ? styles.tabActive : ''}"
                onclick={() => activeView = 'flow'}>Flow</button>
        <button class="{styles.tab} {activeView === 'cluster' ? styles.tabActive : ''}"
                onclick={() => activeView = 'cluster'}>Cluster</button>
        <button class="{styles.tab} {activeView === 'context' ? styles.tabActive : ''}"
                onclick={() => activeView = 'context'}>Context</button>
    </div>

    {#if activeView === 'flow'}
        <div class={styles.section}>
            <div class={styles.sectionHeader}>
                <h3 class={styles.h3}>Real Liquidation Flow (per bar)</h3>
                <div class={styles.tfSwitcher} role="tablist" aria-label="Timeframe for flow">
                    {#each [['micro','MICRO'],['fast','FAST'],['slow','SLOW'],['macro','MACRO']] as const as item}
                        <button
                            class="{styles.tfBtn} {flowTf === item[0] ? styles.tfBtnActive : ''}"
                            onclick={() => flowTf = item[0]}
                            role="tab"
                            aria-selected={flowTf === item[0]}
                        >{item[1]}</button>
                    {/each}
                </div>
            </div>
            {#if !flow}
                <div class={styles.placeholder}>Awaiting first completed bar with {flowTf.toUpperCase()} liquidation data…</div>
            {:else}
                <div class={styles.statGrid}>
                    <div class={styles.statCard}>
                        <div class={styles.statLabel}>Long Liquidations</div>
                        <div class={styles.statValue + ' ' + styles.bearish}>
                            {fmtUsd(flow.long_liquidations_usd)}
                        </div>
                    </div>
                    <div class={styles.statCard}>
                        <div class={styles.statLabel}>Short Liquidations</div>
                        <div class={styles.statValue + ' ' + styles.bullish}>
                            {fmtUsd(flow.short_liquidations_usd)}
                        </div>
                    </div>
                    <div class={styles.statCard}>
                        <div class={styles.statLabel}>Net Flow</div>
                        <div class={styles.statValue}>
                            {fmtUsd(flow.net_liquidation_usd)}
                        </div>
                    </div>
                    <div class={styles.statCard}>
                        <div class={styles.statLabel}>Events</div>
                        <div class={styles.statValue}>{flow.event_count}</div>
                    </div>
                </div>

                <div class={styles.subSection}>
                    <div class={styles.subLabel}>Cascade State</div>
                    <div class={styles.cascadeRow}>
                        <div class="{styles.cascadeBadge} {flow.cascade_state === 'SUSTAINED' ? styles.cascadeDanger :
                                                      flow.cascade_state === 'DETECTED' ? styles.cascadeWarning :
                                                      flow.cascade_state === 'EXHAUSTED' ? styles.cascadeCooling :
                                                      styles.cascadeNormal}">
                            {flow.cascade_state}
                        </div>
                        <div class={styles.intensityBar}>
                            <div class={styles.intensityFill}
                                 style="width: {Math.min(flow.cascade_intensity, 100).toFixed(1)}%"></div>
                        </div>
                        <div class={styles.intensityText}>
                            Intensity: {flow.cascade_intensity.toFixed(0)}/100
                        </div>
                    </div>
                </div>

                {#if flow.largest_event_usd > 0}
                    <div class={styles.subSection}>
                        <div class={styles.subLabel}>Largest Event</div>
                        <div class={styles.eventDetail}>
                            <span class={styles.detailKey}>Notional:</span>
                            <span class={styles.detailVal}>{fmtUsd(flow.largest_event_usd)}</span>
                            <span class={styles.detailKey}>Price:</span>
                            <span class={styles.detailVal}>{fmtPrice(flow.largest_event_price)}</span>
                            <span class={styles.detailKey}>Side:</span>
                            <span class="{styles.detailVal} {flow.largest_event_side === 'LONG' ? styles.bearish : styles.bullish}">
                                {flow.largest_event_side ?? '—'}
                            </span>
                        </div>
                    </div>
                {/if}
            {/if}
        </div>

    {:else if activeView === 'cluster'}
        <div class={styles.section}>
            <h3 class={styles.h3}>Estimated Liquidation Heatmap</h3>
            {#if !cluster}
                <div class={styles.placeholder}>Cluster matrix refreshes every 5 minutes. Awaiting first computation…</div>
            {:else}
                <div class={styles.subSection}>
                    <div class={styles.subLabel}>Assumptions</div>
                    <div class={styles.assumptionRow}>
                        <span>Source:</span>
                        <code class={styles.code}>{cluster.leverage_assumptions.source}</code>
                        <span>Buckets:</span>
                        <code class={styles.code}>{cluster.leverage_assumptions.buckets.join(', ')}</code>
                        <span>Modulation:</span>
                        <code class={styles.code}>{cluster.leverage_assumptions.funding_modulation_active ? 'on' : 'off'}</code>
                        <span>Confidence:</span>
                        <code class={styles.code}>{(cluster.estimation_confidence * 100).toFixed(0)}%</code>
                    </div>
                </div>

                <div class={styles.subSection}>
                    <div class={styles.subLabel}>Cascade Asymmetry</div>
                    <div class={styles.assumptionRow}>
                        <span>Sign:</span>
                        <code class={styles.code + ' ' + (cluster.cascade_asymmetry < 0 ? styles.bullish : cluster.cascade_asymmetry > 0 ? styles.bearish : '')}>
                            {cluster.cascade_asymmetry.toFixed(3)}
                        </code>
                        <span>Direction:</span>
                        <code class={styles.code}>
                            {cluster.cascade_asymmetry < -0.3 ? 'SHORT_SQUEEZE_RISK' :
                             cluster.cascade_asymmetry > 0.3 ? 'LONG_SQUEEZE_RISK' : 'NEUTRAL'}
                        </code>
                    </div>
                </div>

                <div class={styles.subSection}>
                    <div class={styles.subLabel}>Short Clusters (above mid)</div>
                    {#if cluster.short_clusters.length === 0}
                        <div class={styles.placeholder}>No short-side clusters above noise threshold.</div>
                    {:else}
                        {#each cluster.short_clusters as c}
                            <div class={styles.clusterRow}>
                                <span class={styles.clusterPrice}>{fmtPrice(c.peak_price)}</span>
                                <span class={styles.clusterRange}>
                                    [{fmtPrice(c.price_low)} – {fmtPrice(c.price_high)}]
                                </span>
                                <span class={styles.clusterNotional}>{fmtUsd(c.notional_usd)}</span>
                                <span class={styles.clusterDistance}>{fmtPct(c.distance_from_mid_pct)}</span>
                                <span class="{styles.clusterKind} {styles.kindAbove}">{c.cluster_kind}</span>
                                <span class={styles.clusterMagnet} style="width: {c.magnet_strength.toFixed(0)}px">
                                    {c.magnet_strength.toFixed(0)}
                                </span>
                            </div>
                        {/each}
                    {/if}
                </div>

                <div class={styles.subSection}>
                    <div class={styles.subLabel}>Long Clusters (below mid)</div>
                    {#if cluster.long_clusters.length === 0}
                        <div class={styles.placeholder}>No long-side clusters above noise threshold.</div>
                    {:else}
                        {#each cluster.long_clusters as c}
                            <div class={styles.clusterRow}>
                                <span class={styles.clusterPrice}>{fmtPrice(c.peak_price)}</span>
                                <span class={styles.clusterRange}>
                                    [{fmtPrice(c.price_low)} – {fmtPrice(c.price_high)}]
                                </span>
                                <span class={styles.clusterNotional}>{fmtUsd(c.notional_usd)}</span>
                                <span class={styles.clusterDistance}>{fmtPct(c.distance_from_mid_pct)}</span>
                                <span class="{styles.clusterKind} {styles.kindBelow}">{c.cluster_kind}</span>
                                <span class={styles.clusterMagnet} style="width: {c.magnet_strength.toFixed(0)}px">
                                    {c.magnet_strength.toFixed(0)}
                                </span>
                            </div>
                        {/each}
                    {/if}
                </div>

                <div class={styles.subSection}>
                    <div class={styles.subLabel}>OI Split</div>
                    <div class={styles.assumptionRow}>
                        <span>Long:</span>
                        <code class={styles.code}>{fmtUsd(cluster.total_long_oi_usd)}</code>
                        <span>Short:</span>
                        <code class={styles.code}>{fmtUsd(cluster.total_short_oi_usd)}</code>
                    </div>
                </div>
            {/if}
        </div>

    {:else if activeView === 'context'}
        <div class={styles.section}>
            <h3 class={styles.h3}>Liquidity Context</h3>

            <div class={styles.subSection}>
                <div class={styles.subLabel}>Active Liquidity Signals</div>
                {#if signals.length === 0}
                    <div class={styles.placeholder}>No active signals.</div>
                {:else}
                    {#each signals as sig}
                        <div class={styles.signalRow + ' ' +
                                    (sig.direction === 'BULLISH' ? styles.signalBullish :
                                     sig.direction === 'BEARISH' ? styles.signalBearish : styles.signalNeutral)}>
                            <span class={styles.signalKind}>{sig.kind}</span>
                            <span class={styles.signalDir}>{sig.direction}</span>
                            <span class={styles.signalStrength}>str {sig.strength.toFixed(0)}</span>
                            <span class={styles.signalConf}>conf {(sig.confidence * 100).toFixed(0)}%</span>
                            <ul class={styles.signalEvidence}>
                                {#each sig.evidence as e}
                                    <li>{e}</li>
                                {/each}
                            </ul>
                        </div>
                    {/each}
                {/if}
            </div>
    </div>
{/if}
</div>