<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import { calcLiqPrice, calcSizeUnits, calcEstFees } from '../stores/paperTrading.svelte';
    import styles from './OrderTicket.module.css';

    const app = useAppStore();

    let orderType = $state<'Market' | 'Limit' | 'Stop'>('Market');
    let draftLimitPrice = $state('');
    let draftTriggerPrice = $state('');
    let draftLeverage = $state(app.paperLeverage);
    let lastSavedLeverage = $state(app.paperLeverage);

    const markPrice = $derived(parseFloat(app.priceText) || 0);
    const payAmount = $derived(app.paperTotalAccountValue * 0.25);
    const sizeUnits = $derived(calcSizeUnits(payAmount, draftLeverage, markPrice));
    const liqPrice = $derived(calcLiqPrice(markPrice, app.paperDirection || 'LONG', draftLeverage));
    const estFees = $derived(calcEstFees(sizeUnits, markPrice));
    const isLimitOrStop = $derived(orderType !== 'Market');
    const isLeverageDirty = $derived(draftLeverage !== lastSavedLeverage);

    const activeLongs = $derived(app.paper.activeLongs);
    const activeShorts = $derived(app.paper.activeShorts);
    const hasLong = $derived(activeLongs > 0);
    const hasShort = $derived(activeShorts > 0);
    const canOpenLong = $derived(!app.paperLoading && markPrice > 0 && activeLongs < 4);
    const canOpenShort = $derived(!app.paperLoading && markPrice > 0 && activeShorts < 4);
    const canCloseLong = $derived(!app.paperLoading && activeLongs > 0);
    const canCloseShort = $derived(!app.paperLoading && activeShorts > 0);

    $effect(() => {
        draftLeverage = app.paperLeverage;
        lastSavedLeverage = app.paperLeverage;
    });

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

    async function handleOpenLong() {
        if (app.paperLoading) return;
        if (isLimitOrStop) {
            const dir = 'BUY';
            const result = await app.placeOrder({
                order_type: orderType as 'LIMIT' | 'STOP',
                direction: dir,
                price: orderType === 'Limit' ? parseFloat(draftLimitPrice) || undefined : undefined,
                trigger_price: orderType === 'Stop' ? parseFloat(draftTriggerPrice) || undefined : undefined,
            });
            if (!result.success) alert(result.message);
            else { draftLimitPrice = ''; draftTriggerPrice = ''; }
        } else {
            const result = await app.openSlot('LONG');
            if (!result.success) alert(result.message);
            else await app.fetchPaperStatus();
        }
    }

    async function handleOpenShort() {
        if (app.paperLoading) return;
        if (isLimitOrStop) {
            const dir = 'SELL';
            const result = await app.placeOrder({
                order_type: orderType as 'LIMIT' | 'STOP',
                direction: dir,
                price: orderType === 'Limit' ? parseFloat(draftLimitPrice) || undefined : undefined,
                trigger_price: orderType === 'Stop' ? parseFloat(draftTriggerPrice) || undefined : undefined,
            });
            if (!result.success) alert(result.message);
            else { draftLimitPrice = ''; draftTriggerPrice = ''; }
        } else {
            const result = await app.openSlot('SHORT');
            if (!result.success) alert(result.message);
            else await app.fetchPaperStatus();
        }
    }

    async function handleCloseLong() {
        if (app.paperLoading) return;
        const result = await app.closeSlot();
        if (!result.success) alert(result.message);
        else await app.fetchPaperStatus();
    }

    async function handleCloseShort() {
        if (app.paperLoading) return;
        const result = await app.closeSlot();
        if (!result.success) alert(result.message);
        else await app.fetchPaperStatus();
    }
</script>

<div class={styles.orderTicketContainer}>
    <!-- Long Controls -->
    <div class={styles.pacingGroup}>
        <button
            class={styles.btnLong}
            onclick={handleOpenLong}
            disabled={!canOpenLong}
        >
            LONG ({activeLongs}/4)
        </button>
        <button
            class={styles.btnClose}
            onclick={handleCloseLong}
            disabled={!canCloseLong}
        >
            CLOSE ({4 - activeLongs}/4)
        </button>
    </div>

    <!-- Short Controls -->
    <div class={styles.pacingGroup}>
        <button
            class={styles.btnShort}
            onclick={handleOpenShort}
            disabled={!canOpenShort}
        >
            SHORT ({activeShorts}/4)
        </button>
        <button
            class={styles.btnClose}
            onclick={handleCloseShort}
            disabled={!canCloseShort}
        >
            CLOSE ({4 - activeShorts}/4)
        </button>
    </div>

    <!-- Order Type Pills -->
    <div class={styles.orderTypeRow}>
        <button class="{styles.orderTypeBtn} {orderType === 'Market' ? styles.orderTypeActive : ''}"
            onclick={() => orderType = 'Market'}>Market</button>
        <button class="{styles.orderTypeBtn} {orderType === 'Limit' ? styles.orderTypeActive : ''}"
            onclick={() => orderType = 'Limit'}>Limit</button>
        <button class="{styles.orderTypeBtn} {orderType === 'Stop' ? styles.orderTypeActive : ''}"
            onclick={() => orderType = 'Stop'}>Stop</button>
    </div>

    <!-- Limit / Stop Price Inputs -->
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

    <!-- Size Display -->
    <div class={styles.sizeDisplay}>
        <span class={styles.sizeLabel}>Size</span>
        <span class={styles.sizeValue}>{sizeUnits > 0 ? fmt(sizeUnits, 5) : '—'} units</span>
    </div>

    <!-- Leverage -->
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
            <button class={styles.applyLeverageBtn}
                onclick={handleLeverageApply}>Apply Leverage</button>
        {/if}
    </div>

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
</div>
