<script lang="ts">
    import styles from './MtfMatrix.module.css';
    import type { MtfConfirmation, MtfIndicatorRow } from '../../types';

    interface Props {
        mtf: MtfConfirmation | null | undefined;
    }
    let { mtf }: Props = $props();

    let sortByAgreement = $state(true);

    const rows = $derived.by<MtfIndicatorRow[]>(() => {
        const r = [...(mtf?.rows ?? [])];
        if (sortByAgreement) r.sort((a, b) => b.agreement - a.agreement);
        return r;
    });

    // 3-of-4 confirmation: indicators where ≥3 timeframes share one direction.
    const confirmed3of4 = $derived(
        (mtf?.rows ?? []).filter((row) => {
            const bulls = row.per_tf.filter((d) => d > 0).length;
            const bears = row.per_tf.filter((d) => d < 0).length;
            return bulls >= 3 || bears >= 3;
        }).length,
    );

    function agreementColor(a: number): string {
        if (a >= 0.75) return '#10b981';
        if (a >= 0.5) return '#94a3b8';
        return '#ef4444';
    }
    function dirGlyph(d: number): string { return d > 0 ? '▲' : d < 0 ? '▼' : '·'; }
    function dirColor(d: number): string { return d > 0 ? '#10b981' : d < 0 ? '#ef4444' : '#475569'; }
</script>

<div class={styles.matrix}>
    <div class={styles.head}>
        <span class={styles.title}>MULTI-TIMEFRAME CONFIRMATION</span>
        <span class={styles.agree} style="color:{agreementColor((mtf?.trend_agreement_pct ?? 0) / 100)}">
            {Math.round(mtf?.trend_agreement_pct ?? 0)}%
        </span>
        <span class={styles.confirm} title="Indicators with ≥3/4 timeframe agreement">
            {confirmed3of4} @ 3/4
        </span>
        <button class={styles.sortBtn} onclick={() => (sortByAgreement = !sortByAgreement)}>
            {sortByAgreement ? 'SORT: AGREE' : 'SORT: REGISTRY'}
        </button>
    </div>

    {#if rows.length > 0}
        <table class={styles.table}>
            <thead>
                <tr><th>Indicator</th><th>M</th><th>F</th><th>S</th><th>Mac</th><th>Agree</th></tr>
            </thead>
            <tbody>
                {#each rows as row (row.key)}
                    <tr>
                        <td class={styles.label}>{row.display_name}</td>
                        {#each row.per_tf as d}
                            <td class={styles.glyph} style="color:{dirColor(d)}">{dirGlyph(d)}</td>
                        {/each}
                        <td class={styles.agreeCell} style="color:{agreementColor(row.agreement)}">
                            {Math.round(row.agreement * 100)}%
                        </td>
                    </tr>
                {/each}
            </tbody>
        </table>
    {:else}
        <div class={styles.empty}>No cross-timeframe data yet.</div>
    {/if}
</div>
