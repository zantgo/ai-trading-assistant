<script lang="ts">
    import { onMount } from 'svelte';
    import { useAppStore } from '../state.svelte';
    import type { RiskObjectDto } from '../types';
    import styles from './RiskProfilePanel.module.css';

    const app = useAppStore();
    let { pair }: { pair: { symbol: string } } = $props();

    onMount(() => {
        if (!app.riskProfile) app.fetchRiskProfile();
    });

    const profile = $derived(app.riskProfile?.profile ?? null);
    const rrHistory = $derived(app.riskProfile?.rr_history ?? []);

    function pct(x: number): string {
        return `${(x * 100).toFixed(0)}%`;
    }

    function riskClass(score: number): string {
        if (score >= 0.75) return styles.sevCritical;
        if (score >= 0.6) return styles.sevHigh;
        if (score >= 0.45) return styles.sevElevated;
        if (score >= 0.3) return styles.sevNormal;
        if (score >= 0.15) return styles.sevSafe;
        return styles.sevVerySafe;
    }

    function permClass(perm: string): string {
        switch (perm) {
            case 'Allowed': return styles.permAllowed;
            case 'High Caution': return styles.permCaution;
            case 'Restricted': return styles.permRestricted;
            default: return styles.permBlocked;
        }
    }

    const categories = $derived(
        profile
            ? ([
                  ['Market', profile.market],
                  ['Structural', profile.structural],
                  ['Momentum', profile.momentum],
                  ['Volatility', profile.volatility],
                  ['Liquidity', profile.liquidity],
                  ['Behavioral', profile.behavioral],
              ] as [string, RiskObjectDto][])
            : []
    );
</script>

