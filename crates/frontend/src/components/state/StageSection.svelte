<script lang="ts">
    import { untrack } from 'svelte';
    import styles from './StageSection.module.css';
    import IndicatorCard from './IndicatorCard.svelte';
    import { STAGE_META, categoryForKey, type DecisionStage } from '../../lib/decisionStages';
    import type { IndicatorMap, IndicatorMeta } from '../../types';

    interface Props {
        stage: DecisionStage;
        metas: IndicatorMeta[];
        map: IndicatorMap | undefined | null;
        priceRef?: number;
        weights?: Record<string, number>;
        sparks?: Record<string, number[]>;
        startOpen?: boolean;
    }
    let { stage, metas, map, priceRef = 0, weights = {}, sparks = {}, startOpen = true }: Props = $props();

    let open = $state(untrack(() => startOpen));

    const meta = $derived(STAGE_META[stage]);

    // Stage roll-up: mean normalized over present, directional, non-neutral
    // indicators + count of live signals. Purely presentational aggregation.
    const rollup = $derived.by(() => {
        let sum = 0;
        let n = 0;
        let signalCount = 0;
        for (const m of metas) {
            const dto = map?.[m.key];
            if (!dto) continue;
            signalCount += dto.signals?.length ?? 0;
            if (!m.directional) continue;
            if (Math.abs(dto.normalized) < 0.02) continue;
            sum += dto.normalized;
            n += 1;
        }
        const net = n > 0 ? sum / n : 0;
        return { net, active: n, signalCount };
    });

    function netColor(net: number): string {
        const mag = Math.min(Math.abs(net), 1);
        if (mag >= 0.9) return '#a855f7';
        if (net > 0.1) return `rgb(16, ${Math.round(120 + 135 * mag)}, 129)`;
        if (net < -0.1) return `rgb(${Math.round(180 + 59 * mag)}, 68, 68)`;
        return '#94a3b8';
    }
    const netFillWidth = $derived(`${Math.min(Math.abs(rollup.net), 1) * 50}%`);
    const netFillLeft = $derived(rollup.net >= 0 ? '50%' : `${50 - Math.min(Math.abs(rollup.net), 1) * 50}%`);
</script>

<section class={styles.section} data-stage={stage}>
    <button class={styles.header} onclick={() => (open = !open)} aria-expanded={open}>
        <span class={styles.chevron} data-open={open}>▸</span>
        <span class={styles.title}>{meta.title}</span>
        <span class={styles.subtitle}>{meta.subtitle}</span>

        <span class={styles.rollupBar} title="Stage net bias {rollup.net.toFixed(2)}">
            <span class={styles.rollupZero}></span>
            <span class={styles.rollupFill} style="left:{netFillLeft};width:{netFillWidth};background:{netColor(rollup.net)}"></span>
        </span>
        <span class={styles.rollupVal} style="color:{netColor(rollup.net)}">
            {rollup.net >= 0 ? '+' : ''}{rollup.net.toFixed(2)}
        </span>
        {#if rollup.signalCount > 0}
            <span class={styles.sigCount} title="Live signals in stage">{rollup.signalCount}⚡</span>
        {/if}
    </button>

    {#if open}
        <div class={styles.body}>
            {#each metas as m (m.key)}
                <IndicatorCard
                    meta={m}
                    {map}
                    category={categoryForKey(m.key, m.group)}
                    {priceRef}
                    weight={weights[m.key] ?? null}
                    spark={sparks[m.key] ?? []}
                />
            {/each}
        </div>
    {/if}
</section>
