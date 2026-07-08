<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import type { StatisticalContext, StatisticValue } from '../types';
    import styles from './StatisticalPanel.module.css';

    const app = useAppStore();

    interface ProbEntry { label: string; value: number; obs: number }
    interface DistEntry { label: string; stats: StatisticValue }

    function currentCtx(): StatisticalContext | null {
        const tab = app.activeTab;
        if (!tab || !app.instancesMap[tab]) return null;
        const pair = app.instancesMap[tab];
        const snap = (pair.microTerm as any)?.latestSnapshot;
        if (!snap) return null;
        const ctx = snap.statistical_context as StatisticalContext | undefined;
        if (!ctx || ctx.price_stats === undefined) return null;
        return ctx;
    }

    function fmtPercent(v: number): string { return (v * 100).toFixed(1) + '%'; }
    function fmtNum(v: number, d = 2): string { return Number(v).toFixed(d); }

    function shapeClass(label: string): string {
        const m: Record<string, string> = {
            normal: styles.shapeNormal, compressed: styles.shapeCompressed,
            explosive: styles.shapeExplosive, chaotic: styles.shapeChaotic,
            rare: styles.shapeRare, asymmetric: styles.shapeNormal,
        };
        return m[label] ?? styles.shapeNormal;
    }

    function probClass(v: number): string {
        if (v > 0.6) return styles.probHigh;
        if (v > 0.3) return styles.probMed;
        return styles.probLow;
    }

    function distEntries(ctx: StatisticalContext): DistEntry[] {
        return [
            { label: 'Price', stats: ctx.price_stats },
            { label: 'ATR', stats: ctx.atr_stats },
            { label: 'RSI', stats: ctx.rsi_stats },
            { label: 'BBWP', stats: ctx.bbwp_stats },
        ];
    }

    function probEntries(ctx: StatisticalContext): ProbEntry[] {
        const obs = ctx.observation_counts ?? {};
        return [
            { label: 'Trend Continuation', value: ctx.trend_continuation_prob, obs: obs.trend_continuation ?? 0 },
            { label: 'Mean Reversion', value: ctx.mean_reversion_prob, obs: obs.mean_reversion ?? 0 },
            { label: 'Breakout Success', value: ctx.breakout_success_prob, obs: obs.breakout_success ?? 0 },
            { label: 'Reversal', value: ctx.reversal_prob, obs: obs.reversal ?? 0 },
            { label: 'ATR Expansion', value: ctx.atr_expansion_prob, obs: obs.atr_expansion ?? 0 },
            { label: 'Squeeze Release', value: ctx.squeeze_release_prob, obs: obs.squeeze_release ?? 0 },
            { label: 'Stop Before Target', value: ctx.stop_before_target_prob, obs: obs.stop_before_target ?? 0 },
        ];
    }

    $effect(() => { void app.microTerm?.latestSnapshot; });
</script>

