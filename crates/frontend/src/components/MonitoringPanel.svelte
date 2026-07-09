<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { useAppStore } from '../state.svelte';
    import styles from './MonitoringPanel.module.css';

    const app = useAppStore();
    let { pairKey }: { pairKey: string } = $props();
    const pair = $derived(app.instancesMap[pairKey]);

    let data = $state<any>(null);
    let loading = $state(false);
    let error = $state('');
    let timer: ReturnType<typeof setInterval> | null = null;

    async function fetchData() {
        if (loading) return;
        loading = true;
        error = '';
        try {
            const res = await fetch(`/api/monitor/active-trades?symbol=${encodeURIComponent(pairKey)}`);
            if (res.ok) data = await res.json();
            else error = `Server returned ${res.status}`;
        } catch (e: any) {
            error = e?.message || 'Fetch failed';
        }
        loading = false;
    }

    onMount(() => { fetchData(); timer = setInterval(fetchData, 5000); });
    onDestroy(() => { if (timer) clearInterval(timer); });

    function pctBar(score: number, threshold: number): string {
        return `${Math.min((score / Math.max(threshold * 2, 1)) * 100, 100)}%`;
    }
    function barColor(score: number, threshold: number): string {
        if (score >= threshold) return '#ef5350';
        if (score >= threshold * 0.7) return '#ffa726';
        return '#66bb6a';
    }
    function pnlClass(v: number): string {
        if (v > 0) return styles.pnlPos;
        if (v < 0) return styles.pnlNeg;
        return '';
    }
    function fmtUsd(v: number | null | undefined): string {
        if (v == null || isNaN(v)) return '$0.00';
        return '$' + v.toFixed(2);
    }
    function fmtPct(v: number | null | undefined): string {
        if (v == null || isNaN(v)) return '0.00%';
        return (v >= 0 ? '+' : '') + v.toFixed(2) + '%';
    }
</script>

