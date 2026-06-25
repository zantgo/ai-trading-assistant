<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import styles from './ChartToggles.module.css';

    const app = useAppStore();
    let { pairKey }: { pairKey: string } = $props();
    const pair = $derived(app.instancesMap[pairKey]);

    function syncAll(fn: (tf: any) => void) {
        if (!pair) return;
        fn(pair.microTerm);
        fn(pair.smallTerm);
        fn(pair.mediumTerm);
        fn(pair.largeTerm);
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
    </div>
</div>
{/if}