<div class={styles.riskPanel}>
    <div class={styles.header}>
        <h2 class={styles.title}>Institutional Risk Management — {pair.symbol}</h2>
        <button class={styles.refreshBtn} onclick={() => app.fetchRiskProfile()} disabled={app.riskProfileLoading}>
            {app.riskProfileLoading ? 'Loading…' : 'Refresh'}
        </button>
    </div>

    {#if app.riskProfileError}
        <div class={styles.errorBox}>{app.riskProfileError}</div>
    {:else if !app.riskProfile}
        <div class={styles.infoBox}>Loading risk profile…</div>
    {:else if !app.riskProfile.available || !profile}
        <div class={styles.infoBox}>
            {app.riskProfile.message ?? 'Risk profile is not available yet.'}
        </div>

        {#if rrHistory.length > 0}
            <div class={styles.rrCard}>
                <h3 class={styles.sectionTitle}>Adaptive Reward/Risk — Block History</h3>
                <table class={styles.rrTable}>
                    <thead>
                        <tr><th>Block</th><th>W</th><th>L</th><th>Win%</th><th>Breakeven</th><th>Recommended</th><th>Net</th></tr>
                    </thead>
                    <tbody>
                        {#each rrHistory as b}
                            <tr>
                                <td>#{b.block_index}</td>
                                <td>{b.wins}</td>
                                <td>{b.losses}</td>
                                <td>{pct(b.win_rate_estimate)}</td>
                                <td>1 : {b.breakeven_ratio.toFixed(2)}</td>
                                <td class={styles.mono}>1 : {b.recommended_ratio.toFixed(2)}</td>
                                <td class={b.net_block_pnl >= 0 ? styles.pos : styles.neg}>{b.net_block_pnl.toFixed(2)}</td>
                            </tr>
                        {/each}
                    </tbody>
                </table>
            </div>
        {/if}
    {:else}
        <!-- Overall summary -->
        <div class={styles.summaryRow}>
            <div class={styles.overallCard}>
                <span class={styles.overallLabel}>Overall Risk</span>
                <span class="{styles.overallScore} {riskClass(profile.overall_risk)}">
                    {profile.overall_risk.toFixed(2)}
                </span>
                <span class={styles.overallSub}>{profile.overall_level} · conf {pct(profile.overall_confidence)}</span>
                <div class={styles.bar}>
                    <div class="{styles.barFill} {riskClass(profile.overall_risk)}" style="width: {profile.overall_risk * 100}%"></div>
                </div>
            </div>

            <div class={styles.statCard}>
                <span class={styles.statLabel}>Trade Permission</span>
                <span class="{styles.permBadge} {permClass(profile.permission)}">{profile.permission}</span>
                <span class={styles.statSub}>Drawdown: {profile.drawdown_state}</span>
            </div>

            <div class={styles.statCard}>
                <span class={styles.statLabel}>Exposure Tier</span>
                <span class={styles.statValue}>{profile.exposure}</span>
                <span class={styles.statSub}>Alloc: {profile.recommended_allocation_pct.toFixed(2)}%</span>
            </div>

            <div class={styles.statCard}>
                <span class={styles.statLabel}>Opportunity vs Risk</span>
                <span class={styles.statValue}>{pct(profile.opportunity_score)} / {pct(profile.overall_risk)}</span>
                <span class={styles.statSub}>
                    {profile.opportunity_score > profile.overall_risk ? 'Opportunity favored' : 'Risk favored'}
                </span>
            </div>
        </div>

        <!-- Adaptive Reward/Risk -->
        <div class={styles.rrCard}>
            <h3 class={styles.sectionTitle}>Adaptive Reward/Risk Recommendation</h3>
            <div class={styles.rrGrid}>
                <div class={styles.rrMetric}>
                    <span class={styles.rrLabel}>Win Rate (Beta-smoothed)</span>
                    <span class={styles.rrValue}>{pct(profile.reward_risk.win_rate_estimate)}</span>
                </div>
                <div class={styles.rrMetric}>
                    <span class={styles.rrLabel}>Breakeven Ratio</span>
                    <span class={styles.rrValue}>1 : {profile.reward_risk.breakeven_ratio.toFixed(2)}</span>
                </div>
                <div class={styles.rrMetric}>
                    <span class={styles.rrLabel}>Recommended Ratio</span>
                    <span class="{styles.rrValue} {styles.rrHighlight}">1 : {profile.reward_risk.recommended_ratio.toFixed(2)}</span>
                </div>
                <div class={styles.rrMetric}>
                    <span class={styles.rrLabel}>Confidence</span>
                    <span class={styles.rrValue}>{pct(profile.reward_risk.confidence)}</span>
                </div>
                <div class={styles.rrMetric}>
                    <span class={styles.rrLabel}>Sample Size</span>
                    <span class={styles.rrValue}>{profile.reward_risk.sample_size} trades</span>
                </div>
            </div>
            <p class={styles.rrNote}>
                Recommendation adapts to realized performance: higher win rate → lower required reward/risk;
                lower win rate → higher required reward/risk. Anchored at 50% → 1:1, always targeting positive expectancy.
            </p>
        </div>

        <!-- Category breakdown -->
        <div class={styles.catGrid}>
            {#each categories as [name, obj]}
                <div class={styles.catCard}>
                    <div class={styles.catHead}>
                        <span class={styles.catName}>{name}</span>
                        <span class="{styles.catLevel} {riskClass(obj.score)}">{obj.level}</span>
                    </div>
                    <div class={styles.bar}>
                        <div class="{styles.barFill} {riskClass(obj.score)}" style="width: {obj.score * 100}%"></div>
                    </div>
                    <div class={styles.catMeta}>
                        <span>Score {obj.score.toFixed(2)}</span>
                        <span>Pctl {obj.historical_percentile.toFixed(0)}</span>
                        <span>{obj.trend}</span>
                        <span>Conf {pct(obj.confidence)}</span>
                    </div>
                    <p class={styles.catExpl}>{obj.explanation}</p>
                </div>
            {/each}
        </div>

        <div class={styles.explBox}>{profile.explanation}</div>

        {#if rrHistory.length > 0}
            <div class={styles.rrCard}>
                <h3 class={styles.sectionTitle}>Reward/Risk Block History</h3>
                <table class={styles.rrTable}>
                    <thead>
                        <tr><th>Block</th><th>W</th><th>L</th><th>Win%</th><th>Breakeven</th><th>Recommended</th><th>Net</th></tr>
                    </thead>
                    <tbody>
                        {#each rrHistory as b}
                            <tr>
                                <td>#{b.block_index}</td>
                                <td>{b.wins}</td>
                                <td>{b.losses}</td>
                                <td>{pct(b.win_rate_estimate)}</td>
                                <td>1 : {b.breakeven_ratio.toFixed(2)}</td>
                                <td class={styles.mono}>1 : {b.recommended_ratio.toFixed(2)}</td>
                                <td class={b.net_block_pnl >= 0 ? styles.pos : styles.neg}>{b.net_block_pnl.toFixed(2)}</td>
                            </tr>
                        {/each}
                    </tbody>
                </table>
            </div>
        {/if}
    {/if}
</div>