{#if currentCtx()}
    {@const ctx = currentCtx() as StatisticalContext}
    <div class={styles.panel}>
        <!-- Market Shape -->
        <div class={styles.sectionTitle}>Market Shape</div>
        <div class={styles.grid}>
            <div class={styles.card}>
                <div class={styles.cardTitle}>Classification</div>
                <span class={styles.shapeBadge + ' ' + shapeClass(ctx.market_shape_label)}>
                    {ctx.market_shape_label}
                </span>
            </div>
            <div class={styles.card}>
                <div class={styles.cardTitle}>Volatility Percentile</div>
                <div class={styles.cardValue}>{fmtNum(ctx.volatility_percentile, 1)}%</div>
                <div class={styles.probBar}><div class="{styles.probFill} {probClass(ctx.volatility_percentile / 100)}" style="width:{ctx.volatility_percentile}%"></div></div>
            </div>
            <div class={styles.card}>
                <div class={styles.cardTitle}>Compression Percentile</div>
                <div class={styles.cardValue}>{fmtNum(ctx.compression_percentile, 1)}%</div>
                <div class={styles.probBar}><div class="{styles.probFill} {probClass(ctx.compression_percentile / 100)}" style="width:{ctx.compression_percentile}%"></div></div>
            </div>
            <div class={styles.card}>
                <div class={styles.cardTitle}>Entropy / Predictability</div>
                <div class={styles.cardValue}>{fmtNum(ctx.entropy, 2)} / {fmtPercent(ctx.market_predictability)}</div>
            </div>
            <div class={styles.card}>
                <div class={styles.cardTitle}>Tail Risk</div>
                <div class={styles.cardValue}>{fmtNum(ctx.tail_risk, 1)}</div>
                <div class={styles.cardSub}>{ctx.tail_risk > 2 ? 'Elevated left tail' : 'Normal tails'}</div>
            </div>
            <div class={styles.card}>
                <div class={styles.cardTitle}>Skewness / Kurtosis</div>
                <div class={styles.cardValue}>{fmtNum(ctx.skewness, 2)} / {fmtNum(ctx.kurtosis, 1)}</div>
            </div>
        </div>

        <!-- Distribution -->
        <div class={styles.sectionTitle}>Price Distribution</div>
        <div class={styles.grid}>
            {#each distEntries(ctx) as entry}
                <div class={styles.card}>
                    <div class={styles.cardTitle}>{entry.label}</div>
                    <div class={styles.rowBetween}>
                        <span class={styles.statLabel}>Current</span>
                        <span class={styles.statNumber}>{fmtNum(entry.stats.current, 1)}</span>
                    </div>
                    <div class={styles.rowBetween}>
                        <span class={styles.statLabel}>Mean</span>
                        <span class={styles.statNumber}>{fmtNum(entry.stats.mean, 1)}</span>
                    </div>
                    <div class={styles.rowBetween}>
                        <span class={styles.statLabel}>%ile</span>
                        <span class={styles.statNumber}>{fmtNum(entry.stats.percentile, 0)}%</span>
                    </div>
                    <div class={styles.rowBetween}>
                        <span class={styles.statLabel}>Z-score</span>
                        <span class={styles.statNumber}>{fmtNum(entry.stats.z_score, 2)}&sigma;</span>
                    </div>
                    <div class={styles.rowBetween}>
                        <span class={styles.statLabel}>Trend</span>
                        <span class={styles.statNumber}>{entry.stats.trend}</span>
                    </div>
                </div>
            {/each}
        </div>

        <!-- Probabilities -->
        <div class={styles.sectionTitle}>Empirical Probabilities</div>
        <div class={styles.grid}>
            {#each probEntries(ctx) as entry}
                <div class={styles.card}>
                    <div class={styles.cardTitle}>{entry.label}</div>
                    <div class={styles.cardValue}>{fmtPercent(entry.value)}</div>
                    <div class={styles.probBar}>
                        <div class="{styles.probFill} {probClass(entry.value)}" style="width:{Math.max(entry.value * 100, 3)}%"></div>
                    </div>
                    <div class={styles.cardSub}>{entry.obs} observations</div>
                </div>
            {/each}
        </div>

        <!-- Confidence & Reliability -->
        <div class={styles.sectionTitle}>Confidence & Reliability</div>
        <div class={styles.grid}>
            <div class={styles.card}>
                <div class={styles.cardTitle}>Confidence Score</div>
                <div class={styles.largeNumber}>{fmtNum(ctx.confidence_score, 1)}</div>
                <div class={styles.cardSub}>/ 100</div>
            </div>
            <div class={styles.card}>
                <div class={styles.cardTitle}>Historical Reliability</div>
                <div class={styles.cardValue}>{fmtPercent(ctx.historical_reliability)}</div>
            </div>
            <div class={styles.card}>
                <div class={styles.cardTitle}>95% Prediction Interval</div>
                <div class={styles.cardValue}>{fmtNum(ctx.prediction_interval_95[0], 2)}% &ndash; {fmtNum(ctx.prediction_interval_95[1], 2)}%</div>
            </div>
            <div class={styles.card}>
                <div class={styles.cardTitle}>Bootstrap 95% CI</div>
                <div class={styles.cardValue}>{fmtNum(ctx.bootstrap_confidence_95[0], 2)}% &ndash; {fmtNum(ctx.bootstrap_confidence_95[1], 2)}%</div>
            </div>
        </div>

        <!-- Relationships -->
        <div class={styles.sectionTitle}>Indicator Relationships</div>
        <div class={styles.grid}>
            <div class={styles.card}>
                <div class={styles.cardTitle}>Feature Agreement</div>
                <div class={styles.cardValue}>{fmtPercent(ctx.feature_agreement)}</div>
            </div>
            <div class={styles.card}>
                <div class={styles.cardTitle}>Indicator Redundancy</div>
                <div class={styles.cardValue}>{fmtPercent(ctx.indicator_redundancy)}</div>
                <div class={styles.cardSub}>{ctx.indicator_redundancy > 0.7 ? 'Many indicators aligned' : 'Indicators diverse'}</div>
            </div>
            <div class={styles.card}>
                <div class={styles.cardTitle}>Consensus Stability</div>
                <div class={styles.cardValue}>{fmtPercent(ctx.consensus_stability)}</div>
            </div>
            <div class={styles.card}>
                <div class={styles.cardTitle}>Trend Consistency</div>
                <div class={styles.cardValue}>{fmtNum(ctx.trend_consistency, 3)}</div>
            </div>
            <div class={styles.card}>
                <div class={styles.cardTitle}>Momentum Consistency</div>
                <div class={styles.cardValue}>{fmtPercent(ctx.momentum_consistency)}</div>
            </div>
        </div>

        <!-- Monte Carlo -->
        <div class={styles.sectionTitle}>Monte Carlo Projection</div>
        <div class={styles.grid}>
            <div class={styles.card}>
                <div class={styles.cardTitle}>P(Target Hit)</div>
                <div class={styles.cardValue}>{fmtPercent(ctx.mc_target_hit_prob)}</div>
            </div>
            <div class={styles.card}>
                <div class={styles.cardTitle}>P(Stop Hit)</div>
                <div class={styles.cardValue}>{fmtPercent(ctx.mc_stop_hit_prob)}</div>
            </div>
            <div class={styles.card}>
                <div class={styles.cardTitle}>Expected Movement</div>
                <div class={styles.cardValue}>{fmtNum(ctx.mc_expected_movement, 2)}%</div>
            </div>
            <div class={styles.card}>
                <div class={styles.cardTitle}>95% DD / MFE</div>
                <div class={styles.cardValue}>{fmtNum(ctx.mc_max_drawdown_95, 2)}% / {fmtNum(ctx.mc_max_favorable_excursion_95, 2)}%</div>
            </div>
            <div class={styles.card}>
                <div class={styles.cardTitle}>Best / Worst / Median</div>
                <div class={styles.cardValue}>{fmtNum(ctx.mc_best_case, 2)}% / {fmtNum(ctx.mc_worst_case, 2)}% / {fmtNum(ctx.mc_median_outcome, 2)}%</div>
            </div>
            <div class={styles.card}>
                <div class={styles.cardTitle}>95% Outcome Range</div>
                <div class={styles.cardValue}>{fmtNum(ctx.mc_confidence_95_range[0], 2)}% &ndash; {fmtNum(ctx.mc_confidence_95_range[1], 2)}%</div>
            </div>
        </div>

        <!-- Kalman Drift Estimate -->
        {#if ctx.kalman_trend_strength > 0 || ctx.kalman_drift !== 0}
            <div class={styles.sectionTitle}>Kalman Drift Estimate</div>
            <div class={styles.grid}>
                <div class={styles.card}>
                    <div class={styles.cardTitle}>Drift (annualized)</div>
                    <div class={styles.cardValue}>{fmtNum(ctx.kalman_drift * 100, 3)}%</div>
                    <div class={styles.cardSub}>{ctx.kalman_drift > 0 ? 'Upward bias' : ctx.kalman_drift < 0 ? 'Downward bias' : 'Flat'}</div>
                </div>
                <div class={styles.card}>
                    <div class={styles.cardTitle}>Noise Volatility</div>
                    <div class={styles.cardValue}>{fmtNum(ctx.kalman_noise_vol, 3)}%</div>
                </div>
                <div class={styles.card}>
                    <div class={styles.cardTitle}>Trend Strength (SNR)</div>
                    <div class={styles.cardValue}>{fmtNum(ctx.kalman_trend_strength, 2)}</div>
                    <div class={styles.probBar}><div class="{styles.probFill} {probClass(Math.min(ctx.kalman_trend_strength, 1))}" style="width:{Math.min(ctx.kalman_trend_strength * 100, 100)}%"></div></div>
                </div>
            </div>
        {/if}

        <!-- Derived Features -->
        <div class={styles.sectionTitle}>Derived Decision Features</div>
        <div class={styles.grid}>
            <div class={styles.card}>
                <div class={styles.cardTitle}>Market Stretch</div>
                <div class={styles.cardValue}>{fmtNum(ctx.market_stretch_score, 2)}</div>
            </div>
            <div class={styles.card}>
                <div class={styles.cardTitle}>Trend Reliability</div>
                <div class={styles.cardValue}>{fmtPercent(ctx.trend_reliability)}</div>
            </div>
            <div class={styles.card}>
                <div class={styles.cardTitle}>Momentum Stability</div>
                <div class={styles.cardValue}>{fmtPercent(ctx.momentum_stability)}</div>
            </div>
            <div class={styles.card}>
                <div class={styles.cardTitle}>Breakout Confidence</div>
                <div class={styles.cardValue}>{fmtPercent(ctx.breakout_confidence)}</div>
            </div>
            <div class={styles.card}>
                <div class={styles.cardTitle}>Trend Confidence</div>
                <div class={styles.cardValue}>{fmtPercent(ctx.trend_confidence)}</div>
            </div>
            <div class={styles.card}>
                <div class={styles.cardTitle}>Risk Confidence</div>
                <div class={styles.cardValue}>{fmtPercent(ctx.risk_confidence)}</div>
            </div>
            <div class={styles.card}>
                <div class={styles.cardTitle}>Volatility Shock</div>
                <div class={styles.cardValue}>{fmtPercent(ctx.volatility_shock_prob)}</div>
            </div>
            <div class={styles.card}>
                <div class={styles.cardTitle}>Expected Opportunity</div>
                <div class={styles.cardValue}>{fmtNum(ctx.expected_opportunity, 3)}</div>
            </div>
        </div>

        <!-- Anomaly -->
        {#if ctx.anomaly_score > 0.3}
            <div class={styles.sectionTitle}>Anomaly Detection</div>
            <div class={styles.card + ' ' + styles.anomalyBlock}>
                <div class={styles.cardValue} style="color:#f59e0b">{fmtPercent(ctx.anomaly_score)}</div>
                <div class={styles.cardSub}>{ctx.top_anomaly_reason || 'Market behaving unusually'}</div>
            </div>
        {/if}

        <!-- Top Predictors -->
        {#if ctx.top_predictive_indicators && ctx.top_predictive_indicators.length > 0}
            <div class={styles.sectionTitle}>Top Predictive Indicators</div>
            <div class={styles.card}>
                {#each ctx.top_predictive_indicators as pair_}
                    <div class={styles.pairRow}>
                        <span class={styles.pairName}>{pair_[0]}</span>
                        <span class={styles.pairValue}>{fmtNum(pair_[1], 3)}</span>
                    </div>
                {/each}
            </div>
        {/if}
    </div>
{:else}
    <div class={styles.emptyState}>
        Statistical context will appear here once data arrives.
        Awaiting first completed candle snapshot...
    </div>
{/if}
