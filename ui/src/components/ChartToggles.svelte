<script lang="ts">
    import { useAppStore } from '../state.svelte';
    // AUDIT-FE-H1: the cluster-refresh status pill was never mounted —
    // a failing /api/liquidity/cluster-status refresh left the heatmap
    // silently empty with no explanation.
    import LiquidityStatusPanel from './LiquidityStatusPanel.svelte';
    import styles from './ChartToggles.module.css';
    const app = useAppStore();
    let { pairKey, symbol }: { pairKey: string; symbol: string } = $props();
    const pair = $derived(app.instancesMap[pairKey]);

    function syncAll(fn: (tf: any) => void) {
        if (!pair) return;
        fn(pair.microTerm);
        fn(pair.fastTerm);
        fn(pair.slowTerm);
        fn(pair.macroTerm);
    }

    function toggleLineMode() {
        if (!pair) return;
        pair.priceLineMode = !pair.priceLineMode;
    }

    function toggleVwap() {
        if (!pair) return;
        const v = !pair.microTerm.showVwap;
        syncAll(tf => { tf.showVwap = v; });
    }

    function toggleBb() {
        if (!pair) return;
        const v = !pair.microTerm.showBb;
        syncAll(tf => { tf.showBb = v; });
    }

    function toggleEma(label: 'Fast' | 'Medium' | 'Slow' | 'Long') {
        if (!pair) return;
        const key = `showEma${label}` as keyof typeof pair;
        (pair as any)[key] = !(pair as any)[key];
    }

    function toggleLiqHeatmap() {
        if (!pair) return;
        const v = !pair.microTerm.showLiqHeatmap;
        syncAll(tf => { tf.showLiqHeatmap = v; });
    }

    function toggleVolumeProfile() {
        if (!pair) return;
        const v = !pair.microTerm.showVolumeProfile;
        syncAll(tf => { tf.showVolumeProfile = v; });
    }

    /// New v6.6 overlay toggles. All sync across the 4 timeframes the same
    /// way LIQ HEATMAP and VOL PROFILE do.
    function toggleAnchoredVwap() {
        if (!pair) return;
        const v = !pair.microTerm.showAnchoredVwap;
        syncAll(tf => { tf.showAnchoredVwap = v; });
    }

    function toggleSupertrend() {
        if (!pair) return;
        const v = !pair.microTerm.showSupertrend;
        syncAll(tf => { tf.showSupertrend = v; });
    }

    function toggleDonchian() {
        if (!pair) return;
        const v = !pair.microTerm.showDonchian;
        syncAll(tf => { tf.showDonchian = v; });
    }

    function toggleIchimoku() {
        if (!pair) return;
        const v = !pair.microTerm.showIchimoku;
        syncAll(tf => { tf.showIchimoku = v; });
    }

    function toggleSupportResistance() {
        if (!pair) return;
        const v = !pair.microTerm.showSupportResistance;
        syncAll(tf => { tf.showSupportResistance = v; });
    }

    function togglePivotPoints() {
        if (!pair) return;
        const v = !pair.microTerm.showPivotPoints;
        syncAll(tf => { tf.showPivotPoints = v; });
    }

    function toggleFibonacci() {
        if (!pair) return;
        const v = !pair.microTerm.showFib;
        syncAll(tf => { tf.showFib = v; });
    }

    function toggleSmc() {
        if (!pair) return;
        const v = !pair.microTerm.showSmcStructure;
        syncAll(tf => {
            tf.showSmcStructure = v;
            tf.showSmcLiquidity = v;
        });
    }

    function toggleFvg() {
        if (!pair) return;
        const v = !pair.microTerm.showFvgZones;
        syncAll(tf => { tf.showFvgZones = v; });
    }

    function toggleOrderBlocks() {
        if (!pair) return;
        const v = !pair.microTerm.showOrderBlocks;
        syncAll(tf => { tf.showOrderBlocks = v; });
    }

    function toggleRibbon() {
        if (!pair) return;
        const v = !pair.microTerm.showDerivativeRibbon;
        syncAll(tf => { tf.showDerivativeRibbon = v; });
    }

    /// v6.7: fills in the previously-empty pills for the registers'
    /// non-rendered indicators. Each is per-TF and gated by its own
    /// `show*` flag.
    function toggleKeltner() {
        if (!pair) return;
        const v = !pair.microTerm.showKeltner;
        syncAll(tf => { tf.showKeltner = v; });
    }

    function toggleStddevChan() {
        if (!pair) return;
        const v = !pair.microTerm.showStddevChan;
        syncAll(tf => { tf.showStddevChan = v; });
    }

    function togglePsar() {
        if (!pair) return;
        const v = !pair.microTerm.showPsar;
        syncAll(tf => { tf.showPsar = v; });
    }
</script>

