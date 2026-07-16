<script lang="ts">
    import styles from './CommissionCalculator.module.css';

    // ── Fee reference calculator (client-side) ──
    let calcLeverage = $state(10);
    let calcCapital = $state(1000);
    let calcFeePct = $state(0.06);

    const calcNotional = $derived(calcCapital * calcLeverage);
    const calcFees = $derived((calcFeePct / 100) * calcNotional * 2);
    const calcMinProfitPct = $derived(calcCapital > 0 ? (calcFees / calcCapital) * 100 : 0);

    function formatUsd(v: number | undefined | null): string {
        if (v == null || isNaN(v)) return '$0.00';
        return '$' + v.toFixed(2);
    }

    function formatPct(v: number | undefined | null): string {
        if (v == null || isNaN(v)) return '0.00%';
        return v.toFixed(2) + '%';
    }
</script>

<div class={styles.ccLayout}>
    <div class={styles.ccCard} style="max-width: 620px; margin: 0 auto;">
        <h3 class={styles.ccCardTitle}>FEE REFERENCE CALCULATOR</h3>
        <p class={styles.ccCardSub}>Calculate round-trip fees and minimum profit needed to break even</p>
        <div class={styles.ccCalcRow}>
            <div class={styles.ccCalcField}>
                <label class={styles.ccCalcLabel} for="cc-calc-leverage">Leverage</label>
                <input id="cc-calc-leverage" type="number" min="1" max="150" step="1" bind:value={calcLeverage} class={styles.ccCalcInput} />
            </div>
            <div class={styles.ccCalcField}>
                <label class={styles.ccCalcLabel} for="cc-calc-capital">Capital ($)</label>
                <input id="cc-calc-capital" type="number" min="1" step="100" bind:value={calcCapital} class={styles.ccCalcInput} />
            </div>
            <div class={styles.ccCalcField}>
                <label class={styles.ccCalcLabel} for="cc-calc-fee">Exchange Fee (%)</label>
                <input id="cc-calc-fee" type="number" min="0" max="10" step="0.01" bind:value={calcFeePct} class={styles.ccCalcInput} />
            </div>
        </div>
        <div class={styles.ccCalcResults}>
            <div class={styles.ccCalcResultItem}>
                <span class={styles.ccCalcResultLabel}>Notional Value</span>
                <span class={styles.ccCalcResultValue}>{formatUsd(calcNotional)}</span>
            </div>
            <div class={styles.ccCalcResultItem}>
                <span class={styles.ccCalcResultLabel}>Round-Trip Fees</span>
                <span class="{styles.ccCalcResultValue} {calcMinProfitPct > 3 ? styles.ccFeeWarn : ''}">{formatUsd(calcFees)}</span>
            </div>
            <div class={styles.ccCalcResultItem}>
                <span class={styles.ccCalcResultLabel}>Min Profit to Cover</span>
                <span class={styles.ccCalcResultValue}>{formatUsd(calcFees)} <span class={styles.ccCalcResultSub}>(open + close)</span></span>
            </div>
            <div class={styles.ccCalcResultItem}>
                <span class={styles.ccCalcResultLabel}>Min Profit %</span>
                <span class="{styles.ccCalcResultValue} {calcMinProfitPct > 3 ? styles.ccFeeWarn : ''}">{formatPct(calcMinProfitPct)}</span>
            </div>
        </div>
    </div>
</div>
