<script lang="ts">
    // StructuralAnchorsStrip — Row 4 of the redesigned Metrics view.
    //
    // Always-visible compact strip showing three tiers of structural context
    // for the active timeframe: Volume Profile (POC/VAH/VAL), Fibonacci
    // (Golden Pocket + direction), and Liquidity ladder (distance+strength
    // of clusters on each side of current price). Each tile is collapsed
    // to a 4-stat summary by default; click ▾ to expand into details.
    //
    // Data sources (all per-TF, refreshed per candle cadence):
    //   - microTf.volumeProfile    : VolumeProfileSnapshot
    //   - activeTfObj.indicators.fibonacci.values : GP zone + ext targets
    //   - microTf.cluster          : LiquidationClusterMatrix (top 3 per side)
    //   - microTf.liquidity        : LiquidityFlow (latest bar long/short liqs)

    import type {
        LiquidationCluster,
        LiquidationClusterMatrix,
        LiquidityFlow,
        VolumeProfileBin,
        VolumeProfileSnapshot,
        TimeframeTelemetry,
    } from '../types';
    import { formatTimeframeLabel } from '../lib/telemetry';
    import styles from './StructuralAnchorsStrip.module.css';

    interface Props {
        /** Active timeframe telemetry. */
        tf: TimeframeTelemetry | undefined;
        /** Micro timeframe telemetry — used for cluster/liquidity (per docs,
         *  the cluster refresh is anchored to the micro candle cadence). */
        microTf: TimeframeTelemetry | undefined;
        /** Current price (number); comes from `priceText` of `tf`. */
        markPrice: number;
    }

    let { tf, microTf, markPrice }: Props = $props();

    // ── Collapse state (default = collapsed summary) ──
    let vpOpen = $state(false);
    let fibOpen = $state(false);
    let liqOpen = $state(false);

    // ── Volume Profile ──
    const vp = $derived<VolumeProfileSnapshot | null>(microTf?.volumeProfile ?? null);

    const vpHvn = $derived.by<VolumeProfileBin[]>(() => {
        if (!vp) return [];
        const mean = vp.bins.reduce((acc: number, b: VolumeProfileBin) => acc + b.volume, 0) / Math.max(1, vp.bins.length);
        if (!isFinite(mean) || mean <= 0) return [];
        return vp.bins
            .filter((b: VolumeProfileBin) => b.volume >= 1.5 * mean)
            .sort((a: VolumeProfileBin, b: VolumeProfileBin) => b.volume - a.volume)
            .slice(0, 3);
    });

    const vpBuySellSplit = $derived.by<{ buy: number; sell: number; bias: number }>(() => {
        if (!vp) return { buy: 0, sell: 0, bias: 0 };
        const buy = vp.bins.reduce((a: number, b: VolumeProfileBin) => a + b.buy_volume, 0);
        const sell = vp.bins.reduce((a: number, b: VolumeProfileBin) => a + b.sell_volume, 0);
        const total = buy + sell;
        return {
            buy,
            sell,
            bias: total > 0 ? (buy - sell) / total : 0,
        };
    });

    const vpPosition = $derived.by<{ inVa: boolean; rangePos: number }>(() => {
        if (!vp || !isFinite(markPrice) || markPrice <= 0) return { inVa: false, rangePos: 0 };
        const inVa = markPrice >= vp.value_area_low && markPrice <= vp.value_area_high;
        const range = vp.range_high - vp.range_low;
        const rangePos = range > 0 ? (markPrice - vp.range_low) / range : 0;
        return { inVa, rangePos };
    });

    const vpRefreshAge = $derived.by<number | null>(() => {
        if (!vp) return null;
        return Math.max(0, Date.now() - vp.timestamp_ms);
    });

    // ── Fibonacci ──
    const fibVals = $derived.by<Record<string, number | undefined>>(() => {
        const dto = tf?.indicators?.['fibonacci'];
        return (dto?.values ?? {}) as Record<string, number | undefined>;
    });

    const fibNorm = $derived.by<number>(() => {
        const dto = tf?.indicators?.['fibonacci'];
        return dto?.normalized ?? 0;
    });

    const fibGp = $derived.by<{ top: number | null; bottom: number | null }>(() => {
        const top = fibVals['gp_top'];
        const bottom = fibVals['gp_bottom'];
        return {
            top: typeof top === 'number' ? top : null,
            bottom: typeof bottom === 'number' ? bottom : null,
        };
    });

    const fibStatus = $derived.by<string>(() => {
        const { top, bottom } = fibGp;
        if (top == null || bottom == null || !isFinite(markPrice) || markPrice <= 0) return 'NO DATA';
        const lo = Math.min(top, bottom);
        const hi = Math.max(top, bottom);
        if (markPrice >= lo && markPrice <= hi) {
            const halfRange = (hi - lo) / 2;
            const dist = Math.abs(markPrice - (lo + halfRange)) / Math.max(halfRange, 1e-9);
            return `INSIDE GP (-${(dist * 100).toFixed(2)}% from center)`;
        }
        const ref = markPrice > hi ? hi : lo;
        const pct = ((markPrice - ref) / ref) * 100;
        const sign = pct >= 0 ? '+' : '';
        return markPrice > hi
            ? `${sign}${pct.toFixed(2)}% ABOVE GP`
            : `${pct.toFixed(2)}% BELOW GP`;
    });

    const fibSwing = $derived.by<'BULL' | 'BEAR' | 'NEUTRAL'>(() => {
        if (fibNorm > 0.05) return 'BULL';
        if (fibNorm < -0.05) return 'BEAR';
        return 'NEUTRAL';
    });

    const fibCoefficients: Array<{ key: string; label: string }> = [
        { key: 'fib_0236', label: '0.236' },
        { key: 'fib_0382', label: '0.382' },
        { key: 'fib_0500', label: '0.500' },
        { key: 'fib_0618', label: '0.618' },
        { key: 'fib_0660', label: '0.660' },
        { key: 'fib_0786', label: '0.786' },
    ];

    const fibExtTargets = $derived.by<{ ext1618: number | null; ext2618: number | null }>(() => {
        const a = fibVals['ext_1618'];
        const b = fibVals['ext_2618'];
        return {
            ext1618: typeof a === 'number' ? a : null,
            ext2618: typeof b === 'number' ? b : null,
        };
    });

    // ── Liquidity ──
    const cluster = $derived<LiquidationClusterMatrix | null>(microTf?.cluster ?? null);
    const flow = $derived<LiquidityFlow | null>(microTf?.liquidity ?? null);

    const shortClusters = $derived<LiquidationCluster[]>(cluster?.short_clusters ?? []);
    const longClusters = $derived<LiquidationCluster[]>(cluster?.long_clusters ?? []);

    /** Top 3 above-side clusters, sorted by closeness to current price. */
    const topAbove = $derived.by<LiquidationCluster[]>(() => {
        return [...shortClusters]
            .sort((a: LiquidationCluster, b: LiquidationCluster) => Math.abs(a.distance_from_mid_pct) - Math.abs(b.distance_from_mid_pct))
            .slice(0, 3);
    });

    /** Top 3 below-side clusters, sorted by closeness to current price. */
    const topBelow = $derived.by<LiquidationCluster[]>(() => {
        return [...longClusters]
            .sort((a: LiquidationCluster, b: LiquidationCluster) => Math.abs(a.distance_from_mid_pct) - Math.abs(b.distance_from_mid_pct))
            .slice(0, 3);
    });

    const oiSplit = $derived.by<{ longPct: number; shortPct: number } | null>(() => {
        if (!cluster) return null;
        const total = cluster.total_long_oi_usd + cluster.total_short_oi_usd;
        if (total <= 0) return null;
        return {
            longPct: (cluster.total_long_oi_usd / total) * 100,
            shortPct: (cluster.total_short_oi_usd / total) * 100,
        };
    });

    const cascadeState = $derived<string>(flow?.cascade_state ?? 'NONE');

    // ── Formatters ──
    function fmtPx(n: number | null | undefined): string {
        if (n == null || !isFinite(n) || n <= 0) return '—';
        if (markPrice >= 1000) return `$${n.toFixed(0)}`;
        if (markPrice >= 1) return `$${n.toFixed(2)}`;
        return `$${n.toFixed(4)}`;
    }

    function fmtUsd(n: number): string {
        if (!isFinite(n)) return '$0';
        if (Math.abs(n) >= 1_000_000) return `$${(n / 1_000_000).toFixed(2)}M`;
        if (Math.abs(n) >= 1_000) return `$${(n / 1_000).toFixed(1)}K`;
        return `$${n.toFixed(0)}`;
    }

    function fmtAge(ms: number | null): string {
        if (ms == null) return '—';
        const s = Math.floor(ms / 1000);
        if (s < 60) return `${s}s ago`;
        if (s < 3600) return `${Math.floor(s / 60)}m ago`;
        return `${Math.floor(s / 3600)}h ago`;
    }

    function fibDistancePct(): string {
        const { top, bottom } = fibGp;
        if (top == null || bottom == null || !isFinite(markPrice) || markPrice <= 0) return '—';
        const mid = (top + bottom) / 2;
        const half = Math.max((Math.abs(top - bottom) / 2), 1e-9);
        const d = ((markPrice - mid) / mid) * 100;
        const sign = d >= 0 ? '+' : '';
        return markPrice >= Math.min(top, bottom) && markPrice <= Math.max(top, bottom)
            ? `inside GP ${d >= 0 ? '+' : ''}${d.toFixed(2)}%`
            : `${sign}${d.toFixed(2)}%`;
    }

    function magnetBars(strength: number): string {
        const filled = Math.round((strength ?? 0) / 14);
        return '▓'.repeat(Math.min(7, Math.max(0, filled))) + '░'.repeat(7 - Math.min(7, Math.max(0, filled)));
    }

    function vpBinPct(v: number): number {
        if (!vp) return 0;
        const max = Math.max(...vp.bins.map((b) => b.volume), 1);
        return Math.min(100, Math.max(0, (v / max) * 100));
    }

    function cascadeClass(state: string): string {
        if (state === 'SUSTAINED') return styles.cascadeDanger ?? '';
        if (state === 'DETECTED') return styles.cascadeWarning ?? '';
        if (state === 'EXHAUSTED') return styles.cascadeCooling ?? '';
        return styles.cascadeNormal ?? '';
    }
