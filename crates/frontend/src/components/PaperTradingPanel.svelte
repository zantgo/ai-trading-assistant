<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import styles from '../App.module.css';

    const app = useAppStore();

    let draftTpLevels = $state(1);
    let draftSlLevels = $state(1);
    let draftCostInputPrice = $state(app.costPriceInput);
    let draftCostOutputPrice = $state(app.costPriceOutput);
    let costSaveStatus = $state<'idle' | 'saving' | 'success' | 'error'>('idle');

    async function saveCostConfig() {
        costSaveStatus = 'saving';
        try {
            const res = await fetch('/api/config/costs', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    price_per_1m_input_tokens: Number(draftCostInputPrice),
                    price_per_1m_output_tokens: Number(draftCostOutputPrice),
                }),
            });
            costSaveStatus = res.ok ? 'success' : 'error';
            if (res.ok) {
                app.costPriceInput = Number(draftCostInputPrice);
                app.costPriceOutput = Number(draftCostOutputPrice);
                setTimeout(() => { costSaveStatus = 'idle'; }, 2000);
            }
        } catch (_) {
            costSaveStatus = 'error';
        }
    }
</script>

<div class={styles.workspaceInnerContent + " " + 'animate-fade'}>
    <div class={styles.paperLayout}>
        <div class={styles.paperPositionsCol}>
            <h3 class={styles.cardTitle} style="margin-top: 0;">Active Paper Position</h3>
            {#if app.activePaperPosition}
                {@const pos = app.activePaperPosition as any}
                <div class={styles.paperPositionCard} class:direction-long={pos.direction === 'LONG'} class:direction-short={pos.direction === 'SHORT'}>
                    <div class={styles.ppHeader}>
                        <span class={styles.ppDirection}>{pos.direction}</span>
                        <span class={styles.ppSymbol}>{pos.symbol}</span>
                    </div>
                    <div class={styles.ppDetails}>
                        <div class={styles.ppRow}><span>Entry Price:</span><span>${(pos.entry_price ?? 0).toFixed(2)}</span></div>
                        <div class={styles.ppRow}><span>Size:</span><span>{(pos.size ?? 0).toFixed(4)} units</span></div>
                        <div class={styles.ppRow}><span>Allocated:</span><span>${(pos.allocated_usd ?? 0).toFixed(2)}</span></div>
                    </div>
                    <div class={styles.ppPnlSection}>
                        <div class={styles.ppRow}><span>Unrealized P&L:</span>
                            <span class:pnl-positive={app.paperUnrealizedPnl >= 0} class:pnl-negative={app.paperUnrealizedPnl < 0}>
                                {app.paperUnrealizedPnl >= 0 ? '+' : ''}${app.paperUnrealizedPnl.toFixed(2)}
                            </span>
                        </div>
                        <div class={styles.ppRow}><span>ROI:</span>
                            <span class:pnl-positive={app.paperUnrealizedRoi >= 0} class:pnl-negative={app.paperUnrealizedRoi < 0}>
                                {app.paperUnrealizedRoi.toFixed(2)}%
                            </span>
                        </div>
                    </div>
                    <!-- Scale-In Progress Cockpit -->
                    <div class={styles.scaleInCockpit}>
                        <div class={styles.ppRow}><span>Avg Entry Price:</span><span class={styles.mono}>${app.paperAvgEntryPrice.toFixed(2)}</span></div>
                        <div class={styles.ppRow}><span>Portions Filled:</span><span class={styles.mono}>{app.paperFilledPortions} / 3</span></div>
                        <div class={styles.scaleProgressBar}>
                            <div class={styles.scaleProgressFill} style="width: {Math.min(app.paperFilledPortions / 3 * 100, 100)}%"></div>
                        </div>
                        <div class={styles.ppRow} style="margin-top: 6px;"><span>Invalidation Level:</span><span class={styles.mono + " " + styles.stopLossText}>${app.paperInvalidationLevel.toFixed(2)}</span></div>
                    </div>
                    {#if app.paperScaleInPortions.length > 0}
                        <div class={styles.ppPortionsTable}>
                            <span class={styles.subTitle}>Scale-In Entries</span>
                            {#each app.paperScaleInPortions as portion}
                                <div class={styles.ppRow + " " + styles.portionRow}>
                                    <span>Portion {portion.portion_number}:</span>
                                    <span>${portion.entry_price.toFixed(2)} ({portion.size.toFixed(4)} units)</span>
                                </div>
                            {/each}
                        </div>
                    {/if}
                    {#if app.paperTakeProfitTargets.length > 0}
                        <div class={styles.ppTargetsTable}>
                            <span class={styles.subTitle}>Take-Profit Targets</span>
                            {#each app.paperTakeProfitTargets as tgt}
                                <div class={styles.ppRow + " " + styles.targetRow}>
                                    <span>${tgt.target_price.toFixed(2)} ({(tgt.size_fraction * 100).toFixed(0)}%):</span>
                                    <span class={styles.targetStatus} class:target-hit={tgt.is_hit}>
                                        {tgt.is_hit ? '✓ FILLED' : '○ PENDING'}
                                    </span>
                                </div>
                            {/each}
                        </div>
                    {/if}
                    <button class={styles.paperCloseBtn} onclick={() => app.closePaperPosition()}
                            disabled={app.paperLoading}>
                        Close Position (Market)
                    </button>
                </div>
            {:else}
                <div class={styles.paperEmptyState}>
                    <p>No active paper position.</p>
                    <div class={styles.paperActionBtns}>
                        <button class={styles.paperOpenBtn + " " + styles.directionLong} onclick={() => app.openPaperPosition('LONG')}
                                disabled={app.paperLoading}>
                            Open Long
                        </button>
                        <button class={styles.paperOpenBtn + " " + styles.directionShort} onclick={() => app.openPaperPosition('SHORT')}
                                disabled={app.paperLoading}>
                            Open Short
                        </button>
                    </div>
                </div>
            {/if}
        </div>

        <div class={styles.paperLedgerCol}>
            <h3 class={styles.cardTitle} style="margin-top: 0;">Account Ledger</h3>
            <div class={styles.paperLedgerCard}>
                <div class={styles.ledgerRow}><span>Total Balance:</span><span class={styles.mono}>${app.paperTotalAccountValue.toFixed(2)}</span></div>
                <div class={styles.ledgerRow}><span>Available Cash:</span><span class={styles.mono}>${app.paperCashBalance.toFixed(2)}</span></div>
                <div class={styles.ledgerRow}><span>Margin Used:</span><span class={styles.mono}>
                    ${app.paperMarginUsed.toFixed(2)} ({app.paperAllocationPct}%)
                </span></div>
                <div class={styles.ledgerDivider}></div>
                <div class={styles.ledgerRow}><span>Trade Capacity:</span></div>
                <div class={styles.capacityBarContainer}>
                    <div class={styles.capacityBarTrack}>
                        <div class={styles.capacityBarFill} style="width: {app.paperMaxTrades > 0 ? (app.paperActiveTrades / app.paperMaxTrades * 100) : 0}%"></div>
                    </div>
                    <span class={styles.capacityText}>{app.paperActiveTrades} / {app.paperMaxTrades} Active</span>
                </div>
                <div class={styles.ledgerRow} style="margin-top: 8px;">
                    <span>Available Trades:</span><span class={styles.mono}>{app.paperAvailableTrades}</span>
                </div>
            </div>

            <!-- TP / SL Configuration -->
            <div class={styles.settingGroupBox} style="margin-top: 12px;">
                <span class={styles.selectorsLabel}>Position Levels</span>
                <div class={styles.inputRow} style="margin-top: 8px;">
                    <label for="tpLevels" style="width: 80px;">TP Levels:</label>
                    <select id="tpLevels" bind:value={draftTpLevels} class={styles.tfUnitSelect}>
                        <option value={1}>1 Level</option>
                        <option value={2}>2 Levels</option>
                        <option value={3}>3 Levels</option>
                    </select>
                </div>
                <div class={styles.inputRow} style="margin-top: 4px;">
                    <label for="slLevels" style="width: 80px;">SL Levels:</label>
                    <select id="slLevels" bind:value={draftSlLevels} class={styles.tfUnitSelect}>
                        <option value={1}>1 Level</option>
                        <option value={2}>2 Levels</option>
                        <option value={3}>3 Levels</option>
                    </select>
                </div>
            </div>
        </div>

        <!-- Token Cost Calculator -->
        <div class={styles.settingGroupBox} style="margin-top: 12px;">
            <span class={styles.selectorsLabel}>AI Token Cost Calculator (per 1M tokens)</span>
            <div class={styles.inputRow} style="margin-top: 4px;">
                <label for="costInput">Input Price $/1M:</label>
                <input id="costInput" type="number" bind:value={draftCostInputPrice} min="0" step="0.01" />
            </div>
            <div class={styles.inputRow} style="margin-top: 8px;">
                <label for="costOutput">Output Price $/1M:</label>
                <input id="costOutput" type="number" bind:value={draftCostOutputPrice} min="0" step="0.01" />
            </div>
            <button class={styles.keySaveBtn} style="margin-top: 8px; width: 100%;"
                    disabled={costSaveStatus === 'saving'} onclick={saveCostConfig}>
                {costSaveStatus === 'saving' ? 'Saving...' : 'Save Cost Config'}
            </button>
            {#if costSaveStatus === 'success'}
                <div class={styles.statusMsg + " " + styles.successMsg}>Pricing saved.</div>
            {/if}
        </div>
    </div>
</div>
