<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import styles from './RiskOverview.module.css';

    const app = useAppStore();

    const allInstances = $derived(Object.keys(app.instancesMap));
    const totalInstances = $derived(allInstances.length);
    const activeCount = $derived(
        Object.values(app.instancesMap).filter((inst) => inst.isConnected).length,
    );

    const perInstanceCapital = $derived(
        totalInstances > 0
            ? app.sessionCapital / totalInstances
            : app.sessionCapital,
    );

    const perInstancePct = $derived(
        totalInstances > 0 ? 100 / totalInstances : 0,
    );

    const marginUsed = $derived(app.paper?.paperMarginUsed ?? 0);
    const utilizationPct = $derived(
        app.sessionCapital > 0 ? (marginUsed / app.sessionCapital) * 100 : 0,
    );
    const availableCapital = $derived(Math.max(0, app.sessionCapital - marginUsed));

    function fmtUsd(v: number): string {
        return v.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 });
    }

    function utilizationColor(pct: number): string {
        if (pct >= 90) return '#ef5350';
        if (pct >= 70) return '#ffa726';
        if (pct >= 40) return '#64ffda';
        return '#10b981';
    }
</script>

<div class={styles.container}>
    <!-- Hero section -->
    <div class={styles.hero}>
        <div class={styles.heroTitle}>PORTFOLIO CAPITAL</div>
        <div class={styles.heroAmount}>
            {app.sessionCurrency} {fmtUsd(app.sessionCapital)}
        </div>
        <div class={styles.heroMode}>
            {app.sessionMode?.toUpperCase()} · {app.sessionExchange}
        </div>
    </div>

    <!-- Stat cards -->
    <div class={styles.statGrid}>
        <div class={styles.statCard}>
            <span class={styles.statLabel}>Total Instances</span>
            <span class={styles.statValue}>{totalInstances}</span>
            <span class={styles.statHint}>{activeCount} active</span>
        </div>
        <div class={styles.statCard}>
            <span class={styles.statLabel}>Per-Instance Max</span>
            <span class={styles.statValue}>{app.sessionCurrency} {fmtUsd(perInstanceCapital)}</span>
            <span class={styles.statHint}>{perInstancePct.toFixed(0)}% of portfolio</span>
        </div>
        <div class={styles.statCard}>
            <span class={styles.statLabel}>Margin Used</span>
            <span class={styles.statValue} style="color:{utilizationColor(utilizationPct)}">
                {app.sessionCurrency} {fmtUsd(marginUsed)}
            </span>
            <span class={styles.statHint}>{utilizationPct.toFixed(1)}% utilized</span>
        </div>
        <div class={styles.statCard}>
            <span class={styles.statLabel}>Available</span>
            <span class={styles.statValue} style="color:{availableCapital > 0 ? '#10b981' : '#ef5350'}">
                {app.sessionCurrency} {fmtUsd(availableCapital)}
            </span>
            <span class={styles.statHint}>{app.sessionCapital > 0 ? ((availableCapital / app.sessionCapital) * 100).toFixed(0) : 0}% free</span>
        </div>
    </div>

    <!-- Instance allocations -->
    <div class={styles.section}>
        <div class={styles.sectionTitle}>INSTANCE ALLOCATIONS</div>
        {#if allInstances.length === 0}
            <div class={styles.emptyState}>No instances configured. Add trading pairs from the Instances view.</div>
        {:else}
            <div class={styles.allocList}>
                {#each allInstances as pairKey (pairKey)}
                    {@const inst = app.instancesMap[pairKey]}
                    <div class={styles.allocRow}>
                        <div class={styles.allocInfo}>
                            <span class={styles.allocSymbol}>
                                {inst.symbol}
                                <span class={inst.isConnected ? styles.statusLive : styles.statusOffline}>
                                    {inst.isConnected ? '●' : '○'}
                                </span>
                            </span>
                            <span class={styles.allocLabel}>
                                {perInstancePct.toFixed(1)}% allocation
                            </span>
                        </div>
                        <div class={styles.allocBarWrap}>
                            <div class={styles.allocBarTrack}>
                                <div
                                    class={styles.allocBarFill}
                                    style="width:{perInstancePct}%;background:#64ffda"
                                ></div>
                            </div>
                            <span class={styles.allocBarLabel}>
                                {perInstancePct.toFixed(0)}%
                            </span>
                        </div>
                        <div class={styles.allocAmounts}>
                            <span class={styles.allocUsed}>
                                {app.sessionCurrency} {fmtUsd(perInstanceCapital)}
                            </span>
                        </div>
                    </div>
                {/each}
            </div>
        {/if}
    </div>

    <!-- Configuration section -->
    <div class={styles.section}>
        <div class={styles.sectionTitle}>MAX MARGIN PER INSTANCE</div>
        <div class={styles.configRow}>
            <span class={styles.configDesc}>
                Each of the {totalInstances || 0} instance{totalInstances !== 1 ? 's' : ''} receives an equal {perInstancePct.toFixed(1)}% share
                of the {app.sessionCurrency} {fmtUsd(app.sessionCapital)} total portfolio.
                This is the maximum margin available for that pair's positions.
            </span>
            <div class={styles.configFields}>
                <div class={styles.configItem}>
                    <span class={styles.configLabel}>Total Portfolio</span>
                    <span class={styles.configVal}>{app.sessionCurrency} {fmtUsd(app.sessionCapital)}</span>
                </div>
                <div class={styles.configItem}>
                    <span class={styles.configLabel}>Active Instances</span>
                    <span class={styles.configVal}>{totalInstances}</span>
                </div>
                <div class={styles.configItem}>
                    <span class={styles.configLabel}>Per-Instance Max</span>
                    <span class={styles.configVal}>{app.sessionCurrency} {fmtUsd(perInstanceCapital)}</span>
                </div>
            </div>
        </div>
    </div>
</div>
