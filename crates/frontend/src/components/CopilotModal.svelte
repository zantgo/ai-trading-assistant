<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import styles from './CopilotModal.module.css';

    const app = useAppStore();
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

{#if app.isAssistantModalOpen && app.multiAgentResponse}
    {@const resp = app.multiAgentResponse!}
    {@const pt = resp.phase_two}
    {@const indicators = resp.phase_one}
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
                        <h3 class={styles.sectionHeading}>Phase 2 — Master Synthesis</h3>

                        <div class={styles.srRibbon}>
                            <div class="{styles.srBlock} {styles.srSupport}">
                                <span class={styles.srLabel}>SUPPORT</span>
                                {#if pt.support_and_resistance.detected_support_levels.length > 0}
                                    {#each pt.support_and_resistance.detected_support_levels as lvl}
                                        <span class={styles.srLevel}>{lvl}</span>
                                    {/each}
                                {:else}
                                    <span class="{styles.srLevel} {styles.srNone}">None</span>
                                {/if}
                            </div>
                            <div class="{styles.srBlock} {styles.srCurrent}">
                                <span class={styles.srLabel}>PRICE</span>
                                <span class="{styles.srLevel} {styles.srPriceLabel}">{price !== null ? price.toFixed(4) : '--'}</span>
                            </div>
                            <div class="{styles.srBlock} {styles.srResistance}">
                                <span class={styles.srLabel}>RESISTANCE</span>
                                {#if pt.support_and_resistance.detected_resistance_levels.length > 0}
                                    {#each pt.support_and_resistance.detected_resistance_levels as lvl}
                                        <span class={styles.srLevel}>{lvl}</span>
                                    {/each}
                                {:else}
                                    <span class="{styles.srLevel} {styles.srNone}">None</span>
                                {/if}
                            </div>
                        </div>
                        <p class={styles.srStructural}>{pt.support_and_resistance.structural_analysis}</p>

                        <div class="{styles.decisionCallout} {pt.position_recommendation.action === 'Hold' || pt.position_recommendation.action === 'Open Long' ? styles.decisionGreen : pt.position_recommendation.action === 'Close' ? styles.decisionRed : styles.decisionAmber}">
                            <span class={styles.decisionAction}>{pt.position_recommendation.action}</span>
                            <span class={styles.decisionTrend}>{pt.general_trend}</span>
                            <p class={styles.decisionRationale}>{pt.position_recommendation.rationale}</p>
                        </div>

                        <div class={styles.synthesisSummary}>
                            <span class={styles.synthCount}>{pt.indicator_synthesis.summary_count}</span>
                            <p class={styles.synthEval}>{pt.indicator_synthesis.evaluation}</p>
                        </div>
                    </div>

                    <h3 class={styles.sectionHeading}>Phase 1 — Individual Indicator Agents</h3>
                    <div class={styles.indicatorGrid}>
                        {#each indicators as ind}
                            <div class="{styles.phaseOneCard} {ind.signal === 'BULLISH' ? styles.pocBullish : ind.signal === 'BEARISH' ? styles.pocBearish : ind.signal === 'SIDEWAYS' ? styles.pocSideways : ind.signal === 'UNAVAILABLE' ? styles.pocUnavailable : ''} {ind.divergence_status === 'potential' ? styles.divPotential : ''} {ind.divergence_status === 'confirmed' ? styles.divConfirmed : ''}">
                                <span class={styles.pocName}>{ind.indicator_name}</span>
                                <span class={styles.pocSignal}>{ind.signal}</span>
                                {#if ind.divergence_status === 'potential'}
                                    <span class="{styles.divBadge} {styles.divBadgePotential}">POTENTIAL</span>
                                {:else if ind.divergence_status === 'confirmed'}
                                    <span class="{styles.divBadge} {styles.divBadgeConfirmed}">{ind.divergence_type === 'bullish' ? '✓ BULLISH' : '✗ BEARISH'}</span>
                                {/if}
                                <p class={styles.pocReason}>{ind.reason}</p>
                            </div>
                        {/each}
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
