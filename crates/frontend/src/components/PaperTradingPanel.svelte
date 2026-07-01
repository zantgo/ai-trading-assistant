<script lang="ts">
    import { onDestroy } from 'svelte';
    import { useAppStore } from '../state.svelte';
    import { calcLiqPrice, calcSizeUnits, calcEstFees } from '../stores/paperTrading.svelte';
    import BottomConsole from './BottomConsole.svelte';
    import styles from './PaperTradingPanel.module.css';

    const app = useAppStore();

    let executionDirection = $state<'LONG' | 'SHORT'>('LONG');
    let orderType = $state<'Market' | 'Limit' | 'Stop'>('Market');
    let draftLimitPrice = $state('');
    let draftTriggerPrice = $state('');
    let draftLeverage = $state(app.paperLeverage);
    let lastSavedLeverage = $state(app.paperLeverage);
    let activeConsoleTab = $state<'positions' | 'orders' | 'history'>('positions');
    let expandedPositionId = $state<number | null>(null);
    let showCloseDropdown = $state(false);

    const markPrice = $derived(parseFloat(app.priceText) || 0);
    const payAmount = $derived(app.paperTotalAccountValue * 0.25);
    const sizeUnits = $derived(calcSizeUnits(payAmount, draftLeverage, markPrice));
    const liqPrice = $derived(calcLiqPrice(markPrice, executionDirection, draftLeverage));
    const estFees = $derived(calcEstFees(sizeUnits, markPrice));
    const isLimitOrStop = $derived(orderType !== 'Market');
    const hasPosition = $derived(app.paperDirection !== '');
    const canExecute = $derived(!app.paperLoading && payAmount > 0 && markPrice > 0);
    const isLeverageDirty = $derived(draftLeverage !== lastSavedLeverage);

    const showNettingWarning = $derived(
        hasPosition && isLimitOrStop &&
        ((executionDirection === 'LONG' && app.paperDirection === 'SHORT') ||
         (executionDirection === 'SHORT' && app.paperDirection === 'LONG'))
    );

    $effect(() => {
        draftLeverage = app.paperLeverage;
        lastSavedLeverage = app.paperLeverage;
    });

    $effect(() => {
        app.fetchPaperStatus();
        app.fetchPaperHistory();
        app.fetchOpenOrders();
    });

    const pollInterval = setInterval(() => {
        app.fetchPaperStatus();
        app.fetchOpenOrders();
    }, 5000);

    onDestroy(() => clearInterval(pollInterval));

    function fmt(n: number, decimals = 2): string {
        if (!isFinite(n)) return '—';
        return n.toFixed(decimals);
    }

    async function handleLeverageApply() {
        app.paperLeverage = draftLeverage;
        await app.savePaperConfig(app.paperInitialUSD, app.paperAllocationPct, app.paperAutoExecute);
        lastSavedLeverage = draftLeverage;
        await app.fetchPaperStatus();
    }

    async function handleExecute() {
        if (app.paperLoading) return;
        if (isLimitOrStop) {
            const dir = executionDirection === 'LONG' ? 'BUY' : 'SELL';
            const result = await app.placeOrder({
                order_type: orderType as 'LIMIT' | 'STOP',
                direction: dir,
                price: orderType === 'Limit' ? parseFloat(draftLimitPrice) || undefined : undefined,
                trigger_price: orderType === 'Stop' ? parseFloat(draftTriggerPrice) || undefined : undefined,
            });
            if (!result.success) alert(result.message);
            else { draftLimitPrice = ''; draftTriggerPrice = ''; }
        } else {
            const result = await app.openPositionPct(executionDirection, 25);
            if (!result.success) alert(result.message);
            else await app.fetchPaperStatus();
        }
    }
</script>

