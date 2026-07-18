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
    function toggleIchimoku() {
        if (!pair) return;
        const v = !pair.microTerm.showIchimoku;
        syncAll(tf => { tf.showIchimoku = v; });
    }
    function toggleHullMa() {
        if (!pair) return;
        const v = !pair.microTerm.showHullMa;
        syncAll(tf => { tf.showHullMa = v; });
    }
    function togglePsar() {
        if (!pair) return;
        const v = !pair.microTerm.showPsar;
        syncAll(tf => { tf.showPsar = v; });
    }
    function toggleStddevChan() {
        if (!pair) return;
        const v = !pair.microTerm.showStddevChan;
        syncAll(tf => { tf.showStddevChan = v; });
    }
    function toggleFib() {
        if (!pair) return;
        const v = !pair.microTerm.showFib;
        syncAll(tf => { tf.showFib = v; });
    }

    function toggleEma(label: 'Fast' | 'Medium' | 'Slow' | 'Long') {
        if (!pair) return;
        const key = `showEma${label}` as keyof typeof pair;
        (pair as any)[key] = !(pair as any)[key];
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
        <button class="{styles.togglePill} {styles.bbPill} {pair.microTerm.showBb ? styles.active : ''}"
            onclick={toggleBb}>BOLLINGER</button>
        <button class="{styles.togglePill} {styles.supertrendPill} {pair.microTerm.showSupertrend ? styles.active : ''}"
            onclick={toggleSupertrend}>SUPERTREND</button>
        <button class="{styles.togglePill} {styles.keltnerPill} {pair.microTerm.showKeltner ? styles.active : ''}"
            onclick={toggleKeltner}>KELTNER</button>
        <button class="{styles.togglePill} {styles.donchianPill} {pair.microTerm.showDonchian ? styles.active : ''}"
            onclick={toggleDonchian}>DONCHIAN</button>
    </div>
    <div class={styles.togglesSeparator}></div>
    <div class={styles.togglesGroup}>
        <button class="{styles.togglePill} {pair.microTerm.showIchimoku ? styles.active : ''}"
            onclick={toggleIchimoku}>ICHIMOKU</button>
        <button class="{styles.togglePill} {pair.microTerm.showHullMa ? styles.active : ''}"
            onclick={toggleHullMa}>HULL MA</button>
        <button class="{styles.togglePill} {pair.microTerm.showPsar ? styles.active : ''}"
            onclick={togglePsar}>PSAR</button>
        <button class="{styles.togglePill} {pair.microTerm.showStddevChan ? styles.active : ''}"
            onclick={toggleStddevChan}>STDDEV</button>
        <button class="{styles.togglePill} {pair.microTerm.showFib ? styles.active : ''}"
            onclick={toggleFib}>FIB</button>
    </div>
</div>
{/if}
