<script lang="ts">
    import { onMount } from 'svelte';
    import styles from './PortfolioDashboard.module.css';

    // ── Live data (v7, informational) ──────────────────────────────────
    interface InstanceSummary {
        id: string;
        pair: string;
        status: string;
    }

    interface PortfolioState {
        instance_id: string;
        symbol: string;
        initial_capital: string;
        current_equity: string;
        peak_equity: string;
        max_drawdown_pct: string;
        realized_pnl: string;
        unrealized_pnl: string;
        daily_pnl: string;
        starting_session_equity: string;
        safety_state: string;
        safety_context: string;
        consecutive_losses: Record<string, number>;
        systemic_risk_score: number;
        lifecycle: string;
        exposure: {
            gross_exposure: string;
            net_exposure: string;
            net_exposure_pct: string;
            long_exposure: string;
            short_exposure: string;
            symbol_concentration: Record<string, string>;
            max_single_pair_pct: string;
        };
        capital: {
            available_margin: string;
            committed_margin: string;
            margin_usage_ratio: string;
            leverage_ratio: string;
            margin_alert: string | null;
        };
        position_count: number;
        positions: {
            symbol: string;
            direction: string;
            size: string;
            entry_price: string;
            mark_price: string;
            unrealized_pnl: string;
            roi_pct: string;
            stop_loss_price: string | null;
            take_profit_price: string | null;
        }[];
    }

    interface SafetyState {
        instance_id: string;
        safety_state: string;
        consecutive_losses: Record<string, number>;
        peak_equity: string;
        current_equity: number;
        initial_capital: number;
        context: string;
        daily_pnl: string;
        max_drawdown_pct: string;
        margin_usage_ratio: string;
    }

    type Panel = 'overview' | 'positions' | 'exposure' | 'capital' | 'safety';
    let activePanel = $state<Panel>('overview');

    let instances = $state<InstanceSummary[]>([]);
    let selectedId = $state('');
    let portfolio = $state<PortfolioState | null>(null);
    let safety = $state<SafetyState | null>(null);
    let loading = $state(true);
    let error = $state('');
    let resetting = $state(false);

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
            const [pRes, sRes] = await Promise.all([
                fetch(`/api/instances/${selectedId}/portfolio`),
                fetch(`/api/instances/${selectedId}/safety`),
            ]);
            if (pRes.ok) portfolio = (await pRes.json()) as PortfolioState;
            if (sRes.ok) safety = (await sRes.json()) as SafetyState;
            error = '';
        } catch (e) {
            error = String(e);
        } finally {
            loading = false;
        }
    }

    async function sessionReset() {
        if (!selectedId || resetting) return;
        resetting = true;
        try {
            await fetch(`/api/instances/${selectedId}/safety/session-reset`, { method: 'POST' });
            await refresh();
        } finally {
            resetting = false;
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
    function fmtUsd(v: string | number | null | undefined): string {
        if (v == null || v === '' || !isFinite(Number(v))) return '—';
        return Number(v).toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 });
    }

    function signedUsd(v: string | number | null | undefined): string {
        if (v == null || v === '' || !isFinite(Number(v))) return '—';
        const n = Number(v);
        return (n > 0 ? '+' : '') + fmtUsd(n);
    }

    function fmtPct(v: string | number | null | undefined): string {
        if (v == null || v === '' || !isFinite(Number(v))) return '—';
        return `${Number(v).toFixed(2)}%`;
    }

    function fmtNum(v: number | null | undefined): string {
        if (v == null || !isFinite(v)) return '—';
        return v.toFixed(1);
    }

    function pnlClass(v: string | number | null | undefined): string {
        const n = v == null || v === '' ? 0 : Number(v);
        return n > 0 ? styles.pos : n < 0 ? styles.neg : '';
    }

    function safetyBadge(s: string | undefined): string {
        const m: Record<string, string> = {
            NORMAL: styles.badgeNormal,
            WARN: styles.badgeWarn,
            CAUTIOUS: styles.badgeCautious,
            SUSPENDED: styles.badgeSuspended,
            DRAWDOWN_STOP: styles.badgeDrawdown,
        };
        return m[s ?? ''] ?? styles.badgeNormal;
    }

    function alertBadge(a: string | null): string {
        if (!a) return '';
        if (a === 'EMERGENCY') return styles.badgeDrawdown;
        if (a === 'CLOSE_ONLY') return styles.badgeSuspended;
        return styles.badgeWarn;
    }
