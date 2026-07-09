<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import styles from './ObservabilityHub.module.css';
    import { iRaw, emaStackState, atrVolatilityRegime } from '../lib/telemetry';
    const app = useAppStore();

    const pairKey = $derived(app.activeTab);
    const pair = $derived(app.instancesMap[pairKey]);
    const microMap = $derived(pair?.microTerm?.indicators ?? {});
    const atrRegime = $derived(atrVolatilityRegime(microMap));
    const emaStack = $derived(emaStackState(microMap));
    const bbwpVal = $derived(iRaw(microMap, 'bbwp'));
    const bbwpText = $derived(bbwpVal == null ? '--' : bbwpVal.toFixed(1));
    const rvolVal = $derived(iRaw(microMap, 'rvol'));

    let interval: ReturnType<typeof setInterval> | undefined;

    $effect(() => {
        if (pairKey) {
            app.fetchSystemStatus();
            app.fetchObservabilityBuffers(pairKey);

            clearInterval(interval);
            interval = setInterval(() => {
                app.fetchSystemStatus();
                app.fetchObservabilityBuffers(pairKey);
            }, 5000);
        }

        return () => {
            clearInterval(interval);
        };
    });

    function formatPnl(v: number | undefined): string {
        if (v == null || isNaN(v)) return '$0.00';
        return (v >= 0 ? '+' : '') + '$' + v.toFixed(2);
    }

    function formatPct(v: number | undefined): string {
        if (v == null || isNaN(v)) return '0.0%';
        return (v >= 0 ? '+' : '') + v.toFixed(1) + '%';
    }

    function formatTime(ts: number): string {
        if (!ts) return '--:--:--';
        return new Date(ts).toLocaleTimeString();
    }
</script>

