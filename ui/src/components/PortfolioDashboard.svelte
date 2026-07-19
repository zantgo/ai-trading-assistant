<script lang="ts">
    import styles from './PortfolioDashboard.module.css';

    type Panel = 'overview' | 'positions' | 'exposure' | 'capital' | 'safety';
    let activePanel = $state<Panel>('overview');
    let expandedPositions = $state<Set<number>>(new Set());

    function togglePosition(id: number) {
        const next = new Set(expandedPositions);
        if (next.has(id)) next.delete(id); else next.add(id);
        expandedPositions = next;
    }

    // ── Placeholder data ────────────────────────────────────────────────

    const safetyState: 'NORMAL' | 'WARN' | 'CAUTIOUS' | 'SUSPENDED' | 'DRAWDOWN_STOP' = 'NORMAL';

    const portfolioSummary = {
        current_equity: 10523.42,
        realized_pnl: 423.50,
        unrealized_pnl: 99.92,
        daily_pnl: 187.60,
        position_count: 1,
        margin_usage_ratio: 0.082,
        leverage_ratio: 0.78,
        gross_exposure: 8210.40,
        net_exposure: 8210.40,
        systemic_risk_score: 28.5,
        peak_equity: 10800.00,
        max_drawdown_pct: 2.56,
        initial_balance: 10000.00,
    };

    interface PortfolioPosition {
        id: number;
        position_id: string;
        symbol: string;
        direction: 'Long' | 'Short';
        entry_price: number;
        average_entry_price: number;
        size: number;
        allocated_usd: number;
        current_price: number;
        unrealized_pnl: number;
        roi_pct: number;
        stop_loss_price: number;
        take_profit_price: number;
        invalidation_level: number;
        target_profit_ratio: number;
        current_portions: number;
        max_portions: number;
        realized_pnl_accumulator: number;
    }

    interface ConcentrationEntry {
        symbol: string;
        exposure_pct: number;
        notional_usd: number;
    }

    interface StanceEntry {
        symbol: string;
        stance: 'ACTIVE' | 'CLOSE_ONLY' | 'AVOID';
        consecutive_losses: number;
    }

    interface VetoTrigger {
        trigger: string;
        target_stance: string;
        hard_exit: boolean;
        scope: string;
        threshold: string;
        current: string;
        active: boolean;
    }

    const positions: PortfolioPosition[] = [
        {
            id: 1, position_id: 'pos-btc-001', symbol: 'BTC-USDT', direction: 'Long',
            entry_price: 68420, average_entry_price: 68442, size: 0.12,
            allocated_usd: 8213.04, current_price: 69150,
            unrealized_pnl: 87.60, roi_pct: 1.07,
            stop_loss_price: 66150, take_profit_price: 72160,
            invalidation_level: 65800, target_profit_ratio: 2.5,
            current_portions: 1, max_portions: 4,
            realized_pnl_accumulator: 0,
        },
    ];

    const concentration: ConcentrationEntry[] = [
        { symbol: 'BTC-USDT', exposure_pct: 77.9, notional_usd: 8210.40 },
        { symbol: 'ETH-USDT', exposure_pct: 0, notional_usd: 0 },
        { symbol: 'SOL-USDT', exposure_pct: 0, notional_usd: 0 },
    ];

    const stances: StanceEntry[] = [
        { symbol: 'BTC-USDT', stance: 'ACTIVE', consecutive_losses: 0 },
        { symbol: 'ETH-USDT', stance: 'ACTIVE', consecutive_losses: 1 },
        { symbol: 'SOL-USDT', stance: 'CLOSE_ONLY', consecutive_losses: 4 },
        { symbol: 'DOGE-USDT', stance: 'AVOID', consecutive_losses: 6 },
    ];

    const vetoTriggers: VetoTrigger[] = [
        { trigger: 'Drawdown breach (30%)', target_stance: 'AVOID', hard_exit: true, scope: 'Platform-wide', threshold: '30%', current: '2.56%', active: false },
        { trigger: 'Margin ceiling (95%)', target_stance: 'CLOSE_ONLY', hard_exit: false, scope: 'Platform-wide', threshold: '95%', current: '8.2%', active: false },
        { trigger: 'Margin exhaustion (100%)', target_stance: 'AVOID', hard_exit: true, scope: 'Platform-wide', threshold: '100%', current: '8.2%', active: false },
        { trigger: 'Exposure limit breach', target_stance: 'CLOSE_ONLY', hard_exit: false, scope: 'Platform-wide', threshold: '50%', current: '78.8%', active: false },
        { trigger: 'Loss streak >= 5', target_stance: 'CLOSE_ONLY', hard_exit: false, scope: 'Per-symbol', threshold: '5', current: '4 (SOL)', active: false },
        { trigger: 'Systemic risk >= 80', target_stance: 'AVOID', hard_exit: true, scope: 'Platform-wide', threshold: '80', current: '28.5', active: false },
    ];

    const correlationMatrix = [
        { pair: 'BTC-ETH', value: 0.72 },
        { pair: 'BTC-SOL', value: 0.58 },
        { pair: 'ETH-SOL', value: 0.64 },
    ];

    function fmtNum(n: number, decimals: number = 2): string {
        if (!isFinite(n)) return '--';
        return n.toFixed(decimals);
    }

    function fmtPnl(n: number): string {
        const prefix = n >= 0 ? '+' : '';
        return prefix + fmtNum(n);
    }

    function fmtPct(n: number): string { return fmtNum(n) + '%'; }

    function pnlClass(n: number): string {
        if (n > 0) return styles.statPositive;
        if (n < 0) return styles.statNegative;
        return styles.statNeutral;
    }

    function stanceBadge(s: string): string {
        const m: Record<string, string> = { ACTIVE: 'badgeLong', CLOSE_ONLY: 'badgeShort', AVOID: 'badgeShort' };
        const styleMap: Record<string, string> = {
            badgeLong: styles.badge + ' ' + styles.badgeLong,
            badgeShort: styles.badge + ' ' + styles.badgeShort,
        };
        return styleMap[m[s]] || styleMap.badgeLong;
    }

    function safetyClass(): string {
        const m: Record<string, string> = {
            NORMAL: styles.safetyNormal, WARN: styles.safetyWarn,
            CAUTIOUS: styles.safetyCautious, SUSPENDED: styles.safetySuspended,
            DRAWDOWN_STOP: styles.safetyDrawdown,
        };
        return m[safetyState] || styles.safetyNormal;
    }

    function safetyBadgeClass(): string {
        const m: Record<string, string> = {
            NORMAL: styles.safetyBadgeNormal, WARN: styles.safetyBadgeWarn,
            CAUTIOUS: styles.safetyBadgeCautious, SUSPENDED: styles.safetyBadgeSuspended,
            DRAWDOWN_STOP: styles.safetyBadgeDrawdown,
        };
        return m[safetyState] || styles.safetyBadgeNormal;
    }

    function safetyIcon(): string {
        const m: Record<string, string> = {
            NORMAL: 'shield-check', WARN: 'alert-triangle',
            CAUTIOUS: 'alert-circle', SUSPENDED: 'x-circle',
            DRAWDOWN_STOP: 'zap',
        };
        return m[safetyState] || 'shield-check';
    }

    function gaugeColor(ratio: number): string {
        if (ratio >= 0.95) return styles.gaugeRed;
        if (ratio >= 0.80) return styles.gaugeOrange;
        return styles.gaugeGreen;
    }

    const gaugeColorName = $derived(gaugeColor(portfolioSummary.margin_usage_ratio));

    function correlationColor(v: number): string {
        const r = Math.abs(v);
        if (r >= 0.8) return 'background:#ef5350;';
        if (r >= 0.6) return 'background:#ffb74d;';
        if (r >= 0.4) return 'background:#8f929d;';
        return 'background:#1a1d26;';
    }

    function lossCountColor(n: number): string {
        if (n >= 5) return styles.statNegative;
        if (n >= 3) return styles.statNeutral;
        return styles.statPositive;
    }
