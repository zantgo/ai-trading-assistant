<script lang="ts">
    import styles from './MomentumMeter.module.css';

    // Continuous [-1.0, 1.0] momentum meter. The needle glides from the far
    // left (-1.0 bearish) through center (0.0 equilibrium) to the far right
    // (+1.0 bullish).
    let {
        label = '',
        normalized = 0,
        stateLabel = '',
    }: { label?: string; normalized?: number; stateLabel?: string } = $props();

    const clamped = $derived.by(() => {
        // Audit fix (m8): a NaN `normalized` propagated into the needle
        // position (`left: NaN%`) and the magnitude tone. Guard it.
        const v = normalized ?? 0;
        if (!Number.isFinite(v)) return 0;
        return Math.max(-1, Math.min(1, v));
    });
    const needleLeft = $derived((clamped + 1) * 50);
    const magnitude = $derived(Math.abs(clamped));

    const toneClass = $derived(
        magnitude >= 0.9
            ? styles.climax
            : clamped > 0.1
              ? styles.bullish
              : clamped < -0.1
                ? styles.bearish
                : styles.neutral,
    );
</script>

<div class={styles.meter}>
    {#if label}
        <div class={styles.header}>
            <span class={styles.label}>{label}</span>
            <span class="{styles.value} {toneClass}">{clamped.toFixed(2)}</span>
        </div>
    {/if}
    <div class={styles.track}>
        <div class={styles.centerTick}></div>
        <div
            class="{styles.fill} {clamped >= 0 ? styles.fillBull : styles.fillBear}"
            style="left: {clamped >= 0 ? 50 : needleLeft}%; width: {magnitude * 50}%;"
        ></div>
        <div class="{styles.needle} {toneClass}" style="left: {needleLeft}%;"></div>
    </div>
    <div class={styles.footer}>
        <span class={styles.poleBear}>-1.0</span>
        {#if stateLabel}
            <span class="{styles.state} {toneClass}">{stateLabel}</span>
        {/if}
        <span class={styles.poleBull}>+1.0</span>
    </div>
</div>
