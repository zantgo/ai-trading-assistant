<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import styles from './WorkflowState.module.css';

    const app = useAppStore();
    let { pairKey }: { pairKey: string } = $props();
    const pair = $derived(app.instancesMap[pairKey]);

    // ── Portfolio exposure ──
    const allInstances = $derived(Object.keys(app.instancesMap));
    const totalInstances = $derived(allInstances.length);
    const perInstanceCapital = $derived(
        totalInstances > 0 ? app.sessionCapital / totalInstances : app.sessionCapital,
    );
    const marginUsed = $derived(app.paper?.paperMarginUsed ?? 0);
    const utilizationPct = $derived(
        app.sessionCapital > 0 ? (marginUsed / app.sessionCapital) * 100 : 0,
    );
    const availableCapital = $derived(Math.max(0, app.sessionCapital - marginUsed));

    // ── Position state ──
    const paperPos = $derived(app.paper?.activePaperPosition);
    const hasPosition = $derived(!!paperPos && (app.paper?.paperDirection ?? '') !== '');
    const positionDir = $derived(app.paper?.paperDirection ?? '');
    const entryPrice = $derived(app.paper?.paperAvgEntryPrice ?? 0);
    const posSizePct = $derived(app.paper?.paperPositionPct ?? 0);
    const markPrice = $derived(pair?.microTerm?.latestSnapshot?.current_price as number ?? 0);
    const unrealizedPnl = $derived(app.paper?.paperUnrealizedPnl ?? 0);
    const unlockRealizedRoi = $derived(app.paper?.paperUnrealizedRoi ?? 0);

    const stopLoss = $derived(paperPos?.stop_loss as number | undefined ?? null);
    const takeProfit = $derived(paperPos?.take_profit as number | undefined ?? null);

    // ── Formatting ──
    function fmtUsd(v: number): string {
        return v.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 });
    }
    function fmtSmall(v: number): string {
        return v.toLocaleString('en-US', { minimumFractionDigits: 4, maximumFractionDigits: 8 });
    }
    function pnlColor(v: number): string {
        if (v > 0) return '#10b981';
        if (v < 0) return '#ef4444';
        return '#94a3b8';
    }
    function utilizationColor(pct: number): string {
        if (pct >= 90) return '#ef5350';
        if (pct >= 70) return '#ffa726';
        return '#10b981';
    }

    // ── Modified SL/TP state ──
    let showModifySl = $state(false);
    let showModifyTp = $state(false);
    let newSlValue = $state('');
    let newTpValue = $state('');
</script>

