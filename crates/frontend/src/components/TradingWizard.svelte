<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import { requestAssistantAnalysis } from '../lib/analysis.svelte';
    import styles from './TradingWizard.module.css';

    const app = useAppStore();

    let currentStep = $state(1);
    let entryPrice = $state(app.entryPriceVal);
    let stopLoss = $state(app.stopLossVal);

    const STEPS = [
        { num: 1, label: 'Setup' },
        { num: 2, label: 'Trigger' },
        { num: 3, label: 'Confirmation' },
        { num: 4, label: 'Execution' },
    ];

    function getStepClass(stepNum: number): string {
        if (stepNum < currentStep) return styles.completed;
        if (stepNum === currentStep) return styles.active;
        return '';
    }

    function canProceedFromStep1(): boolean {
        if (app.currentPosition === 'None') return true;
        const ep = parseFloat(entryPrice);
        return !isNaN(ep) && ep > 0;
    }

    function goNext() {
        if (currentStep < 4) {
            if (currentStep === 1) {
                app.entryPriceVal = entryPrice;
                app.stopLossVal = stopLoss;
            }
            if (currentStep === 2) {
                requestAnalysis();
                return;
            }
            currentStep++;
        }
    }

    function goBack() {
        if (currentStep > 1) currentStep--;
    }

    async function requestAnalysis() {
        currentStep = 2;
        await requestAssistantAnalysis(app);
        if (app.wizardResponse) {
            currentStep = 3;
        }
    }

    function getActionClass(action: string): string {
        if (action === 'Hold' || action === 'Open Long') return styles.actionGreen;
        if (action === 'Close') return styles.actionRed;
        return styles.actionAmber;
    }

    const snap = () => app.microTerm.latestSnapshot || {};
    const s = $derived(snap());
</script>

