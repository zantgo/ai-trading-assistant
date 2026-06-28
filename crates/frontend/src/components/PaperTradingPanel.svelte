<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import styles from './PaperTradingPanel.module.css';

    const app = useAppStore();

    // Trade panel state
    let draftPct = $state(100);
    let draftOrderType = $state<'Market' | 'Limit' | 'Stop'>('Market');
    let draftLimitPrice = $state('');
    let draftTriggerPrice = $state('');
    let draftLeverage = $state(app.paperLeverage);
    let showTpSl = $state(false);

    // TP/SL draft state
    let tpTargets = $state<{ pct: number; price: string }[]>([]);
    let slTargets = $state<{ pct: number; price: string }[]>([]);

    // Dirty tracking for Apply button
    let lastSavedLeverage = $state(app.paperLeverage);
    let isLeverageDirty = $derived(draftLeverage !== lastSavedLeverage);

    // Derive available percentages for sliders
    let posPct = $derived(app.paperPositionPct);
    let freePct = $derived(app.paperFreeBalancePct);
    let hasPosition = $derived(posPct > 0);
    let direction = $derived(app.paperDirection);

    let maxTpSlots = $derived(Math.floor(posPct / 10));
    let canOpenMore = $derived(freePct >= 10 && posPct < 100);

    // Button state
    let primaryLabel = $derived(
        hasPosition ? `Close ${direction} ${freePct >= 10 ? `| +Open ${direction}` : ''}` :
        `Open`
    );

    // Pct presets for quick selection
    const pctPresets = [10, 20, 30, 40, 50, 60, 70, 80, 90, 100];

    $effect(() => {
        draftLeverage = app.paperLeverage;
        lastSavedLeverage = app.paperLeverage;
    });

    async function handleLeverageApply() {
        app.paperLeverage = draftLeverage;
        await app.savePaperConfig(app.paperInitialUSD, app.paperAllocationPct, app.paperAutoExecute);
        lastSavedLeverage = draftLeverage;
        await app.fetchPaperStatus();
    }

    async function handleOpen(d: 'LONG' | 'SHORT') {
        if (draftPct < 10) return;
        const result = await app.openPositionPct(d, draftPct);
        if (!result.success) alert(result.message);
        else await app.fetchPaperStatus();
    }

    async function handleClose(pct?: number) {
        const cpct = pct ?? 100;
        if (cpct < 10) return;
        const result = await app.closePositionPct(cpct);
        if (!result.success) alert(result.message);
        else await app.fetchPaperStatus();
    }

    function addTpSlot() {
        if (tpTargets.length >= maxTpSlots) return;
        tpTargets = [...tpTargets, { pct: 10, price: '' }];
    }

    function addSlSlot() {
        if (slTargets.length >= maxTpSlots) return;
        slTargets = [...slTargets, { pct: 10, price: '' }];
    }

    function removeTpSlot(i: number) {
        tpTargets = tpTargets.filter((_, idx) => idx !== i);
    }

    function removeSlSlot(i: number) {
        slTargets = slTargets.filter((_, idx) => idx !== i);
    }

    async function saveTpSl() {
        const validTps = tpTargets.filter(t => t.price && parseFloat(t.price) > 0).map(t => ({ pct: t.pct, price: parseFloat(t.price) }));
        const validSls = slTargets.filter(s => s.price && parseFloat(s.price) > 0).map(s => ({ pct: s.pct, price: parseFloat(s.price) }));
        if (validTps.length > 0) await app.setTpTargets(validTps);
        if (validSls.length > 0) await app.setSlLevels(validSls);
        await app.fetchPaperStatus();
    }
</script>