</script>

<div class={styles.dashboard}>
    <header class={styles.header}>
        <div class={styles.headerLeft}>
            <h2 class={styles.title}>PORTFOLIO</h2>
            <span class="{styles.badge} {safetyBadge(portfolio?.safety_state)}">
                {portfolio?.safety_state ?? '—'}
            </span>
            <span class="{styles.badge} {styles.badgeLifecycle}">{portfolio?.lifecycle ?? '—'}</span>
        </div>
        <div class={styles.headerRight}>
            <select class={styles.instanceSelect} bind:value={selectedId} onchange={refresh}>
                {#each instances as inst (inst.id)}
                    <option value={inst.id}>{inst.pair} ({inst.id})</option>
                {/each}
            </select>
        </div>
    </header>

    <div class={styles.sidebarLayout}>
        <nav class={styles.sidebar}>
            <button class="{styles.sidebarBtn} {activePanel === 'overview' ? styles.sidebarBtnActive : ''}" onclick={() => activePanel = 'overview'}>Overview</button>
            <button class="{styles.sidebarBtn} {activePanel === 'positions' ? styles.sidebarBtnActive : ''}" onclick={() => activePanel = 'positions'}>Positions</button>
            <button class="{styles.sidebarBtn} {activePanel === 'exposure' ? styles.sidebarBtnActive : ''}" onclick={() => activePanel = 'exposure'}>Exposure</button>
            <button class="{styles.sidebarBtn} {activePanel === 'capital' ? styles.sidebarBtnActive : ''}" onclick={() => activePanel = 'capital'}>Capital</button>
            <button class="{styles.sidebarBtn} {activePanel === 'safety' ? styles.sidebarBtnActive : ''}" onclick={() => activePanel = 'safety'}>Safety</button>
        </nav>

        <div class={styles.content}>
            {#if loading}
                <div class={styles.empty}>Loading portfolio state…</div>
            {:else if error && !portfolio}
                <div class={styles.empty}>{error}</div>
            {:else if !portfolio}
                <div class={styles.empty}>No portfolio state available (is the daemon running?).</div>
            {:else if activePanel === 'overview'}
                <h3 class={styles.sectionTitle}>Account Overview</h3>
                <p class={styles.infoLine}>PME reports current portfolio state. It never executes: the automation executor is the only thing that trades, and it blocks new entries in DRAWDOWN_STOP / SUSPENDED.</p>
                <div class={styles.statsGrid}>
                    <div class={styles.statCard}><div class={styles.statLabel}>Equity</div><div class={styles.statValue}>${fmtUsd(portfolio.current_equity)}</div><div class={styles.statSub}>cash + realized PnL</div></div>
                    <div class={styles.statCard}><div class={styles.statLabel}>Initial Capital</div><div class={styles.statValue}>${fmtUsd(portfolio.initial_capital)}</div><div class={styles.statSub}>per instance</div></div>
                    <div class={styles.statCard}><div class={styles.statLabel}>Peak Equity</div><div class={styles.statValue}>${fmtUsd(portfolio.peak_equity)}</div><div class={styles.statSub}>high-water mark</div></div>
                    <div class={styles.statCard}><div class={styles.statLabel}>Max Drawdown</div><div class={styles.statValue} style="color:#ef5350">{fmtPct(portfolio.max_drawdown_pct)}</div><div class={styles.statSub}>from peak</div></div>
                    <div class={styles.statCard}><div class={styles.statLabel}>Realized PnL</div><div class="{styles.statValue} {pnlClass(portfolio.realized_pnl)}">{signedUsd(portfolio.realized_pnl)}</div><div class={styles.statSub}>net of fees</div></div>
                    <div class={styles.statCard}><div class={styles.statLabel}>Unrealized PnL</div><div class="{styles.statValue} {pnlClass(portfolio.unrealized_pnl)}">{signedUsd(portfolio.unrealized_pnl)}</div><div class={styles.statSub}>mark-to-market</div></div>
                    <div class={styles.statCard}><div class={styles.statLabel}>Daily PnL</div><div class="{styles.statValue} {pnlClass(portfolio.daily_pnl)}">{signedUsd(portfolio.daily_pnl)}</div><div class={styles.statSub}>vs session start</div></div>
                    <div class={styles.statCard}><div class={styles.statLabel}>Open Positions</div><div class={styles.statValue}>{portfolio.position_count}</div><div class={styles.statSub}>across this instance</div></div>
                    <div class={styles.statCard}><div class={styles.statLabel}>Systemic Risk</div><div class={styles.statValue}>{fmtNum(portfolio.systemic_risk_score)}</div><div class={styles.statSub}>MME L7 (display)</div></div>
                </div>
                <div class={styles.sessionBar}>
                    <span class={styles.sessionLabel}>Session: started at ${fmtUsd(portfolio.starting_session_equity)}</span>
                    <button class={styles.resetBtn} onclick={sessionReset} disabled={resetting}>
                        {resetting ? 'Resetting…' : 'Reset session'}
                    </button>
                </div>

            {:else if activePanel === 'positions'}
                <h3 class={styles.sectionTitle}>Positions</h3>
                {#if portfolio.positions.length === 0}
                    <div class={styles.empty}>No open positions.</div>
                {:else}
                    <table class={styles.table}>
                        <thead><tr><th>Symbol</th><th>Side</th><th class={styles.tdRight}>Size</th><th class={styles.tdRight}>Entry</th><th class={styles.tdRight}>Mark</th><th class={styles.tdRight}>uPnL</th><th class={styles.tdRight}>ROI</th><th class={styles.tdRight}>SL</th><th class={styles.tdRight}>TP</th></tr></thead>
                        <tbody>
                            {#each portfolio.positions as p (p.symbol)}
                                <tr>
                                    <td class={styles.tdMono}>{p.symbol}</td>
                                    <td class={p.direction === 'LONG' ? styles.pos : styles.neg}>{p.direction}</td>
                                    <td class={styles.tdRight}>{Number(p.size).toFixed(4)}</td>
                                    <td class={styles.tdRight}>${fmtUsd(p.entry_price)}</td>
                                    <td class={styles.tdRight}>${fmtUsd(p.mark_price)}</td>
                                    <td class="{styles.tdRight} {pnlClass(p.unrealized_pnl)}">{signedUsd(p.unrealized_pnl)}</td>
                                    <td class="{styles.tdRight} {pnlClass(p.roi_pct)}">{fmtPct(p.roi_pct)}</td>
                                    <td class={styles.tdRight}>${fmtUsd(p.stop_loss_price)}</td>
                                    <td class={styles.tdRight}>${fmtUsd(p.take_profit_price)}</td>
                                </tr>
                            {/each}
                        </tbody>
                    </table>
                {/if}

            {:else if activePanel === 'exposure'}
                <h3 class={styles.sectionTitle}>Exposure</h3>
                <div class={styles.statsGrid}>
                    <div class={styles.statCard}><div class={styles.statLabel}>Gross Exposure</div><div class={styles.statValue}>${fmtUsd(portfolio.exposure.gross_exposure)}</div><div class={styles.statSub}>total notional</div></div>
                    <div class={styles.statCard}><div class={styles.statLabel}>Net Exposure</div><div class={styles.statValue}>${fmtUsd(portfolio.exposure.net_exposure)}</div><div class={styles.statSub}>{fmtPct(portfolio.exposure.net_exposure_pct)} of equity</div></div>
                    <div class={styles.statCard}><div class={styles.statLabel}>Long</div><div class={styles.statValue} style="color:#58d68d">${fmtUsd(portfolio.exposure.long_exposure)}</div><div class={styles.statSub}>long notional</div></div>
                    <div class={styles.statCard}><div class={styles.statLabel}>Short</div><div class={styles.statValue} style="color:#ff7b7b">${fmtUsd(portfolio.exposure.short_exposure)}</div><div class={styles.statSub}>short notional</div></div>
                </div>
                <h4 class={styles.subTitle}>Symbol Concentration</h4>
                {#if Object.keys(portfolio.exposure.symbol_concentration).length === 0}
                    <div class={styles.empty}>No exposure.</div>
                {:else}
                    <div class={styles.concList}>
                        {#each Object.entries(portfolio.exposure.symbol_concentration) as [sym, pct] (sym)}
                            <div class={styles.concRow}>
                                <span class={styles.tdMono}>{sym}</span>
                                <div class={styles.concTrack}>
                                    <div class={styles.concFill} style="width:{Math.min(Number(pct) * 100, 100)}%"></div>
                                </div>
                                <span class={styles.concPct}>{fmtPct(Number(pct) * 100)}</span>
                                <span class={styles.concLimit}>limit 20%</span>
                            </div>
                        {/each}
                    </div>
                {/if}

            {:else if activePanel === 'capital'}
                <h3 class={styles.sectionTitle}>Capital</h3>
                <div class={styles.statsGrid}>
                    <div class={styles.statCard}><div class={styles.statLabel}>Available Margin</div><div class={styles.statValue}>${fmtUsd(portfolio.capital.available_margin)}</div><div class={styles.statSub}>free for new entries</div></div>
                    <div class={styles.statCard}><div class={styles.statLabel}>Committed Margin</div><div class={styles.statValue}>${fmtUsd(portfolio.capital.committed_margin)}</div><div class={styles.statSub}>on open positions</div></div>
                    <div class={styles.statCard}><div class={styles.statLabel}>Margin Usage</div><div class={styles.statValue}>{fmtPct(Number(portfolio.capital.margin_usage_ratio) * 100)}</div><div class={styles.statSub}>80% warn · 95% close-only · 100% emergency</div></div>
                    <div class={styles.statCard}><div class={styles.statLabel}>Leverage</div><div class={styles.statValue}>{Number(portfolio.capital.leverage_ratio).toFixed(2)}×</div><div class={styles.statSub}>effective</div></div>
                </div>
                {#if portfolio.capital.margin_alert}
                    <div class="{styles.alertBanner} {alertBadge(portfolio.capital.margin_alert)}">
                        MARGIN ALERT: {portfolio.capital.margin_alert}
                    </div>
                {/if}

            {:else if activePanel === 'safety'}
                <h3 class={styles.sectionTitle}>Safety</h3>
                <div class={styles.safetyCard}>
                    <span class="{styles.badge} {safetyBadge(portfolio.safety_state)}">{portfolio.safety_state}</span>
                    <p class={styles.safetyContext}>{portfolio.safety_context}</p>
                    <p class={styles.infoLine}>
                        Safety state is informational. The automation executor refuses new entries in DRAWDOWN_STOP / SUSPENDED;
                        open positions are always managed (TP/SL/invalidation remain armed).
                    </p>
                </div>
                <div class={styles.statsGrid}>
                    <div class={styles.statCard}><div class={styles.statLabel}>Peak Equity</div><div class={styles.statValue}>${fmtUsd(safety?.peak_equity ?? portfolio.peak_equity)}</div><div class={styles.statSub}>high-water mark</div></div>
                    <div class={styles.statCard}><div class={styles.statLabel}>Max Drawdown</div><div class={styles.statValue} style="color:#ef5350">{fmtPct(safety?.max_drawdown_pct ?? portfolio.max_drawdown_pct)}</div><div class={styles.statSub}>from peak</div></div>
                    <div class={styles.statCard}><div class={styles.statLabel}>Daily PnL</div><div class="{styles.statValue} {pnlClass(safety?.daily_pnl ?? portfolio.daily_pnl)}">{signedUsd(safety?.daily_pnl ?? portfolio.daily_pnl)}</div><div class={styles.statSub}>session</div></div>
                </div>
                <h4 class={styles.subTitle}>Consecutive Losses</h4>
                {#if Object.keys(portfolio.consecutive_losses).length === 0}
                    <div class={styles.empty}>No losses recorded.</div>
                {:else}
                    <div class={styles.concList}>
                        {#each Object.entries(portfolio.consecutive_losses) as [sym, count] (sym)}
                            <div class={styles.concRow}>
                                <span class={styles.tdMono}>{sym}</span>
                                <span class="{styles.concPct} {count >= 5 ? styles.neg : count >= 3 ? styles.warn : ''}">{count} consecutive</span>
                            </div>
                        {/each}
                    </div>
                {/if}
                <div class={styles.sessionBar}>
                    <button class={styles.resetBtn} onclick={sessionReset} disabled={resetting}>
                        {resetting ? 'Resetting…' : 'Reset session (rebaseline peak + daily)'}
                    </button>
                </div>
            {/if}
        </div>
    </div>
</div>