<div class={styles.obsHud + ' animate-fade'}>
    {#if pair && app.systemHeartbeat}
        <!-- Top Row: Vital Senses Metrics -->
        <div class={styles.vitalStatsGrid}>
            <div class={styles.vitalCard}>
                <span class={styles.vitalLabel}>Websocket Status</span>
                <span class="{styles.vitalValue} {app.systemHeartbeat.connected ? styles.textGreen : styles.textRed}">
                    {app.systemHeartbeat.connected ? 'CONNECTED' : 'DISCONNECTED'}
                </span>
            </div>
            <div class={styles.vitalCard}>
                <span class={styles.vitalLabel}>Core Latency</span>
                <span class={styles.vitalValue + ' ' + styles.textBlue}>{app.systemHeartbeat.latency_ms} ms</span>
            </div>
        </div>

        <!-- Middle Row: Market Regime Classification -->
        <div class="{styles.regimeBanner} {atrRegime === 'expanding' ? styles.regimeTrending : atrRegime === 'contracting' ? styles.regimeCompression : styles.regimeStable}">
            <div class={styles.regimeTitle}>
                ACTIVE VOLATILITY REGIME: {atrRegime.toUpperCase()}
            </div>
            <div class={styles.regimeMetrics + ' font-mono'}>
                <span>BBWP Percentile: {bbwpText}%</span>
                <span>Relative Volume (RVOL): {rvolVal != null ? rvolVal.toFixed(2) : '--'}</span>
            </div>
        </div>

        <!-- Parallel Agent Progress Map -->
        <div class={styles.agentMatrixCard}>
            <h3 class={styles.cardTitle}>Parallel Agent Matrix</h3>
            <div class={styles.agentGrid}>
                <!-- Trend Agent -->
                <div class="{styles.agentNode} {emaStack !== 'tangled' ? styles.complete : ''}">
                    <div class={styles.agentNodeHeader}>
                        <span class={styles.agentNodeName}>TREND AGENT</span>
                        <span class={styles.agentNodeStatus}>{emaStack.toUpperCase()}</span>
                    </div>
                    <p class={styles.agentNodeThought}>
                        Evaluating trend fanning order. Stacking order is: {emaStack.toUpperCase()}.
                        Price relative to long EMA: {pair.microTerm.priceText}.
                    </p>
                </div>

                <!-- Volatility Agent -->
                <div class="{styles.agentNode} {bbwpText !== '--' ? styles.complete : ''}">
                    <div class={styles.agentNodeHeader}>
                        <span class={styles.agentNodeName}>VOLATILITY AGENT</span>
                        <span class={styles.agentNodeStatus}>{atrRegime.toUpperCase()}</span>
                    </div>
                    <p class={styles.agentNodeThought}>
                        Monitoring Bollinger Band compression limits and ATR average true range lines.
                        BBWP Percentile currently evaluating at {bbwpText}%.
                    </p>
                </div>

                <!-- Structure Agent -->
                <div class="{styles.agentNode} {app.markedSupportLevels.length > 0 ? styles.complete : ''}">
                    <div class={styles.agentNodeHeader}>
                        <span class={styles.agentNodeName}>STRUCTURE AGENT</span>
                        <span class={styles.agentNodeStatus}>ACTIVE</span>
                    </div>
                    <p class={styles.agentNodeThought}>
                        Parsing pivot high and low points.
                        Tracking active levels: Support [{app.markedSupportLevels.slice(0,2).join(', ')}] |
                        Resistance [{app.markedResistanceLevels.slice(0,2).join(', ')}].
                    </p>
                </div>

                <!-- Risk Agent -->
                <div class="{styles.agentNode} {app.paperMaxRiskPct > 0 ? styles.complete : ''}">
                    <div class={styles.agentNodeHeader}>
                        <span class={styles.agentNodeName}>RISK GATE AGENT</span>
                        <span class={styles.agentNodeStatus}>{app.paperMaxRiskPct}% RISK</span>
                    </div>
                    <p class={styles.agentNodeThought}>
                        Enforcing dynamic portfolio limits. Maximum allowable risk per trade: {app.paperMaxRiskPct}%.
                        Leverage bound is limited strictly to {app.paperLeverage}x.
                    </p>
                </div>

                <!-- Position Agent -->
                <div class="{styles.agentNode} {pair.currentPosition !== 'None' ? styles.complete : ''}">
                    <div class={styles.agentNodeHeader}>
                        <span class={styles.agentNodeName}>POSITION AGENT</span>
                        <span class={styles.agentNodeStatus}>{pair.currentPosition?.toUpperCase()}</span>
                    </div>
                    <p class={styles.agentNodeThought}>
                        Monitoring open execution states. Active position: {pair.currentPosition}.
                        Stop Loss target configured dynamically at ${app.paperInvalidationLevel.toFixed(2)}.
                    </p>
                </div>
            </div>
        </div>

        <!-- Bottom Row: Historical Decision & Trade Buffers -->
        <div class={styles.buffersTwoCol}>
            <div class="{styles.bufferBox} {styles.card}">
                <h4 class={styles.cardTitle}>Decision Memory Buffer</h4>
                <div class={styles.bufferList}>
                    {#if app.recentDecisions.length === 0}
                        <p class={styles.bufferEmpty}>No logged decision runs in memory.</p>
                    {:else}
                        {#each app.recentDecisions as dec (dec.id)}
                            <div class={styles.bufferRow}>
                                <span class={styles.bufferTime}>{formatTime(dec.timestamp)}</span>
                                <span class={'buffer-regime ' + styles.textBlue + ' font-semibold'}>{dec.regime_classification}</span>
                                <span class={styles.bufferAction}>{dec.confidence_score}% conf</span>
                                <span class={'buffer-score font-mono'}>{dec.eight_factor_score} pts</span>
                            </div>
                        {/each}
                    {/if}
                </div>
            </div>

            <div class="{styles.bufferBox} {styles.card}">
                <h4 class={styles.cardTitle}>Completed Trades Buffer</h4>
                <div class={styles.bufferList}>
                    {#if app.completedTrades.length === 0}
                        <p class={styles.bufferEmpty}>No completed trade ledger entries discovered.</p>
                    {:else}
                        {#each app.completedTrades as trade (trade.id)}
                            <div class={styles.bufferRow}>
                                <span class={styles.bufferTime}>{formatTime(trade.closed_at)}</span>
                                <span class={'buffer-dir ' + (trade.realized_pnl >= 0 ? styles.textGreen : styles.textRed)}>
                                    {trade.direction}
                                </span>
                                <span class={'buffer-pnl font-semibold ' + (trade.realized_pnl >= 0 ? styles.textGreen : styles.textRed)}>
                                    {formatPnl(trade.realized_pnl)} ({formatPct(trade.roi_pct)})
                                </span>
                                <span class="buffer-score font-mono">{trade.execution_score.toFixed(1)}/10</span>
                            </div>
                        {/each}
                    {/if}
                </div>
            </div>
        </div>
    {/if}
</div>

