<script lang="ts">
    // LevelsView — Facet #4 of the redesigned Metrics view.
    //
    // Surfaces all LevelTest signals, grouped by the kind of price
    // structure they expose (Pivot / Fibonacci / S/R / VWAP / ChannelMid /
    // Ichimoku / VolumeNode / SMC Zones / Other). Each row carries the
    // producer indicator, parsed level name, role (support/resistance/
    // neutral), and the latest status from the signal payload.
    //
    // A Fibonacci Ladder is rendered at the top of the view, surfacing the
    // current retracement grid (0.236 … 0.786) + Golden Pocket zone +
    // extension targets — independent of whether a LevelTest signal fired.

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
    import { formatTimeframeLabel } from '../../lib/telemetry';
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

    // ── Fibonacci Ladder data ──
    const fibVals = $derived.by<Record<string, number | undefined>>(() => {
        const dto = tf.indicators?.['fibonacci'];
        return (dto?.values ?? {}) as Record<string, number | undefined>;
    });

    const fibNorm = $derived<number>(
        tf.indicators?.['fibonacci']?.normalized ?? 0,
    );

    const fibConfidence = $derived.by<number>(() => {
        const dto = tf.indicators?.['fibonacci'];
        return confPct(dto?.confidence ?? 0);
    });

    const fibSwing = $derived.by<'BULL' | 'BEAR' | 'NEUTRAL'>(() => {
        if (fibNorm > 0.05) return 'BULL';
        if (fibNorm < -0.05) return 'BEAR';
        return 'NEUTRAL';
    });

    const fibCoefficients: Array<{ key: string; label: string; gp?: boolean }> = [
        { key: 'fib_0236', label: '0.236' },
        { key: 'fib_0382', label: '0.382' },
        { key: 'fib_0500', label: '0.500' },
        { key: 'fib_0618', label: '0.618', gp: true },
        { key: 'fib_0660', label: '0.660', gp: true },
        { key: 'fib_0786', label: '0.786' },
    ];

    const fibGpTop = $derived<number | null>(
        typeof fibVals['gp_top'] === 'number' ? fibVals['gp_top'] : null,
    );
    const fibGpBottom = $derived<number | null>(
        typeof fibVals['gp_bottom'] === 'number' ? fibVals['gp_bottom'] : null,
    );
    const fibExt1618 = $derived<number | null>(
        typeof fibVals['ext_1618'] === 'number' ? fibVals['ext_1618'] : null,
    );
    const fibExt2618 = $derived<number | null>(
        typeof fibVals['ext_2618'] === 'number' ? fibVals['ext_2618'] : null,
    );

    const fibPosition = $derived.by<string>(() => {
        const top = fibGpTop;
        const bot = fibGpBottom;
        const price = tf.priceText ? parseFloat(tf.priceText) : NaN;
        if (top == null || bot == null || !isFinite(price) || price <= 0) return 'NO DATA';
        const lo = Math.min(top, bot);
        const hi = Math.max(top, bot);
        if (price >= lo && price <= hi) return 'INSIDE GP';
        if (price > hi) return `+${((price - hi) / hi * 100).toFixed(2)}% ABOVE GP`;
        return `${((lo - price) / price * 100).toFixed(2)}% BELOW GP`;
    });

    const fibHasData = $derived<boolean>(
        fibGpTop != null && fibGpBottom != null,
    );

    function confidenceOf(key: string): number {
        return confPct(tf.indicators?.[key]?.confidence ?? 0);
    }

    function roleClass(role: 'support' | 'resistance' | 'neutral'): string {
        if (role === 'support') return styles.roleSupport ?? '';
        if (role === 'resistance') return styles.roleResistance ?? '';
        return styles.roleNeutral ?? '';
    }

    function fmtPx(n: number | null | undefined): string {
        if (n == null || !isFinite(n) || n <= 0) return '—';
        const px = tf.priceText ? parseFloat(tf.priceText) : 0;
        if (px >= 1000) return `$${n.toFixed(0)}`;
        if (px >= 1) return `$${n.toFixed(2)}`;
        return `$${n.toFixed(4)}`;
    }

    function swingClass(s: 'BULL' | 'BEAR' | 'NEUTRAL'): string {
        if (s === 'BULL') return styles.bullBadge ?? '';
        if (s === 'BEAR') return styles.bearBadge ?? '';
        return styles.neutralBadge ?? '';
    }
</script>

<div class={styles.view}>
    <!-- ── Fibonacci Ladder (always shown when fib data is present) ── -->
    {#if fibHasData}
        <section class={styles.fibSection}>
            <header class={styles.fibHeader}>
                <span class={styles.fibTitle}>FIBONACCI LADDER</span>
                <span class="{styles.fibSwing} {swingClass(fibSwing)}">
                    {fibSwing} SWING
                </span>
                <span class={styles.fibConfidence}>conf {fibConfidence}%</span>
                <span class={styles.fibPosition}>{fibPosition}</span>
            </header>
            <div class={styles.fibLadder}>
                {#each fibCoefficients as coeff (coeff.key)}
                    {@const v = fibVals[coeff.key]}
                    {@const present = typeof v === 'number'}
                    <div class="{styles.fibRow} {coeff.gp ? styles.fibRowGp ?? '' : ''} {present ? '' : styles.fibRowMissing ?? ''}">
                        <span class={styles.fibCoeff}>{coeff.label}</span>
                        <span class={styles.fibPrice}>{present ? fmtPx(v as number) : '—'}</span>
                        {#if coeff.gp}
                            <span class={styles.fibBadge}>GP ZONE</span>
                        {/if}
                    </div>
                {/each}
            </div>
            {#if fibExt1618 || fibExt2618}
                <div class={styles.fibExt}>
                    <span class={styles.fibExtLabel}>EXTENSIONS</span>
                    <span class={styles.fibExtItem}>
                        <span class={styles.fibExtCoeff}>1.618</span>
                        <span class={styles.fibExtPrice}>{fmtPx(fibExt1618)}</span>
                    </span>
                    <span class={styles.fibExtItem}>
                        <span class={styles.fibExtCoeff}>2.618</span>
                        <span class={styles.fibExtPrice}>{fmtPx(fibExt2618)}</span>
                    </span>
                </div>
            {/if}
            <footer class={styles.fibFooter}>
                Timeframe: {formatTimeframeLabel(tf.barDurationSec)} · Updates on each completed candle
            </footer>
        </section>
    {/if}

    <!-- ── Standard LevelTest accordion ── -->
    {#if rows.length === 0 && !fibHasData}
        <div class={styles.placeholder}>
            No active level tests. LevelTest signals fire when price trades
            into a structural level's proximity band (default 0.5% / 0.15% for pivots).
        </div>
    {:else if rows.length === 0}
        <div class={styles.placeholder}>
            No active level-test signals. Fibonacci ladder is shown above.
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