</script>

<section class={styles.strip} aria-label="Structural anchors">
    <header class={styles.header}>
        <span class={styles.title}>STRUCTURAL ANCHORS</span>
        <span class={styles.subtitle}>Tier-2 structural context (always visible)</span>
    </header>

    <div class={styles.grid}>
        <!-- ── Volume Profile tile ── -->
        <article class={styles.tile}>
            <button
                class={styles.tileHeader}
                onclick={() => vpOpen = !vpOpen}
                aria-expanded={vpOpen}
            >
                <span class={styles.tileTitle}>VOLUME PROFILE</span>
                <span class={styles.tileCaret}>{vpOpen ? '▾' : '▸'}</span>
            </button>

            {#if !vp}
                <div class={styles.placeholder}>Awaiting volume profile…</div>
            {:else}
                <dl class={styles.kvList + ' ' + (vpOpen ? '' : (styles.collapsed ?? ''))}>
                    <div class={styles.kv}>
                        <dt class={styles.k}>POC</dt>
                        <dd class={styles.v + ' ' + (styles.vEmph ?? '')}>{fmtPx(vp.poc_price)}</dd>
                    </div>
                    <div class={styles.kv}>
                        <dt class={styles.k}>VAH</dt>
                        <dd class={styles.v}>{fmtPx(vp.value_area_high)}</dd>
                    </div>
                    <div class={styles.kv}>
                        <dt class={styles.k}>VAL</dt>
                        <dd class={styles.v}>{fmtPx(vp.value_area_low)}</dd>
                    </div>
                    <div class={styles.kv}>
                        <dt class={styles.k}>Range</dt>
                        <dd class={styles.v}>{fmtPx(vp.range_low)} – {fmtPx(vp.range_high)}</dd>
                    </div>
                </dl>
                <div class={styles.positionBadge + ' ' + (vpPosition.inVa ? styles.inVa ?? '' : styles.outVa ?? '')}>
                    {vpPosition.inVa ? 'INSIDE VALUE AREA' : `OUTSIDE VA · ${(vpPosition.rangePos * 100).toFixed(0)}% of range`}
                </div>

                {#if vpOpen}
                    <div class={styles.expand}>
                        <div class={styles.expandTitle}>Bins: {vp.num_bins} · Total Vol: {fmtUsd(vp.total_volume)}</div>
                        <div class={styles.expandTitle}>Refresh: {fmtAge(vpRefreshAge)}</div>

                        {#if vpHvn.length > 0}
                            <div class={styles.expandSection}>
                                <div class={styles.expandLabel}>Top HVN nodes</div>
                                {#each vpHvn as bin, i}
                                    <div class={styles.hvnRow}>
                                        <span class={styles.hvnRank}>{i + 1}</span>
                                        <span class={styles.hvnPrice}>{fmtPx((bin.price_low + bin.price_high) / 2)}</span>
                                        <span class={styles.hvnRange}>[{fmtPx(bin.price_low)}–{fmtPx(bin.price_high)}]</span>
                                        <span class={styles.hvnBar}>
                                            <span class={styles.hvnBarFill} style="width: {vpBinPct(bin.volume)}%"></span>
                                        </span>
                                        <span class={styles.hvnVol}>{fmtUsd(bin.volume)}</span>
                                    </div>
                                {/each}
                            </div>
                        {/if}

                        <div class={styles.expandSection}>
                            <div class={styles.expandLabel}>Buy/Sell split</div>
                            <div class={styles.bsSplit}>
                                <span class={styles.bsBuy} style="width: {(50 + vpBuySellSplit.bias * 50).toFixed(1)}%">
                                    BUY {fmtUsd(vpBuySellSplit.buy)}
                                </span>
                                <span class={styles.bsSell} style="width: {(50 - vpBuySellSplit.bias * 50).toFixed(1)}%">
                                    SELL {fmtUsd(vpBuySellSplit.sell)}
                                </span>
                            </div>
                            <div class={styles.expandLabel}>
                                Bias: {(vpBuySellSplit.bias * 100).toFixed(1)}%
                                ({vpBuySellSplit.bias > 0 ? 'buy-skewed' : vpBuySellSplit.bias < 0 ? 'sell-skewed' : 'balanced'})
                            </div>
                        </div>
                    </div>
                {/if}
            {/if}
        </article>

        <!-- ── Fibonacci tile ── -->
        <article class={styles.tile}>
            <button
                class={styles.tileHeader}
                onclick={() => fibOpen = !fibOpen}
                aria-expanded={fibOpen}
            >
                <span class={styles.tileTitle}>FIBONACCI</span>
                <span class={styles.tileCaret}>{fibOpen ? '▾' : '▸'}</span>
            </button>

            {#if fibGp.top == null || fibGp.bottom == null}
                <div class={styles.placeholder}>Awaiting fibonacci swing leg…</div>
            {:else}
                <dl class={styles.kvList}>
                    <div class={styles.kv}>
                        <dt class={styles.k}>GP Top</dt>
                        <dd class={styles.v + ' ' + (styles.vEmph ?? '')}>{fmtPx(fibGp.top)}</dd>
                    </div>
                    <div class={styles.kv}>
                        <dt class={styles.k}>GP Bot</dt>
                        <dd class={styles.v + ' ' + (styles.vEmph ?? '')}>{fmtPx(fibGp.bottom)}</dd>
                    </div>
                    <div class={styles.kv}>
                        <dt class={styles.k}>Direction</dt>
                        <dd class="{styles.v} {fibSwing === 'BULL' ? styles.bull ?? '' : fibSwing === 'BEAR' ? styles.bear ?? '' : styles.neutral ?? ''}">
                            {fibSwing} SWING
                        </dd>
                    </div>
                    <div class={styles.kv}>
                        <dt class={styles.k}>Status</dt>
                        <dd class={(styles.v ?? '') + ' ' + (styles.vMono ?? '')}>{fibStatus}</dd>
                    </div>
                </dl>
                <div class={styles.distanceBadge}>
                    Price vs GP: <span class={styles.distanceVal}>{fibDistancePct()}</span>
                </div>

                {#if fibOpen}
                    <div class={styles.expand}>
                        <div class={styles.expandSection}>
                            <div class={styles.expandLabel}>Retracement ladder</div>
                            <div class={styles.fibLadder}>
                                {#each fibCoefficients as coeff}
                                    {@const v = fibVals[coeff.key]}
                                    {@const isGp = coeff.key === 'fib_0618' || coeff.key === 'fib_0660'}
                                    <div class="{styles.fibRow} {isGp ? styles.fibRowGp ?? '' : ''} {typeof v === 'number' ? '' : styles.fibRowMissing ?? ''}">
                                        <span class={styles.fibCoeff}>{coeff.label}</span>
                                        <span class={styles.fibPrice}>{fmtPx(typeof v === 'number' ? v : null)}</span>
                                    </div>
                                {/each}
                            </div>
                        </div>
                        <div class={styles.expandSection}>
                            <div class={styles.expandLabel}>Extension targets</div>
                            {#if fibExtTargets.ext1618 || fibExtTargets.ext2618}
                                <div class={styles.extRow}>
                                    <span class={styles.extLabel}>1.618</span>
                                    <span class={styles.extPrice}>{fmtPx(fibExtTargets.ext1618)}</span>
                                </div>
                                <div class={styles.extRow}>
                                    <span class={styles.extLabel}>2.618</span>
                                    <span class={styles.extPrice}>{fmtPx(fibExtTargets.ext2618)}</span>
                                </div>
                            {:else}
                                <div class={styles.placeholder}>No extension data</div>
                            {/if}
                        </div>
                    </div>
                {/if}
            {/if}
        </article>

        <!-- ── Liquidity tile ── -->
        <article class={styles.tile}>
            <button
                class={styles.tileHeader}
                onclick={() => liqOpen = !liqOpen}
                aria-expanded={liqOpen}
            >
                <span class={styles.tileTitle}>LIQUIDITY</span>
                <span class={styles.tileCaret}>{liqOpen ? '▾' : '▸'}</span>
            </button>

            {#if !cluster}
                <div class={styles.placeholder}>Awaiting liquidation clusters…</div>
            {:else}
                <div class={styles.cascadeRow}>
                    <span class="{styles.cascadeBadge} {cascadeClass(cascadeState)}">
                        CASCADE {cascadeState}
                    </span>
                    <span class={styles.intensityNum}>{flow?.cascade_intensity?.toFixed(0) ?? '—'}/100</span>
                </div>
                {#if oiSplit}
                    <div class={styles.oiSplit}>
                        OI: <span class={styles.oiLong}>{oiSplit.longPct.toFixed(0)}% long</span> /
                        <span class={styles.oiShort}>{oiSplit.shortPct.toFixed(0)}% short</span>
                    </div>
                {/if}

                {#if topAbove.length > 0 || topBelow.length > 0}
                    <div class={styles.ladder} aria-label="Distance / strength ladder">
                        {#if topAbove.length > 0}
                            <div class={styles.ladderSection}>
                                <div class={styles.ladderSideLabel}>▴ ABOVE · short liq if dumped</div>
                                {#each topAbove as c}
                                    <div class={styles.ladderRow}>
                                        <span class={styles.ladderPrice}>{fmtPx(c.peak_price)}</span>
                                        <span class={(styles.ladderDist ?? '') + ' ' + (styles.ladderDistAbove ?? '')}>
                                            +{Math.abs(c.distance_from_mid_pct).toFixed(2)}%
                                        </span>
                                        <span class={styles.ladderNotional}>{fmtUsd(c.notional_usd)}</span>
                                        <span class={styles.ladderMagnet}>{magnetBars(c.magnet_strength)}</span>
                                    </div>
                                {/each}
                            </div>
                        {/if}

                        <div class={styles.ladderMid}>
                            <span>Current</span>
                            <span class={styles.ladderMidPrice}>{fmtPx(markPrice)}</span>
                        </div>

                        {#if topBelow.length > 0}
                            <div class={styles.ladderSection}>
                                <div class={styles.ladderSideLabel}>▾ BELOW · long liq if dropped</div>
                                {#each topBelow as c}
                                    <div class={styles.ladderRow}>
                                        <span class={styles.ladderPrice}>{fmtPx(c.peak_price)}</span>
                                        <span class={(styles.ladderDist ?? '') + ' ' + (styles.ladderDistBelow ?? '')}>
                                            −{Math.abs(c.distance_from_mid_pct).toFixed(2)}%
                                        </span>
                                        <span class={styles.ladderNotional}>{fmtUsd(c.notional_usd)}</span>
                                        <span class={styles.ladderMagnet}>{magnetBars(c.magnet_strength)}</span>
                                    </div>
                                {/each}
                            </div>
                        {/if}
                    </div>
                {:else}
                    <div class={styles.placeholder}>No clusters above noise threshold</div>
                {/if}

                {#if liqOpen && flow}
                    <div class={styles.expand}>
                        <div class={styles.expandSection}>
                            <div class={styles.expandLabel}>Latest bar (microTF)</div>
                            <div class={styles.liqStatRow}>
                                <span class={styles.liqStatLabel}>Long liqs</span>
                                <span class={styles.liqStatVal + ' ' + styles.bearish}>{fmtUsd(flow.long_liquidations_usd)}</span>
                            </div>
                            <div class={styles.liqStatRow}>
                                <span class={styles.liqStatLabel}>Short liqs</span>
                                <span class={styles.liqStatVal + ' ' + styles.bullish}>{fmtUsd(flow.short_liquidations_usd)}</span>
                            </div>
                            <div class={styles.liqStatRow}>
                                <span class={styles.liqStatLabel}>Net</span>
                                <span class={styles.liqStatVal}>{fmtUsd(flow.net_liquidation_usd)}</span>
                            </div>
                            <div class={styles.liqStatRow}>
                                <span class={styles.liqStatLabel}>Events</span>
                                <span class={styles.liqStatVal}>{flow.event_count}</span>
                            </div>
                            {#if flow.largest_event_usd > 0}
                                <div class={styles.liqStatRow}>
                                    <span class={styles.liqStatLabel}>Largest</span>
                                    <span class={styles.liqStatVal}>
                                        {fmtUsd(flow.largest_event_usd)} @ {fmtPx(flow.largest_event_price)}
                                        <span class={flow.largest_event_side === 'LONG' ? styles.bearish ?? '' : styles.bullish ?? ''}>
                                            ({flow.largest_event_side ?? '—'})
                                        </span>
                                    </span>
                                </div>
                            {/if}
                        </div>
                    </div>
                {/if}
            {/if}
        </article>
    </div>

    <footer class={styles.footer}>
        <span class={styles.footerHint}>
            Source: {tf ? formatTimeframeLabel(tf.barDurationSec) : '—'} (volume profile / fibonacci) · microTF (clusters / flow)
        </span>
    </footer>
</section>