<div class={styles.panelLayout}>

    <aside class={styles.orderTicket}>
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

        <div class={styles.orderTypeRow}>
            <button class="{styles.orderTypeBtn} {orderType === 'Market' ? styles.orderTypeActive : ''}"
                onclick={() => orderType = 'Market'}>Market</button>
            <button class="{styles.orderTypeBtn} {orderType === 'Limit' ? styles.orderTypeActive : ''}"
                onclick={() => orderType = 'Limit'}>Limit</button>
            <button class="{styles.orderTypeBtn} {orderType === 'Stop' ? styles.orderTypeActive : ''}"
                onclick={() => orderType = 'Stop'}>Stop</button>
        </div>

        <div class={styles.paySection}>
            <span class={styles.payLabel}>Pay Margin: ${payAmount.toFixed(2)} (25% Fixed) [USDT]</span>
        </div>

        {#if orderType === 'Limit'}
            <div class={styles.priceInputRow}>
                <span class={styles.priceInputLabel}>Limit Price</span>
                <input type="number" class={styles.priceField}
                    bind:value={draftLimitPrice} step="0.01" placeholder="0.00" />
            </div>
        {:else if orderType === 'Stop'}
            <div class={styles.priceInputRow}>
                <span class={styles.priceInputLabel}>Trigger Price</span>
                <input type="number" class={styles.priceField}
                    bind:value={draftTriggerPrice} step="0.01" placeholder="0.00" />
            </div>
        {/if}

        <div class={styles.sizeDisplay}>
            <span class={styles.sizeLabel}>Size</span>
            <span class={styles.sizeValue}>{sizeUnits > 0 ? fmt(sizeUnits, 5) : '—'} units</span>
        </div>

        <div class={styles.leverageSection}>
            <div class={styles.leverageHeader}>
                <span class={styles.leverageLabel}>Leverage</span>
                <span class={styles.leverageValue}>{draftLeverage}x</span>
            </div>
            <input type="range" class={styles.leverageSlider}
                bind:value={draftLeverage} min="1" max="100" step="1" />
            <div class={styles.leverageMarks}>
                <span>1x</span><span>10x</span><span>25x</span><span>50x</span><span>75x</span><span>100x</span>
            </div>
            {#if isLeverageDirty}
                <button class={styles.executeBtn + ' ' + styles.executeBtnLong}
                    style="font-size:10px;padding:6px;"
                    onclick={handleLeverageApply}>Apply Leverage</button>
            {/if}
        </div>

        {#if showNettingWarning}
            <div class={styles.nettingWarning}>
                Note: This order will reduce or close your active {app.paperDirection} position upon execution (Netting).
            </div>
        {/if}

        <div class={styles.executionSummary}>
            <div class={styles.summaryRow}>
                <span class={styles.summaryLabel}>Est. Liq Price</span>
                <span class="{styles.summaryValue} {markPrice > 0 && liqPrice > 0 ? styles.summaryDanger : ''}">
                    {markPrice > 0 ? '$' + fmt(liqPrice) : '—'}
                </span>
            </div>
            <div class={styles.summaryRow}>
                <span class={styles.summaryLabel}>Position Size</span>
                <span class={styles.summaryValue}>{sizeUnits > 0 ? fmt(sizeUnits, 5) : '—'}</span>
            </div>
            <div class={styles.summaryRow}>
                <span class={styles.summaryLabel}>Margin Required</span>
                <span class={styles.summaryValue}>{payAmount > 0 ? '$' + fmt(payAmount) : '—'}</span>
            </div>
            <div class={styles.summaryRow}>
                <span class={styles.summaryLabel}>Est. Fees (0.04%)</span>
                <span class={styles.summaryValue}>{estFees > 0 ? '$' + fmt(estFees) : '—'}</span>
            </div>
        </div>

        {#if !canExecute}
            <button class={styles.executeBtn} disabled>{app.paperLoading ? 'Processing...' : 'Awaiting Market Data'}</button>
        {:else}
            <button
                class="{styles.executeBtn} {executionDirection === 'LONG' ? styles.executeBtnLong : styles.executeBtnShort}"
                onclick={handleExecute}
            >
                {app.paperLoading ? 'Processing...' : (
                    isLimitOrStop
                        ? `${orderType} ${executionDirection === 'LONG' ? 'Buy' : 'Sell'}`
                        : (executionDirection === 'LONG' ? 'Buy / Long' : 'Sell / Short')
                )}
            </button>
        {/if}
    </aside>

    <BottomConsole
        bind:activeConsoleTab
        bind:expandedPositionId
        bind:showCloseDropdown
    />

</div>
