<script lang="ts">
    import styles from './ConfluenceHero.module.css';
    import type { MonitorTimeframe, ContributionDto, PositionState } from '../../types';

    interface Props {
        tf: MonitorTimeframe | null | undefined;
        /** Held position, to select the relevant opposite-signal exit gauge. */
        position?: PositionState;
        topN?: number;
    }
    let { tf, position = 'None', topN = 5 }: Props = $props();

    function scoreColor(score: number): string {
        const mag = Math.min(Math.abs(score) / 100, 1);
        if (mag >= 0.9) return '#a855f7';
        if (score > 5) return `rgb(16, ${Math.round(120 + 135 * mag)}, 129)`;
        if (score < -5) return `rgb(${Math.round(180 + 59 * mag)}, 68, 68)`;
        return '#94a3b8';
    }

    const score = $derived(tf?.confluence_score ?? 0);
    const fillWidth = $derived(`${Math.min(Math.abs(score), 100) / 2}%`);
    const fillLeft = $derived(score >= 0 ? '50%' : `${50 - Math.min(Math.abs(score), 100) / 2}%`);

    // Contributions are signed in the bull-bias frame: positive → bullish push,
    // negative → bearish push. Split into drivers and opposers.
    const sorted = $derived<ContributionDto[]>([...(tf?.contributions ?? [])].sort((a, b) => b.contribution - a.contribution));
    const drivers = $derived(sorted.filter((c) => c.contribution > 0.001).slice(0, topN));
    const opposers = $derived(sorted.filter((c) => c.contribution < -0.001).reverse().slice(0, topN));
    const maxAbs = $derived(Math.max(0.0001, ...sorted.map((c) => Math.abs(c.contribution))));

    function barPct(v: number): string {
        return `${Math.min(Math.abs(v) / maxAbs, 1) * 100}%`;
    }

    // Opposite-signal exit: pick the score for the held direction; the actual
    // automated risk trigger fires at the threshold (default 60).
    const oppScore = $derived(
        position === 'Long' ? (tf?.opposite_score_long ?? 0)
        : position === 'Short' ? (tf?.opposite_score_short ?? 0)
        : 0,
    );
    const oppThreshold = $derived(tf?.opposite_exit_threshold ?? 60);
    const oppPct = $derived(Math.min((oppScore / Math.max(oppThreshold, 1)) * 100, 130));
    const oppBreached = $derived(oppScore >= oppThreshold);
</script>

<div class={styles.hero}>
    <div class={styles.head}>
        <span class={styles.title}>CONFLUENCE</span>
        <span class={styles.regime}>{tf?.regime ?? '—'}</span>
        <span class={styles.gate} title="Regime gate (choppiness × adx)">gate ×{(tf?.regime_gate ?? 1).toFixed(2)}</span>
        <span class={styles.weight} title="Active directional weight">w {(tf?.active_weight ?? 0).toFixed(1)}</span>
    </div>

    <div class={styles.dialRow}>
        <span class={styles.dialScore} style="color:{scoreColor(score)}">{score > 0 ? '+' : ''}{score}</span>
        <div class={styles.dialTrack}>
            <div class={styles.dialZero}></div>
            <div class={styles.dialFill} style="left:{fillLeft};width:{fillWidth};background:{scoreColor(score)}"></div>
        </div>
        <span class={styles.dialScale}>±100</span>
    </div>

    <div class={styles.drivers}>
        <div class={styles.col}>
            <div class={styles.colHead} style="color:#10b981">TOP DRIVERS</div>
            {#each drivers as c (c.key)}
                <div class={styles.driverRow} title="{c.display_name}: {c.contribution.toFixed(3)}">
                    <span class={styles.driverName}>{c.display_name}</span>
                    <span class={styles.driverBarWrap}>
                        <span class={styles.driverBar} style="width:{barPct(c.contribution)};background:#10b981"></span>
                    </span>
                    <span class={styles.driverVal} style="color:#10b981">+{c.contribution.toFixed(2)}</span>
                </div>
            {:else}
                <div class={styles.empty}>none</div>
            {/each}
        </div>
        <div class={styles.col}>
            <div class={styles.colHead} style="color:#ef4444">TOP OPPOSERS</div>
            {#each opposers as c (c.key)}
                <div class={styles.driverRow} title="{c.display_name}: {c.contribution.toFixed(3)}">
                    <span class={styles.driverName}>{c.display_name}</span>
                    <span class={styles.driverBarWrap}>
                        <span class={styles.driverBar} style="width:{barPct(c.contribution)};background:#ef4444"></span>
                    </span>
                    <span class={styles.driverVal} style="color:#ef4444">{c.contribution.toFixed(2)}</span>
                </div>
            {:else}
                <div class={styles.empty}>none</div>
            {/each}
        </div>
    </div>

    {#if position !== 'None'}
        <div class={styles.oppRow} data-breached={oppBreached}>
            <span class={styles.oppLabel}>OPPOSITE-EXIT ({position})</span>
            <div class={styles.oppTrack}>
                <div class={styles.oppFill} style="width:{Math.min(oppPct, 100)}%"></div>
                <div class={styles.oppThresholdMark} style="left:{(oppThreshold / Math.max(oppThreshold, 1)) * 100 / 1.3}%"></div>
            </div>
            <span class={styles.oppVal} style="color:{oppBreached ? '#ef4444' : '#94a3b8'}">
                {oppScore} / {oppThreshold}
            </span>
        </div>
        {#if oppBreached}<div class={styles.oppWarn}>⚠ OPPOSITE-SIGNAL EXIT THRESHOLD BREACHED</div>{/if}
    {/if}
</div>
