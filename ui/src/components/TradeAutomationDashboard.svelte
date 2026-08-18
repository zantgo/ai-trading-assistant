<script lang="ts">
    import { onMount } from 'svelte';
    import { useAppStore } from '../state.svelte';
    import styles from './TradeAutomationDashboard.module.css';

    const app = useAppStore();

    // ── Live data (v7) ─────────────────────────────────────────────────
    interface InstanceSummary {
        id: string;
        pair: string;
        status: string;
    }

    interface AutomationState {
        instance_id: string;
        symbol: string;
        mode: 'paper' | 'live';
        enabled: boolean;
        phase: 'idle' | 'pending_entry' | 'position_open' | null;
        fingerprint: string | null;
        tracked_setup: {
            symbol: string;
            direction: string;
            setup_type: string;
            score: number;
            source_tf: string;
            entry_mid: string;
            entry_zone_low: string;
            entry_zone_high: string;
            sl: string;
            tp: string;
            net_rr: number;
            time_horizon: string;
        } | null;
        projection: {
            risk_capital: string;
            position_size_units: string;
            position_notional: string;
            margin_required: string;
            liquidation_price: string;
            entry_fee_usd: string;
            exit_fee_usd: string;
            total_fees: string;
            net_profit_usd: string;
            roi_pct: string;
            net_rr: string | null;
        } | null;
        entry_order: Order | null;
        bracket: { tp_order: Order | null; sl_order: Order | null };
        position: {
            symbol: string;
            direction: string;
            size: string;
            entry_price: string;
            unrealized_pnl: string;
        } | null;
        invalidation: { state: string; detail: string };
        activity_log: { ts: number; event: string; detail: string }[];
        safety_gate: { blocked: boolean; reason: string | null };
        lifecycle: string;
        equity: string;
        open_positions_count: number;
    }

    interface Order {
        id: string | null;
        side: string;
        order_type: string;
        price: string | null;
        size: string;
        status: string;
        filled_size: string;
        fill_price: string | null;
        reduce_only: boolean;
        created_at: number;
    }

    interface TradeRow {
        id: number;
        symbol: string;
        direction: string;
        entry_price: number;
        exit_price: number;
        size: number;
        commission_fees: number;
        realized_pnl: number;
        roi_pct: number;
        trigger_source: string;
        entry_timestamp: number;
        exit_timestamp: number;
    }

    let instances = $state<InstanceSummary[]>([]);
    let selectedId = $state('');
    let automation = $state<AutomationState | null>(null);
    let trades = $state<TradeRow[]>([]);
    let loading = $state(true);
    let error = $state('');
    let closing = $state(false);
    let switchingMode = $state(false);
    let modeError = $state('');

    async function toggleMode() {
        if (!selectedId || switchingMode) return;
        switchingMode = true;
        modeError = '';
        const target = automation?.mode === 'live' ? 'paper' : 'live';
        const result = await app.setInstanceMode(selectedId, target);
        if (!result.ok) {
            modeError = result.error ?? 'Mode switch failed';
        } else {
            await refresh();
        }
        switchingMode = false;
    }

    async function loadInstances() {
        try {
            const res = await fetch('/api/instances');
            const data = await res.json();
            instances = data.instances ?? [];
            if (!selectedId && instances.length > 0) {
                selectedId = instances[0].id;
            }
        } catch {
            instances = [];
        }
    }

    async function refresh() {
        if (!selectedId) return;
        try {
            const [autoRes, tradesRes] = await Promise.all([
                fetch(`/api/instances/${selectedId}/automation`),
                fetch('/api/trade-ledger?limit=50'),
            ]);
            if (autoRes.ok) {
                automation = (await autoRes.json()) as AutomationState;
                error = '';
            }
            if (tradesRes.ok) {
                trades = (await tradesRes.json()) as TradeRow[];
            }
        } catch (e) {
            error = String(e);
        } finally {
            loading = false;
        }
    }

    async function closeNow() {
        if (!selectedId || closing) return;
        closing = true;
        try {
            await fetch(`/api/instances/${selectedId}/automation/close`, { method: 'POST' });
            await refresh();
        } finally {
            closing = false;
        }
    }

    onMount(() => {
        const boot = async () => {
            await loadInstances();
            await refresh();
        };
        boot();
        const timer = setInterval(refresh, 2000);
        return () => clearInterval(timer);
    });

    // ── Formatters ─────────────────────────────────────────────────────
    function fmtTs(ts: number): string {
        const d = new Date(ts);
        return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
    }

    function fmtUsd(v: string | number | null | undefined): string {
        if (v == null || v === '' || !isFinite(Number(v))) return '—';
        const n = Number(v);
        return n.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 });
    }

    function signedUsd(v: string | number | null | undefined): string {
        if (v == null || v === '' || !isFinite(Number(v))) return '—';
        const n = Number(v);
        return (n > 0 ? '+' : '') + fmtUsd(n);
    }

    function fmtNum(v: string | number | null | undefined, d = 2): string {
        if (v == null || v === '' || !isFinite(Number(v))) return '—';
        return Number(v).toFixed(d);
    }

    function statusLabel(s: string): string {
        const m: Record<string, string> = {
            Open: 'OPEN',
            Closed: 'CLOSED',
            Cancelled: 'CANCELLED',
            PartiallyFilled: 'PARTIAL',
            Pending: 'PENDING',
            Submitted: 'SUBMITTED',
            Rejected: 'REJECTED',
        };
        return m[s] ?? s.toUpperCase();
    }

    function orderStatusClass(s: string): string {
        if (s === 'Closed') return styles.badgeFilled;
        if (s === 'Cancelled' || s === 'Rejected') return styles.badgeBlocked;
        return styles.badgePending;
    }

    function phaseLabel(p: string | null): string {
        switch (p) {
            case 'pending_entry': return 'WAITING ENTRY';
            case 'position_open': return 'POSITION OPEN';
            case 'idle': return 'IDLE — scanning for setups';
            default: return '—';
        }
    }

    function eventLabel(e: string): string {
        const m: Record<string, string> = {
            setup_accepted: 'SETUP ACCEPTED',
            entry_rejected: 'ENTRY REJECTED',
            entry_filled: 'ENTRY FILLED',
            bracket_armed: 'BRACKET ARMED',
            invalidated_level: 'INVALIDATED — level breached',
            invalidated_signal: 'INVALIDATED — recommendation flipped',
            cancelled_replaced: 'CANCELLED — setup replaced',
            position_closed: 'POSITION CLOSED',
            close_error: 'CLOSE ERROR',
            recovery_flatten: 'RECOVERY — flattened at last mark',
        };
        return m[e] ?? e.replace(/_/g, ' ').toUpperCase();
    }

    function eventClass(e: string): string {
        if (e === 'invalidated_level' || e === 'invalidated_signal' || e === 'close_error') return styles.eventBad;
        if (e === 'entry_filled' || e === 'position_closed') return styles.eventGood;
        return styles.eventNeutral;
    }

    function pnlClass(v: string | number | null | undefined): string {
        const n = v == null || v === '' ? 0 : Number(v);
        return n > 0 ? styles.pos : n < 0 ? styles.neg : '';
    }
