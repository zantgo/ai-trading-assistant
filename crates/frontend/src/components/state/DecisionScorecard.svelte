<script lang="ts">
    import styles from './DecisionScorecard.module.css';
    import type { DecisionContext } from '../../types';

    interface Props {
        dc: DecisionContext | null | undefined;
    }
    let { dc }: Props = $props();

    function q(v: number): number {
        return Math.round(Math.max(0, Math.min(1, v)) * 100);
    }
    function qColor(v: number): string {
        if (v >= 0.66) return '#10b981';
        if (v >= 0.4) return '#f59e0b';
        return '#ef4444';
    }
    function biasColor(b: number): string {
        if (b > 0.1) return '#10b981';
        if (b < -0.1) return '#ef4444';
        return '#94a3b8';
    }

    // Trade Readiness decomposition (mirrors decision_context.rs synthesis weights).
    const inputs = $derived(
        dc
            ? [
                  { label: 'Trade Quality', value: dc.trade_quality, weight: 0.30 },
                  { label: 'Safety (1−Risk)', value: 1 - dc.risk_level, weight: 0.25 },
                  { label: 'Market Quality', value: dc.market_quality, weight: 0.20 },
                  { label: 'Trend Persistence', value: dc.trend_persistence, weight: 0.15 },
                  { label: 'Regime Confidence', value: dc.regime_confidence, weight: 0.10 },
              ]
            : [],
    );
    const readiness = $derived(dc?.trade_readiness ?? 0);
    const bias = $derived(dc?.directional_bias ?? 0);
</script>

<div class={styles.card}>
    <div class={styles.head}>
        <span class={styles.title}>DECISION CONTEXT</span>
    </div>

    {#if dc}
        <div class={styles.readinessRow}>
            <div class={styles.readinessGauge}>
                <div class={styles.readinessFill} style="width:{q(readiness)}%;background:{qColor(readiness)}"></div>
            </div>
            <div class={styles.readinessNum} style="color:{qColor(readiness)}">
                <span class={styles.readinessVal}>{q(readiness)}</span>
                <span class={styles.readinessLbl}>TRADE READINESS</span>
            </div>
        </div>

        <div class={styles.breakdown}>
            {#each inputs as it}
                <div class={styles.inputRow} title="weight {Math.round(it.weight * 100)}%">
                    <span class={styles.inputLabel}>{it.label}</span>
                    <span class={styles.inputWeight}>×{it.weight.toFixed(2)}</span>
                    <span class={styles.inputBarWrap}>
                        <span class={styles.inputBar} style="width:{q(it.value)}%;background:{qColor(it.value)}"></span>
                    </span>
                    <span class={styles.inputVal} style="color:{qColor(it.value)}">{q(it.value)}</span>
                </div>
            {/each}
        </div>

        <div class={styles.stats}>
            <div class={styles.stat}>
                <span class={styles.statLabel}>BIAS</span>
                <span class={styles.statVal} style="color:{biasColor(bias)}">{bias >= 0 ? '+' : ''}{bias.toFixed(2)}</span>
            </div>
            <div class={styles.stat}>
                <span class={styles.statLabel}>P(BULL)</span>
                <span class={styles.statVal} style="color:#10b981">{q(dc.bullish_probability)}%</span>
            </div>
            <div class={styles.stat}>
                <span class={styles.statLabel}>P(BEAR)</span>
                <span class={styles.statVal} style="color:#ef4444">{q(dc.bearish_probability)}%</span>
            </div>
            <div class={styles.stat}>
                <span class={styles.statLabel}>CONSENSUS</span>
                <span class={styles.statVal}>{q(dc.consensus)}%</span>
            </div>
            <div class={styles.stat}>
                <span class={styles.statLabel}>RECOMM STOP</span>
                <span class={styles.statVal}>${dc.recommended_stop?.toFixed(2) ?? '—'}</span>
            </div>
            <div class={styles.stat}>
                <span class={styles.statLabel}>R:R</span>
                <span class={styles.statVal}>{dc.reward_risk_ratio.toFixed(2)}</span>
            </div>
            <div class={styles.stat}>
                <span class={styles.statLabel}>EXP VOL</span>
                <span class={styles.statVal}>{dc.expected_volatility.toFixed(1)}</span>
            </div>
        </div>
    {:else}
        <div class={styles.empty}>Awaiting first completed candle…</div>
    {/if}
</div>
