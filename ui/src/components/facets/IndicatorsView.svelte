<script lang="ts">
    // IndicatorsView — Facet #1 of the redesigned Metrics view.
    //
    // Renders the full registry of enabled, directional indicators grouped
    // into 8 functional groups. Each row exposes the standard indicator
    // payload (raw / norm / state / confidence) plus the signals count;
    // clicking the row expands it inline to show the full signals[] and
    // any auxiliary `values{}` sub-fields (e.g. MACD line/signal/hist).
    //
    // Replaces the previous TelemetryTable.svelte, which grouped rows by
    // IndicatorClass (Leading/Hybrid/Lagging) — a developer taxonomy that
    // doesn't match how a trader thinks about the market.

    import type { IndicatorDto, IndicatorMeta, IndicatorSignal, TimeframeTelemetry } from '../../types';
    import { GROUP_ORDER, GROUP_META } from '../../lib/groupMeta';
    import { filterRegistry, filterSignals, type FilterState } from '../../lib/filtering';
    import { iRaw, iSub, fmt, fmtPrice, isSqueezeOn } from '../../lib/telemetry';
    import { confPct, normColor, dirColor, dirClass, ageLabel } from '../../lib/scoreStyles';
    import styles from './IndicatorsView.module.css';

    interface Props {
        tf: TimeframeTelemetry;
        registry: IndicatorMeta[];
        filters: FilterState;
        /** When set, this group is expanded by default (used by GroupConfluenceGrid scroll-to). */
        focusGroup?: string | null;
    }

    let { tf, registry, filters, focusGroup = null }: Props = $props();

    const SIGNAL_ABBR: Record<string, string> = {
        Divergence: 'DIV', Crossover: 'CRO', Threshold: 'TH', Breakout: 'BO',
        BandTouch: 'BT', ZeroLineCross: '0X', CompressionRelease: 'SQZ',
        LevelTest: 'LV', TrendFlip: 'FLIP', VolumeClimax: 'VOL',
        StackChange: 'STK', PatternForming: 'PAT',
    };

    const filteredRegistry = $derived(
        filterRegistry(registry, filters, (k) => tf.indicators?.[k]?.signals ?? []),
    );

    const groups = $derived.by(() => {
        const map = new Map<string, IndicatorMeta[]>();
        for (const g of GROUP_ORDER) map.set(g, []);
        for (const m of filteredRegistry) {
            const list = map.get(m.group);
            if (list) list.push(m);
        }
        return GROUP_ORDER
            .map((g) => ({ group: g, items: map.get(g) ?? [] }))
            .filter((g) => g.items.length > 0);
    });

    let expandedGroups = $state<Record<string, boolean>>({});
    let expandedRows = $state<Record<string, boolean>>({});

    // Auto-expand focused group when it changes.
    $effect(() => {
        if (focusGroup) {
            expandedGroups[focusGroup] = true;
        }
    });

    function signalsFor(key: string): IndicatorSignal[] {
        return tf.indicators?.[key]?.signals ?? [];
    }

    function filteredSignalsFor(key: string): IndicatorSignal[] {
        return filterSignals(signalsFor(key), filters);
    }

    function rawVal(meta: IndicatorMeta): number | null {
        if (meta.value_source.startsWith('sub:')) {
            return iSub(tf.indicators ?? {}, meta.key, meta.value_source.slice(4));
        }
        return iRaw(tf.indicators ?? {}, meta.key);
    }

    function formatRaw(meta: IndicatorMeta): string {
        if (meta.value_format === 'onoff') {
            return meta.key === 'squeeze'
                ? (isSqueezeOn(tf.indicators ?? {}) ? 'ON' : 'OFF')
                : (rawVal(meta) != null ? 'ON' : 'OFF');
        }
        const v = rawVal(meta);
        switch (meta.value_format) {
            case 'percent1':  return v == null ? '--' : `${v.toFixed(1)}%`;
            case 'price':     return fmtPrice(v, parseFloat(tf.priceText ?? '0') || 0);
            case 'ratio2':    return (v ?? 1).toFixed(2);
            case 'decimals1': return fmt(v, 1);
            case 'decimals4': return fmt(v, 4);
            case 'decimals2':
            default:          return fmt(v, 2);
        }
    }

    function normalized(key: string): number {
        return tf.indicators?.[key]?.normalized ?? 0;
    }

    function stateLabel(key: string): string {
        return tf.indicators?.[key]?.state_label ?? '--';
    }

    function confidence(key: string): number {
        return confPct(tf.indicators?.[key]?.confidence ?? 0);
    }

    function valuesList(key: string): Array<[string, number]> {
        const v = tf.indicators?.[key]?.values;
        if (!v) return [];
        return Object.entries(v).filter(([, n]) => n != null && !isNaN(n));
    }

    function badgeStyle(sig: IndicatorSignal): { text: string; cls: string; title: string } {
        const abbr = SIGNAL_ABBR[sig.kind] ?? sig.kind;
        const age = (sig.age_bars ?? 0) === 0 ? '' : `·${sig.age_bars}`;
        const cls = dirClass(sig.direction);
        return {
            text: `${abbr}${age}`,
            cls,
            title: `${sig.kind} · ${sig.direction} · ${sig.status} (${ageLabel(sig.age_bars)}): ${sig.label}`,
        };
    }

    function toggleGroup(g: string) {
        expandedGroups[g] = !expandedGroups[g];
        expandedGroups = { ...expandedGroups };
    }

    function toggleRow(key: string) {
        expandedRows[key] = !expandedRows[key];
        expandedRows = { ...expandedRows };
    }
