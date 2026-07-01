<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import { calcLiqPrice, calcSizeUnits, calcEstFees } from '../stores/paperTrading.svelte';
    import styles from './PaperTradingPanel.module.css';

    const app = useAppStore();

    let executionDirection = $state<'LONG' | 'SHORT'>('LONG');
    let orderType = $state<'Market' | 'Limit' | 'Stop'>('Market');
    let draftPayAmount = $state('');
    let draftLimitPrice = $state('');
    let draftStopPrice = $state('');
    let draftLeverage = $state(app.paperLeverage);
    let draftTpPrice = $state('');
    let draftSlPrice = $state('');
    let showAdvanced = $state(false);
    let activeConsoleTab = $state<'positions' | 'orders' | 'history'>('positions');

    let lastSavedLeverage = $state(app.paperLeverage);

    const markPrice = $derived(parseFloat(app.priceText) || 0);
    const payAmount = $derived(parseFloat(draftPayAmount) || 0);
    const rawPct = $derived(
        app.paperTotalAccountValue > 0 ? (payAmount / app.paperTotalAccountValue) * 100 : 0
    );
    const snappedPct = $derived(Math.min(100, Math.max(10, Math.round(rawPct / 10) * 10)));
    const sizeUnits = $derived(calcSizeUnits(payAmount, draftLeverage, markPrice));
    const liqPrice = $derived(calcLiqPrice(markPrice, executionDirection, draftLeverage));
    const estFees = $derived(calcEstFees(sizeUnits, markPrice));
    const isLimitOrStop = $derived(orderType !== 'Market');
    const hasPosition = $derived(app.paperDirection !== '');
    const positionCount = $derived(hasPosition ? 1 : 0);
    const canExecute = $derived(
        !app.paperLoading && !isLimitOrStop && payAmount > 0 && snappedPct >= 10
    );
    const isLeverageDirty = $derived(draftLeverage !== lastSavedLeverage);

    $effect(() => {
        draftLeverage = app.paperLeverage;
        lastSavedLeverage = app.paperLeverage;
    });

    $effect(() => {
        app.fetchPaperStatus();
        app.fetchPaperHistory();
    });

    function fmt(n: number, decimals = 2): string {
        if (!isFinite(n)) return '—';
        return n.toFixed(decimals);
    }

    function fmtTs(ts: number): string {
        if (!ts) return '—';
        return new Date(ts).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    }

    function fmtPnl(val: number): string {
        if (!isFinite(val)) return '$0.00';
        return (val >= 0 ? '+' : '') + '$' + val.toFixed(2);
    }

    async function handleLeverageApply() {
        app.paperLeverage = draftLeverage;
        await app.savePaperConfig(app.paperInitialUSD, app.paperAllocationPct, app.paperAutoExecute);
        lastSavedLeverage = draftLeverage;
        await app.fetchPaperStatus();
    }

    async function handleExecute() {
        if (app.paperLoading || isLimitOrStop || snappedPct < 10) return;
        const result = await app.openPositionPct(executionDirection, snappedPct);
        if (!result.success) alert(result.message);
        else {
            await app.fetchPaperStatus();
            draftPayAmount = '';
        }
    }

    async function handleClose() {
        if (app.paperLoading) return;
        const result = await app.closePositionPct(100);
        if (!result.success) alert(result.message);
        else await app.fetchPaperStatus();
    }

    async function handleApplyTpSl() {
        const tp = parseFloat(draftTpPrice);
        const sl = parseFloat(draftSlPrice);
        if (tp > 0) await app.setTpTargets([{ pct: 100, price: tp }]);
        if (sl > 0) await app.setSlLevels([{ pct: 100, price: sl }]);
        await app.fetchPaperStatus();
        draftTpPrice = '';
        draftSlPrice = '';
    }
</script>

