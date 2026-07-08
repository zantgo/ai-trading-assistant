<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import { requestAssistantAnalysis, openAssistantChat } from '../lib/analysis.svelte';
    import { fmtPrice } from '../lib/telemetry';
    import styles from '../App.module.css';
    import panel from './AiAssistantPanel.module.css';

    const app = useAppStore();

    async function requestAnalysis() {
        await requestAssistantAnalysis(app);
    }

    function openAssistantModal() {
        openAssistantChat(app, () => null);
    }
</script>

<div class={styles.workspaceInnerContent + " " + 'animate-fade'}>
    <aside class={styles.sidebarPanel + " " + 'font-sans'}>
        <div class={styles.sidebarSection + " " + styles.signalsBox}>
            <h3 class={styles.sectionTitle}>AI ASSISTANT</h3>
            <div class={styles.signalsContent}>
                <div class={styles.positionSelector}>
                    <span class={styles.subTitle}>Current Position:</span>
                    <label>
                        <input type="radio" bind:group={app.currentPosition} value="None" /> None
                    </label>
                    <label>
                        <input type="radio" bind:group={app.currentPosition} value="Long" /> Long
                    </label>
                    <label>
                        <input type="radio" bind:group={app.currentPosition} value="Short" /> Short
                    </label>
                </div>

                {#if app.currentPosition !== 'None'}
                    <div class={styles.entryPriceInput}>
                        <label for="entryPrice">Entry Price ($):</label>
                        <input id="entryPrice" type="number" step="any"
                               bind:value={app.entryPriceVal} placeholder="0.00" />
                    </div>
                    <div class="{styles.entryPriceInput} {panel.entryPriceInputOffset}">
                        <label for="stopLoss">Stop Loss ($):</label>
                        <input id="stopLoss" type="number" step="any"
                               bind:value={app.stopLossVal} placeholder="0.00" />
                        <small class={panel.entryHint}>
                            Left blank? Defaults to 1% risk distance.
                        </small>
                    </div>
                {/if}

                {#if app.currentPosition !== 'None' && app.commissionProjection}
                    <div class={styles.commissionQuickSummary} class:cc-quick-viable={app.commissionProjection.trade_viable} class:cc-quick-not-viable={!app.commissionProjection.trade_viable}>
                        <span class={styles.ccQuickLabel}>Commission Check:</span>
                        <span class={styles.ccQuickFees}>Fees: ${app.commissionProjection.fee_breakdown.total_fees.toFixed(2)}</span>
                        <span class={styles.ccQuickNet}>Net: ${app.commissionProjection.max_gain_net_after_fees.toFixed(2)}</span>
                        <span class={styles.ccQuickBadge}>{app.commissionProjection.trade_viable ? '✓ Viable' : '✗ Not Viable'}</span>
                    </div>
                {/if}

                <button class={styles.analyzeBtn} onclick={requestAnalysis} disabled={app.assistantLoading}>
                    {app.assistantLoading ? 'Analyzing Market...' : 'Request AI Assistant Analysis'}
                </button>

                {#if app.assistantLoading}
                    <div class={styles.loadingIndicator}>
                        <span class={styles.dot + " " + styles.pulseBlue}></span>
                        <span class={styles.statusText}>
                            Two-Agent Pipeline: Analyzing market data...
                        </span>
                    </div>
                {/if}

                {#if app.assistantError}
                    <div class={styles.errorBox}>
                        <span>Failed: {app.assistantError}</span>
                    </div>
                {/if}

                {#if app.wizardResponse && !app.assistantLoading}
                    {@const resp = app.wizardResponse}
                    {@const analyst = resp.analyst_document}
                    {@const trader = resp.trader_decision}
                    <div class={styles.analysisResult + " " + styles.clickableResult} onclick={openAssistantModal} role="button" tabindex="0" onkeydown={(e) => { if (e.key === 'Enter') openAssistantModal() }}>
                        <div class="{styles.resultBlock} {styles.reveal} {panel.revealDelay0}">
                            <h4 class={styles.resultStageTitle}>Analyst Report</h4>
                            <p class={styles.resultReasoning}>{analyst.market_summary.substring(0, 120)}...</p>
                        </div>
                        <div class="{styles.resultBlock} {styles.reveal} {panel.revealDelay150}">
                            <h4 class={styles.resultStageTitle}>Confluence</h4>
                            <p class={styles.resultReasoning}>{analyst.confluence_summary.substring(0, 100)}...</p>
                        </div>
                        <div class="{styles.resultBlock} {styles.resultAction} {styles.reveal} {panel.revealDelay300}">
                            <h4 class={styles.resultStageTitle}>Trader Decision</h4>
                            <span class={styles.actionCall} class:action-green={trader.action === 'Hold' || trader.action === 'Open Long'} class:action-red={trader.action === 'Close'} class:action-amber={trader.action === 'Wait' || trader.action === 'Open Short'}>
                                {trader.action} ({trader.confidence}%)
                            </span>
                            <p class={styles.resultReasoning}>{trader.rationale.substring(0, 150)}...</p>
                            {#if trader.risk_notes && trader.risk_notes !== 'No significant risk flags.'}
                                <div class={styles.riskNotes}>⚠ {trader.risk_notes}</div>
                            {/if}
                            {#if app.riskProfile?.profile}
                                <div class={styles.riskContext}>
                                    <span>IRML: {app.riskProfile.profile.permission} · Risk {Math.round(app.riskProfile.profile.overall_risk * 100)}%</span>
                                </div>
                            {/if}
                        </div>
                        <div class={styles.clickHint}>Click for full analysis & chat</div>
                    </div>
                {:else if !app.assistantLoading && !app.assistantError}
                    <p class={styles.signalsPlaceholder}>
                        Select your current position and request an AI market analysis using the two-agent pipeline.
                    </p>
                {/if}
            </div>
        </div>

        <div class={styles.sidebarSection + " " + styles.historyBox}>
            <h3 class={styles.sectionTitle}>ANALYSIS HISTORY</h3>
            <div class={styles.historyContent}>
                {#if app.assistantHistory.length === 0}
                    <p class={styles.signalsPlaceholder}>No history recorded yet.</p>
                {:else}
                    <div class={styles.historyTableWrap}>
                        <table class={styles.historyTable}>
                            <thead>
                                <tr>
                                    <th>Time</th>
                                    <th>Pos</th>
                                    <th>Action</th>
                                    <th>Entry $</th>
                                    <th>Δ%</th>
                                </tr>
                            </thead>
                            <tbody>
                                {#each app.assistantHistory as rec}
                                    {@const recPrice = parseFloat(rec.price_at_analysis) || 0}
                                    {@const latestPrice = parseFloat(app.historyLatestClose) || 0}
                                    {@const delta = recPrice > 0 ? ((latestPrice - recPrice) / recPrice * 100) : 0}
                                    <tr>
                                        <td class={styles.colTime}>{rec.created_at.substring(11, 19)}</td>
                                        <td>{rec.position}</td>
                                        <td class={styles.colAction} class:action-text-green={rec.recommended_action === 'Hold' || rec.recommended_action === 'Open Long'} class:action-text-red={rec.recommended_action === 'Close'} class:action-text-amber={rec.recommended_action === 'Wait' || rec.recommended_action === 'Open Short'}>
                                            {rec.recommended_action.substring(0, 4)}
                                        </td>
                                        <td class={styles.colPrice}>{fmtPrice(recPrice, recPrice)}</td>
                                        <td class={styles.colDelta} class:delta-positive={delta > 0} class:delta-negative={delta < 0}>{delta.toFixed(2)}%</td>
                                    </tr>
                                {/each}
                            </tbody>
                        </table>
                    </div>
                {/if}
            </div>
        </div>
    </aside>
</div>