</script>

<div class={styles.view}>
    {#if filteredRegistry.length === 0}
        <div class={styles.placeholder}>
            No indicators match the current filters.
        </div>
    {:else}
        {#each groups as g (g.group)}
            {@const meta = GROUP_META[g.group as keyof typeof GROUP_META]}
            {@const isOpen = expandedGroups[g.group] ?? true}
            <section class={styles.group} style="--accent: {meta.accent}">
                <button class={styles.groupHeader} onclick={() => toggleGroup(g.group)}>
                    <span class={styles.caret}>{isOpen ? '▼' : '▶'}</span>
                    <span class={styles.groupName}>{meta.label}</span>
                    <span class={styles.groupCount}>{g.items.length}</span>
                </button>
                {#if isOpen}
                    <div class={styles.tableWrap}>
                        <div class={styles.colHeaders}>
                            <span class={styles.colName}>Indicator</span>
                            <span class={styles.colClass}>Class</span>
                            <span class={styles.colRaw}>Raw</span>
                            <span class={styles.colNorm}>Norm</span>
                            <span class={styles.colState}>State</span>
                            <span class={styles.colConf}>Conf</span>
                            <span class={styles.colSig}>Signals</span>
                        </div>
                        {#each g.items as m (m.key)}
                            {@const n = normalized(m.key)}
                            {@const sigs = filteredSignalsFor(m.key)}
                            {@const rowOpen = expandedRows[m.key] ?? false}
                            {@const normBg = normColor(n)}
                            <div class={styles.rowWrap}>
                                <button
                                    class="{styles.row} {rowOpen ? styles.rowOpen : ''}"
                                    onclick={() => toggleRow(m.key)}
                                >
                                    <span class={styles.colName}>
                                        {#if !m.directional}<span class={styles.gateMarker} title="Non-directional gate">◐</span>{/if}
                                        {m.display_name}
                                        {#if m.supports_divergence}<span class={styles.divMarker} title="Supports divergence">△</span>{/if}
                                    </span>
                                    <span class="{styles.colClass} {styles[`class_${m.class}`]}">{m.class}</span>
                                    <span class={styles.colRaw}>{formatRaw(m)}</span>
                                    <span class="{styles.colNorm}" style="color: {normBg}; font-weight: 700;">
                                        {n.toFixed(2)}
                                    </span>
                                    <span class={styles.colState}>{stateLabel(m.key)}</span>
                                    <span class={styles.colConf}>{confidence(m.key)}%</span>
                                    <span class={styles.colSig}>
                                        {#if sigs.length === 0}
                                            <span class={styles.sigEmpty}>·</span>
                                        {:else}
                                            {#each sigs as sig (sig.label + sig.kind)}
                                                {@const b = badgeStyle(sig)}
                                                <span class="{styles.signalBadge} {styles[b.cls]} {sig.status === 'Confirmed' ? styles.signalConfirmed : ''} {sig.status === 'Active' ? styles.signalActive : ''}"
                                                      title={b.title}>{b.text}</span>
                                            {/each}
                                        {/if}
                                    </span>
                                </button>

                                {#if rowOpen}
                                    <div class={styles.expanded}>
                                        {#if sigs.length > 0}
                                            <div class={styles.expSection}>
                                                <div class={styles.expLabel}>Active Signals ({sigs.length})</div>
                                                {#each sigs as sig (sig.label + sig.kind)}
                                                    {@const sc = sig.status === 'Confirmed' ? styles.sigConfirmed : sig.status === 'Active' ? styles.sigActive : ''}
                                                    <div class="{styles.expSigRow} {sc}">
                                                        <span class="{styles.expSigBadge} {styles[dirClass(sig.direction)]}">{SIGNAL_ABBR[sig.kind] ?? sig.kind}</span>
                                                        <span class={styles.expSigLabel}>{sig.label}</span>
                                                        <span class={styles.expSigMeta}>
                                                            <span class={styles.expMetaItem} style="color: {dirColor(sig.direction)}">{sig.direction}</span>
                                                            <span class={styles.expMetaItem}>{sig.status}</span>
                                                            <span class={styles.expMetaItem}>str {(sig.strength * 100).toFixed(0)}</span>
                                                            <span class={styles.expMetaItem}>age {ageLabel(sig.age_bars)}</span>
                                                        </span>
                                                    </div>
                                                {/each}
                                            </div>
                                        {/if}

                                        {#if valuesList(m.key).length > 0}
                                            <div class={styles.expSection}>
                                                <div class={styles.expLabel}>Component Lines</div>
                                                <div class={styles.expValuesGrid}>
                                                    {#each valuesList(m.key) as [k, v] (k)}
                                                        <div class={styles.expValueCell}>
                                                            <span class={styles.expValueKey}>{k}</span>
                                                            <span class={styles.expValueNum}>{typeof v === 'number' ? v.toFixed(4) : v}</span>
                                                        </div>
                                                    {/each}
                                                </div>
                                            </div>
                                        {/if}
                                    </div>
                                {/if}
                            </div>
                        {/each}
                    </div>
                {/if}
            </section>
        {/each}
    {/if}
</div>
