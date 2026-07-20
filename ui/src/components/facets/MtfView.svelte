<script lang="ts">
    // MtfView — Facet #6 of the redesigned Metrics view.
    //
    // Cross-timeframe comparison: lists every enabled indicator with its
    // normalized value across the 4 timeframes (Micro / Fast / Slow / Macro)
    // and a per-row agreement ratio (bullish / bearish / mixed). Helps the
    // trader see at a glance which indicators agree across timeframes and
    // which diverge — a key signal of regime change.

    import type { IndicatorMeta, TimeframeTelemetry } from '../../types';
    import { GROUP_ORDER, GROUP_META } from '../../lib/groupMeta';
    import { filterRegistry, type FilterState } from '../../lib/filtering';
    import { normColor } from '../../lib/scoreStyles';
    import styles from './MtfView.module.css';

    interface Props {
        pair: {
            microTerm: TimeframeTelemetry;
            fastTerm: TimeframeTelemetry;
            slowTerm: TimeframeTelemetry;
            macroTerm: TimeframeTelemetry;
        };
        registry: IndicatorMeta[];
        filters: FilterState;
    }

    let { pair, registry, filters }: Props = $props();

    interface TimeframeSlot {
        label: string;
        tf: TimeframeTelemetry;
        secs: number;
    }

    const SLOTS = $derived<TimeframeSlot[]>([
        { label: 'Micro', tf: pair.microTerm, secs: pair.microTerm.barDurationSec },
        { label: 'Fast',  tf: pair.fastTerm,  secs: pair.fastTerm.barDurationSec  },
        { label: 'Slow',  tf: pair.slowTerm,  secs: pair.slowTerm.barDurationSec  },
        { label: 'Macro', tf: pair.macroTerm, secs: pair.macroTerm.barDurationSec },
    ]);

    const filteredRegistry = $derived(filterRegistry(registry, filters));

    interface IndicatorMtf {
        meta: IndicatorMeta;
        values: number[];      // 4 entries (one per TF)
        active: boolean[];     // whether the indicator is present on that TF
        agreement: number;     // -1..+1 (avg of values, gated by presence)
        agreementLabel: 'BULL' | 'BEAR' | 'MIXED';
    }

    const rows = $derived.by<IndicatorMtf[]>(() => {
        const out: IndicatorMtf[] = [];
        for (const meta of filteredRegistry) {
            const values: number[] = [];
            const active: boolean[] = [];
            for (const slot of SLOTS) {
                const dto = slot.tf.indicators?.[meta.key];
                if (dto) {
                    values.push(dto.normalized ?? 0);
                    active.push(true);
                } else {
                    values.push(0);
                    active.push(false);
                }
            }
            const presentVals = values.filter((_, i) => active[i]);
            const agreement = presentVals.length > 0
                ? presentVals.reduce((a, b) => a + b, 0) / presentVals.length
                : 0;
            const label: 'BULL' | 'BEAR' | 'MIXED' =
                agreement > 0.2 ? 'BULL' :
                agreement < -0.2 ? 'BEAR' :
                'MIXED';
            out.push({ meta, values, active, agreement, agreementLabel: label });
        }
        return out;
    });

    const groups = $derived.by(() => {
        const map = new Map<string, IndicatorMtf[]>();
        for (const g of GROUP_ORDER) map.set(g, []);
        for (const r of rows) {
            const list = map.get(r.meta.group);
            if (list) list.push(r);
        }
        return GROUP_ORDER
            .map((g) => ({ group: g, items: map.get(g) ?? [] }))
            .filter((g) => g.items.length > 0);
    });

    function fmtTimeframe(secs: number): string {
        if (!secs || secs <= 0) return '--';
        if (secs >= 86400) return `${secs / 86400}d`;
        if (secs >= 3600) return `${secs / 3600}h`;
        if (secs >= 60) return `${secs / 60}m`;
        return `${secs}s`;
    }

    function agClass(label: 'BULL' | 'BEAR' | 'MIXED'): string {
        if (label === 'BULL') return styles.agBull ?? '';
        if (label === 'BEAR') return styles.agBear ?? '';
        return styles.agMixed ?? '';
    }
</script>

<div class={styles.view}>
    {#if rows.length === 0}
        <div class={styles.placeholder}>No indicators match the current filters.</div>
    {:else}
        <div class={styles.summary}>
            {#each SLOTS as slot (slot.label)}
                <div class={styles.summarySlot}>
                    <div class={styles.summaryLabel}>{slot.label}</div>
                    <div class={styles.summarySecs}>{fmtTimeframe(slot.secs)}</div>
                </div>
            {/each}
        </div>

        {#each groups as g (g.group)}
            {@const meta = GROUP_META[g.group as keyof typeof GROUP_META]}
            <section class={styles.section} style="--accent: {meta.accent}">
                <header class={styles.sectionHeader}>
                    <span class={styles.sectionTitle}>{meta.label}</span>
                    <span class={styles.sectionCount}>{g.items.length}</span>
                </header>
                <div class={styles.body}>
                    {#each g.items as r (r.meta.key)}
                        <div class={styles.row}>
                            <span class={styles.indicatorName}>
                                {#if !r.meta.directional}<span class={styles.gateMarker}>◐</span>{/if}
                                {r.meta.display_name}
                            </span>
                            {#each r.values as v, i (i)}
                                <span class="{styles.normCell} {r.active[i] ? '' : styles.normEmpty}"
                                      style="color: {r.active[i] ? normColor(v) : 'rgba(255,255,255,0.2)'}; font-weight: 700;">
                                    {r.active[i] ? (v >= 0 ? '+' : '') + v.toFixed(2) : '·'}
                                </span>
                            {/each}
                            <span class="{styles.agreement} {agClass(r.agreementLabel)}">
                                {r.agreementLabel}
                            </span>
                            <span class={styles.agreementNum}>
                                {(r.agreement >= 0 ? '+' : '') + r.agreement.toFixed(2)}
                            </span>
                        </div>
                    {/each}
                </div>
            </section>
        {/each}
    {/if}
</div>
