<script lang="ts">
    // LevelsView — Facet #4 of the redesigned Metrics view.
    //
    // Surfaces all LevelTest signals, grouped by the kind of price
    // structure they expose (Pivot / Fibonacci / S/R / VWAP / ChannelMid /
    // Ichimoku / VolumeNode / SMC Zones / Other). Each row carries the
    // producer indicator, parsed level name, role (support/resistance/
    // neutral), and the latest status from the signal payload.

    import type {
        IndicatorMeta, IndicatorSignal, TimeframeTelemetry,
    } from '../../types';
    import {
        LEVEL_KIND_ORDER, LEVEL_KIND_META,
        classifyLevelKey, parseLevelLabel,
        type LevelKind,
    } from '../../lib/levelKind';
    import type { FilterState } from '../../lib/filtering';
    import { confPct, dirColor, ageLabel } from '../../lib/scoreStyles';
    import styles from './LevelsView.module.css';

    interface Props {
        tf: TimeframeTelemetry;
        registry: IndicatorMeta[];
        filters: FilterState;
    }

    let { tf, registry, filters }: Props = $props();

    interface LevelRow {
        indicatorKey: string;
        displayName: string;
        signal: IndicatorSignal;
        levelName: string;
        kind: LevelKind;
        role: 'support' | 'resistance' | 'neutral';
    }

    const rows = $derived.by<LevelRow[]>(() => {
        const out: LevelRow[] = [];
        for (const meta of registry) {
            const sigs = tf.indicators?.[meta.key]?.signals ?? [];
            for (const sig of sigs) {
                if (sig.kind !== 'LevelTest') continue;
                if (filters.confirmedPlusOnly && sig.status === 'Potential') continue;
                if (filters.query && !sig.label?.toLowerCase().includes(filters.query.toLowerCase())) continue;
                const parsed = parseLevelLabel(meta.key, sig.label);
                out.push({
                    indicatorKey: meta.key,
                    displayName: meta.display_name,
                    signal: sig,
                    levelName: parsed.name,
                    kind: classifyLevelKey(meta.key),
                    role: parsed.role,
                });
            }
        }
        return out.sort((a, b) => b.signal.strength - a.signal.strength);
    });

    const grouped = $derived.by(() => {
        const map = new Map<LevelKind, LevelRow[]>();
        for (const k of LEVEL_KIND_ORDER) map.set(k, []);
        for (const r of rows) {
            const list = map.get(r.kind);
            if (list) list.push(r);
        }
        return LEVEL_KIND_ORDER
            .map((k) => ({ kind: k, rows: map.get(k) ?? [] }))
            .filter((g) => g.rows.length > 0);
    });

    function confidenceOf(key: string): number {
        return confPct(tf.indicators?.[key]?.confidence ?? 0);
    }

    function roleClass(role: 'support' | 'resistance' | 'neutral'): string {
        if (role === 'support') return styles.roleSupport ?? '';
        if (role === 'resistance') return styles.roleResistance ?? '';
        return styles.roleNeutral ?? '';
    }
</script>

<div class={styles.view}>
    {#if rows.length === 0}
        <div class={styles.placeholder}>
            No active level tests. LevelTest signals fire when price trades
            into a structural level's proximity band (default 0.5% / 0.15% for pivots).
        </div>
    {:else}
        {#each grouped as g (g.kind)}
            {@const meta = LEVEL_KIND_META[g.kind]}
            <section class={styles.section} style="--accent: {meta.accent}">
                <header class={styles.sectionHeader}>
                    <span class={styles.sectionTitle}>{meta.label}</span>
                    <span class={styles.sectionDesc}>{meta.description}</span>
                    <span class={styles.sectionCount}>{g.rows.length}</span>
                </header>
                <div class={styles.body}>
                    {#each g.rows as row (row.indicatorKey + row.signal.label + row.signal.kind)}
                        <div class="{styles.row} {roleClass(row.role)}">
                            <span class={styles.producer}>{row.displayName}</span>
                            <span class="{styles.role} {roleClass(row.role)}">{row.role}</span>
                            <span class={styles.levelName}>{row.levelName}</span>
                            <span class={styles.direction}
                                  style="color: {dirColor(row.signal.direction)}">
                                {row.signal.direction}
                            </span>
                            <span class={styles.status}>{row.signal.status}</span>
                            <span class={styles.strength}>str {(row.signal.strength * 100).toFixed(0)}</span>
                            <span class={styles.conf}>conf {confidenceOf(row.indicatorKey)}%</span>
                            <span class={styles.age}>age {ageLabel(row.signal.age_bars)}</span>
                        </div>
                    {/each}
                </div>
            </section>
        {/each}
    {/if}
</div>
