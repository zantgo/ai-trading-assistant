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
    //
    // v6.13: the freeform cross-TF signals list is replaced by three stacked
    // 4-TF-column tables in the same visual language as the indicator grid
    // (each with its own Micro / Fast / Slow / Macro header row):
    //   SIGNALS      — 12 signal kinds × per-TF active-signal counts
    //   DIVERGENCES  — divergence-capable indicators × strongest sub-type per TF
    //   LEVELS       — 9 level kinds × per-TF LevelTest-signal counts

    import type {
        IndicatorMeta, IndicatorSignal, SignalKind, TimeframeTelemetry,
    } from '../../types';
    import { GROUP_ORDER, GROUP_META } from '../../lib/groupMeta';
    import { normColor } from '../../lib/scoreStyles';
    import {
        classifyDivergence, divergenceLabel, divergenceAccent,
        type DivergenceSubKind,
    } from '../../lib/divergence';
    import {
        LEVEL_KIND_ORDER, LEVEL_KIND_META, classifyLevelKey,
        type LevelKind,
    } from '../../lib/levelKind';
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

    // ── Stacked cross-timeframe tables (v6.13) ────────────────────────

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

    // ── Signals: kind × TF active counts ──
    interface MtfSignalRow {
        slotIndex: number;
        signal: IndicatorSignal;
    }

    const signalRows = $derived.by<MtfSignalRow[]>(() => {
        const out: MtfSignalRow[] = [];
        SLOTS.forEach((slot, slotIndex) => {
            for (const meta of registry) {
                const sigs = slot.tf.indicators?.[meta.key]?.signals ?? [];
                for (const sig of sigs) {
                    out.push({ slotIndex, signal: sig });
                }
            }
        });
        return out;
    });

    const signalCountsByKind = $derived.by<Record<SignalKind, number[]>>(() => {
        const out = {} as Record<SignalKind, number[]>;
        for (const k of SIGNAL_KIND_ORDER) out[k] = [0, 0, 0, 0];
        for (const r of signalRows) {
            if (!out[r.signal.kind]) continue;
            out[r.signal.kind][r.slotIndex]++;
        }
        return out;
    });

    const totalSignalCount = $derived<number>(signalRows.length);

    // ── Divergences: capable indicators × strongest sub-type per TF ──
    const DIVERGENCE_KEYS = new Set([
        'rsi', 'macd', 'stochastic', 'chandemo',
        'obv', 'cmf', 'mfi', 'squeeze', 'oi_price_divergence',
    ]);

    interface MtfDivergenceCell {
        sub: DivergenceSubKind | null;
        strength: number;
    }

    interface MtfDivergenceRow {
        meta: IndicatorMeta;
        cells: MtfDivergenceCell[]; // 4 entries (one per TF)
    }

    const divergenceRows = $derived.by<MtfDivergenceRow[]>(() => {
        const out: MtfDivergenceRow[] = [];
        for (const meta of registry) {
            if (!DIVERGENCE_KEYS.has(meta.key) && !meta.supports_divergence) continue;
            const cells: MtfDivergenceCell[] = SLOTS.map((slot) => {
                const sigs = slot.tf.indicators?.[meta.key]?.signals ?? [];
                let best: MtfDivergenceCell = { sub: null, strength: -1 };
                for (const sig of sigs) {
                    if (sig.kind !== 'Divergence') continue;
                    const sub = classifyDivergence(sig.label, sig.points ?? null, sig.direction);
                    if (sig.strength > best.strength) best = { sub, strength: sig.strength };
                }
                return best.sub ? best : { sub: null, strength: 0 };
            });
            out.push({ meta, cells });
        }
        return out;
    });

    const totalDivergenceCount = $derived<number>(
        divergenceRows.reduce((acc, r) => acc + r.cells.filter((c) => c.sub).length, 0),
    );

    function divShort(sub: DivergenceSubKind | null): string {
        switch (sub) {
            case 'RegularBull': return 'BULL';
            case 'RegularBear': return 'BEAR';
            case 'HiddenBull':  return 'H-BULL';
            case 'HiddenBear':  return 'H-BEAR';
            case 'Unknown':     return 'UNK';
            default:            return '·';
        }
    }

    // ── Levels: level kind × TF LevelTest counts ──
    const levelCountsByKind = $derived.by<Record<LevelKind, number[]>>(() => {
        const out = {} as Record<LevelKind, number[]>;
        for (const k of LEVEL_KIND_ORDER) out[k] = [0, 0, 0, 0];
        SLOTS.forEach((slot, slotIndex) => {
            for (const meta of registry) {
                const sigs = slot.tf.indicators?.[meta.key]?.signals ?? [];
                for (const sig of sigs) {
                    if (sig.kind !== 'LevelTest') continue;
                    const kind = classifyLevelKey(meta.key);
                    if (!out[kind]) continue;
                    out[kind][slotIndex]++;
                }
            }
        });
        return out;
    });

    const totalLevelCount = $derived<number>(
        LEVEL_KIND_ORDER.reduce((acc, k) => acc + levelCountsByKind[k].reduce((a, b) => a + b, 0), 0),
    );

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

        <!-- ── Stacked cross-timeframe tables (v6.13) ─────────────────────
             Signals / Divergences / Levels each rendered as a 4-TF-column
             table in the same visual language as the indicator grid above:
             a per-table Micro/Fast/Slow/Macro header row, one row per
             entity, '·' in empty cells. -->
        {#snippet tfHeaderRow(firstLabel: string)}
            <div class={styles.tblSummary}>
                <span class={styles.tblSummaryFirst}>{firstLabel}</span>
                {#each SLOTS as slot (slot.label)}
                    <div class={styles.summarySlot}>
                        <div class={styles.summaryLabel}>{slot.label}</div>
                        <div class={styles.summarySecs}>{fmtTimeframe(slot.secs)}</div>
                    </div>
                {/each}
                <span class={styles.tblSummaryTotal}>TOTAL</span>
            </div>
        {/snippet}

        <!-- ── Signals: kind × TF active counts ── -->
        <section class="{styles.tblSection} {styles.tblSignals}">
            <header class={styles.tblHeader}>
                <span class={styles.tblTitle}>Signals</span>
                <span class={styles.tblCount}>{totalSignalCount} signal{totalSignalCount === 1 ? '' : 's'}</span>
                <span class={styles.tblHint}>active signals per kind, per timeframe</span>
            </header>
            {@render tfHeaderRow('KIND')}
            {#if totalSignalCount === 0}
                <div class={styles.tblEmpty}>
                    No signals active. Signals are published on each completed candle.
                </div>
            {:else}
                <div class={styles.tblBody}>
                    {#each SIGNAL_KIND_ORDER as kind (kind)}
                        {@const counts = signalCountsByKind[kind]}
                        {@const kindTotal = counts.reduce((a, b) => a + b, 0)}
                        <div class={styles.tblRow}>
                            <span class={styles.tblKind}>
                                <span class={styles.tblKindName}>{kind}</span>
                                <span class={styles.tblKindAbbr}>{SIGNAL_ABBR[kind]}</span>
                            </span>
                            {#each counts as c, i (i)}
                                <span class="{styles.tblCountCell} {c > 0 ? '' : styles.tblCellEmpty}">
                                    {c > 0 ? c : '·'}
                                </span>
                            {/each}
                            <span class={styles.tblTotal}>{kindTotal}</span>
                        </div>
                    {/each}
                </div>
            {/if}
        </section>

        <!-- ── Divergences: capable indicators × strongest sub-type per TF ── -->
        <section class="{styles.tblSection} {styles.tblDivergences}">
            <header class={styles.tblHeader}>
                <span class={styles.tblTitle}>Divergences</span>
                <span class={styles.tblCount}>{totalDivergenceCount} divergence{totalDivergenceCount === 1 ? '' : 's'}</span>
                <span class={styles.tblHint}>strongest active divergence per oscillator, per timeframe</span>
            </header>
            {@render tfHeaderRow('INDICATOR')}
            {#if totalDivergenceCount === 0}
                <div class={styles.tblEmpty}>
                    No active divergences. Divergence signals appear when an oscillator
                    disagrees directionally with price over 20-bar pivots.
                </div>
            {:else}
                <div class={styles.tblBody}>
                    {#each divergenceRows as r (r.meta.key)}
                        <div class={styles.tblRow}>
                            <span class={styles.tblKindName}>{r.meta.display_name}</span>
                            {#each r.cells as cell, i (i)}
                                {@const sub = cell.sub}
                                <span
                                    class="{styles.tblCountCell} {sub ? '' : styles.tblCellEmpty}"
                                    style={sub ? `color: ${divergenceAccent(sub)}; font-weight: 700;` : ''}
                                    title={sub
                                        ? `${divergenceLabel(sub)} · str ${(cell.strength * 100).toFixed(0)}%`
                                        : 'no active divergence'}
                                >
                                    {sub ? divShort(sub) : '·'}
                                </span>
                            {/each}
                            <span class={styles.tblTotal}>{r.cells.filter((c) => c.sub).length}</span>
                        </div>
                    {/each}
                </div>
            {/if}
        </section>

        <!-- ── Levels: level kind × TF LevelTest counts ── -->
        <section class="{styles.tblSection} {styles.tblLevels}">
            <header class={styles.tblHeader}>
                <span class={styles.tblTitle}>Levels</span>
                <span class={styles.tblCount}>{totalLevelCount} level test{totalLevelCount === 1 ? '' : 's'}</span>
                <span class={styles.tblHint}>active LevelTest signals per level kind, per timeframe</span>
            </header>
            {@render tfHeaderRow('LEVEL KIND')}
            {#if totalLevelCount === 0}
                <div class={styles.tblEmpty}>
                    No active level tests. LevelTest signals fire when price trades
                    into a structural level's proximity band.
                </div>
            {:else}
                <div class={styles.tblBody}>
                    {#each LEVEL_KIND_ORDER as kind (kind)}
                        {@const counts = levelCountsByKind[kind]}
                        {@const kindTotal = counts.reduce((a, b) => a + b, 0)}
                        <div class={styles.tblRow}>
                            <span class={styles.tblKind}>
                                <span class={styles.tblKindName}>{LEVEL_KIND_META[kind].label}</span>
                            </span>
                            {#each counts as c, i (i)}
                                <span class="{styles.tblCountCell} {c > 0 ? '' : styles.tblCellEmpty}">
                                    {c > 0 ? c : '·'}
                                </span>
                            {/each}
                            <span class={styles.tblTotal}>{kindTotal}</span>
                        </div>
                    {/each}
                </div>
            {/if}
        </section>
    {/if}
</div>