<div class={styles.container}>
    <!-- Portfolio Exposure -->
    <div class={styles.section}>
        <div class={styles.sectionTitle}>PORTFOLIO EXPOSURE</div>
        <div class={styles.exposureGrid}>
            <div class={styles.exposureCard}>
                <span class={styles.exposureLabel}>Capital</span>
                <span class={styles.exposureValue}>{app.sessionCurrency} {fmtUsd(app.sessionCapital)}</span>
            </div>
            <div class={styles.exposureCard}>
                <span class={styles.exposureLabel}>Per Instance</span>
                <span class={styles.exposureValue}>{app.sessionCurrency} {fmtUsd(perInstanceCapital)}</span>
            </div>
            <div class={styles.exposureCard}>
                <span class={styles.exposureLabel}>Margin Used</span>
                <span class={styles.exposureValue} style="color:{utilizationColor(utilizationPct)}">{app.sessionCurrency} {fmtUsd(marginUsed)}</span>
            </div>
            <div class={styles.exposureCard}>
                <span class={styles.exposureLabel}>Available</span>
                <span class={styles.exposureValue} style="color:#10b981">{app.sessionCurrency} {fmtUsd(availableCapital)}</span>
            </div>
        </div>
        <div class={styles.utilBar}>
            <div class={styles.utilTrack}>
                <div class={styles.utilFill} style="width:{Math.min(utilizationPct, 100)}%;background:{utilizationColor(utilizationPct)}"></div>
            </div>
            <span class={styles.utilText}>{utilizationPct.toFixed(1)}% utilized</span>
        </div>
    </div>

    <!-- Current Position -->
    <div class={styles.section}>
        <div class={styles.sectionTitle}>CURRENT POSITION — {pair?.symbol ?? pairKey}</div>
        {#if hasPosition && positionDir}
            <div class={styles.positionBox}>
                <div class={styles.posHeader}>
                    <span class={styles.posDir} class:posLong={positionDir === 'LONG'} class:posShort={positionDir === 'SHORT'}>
                        {positionDir}
                    </span>
                    <span class={styles.posSize}>Size: {posSizePct.toFixed(0)}%</span>
                </div>
                <div class={styles.posGrid}>
                    <div class={styles.posItem}>
                        <span class={styles.posLabel}>Entry</span>
                        <span class={styles.posValue}>${fmtSmall(entryPrice)}</span>
                    </div>
                    <div class={styles.posItem}>
                        <span class={styles.posLabel}>Mark</span>
                        <span class={styles.posValue}>${fmtSmall(markPrice)}</span>
                    </div>
                    <div class={styles.posItem}>
                        <span class={styles.posLabel}>PnL</span>
                        <span class={styles.posValue} style="color:{unrealizedPnl > 0 ? '#10b981' : unrealizedPnl < 0 ? '#ef4444' : '#94a3b8'}">
                            {unrealizedPnl >= 0 ? '+' : ''}{fmtUsd(unrealizedPnl)} ({unlockRealizedRoi >= 0 ? '+' : ''}{unlockRealizedRoi.toFixed(2)}%)
                        </span>
                    </div>
                    <div class={styles.posItem}>
                        <span class={styles.posLabel}>Stop Loss</span>
                        <span class={styles.posValue} style="color:#ef4444">{stopLoss !== null ? '$' + fmtSmall(stopLoss as number) : '—'}</span>
                    </div>
                    <div class={styles.posItem}>
                        <span class={styles.posLabel}>Take Profit</span>
                        <span class={styles.posValue} style="color:#10b981">{takeProfit !== null ? '$' + fmtSmall(takeProfit as number) : '—'}</span>
                    </div>
                </div>
            </div>
        {:else}
            <div class={styles.noPosition}>
                <span class={styles.noPosDot}>●</span>
                <span>No active position</span>
            </div>
        {/if}
    </div>

    <!-- Decision Tree -->
    <div class={styles.section}>
        <div class={styles.sectionTitle}>DECISION TREE</div>
        {#if !hasPosition}
            <div class={styles.actionRow}>
                <button class={styles.actionBtn} class:actionLong={true}>Open Long</button>
                <button class={styles.actionBtn} class:actionShort={true}>Open Short</button>
                <button class={styles.actionBtn} class:actionNeutral={true}>Do Nothing</button>
            </div>
        {:else}
            <div class={styles.actionRow}>
                <button class={styles.actionBtn} class:actionNeutral={true}>Do Nothing</button>
                <button class={styles.actionBtn} class:actionClose={true}>Close Position</button>
                <button class={styles.actionBtn} class:actionModify={true} onclick={() => { showModifySl = !showModifySl; showModifyTp = false; }}>
                    Move Stop Loss
                </button>
                <button class={styles.actionBtn} class:actionModify={true} onclick={() => { showModifyTp = !showModifyTp; showModifySl = false; }}>
                    Move Take Profit
                </button>
            </div>

            {#if showModifySl}
                <div class={styles.modifyForm}>
                    <input type="text" class={styles.modifyInput} bind:value={newSlValue} placeholder="New stop loss price" />
                    <span class={styles.modifyHint}>Current: {stopLoss !== null ? '$' + fmtSmall(stopLoss as number) : 'none'}</span>
                    <button class={styles.modifyConfirm}>Confirm</button>
                </div>
            {/if}
            {#if showModifyTp}
                <div class={styles.modifyForm}>
                    <input type="text" class={styles.modifyInput} bind:value={newTpValue} placeholder="New take profit price" />
                    <span class={styles.modifyHint}>Current: {takeProfit !== null ? '$' + fmtSmall(takeProfit as number) : 'none'}</span>
                    <button class={styles.modifyConfirm}>Confirm</button>
                </div>
            {/if}
        {/if}
    </div>
</div>
