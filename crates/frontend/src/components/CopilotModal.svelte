<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import styles from './CopilotModal.module.css';
    import MomentumMeter from './MomentumMeter.svelte';
    import type { DecisionOutput } from '../types';

    const app = useAppStore();
    const copilotMicroInd = $derived(app.instancesMap[app.activeTab]?.microTerm?.indicators ?? {});
    const decision = $derived<DecisionOutput | null>(app.decisionOutput);
    const snap = $derived(app.latestSnapshot || {});
    const price = $derived(snap.mid_price ? parseFloat(String(snap.mid_price)) : null);
    const fb = $derived(decision?.factor_breakdown);

    let isOpen = $state(false);

    $effect(() => {
        if (decision) {
            isOpen = true;
        }
    });

    function close() { isOpen = false; }
    function handleBackdropClick() { close(); }
    function handleBackdropKeydown(e: KeyboardEvent) { if (e.key === 'Escape') close(); }
    function stopPropagation(e: Event) { e.stopPropagation(); }
</script>

{#if isOpen && decision}
    <!-- svelte-ignore a11y_interactive_supports_focus -->
    <div class={styles.modalBackdrop} onclick={handleBackdropClick} onkeydown={handleBackdropKeydown} role="dialog" tabindex="0">
        <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
        <div class={styles.modalWindow} onclick={stopPropagation} onkeydown={stopPropagation} role="document">
            <div class={styles.modalHeader}>
                <h2 class={styles.modalTitle}>Decision Analysis — {app.activeSymbol}</h2>
                <button class={styles.modalCloseBtn} onclick={close}>&#10005;</button>
            </div>

            <div class={styles.modalBody}>
                <div class={styles.modalLeft}>
                    <div class={styles.masterSynthesis}>
                        <h3 class={styles.sectionHeading}>Decision Result</h3>

                        <div class={styles.srRibbon}>
                            <div class="{styles.srBlock} {styles.srCurrent}">
                                <span class={styles.srLabel}>PRICE</span>
                                <span class="{styles.srLevel} {styles.srPriceLabel}">{price !== null ? price.toFixed(4) : '--'}</span>
                            </div>
                        </div>

                        <div class="{styles.decisionCallout} {decision.action === 'Hold' || decision.action === 'Open Long' ? styles.decisionGreen : decision.action === 'Close' ? styles.decisionRed : styles.decisionAmber}">
                            <span class={styles.decisionAction}>{decision.action}</span>
                            <span class={styles.decisionTrend}>{decision.confidence}% confidence</span>
                            <p class={styles.decisionRationale}>{decision.rationale}</p>

                            <div style="height:4px;border-radius:2px;background:#1c212e;margin-top:10px;">
                                <div style="height:100%;border-radius:2px;background:{decision.confidence > 70 ? 'var(--success, #66bb6a)' : decision.confidence > 40 ? 'var(--warning, #ffa726)' : 'var(--danger, #ef5350)'};width:{decision.confidence}%;"></div>
                            </div>
                        </div>

                        {#if decision.risk_notes && decision.risk_notes !== 'No significant risk flags.'}
                            <div class={styles.synthesisSummary}>
                                <span class={styles.synthCount}>Risk</span>
                                <p class={styles.synthEval}>{decision.risk_notes}</p>
                            </div>
                        {/if}
                    </div>

                    {#if fb}
                        <div>
                            <h3 class={styles.sectionHeading}>Factor Breakdown</h3>
                            <div class={styles.indicatorGrid}>
                                <div class="{styles.phaseOneCard} {fb.confluence_norm >= 5 ? styles.pocBullish : styles.pocSideways}">
                                    <span class={styles.pocName}>Confluence</span>
                                    <p class={styles.pocReason}>{fb.confluence_norm.toFixed(1)}</p>
                                </div>
                                <div class="{styles.phaseOneCard} {fb.trade_readiness >= 5 ? styles.pocBullish : styles.pocSideways}">
                                    <span class={styles.pocName}>Readiness</span>
                                    <p class={styles.pocReason}>{fb.trade_readiness.toFixed(1)}</p>
                                </div>
                                <div class="{styles.phaseOneCard} {fb.trade_quality >= 5 ? styles.pocBullish : styles.pocSideways}">
                                    <span class={styles.pocName}>Quality</span>
                                    <p class={styles.pocReason}>{fb.trade_quality.toFixed(1)}</p>
                                </div>
                                <div class="{styles.phaseOneCard} {fb.safety_score >= 5 ? styles.pocBullish : styles.pocSideways}">
                                    <span class={styles.pocName}>Safety</span>
                                    <p class={styles.pocReason}>{fb.safety_score.toFixed(1)}</p>
                                </div>
                                <div class="{styles.phaseOneCard} {fb.trend_persistence >= 5 ? styles.pocBullish : styles.pocSideways}">
                                    <span class={styles.pocName}>Trend</span>
                                    <p class={styles.pocReason}>{fb.trend_persistence.toFixed(1)}</p>
                                </div>
                                <div class="{styles.phaseOneCard} {fb.regime_confidence >= 5 ? styles.pocBullish : styles.pocSideways}">
                                    <span class={styles.pocName}>Regime</span>
                                    <p class={styles.pocReason}>{fb.regime_confidence.toFixed(1)}</p>
                                </div>
                                <div class="{styles.phaseOneCard} {fb.breakout_confidence >= 5 ? styles.pocBullish : styles.pocSideways}">
                                    <span class={styles.pocName}>Breakout</span>
                                    <p class={styles.pocReason}>{fb.breakout_confidence.toFixed(1)}</p>
                                </div>
                                <div class="{styles.phaseOneCard} {fb.signal_decay <= 3 ? styles.pocBullish : styles.pocSideways}">
                                    <span class={styles.pocName}>Decay</span>
                                    <p class={styles.pocReason}>{fb.signal_decay.toFixed(1)}</p>
                                </div>
                            </div>
                            <div class={styles.synthesisSummary}>
                                <span class={styles.synthCount}>Final Score</span>
                                <p class={styles.synthEval}>
                                    {fb.final_score.toFixed(1)} / 10 · Regime {fb.regime} · Gates {fb.hard_gates_passed}
                                </p>
                            </div>
                        </div>
                    {/if}

                    <h3 class={styles.sectionHeading}>Momentum Meters</h3>
                    <div class={styles.momentumMeters}>
                        <MomentumMeter label="RSI" normalized={copilotMicroInd['rsi']?.normalized ?? 0} stateLabel={copilotMicroInd['rsi']?.state_label ?? 'UNKNOWN'} />
                        <MomentumMeter label="MACD" normalized={copilotMicroInd['macd']?.normalized ?? 0} stateLabel={copilotMicroInd['macd']?.state_label ?? 'UNKNOWN'} />
                        <MomentumMeter label="SQUEEZE" normalized={copilotMicroInd['squeeze']?.normalized ?? 0} stateLabel={copilotMicroInd['squeeze']?.state_label ?? 'UNKNOWN'} />
                    </div>
                </div>
            </div>
        </div>
    </div>
{/if}
