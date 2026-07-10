<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { useAppStore } from '../state.svelte';
    import type { InstanceSummary } from '../types';
    import styles from './GeneralDashboard.module.css';

    const app = useAppStore();
    let instances = $state<InstanceSummary[]>([]);
    let loading = $state(true);

    let utcTime = $state('');
    let clockInterval: ReturnType<typeof setInterval>;

    function updateUtcClock() {
        const now = new Date();
        utcTime = now.toISOString().replace('T', ' ').slice(0, 19) + ' UTC';
    }

    interface ConfluenceData {
        bullish: number;
        bearish: number;
        total: number;
        avgRsi: number;
        strongTrends: number;
        dominantRegime: string;
        overallConviction: number;
    }

    interface PairSnapshot {
        pair: string;
        symbol: string;
        price: string;
        rsi: number | null;
        adx: number | null;
        regime: string;
        trend: string;
        overall: string;
    }

    let confluence = $derived.by<ConfluenceData | null>(() => {
        const pairs = Object.keys(app.instancesMap);
        const snapshots: Record<string, unknown>[] = [];
        for (const key of pairs) {
            const snap = app.instancesMap[key].fastTerm.latestSnapshot;
            if (snap) snapshots.push(snap);
        }
        if (snapshots.length === 0) return null;

        let bullish = 0;
        let bearish = 0;
        let rsiSum = 0;
        let rsiCount = 0;
        let strongTrends = 0;
        const regimeCounts: Record<string, number> = {};
        let convictionSum = 0;
        let convictionCount = 0;

        for (const s of snapshots) {
            const indicators = s.indicators as Record<string, any> | undefined;
            const ctx = s.context as Record<string, any> | undefined;

            if (ctx?.trend?.score !== undefined) {
                if (ctx.trend.score > 0) bullish++;
                else if (ctx.trend.score < 0) bearish++;
            }

            if (indicators?.rsi?.raw_value !== undefined) {
                rsiSum += indicators.rsi.raw_value;
                rsiCount++;
            }

            if (indicators?.adx?.values?.adx !== undefined) {
                if (indicators.adx.values.adx > 25) strongTrends++;
            }

            if (ctx?.regime) {
                const r = ctx.regime as string;
                regimeCounts[r] = (regimeCounts[r] || 0) + 1;
            }

            if (ctx?.overall_score !== undefined) {
                convictionSum += ctx.overall_score;
                convictionCount++;
            }
        }

        let dominantRegime = '';
        let maxCount = 0;
        for (const [r, c] of Object.entries(regimeCounts)) {
            if (c > maxCount) { dominantRegime = r; maxCount = c; }
        }

        return {
            bullish,
            bearish,
            total: snapshots.length,
            avgRsi: rsiCount > 0 ? rsiSum / rsiCount : 0,
            strongTrends,
            dominantRegime,
            overallConviction: convictionCount > 0 ? convictionSum / convictionCount : 0,
        };
    });

    let pairSnapshots = $derived.by<PairSnapshot[]>(() => {
        const pairs = Object.keys(app.instancesMap);
        const result: PairSnapshot[] = [];
        for (const key of pairs) {
            const snap = app.instancesMap[key].fastTerm.latestSnapshot;
            if (!snap) continue;
            const indicators = snap.indicators as Record<string, any> | undefined;
            const ctx = snap.context as Record<string, any> | undefined;

            result.push({
                pair: key,
                symbol: app.instancesMap[key].symbol,
                price: snap.mid_price != null ? String(snap.mid_price) : '--',
                rsi: indicators?.rsi?.raw_value ?? null,
                adx: indicators?.adx?.values?.adx ?? null,
                regime: ctx?.regime ?? '--',
                trend: ctx?.trend?.label ?? '--',
                overall: ctx?.overall_label ?? '--',
            });
        }
        return result;
    });

    async function fetchInstances() {
        try {
            const res = await fetch('/api/instances');
            if (res.ok) {
                const data = await res.json();
                instances = data.instances || [];
            }
        } catch (e) {
            console.error('Error fetching instances:', e);
        } finally {
            loading = false;
        }
    }

    let pollInterval: ReturnType<typeof setInterval>;

    onMount(() => {
        fetchInstances();
        pollInterval = setInterval(fetchInstances, 5000);
        updateUtcClock();
        clockInterval = setInterval(updateUtcClock, 1000);
    });

    onDestroy(() => {
        clearInterval(pollInterval);
        clearInterval(clockInterval);
    });

    function navigateToInstances() {
        app.currentGlobalView = 'instances';
    }

    function regimeClass(regime: string): string {
        switch (regime) {
            case 'TRENDING': return styles.regimeTrending;
            case 'RANGE': return styles.regimeRange;
            case 'EXPANSION': return styles.regimeExpansion;
            case 'COMPRESSION': return styles.regimeCompression;
            default: return '';
        }
    }

    function trendClass(label: string): string {
        if (label.includes('BULL')) return styles.bullish;
        if (label.includes('BEAR')) return styles.bearish;
        return '';
    }

    function overallClass(label: string): string {
        if (label.includes('BULL')) return styles.bullish;
        if (label.includes('BEAR')) return styles.bearish;
        return '';
    }

    function rsiClass(rsi: number | null): string {
        if (rsi === null) return '';
        if (rsi > 70) return styles.rsiOverbought;
        if (rsi < 30) return styles.rsiOversold;
        return '';
    }

    function formatPrice(p: string): string {
        const n = parseFloat(p);
        if (isNaN(n)) return '--';
        if (n >= 1000) return n.toLocaleString(undefined, { maximumFractionDigits: 2 });
        if (n >= 1) return n.toFixed(2);
        if (n >= 0.01) return n.toFixed(4);
        if (n >= 0.0001) return n.toFixed(6);
        return n.toFixed(8);
    }

    function formatRsi(r: number | null): string {
        if (r === null) return '--';
        return r.toFixed(1);
    }

    function formatAdx(a: number | null): string {
        if (a === null) return '--';
        return a.toFixed(1);
    }

    function formatConviction(s: number): string {
        const pct = Math.round(s);
        return pct >= 0 ? `+${pct}` : `${pct}`;
    }