</script>

<div class={styles.dashboard}>
    <div class={styles.sidebar}>
        <h2 class={styles.sidebarTitle}>PORTFOLIO</h2>
        <button class="{styles.sidebarBtn} {activePanel === 'overview' ? styles.sidebarBtnActive : ''}" onclick={() => activePanel = 'overview'}>Overview</button>
        <button class="{styles.sidebarBtn} {activePanel === 'positions' ? styles.sidebarBtnActive : ''}" onclick={() => activePanel = 'positions'}>Positions</button>
        <button class="{styles.sidebarBtn} {activePanel === 'exposure' ? styles.sidebarBtnActive : ''}" onclick={() => activePanel = 'exposure'}>Exposure</button>
        <button class="{styles.sidebarBtn} {activePanel === 'capital' ? styles.sidebarBtnActive : ''}" onclick={() => activePanel = 'capital'}>Capital</button>
        <button class="{styles.sidebarBtn} {activePanel === 'safety' ? styles.sidebarBtnActive : ''}" onclick={() => activePanel = 'safety'}>Safety</button>
    </div>

    <div class={styles.content}>
        <!-- ─── OVERVIEW ─────────────────────────────────────────── -->
        {#if activePanel === 'overview'}
            <h3 class={styles.sectionTitle}>Portfolio Management</h3>
            <p class={styles.sectionDesc}>
                Capital custodian and safety authority — tracks positions, aggregates exposure,
                manages capital and margin, and enforces systemic safety veto.
            </p>

            <div class="{styles.safetyBanner} {safetyClass()}">
                <span class={styles.safetyIcon}>{safetyIcon()}</span>
                <div style="flex:1">
                    <strong>Safety State: {safetyState.replace('_', ' ')}</strong>
                    {#if safetyState === 'NORMAL'}
                        <span style="margin-left:0.5rem; font-weight:400; font-size:0.75rem">Full trading authorized across all symbols</span>
                    {:else if safetyState === 'WARN'}
                        <span style="margin-left:0.5rem; font-weight:400; font-size:0.75rem">Early warning — daily drawdown threshold exceeded. No stance changes.</span>
                    {:else if safetyState === 'CAUTIOUS'}
                        <span style="margin-left:0.5rem; font-weight:400; font-size:0.75rem">Consecutive losses detected — monitor closely</span>
                    {:else if safetyState === 'SUSPENDED'}
                        <span style="margin-left:0.5rem; font-weight:400; font-size:0.75rem">CLOSE_ONLY for affected symbol — 8h cooldown</span>
                    {:else}
                        <span style="margin-left:0.5rem; font-weight:400; font-size:0.75rem">All stances → AVOID — Hard Exit active</span>
                    {/if}
                </div>
                <span class="{styles.safetyBadge} {safetyBadgeClass()}">{safetyState.replace('_', ' ')}</span>
            </div>

            <div class={styles.statsGrid}>
                <div class={styles.statCard}>
                    <div class={styles.statLabel}>Current Equity</div>
                    <div class={styles.statValue}>${fmtNum(portfolioSummary.current_equity)}</div>
                    <div class="statSub">Initial: ${fmtNum(portfolioSummary.initial_balance)}</div>
                </div>
                <div class={styles.statCard}>
                    <div class={styles.statLabel}>Realized P&L</div>
                    <div class="{styles.statValue} {pnlClass(portfolioSummary.realized_pnl)}">{fmtPnl(portfolioSummary.realized_pnl)}</div>
                </div>
                <div class={styles.statCard}>
                    <div class={styles.statLabel}>Unrealized P&L</div>
                    <div class="{styles.statValue} {pnlClass(portfolioSummary.unrealized_pnl)}">{fmtPnl(portfolioSummary.unrealized_pnl)}</div>
                </div>
                <div class={styles.statCard}>
                    <div class={styles.statLabel}>Daily P&L</div>
                    <div class="{styles.statValue} {pnlClass(portfolioSummary.daily_pnl)}">{fmtPnl(portfolioSummary.daily_pnl)}</div>
                </div>
                <div class={styles.statCard}>
                    <div class={styles.statLabel}>Active Positions</div>
                    <div class={styles.statValue}>{portfolioSummary.position_count}</div>
                </div>
                <div class={styles.statCard}>
                    <div class={styles.statLabel}>Margin Usage</div>
                    <div class={styles.statValue}>{fmtPct(portfolioSummary.margin_usage_ratio * 100)}</div>
                    <div class={styles.statSub}>of equity committed</div>
                </div>
                <div class={styles.statCard}>
                    <div class={styles.statLabel}>Leverage Ratio</div>
                    <div class={styles.statValue}>{fmtNum(portfolioSummary.leverage_ratio)}x</div>
                    <div class={styles.statSub}>gross / equity</div>
                </div>
                <div class={styles.statCard}>
                    <div class={styles.statLabel}>Systemic Risk</div>
                    <div class="{styles.statValue} {pnlClass(50 - portfolioSummary.systemic_risk_score)}">{fmtNum(portfolioSummary.systemic_risk_score, 0)}</div>
                    <div class={styles.statSub}>from Overview Matrix</div>
                </div>
            </div>

            <div class={styles.grid2}>
                <div>
                    <h4 style="font-size:0.75rem; color:#5a5f6e; text-transform:uppercase; letter-spacing:0.05em; margin-bottom:0.5rem">Equity Composition</h4>
                    <table class={styles.table}>
                        <tbody>
                            <tr><td>Initial Balance</td><td class={styles.tdRight}>${fmtNum(portfolioSummary.initial_balance)}</td></tr>
                            <tr><td>Realized P&L</td><td class="{styles.tdRight} {pnlClass(portfolioSummary.realized_pnl)}">{fmtPnl(portfolioSummary.realized_pnl)}</td></tr>
                            <tr><td>Unrealized P&L</td><td class="{styles.tdRight} {pnlClass(portfolioSummary.unrealized_pnl)}">{fmtPnl(portfolioSummary.unrealized_pnl)}</td></tr>
                            <tr style="border-top:1px solid #2a2e39"><td style="font-weight:600">Current Equity</td><td class="{styles.tdRight} {pnlClass(portfolioSummary.current_equity - portfolioSummary.initial_balance)}">${fmtNum(portfolioSummary.current_equity)}</td></tr>
                        </tbody>
                    </table>
                </div>
                <div>
                    <h4 style="font-size:0.75rem; color:#5a5f6e; text-transform:uppercase; letter-spacing:0.05em; margin-bottom:0.5rem">Capital Matrix (Canonical)</h4>
                    <div style="font-size:0.72rem; color:#888; font-family:var(--mono); line-height:1.8">
                        <div>equity = initial + realized + min(0, unrealized)</div>
                        <div style="color:#ccc">$10,523.42 = $10,000 + $423.50 + $99.92</div>
                        <div style="margin-top:0.5rem">available = equity - committed_margin</div>
                        <div style="color:#ccc">$9,702.38 = $10,523.42 - $821.04</div>
                        <div style="margin-top:0.5rem">margin_usage = committed / equity</div>
                        <div style="color:#4caf50">8.2% — Healthy</div>
                    </div>
                </div>
            </div>

        <!-- ─── POSITIONS ─────────────────────────────────────────── -->
        {:else if activePanel === 'positions'}
            <h3 class={styles.sectionTitle}>Position Matrix</h3>
            <p class={styles.sectionDesc}>
                Live position tracker: entry prices, scaled entries (1–4 slots),
                unrealized P&L, stop-loss and take-profit levels, and thesis invalidation points.
            </p>

            {#if positions.length === 0}
                <div class={styles.placeholder}>No active positions</div>
            {:else}
                {#each positions as pos (pos.id)}
                    <div class={styles.positionCard}>
                        <div class={styles.positionHeader} role="button" tabindex="0" onclick={() => togglePosition(pos.id)} onkeydown={(e) => e.key === 'Enter' && togglePosition(pos.id)}>
                            <span class={styles.positionSymbol}>{pos.symbol}</span>
                            <span class="{styles.positionDirection} {pos.direction === 'Long' ? styles.statPositive : styles.statNegative}">{pos.direction}</span>
                            <span style="font-size:0.72rem; color:#888">{pos.size} units · ${fmtNum(pos.allocated_usd)} allocated</span>
                            <span class="{styles.positionPnl} {pnlClass(pos.unrealized_pnl)}">{fmtPnl(pos.unrealized_pnl)} ({fmtPct(pos.roi_pct)})</span>
                            <div class={styles.positionMeta}>
                                <div class={styles.slotsIndicator}>
                                    {#each Array(pos.max_portions) as _, i}
                                        <div class="{styles.slotDot} {i < pos.current_portions ? styles.slotDotActive : styles.slotDotVacant}"></div>
                                    {/each}
                                </div>
                                <span>{pos.current_portions}/{pos.max_portions} slots</span>
                            </div>
                            <span class="{styles.expandIcon} {expandedPositions.has(pos.id) ? styles.expandIconOpen : ''}">▶</span>
                        </div>

                        {#if expandedPositions.has(pos.id)}
                            <div class={styles.positionDetail}>
                                <div class={styles.positionDetailGrid}>
                                    <div class={styles.positionField}>
                                        <span class={styles.positionFieldLabel}>Entry Price</span>
                                        <span class={styles.positionFieldValue}>${fmtNum(pos.entry_price)}</span>
                                    </div>
                                    <div class={styles.positionField}>
                                        <span class={styles.positionFieldLabel}>Avg Entry (VWAP)</span>
                                        <span class={styles.positionFieldValue}>${fmtNum(pos.average_entry_price)}</span>
                                    </div>
                                    <div class={styles.positionField}>
                                        <span class={styles.positionFieldLabel}>Current Price</span>
                                        <span class={styles.positionFieldValue}>${fmtNum(pos.current_price)}</span>
                                    </div>
                                    <div class={styles.positionField}>
                                        <span class={styles.positionFieldLabel}>Allocated USD</span>
                                        <span class={styles.positionFieldValue}>${fmtNum(pos.allocated_usd)}</span>
                                    </div>
                                    <div class={styles.positionField}>
                                        <span class={styles.positionFieldLabel}>Stop-Loss Price</span>
                                        <span class={styles.positionFieldValue} style="color:#ef5350">${fmtNum(pos.stop_loss_price)}</span>
                                    </div>
                                    <div class={styles.positionField}>
                                        <span class={styles.positionFieldLabel}>Take-Profit Price</span>
                                        <span class={styles.positionFieldValue} style="color:#4caf50">${fmtNum(pos.take_profit_price)}</span>
                                    </div>
                                    <div class={styles.positionField}>
                                        <span class={styles.positionFieldLabel}>Invalidation Level</span>
                                        <span class={styles.positionFieldValue} style="color:#ffb74d">${fmtNum(pos.invalidation_level)}</span>
                                    </div>
                                    <div class={styles.positionField}>
                                        <span class={styles.positionFieldLabel}>Target R:R</span>
                                        <span class={styles.positionFieldValue}>{fmtNum(pos.target_profit_ratio)}</span>
                                    </div>
                                    <div class={styles.positionField}>
                                        <span class={styles.positionFieldLabel}>Realized P&L (Scale-out)</span>
                                        <span class="{styles.positionFieldValue} {pnlClass(pos.realized_pnl_accumulator)}">{fmtPnl(pos.realized_pnl_accumulator)}</span>
                                    </div>
                                    <div class={styles.positionField}>
                                        <span class={styles.positionFieldLabel}>Slot Fill</span>
                                        <span class={styles.positionFieldValue}>{pos.current_portions}/{pos.max_portions} active</span>
                                    </div>
                                    <div class={styles.positionField}>
                                        <span class={styles.positionFieldLabel}>Unrealized P&L</span>
                                        <span class="{styles.positionFieldValue} {pnlClass(pos.unrealized_pnl)}">{fmtPnl(pos.unrealized_pnl)}</span>
                                    </div>
                                    <div class={styles.positionField}>
                                        <span class={styles.positionFieldLabel}>ROI %</span>
                                        <span class="{styles.positionFieldValue} {pnlClass(pos.roi_pct)}">{fmtPct(pos.roi_pct)}</span>
                                    </div>
                                </div>
                            </div>
                        {/if}
                    </div>
                {/each}
            {/if}

        <!-- ─── EXPOSURE ──────────────────────────────────────────── -->
        {:else if activePanel === 'exposure'}
            <h3 class={styles.sectionTitle}>Exposure Matrix</h3>
            <p class={styles.sectionDesc}>
                Aggregate exposure breakdown, concentration analysis, and cross-symbol correlation.
                Concentration limits: max single-pair 20%, max portfolio 50%, max correlation 0.8.
            </p>

            <h4 style="font-size:0.75rem; color:#5a5f6e; text-transform:uppercase; letter-spacing:0.05em; margin-bottom:0.5rem">Exposure Summary</h4>
            <div class={styles.statsGrid}>
                <div class={styles.statCard}>
                    <div class={styles.statLabel}>Gross Exposure</div>
                    <div class={styles.statValue}>${fmtNum(portfolioSummary.gross_exposure)}</div>
                    <div class={styles.statSub}>Total absolute notional</div>
                </div>
                <div class={styles.statCard}>
                    <div class={styles.statLabel}>Net Exposure</div>
                    <div class={styles.statValue}>${fmtNum(portfolioSummary.net_exposure)}</div>
                    <div class={styles.statSub}>{fmtPct(portfolioSummary.net_exposure / portfolioSummary.current_equity * 100)} of equity</div>
                </div>
                <div class={styles.statCard}>
                    <div class={styles.statLabel}>Long Exposure</div>
                    <div class="{styles.statValue} {styles.statPositive}">$8,210.40</div>
                </div>
                <div class={styles.statCard}>
                    <div class={styles.statLabel}>Short Exposure</div>
                    <div class={styles.statValue} style="color:#5a5f6e">$0.00</div>
                </div>
            </div>

            <div style="margin-top:0.25rem; margin-bottom:1.5rem">
                <div class={styles.exposureRow}>
                    <span class={styles.exposureLabel}>Long</span>
                    <div class={styles.exposureBar}>
                        <div class="{styles.exposureFill} {styles.exposureFillLong}" style="width:77.9%"></div>
                    </div>
                    <span class={styles.exposureValue}>$8,210.40 (77.9%)</span>
                </div>
                <div class={styles.exposureRow}>
                    <span class={styles.exposureLabel}>Short</span>
                    <div class={styles.exposureBar}>
                        <div class="{styles.exposureFill} {styles.exposureFillShort}" style="width:0%"></div>
                    </div>
                    <span class={styles.exposureValue}>$0.00 (0%)</span>
                </div>
                <div class={styles.exposureRow}>
                    <span class={styles.exposureLabel}>Net</span>
                    <div class={styles.exposureBar}>
                        <div class="{styles.exposureFill} {styles.exposureFillNet}" style="width:77.9%"></div>
                    </div>
                    <span class={styles.exposureValue}>$8,210.40 (77.9%)</span>
                </div>
            </div>

            <h4 style="font-size:0.75rem; color:#5a5f6e; text-transform:uppercase; letter-spacing:0.05em; margin-bottom:0.5rem">Symbol Concentration</h4>
            {#each concentration as c}
                <div class={styles.concentrationCard}>
                    <div class={styles.concentrationHeader}>
                        <span class={styles.concentrationSymbol}>{c.symbol}</span>
                        <span class="{styles.concentrationPct} {c.exposure_pct > 20 ? styles.statNegative : pnlClass(0)}">
                            {fmtPct(c.exposure_pct)} of equity
                        </span>
                    </div>
                    <div style="position:relative">
                        <div class={styles.concentrationLimitBar}>
                            <div class={styles.concentrationLimitFill}
                                 style="width:{Math.min(c.exposure_pct, 100)}%; background:{c.exposure_pct > 20 ? '#ef5350' : c.exposure_pct > 10 ? '#ffb74d' : '#4caf50'}">
                            </div>
                        </div>
                        <div style="position:absolute; top:-2px; left:20%; height:10px; width:1px; background:#ef5350"></div>
                    </div>
                    <div class={styles.concentrationLimitLabel}>Limit: 20%</div>
                </div>
            {/each}

            <h4 style="font-size:0.75rem; color:#5a5f6e; text-transform:uppercase; letter-spacing:0.05em; margin:1rem 0 0.5rem">Cross-Symbol Correlation</h4>
            <div style="display:flex; flex-direction:column; gap:0.25rem">
                {#each correlationMatrix as corr}
                    <div style="display:flex; align-items:center; gap:0.5rem; font-size:0.72rem">
                        <span style="width:70px; color:#888">{corr.pair}</span>
                        <div style="flex:1; height:8px; border-radius:4px; background:#1a1d26; overflow:hidden">
                            <div style="height:100%; border-radius:4px; width:{Math.abs(corr.value) * 100}%; background:{corr.value > 0.8 ? '#ef5350' : corr.value > 0.6 ? '#ffb74d' : '#8f929d'}"></div>
                        </div>
                        <span style="{corr.value > 0.8 ? 'color:#ef5350' : corr.value > 0.6 ? 'color:#ffb74d' : 'color:#8f929d'}; font-variant-numeric:tabular-nums; width:35px">{fmtNum(corr.value)}</span>
                    </div>
                {/each}
            </div>
            <div style="font-size:0.6rem; color:#5a5f6e; margin-top:0.3rem">Max correlation limit: 0.8</div>

        <!-- ─── CAPITAL ────────────────────────────────────────────── -->
        {:else if activePanel === 'capital'}
            <h3 class={styles.sectionTitle}>Capital Matrix</h3>
            <p class={styles.sectionDesc}>
                Capital custodian: tracks equity, margin, leverage, and enforces
                liquidation risk thresholds with automated stance adjustments.
            </p>

            <div class={styles.statsGrid}>
                <div class={styles.statCard}>
                    <div class={styles.statLabel}>Initial Balance</div>
                    <div class={styles.statValue}>${fmtNum(portfolioSummary.initial_balance)}</div>
                </div>
                <div class={styles.statCard}>
                    <div class={styles.statLabel}>Current Equity</div>
                    <div class={styles.statValue}>${fmtNum(portfolioSummary.current_equity)}</div>
                    <div class={styles.statSub}>+{fmtNum((portfolioSummary.current_equity / portfolioSummary.initial_balance - 1) * 100)}%</div>
                </div>
                <div class={styles.statCard}>
                    <div class={styles.statLabel}>Available Margin</div>
                    <div class={styles.statValue} style="color:#4caf50">${fmtNum(portfolioSummary.current_equity - 821.04)}</div>
                </div>
                <div class={styles.statCard}>
                    <div class={styles.statLabel}>Committed Margin</div>
                    <div class={styles.statValue}>$821.04</div>
                </div>
                <div class={styles.statCard}>
                    <div class={styles.statLabel}>Realized P&L</div>
                    <div class="{styles.statValue} {pnlClass(portfolioSummary.realized_pnl)}">{fmtPnl(portfolioSummary.realized_pnl)}</div>
                </div>
                <div class={styles.statCard}>
                    <div class={styles.statLabel}>Unrealized P&L</div>
                    <div class="{styles.statValue} {pnlClass(portfolioSummary.unrealized_pnl)}">{fmtPnl(portfolioSummary.unrealized_pnl)}</div>
                </div>
                <div class={styles.statCard}>
                    <div class={styles.statLabel}>Daily P&L</div>
                    <div class="{styles.statValue} {pnlClass(portfolioSummary.daily_pnl)}">{fmtPnl(portfolioSummary.daily_pnl)}</div>
                </div>
                <div class={styles.statCard}>
                    <div class={styles.statLabel}>Session Start Equity</div>
                    <div class={styles.statValue}>${fmtNum(portfolioSummary.initial_balance)}</div>
                </div>
            </div>

            <h4 style="font-size:0.75rem; color:#5a5f6e; text-transform:uppercase; letter-spacing:0.05em; margin-bottom:0.5rem">Margin Usage Ratio</h4>
            <div class={styles.capitalGauge}>
                <div class={styles.capitalGaugeHeader}>
                    <span class={styles.capitalGaugeLabel}>{fmtPct(portfolioSummary.margin_usage_ratio * 100)}</span>
                </div>
                <div class={styles.capitalGaugeBar}>
                    <div class="{styles.capitalGaugeFill} {gaugeColorName}" style="width:{Math.min(portfolioSummary.margin_usage_ratio * 100, 100)}%"></div>
                </div>
                <div class={styles.capitalGaugeMarkers}>
                    <span>0%</span>
                    <span style="position:absolute; left:80%; color:#ffb74d">WARN 80%</span>
                    <span style="position:absolute; left:95%; color:#ef5350">CLOSE 95%</span>
                </div>
            </div>

            <h4 style="font-size:0.75rem; color:#5a5f6e; text-transform:uppercase; letter-spacing:0.05em; margin:1rem 0 0.5rem">Liquidation Risk Thresholds</h4>
            <table class={styles.table}>
                <thead><tr><th>Threshold</th><th>Action</th><th>Current</th><th>Status</th></tr></thead>
                <tbody>
                    <tr>
                        <td class={styles.tdMono}>margin_usage >= 0.80</td>
                        <td>Warning to Portfolio Layer</td>
                        <td class={styles.tdRight}>8.2%</td>
                        <td class={styles.statPositive}>OK</td>
                    </tr>
                    <tr>
                        <td class={styles.tdMono}>margin_usage >= 0.95</td>
                        <td>CLOSE_ONLY for all symbols</td>
                        <td class={styles.tdRight}>8.2%</td>
                        <td class={styles.statPositive}>OK</td>
                    </tr>
                    <tr>
                        <td class={styles.tdMono}>margin_usage >= 1.00</td>
                        <td style="color:#ef5350">AVOID + Hard Exit</td>
                        <td class={styles.tdRight}>8.2%</td>
                        <td class={styles.statPositive}>OK</td>
                    </tr>
                </tbody>
            </table>

            <div style="margin-top:1rem">
                <h4 style="font-size:0.75rem; color:#5a5f6e; text-transform:uppercase; letter-spacing:0.05em; margin-bottom:0.5rem">Leverage Ratio</h4>
                <div style="display:flex; gap:0.75rem; align-items:center">
                    <span style="font-size:1.5rem; font-weight:700; font-variant-numeric:tabular-nums">{fmtNum(portfolioSummary.leverage_ratio)}x</span>
                    <span style="font-size:0.72rem; color:#888">gross_exposure / current_equity</span>
                </div>
            </div>

        <!-- ─── SAFETY ─────────────────────────────────────────────── -->
        {:else if activePanel === 'safety'}
            <h3 class={styles.sectionTitle}>Safety Authority</h3>
            <p class={styles.sectionDesc}>
                Systemic safety manager: risk-ranked circuit breakers, per-symbol stances,
                consecutive loss dropout, drawdown enforcement, and emergency Hard Exit.
            </p>

            <div class="{styles.safetyBanner} {safetyClass()}">
                <span class={styles.safetyIcon}>{safetyIcon()}</span>
                <div style="flex:1">
                    <strong>Current Safety State: {safetyState.replace('_', ' ')}</strong>
                    <span style="margin-left:0.5rem; font-weight:400; font-size:0.75rem">
                        {#if safetyState === 'NORMAL'}All systems clear — full trading authorized{:else if safetyState === 'WARN'}Monitor drawdown — dawn warning active{:else}Restrictions active — see triggers below{/if}
                    </span>
                </div>
                <span class="{styles.safetyBadge} {safetyBadgeClass()}">{safetyState.replace('_', ' ')}</span>
            </div>

            <h4 style="font-size:0.75rem; color:#5a5f6e; text-transform:uppercase; letter-spacing:0.05em; margin-bottom:0.5rem">Per-Symbol Stances & Loss Streaks</h4>
            <div class={styles.statsGrid}>
                {#each stances as s}
                    <div class={styles.statCard}>
                        <div class={styles.statLabel}>{s.symbol}</div>
                        <div class={styles.statValue}>
                            <span class={stanceBadge(s.stance)}>{s.stance.replace('_', ' ')}</span>
                        </div>
                        <div class="{styles.statSub} {lossCountColor(s.consecutive_losses)}">
                            {s.consecutive_losses} consecutive losses
                        </div>
                    </div>
                {/each}
            </div>

            <h4 style="font-size:0.75rem; color:#5a5f6e; text-transform:uppercase; letter-spacing:0.05em; margin-bottom:0.5rem">Consecutive Loss Tracker</h4>
            {#each stances.filter(s => s.consecutive_losses > 0) as s}
                <div class={styles.lossCard}>
                    <div class={styles.lossHeader}>
                        <span class={styles.lossSymbol}>{s.symbol}</span>
                        <span class="{styles.lossCount} {lossCountColor(s.consecutive_losses)}">{s.consecutive_losses} / 5</span>
                    </div>
                    <div style="position:relative">
                        <div class={styles.lossBar}>
                            <div class={styles.lossBarFill} style="width:{(s.consecutive_losses / 5) * 100}%; background:{s.consecutive_losses >= 5 ? '#ef5350' : s.consecutive_losses >= 3 ? '#ffb74d' : '#4caf50'}"></div>
                        </div>
                        <div class="{styles.lossThreshold} {styles.lossThresholdCautious}"></div>
                        <div class="{styles.lossThreshold} {styles.lossThresholdSuspended}"></div>
                    </div>
                    <div class={styles.lossLegend}>
                        <span>0</span>
                        <span style="color:#ffb74d">CAUTIOUS (3)</span>
                        <span style="color:#ef5350">SUSPENDED (5)</span>
                    </div>
                </div>
            {/each}

            {#if stances.every(s => s.consecutive_losses === 0)}
                <div class={styles.placeholder} style="padding:1rem; font-size:0.78rem">No consecutive losses — all symbols healthy</div>
            {/if}

            <h4 style="font-size:0.75rem; color:#5a5f6e; text-transform:uppercase; letter-spacing:0.05em; margin:1rem 0 0.5rem">Drawdown Monitor</h4>
            <div class={styles.drawdownCard}>
                <div class={styles.drawdownStats}>
                    <div class={styles.drawdownStat}>
                        <span class={styles.drawdownStatLabel}>Peak Equity</span>
                        <span class={styles.drawdownStatValue}>${fmtNum(portfolioSummary.peak_equity)}</span>
                    </div>
                    <div class={styles.drawdownStat}>
                        <span class={styles.drawdownStatLabel}>Current Drawdown</span>
                        <span class="{styles.drawdownStatValue} {pnlClass(-portfolioSummary.max_drawdown_pct)}">{fmtPct(portfolioSummary.max_drawdown_pct)}</span>
                    </div>
                    <div class={styles.drawdownStat}>
                        <span class={styles.drawdownStatLabel}>Drawdown Limit</span>
                        <span class={styles.drawdownStatValue} style="color:#ef5350">30%</span>
                    </div>
                </div>
                <div style="position:relative">
                    <div class={styles.drawdownBar}>
                        <div class={styles.drawdownBarFill} style="width:{(portfolioSummary.max_drawdown_pct / 30) * 100}%; background:{portfolioSummary.max_drawdown_pct > 20 ? '#ef5350' : portfolioSummary.max_drawdown_pct > 10 ? '#ffb74d' : '#4caf50'}"></div>
                    </div>
                    <div style="position:absolute; top:-3px; left:100%; height:14px; width:1px; background:#ef5350"></div>
                </div>
            </div>

            <h4 style="font-size:0.75rem; color:#5a5f6e; text-transform:uppercase; letter-spacing:0.05em; margin:1rem 0 0.5rem">Veto Trigger Reference</h4>
            <p class={styles.sectionDesc} style="margin-bottom:0.5rem">
                Pre-trade safety veto chain. Triggers evaluated in order — first match wins.
            </p>
            {#each vetoTriggers as v}
                <div class={styles.vetoCard}>
                    <div class={styles.vetoHeader}>
                        <span class={styles.vetoTrigger}>{v.trigger}</span>
                        <span class={styles.vetoArrow}>→</span>
                        <span class="{styles.vetoResult} {v.target_stance === 'AVOID' ? styles.statNegative : styles.statNeutral}">{v.target_stance}</span>
                        {#if v.hard_exit}
                            <span class={styles.vetoHardExit}>HARD EXIT</span>
                        {/if}
                    </div>
                    <div class={styles.vetoMeta}>
                        <span>Scope: {v.scope}</span>
                        <span>Threshold: {v.threshold}</span>
                        <span class={v.active ? styles.statNegative : styles.statPositive}>
                            Current: {v.current} — {v.active ? 'BREACHED' : 'OK'}
                        </span>
                    </div>
                </div>
            {/each}
        {/if}
    </div>
</div>
