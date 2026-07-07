<script lang="ts">
    import type { CommissionProjection } from '../types';
    import styles from './PositionSelector.module.css';

    let { currentPosition = $bindable('None' as 'None' | 'Long' | 'Short'), entryPriceVal = $bindable(''), stopLossVal = $bindable(''), commissionProjection = null as CommissionProjection | null } = $props();
</script>

<div class={styles.positionSelector}>
    <span class={styles.subTitle}>Current Position:</span>
    <label>
        <input type="radio" bind:group={currentPosition} value="None" /> None
    </label>
    <label>
        <input type="radio" bind:group={currentPosition} value="Long" /> Long
    </label>
    <label>
        <input type="radio" bind:group={currentPosition} value="Short" /> Short
    </label>
</div>

{#if currentPosition !== 'None'}
    <div class={styles.entryPriceInput}>
        <label for="entryPrice">Entry Price ($):</label>
        <input id="entryPrice" type="number" step="any"
               bind:value={entryPriceVal} placeholder="0.00" />
    </div>
    <div class="{styles.entryPriceInput} {styles.entryPriceOffset}">
        <label for="stopLoss">Stop Loss ($):</label>
        <input id="stopLoss" type="number" step="any"
               bind:value={stopLossVal} placeholder="0.00" />
        <small class={styles.entryHint}>
            Left blank? Defaults to 1% risk distance.
        </small>
    </div>
{/if}

{#if currentPosition !== 'None' && commissionProjection}
    <div class="{styles.commissionQuickSummary} {commissionProjection.trade_viable ? styles.ccQuickViable : styles.ccQuickNotViable}">
        <span class={styles.ccQuickLabel}>Commission Check:</span>
        <span class={styles.ccQuickFees}>Fees: ${commissionProjection.fee_breakdown.total_fees.toFixed(2)}</span>
        <span class={styles.ccQuickNet}>Net: ${commissionProjection.max_gain_net_after_fees.toFixed(2)}</span>
        <span class={styles.ccQuickBadge}>{commissionProjection.trade_viable ? '✓ Viable' : '✗ Not Viable'}</span>
    </div>
{/if}