<div class={styles.panelLayout}>

    <!-- ═══ SIDEBAR ORDER TICKET ═══ -->
    <aside class={styles.orderTicket}>

        <!-- Direction Tabs -->
        <div class={styles.directionTabs}>
            <button
                class="{styles.directionTab} {executionDirection === 'LONG' ? styles.directionLongActive : ''}"
                onclick={() => executionDirection = 'LONG'}
            >Long</button>
            <button
                class="{styles.directionTab} {executionDirection === 'SHORT' ? styles.directionShortActive : ''}"
                onclick={() => executionDirection = 'SHORT'}
            >Short</button>
        </div>

        <!-- Order Type Pills -->
        <div class={styles.orderTypeRow}>
            <button
                class="{styles.orderTypeBtn} {orderType === 'Market' ? styles.orderTypeActive : ''}"
                onclick={() => orderType = 'Market'}
            >Market</button>
            <button
                class="{styles.orderTypeBtn} {orderType === 'Limit' ? styles.orderTypeActive : ''}"
                onclick={() => orderType = 'Limit'}
            >Limit</button>
            <button
                class="{styles.orderTypeBtn} {orderType === 'Stop' ? styles.orderTypeActive : ''}"
                onclick={() => orderType = 'Stop'}
            >Stop</button>
        </div>

        <!-- Pay Input -->
        <div class={styles.paySection}>
            <span class={styles.payLabel}>Pay</span>
            <div class={styles.payInputRow}>
                <input
                    type="number" class={styles.payField}
                    bind:value={draftPayAmount}
                    min="0" step="1" placeholder="0.00"
                />
                <span class={styles.payCurrency}>USDT</span>
            </div>
        </div>

        <!-- Limit / Stop Price -->
        {#if orderType === 'Limit'}
            <div class={styles.priceInputRow}>
                <span class={styles.priceInputLabel}>Limit Price</span>
                <input
                    type="number" class={styles.priceField}
                    bind:value={draftLimitPrice}
                    step="0.01" placeholder="0.00"
                />
            </div>
        {:else if orderType === 'Stop'}
            <div class={styles.priceInputRow}>
                <span class={styles.priceInputLabel}>Trigger Price</span>
                <input
                    type="number" class={styles.priceField}
                    bind:value={draftStopPrice}
                    step="0.01" placeholder="0.00"
                />
            </div>
        {/if}

        <!-- Size Readout -->
        <div class={styles.sizeDisplay}>
            <span class={styles.sizeLabel}>Size</span>
            <span class={styles.sizeValue}>
                {sizeUnits > 0 ? fmt(sizeUnits, 5) : '—'} units
            </span>
        </div>

        <!-- Leverage -->
        <div class={styles.leverageSection}>
            <div class={styles.leverageHeader}>
                <span class={styles.leverageLabel}>Leverage</span>
                <span class={styles.leverageValue}>{draftLeverage}x</span>
            </div>
            <input type="range" class={styles.leverageSlider}
                bind:value={draftLeverage} min="1" max="100" step="1"
            />
            <div class={styles.leverageMarks}>
                <span>1x</span><span>10x</span><span>25x</span><span>50x</span><span>75x</span><span>100x</span>
            </div>
            {#if isLeverageDirty}
                <button class={styles.executeBtn + ' ' + styles.executeBtnLong}
                    style="font-size:10px;padding:6px;"
                    onclick={handleLeverageApply}
                >Apply Leverage</button>
            {/if}
        </div>

        <!-- TP / SL Advanced -->
        <button class={styles.advancedToggle} onclick={() => showAdvanced = !showAdvanced}>
            {showAdvanced ? '▼' : '▶'} TP / SL
        </button>
        {#if showAdvanced}
            <div class={styles.advancedInputs}>
                <div class={styles.tpSlRow}>
                    <span class="{styles.tpSlLabel} {styles.tpLabel}">TP</span>
                    <input
                        type="number" class={styles.tpSlInput}
                        bind:value={draftTpPrice}
                        step="0.01" placeholder="Take Profit"
                    />
                </div>
                <div class={styles.tpSlRow}>
                    <span class="{styles.tpSlLabel} {styles.slLabel}">SL</span>
                    <input
                        type="number" class={styles.tpSlInput}
                        bind:value={draftSlPrice}
                        step="0.01" placeholder="Stop Loss"
                    />
                </div>
                <button class={styles.executeBtn + ' ' + styles.executeBtnLong}
                    style="font-size:10px;padding:6px;"
                    disabled={!(draftTpPrice || draftSlPrice)}
                    onclick={handleApplyTpSl}
                >Set TP/SL</button>
            </div>
        {/if}

        <!-- Execution Summary -->
        <div class={styles.executionSummary}>
            <div class={styles.summaryRow}>
                <span class={styles.summaryLabel}>Est. Liq Price</span>
                <span class="{styles.summaryValue} {markPrice > 0 && liqPrice > 0 ? styles.summaryDanger : ''}">
                    {markPrice > 0 ? '$' + fmt(liqPrice) : '—'}
                </span>
            </div>
            <div class={styles.summaryRow}>
                <span class={styles.summaryLabel}>Position Size</span>
                <span class={styles.summaryValue}>
                    {sizeUnits > 0 ? fmt(sizeUnits, 5) : '—'}
                </span>
            </div>
            <div class={styles.summaryRow}>
                <span class={styles.summaryLabel}>Margin Required</span>
                <span class={styles.summaryValue}>
                    {payAmount > 0 ? '$' + fmt(payAmount) : '—'}
                </span>
            </div>
            <div class={styles.summaryRow}>
                <span class={styles.summaryLabel}>Est. Fees (0.04%)</span>
                <span class={styles.summaryValue}>
                    {estFees > 0 ? '$' + fmt(estFees) : '—'}
                </span>
            </div>
        </div>

        <!-- Execute Button -->
        {#if isLimitOrStop}
            <button class={styles.executeBtn} disabled>
                {orderType} Not Available in Paper Mode
            </button>
            <div class={styles.unsupportedHint}>
                Paper trading currently supports Market orders only.
            </div>
        {:else if hasPosition && executionDirection !== app.paperDirection}
            <button class={styles.executeBtn} disabled>
                Close {app.paperDirection} Position First
            </button>
            <div class={styles.unsupportedHint}>
                Netting will close your existing {app.paperDirection} position at market price.
            </div>
        {:else}
            <button
                class="{styles.executeBtn} {executionDirection === 'LONG' ? styles.executeBtnLong : styles.executeBtnShort}"
                disabled={!canExecute}
                onclick={handleExecute}
            >
                {app.paperLoading ? 'Processing...' : (executionDirection === 'LONG' ? 'Buy / Long' : 'Sell / Short')}
            </button>
        {/if}
    </aside>

    <!-- ═══ BOTTOM CONSOLE ═══ -->
    <div class={styles.consoleWorkspace}>

        <!-- Tab Bar -->
        <div class={styles.consoleTabBar}>
            <button
                class="{styles.consoleTab} {activeConsoleTab === 'positions' ? styles.consoleTabActive : ''}"
                onclick={() => activeConsoleTab = 'positions'}
            >
                Positions<span class={styles.consoleTabCount}>{positionCount}</span>
            </button>
            <button
                class="{styles.consoleTab} {activeConsoleTab === 'orders' ? styles.consoleTabActive : ''}"
                onclick={() => activeConsoleTab = 'orders'}
            >
                Open Orders<span class={styles.consoleTabCount}>0</span>
            </button>
            <button
                class="{styles.consoleTab} {activeConsoleTab === 'history' ? styles.consoleTabActive : ''}"
                onclick={() => activeConsoleTab = 'history'}
            >
                History<span class={styles.consoleTabCount}>{app.paperHistory.length}</span>
            </button>
        </div>

        <!-- Positions Table -->
        {#if activeConsoleTab === 'positions'}
            {@const pos = app.activePaperPosition ?? ({} as Record<string, unknown>)}
            {@const entryPx = (pos.average_entry_price as number) ?? (pos.entry_price as number) ?? 0}
            {@const posSize = (pos.size as number) ?? 0}
            {@const posLiq = entryPx > 0 ? calcLiqPrice(entryPx, app.paperDirection as 'LONG' | 'SHORT', app.paperLeverage) : 0}
            <div class={styles.tableWrapper}>
                {#if hasPosition}
                    <table class={styles.table}>
                        <thead>
                            <tr>
                                <th>Market</th>
                                <th>Side</th>
                                <th class={styles.tableColRight}>Size</th>
                                <th class={styles.tableColRight}>Entry</th>
                                <th class={styles.tableColRight}>Mark</th>
                                <th class={styles.tableColRight}>Liq Price</th>
                                <th class={styles.tableColRight}>Margin</th>
                                <th class={styles.tableColRight}>P&L</th>
                                <th class={styles.tableColRight}>ROI</th>
                                <th></th>
                            </tr>
                        </thead>
                        <tbody>
                            <tr>
                                <td class={styles.marketCell}>{app.activeTab}</td>
                                <td class="{styles.directionCell} {app.paperDirection === 'LONG' ? styles.directionLong : styles.directionShort}">
                                    {app.paperDirection}
                                </td>
                                <td class={styles.numRight}>{fmt(posSize, 5)}</td>
                                <td class={styles.numRight}>
                                    {entryPx > 0 ? '$' + fmt(entryPx) : '—'}
                                </td>
                                <td class={styles.numRight}>
                                    {markPrice > 0 ? '$' + fmt(markPrice) : '—'}
                                </td>
                                <td class={styles.numRight}>
                                    {posLiq > 0 ? '$' + fmt(posLiq) : '—'}
                                </td>
                                <td class={styles.numRight}>
                                    ${app.paperMarginUsed.toFixed(2)}
                                </td>
                                <td class="{styles.numRight} {app.paperUnrealizedPnl >= 0 ? styles.pnlPositive : styles.pnlNegative}">
                                    {fmtPnl(app.paperUnrealizedPnl)}
                                </td>
                                <td class="{styles.numRight} {app.paperUnrealizedRoi >= 0 ? styles.pnlPositive : styles.pnlNegative}">
                                    {app.paperUnrealizedRoi.toFixed(2)}%
                                </td>
                                <td>
                                    <button class={styles.closeBtn} disabled={app.paperLoading}
                                        onclick={handleClose}
                                    >Close</button>
                                </td>
                            </tr>
                        </tbody>
                    </table>
                {:else}
                    <div class={styles.emptyState}>No active position</div>
                {/if}
            </div>

        <!-- Open Orders Table -->
        {:else if activeConsoleTab === 'orders'}
            <div class={styles.emptyState}>
                Paper trading currently supports Market orders only.
            </div>

        <!-- History Table -->
        {:else}
            <div class={styles.tableWrapper}>
                {#if app.paperHistory.length > 0}
                    <table class={styles.table}>
                        <thead>
                            <tr>
                                <th>Time</th>
                                <th>Market</th>
                                <th>Side</th>
                                <th class={styles.tableColRight}>Entry</th>
                                <th class={styles.tableColRight}>Exit</th>
                                <th class={styles.tableColRight}>P&L</th>
                                <th class={styles.tableColRight}>ROI</th>
                                <th>Trigger</th>
                            </tr>
                        </thead>
                        <tbody>
                            {#each app.paperHistory as trade}
                                {@const t = trade as Record<string, unknown>}
                                <tr>
                                    <td>{fmtTs((t.exit_timestamp as number) ?? 0)}</td>
                                    <td class={styles.marketCell}>{(t.symbol as string) ?? '—'}</td>
                                    <td class="{styles.directionCell} {(t.direction as string) === 'LONG' ? styles.directionLong : styles.directionShort}">
                                        {t.direction as string}
                                    </td>
                                    <td class={styles.numRight}>
                                        {(t.entry_price as number) > 0 ? '$' + fmt(t.entry_price as number) : '—'}
                                    </td>
                                    <td class={styles.numRight}>
                                        {(t.exit_price as number) > 0 ? '$' + fmt(t.exit_price as number) : '—'}
                                    </td>
                                    <td class="{styles.numRight} {(t.realized_pnl as number) >= 0 ? styles.pnlPositive : styles.pnlNegative}">
                                        {fmtPnl((t.realized_pnl as number) ?? 0)}
                                    </td>
                                    <td class="{styles.numRight} {(t.roi_pct as number) >= 0 ? styles.pnlPositive : styles.pnlNegative}">
                                        {fmt((t.roi_pct as number) ?? 0)}%
                                    </td>
                                    <td>{(t.trigger as string) ?? '—'}</td>
                                </tr>
                            {/each}
                        </tbody>
                    </table>
                {:else}
                    <div class={styles.emptyState}>No trade history</div>
                {/if}
            </div>
        {/if}

        <!-- Account Mini Bar -->
        <div class={styles.accountBar}>
            <div class={styles.accountItem}>
                <span class={styles.accountItemLabel}>Balance</span>
                <span class={styles.accountItemValue}>${app.paperTotalAccountValue.toFixed(2)}</span>
            </div>
            <div class={styles.accountItem}>
                <span class={styles.accountItemLabel}>Available</span>
                <span class={styles.accountItemValue}>${app.paperCashBalance.toFixed(2)}</span>
            </div>
            <div class={styles.accountItem}>
                <span class={styles.accountItemLabel}>Margin Used</span>
                <span class={styles.accountItemValue}>${app.paperMarginUsed.toFixed(2)}</span>
            </div>
            <div class={styles.accountItem}>
                <span class={styles.accountItemLabel}>Leverage</span>
                <span class={styles.accountItemValue}>{app.paperLeverage}x</span>
            </div>
        </div>
    </div>

</div>