</script>

<div class={styles.dashboardView}>
    <div class={styles.headerRow}>
        <h2 class={styles.dashboardTitle}>Market Dashboard</h2>
        <div class={styles.utcClock}>{utcTime}</div>
    </div>

    {#if loading}
        <div class={styles.loadingRow}>Loading...</div>
    {:else if instances.length === 0}
        <div class={styles.emptyState}>
            <div class={styles.emptyIcon}>📊</div>
            <h3 class={styles.emptyTitle}>No trading pairs configured</h3>
            <p class={styles.emptyText}>
                Create instances from the Instances tab to start monitoring markets in real-time.
            </p>
            <button class={styles.emptyBtn} onclick={navigateToInstances}>
                Go to Instances
            </button>
        </div>
    {:else}
        <!-- Market Confluence Bar -->
        {#if confluence}
            <div class={styles.confluenceBar}>
                <div class={styles.confluenceCard}>
                    <span class={styles.confluenceLabel}>Trend Breadth</span>
                    <span class={styles.confluenceValue}>
                        <span class={styles.bullish}>{confluence.bullish}</span>
                        <span class={styles.confluenceSep}> / </span>
                        <span class={styles.bearish}>{confluence.bearish}</span>
                        <span class={styles.confluenceSep}> / </span>
                        <span class={styles.confluenceDim}>{confluence.total}</span>
                    </span>
                    <span class={styles.confluenceSub}>Bullish / Bearish / Total</span>
                </div>
                <div class={styles.confluenceCard}>
                    <span class={styles.confluenceLabel}>Avg RSI</span>
                    <span class={styles.confluenceValue} class:rsiOverbought={confluence.avgRsi > 70} class:rsiOversold={confluence.avgRsi < 30}>
                        {confluence.avgRsi.toFixed(1)}
                    </span>
                    <span class={styles.confluenceSub}>Across {confluence.total} pairs</span>
                </div>
                <div class={styles.confluenceCard}>
                    <span class={styles.confluenceLabel}>Strong Trends</span>
                    <span class={styles.confluenceValue}>
                        {confluence.strongTrends} / {confluence.total}
                    </span>
                    <span class={styles.confluenceSub}>ADX &gt; 25</span>
                </div>
                <div class={styles.confluenceCard}>
                    <span class={styles.confluenceLabel}>Dominant Regime</span>
                    <span class={styles.confluenceValue}>
                        {confluence.dominantRegime || '--'}
                    </span>
                    <span class={styles.confluenceSub}>Most common regime</span>
                </div>
                <div class={styles.confluenceCard}>
                    <span class={styles.confluenceLabel}>Conviction</span>
                    <span class="{styles.confluenceValue} {confluence.overallConviction > 0 ? styles.bullish : ''} {confluence.overallConviction < 0 ? styles.bearish : ''}">
                        {formatConviction(confluence.overallConviction)}
                    </span>
                    <span class={styles.confluenceSub}>Avg overall score</span>
                </div>
            </div>
        {/if}

        <!-- Pair Confluence Table -->
        {#if pairSnapshots.length > 0 && confluence}
            <div class={styles.sectionHeader}>
                <h3>Pair Overview</h3>
            </div>
            <div class={styles.confluenceTableWrapper}>
                <table class={styles.confluenceTable}>
                    <thead>
                        <tr>
                            <th>Pair</th>
                            <th>Price</th>
                            <th>RSI</th>
                            <th>ADX</th>
                            <th>Regime</th>
                            <th>Trend</th>
                            <th>Overall</th>
                        </tr>
                    </thead>
                    <tbody>
                        {#each pairSnapshots as ps}
                            <tr>
                                <td class={styles.colPair}>{ps.symbol}</td>
                                <td class={styles.colMono}>{formatPrice(ps.price)}</td>
                                <td class="{styles.colMono} {rsiClass(ps.rsi)}">{formatRsi(ps.rsi)}</td>
                                <td class="{styles.colMono} {ps.adx !== null && ps.adx > 25 ? styles.adxStrong : ''}">{formatAdx(ps.adx)}</td>
                                <td><span class="{styles.regimeBadge} {regimeClass(ps.regime)}">{ps.regime}</span></td>
                                <td class="{trendClass(ps.trend)}">{ps.trend}</td>
                                <td class="{overallClass(ps.overall)}">{ps.overall}</td>
                            </tr>
                        {/each}
                    </tbody>
                </table>
            </div>
        {:else if instances.length > 0}
            <div class={styles.sectionHeader}>
                <p class={styles.noData}>Waiting for market data...</p>
            </div>
        {/if}

        <!-- Active Instances -->
        <div class={styles.sectionHeader}>
            <h3>Active Instances</h3>
        </div>
        <div class={styles.instancesTableWrapper}>
            <table class={styles.instancesTable}>
                <thead>
                    <tr>
                        <th>Pair</th>
                        <th>Status</th>
                        <th>Initial Capital</th>
                        <th>Equity</th>
                        <th>Consec. Losses</th>
                        <th>Caution</th>
                    </tr>
                </thead>
                <tbody>
                    {#each instances as inst}
                        <tr>
                            <td class={styles.colPair}>{inst.symbol}</td>
                            <td>
                                <span class="{styles.statusBadge} {inst.status === 'running' ? styles.statusRunning : ''} {inst.status === 'paused' ? styles.statusPaused : ''} {inst.status === 'stopped' ? styles.statusStopped : ''}">
                                    {inst.status}
                                </span>
                            </td>
                            <td class={styles.colMono}>${inst.initial_capital.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}</td>
                            <td class="{styles.colMono} {inst.current_equity >= 0 ? styles.positive : ''} {inst.current_equity < 0 ? styles.negative : ''}">${inst.current_equity.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}</td>
                            <td class={styles.colMono}>{inst.consecutive_losses}</td>
                            <td>
                                <span class="{styles.cautionBadge} {inst.caution_level === 'normal' ? styles.cautionNormal : ''} {inst.caution_level === 'cautious' ? styles.cautionCautious : ''} {(inst.caution_level === 'suspended' || inst.caution_level === 'drawdown_stop') ? styles.cautionWarn : ''}">
                                    {inst.caution_level}
                                </span>
                            </td>
                        </tr>
                    {/each}
                </tbody>
            </table>
        </div>
    {/if}
</div>
