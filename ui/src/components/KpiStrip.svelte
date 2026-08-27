<script lang="ts">
    // KpiStrip — the HeaderKpiStrip card vocabulary as a data-driven
    // component: { label, value, sub, color? }[]. Rendering identical
    // KPI cards across TAE / PME / PAE keeps the three engines visually
    // consistent.
    import styles from '../styles/engine-dashboard.module.css';

    export interface KpiItem {
        label: string;
        value: string;
        sub?: string;
        color?: string;
    }

    let { items }: { items: KpiItem[] } = $props();
</script>

<div class={styles.kpiStrip}>
    {#each items as kpi (kpi.label)}
        <div class={styles.kpi}>
            <div class={styles.kpiLabel}>{kpi.label}</div>
            <div class={styles.kpiValue} style={kpi.color ? `color: ${kpi.color}` : ''}>{kpi.value}</div>
            {#if kpi.sub}
                <div class={styles.kpiSub}>{kpi.sub}</div>
            {/if}
        </div>
    {/each}
</div>
