<script lang="ts">
    import styles from './settings.module.css';

    const INDICATORS = [
        { key: 'rsi', label: 'RSI', defaultWeight: 10 },
        { key: 'rsi_divergence', label: 'RSI Divergence', defaultWeight: 20 },
        { key: 'macd', label: 'MACD', defaultWeight: 10 },
        { key: 'macd_divergence', label: 'MACD Divergence', defaultWeight: 10 },
        { key: 'support_resistance', label: 'S/R Alignment', defaultWeight: 10 },
        { key: 'ema_stack', label: 'Trend (EMA Stack)', defaultWeight: 20 },
        { key: 'ema200', label: '200 EMA', defaultWeight: 10 },
        { key: 'patterns', label: 'Patterns/Breakout', defaultWeight: 10 },
    ];

    let {
        onchange,
        initial,
    }: { onchange?: (w: Record<string, number>) => void; initial?: Record<string, number> | null } = $props();

    let weights = $state<Record<string, number>>(
        initial ? { ...initial } : Object.fromEntries(INDICATORS.map((i) => [i.key, i.defaultWeight])),
    );

    let totalWeight = $derived(Object.values(weights).reduce((s, w) => s + w, 0));

    function update(key: string, value: number) {
        weights = { ...weights, [key]: value };
        onchange?.({ ...weights });
    }
</script>

<div class={styles.panel}>
    <h4 class={styles.panelTitle}>Indicator Weights</h4>
    <p class={styles.panelDesc}>
        Customize the weight of each indicator in the 8-factor confluence score.
    </p>
    <div class={styles.weightGrid}>
        {#each INDICATORS as ind}
            <div class={styles.weightRow}>
                <label class={styles.weightLabel}>{ind.label}</label>
                <div class={styles.weightControl}>
                    <input
                        type="range"
                        min="0"
                        max="100"
                        value={weights[ind.key]}
                        oninput={(e) => update(ind.key, parseInt(e.currentTarget.value))}
                        class={styles.weightSlider}
                    />
                    <input
                        type="number"
                        min="0"
                        max="100"
                        value={weights[ind.key]}
                        oninput={(e) => update(ind.key, parseInt(e.currentTarget.value) || 0)}
                        class={styles.weightInput}
                    />
                </div>
            </div>
        {/each}
    </div>
    <div class={styles.totalRow} class:styles.totalWarn={totalWeight !== 100 && totalWeight !== 90}>
        Total: {totalWeight} / 100
    </div>
</div>