{#if pair}
<div class={styles.panel}>
    <div class={styles.panelHeader}>
        <span class={styles.title}>ACTIVE POSITION MONITOR</span>
        <button class={styles.refreshBtn} onclick={fetchData}>{loading ? '…' : '⟳'}</button>
    </div>

    {#if error}
        <div class={styles.errorBox}>{error}</div>
    {:else if !data}
        <div class={styles.emptyState}>Loading position data...</div>
    {:else if !data.has_active_position}
        <div class={styles.emptyState}>
            <span class={styles.emptyIcon}>○</span>
            <span>No active position</span>
            <span class={styles.emptyHint}>Open a trade to begin monitoring</span>
        </div>
    {:else}
        <!-- Position Summary + Exit Signals (side by side) -->
        <div class={styles.topRow}>
            <!-- Position Summary Card -->
            <div class={styles.summaryCard}>
                <div class={styles.cardTitle}>POSITION SUMMARY</div>
                <div class={styles.summaryGrid}>
                    <div class={styles.summaryItem}>
                        <span class={styles.summaryLabel}>Direction</span>
                        <span class="{styles.summaryValue} {data.direction === 'LONG' ? styles.directionLong : styles.directionShort}">
                            {data.direction}
                        </span>
                    </div>
                    <div class={styles.summaryItem}>
                        <span class={styles.summaryLabel}>Avg Entry</span>
                        <span class={styles.summaryValue}>{fmtUsd(data.average_entry_price)}</span>
                    </div>
                    <div class={styles.summaryItem}>
                        <span class={styles.summaryLabel}>Mark Price</span>
                        <span class={styles.summaryValue}>{fmtUsd(pair?.microTerm?.priceText ? parseFloat(String(pair.microTerm.priceText)) : null)}</span>
                    </div>
                    <div class={styles.summaryItem}>
                        <span class={styles.summaryLabel}>Total Size</span>
                        <span class={styles.summaryValue}>{data.total_size.toFixed(6)}</span>
                    </div>
                    <div class={styles.summaryItem}>
                        <span class={styles.summaryLabel}>Unrealized PnL</span>
                        <span class="{styles.summaryValue} {pnlClass(data.unrealized_pnl)}">{fmtUsd(data.unrealized_pnl)}</span>
                    </div>
                    <div class={styles.summaryItem}>
                        <span class={styles.summaryLabel}>ROI</span>
                        <span class="{styles.summaryValue} {pnlClass(data.unrealized_roi_pct)}">{fmtPct(data.unrealized_roi_pct)}</span>
                    </div>
                </div>
                <div class={styles.accountRow}>
                    <div class={styles.acctItem}>
                        <span>Margin Used</span>
                        <span>{fmtUsd(data.margin_used)}</span>
                    </div>
                    <div class={styles.acctItem}>
                        <span>Account Value</span>
                        <span>{fmtUsd(data.account_value)}</span>
                    </div>
                </div>
            </div>

            <!-- Exit Signal Card -->
            <div class={styles.exitCard}>
                <div class={styles.cardTitle}>EXIT SIGNALS</div>
                {#if data.exit_signals}
                    {@const es = data.exit_signals}
                    {@const thresh = es.opposite_exit_threshold}
                    {@const score = data.direction === 'LONG' ? es.opposite_score_short : es.opposite_score_long}
                    <div class={styles.exitSection}>
                        <span class={styles.exitLabel}>Opposite Score ({data.direction === 'LONG' ? 'SHORT' : 'LONG'} bias)</span>
                        <div class={styles.exitBarWrap}>
                            <div class={styles.exitBarTrack}>
                                <div class={styles.exitBarFill} style="width:{pctBar(score, thresh)};background:{barColor(score, thresh)}"></div>
                            </div>
                            <span class={styles.exitScore} style="color:{barColor(score, thresh)}">{score}/{thresh}</span>
                        </div>
                        {#if score >= thresh}
                            <span class={styles.exitWarning}>⚠ Exit signal triggered</span>
                        {:else if score >= thresh * 0.7}
                            <span class={styles.exitCaution}>⚡ Approaching threshold</span>
                        {:else}
                            <span class={styles.exitSafe}>✓ Safe</span>
                        {/if}
                    </div>
                    <div class={styles.exitSection}>
                        <span class={styles.exitLabel}>Invalidation Level</span>
                        <span class={styles.exitValue}>
                            {es.invalidation_level != null ? fmtUsd(es.invalidation_level) : 'Not set'}
                        </span>
                    </div>
                {/if}
            </div>
        </div>

        <!-- Slot Details -->
        {#if data.slots?.length > 0}
            <div class={styles.slotsCard}>
                <div class={styles.cardTitle}>SLOT DETAILS</div>
                <div class={styles.slotsTable}>
                    <div class={styles.slotsHeader}>
                        <span>Slot</span>
                        <span>Entry</span>
                        <span>Size</span>
                        <span>PnL</span>
                        <span>TPs</span>
                    </div>
                    {#each data.slots as slot, i}
                        {@const p = pair?.microTerm?.priceText ? parseFloat(String(pair.microTerm.priceText)) : 0}
                        <div class={styles.slotRow}>
                            <span class={styles.slotName}>#{i + 1}</span>
                            <span>{fmtUsd(slot.entry_price)}</span>
                            <span>{slot.size.toFixed(5)}</span>
                            <span class={pnlClass(slot.unrealized_pnl)}>{fmtUsd(slot.unrealized_pnl)}</span>
                            <span class={styles.slotTps}>
                                {#if slot.take_profit_prices?.length}
                                    {slot.take_profit_prices.map((tp: number) => fmtUsd(tp)).join(' / ')}
                                {:else}
                                    —
                                {/if}
                            </span>
                        </div>
                    {/each}
                </div>
            </div>
        {/if}

        <!-- Bottom Row: Trailing Stop + Safety State -->
        <div class={styles.bottomRow}>
            <!-- Trailing Stop Card -->
            <div class={styles.trailCard}>
                <div class={styles.cardTitle}>TRAILING STOP</div>
                {#if data.break_even_trail}
                    {@const bt = data.break_even_trail}
                    <div class={styles.trailStatus}>
                        <span class={bt.enabled ? styles.trailActive : styles.trailInactive}>
                            {bt.enabled ? '● Active' : '○ Inactive'}
                        </span>
                        {#if bt.enabled && bt.trail_price}
                            <span class={styles.trailPrice}>Trail: {fmtUsd(bt.trail_price)}</span>
                        {/if}
                    </div>
                {/if}
            </div>

            <!-- Safety State Card -->
            <div class={styles.safetyCard}>
                <div class={styles.cardTitle}>SAFETY STATE</div>
                {#if data.safety_state}
                    {@const ss = data.safety_state}
                    <div class={styles.safetyRow}>
                        <div class={styles.safetyItem}>
                            <span class={styles.safetyLabel}>Consecutive Losses</span>
                            <span class="{styles.safetyValue} {ss.consecutive_losses >= ss.suspend_threshold ? styles.safetyDanger : ss.consecutive_losses >= ss.caution_threshold ? styles.safetyWarning : ''}">
                                {ss.consecutive_losses}
                            </span>
                        </div>
                        <div class={styles.safetyItem}>
                            <span class={styles.safetyLabel}>Caution @</span>
                            <span class={styles.safetyValueMuted}>{ss.caution_threshold}</span>
                        </div>
                        <div class={styles.safetyItem}>
                            <span class={styles.safetyLabel}>Suspend @</span>
                            <span class={styles.safetyValueMuted}>{ss.suspend_threshold}</span>
                        </div>
                    </div>
                {/if}
            </div>
        </div>
    {/if}
</div>
{/if}