<div class={styles.container}>
    <!-- Position Status Card -->
    <div class={styles.positionCard} class:hasPosition class:emptyPosition={!hasPosition}>
        <div class={styles.positionHeader}>
            <div class={styles.headerLeft}>
                <span class={styles.pairLabel}>{app.activeTab || '—'} / USDT</span>
                {#if hasPosition}
                    <span class={styles.directionBadge} class:directionLong={direction === 'LONG'} class:directionShort={direction === 'SHORT'}>
                        {direction} {posPct}%
                    </span>
                {:else}
                    <span class={styles.directionBadge + ' ' + styles.noPosition}>No Position</span>
                {/if}
            </div>
            <div class={styles.headerRight}>
                <span class={styles.priceLabel}>Mark: </span>
                <span class={styles.priceValue}>${app.paperAvgEntryPrice.toFixed(2) || '—'}</span>
            </div>
        </div>

        {#if hasPosition}
            <div class={styles.balanceBars}>
                <div class={styles.barRow}>
                    <span class={styles.barLabel}>Used</span>
                    <div class={styles.barTrack}>
                        <div class={styles.barFill} class:barLong={direction === 'LONG'} class:barShort={direction === 'SHORT'}
                             style="width: {posPct}%"></div>
                    </div>
                    <span class={styles.barValue}>{posPct}%</span>
                </div>
                <div class={styles.barRow}>
                    <span class={styles.barLabel}>Free</span>
                    <div class={styles.barTrack}>
                        <div class={styles.barFill + ' ' + styles.barFree} style="width: {freePct}%"></div>
                    </div>
                    <span class={styles.barValue}>{freePct}%</span>
                </div>
            </div>

            <!-- Close options when position open -->
            <div class={styles.closeSection}>
                <span class={styles.sectionLabel}>Close Position</span>
                <div class={styles.pctGrid}>
                    {#each pctPresets.filter(p => p <= posPct) as p}
                        <button class={styles.pctBtn} class:pctActive={draftPct === p}
                                onclick={() => { draftPct = p; handleClose(p); }}>
                            {p}%
                        </button>
                    {/each}
                </div>
            </div>

            <!-- TP / SL Toggle -->
            <button class={styles.tpSlToggle} onclick={() => showTpSl = !showTpSl}>
                {showTpSl ? '▼' : '▶'} TP / SL ({maxTpSlots} slots available)
            </button>

            {#if showTpSl}
                <div class={styles.tpSlPanel}>
                    <div class={styles.tpSlSection}>
                        <span class={styles.tpSlTitle + ' ' + styles.tpTitle}>Take Profit</span>
                        {#each tpTargets as t, i}
                            <div class={styles.tpSlRow}>
                                <input type="number" class={styles.pctInput} bind:value={t.pct} min="10" max="100" step="10" />
                                <span class={styles.pctUnit}>% @ $</span>
                                <input type="number" class={styles.priceInput} bind:value={t.price} step="0.01" placeholder="Price" />
                                <button class={styles.removeBtn} onclick={() => removeTpSlot(i)}>×</button>
                            </div>
                        {/each}
                        {#if tpTargets.length < maxTpSlots}
                            <button class={styles.addBtn + ' ' + styles.addTp} onclick={addTpSlot}>+ Add TP</button>
                        {/if}
                    </div>
                    <div class={styles.tpSlSection}>
                        <span class={styles.tpSlTitle + ' ' + styles.slTitle}>Stop Loss</span>
                        {#each slTargets as s, i}
                            <div class={styles.tpSlRow}>
                                <input type="number" class={styles.pctInput} bind:value={s.pct} min="10" max="100" step="10" />
                                <span class={styles.pctUnit}>% @ $</span>
                                <input type="number" class={styles.priceInput} bind:value={s.price} step="0.01" placeholder="Price" />
                                <button class={styles.removeBtn} onclick={() => removeSlSlot(i)}>×</button>
                            </div>
                        {/each}
                        {#if slTargets.length < maxTpSlots}
                            <button class={styles.addBtn + ' ' + styles.addSl} onclick={addSlSlot}>+ Add SL</button>
                        {/if}
                    </div>
                    <button class={styles.applyTpSlBtn} onclick={saveTpSl}>Apply TP/SL</button>
                </div>
            {/if}

            <!-- Unrealized P&L -->
            <div class={styles.pnlSection}>
                <div class={styles.pnlRow}>
                    <span>Unrealized P&L</span>
                    <span class={styles.pnlValue} class:pnlPos={app.paperUnrealizedPnl >= 0} class:pnlNeg={app.paperUnrealizedPnl < 0}>
                        {app.paperUnrealizedPnl >= 0 ? '+' : ''}${app.paperUnrealizedPnl.toFixed(2)}
                    </span>
                </div>
                <div class={styles.pnlRow}>
                    <span>ROI</span>
                    <span class={styles.pnlValue} class:pnlPos={app.paperUnrealizedRoi >= 0} class:pnlNeg={app.paperUnrealizedRoi < 0}>
                        {app.paperUnrealizedRoi.toFixed(2)}%
                    </span>
                </div>
            </div>
        {:else}
            <!-- No position — balance bars -->
            <div class={styles.balanceBars}>
                <div class={styles.barRow}>
                    <span class={styles.barLabel}>Free</span>
                    <div class={styles.barTrack}>
                        <div class={styles.barFill + ' ' + styles.barFree} style="width: 100%"></div>
                    </div>
                    <span class={styles.barValue}>100%</span>
                </div>
            </div>
        {/if}
    </div>

    <!-- Trade Controls -->
    <div class={styles.tradeControls}>
        <!-- Leverage -->
        <div class={styles.controlGroup}>
            <div class={styles.controlHeader}>
                <span class={styles.controlLabel}>Leverage</span>
                <span class={styles.controlValue}>{draftLeverage}x</span>
            </div>
            <div class={styles.sliderRow}>
                <input type="range" class={styles.slider} bind:value={draftLeverage} min="1" max="100" step="1" />
            </div>
            <div class={styles.sliderLabels}>
                <span>1x</span><span>25x</span><span>50x</span><span>75x</span><span>100x</span>
            </div>
            <button class={isLeverageDirty ? styles.applyBtn : styles.savedBtn}
                    disabled={!isLeverageDirty} onclick={handleLeverageApply}>
                {isLeverageDirty ? 'Apply' : 'Saved ✓'}
            </button>
        </div>

        <!-- Order Type -->
        <div class={styles.controlGroup}>
            <span class={styles.controlLabel}>Order Type</span>
            <div class={styles.orderTypeRow}>
                <button class={styles.orderTypeBtn} class:orderActive={draftOrderType === 'Market'}
                        onclick={() => draftOrderType = 'Market'}>Market</button>
                <button class={styles.orderTypeBtn} class:orderActive={draftOrderType === 'Limit'}
                        onclick={() => draftOrderType = 'Limit'}>Limit</button>
                <button class={styles.orderTypeBtn} class:orderActive={draftOrderType === 'Stop'}
                        onclick={() => draftOrderType = 'Stop'}>Stop</button>
            </div>
            {#if draftOrderType === 'Limit'}
                <input type="number" class={styles.priceField} bind:value={draftLimitPrice} step="0.01" placeholder="Limit Price" />
            {:else if draftOrderType === 'Stop'}
                <input type="number" class={styles.priceField} bind:value={draftTriggerPrice} step="0.01" placeholder="Trigger Price" />
            {/if}
        </div>

        <!-- Position Size -->
        <div class={styles.controlGroup}>
            <div class={styles.controlHeader}>
                <span class={styles.controlLabel}>Position Size</span>
                <span class={styles.controlValue}>{draftPct}%</span>
            </div>
            <div class={styles.sliderRow}>
                <input type="range" class={styles.slider} bind:value={draftPct} min="10" max="100" step="10" />
            </div>
            <div class={styles.pctGrid}>
                {#each pctPresets as p}
                    <button class={styles.pctBtn} class:pctActive={draftPct === p} onclick={() => draftPct = p}>{p}%</button>
                {/each}
            </div>
        </div>

        <!-- Action Buttons -->
        {#if hasPosition}
            <div class={styles.actionSection}>
                <button class={styles.actionBtn + ' ' + styles.closeBtn} onclick={() => handleClose(draftPct)}
                        disabled={app.paperLoading || draftPct > posPct}>
                    {app.paperLoading ? 'Processing...' : `Close ${draftPct}% of ${direction}`}
                </button>
                {#if canOpenMore && direction}
                    <button class={styles.actionBtn + ' ' + (direction === 'LONG' ? styles.longBtn : styles.shortBtn)}
                            onclick={() => handleOpen(direction as 'LONG' | 'SHORT')}
                            disabled={app.paperLoading}>
                        + Add {draftPct}% {direction}
                    </button>
                {/if}
                {#if freePct >= 10}
                    <button class={styles.actionBtn + ' ' + (direction === 'LONG' ? styles.shortBtn : styles.longBtn)}
                            onclick={() => handleOpen(direction === 'LONG' ? 'SHORT' : 'LONG')}
                            disabled={app.paperLoading}>
                        Flip to {(direction === 'LONG' ? 'SHORT' : 'LONG')} {draftPct}%
                    </button>
                {/if}
            </div>
        {:else}
            <div class={styles.actionSection}>
                <button class={styles.actionBtn + ' ' + styles.longBtn} onclick={() => handleOpen('LONG')}
                        disabled={app.paperLoading}>
                    {app.paperLoading ? 'Processing...' : `Open Long ${draftPct}%`}
                </button>
                <button class={styles.actionBtn + ' ' + styles.shortBtn} onclick={() => handleOpen('SHORT')}
                        disabled={app.paperLoading}>
                    {app.paperLoading ? 'Processing...' : `Open Short ${draftPct}%`}
                </button>
            </div>
        {/if}

        <!-- Margin Info -->
        <div class={styles.marginInfo}>
            <div class={styles.marginRow}><span>Margin Mode</span><span>Isolated</span></div>
            <div class={styles.marginRow}><span>Position Mode</span><span>One-Way</span></div>
            <div class={styles.marginRow}><span>Leverage</span><span>{draftLeverage}x</span></div>
        </div>
    </div>

    <!-- Account Summary -->
    <div class={styles.accountCard}>
        <h3 class={styles.accountTitle}>Account</h3>
        <div class={styles.accountGrid}>
            <div class={styles.accountItem}>
                <span class={styles.accountLabel}>Balance</span>
                <span class={styles.accountValue}>${app.paperTotalAccountValue.toFixed(2)}</span>
            </div>
            <div class={styles.accountItem}>
                <span class={styles.accountLabel}>Available</span>
                <span class={styles.accountValue}>${app.paperCashBalance.toFixed(2)}</span>
            </div>
            <div class={styles.accountItem}>
                <span class={styles.accountLabel}>Margin Used</span>
                <span class={styles.accountValue}>${app.paperMarginUsed.toFixed(2)}</span>
            </div>
            <div class={styles.accountItem}>
                <span class={styles.accountLabel}>Free Balance</span>
                <span class={styles.accountValue}>{freePct}%</span>
            </div>
        </div>
    </div>
</div>
