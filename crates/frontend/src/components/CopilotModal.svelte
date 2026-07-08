<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import styles from './CopilotModal.module.css';
    import MomentumMeter from './MomentumMeter.svelte';

    const app = useAppStore();
    const copilotMicroInd = $derived(app.instancesMap[app.activeTab]?.microTerm?.indicators ?? {});
    let chatContainer = $state<HTMLDivElement | null>(null);

    function closeModal() {
        app.isAssistantModalOpen = false;
    }

    function scrollChatToBottom() {
        requestAnimationFrame(() => {
            if (chatContainer) {
                chatContainer.scrollTop = chatContainer.scrollHeight;
            }
        });
    }

    async function sendChatMessage() {
        const text = app.chatInputText.trim();
        if (!text || app.isChatLoading) return;

        app.chatHistory.push({ role: 'user', content: text });
        app.chatInputText = '';
        app.isChatLoading = true;
        scrollChatToBottom();

        try {
            const res = await fetch('/api/chat', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ history: app.chatHistory }),
            });

            if (!res.ok) {
                throw new Error(`Server returned ${res.status}`);
            }

            const data = await res.json();
            app.chatHistory.push({ role: 'assistant', content: data.reply });
            scrollChatToBottom();
        } catch (e: any) {
            app.chatHistory.push({
                role: 'assistant',
                content: `Sorry, I couldn't process that request: ${e.message || 'Unknown error'}`,
            });
            scrollChatToBottom();
        } finally {
            app.isChatLoading = false;
        }
    }

    function handleBackdropClick() {
        closeModal();
    }

    function handleBackdropKeydown(e: KeyboardEvent) {
        if (e.key === 'Escape') closeModal();
    }

    function stopPropagation(e: Event) {
        e.stopPropagation();
    }
</script>