</script>

<div class={styles.dashboard}>
    <header class={styles.header}>
        <div class={styles.headerLeft}>
            <h2 class={styles.title}>TRADE AUTOMATION</h2>
            <span class="{styles.modeBadge} {automation?.mode === 'live' ? styles.modeLive : styles.modePaper}">
                {automation?.mode?.toUpperCase() ?? 'PAPER'}
            </span>
            <button class="{styles.modeToggle} {automation?.mode === 'live' ? styles.modeToggleLive : styles.modeTogglePaper}"
                onclick={toggleMode} disabled={switchingMode || !selectedId}>
                {switchingMode ? 'Switching…' : automation?.mode === 'live' ? 'Switch to PAPER' : 'Switch to LIVE'}
            </button>
            {#if automation?.enabled}
                <span class="{styles.badge} {styles.badgeRunning}">AUTOMATION ON</span>
            {:else}
                <span class="{styles.badge} {styles.badgeStopped}">AUTOMATION OFF</span>
            {/if}
        </div>
        <div class={styles.headerRight}>
            <select class={styles.instanceSelect} bind:value={selectedId} onchange={refresh}>
                {#each instances as inst (inst.id)}
                    <option value={inst.id}>{inst.pair} ({inst.id})</option>
                {/each}
            </select>
            <span class="{styles.badge} {automation?.lifecycle === 'RUNNING' ? styles.badgeRunning : automation?.lifecycle === 'PAUSED' ? styles.badgePaused : styles.badgeStopped}">
                {automation?.lifecycle ?? '—'}
            </span>
            {#if automation?.safety_gate?.blocked}
                <span class="{styles.badge} {styles.badgeBlocked}">SAFETY: {automation.safety_gate.reason}</span>
            {/if}
            {#if modeError}
                <span class="{styles.badge} {styles.badgeBlocked}">{modeError}</span>
            {/if}
        </div>
    </header>

    {#if loading}
        <div class={styles.empty}>Loading automation state…</div>
    {:else if error && !automation}
        <div class={styles.empty}>{error}</div>
    {:else if !automation}
        <div class={styles.empty}>No automation state available (is the daemon running with [workspace.minimal_tae] enabled?).</div>
    {:else}
        <!-- ── Equity strip ── -->
        <div class={styles.statsGrid}>
            <div class={styles.statCard}>
                <div class={styles.statLabel}>Equity</div>
                <div class={styles.statValue}>${fmtUsd(automation.equity)}</div>
                <div class={styles.statSub}>Unified engine ledger</div>
            </div>
            <div class={styles.statCard}>
                <div class={styles.statLabel}>Open Positions</div>
                <div class={styles.statValue}>{automation.open_positions_count}</div>
                <div class={styles.statSub}>Across all symbols</div>
            </div>
            <div class={styles.statCard}>
                <div class={styles.statLabel}>Executor Phase</div>
                <div class={styles.statValue} style="font-size:0.85rem">{phaseLabel(automation.phase)}</div>
                <div class={styles.statSub}>{automation.symbol}</div>
            </div>
            <div class={styles.statCard}>
                <div class={styles.statLabel}>Tracked Setup</div>
                <div class={styles.statValue} style="font-size:0.85rem">
                    {automation.tracked_setup ? `${automation.tracked_setup.direction} ${automation.tracked_setup.setup_type}` : '—'}
                </div>
                <div class={styles.statSub}>
                    {automation.tracked_setup ? `score ${fmtNum(automation.tracked_setup.score, 0)} · RR ${fmtNum(automation.tracked_setup.net_rr)} · ${automation.tracked_setup.source_tf}` : 'No active setup'}
                </div>
            </div>
        </div>

        <!-- ── Active setup + projected risk and return ── -->
        {#if automation.tracked_setup}
            <section class={styles.section}>
                <h3 class={styles.sectionTitle}>Active Setup</h3>
                {#if automation.tracked_setup.direction === 'LONG'}
                    <span class="{styles.badge} {styles.badgeLong}">LONG</span>
                {:else}
                    <span class="{styles.badge} {styles.badgeShort}">SHORT</span>
                {/if}
                <span class={styles.setupType}>{automation.tracked_setup.setup_type}</span>
                <span class={styles.setupMeta}>score {fmtNum(automation.tracked_setup.score, 0)} · RR {fmtNum(automation.tracked_setup.net_rr)} · {automation.tracked_setup.time_horizon} · source {automation.tracked_setup.source_tf}</span>

                <div class={styles.setupGrid}>
                    <div class={styles.setupBox}>
                        <div class={styles.setupLabel}>Entry (limit)</div>
                        <div class={styles.setupValue}>${fmtUsd(automation.tracked_setup.entry_mid)}</div>
                        <div class={styles.setupSub}>zone ${fmtUsd(automation.tracked_setup.entry_zone_low)} – ${fmtUsd(automation.tracked_setup.entry_zone_high)}</div>
                    </div>
                    <div class={styles.setupBox}>
                        <div class={styles.setupLabel}>Stop-Loss (invalidation)</div>
                        <div class={styles.setupValue} style="color:#ef5350">${fmtUsd(automation.tracked_setup.sl)}</div>
                        <div class={styles.setupSub}>LEVEL invalidation</div>
                    </div>
                    <div class={styles.setupBox}>
                        <div class={styles.setupLabel}>Take-Profit</div>
                        <div class={styles.setupValue} style="color:#4caf50">${fmtUsd(automation.tracked_setup.tp)}</div>
                        <div class={styles.setupSub}>target zone midpoint</div>
                    </div>
                </div>

                {#if automation.projection}
                    <div class={styles.projection}>
                        <span class={styles.projectionTitle}>PROJECTED RISK AND RETURN</span>
                        <div class={styles.projectionGrid}>
                            <div class={styles.projItem}><span class={styles.projLabel}>Size</span><span class={styles.projValue}>{fmtNum(automation.projection.position_size_units, 4)}</span></div>
                            <div class={styles.projItem}><span class={styles.projLabel}>Notional</span><span class={styles.projValue}>${fmtUsd(automation.projection.position_notional)}</span></div>
                            <div class={styles.projItem}><span class={styles.projLabel}>Risk capital</span><span class={styles.projValue}>${fmtUsd(automation.projection.risk_capital)}</span></div>
                            <div class={styles.projItem}><span class={styles.projLabel}>Margin</span><span class={styles.projValue}>${fmtUsd(automation.projection.margin_required)}</span></div>
                            <div class={styles.projItem}><span class={styles.projLabel}>Liq. price</span><span class={styles.projValue}>${fmtUsd(automation.projection.liquidation_price)}</span></div>
                            <div class={styles.projItem}><span class={styles.projLabel}>Entry fee</span><span class={styles.projValue}>${fmtUsd(automation.projection.entry_fee_usd)}</span></div>
                            <div class={styles.projItem}><span class={styles.projLabel}>Exit fee</span><span class={styles.projValue}>${fmtUsd(automation.projection.exit_fee_usd)}</span></div>
                            <div class={styles.projItem}><span class={styles.projLabel}>Total fees</span><span class={styles.projValue}>${fmtUsd(automation.projection.total_fees)}</span></div>
                            <div class={styles.projItem}><span class={styles.projLabel}>Profit @ TP</span><span class="{styles.projValue} {pnlClass(automation.projection.net_profit_usd)}">${signedUsd(automation.projection.net_profit_usd)}</span></div>
                            <div class={styles.projItem}><span class={styles.projLabel}>ROI @ TP</span><span class="{styles.projValue} {pnlClass(automation.projection.roi_pct)}">{fmtNum(automation.projection.roi_pct)}%</span></div>
                        </div>
                    </div>
                {/if}
            </section>
        {:else}
            <section class={styles.section}>
                <h3 class={styles.sectionTitle}>Active Setup</h3>
                <div class={styles.empty}>No eligible setup right now — the executor scans the 4 timeframes on every completed candle. Setups must be Actionable (net RR ≥ 1.0) and READY.</div>
            </section>
        {/if}

        <!-- ── Order board ── -->
        <section class={styles.section}>
            <h3 class={styles.sectionTitle}>Order Board</h3>
            {#if !automation.entry_order && !automation.bracket.tp_order && !automation.bracket.sl_order}
                <div class={styles.empty}>No orders.</div>
            {:else}
                <table class={styles.table}>
                    <thead><tr><th>Role</th><th>Type</th><th>Side</th><th class={styles.tdRight}>Price</th><th class={styles.tdRight}>Size</th><th class={styles.tdRight}>Filled</th><th>Status</th></tr></thead>
                    <tbody>
                        {#if automation.entry_order}
                            <tr>
                                <td class={styles.tdMono}>ENTRY</td>
                                <td class={styles.tdMono}>{automation.entry_order.order_type}</td>
                                <td class={automation.entry_order.side === 'BUY' ? styles.pos : styles.neg}>{automation.entry_order.side}</td>
                                <td class={styles.tdRight}>${fmtUsd(automation.entry_order.price)}</td>
                                <td class={styles.tdRight}>{fmtNum(automation.entry_order.size, 4)}</td>
                                <td class={styles.tdRight}>{fmtNum(automation.entry_order.filled_size, 4)}</td>
                                <td><span class="{styles.badge} {orderStatusClass(automation.entry_order.status)}">{statusLabel(automation.entry_order.status)}</span></td>
                            </tr>
                        {/if}
                        {#if automation.bracket.tp_order}
                            <tr>
                                <td class={styles.tdMono}>TP</td>
                                <td class={styles.tdMono}>{automation.bracket.tp_order.order_type}</td>
                                <td class={automation.bracket.tp_order.side === 'BUY' ? styles.pos : styles.neg}>{automation.bracket.tp_order.side}</td>
                                <td class={styles.tdRight}>${fmtUsd(automation.bracket.tp_order.price)}</td>
                                <td class={styles.tdRight}>{fmtNum(automation.bracket.tp_order.size, 4)}</td>
                                <td class={styles.tdRight}>{fmtNum(automation.bracket.tp_order.filled_size, 4)}</td>
                                <td><span class="{styles.badge} {orderStatusClass(automation.bracket.tp_order.status)}">{statusLabel(automation.bracket.tp_order.status)}</span></td>
                            </tr>
                        {/if}
                        {#if automation.bracket.sl_order}
                            <tr>
                                <td class={styles.tdMono}>SL</td>
                                <td class={styles.tdMono}>{automation.bracket.sl_order.order_type}</td>
                                <td class={automation.bracket.sl_order.side === 'BUY' ? styles.pos : styles.neg}>{automation.bracket.sl_order.side}</td>
                                <td class={styles.tdRight}>${fmtUsd(automation.bracket.sl_order.price)}</td>
                                <td class={styles.tdRight}>{fmtNum(automation.bracket.sl_order.size, 4)}</td>
                                <td class={styles.tdRight}>{fmtNum(automation.bracket.sl_order.filled_size, 4)}</td>
                                <td><span class="{styles.badge} {orderStatusClass(automation.bracket.sl_order.status)}">{statusLabel(automation.bracket.sl_order.status)}</span></td>
                            </tr>
                        {/if}
                    </tbody>
                </table>
            {/if}
        </section>

        <!-- ── Position card ── -->
        <section class={styles.section}>
            <h3 class={styles.sectionTitle}>Position</h3>
            {#if automation.position}
                <div class={styles.positionCard}>
                    <div class={styles.positionRow}>
                        <span class="{styles.badge} {automation.position.direction === 'LONG' ? styles.badgeLong : styles.badgeShort}">{automation.position.direction}</span>
                        <span class={styles.positionSymbol}>{automation.position.symbol}</span>
                        <span class={styles.positionMeta}>size {fmtNum(automation.position.size, 4)} · entry ${fmtUsd(automation.position.entry_price)}</span>
                        <span class="{styles.positionPnl} {pnlClass(automation.position.unrealized_pnl)}">uPnL {signedUsd(automation.position.unrealized_pnl)}</span>
                        <button class="{styles.closeBtn}" onclick={closeNow} disabled={closing}>{closing ? 'Closing…' : 'Close now'}</button>
                    </div>
                    <div class={styles.invalidationBanner}>
                        <strong>Invalidation:</strong> a position closes when price hits TP or SL (LEVEL), or when the recommendation flips to the opposite direction (SIGNAL → close at market). A neutral signal does not invalidate an open position.
                    </div>
                </div>
            {:else}
                <div class={styles.empty}>No open position.</div>
            {/if}
        </section>

        <!-- ── Activity log ── -->
        <section class={styles.section}>
            <h3 class={styles.sectionTitle}>Activity Log</h3>
            {#if automation.activity_log.length === 0}
                <div class={styles.empty}>No events yet.</div>
            {:else}
                <div class={styles.activityList}>
                    {#each automation.activity_log as a (a.ts)}
                        <div class={styles.activityRow}>
                            <span class={styles.activityTs}>{fmtTs(a.ts)}</span>
                            <span class="{styles.activityEvent} {eventClass(a.event)}">{eventLabel(a.event)}</span>
                            <span class={styles.activityDetail}>{a.detail}</span>
                        </div>
                    {/each}
                </div>
            {/if}
        </section>

        <!-- ── Trade history ── -->
        <section class={styles.section}>
            <h3 class={styles.sectionTitle}>Trade History</h3>
            {#if trades.length === 0}
                <div class={styles.empty}>No closed trades yet. Closed trades appear here with their exit reason (TP / SL / invalidated / manual / stop flatten).</div>
            {:else}
                <table class={styles.table}>
                    <thead>
                        <tr>
                            <th>Exited</th><th>Symbol</th><th>Side</th>
                            <th class={styles.tdRight}>Entry</th><th class={styles.tdRight}>Exit</th>
                            <th class={styles.tdRight}>Size</th><th class={styles.tdRight}>Fees</th>
                            <th class={styles.tdRight}>P&L</th><th class={styles.tdRight}>ROI</th><th>Trigger</th>
                        </tr>
                    </thead>
                    <tbody>
                        {#each trades as t (t.id)}
                            <tr>
                                <td>{fmtTs(t.exit_timestamp)}</td>
                                <td class={styles.tdMono}>{t.symbol}</td>
                                <td class={t.direction === 'LONG' ? styles.pos : styles.neg}>{t.direction}</td>
                                <td class={styles.tdRight}>${fmtUsd(t.entry_price)}</td>
                                <td class={styles.tdRight}>${fmtUsd(t.exit_price)}</td>
                                <td class={styles.tdRight}>{fmtNum(t.size, 4)}</td>
                                <td class={styles.tdRight}>${fmtUsd(t.commission_fees)}</td>
                                <td class="{styles.tdRight} {pnlClass(t.realized_pnl)}">{signedUsd(t.realized_pnl)}</td>
                                <td class="{styles.tdRight} {pnlClass(t.roi_pct)}">{fmtNum(t.roi_pct)}%</td>
                                <td class={styles.tdMono}>{t.trigger_source}</td>
                            </tr>
                        {/each}
                    </tbody>
                </table>
            {/if}
        </section>
    {/if}
</div>