<div class={styles.wizardContainer}>
    <!-- Progress Stepper -->
    <div class={styles.stepperWrapper}>
        <div class={styles.stepper}>
            {#each STEPS as step, i}
                <div class={styles.stepDot}>
                    <div class="{styles.stepCircle} {getStepClass(step.num)}">
                        {currentStep > step.num ? '✓' : step.num}
                    </div>
                    {#if i < STEPS.length - 1}
                        <div class="{styles.stepConnector} {currentStep > step.num ? styles.completed : ''}"></div>
                    {/if}
                </div>
            {/each}
        </div>
        <div style="display:flex;gap:12px;margin-top:6px;">
            {#each STEPS as step}
                <span class="{styles.stepLabel} {getStepClass(step.num)}">{step.label}</span>
            {/each}
        </div>
    </div>

    <div class={styles.stepContent}>
        <!-- Step 1: Setup -->
        {#if currentStep === 1}
            <div class={styles.stepCard}>
                <h3 class={styles.stepTitle}>Step 1: Setup</h3>
                <p class={styles.stepDesc}>Select your current trading position and entry details.</p>
                <div class={styles.setupForm}>
                    <div class={styles.inputGroup}>
                        <span class={styles.inputGroup} style="margin-bottom:4px;">Current Position</span>
                        <div class={styles.positionRadioGroup}>
                            <label>
                                <input type="radio" bind:group={app.currentPosition} value="None" />
                                <span>None</span>
                            </label>
                            <label>
                                <input type="radio" bind:group={app.currentPosition} value="Long" />
                                <span>Long</span>
                            </label>
                            <label>
                                <input type="radio" bind:group={app.currentPosition} value="Short" />
                                <span>Short</span>
                            </label>
                        </div>
                    </div>

                    {#if app.currentPosition !== 'None'}
                        <div class={styles.inputGroup}>
                            <label for="ep">Entry Price ($)</label>
                            <input id="ep" type="number" step="any" bind:value={entryPrice} placeholder="0.00" />
                        </div>
                        <div class={styles.inputGroup}>
                            <label for="sl">Stop Loss ($) — optional</label>
                            <input id="sl" type="number" step="any" bind:value={stopLoss} placeholder="0.00" />
                        </div>
                    {/if}
                </div>

                <div class={styles.navButtons}>
                    <div></div>
                    <button class={styles.btnPrimary} onclick={goNext} disabled={!canProceedFromStep1()}>
                        Next: Trigger →
                    </button>
                </div>
            </div>

        <!-- Step 2: Trigger -->
        {:else if currentStep === 2}
            <div class={styles.stepCard}>
                <h3 class={styles.stepTitle}>Step 2: Trigger Analysis</h3>
                <p class={styles.stepDesc}>Review current market conditions and request AI analysis.</p>

                {#if s && Object.keys(s).length > 0}
                    <div class={styles.marketSnapshot}>
                        <div class={styles.snapshotItem}>
                            <div class={styles.snapshotLabel}>Price</div>
                            <div class={styles.snapshotValue}>{app.priceText !== '--' ? '$' + app.priceText : '--'}</div>
                        </div>
                        {#if s.rsi_14}
                            {@const rsi = parseFloat(String(s.rsi_14))}
                            <div class={styles.snapshotItem}>
                                <div class={styles.snapshotLabel}>RSI (14)</div>
                                <div class="{styles.snapshotValue} {rsi > 70 ? styles.down : rsi < 30 ? styles.up : ''}">
                                    {rsi.toFixed(1)}
                                </div>
                            </div>
                        {/if}
                        {#if s.macd_hist}
                            {@const h = parseFloat(String(s.macd_hist))}
                            <div class={styles.snapshotItem}>
                                <div class={styles.snapshotLabel}>MACD Hist</div>
                                <div class="{styles.snapshotValue} {h > 0 ? styles.up : styles.down}">
                                    {h.toFixed(4)}
                                </div>
                            </div>
                        {/if}
                        {#if s.adx_14}
                            <div class={styles.snapshotItem}>
                                <div class={styles.snapshotLabel}>ADX</div>
                                <div class={styles.snapshotValue}>{parseFloat(String(s.adx_14)).toFixed(1)}</div>
                            </div>
                        {/if}
                        {#if s.squeeze_on != null}
                            <div class={styles.snapshotItem}>
                                <div class={styles.snapshotLabel}>Squeeze</div>
                                <div class={styles.snapshotValue}>{s.squeeze_on ? 'ON' : 'OFF'}</div>
                            </div>
                        {/if}
                        {#if s.atr_14}
                            <div class={styles.snapshotItem}>
                                <div class={styles.snapshotLabel}>ATR</div>
                                <div class={styles.snapshotValue}>{parseFloat(String(s.atr_14)).toFixed(4)}</div>
                            </div>
                        {/if}
                    </div>
                {/if}

                {#if app.assistantLoading}
                    <div class={styles.loadingBox}>
                        <div class={styles.loadingSpinner}></div>
                        <p class={styles.loadingText}>
                            AI Analyst is preparing the market report...
                        </p>
                    </div>
                {:else}
                    <p style="color: var(--text-muted, #888); font-size: 13px; text-align: center; margin: 18px 0;">
                        Position: <strong>{app.currentPosition}</strong>{#if app.currentPosition !== 'None'} | Entry: ${entryPrice || '0.00'}{/if}
                    </p>
                {/if}

                {#if app.assistantError}
                    <div class={styles.errorBox}>{app.assistantError}</div>
                {/if}

                <div class={styles.navButtons}>
                    <button class={styles.btnSecondary} onclick={goBack}>← Back</button>
                    <button class={styles.btnPrimary} onclick={requestAnalysis} disabled={app.assistantLoading}>
                        {app.assistantLoading ? 'Analyzing...' : 'Request AI Analysis'}
                    </button>
                </div>
            </div>

        <!-- Step 3: Confirmation (Analyst Document) -->
        {:else if currentStep === 3 && app.wizardResponse}
            {@const doc = app.wizardResponse.analyst_document}
            <div class={styles.stepCard}>
                <h3 class={styles.stepTitle}>Step 3: Market Analysis Report</h3>
                <p class={styles.stepDesc}>AI Analyst's structured market assessment. Review before proceeding to the decision.</p>

                <div class={styles.analystDoc}>
                    <div class={styles.docSection}>
                        <div class={styles.docSectionTitle}>Market Summary</div>
                        <div class={styles.docSectionBody}>{doc.market_summary}</div>
                    </div>
                    <div class={styles.docSection}>
                        <div class={styles.docSectionTitle}>Trend Indicators</div>
                        <div class={styles.docSectionBody}>{doc.trend_indicators}</div>
                    </div>
                    <div class={styles.docSection}>
                        <div class={styles.docSectionTitle}>Momentum Indicators</div>
                        <div class={styles.docSectionBody}>{doc.momentum_indicators}</div>
                    </div>
                    <div class={styles.docSection}>
                        <div class={styles.docSectionTitle}>Volatility Indicators</div>
                        <div class={styles.docSectionBody}>{doc.volatility_indicators}</div>
                    </div>
                    <div class={styles.docSection}>
                        <div class={styles.docSectionTitle}>Volume & Flow</div>
                        <div class={styles.docSectionBody}>{doc.volume_indicators}</div>
                    </div>
                    <div class={styles.docSection}>
                        <div class={styles.docSectionTitle}>Structure (S/R, Fib, Patterns)</div>
                        <div class={styles.docSectionBody}>{doc.structure_indicators}</div>
                    </div>
                    <div class={styles.docSection}>
                        <div class={styles.docSectionTitle}>Active Signals</div>
                        <div class={styles.docSectionBody}>{doc.active_signals}</div>
                    </div>
                    <div class={styles.docSection}>
                        <div class={styles.docSectionTitle}>Confluence Summary</div>
                        <div class={styles.docSectionBody}>{doc.confluence_summary}</div>
                    </div>
                </div>

                <div class={styles.navButtons}>
                    <button class={styles.btnSecondary} onclick={goBack}>← Back</button>
                    <button class={styles.btnPrimary} onclick={() => currentStep = 4}>
                        Proceed to Decision →
                    </button>
                </div>
            </div>

        <!-- Step 4: Execution (Trader Decision) -->
        {:else if currentStep === 4 && app.wizardResponse}
            {@const dec = app.wizardResponse.trader_decision}
            <div class={styles.stepCard}>
                <h3 class={styles.stepTitle}>Step 4: Trading Decision</h3>
                <p class={styles.stepDesc}>AI Trader's final decision based on the analyst's report.</p>

                <div class={styles.decisionBlock}>
                    <div class="{styles.actionBadge} {getActionClass(dec.action)}">
                        {dec.action}
                    </div>

                    <div class={styles.confidenceBar}>
                        <span class={styles.confidenceLabel}>Confidence</span>
                        <div class={styles.confidenceTrack}>
                            <div class={styles.confidenceFill}
                                style="width: {dec.confidence}%; background: {dec.confidence > 70 ? 'var(--success, #66bb6a)' : dec.confidence > 40 ? 'var(--warning, #ffa726)' : 'var(--danger, #ef5350)'};">
                            </div>
                        </div>
                        <span class={styles.confidencePct}>{dec.confidence}%</span>
                    </div>

                    <div class={styles.rationaleBox}>
                        <div class={styles.rationaleTitle}>Decision Rationale</div>
                        <div class={styles.rationaleText}>{dec.rationale}</div>
                    </div>

                    {#if dec.risk_notes && dec.risk_notes !== 'No significant risk flags.'}
                        <div class={styles.riskBox}>
                            <div class={styles.rationaleTitle}>⚠ Risk Notes</div>
                            <div class={styles.rationaleText}>{dec.risk_notes}</div>
                        </div>
                    {/if}
                </div>

                <div class={styles.navButtons}>
                    <button class={styles.btnSecondary} onclick={goBack}>← Back to Report</button>
                    <button class={styles.btnPrimary} onclick={() => currentStep = 1}>
                        New Analysis ↻
                    </button>
                </div>
            </div>
        {/if}
    </div>
</div>
