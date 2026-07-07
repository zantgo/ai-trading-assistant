<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import { calcLiqPrice } from '../stores/paperTrading.svelte';
    import { getDecimalCount } from '../lib/telemetry';
    import styles from './PaperTradingPanel.module.css';
    import panel from './BottomConsole.module.css';

    const app = useAppStore();

    let {
        activeConsoleTab = $bindable<'positions' | 'orders' | 'history'>('positions'),
        expandedPositionId = $bindable<number | null>(null),
        showCloseDropdown = $bindable(false),
    }: {
        activeConsoleTab: 'positions' | 'orders' | 'history';
        expandedPositionId: number | null;
        showCloseDropdown: boolean;
    } = $props();

    const markPrice = $derived(parseFloat(app.priceText) || 0);
    const hasPosition = $derived(app.paperDirection !== '');
    const positionCount = $derived(hasPosition ? 1 : 0);
    const positionBrackets = $derived(
        app.paper.openOrders.filter((o: { is_reduce_only: boolean }) => o.is_reduce_only)
    );

    let bracketPrice = $state('');
    let bracketSize = $state(25);
    let bracketType = $state<'TP' | 'SL'>('TP');
    let copiedLabel = $state('');

    function fmt(n: number, decimals = 2): string {
        if (!isFinite(n)) return '—';
        return n.toFixed(decimals);
    }

    // Price-scaled formatter keyed off the active tab's current price.
    function fmtPx(n: number): string {
        if (!isFinite(n)) return '—';
        return n.toFixed(getDecimalCount(markPrice));
    }

    function fmtTs(ts: number): string {
        if (!ts) return '—';
        return new Date(ts).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    }

    function fmtPnl(val: number): string {
        if (!isFinite(val)) return '$0.00';
        return (val >= 0 ? '+' : '') + '$' + val.toFixed(2);
    }

    async function handleClose(pct?: number) {
        if (app.paperLoading) return;
        const cpct = pct ?? 100;
        const result = await app.closePositionPct(cpct);
        if (!result.success) alert(result.message);
        else { await app.fetchPaperStatus(); showCloseDropdown = false; }
    }

    async function handleAddBracket() {
        const price = parseFloat(bracketPrice);
        if (!price || price <= 0) return;
        if (bracketType === 'TP') {
            await app.setTpTargets([{ pct: bracketSize, price }]);
        } else {
            await app.setSlLevels([{ pct: bracketSize, price }]);
        }
        await app.fetchPaperStatus();
        await app.fetchOpenOrders();
        bracketPrice = '';
    }

    async function handleCancelBracket(orderId: number) {
        await app.cancelOrder(orderId);
        await app.fetchPaperStatus();
        await app.fetchOpenOrders();
    }

    async function handleCancelEntryOrder(orderId: number) {
        await app.cancelOrder(orderId);
    }

    async function handleCopyJson() {
        let data: unknown;
        if (activeConsoleTab === 'positions') {
            data = {
                symbol: app.activeTab,
                position: app.activePaperPosition,
                brackets: positionBrackets,
                unrealized_pnl: app.paperUnrealizedPnl,
                unrealized_roi: app.paperUnrealizedRoi,
            };
        } else if (activeConsoleTab === 'orders') {
            data = { symbol: app.activeTab, open_orders: app.paper.openOrders.filter((o: { is_reduce_only: boolean }) => !o.is_reduce_only) };
        } else {
            data = { symbol: app.activeTab, history: app.paperHistory };
        }
        try {
            await navigator.clipboard.writeText(JSON.stringify(data, null, 2));
            copiedLabel = 'Copied!';
            setTimeout(() => copiedLabel = '', 2000);
        } catch (_) {
            copiedLabel = 'Failed';
            setTimeout(() => copiedLabel = '', 2000);
        }
    }

    const pctPresets = [25, 50, 100];
</script>