{#if pair}
<div class={styles.chartToggles}>
    <div class={styles.togglesGroup}>
        <span class={styles.togglesLabel}>PRICE</span>
        <button
            class="{styles.togglePill} {!pair.priceLineMode ? styles.active : ''}"
            onclick={toggleLineMode}
        >CANDLES</button>
        <button
            class="{styles.togglePill} {pair.priceLineMode ? styles.active : ''}"
            onclick={toggleLineMode}
        >LINE</button>
    </div>
    <div class={styles.togglesSeparator}></div>
    <div class={styles.togglesGroup}>
        <span class={styles.togglesLabel}>EMA</span>
        <button class="{styles.togglePill} {styles.emaFast} {pair.showEmaFast ? styles.active : ''}"
            onclick={() => toggleEma('Fast')}>INSTANT</button>
        <button class="{styles.togglePill} {styles.emaMedium} {pair.showEmaMedium ? styles.active : ''}"
            onclick={() => toggleEma('Medium')}>FAST</button>
        <button class="{styles.togglePill} {styles.emaSlow} {pair.showEmaSlow ? styles.active : ''}"
            onclick={() => toggleEma('Slow')}>MEDIUM</button>
        <button class="{styles.togglePill} {styles.emaLong} {pair.showEmaLong ? styles.active : ''}"
            onclick={() => toggleEma('Long')}>SLOW</button>
    </div>
    <div class={styles.togglesSeparator}></div>
    <div class={styles.togglesGroup}>
        <button class="{styles.togglePill} {styles.vwapPill} {pair.microTerm.showVwap ? styles.active : ''}"
            onclick={toggleVwap}>VWAP</button>
        <button class="{styles.togglePill} {styles.bbPill} {pair.microTerm.showBb ? styles.active : ''}"
            onclick={toggleBb}>BOLLINGER</button>
        <button class="{styles.togglePill} {styles.avwapPill} {pair.microTerm.showAnchoredVwap ? styles.active : ''}"
            onclick={toggleAnchoredVwap}>ANC VWAP</button>
        <button class="{styles.togglePill} {styles.supertrendPill} {pair.microTerm.showSupertrend ? styles.active : ''}"
            onclick={toggleSupertrend}>SUPERTREND</button>
        <button class="{styles.togglePill} {styles.donchianPill} {pair.microTerm.showDonchian ? styles.active : ''}"
            onclick={toggleDonchian}>DONCHIAN</button>
    </div>
    <div class={styles.togglesSeparator}></div>
    <div class={styles.togglesGroup}>
        <span class={styles.togglesLabel}>LEVELS</span>
        <button class="{styles.togglePill} {styles.srPill} {pair.microTerm.showSupportResistance ? styles.active : ''}"
            onclick={toggleSupportResistance}>S/R</button>
        <button class="{styles.togglePill} {styles.pivotPill} {pair.microTerm.showPivotPoints ? styles.active : ''}"
            onclick={togglePivotPoints}>PIVOT</button>
        <button class="{styles.togglePill} {styles.fibPill} {pair.microTerm.showFib ? styles.active : ''}"
            onclick={toggleFibonacci}>FIB</button>
        <button class="{styles.togglePill} {styles.ichimokuPill} {pair.microTerm.showIchimoku ? styles.active : ''}"
            onclick={toggleIchimoku}>ICHIMOKU</button>
    </div>
    <div class={styles.togglesSeparator}></div>
    <div class={styles.togglesGroup}>
        <span class={styles.togglesLabel}>SMC</span>
        <button class="{styles.togglePill} {styles.smcStructurePill} {pair.microTerm.showSmcStructure ? styles.active : ''}"
            onclick={toggleSmc}>BOS/CHoCH</button>
        <button class="{styles.togglePill} {styles.fvgPill} {pair.microTerm.showFvgZones ? styles.active : ''}"
            onclick={toggleFvg}>FVG</button>
        <button class="{styles.togglePill} {styles.obPill} {pair.microTerm.showOrderBlocks ? styles.active : ''}"
            onclick={toggleOrderBlocks}>ORDER BLOCKS</button>
    </div>
    <div class={styles.togglesSeparator}></div>
    <div class={styles.togglesGroup}>
        <button class="{styles.togglePill} {styles.keltnerPill} {pair.microTerm.showKeltner ? styles.active : ''}"
            onclick={toggleKeltner}>KELTNER</button>
        <button class="{styles.togglePill} {styles.stddevChanPill} {pair.microTerm.showStddevChan ? styles.active : ''}"
            onclick={toggleStddevChan}>STDDEV CH.</button>
        <button class="{styles.togglePill} {styles.psarPill} {pair.microTerm.showPsar ? styles.active : ''}"
            onclick={togglePsar}>PSAR</button>
    </div>
    <div class={styles.togglesSeparator}></div>
    <div class={styles.togglesGroup}>
        <button class="{styles.togglePill} {styles.liqHeatmapPill} {pair.microTerm.showLiqHeatmap ? styles.active : ''}"
            onclick={toggleLiqHeatmap}>LIQ LEVELS</button>
        <LiquidityStatusPanel symbol={symbol} />
        <button class="{styles.togglePill} {styles.volumeProfilePill} {pair.microTerm.showVolumeProfile ? styles.active : ''}"
            onclick={toggleVolumeProfile}>VOL PROFILE</button>
        <button class="{styles.togglePill} {styles.derivativeRibbonPill} {pair.microTerm.showDerivativeRibbon ? styles.active : ''}"
            onclick={toggleRibbon}>DERIVATIVES</button>
    </div>
</div>
{/if}
