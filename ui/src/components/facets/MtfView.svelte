<script lang="ts">
    // MtfView — Facet #6 of the redesigned Metrics view.
    //
    // Cross-timeframe comparison: lists every enabled indicator with its
    // normalized value across the 4 timeframes (Micro / Fast / Slow / Macro)
    // and a per-row agreement ratio (bullish / bearish / mixed). Helps the
    // trader see at a glance which indicators agree across timeframes and
    // which diverge — a key signal of regime change.
    //
    // v6.11: filtering was removed entirely — the grid always lists every
    // registered indicator, and a dedicated CROSS-TIMEFRAME SIGNALS section
    // below the grid shows EVERY signal from EVERY timeframe, unfiltered,
    // tagged with its producing timeframe.

    import type {
        IndicatorMeta, IndicatorSignal, SignalKind, TimeframeTelemetry,
    } from '../../types';
    import { GROUP_ORDER, GROUP_META } from '../../lib/groupMeta';
    import { normColor } from '../../lib/scoreStyles';
    import { dirColor, dirClass, ageLabel } from '../../lib/scoreStyles';
    import styles from './MtfView.module.css';

    interface Props {
        pair: {
            microTerm: TimeframeTelemetry;
            fastTerm: TimeframeTelemetry;
            slowTerm: TimeframeTelemetry;
            macroTerm: TimeframeTelemetry;
        };
        registry: IndicatorMeta[];
    }

    let { pair, registry }: Props = $props();

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

    interface IndicatorMtf {
        meta: IndicatorMeta;
        values: number[];      // 4 entries (one per TF)
        active: boolean[];     // whether the indicator is present on that TF
        agreement: number;     // -1..+1 (avg of values, gated by presence)
        agreementLabel: 'BULL' | 'BEAR' | 'MIXED';
    }

    const rows = $derived.by<IndicatorMtf[]>(() => {
        const out: IndicatorMtf[] = [];
        for (const meta of registry) {
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

    // ── Cross-Timeframe signals (v6.11: ALL signals from ALL 4 TFs) ──
    const SIGNAL_KIND_ORDER: SignalKind[] = [
        'Divergence', 'Crossover', 'Threshold', 'Breakout', 'BandTouch',
        'ZeroLineCross', 'CompressionRelease', 'LevelTest', 'TrendFlip',
        'VolumeClimax', 'StackChange', 'PatternForming',
    ];

    const SIGNAL_ABBR: Record<string, string> = {
        Divergence: 'DIV', Crossover: 'CRO', Threshold: 'TH', Breakout: 'BO',
        BandTouch: 'BT', ZeroLineCross: '0X', CompressionRelease: 'SQZ',
        LevelTest: 'LV', TrendFlip: 'FLIP', VolumeClimax: 'VOL',
        StackChange: 'STK', PatternForming: 'PAT',
    };

    interface MtfSignalRow {
        slotLabel: string;
        displayName: string;
        signal: IndicatorSignal;
    }

    const signalRows = $derived.by<MtfSignalRow[]>(() => {
        const out: MtfSignalRow[] = [];
        for (const slot of SLOTS) {
            for (const meta of registry) {
                const sigs = slot.tf.indicators?.[meta.key]?.signals ?? [];
                for (const sig of sigs) {
                    out.push({ slotLabel: slot.label, displayName: meta.display_name, signal: sig });
                }
            }
        }
        return out;
    });

    const signalsByKind = $derived.by<Record<SignalKind, MtfSignalRow[]>>(() => {
        const out = {} as Record<SignalKind, MtfSignalRow[]>;
        for (const k of SIGNAL_KIND_ORDER) out[k] = [];
        for (const r of signalRows) {
            if (!out[r.signal.kind]) out[r.signal.kind] = [];
            out[r.signal.kind].push(r);
        }
        for (const k of SIGNAL_KIND_ORDER) {
            out[k].sort((a, b) => b.signal.strength - a.signal.strength);
        }
        return out;
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

    function sigRowClass(status: string): string {
        if (status === 'Confirmed') return styles.sigConfirmed ?? '';
        if (status === 'Active') return styles.sigActive ?? '';
        return '';
    }
</script>

<div class={styles.view}>
    {#if rows.length === 0}
        <div class={styles.placeholder}>No indicators in the registry yet. Awaiting indicator registry…</div>
    {:else}
        <div class={styles.summary}>
            <div class={styles.summarySpacer}></div>
            {#each SLOTS as slot (slot.label)}
                <div class={styles.summarySlot}>
                    <div class={styles.summaryLabel}>{slot.label}</div>
                    <div class={styles.summarySecs}>{fmtTimeframe(slot.secs)}</div>
                </div>
            {/each}
            <div class={styles.summarySpacer}></div>
            <div class={styles.summarySpacer}></div>
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

        <!-- ── Cross-Timeframe Signals (v6.11: every signal, every TF, unfiltered) ── -->
        <section class={styles.sigSection}>
            <header class={styles.sigHeader}>
                <span class={styles.sigTitle}>Cross-Timeframe Signals</span>
                <span class={styles.sigCount}>{signalRows.length} signal{signalRows.length === 1 ? '' : 's'}</span>
                <span class={styles.sigHint}>all signals across all timeframes — unfiltered</span>
            </header>
            {#if signalRows.length === 0}
                <div class={styles.sigEmpty}>
                    No signals active. Signals are published on each completed candle.
                </div>
            {:else}
                {#each SIGNAL_KIND_ORDER as kind}
                    {@const sigs = signalsByKind[kind]}
                    {#if sigs.length > 0}
                        <div class={styles.sigKindBlock}>
                            <div class={styles.sigKindHeader}>
                                <span class={styles.sigKindName}>{kind}</span>
                                <span class={styles.sigKindAbbr}>{SIGNAL_ABBR[kind]}</span>
                                <span class={styles.sigKindCount}>{sigs.length}</span>
                            </div>
                            <div class={styles.sigRows}>
                                {#each sigs as row (row.slotLabel + row.displayName + row.signal.label + row.signal.kind)}
                                    <div class="{styles.sigRow} {sigRowClass(row.signal.status)}">
                                        <span class={styles.sigSlot}>{row.slotLabel}</span>
                                        <span class={styles.sigIndicator}>{row.displayName}</span>
                                        <span class="{styles.sigDir} {styles[dirClass(row.signal.direction)]}"
                                              style="color: {dirColor(row.signal.direction)}">
                                            {row.signal.direction}
                                        </span>
                                        <span class={styles.sigStatus}>{row.signal.status}</span>
                                        <span class={styles.sigLabel}>{row.signal.label}</span>
                                        <span class={styles.sigMeta}>
                                            <span class={styles.sigMetaPill}>str {(row.signal.strength * 100).toFixed(0)}</span>
                                            <span class={styles.sigMetaPill}>age {ageLabel(row.signal.age_bars)}</span>
                                        </span>
                                    </div>
                                {/each}
                            </div>
                        </div>
                    {/if}
                {/each}
            {/if}
        </section>
    {/if}
</div>
