<script lang="ts">
    import { onMount } from 'svelte';
    import { useAppStore } from '../state.svelte';
    import Icon from '../lib/Icon.svelte';
    import styles from './PairDashboard.module.css';

    const app = useAppStore();
    let { pairKey }: { pairKey: string } = $props();
    const pair = $derived(app.instancesMap[pairKey]);

    // ── Per-pair data ──
    let trades = $state<any[]>([]);
    let totalPnl = $state(0);
    let winRate = $state(0);
    let profitFactor = $state(0);
    let loading = $state(true);

    async function fetchPairData() {
        loading = true;
        try {
            const symbol = pair?.symbol ?? pairKey;
            const [perfRes, tradesRes] = await Promise.all([
                fetch(`/api/paper/performance?symbol=${encodeURIComponent(symbol)}`),
                fetch(`/api/trades?symbol=${encodeURIComponent(symbol)}&limit=10`),
            ]);
            if (perfRes.ok) {
                const data = await perfRes.json();
                totalPnl = data.total_pnl ?? 0;
                winRate = data.win_rate ?? 0;
                profitFactor = data.profit_factor ?? 0;
            }
            if (tradesRes.ok) {
                const data = await tradesRes.json();
                trades = data.trades?.slice(0, 10) ?? [];
            }
        } catch (_) {} finally {
            loading = false;
        }
    }

    onMount(() => { fetchPairData(); });

    // ── Paper position ──
    const hasPosition = $derived((app.paper?.paperDirection ?? '') !== '');
    const posDir = $derived(app.paper?.paperDirection ?? '');
    const posEntry = $derived(app.paper?.paperAvgEntryPrice ?? 0);
    const posSize = $derived(app.paper?.paperPositionPct ?? 0);
    const markPrice = $derived((pair?.microTerm?.latestSnapshot?.current_price as number) ?? 0);
    const unrealizedPnl = $derived(app.paper?.paperUnrealizedPnl ?? 0);
    const unrealizedRoi = $derived(app.paper?.paperUnrealizedRoi ?? 0);

    function fmtUsd(v: number): string {
        return v.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 });
    }
    function fmtSmall(v: number): string {
        return v.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 8 });
    }
    function pnlClass(v: number): string {
        if (v > 0) return styles.pnlUp;
        if (v < 0) return styles.pnlDown;
        return '';
    }
    function pnlSign(v: number): string {
        return v > 0 ? '+' : '';
    }
</script>

<div class={styles.container}>
    <!-- Pair Info Header -->
    <div class={styles.header}>
        <div class={styles.headerLeft}>
            <span class={styles.headerSymbol}>{pair?.symbol ?? pairKey}</span>
            <span class={styles.headerStatus}>
                <span class={pair?.isConnected ? styles.statusLive : styles.statusOffline}>●</span>
                {pair?.isConnected ? 'Connected' : 'Disconnected'}
            </span>
        </div>
        <div class={styles.headerRight}>
            <span class={styles.headerPrice}>
                {markPrice > 0 ? '$' + fmtSmall(markPrice) : '--'}
            </span>
        </div>
    </div>

    <!-- Position Card -->
    <div class={styles.card}>
        <div class={styles.cardTitle}>CURRENT POSITION</div>
        {#if hasPosition}
            <div class={styles.posGrid}>
                <div class={styles.posItem}>
                    <span class={styles.posLabel}>Direction</span>
                    <span class={styles.posValue} class:posLong={posDir === 'LONG'} class:posShort={posDir === 'SHORT'}>{posDir}</span>
                </div>
                <div class={styles.posItem}>
                    <span class={styles.posLabel}>Entry</span>
                    <span class={styles.posValue}>${fmtSmall(posEntry)}</span>
                </div>
                <div class={styles.posItem}>
                    <span class={styles.posLabel}>Size</span>
                    <span class={styles.posValue}>{posSize.toFixed(0)}%</span>
                </div>
                <div class={styles.posItem}>
                    <span class={styles.posLabel}>Mark</span>
                    <span class={styles.posValue}>${fmtSmall(markPrice)}</span>
                </div>
                <div class={styles.posItem}>
                    <span class={styles.posLabel}>Unreal. PnL</span>
                    <span class={styles.posValue + ' ' + pnlClass(unrealizedPnl)}>{pnlSign(unrealizedPnl)}{fmtUsd(unrealizedPnl)}</span>
                </div>
                <div class={styles.posItem}>
                    <span class={styles.posLabel}>ROI</span>
                    <span class={styles.posValue + ' ' + pnlClass(unrealizedRoi)}>{pnlSign(unrealizedRoi)}{unrealizedRoi.toFixed(2)}%</span>
                </div>
            </div>
        {:else}
            <div class={styles.noPos}>○ No active position</div>
        {/if}
    </div>

    <!-- Stats Cards -->
    <div class={styles.statsRow}>
        <div class={styles.statCard}>
            <span class={styles.statLabel}>Total PnL</span>
            <span class={styles.statValue + ' ' + pnlClass(totalPnl)}>{pnlSign(totalPnl)}{fmtUsd(totalPnl)}</span>
        </div>
        <div class={styles.statCard}>
            <span class={styles.statLabel}>Win Rate</span>
            <span class={styles.statValue}>{(winRate * 100).toFixed(1)}%</span>
        </div>
        <div class={styles.statCard}>
            <span class={styles.statLabel}>Profit Factor</span>
            <span class={styles.statValue}>{profitFactor.toFixed(2)}</span>
        </div>
        <div class={styles.statCard}>
            <span class={styles.statLabel}>Total Trades</span>
            <span class={styles.statValue}>{trades.length}</span>
        </div>
    </div>

    <!-- Recent Trades -->
    <div class={styles.card}>
        <div class={styles.cardTitle}>RECENT TRADES</div>
        {#if trades.length > 0}
            <div class={styles.tradesTable}>
                <div class={styles.tradesHeader}>
                    <span>Direction</span>
                    <span>Entry</span>
                    <span>Exit</span>
                    <span>PnL</span>
                    <span>Date</span>
                </div>
                {#each trades as t}
                    <div class={styles.tradeRow}>
                        <span class={t.direction === 'Open Long' || t.direction === 'LONG' ? 'posLong' : t.direction === 'Open Short' || t.direction === 'SHORT' ? 'posShort' : ''}>
                            {t.direction === 'Open Long' ? 'LONG' : t.direction === 'Open Short' ? 'SHORT' : t.direction}
                        </span>
                        <span>{f(t.entry_price)}</span>
                        <span>{f(t.exit_price)}</span>
                        <span class={pnlClass(t.pnl ?? 0)}>{pnlSign(t.pnl ?? 0)}{f(t.pnl ?? 0)}</span>
                        <span class={styles.tradeDate}>{df(t.closed_at)}</span>
                    </div>
                {/each}
            </div>
        {:else}
            <div class={styles.noPos}>{loading ? 'Loading...' : 'No trades yet'}</div>
        {/if}
    </div>
</div>

<script module>
    function f(v: any): string {
        if (v == null || v === '') return '--';
        const n = Number(v);
        if (isNaN(n)) return String(v);
        return n.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 8 });
    }
    function df(v: any): string {
        if (!v) return '--';
        try { return new Date(v).toLocaleDateString('en-US', { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' }); }
        catch { return String(v); }
    }
</script>