<div class={styles.consoleWorkspace}>

    <!-- Tab Bar + Export Button -->
    <div class={styles.consoleTabBar}>
        <div class={styles.consoleTabs}>
            <button
                class="{styles.consoleTab} {activeConsoleTab === 'positions' ? styles.consoleTabActive : ''}"
                onclick={() => activeConsoleTab = 'positions'}
            >Positions<span class={styles.consoleTabCount}>{positionCount}</span></button>
            <button
                class="{styles.consoleTab} {activeConsoleTab === 'orders' ? styles.consoleTabActive : ''}"
                onclick={() => activeConsoleTab = 'orders'}
            >Open Orders<span class={styles.consoleTabCount}>{app.paper.openOrders.filter((o: { is_reduce_only: boolean }) => !o.is_reduce_only).length}</span></button>
            <button
                class="{styles.consoleTab} {activeConsoleTab === 'history' ? styles.consoleTabActive : ''}"
                onclick={() => activeConsoleTab = 'history'}
            >History<span class={styles.consoleTabCount}>{app.paperHistory.length}</span></button>
        </div>
        <button class={styles.exportBtn} onclick={handleCopyJson} title="Copy JSON">
            {copiedLabel || 'Export JSON'}
        </button>
    </div>

    <!-- Positions Table -->
    {#if activeConsoleTab === 'positions'}
        {@const pos = app.activePaperPosition ?? ({} as Record<string, unknown>)}
        {@const entryPx = (pos.average_entry_price as number) ?? (pos.entry_price as number) ?? 0}
        {@const posSize = (pos.size as number) ?? 0}
        {@const posLiq = entryPx > 0 ? calcLiqPrice(entryPx, app.paperDirection as 'LONG' | 'SHORT', app.paperLeverage) : 0}
        {@const tps = positionBrackets.filter((b: { order_type: string }) => b.order_type === 'LIMIT')}
        {@const sls = positionBrackets.filter((b: { order_type: string }) => b.order_type === 'STOP')}
        <div class={styles.tableWrapper}>
            {#if hasPosition}
                <table class={styles.table}>
                    <thead>
                        <tr>
                            <th class={panel.expandCol}></th>
                            <th>Market</th>
                            <th>Side</th>
                            <th class={styles.tableColRight}>Size</th>
                            <th class={styles.tableColRight}>Entry</th>
                            <th class={styles.tableColRight}>Mark</th>
                            <th class={styles.tableColRight}>Liq Price</th>
                            <th class={styles.tableColRight}>Margin</th>
                            <th class={styles.tableColRight}>P&L</th>
                            <th class={styles.tableColRight}>ROI</th>
                            <th></th>
                        </tr>
                    </thead>
                    <tbody>
                        <tr>
                            <td>
                                <button class={styles.expandIcon}
                                    onclick={() => expandedPositionId = expandedPositionId === pos.id ? null : (pos.id as number)}
                                >{expandedPositionId === pos.id ? '▼' : '▶'}</button>
                            </td>
                            <td class={styles.marketCell}>{app.activeTab}</td>
                            <td class="{styles.directionCell} {app.paperDirection === 'LONG' ? styles.directionLong : styles.directionShort}">
                                {app.paperDirection}
                            </td>
                            <td class={styles.numRight}>{fmt(posSize, 5)}</td>
                            <td class={styles.numRight}>{entryPx > 0 ? '$' + fmtPx(entryPx) : '—'}</td>
                            <td class={styles.numRight}>{markPrice > 0 ? '$' + fmtPx(markPrice) : '—'}</td>
                            <td class={styles.numRight}>{posLiq > 0 ? '$' + fmtPx(posLiq) : '—'}</td>
                            <td class={styles.numRight}>${app.paperMarginUsed.toFixed(2)}</td>
                            <td class="{styles.numRight} {app.paperUnrealizedPnl >= 0 ? styles.pnlPositive : styles.pnlNegative}">
                                {fmtPnl(app.paperUnrealizedPnl)}
                            </td>
                            <td class="{styles.numRight} {app.paperUnrealizedRoi >= 0 ? styles.pnlPositive : styles.pnlNegative}">
                                {app.paperUnrealizedRoi.toFixed(2)}%
                            </td>
                            <td>
                                <div class={styles.closeBtnGroup}>
                                    <button class={styles.closeBtn} disabled={app.paperLoading}
                                        onclick={() => handleClose(100)}>Close 100%</button>
                                    <button class={styles.closeDropdownBtn} disabled={app.paperLoading}
                                        onclick={() => showCloseDropdown = !showCloseDropdown}>▼</button>
                                    {#if showCloseDropdown}
                                        <div class={styles.closeDropdown}>
                                            <button onclick={() => handleClose(25)}>Close 25%</button>
                                            <button onclick={() => handleClose(50)}>Close 50%</button>
                                            <button onclick={() => handleClose(75)}>Close 75%</button>
                                        </div>
                                    {/if}
                                </div>
                            </td>
                        </tr>
                        <!-- Expandable Detail Row -->
                        {#if expandedPositionId === pos.id}
                            <tr class={styles.detailRow}>
                                <td></td>
                                <td colspan="10">
                                    <div class={styles.cockpitGrid}>
                                        <div class={styles.bracketSection}>
                                            <span class={styles.bracketSectionTitle + ' ' + styles.tpLabel}>Take Profit ({tps.length}/2)</span>
                                            {#each tps as b}
                                                <div class={styles.bracketItem}>
                                                    <span class={styles.bracketChip + ' ' + styles.chipTp}>${(b as { price: number | null }).price != null ? fmtPx((b as { price: number }).price) : '—'}</span>
                                                    <span class={styles.bracketSize}>{(b as { size: number }).size}%</span>
                                                    <button class={styles.cancelBracketBtn}
                                                        onclick={() => handleCancelBracket((b as { id: number }).id)}>×</button>
                                                </div>
                                            {/each}
                                            {#if tps.length === 0}
                                                <span class={styles.noBrackets}>—</span>
                                            {/if}
                                        </div>
                                        <div class={styles.bracketSection}>
                                            <span class={styles.bracketSectionTitle + ' ' + styles.slLabel}>Stop Loss ({sls.length}/2)</span>
                                            {#each sls as b}
                                                <div class={styles.bracketItem}>
                                                    <span class={styles.bracketChip + ' ' + styles.chipSl}>${(b as { trigger_price: number | null }).trigger_price != null ? fmtPx((b as { trigger_price: number }).trigger_price) : '—'}</span>
                                                    <span class={styles.bracketSize}>{(b as { size: number }).size}%</span>
                                                    <button class={styles.cancelBracketBtn}
                                                        onclick={() => handleCancelBracket((b as { id: number }).id)}>×</button>
                                                </div>
                                            {/each}
                                            {#if sls.length === 0}
                                                <span class={styles.noBrackets}>—</span>
                                            {/if}
                                        </div>
                                    </div>
                                    <div class={styles.bracketCreator}>
                                        <input type="number" class={styles.creatorInput}
                                            bind:value={bracketPrice} step="0.01" placeholder="Price" />
                                        <select class={styles.creatorSelect} bind:value={bracketSize}>
                                            {#each pctPresets as p}
                                                <option value={p}>{p}%</option>
                                            {/each}
                                        </select>
                                        <select class={styles.creatorSelect} bind:value={bracketType}>
                                            <option value="TP">TP</option>
                                            <option value="SL">SL</option>
                                        </select>
                                        <button class={styles.addBracketBtn}
                                            disabled={!bracketPrice || app.paperLoading}
                                            onclick={handleAddBracket}>Add Bracket</button>
                                    </div>
                                </td>
                            </tr>
                        {/if}
                    </tbody>
                </table>
            {:else}
                <div class={styles.emptyState}>No active position</div>
            {/if}
        </div>

    <!-- Open Orders Table -->
    {:else if activeConsoleTab === 'orders'}
        {@const entryOrders = app.paper.openOrders.filter((o: { is_reduce_only: boolean }) => !o.is_reduce_only)}
        <div class={styles.tableWrapper}>
            {#if entryOrders.length > 0}
                <table class={styles.table}>
                    <thead>
                        <tr>
                            <th>Type</th>
                            <th>Direction</th>
                            <th class={styles.tableColRight}>Price</th>
                            <th class={styles.tableColRight}>Size</th>
                            <th class={styles.tableColRight}>Created</th>
                            <th></th>
                        </tr>
                    </thead>
                    <tbody>
                        {#each entryOrders as order}
                            <tr>
                                <td class={styles.marketCell}>{(order as { order_type: string }).order_type}</td>
                                <td class="{styles.directionCell} {(order as { direction: string }).direction === 'BUY' ? styles.directionLong : styles.directionShort}">
                                    {(order as { direction: string }).direction}</td>
                                <td class={styles.numRight}>
                                    {(order as { price: number | null; trigger_price: number | null }).price ? '$' + fmtPx((order as { price: number | null }).price!) : ((order as { trigger_price: number | null }).trigger_price ? 'Trig $' + fmtPx((order as { trigger_price: number | null }).trigger_price!) : '—')}
                                </td>
                                <td class={styles.numRight}>{(order as { size: number }).size}%</td>
                                <td class={styles.numRight}>{fmtTs((order as { created_at: number }).created_at)}</td>
                                <td>
                                    <button class={styles.closeBtn} onclick={() => handleCancelEntryOrder((order as { id: number }).id)}>Cancel</button>
                                </td>
                            </tr>
                        {/each}
                    </tbody>
                </table>
            {:else}
                <div class={styles.emptyState}>No open entry orders</div>
            {/if}
        </div>

    <!-- History Table -->
    {:else}
        <div class={styles.tableWrapper}>
            {#if app.paperHistory.length > 0}
                <table class={styles.table}>
                    <thead>
                        <tr>
                            <th>Time</th>
                            <th>Market</th>
                            <th>Side</th>
                            <th class={styles.tableColRight}>Entry</th>
                            <th class={styles.tableColRight}>Exit</th>
                            <th class={styles.tableColRight}>P&L</th>
                            <th class={styles.tableColRight}>ROI</th>
                            <th>Trigger</th>
                        </tr>
                    </thead>
                    <tbody>
                        {#each app.paperHistory as trade}
                            {@const t = trade as Record<string, unknown>}
                            <tr>
                                <td>{fmtTs((t.exit_timestamp as number) ?? 0)}</td>
                                <td class={styles.marketCell}>{(t.symbol as string) ?? '—'}</td>
                                <td class="{styles.directionCell} {(t.direction as string) === 'LONG' ? styles.directionLong : styles.directionShort}">
                                    {t.direction as string}
                                </td>
                                <td class={styles.numRight}>
                                    {(t.entry_price as number) > 0 ? '$' + fmtPx(t.entry_price as number) : '—'}
                                </td>
                                <td class={styles.numRight}>
                                    {(t.exit_price as number) > 0 ? '$' + fmtPx(t.exit_price as number) : '—'}
                                </td>
                                <td class="{styles.numRight} {(t.realized_pnl as number) >= 0 ? styles.pnlPositive : styles.pnlNegative}">
                                    {fmtPnl((t.realized_pnl as number) ?? 0)}
                                </td>
                                <td class="{styles.numRight} {(t.roi_pct as number) >= 0 ? styles.pnlPositive : styles.pnlNegative}">
                                    {fmt((t.roi_pct as number) ?? 0)}%
                                </td>
                                <td>{(t.trigger as string) ?? '—'}</td>
                            </tr>
                        {/each}
                    </tbody>
                </table>
            {:else}
                <div class={styles.emptyState}>No trade history</div>
            {/if}
        </div>
    {/if}

    <!-- Account Mini Bar -->
    <div class={styles.accountBar}>
        <div class={styles.accountItem}>
            <span class={styles.accountItemLabel}>Balance</span>
            <span class={styles.accountItemValue}>${app.paperTotalAccountValue.toFixed(2)}</span>
        </div>
        <div class={styles.accountItem}>
            <span class={styles.accountItemLabel}>Available</span>
            <span class={styles.accountItemValue}>${app.paperCashBalance.toFixed(2)}</span>
        </div>
        <div class={styles.accountItem}>
            <span class={styles.accountItemLabel}>Margin Used</span>
            <span class={styles.accountItemValue}>${app.paperMarginUsed.toFixed(2)}</span>
        </div>
        <div class={styles.accountItem}>
            <span class={styles.accountItemLabel}>Leverage</span>
            <span class={styles.accountItemValue}>{app.paperLeverage}x</span>
        </div>
    </div>
</div>
