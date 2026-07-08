<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import styles from './ChartToggles.module.css';

    const app = useAppStore();
    let { pairKey }: { pairKey: string } = $props();
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

    function toggleAvwap() {
        if (!pair) return;
        const v = !pair.microTerm.showAvwap;
        syncAll(tf => { tf.showAvwap = v; });
    }

    function toggleBb() {
        if (!pair) return;
        const v = !pair.microTerm.showBb;
        syncAll(tf => { tf.showBb = v; });
    }

    function toggleSupertrend() {
        if (!pair) return;
        const v = !pair.microTerm.showSupertrend;
        syncAll(tf => { tf.showSupertrend = v; });
    }

    function toggleKeltner() {
        if (!pair) return;
        const v = !pair.microTerm.showKeltner;
        syncAll(tf => { tf.showKeltner = v; });
    }

    function toggleDonchian() {
        if (!pair) return;
        const v = !pair.microTerm.showDonchian;
        syncAll(tf => { tf.showDonchian = v; });
    }

    function togglePivots() {
        if (!pair) return;
        const v = !pair.microTerm.showPivots;
        syncAll(tf => { tf.showPivots = v; });
    }

    function togglePatterns() {
        if (!pair) return;
        const v = !pair.microTerm.showCandlestick;
        syncAll(tf => { tf.showCandlestick = v; });
    }

    function toggleIchimoku() {
        if (!pair) return;
        const v = !pair.microTerm.showIchimoku;
        syncAll(tf => { tf.showIchimoku = v; });
    }

    function toggleChikou() {
        if (!pair) return;
        const v = !pair.microTerm.showChikou;
        syncAll(tf => { tf.showChikou = v; });
    }

    function toggleEma(label: 'Fast' | 'Medium' | 'Slow' | 'Long') {
        if (!pair) return;
        const key = `showEma${label}` as keyof typeof pair;
        (pair as any)[key] = !(pair as any)[key];
    }

    function toggleCci() {
        if (!pair) return;
        const v = !pair.microTerm.showCci;
        syncAll(tf => { tf.showCci = v; });
    }

    function togglePsar() {
        if (!pair) return;
        const v = !pair.microTerm.showPsar;
        syncAll(tf => { tf.showPsar = v; });
    }

    function toggleWilliamsR() {
        if (!pair) return;
        const v = !pair.microTerm.showWilliamsR;
        syncAll(tf => { tf.showWilliamsR = v; });
    }

    function toggleHullMa() {
        if (!pair) return;
        const v = !pair.microTerm.showHullMa;
        syncAll(tf => { tf.showHullMa = v; });
    }

    function toggleAo() {
        if (!pair) return;
        const v = !pair.microTerm.showAo;
        syncAll(tf => { tf.showAo = v; });
    }

    function toggleForceIdx() {
        if (!pair) return;
        const v = !pair.microTerm.showForceIdx;
        syncAll(tf => { tf.showForceIdx = v; });
    }

    function toggleStdDevChnl() {
        if (!pair) return;
        const v = !pair.microTerm.showStdDevChnl;
        syncAll(tf => { tf.showStdDevChnl = v; });
    }

    function toggleVolumeProfile() {
        if (!pair) return;
        const v = !pair.microTerm.showVolumeProfile;
        syncAll(tf => { tf.showVolumeProfile = v; });
    }

    function toggleSmcStructure() {
        if (!pair) return;
        const v = !pair.microTerm.showSmcStructure;
        syncAll(tf => { tf.showSmcStructure = v; });
    }
    function toggleSmcLiquidity() {
        if (!pair) return;
        const v = !pair.microTerm.showSmcLiquidity;
        syncAll(tf => { tf.showSmcLiquidity = v; });
    }
    function toggleSmcFvg() {
        if (!pair) return;
        const v = !pair.microTerm.showSmcFvg;
        syncAll(tf => { tf.showSmcFvg = v; });
    }
    function toggleSmcOrderBlocks() {
        if (!pair) return;
        const v = !pair.microTerm.showSmcOrderBlocks;
        syncAll(tf => { tf.showSmcOrderBlocks = v; });
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
            onclick={() => toggleEma('Fast')}>FAST</button>
        <button class="{styles.togglePill} {styles.emaMedium} {pair.showEmaMedium ? styles.active : ''}"
            onclick={() => toggleEma('Medium')}>MED</button>
        <button class="{styles.togglePill} {styles.emaSlow} {pair.showEmaSlow ? styles.active : ''}"
            onclick={() => toggleEma('Slow')}>SLOW</button>
        <button class="{styles.togglePill} {styles.emaLong} {pair.showEmaLong ? styles.active : ''}"
            onclick={() => toggleEma('Long')}>LONG</button>
    </div>
    <div class={styles.togglesSeparator}></div>
    <div class={styles.togglesGroup}>
        <button class="{styles.togglePill} {styles.vwapPill} {pair.microTerm.showVwap ? styles.active : ''}"
            onclick={toggleVwap}>VWAP</button>
        <button class="{styles.togglePill} {styles.vwapPill} {pair.microTerm.showAvwap ? styles.active : ''}"
            onclick={toggleAvwap}>A-VWAP</button>
        <button class="{styles.togglePill} {styles.bbPill} {pair.microTerm.showBb ? styles.active : ''}"
            onclick={toggleBb}>BOLLINGER</button>
        <button class="{styles.togglePill} {styles.supertrendPill} {pair.microTerm.showSupertrend ? styles.active : ''}"
            onclick={toggleSupertrend}>SUPERTREND</button>
        <button class="{styles.togglePill} {styles.keltnerPill} {pair.microTerm.showKeltner ? styles.active : ''}"
            onclick={toggleKeltner}>KELTNER</button>
        <button class="{styles.togglePill} {styles.donchianPill} {pair.microTerm.showDonchian ? styles.active : ''}"
            onclick={toggleDonchian}>DONCHIAN</button>
        <button class="{styles.togglePill} {styles.pivotsPill} {pair.microTerm.showPivots ? styles.active : ''}"
            onclick={togglePivots}>PIVOTS</button>
        <button class="{styles.togglePill} {styles.patternsPill} {pair.microTerm.showCandlestick ? styles.active : ''}"
            onclick={togglePatterns}>PATTERNS</button>
        <button class="{styles.togglePill} {styles.ichimokuPill} {pair.microTerm.showIchimoku ? styles.active : ''}"
            onclick={toggleIchimoku}>ICHIMOKU</button>
        <button class="{styles.togglePill} {styles.chikouPill} {pair.microTerm.showChikou ? styles.active : ''}"
            onclick={toggleChikou}>CHIKOU</button>
        <button class="{styles.togglePill} {styles.cciPill} {pair.microTerm.showCci ? styles.active : ''}"
            onclick={toggleCci}>CCI</button>
        <button class="{styles.togglePill} {styles.psarPill} {pair.microTerm.showPsar ? styles.active : ''}"
            onclick={togglePsar}>PSAR</button>
        <button class="{styles.togglePill} {styles.williamsRPill} {pair.microTerm.showWilliamsR ? styles.active : ''}"
            onclick={toggleWilliamsR}>W%R</button>
        <button class="{styles.togglePill} {styles.hullMaPill} {pair.microTerm.showHullMa ? styles.active : ''}"
            onclick={toggleHullMa}>HULL MA</button>
        <button class="{styles.togglePill} {styles.aoPill} {pair.microTerm.showAo ? styles.active : ''}"
            onclick={toggleAo}>AO</button>
        <button class="{styles.togglePill} {styles.forceIdxPill} {pair.microTerm.showForceIdx ? styles.active : ''}"
            onclick={toggleForceIdx}>FORCE</button>
        <button class="{styles.togglePill} {styles.stddevChnlPill} {pair.microTerm.showStdDevChnl ? styles.active : ''}"
            onclick={toggleStdDevChnl}>SD CHNL</button>
        <button class="{styles.togglePill} {styles.volumeProfilePill} {pair.microTerm.showVolumeProfile ? styles.active : ''}"
            onclick={toggleVolumeProfile}>VOL PROFILE</button>
        <button class="{styles.togglePill} {styles.smcStructurePill} {pair.microTerm.showSmcStructure ? styles.active : ''}"
            onclick={toggleSmcStructure}>SMC STRUCT</button>
        <button class="{styles.togglePill} {styles.smcLiquidityPill} {pair.microTerm.showSmcLiquidity ? styles.active : ''}"
            onclick={toggleSmcLiquidity}>SMC LIQ</button>
        <button class="{styles.togglePill} {styles.smcFvgPill} {pair.microTerm.showSmcFvg ? styles.active : ''}"
            onclick={toggleSmcFvg}>SMC FVG</button>
        <button class="{styles.togglePill} {styles.smcOrderBlockPill} {pair.microTerm.showSmcOrderBlocks ? styles.active : ''}"
            onclick={toggleSmcOrderBlocks}>SMC OB</button>
    </div>
</div>
{/if}