{#if app.isAssistantModalOpen && app.wizardResponse}
    {@const resp = app.wizardResponse}
    {@const analyst = resp.analyst_document}
    {@const trader = resp.trader_decision}
    {@const snap = app.latestSnapshot || {}}
    {@const price = snap.mid_price ? parseFloat(String(snap.mid_price)) : null}

    <!-- svelte-ignore a11y_interactive_supports_focus -->
    <div class={styles.modalBackdrop} onclick={handleBackdropClick} onkeydown={handleBackdropKeydown} role="dialog" tabindex="0">
        <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
        <div class={styles.modalWindow} onclick={stopPropagation} onkeydown={stopPropagation} role="document">
            <div class={styles.modalHeader}>
                <h2 class={styles.modalTitle}>AI Copilot Intelligence Hub — {app.activeSymbol}</h2>
                <button class={styles.modalCloseBtn} onclick={closeModal}>&#10005;</button>
            </div>

            <div class={styles.modalBody}>
                <div class={styles.modalLeft}>
                    <div class={styles.masterSynthesis}>
                        <h3 class={styles.sectionHeading}>Analyst Report</h3>

                        <div class={styles.srRibbon}>
                            <div class="{styles.srBlock} {styles.srCurrent}">
                                <span class={styles.srLabel}>PRICE</span>
                                <span class="{styles.srLevel} {styles.srPriceLabel}">{price !== null ? price.toFixed(4) : '--'}</span>
                            </div>
                        </div>

                        <p class={styles.srStructural}>{analyst.market_summary}</p>

                        <div class="{styles.decisionCallout} {trader.action === 'Hold' || trader.action === 'Open Long' ? styles.decisionGreen : trader.action === 'Close' ? styles.decisionRed : styles.decisionAmber}">
                            <span class={styles.decisionAction}>{trader.action}</span>
                            <span class={styles.decisionTrend}>{trader.confidence}% confidence</span>
                            <p class={styles.decisionRationale}>{trader.rationale}</p>
                        </div>

                        {#if trader.risk_notes && trader.risk_notes !== 'No significant risk flags.'}
                            <div class={styles.synthesisSummary}>
                                <span class={styles.synthCount}>Risk</span>
                                <p class={styles.synthEval}>{trader.risk_notes}</p>
                            </div>
                        {/if}
                    </div>

                    <h3 class={styles.sectionHeading}>Momentum Meters</h3>
                    <div class={styles.momentumMeters}>
                        <MomentumMeter label="RSI" normalized={copilotMicroInd['rsi']?.normalized ?? 0} stateLabel={copilotMicroInd['rsi']?.state_label ?? 'UNKNOWN'} />
                        <MomentumMeter label="MACD" normalized={copilotMicroInd['macd']?.normalized ?? 0} stateLabel={copilotMicroInd['macd']?.state_label ?? 'UNKNOWN'} />
                        <MomentumMeter label="SQUEEZE" normalized={copilotMicroInd['squeeze']?.normalized ?? 0} stateLabel={copilotMicroInd['squeeze']?.state_label ?? 'UNKNOWN'} />
                    </div>

                    <h3 class={styles.sectionHeading}>Market Analysis Document</h3>
                    <div class={styles.indicatorGrid}>
                        <div class="{styles.phaseOneCard} {styles.pocBullish}">
                            <span class={styles.pocName}>Trend</span>
                            <p class={styles.pocReason}>{analyst.trend_indicators}</p>
                        </div>
                        <div class="{styles.phaseOneCard} {styles.pocBearish}">
                            <span class={styles.pocName}>Momentum</span>
                            <p class={styles.pocReason}>{analyst.momentum_indicators}</p>
                        </div>
                        <div class="{styles.phaseOneCard} {styles.pocSideways}">
                            <span class={styles.pocName}>Volatility</span>
                            <p class={styles.pocReason}>{analyst.volatility_indicators}</p>
                        </div>
                        <div class="{styles.phaseOneCard}">
                            <span class={styles.pocName}>Volume</span>
                            <p class={styles.pocReason}>{analyst.volume_indicators}</p>
                        </div>
                        <div class="{styles.phaseOneCard}">
                            <span class={styles.pocName}>Structure</span>
                            <p class={styles.pocReason}>{analyst.structure_indicators}</p>
                        </div>
                        <div class="{styles.phaseOneCard}">
                            <span class={styles.pocName}>Signals</span>
                            <p class={styles.pocReason}>{analyst.active_signals}</p>
                        </div>
                        <div class="{styles.phaseOneCard}">
                            <span class={styles.pocName}>Confluence</span>
                            <p class={styles.pocReason}>{analyst.confluence_summary}</p>
                        </div>
                    </div>
                </div>

                <div class={styles.modalRight}>
                    <h3 class={styles.sectionHeading}>Real-time Chat</h3>
                    <div class={styles.chatThread} bind:this={chatContainer}>
                        {#each app.chatHistory.filter(m => m.role !== 'system') as msg, i (i)}
                            <div class="{styles.chatBubble} {msg.role === 'user' ? styles.userBubble : styles.assistantBubble}">
                                <span class={styles.bubbleRole}>{msg.role === 'user' ? 'You' : 'Assistant'}</span>
                                <span class={styles.bubbleContent}>{msg.content}</span>
                            </div>
                        {/each}
                        {#if app.isChatLoading}
                            <div class="{styles.chatBubble} {styles.assistantBubble} {styles.typingBubble}">
                                <span class={styles.bubbleRole}>Assistant</span>
                                <span class={styles.bubbleContent}><span class={styles.typingDots}>Thinking<span class={styles.dotAnim}>.</span><span class={styles.dotAnim}>.</span><span class={styles.dotAnim}>.</span></span></span>
                            </div>
                        {/if}
                    </div>

                    <div class={styles.chatInputArea}>
                        <input type="text" class={styles.chatInput} placeholder="Ask details..." bind:value={app.chatInputText} disabled={app.isChatLoading} onkeydown={(e) => { if (e.key === 'Enter') sendChatMessage() }} />
                        <button class={styles.chatSendBtn} onclick={sendChatMessage} disabled={app.isChatLoading || !app.chatInputText.trim()}>Send</button>
                    </div>
                </div>
            </div>
        </div>
    </div>
{/if}
