<script lang="ts">
    import styles from './TradeAutomationDashboard.module.css';

    type Panel = 'overview' | 'policies' | 'observability' | 'paper' | 'lifecycle';
    let activePanel = $state<Panel>('overview');
    let expandedPolicies = $state<Set<number>>(new Set());
    let paperTab = $state<'positions' | 'orders' | 'history'>('positions');

    function togglePolicy(id: number) {
        const next = new Set(expandedPolicies);
        if (next.has(id)) next.delete(id); else next.add(id);
        expandedPolicies = next;
    }

    // ── Placeholder data ────────────────────────────────────────────────
    const operationalMode = 'PaperTrading';

    const overviewStats = {
        active_policies: 3,
        triggered_today: 12,
        blocked_today: 3,
        active_positions: 1,
        lifecycle_running: 4,
        lifecycle_paused: 0,
        lifecycle_stopped: 1,
    };

    interface PolicyCondition {
        field: string;
        operator: 'EQ' | 'GT' | 'LT' | 'GTE' | 'LTE' | 'IN' | 'BETWEEN' | 'NOT_EQ';
        value: string;
        passed?: boolean;
    }

    interface ConditionGroup {
        logic: 'AND' | 'OR';
        conditions: PolicyCondition[];
    }

    interface ExecutionPolicy {
        id: number;
        policy_id: string;
        policy_name: string;
        description: string;
        symbol: string;
        direction: 'Long' | 'Short';
        enabled: boolean;
        stance: 'ACTIVE' | 'CLOSE_ONLY' | 'AVOID';
        trigger_mode: string;
        condition_tree: ConditionGroup[];
        risk: { risk_per_trade_pct: number; max_position_size_usd: number; max_leverage: number; use_dynamic_stops: boolean; fixed_stop_loss_pct: number | null; target_rr_ratio: number };
        cooldown_seconds: number;
    }

    interface ObservableTrigger {
        policy_id: string;
        trigger_timestamp: number;
        result: 'TRIGGERED' | 'BLOCKED_COOLDOWN' | 'BLOCKED_CONFLICT' | 'SKIPPED_STANCE';
        decision_snapshot: { bias: string; directional_guidance: string; confidence: number };
        conditions_evaluated: { field: string; passed: boolean }[];
    }

    interface LifecycleInstance {
        id: string;
        symbol: string;
        state: 'RUNNING' | 'PAUSED' | 'STOPPING' | 'STOPPED';
        stance: 'ACTIVE' | 'CLOSE_ONLY' | 'AVOID';
        start_condition?: string;
        pause_condition?: string;
        stop_condition?: string;
        automation_summary: string;
    }

    const policies: ExecutionPolicy[] = [
        {
            id: 1, policy_id: 'btc-trend-follow', policy_name: 'BTC Trend Following',
            description: 'Long BTC when strong bullish bias and trending bull regime with low risk',
            symbol: 'BTC-USDT', direction: 'Long', enabled: true, stance: 'ACTIVE',
            trigger_mode: 'CandleClose { timeframe: "1h", count: 1 }',
            condition_tree: [
                { logic: 'AND', conditions: [
                    { field: 'decision.bias', operator: 'IN', value: '["StrongBullish","Bullish"]', passed: true },
                    { field: 'analysis.market_regime', operator: 'IN', value: '["TRENDING_BULL","ACCUMULATION"]', passed: true },
                    { field: 'decision.confidence_assessment', operator: 'GTE', value: '0.6', passed: true },
                ]},
                { logic: 'AND', conditions: [
                    { field: 'risk.overall_risk.score', operator: 'LTE', value: '40', passed: false },
                    { field: 'opportunity.opportunity_score', operator: 'GTE', value: '50', passed: true },
                ]},
            ],
            risk: { risk_per_trade_pct: 1.0, max_position_size_usd: 50000, max_leverage: 10, use_dynamic_stops: true, fixed_stop_loss_pct: null, target_rr_ratio: 2.5 },
            cooldown_seconds: 300,
        },
        {
            id: 2, policy_id: 'eth-mean-reversion', policy_name: 'ETH Mean Reversion',
            description: 'Short ETH when RSI overbought in ranging market with high volatility compression',
            symbol: 'ETH-USDT', direction: 'Short', enabled: true, stance: 'ACTIVE',
            trigger_mode: 'Interval { seconds: 60 }',
            condition_tree: [
                { logic: 'AND', conditions: [
                    { field: 'analysis.market_regime', operator: 'EQ', value: '"RANGE"', passed: true },
                    { field: 'decision.strategy_environment', operator: 'EQ', value: '"MeanReversion"', passed: true },
                    { field: 'risk.volatility_risk.score', operator: 'LTE', value: '30', passed: false },
                ]},
            ],
            risk: { risk_per_trade_pct: 0.5, max_position_size_usd: 25000, max_leverage: 5, use_dynamic_stops: true, fixed_stop_loss_pct: 1.5, target_rr_ratio: 2.0 },
            cooldown_seconds: 900,
        },
        {
            id: 3, policy_id: 'sol-breakout', policy_name: 'SOL Breakout Strategy',
            description: 'Long SOL on breakout confirmation in expansion regime with momentum strength',
            symbol: 'SOL-USDT', direction: 'Long', enabled: false, stance: 'CLOSE_ONLY',
            trigger_mode: 'EventDriven { events: ["BREAKOUT_CONFIRMED","SQUEEZE_RELEASE"] }',
            condition_tree: [
                { logic: 'AND', conditions: [
                    { field: 'analysis.market_regime', operator: 'EQ', value: '"EXPANSION"', passed: false },
                    { field: 'decision.directional_guidance', operator: 'IN', value: '["StrongLong","Long"]', passed: true },
                    { field: 'risk.momentum_risk.score', operator: 'LTE', value: '35', passed: false },
                ]},
            ],
            risk: { risk_per_trade_pct: 1.5, max_position_size_usd: 30000, max_leverage: 8, use_dynamic_stops: false, fixed_stop_loss_pct: 2.0, target_rr_ratio: 3.0 },
            cooldown_seconds: 600,
        },
    ];

    const observability: ObservableTrigger[] = [
        { policy_id: 'btc-trend-follow', trigger_timestamp: Date.now() - 120000, result: 'TRIGGERED', decision_snapshot: { bias: 'Bullish', directional_guidance: 'Long', confidence: 0.72 }, conditions_evaluated: [{ field: 'decision.bias', passed: true }, { field: 'analysis.market_regime', passed: true }, { field: 'risk.overall_risk.score', passed: true }, { field: 'decision.confidence_assessment', passed: true }] },
        { policy_id: 'btc-trend-follow', trigger_timestamp: Date.now() - 360000, result: 'BLOCKED_COOLDOWN', decision_snapshot: { bias: 'Bullish', directional_guidance: 'Long', confidence: 0.68 }, conditions_evaluated: [{ field: 'decision.bias', passed: true }, { field: 'analysis.market_regime', passed: true }, { field: 'risk.overall_risk.score', passed: false }] },
        { policy_id: 'eth-mean-reversion', trigger_timestamp: Date.now() - 600000, result: 'TRIGGERED', decision_snapshot: { bias: 'Bearish', directional_guidance: 'Short', confidence: 0.55 }, conditions_evaluated: [{ field: 'analysis.market_regime', passed: true }, { field: 'decision.strategy_environment', passed: true }, { field: 'risk.volatility_risk.score', passed: true }] },
        { policy_id: 'sol-breakout', trigger_timestamp: Date.now() - 900000, result: 'SKIPPED_STANCE', decision_snapshot: { bias: 'StrongBullish', directional_guidance: 'StrongLong', confidence: 0.81 }, conditions_evaluated: [{ field: 'analysis.market_regime', passed: true }, { field: 'decision.directional_guidance', passed: true }] },
        { policy_id: 'btc-trend-follow', trigger_timestamp: Date.now() - 1800000, result: 'BLOCKED_CONFLICT', decision_snapshot: { bias: 'Bullish', directional_guidance: 'Neutral', confidence: 0.45 }, conditions_evaluated: [{ field: 'decision.bias', passed: true }, { field: 'analysis.market_regime', passed: false }] },
    ];

    const lifecycleInstances: LifecycleInstance[] = [
        { id: 'inst-btc', symbol: 'BTC-USDT', state: 'RUNNING', stance: 'ACTIVE', start_condition: 'at_time 00:00 UTC', pause_condition: 'at_price_below $62,000', stop_condition: 'after_duration 24h', automation_summary: 'Auto-start at midnight UTC · Pause below $62k · Stop after 24h' },
        { id: 'inst-eth', symbol: 'ETH-USDT', state: 'RUNNING', stance: 'ACTIVE', start_condition: 'manual', pause_condition: 'at_price_below $2,800', automation_summary: 'Manual start · Pause below $2,800' },
        { id: 'inst-sol', symbol: 'SOL-USDT', state: 'PAUSED', stance: 'CLOSE_ONLY', stop_condition: 'at_price_below $120', automation_summary: 'Paused · CLOSE_ONLY — entries blocked, exits permitted' },
        { id: 'inst-doge', symbol: 'DOGE-USDT', state: 'RUNNING', stance: 'ACTIVE', start_condition: 'at_time 08:00 UTC', pause_condition: 'at_price_above $0.25', automation_summary: 'Auto-start at 8am UTC · Pause above $0.25' },
        { id: 'inst-avax', symbol: 'AVAX-USDT', state: 'STOPPED', stance: 'AVOID', stop_condition: 'manual', automation_summary: 'Stopped · AVOID — all execution triggers blocked' },
    ];

    function fmtTs(ts: number): string {
        const d = new Date(ts);
        return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
    }

    function fmtNum(n: number, decimals: number = 2): string {
        if (!isFinite(n)) return '--';
        return n.toFixed(decimals);
    }

    function resultBadge(r: string): string {
        const m: Record<string, string> = { TRIGGERED: styles.badgeTriggered, BLOCKED_COOLDOWN: styles.badgeBlocked, BLOCKED_CONFLICT: styles.badgeBlocked, SKIPPED_STANCE: styles.badgeSkipped };
        return `${styles.badge} ${m[r] || styles.badgeSkipped}`;
    }

    function stanceBadge(s: string): string {
        const m: Record<string, string> = { ACTIVE: styles.badgeActive, CLOSE_ONLY: styles.badgeCloseOnly, AVOID: styles.badgeAvoid };
        return `${styles.badge} ${m[s] || styles.badgeSkipped}`;
    }

    function stateBadge(s: string): string {
        const m: Record<string, string> = { RUNNING: styles.badgeRunning, PAUSED: styles.badgePaused, STOPPING: styles.badgeBlocked, STOPPED: styles.badgeStopped };
        return `${styles.badge} ${m[s] || styles.badgeStopped}`;
    }

    function modeBadge(m: string): string {
        const map: Record<string, string> = { ManualOnly: styles.badgeModeManual, DeterministicHeuristics: styles.badgeModeHeuristics, PaperTrading: styles.badgeModePaper, LiveTrading: styles.badgeRunning };
        return map[m] || styles.badgeModeManual;
    }

    function directionClass(d: string): string {
        return d === 'Long' ? styles.statPositive : styles.statNegative;
    }
</script>

<div class={styles.dashboard}>
    <div class={styles.sidebar}>
        <h2 class={styles.sidebarTitle}>TRADE AUTOMATION</h2>
        <button class="{styles.sidebarBtn} {activePanel === 'overview' ? styles.sidebarBtnActive : ''}" onclick={() => activePanel = 'overview'}>Overview</button>
        <button class="{styles.sidebarBtn} {activePanel === 'policies' ? styles.sidebarBtnActive : ''}" onclick={() => activePanel = 'policies'}>Policies</button>
        <button class="{styles.sidebarBtn} {activePanel === 'observability' ? styles.sidebarBtnActive : ''}" onclick={() => activePanel = 'observability'}>Observability</button>
        <button class="{styles.sidebarBtn} {activePanel === 'paper' ? styles.sidebarBtnActive : ''}" onclick={() => activePanel = 'paper'}>Paper Trading</button>
        <button class="{styles.sidebarBtn} {activePanel === 'lifecycle' ? styles.sidebarBtnActive : ''}" onclick={() => activePanel = 'lifecycle'}>Lifecycle</button>
    </div>

    <div class={styles.content}>
        {#if activePanel === 'overview'}
            <h3 class={styles.sectionTitle}>Trade Automation Engine</h3>
            <p class={styles.sectionDesc}>
                Evaluates execution policies against MME market intelligence, runs the Position Sizing Protocol,
                and routes orders to the paper or live trading engine. The strategy layer is identical between
                paper and live modes — toggling the operational mode preserves strategy behavior.
            </p>

            <div class={styles.statsGrid}>
                <div class={styles.statCard}>
                    <div class={styles.statLabel}>Operational Mode</div>
                    <div class={styles.statValue}>
                        <span class="{styles.badge} {modeBadge(operationalMode)}">{operationalMode}</span>
                    </div>
                    <div class={styles.statSub}>Paper · Simulated matching</div>
                </div>
                <div class={styles.statCard}>
                    <div class={styles.statLabel}>Active Policies</div>
                    <div class={styles.statValue}>{overviewStats.active_policies} / 3</div>
                    <div class={styles.statSub}>Enabled execution policies</div>
                </div>
                <div class={styles.statCard}>
                    <div class={styles.statLabel}>Triggered Today</div>
                    <div class={styles.statValue}>{overviewStats.triggered_today}</div>
                    <div class={styles.statSub}>{overviewStats.blocked_today} blocked</div>
                </div>
                <div class={styles.statCard}>
                    <div class={styles.statLabel}>Active Positions</div>
                    <div class={styles.statValue}>{overviewStats.active_positions}</div>
                    <div class={styles.statSub}>Paper execution engine</div>
                </div>
                <div class={styles.statCard}>
                    <div class={styles.statLabel}>Lifecycle</div>
                    <div class={styles.statValue}>{overviewStats.lifecycle_running} / 5</div>
                    <div class={styles.statSub}>{overviewStats.lifecycle_running} running · {overviewStats.lifecycle_paused} paused · {overviewStats.lifecycle_stopped} stopped</div>
                </div>
                <div class={styles.statCard}>
                    <div class={styles.statLabel}>Sizing Formula</div>
                    <div class={styles.statValue} style="font-size:1rem; font-family:var(--mono)">S = E·R / Dsl</div>
                    <div class={styles.statSub}>Position Sizing Protocol</div>
                </div>
            </div>

            <h3 class={styles.sectionTitle} style="margin-top:1rem">Execution Flow</h3>
            <table class={styles.table}>
                <thead><tr><th>Layer</th><th>Input</th><th>Output</th><th>Description</th></tr></thead>
                <tbody>
                    <tr><td>L1: Policy</td><td>Decision Matrix · Overview Matrix</td><td>Policy Matrix</td><td>Evaluates conditions against MME output; produces triggering signals</td></tr>
                    <tr><td>L2: Execution</td><td>Policy Matrix · Capital Matrix · Decision Matrix</td><td>Execution Matrix</td><td>Position sizing · order routing · paper/live dispatch · fill tracking</td></tr>
                    <tr><td>L3: Risk Gate</td><td>Execution Matrix · PME Safety Veto</td><td>Approved / Blocked</td><td>Pre-trade safety checks: lifecycle state, stance, capital, exposure</td></tr>
                </tbody>
            </table>

        {:else if activePanel === 'policies'}
            <h3 class={styles.sectionTitle}>Execution Policies</h3>
            <p class={styles.sectionDesc}>
                User-configured policies evaluate condition trees against MME decision matrices.
                Expand each policy to inspect its condition structure and risk parameters.
            </p>

            {#each policies as policy (policy.id)}
                <div class={styles.policyCard}>
                    <div class={styles.policyHeader} role="button" tabindex="0" onclick={() => togglePolicy(policy.id)} onkeydown={(e) => e.key === 'Enter' && togglePolicy(policy.id)}>
                        <span class={styles.policyName}>{policy.policy_name}</span>
                        <span class={styles.policySymbol}>{policy.symbol}</span>
                        <span class="{styles.policyDirection} {directionClass(policy.direction)}">{policy.direction}</span>
                        <span class={stanceBadge(policy.stance)}>{policy.stance.replace('_', ' ')}</span>
                        <div class={styles.policyMeta}>
                            <span>{policy.trigger_mode}</span>
                            <div class="{styles.toggle} {policy.enabled ? styles.toggleOn : styles.toggleOff}" role="switch" aria-checked={policy.enabled}>
                                <div class="{styles.toggleKnob} {policy.enabled ? styles.toggleKnobOn : styles.toggleKnobOff}"></div>
                            </div>
                        </div>
                        <span class="{styles.expandIcon} {expandedPolicies.has(policy.id) ? styles.expandIconOpen : ''}">▶</span>
                    </div>

                    {#if expandedPolicies.has(policy.id)}
                        <div class={styles.policyDetail}>
                            <p style="color:#888; font-size:0.75rem; margin-bottom:0.75rem">{policy.description}</p>

                            <h4 style="font-size:0.75rem; color:#5a5f6e; text-transform:uppercase; letter-spacing:0.05em; margin-bottom:0.4rem">Condition Tree</h4>
                            {#each policy.condition_tree as group, gi}
                                <div class={styles.conditionGroup}>{gi > 0 ? group.logic : ''}</div>
                                <div class={styles.conditionTree}>
                                    {#each group.conditions as cond}
                                        <div class={styles.conditionRow}>
                                            <span class={styles.conditionField}>{cond.field}</span>
                                            <span class={styles.conditionOp}>{cond.operator}</span>
                                            <span class={styles.conditionValue}>{cond.value}</span>
                                            {#if cond.passed !== undefined}
                                                <span class={cond.passed ? styles.conditionPass : styles.conditionFail}>
                                                    {cond.passed ? '✓' : '✗'}
                                                </span>
                                            {/if}
                                        </div>
                                    {/each}
                                </div>
                            {/each}

                            <h4 style="font-size:0.75rem; color:#5a5f6e; text-transform:uppercase; letter-spacing:0.05em; margin:0.75rem 0 0.4rem">Risk Parameters</h4>
                            <div class={styles.riskParams}>
                                <div class={styles.riskParam}>
                                    <span class={styles.riskParamLabel}>Risk per Trade</span>
                                    <span class={styles.riskParamValue}>{policy.risk.risk_per_trade_pct}%</span>
                                </div>
                                <div class={styles.riskParam}>
                                    <span class={styles.riskParamLabel}>Max Position Size</span>
                                    <span class={styles.riskParamValue}>${policy.risk.max_position_size_usd.toLocaleString()}</span>
                                </div>
                                <div class={styles.riskParam}>
                                    <span class={styles.riskParamLabel}>Max Leverage</span>
                                    <span class={styles.riskParamValue}>{policy.risk.max_leverage}x</span>
                                </div>
                                <div class={styles.riskParam}>
                                    <span class={styles.riskParamLabel}>Dynamic Stops</span>
                                    <span class={styles.riskParamValue}>{policy.risk.use_dynamic_stops ? 'Enabled' : 'Disabled'}</span>
                                </div>
                                <div class={styles.riskParam}>
                                    <span class={styles.riskParamLabel}>Stop-Loss</span>
                                    <span class={styles.riskParamValue}>{policy.risk.fixed_stop_loss_pct != null ? policy.risk.fixed_stop_loss_pct + '% fixed' : 'Dynamic (MME)'}</span>
                                </div>
                                <div class={styles.riskParam}>
                                    <span class={styles.riskParamLabel}>Target R:R</span>
                                    <span class={styles.riskParamValue}>{policy.risk.target_rr_ratio}</span>
                                </div>
                                <div class={styles.riskParam}>
                                    <span class={styles.riskParamLabel}>Cooldown</span>
                                    <span class={styles.riskParamValue}>{policy.cooldown_seconds}s</span>
                                </div>
                            </div>
                        </div>
                    {/if}
                </div>
            {/each}

        {:else if activePanel === 'observability'}
            <h3 class={styles.sectionTitle}>Policy Observability</h3>
            <p class={styles.sectionDesc}>
                Per-policy trigger log extracted from the system observability buffer.
                Shows which policies fired, were blocked, or were skipped with full condition trace.
            </p>

            {#if observability.length === 0}
                <div class={styles.placeholder}>No trigger events recorded.</div>
            {:else}
                {#each observability as obs, i}
                    <div class={styles.obsCard}>
                        <div class={styles.obsHeader}>
                            <span class={styles.obsTimestamp}>{fmtTs(obs.trigger_timestamp)}</span>
                            <span class={styles.obsPolicyId}>{obs.policy_id}</span>
                            <span class={styles.obsResultBadge}>
                                <span class={resultBadge(obs.result)}>{obs.result.replace(/_/g, ' ')}</span>
                            </span>
                        </div>
                        <div style="margin-bottom:0.5rem; font-size:0.72rem; color:#888">
                            Decision: <span style="color:#ccc">{obs.decision_snapshot.bias}</span>
                            <span style="margin-left:0.5rem">Guidance: <span style="color:#ccc">{obs.decision_snapshot.directional_guidance}</span></span>
                            <span style="margin-left:0.5rem">Confidence: <span style="color:#ccc">{fmtNum(obs.decision_snapshot.confidence * 100, 0)}%</span></span>
                        </div>
                        <div class={styles.obsConditions}>
                            {#each obs.conditions_evaluated as c}
                                <span class="{styles.obsConditionChip} {c.passed ? styles.obsConditionPass : styles.obsConditionFail}">
                                    {c.field} = {c.passed ? 'PASS' : 'FAIL'}
                                </span>
                            {/each}
                        </div>
                    </div>
                {/each}
            {/if}

        {:else if activePanel === 'paper'}
            <h3 class={styles.sectionTitle}>Paper Trading Engine</h3>
            <p class={styles.sectionDesc}>
                Simulated order matching using DIE mid-price. Same state machine, sizing protocol,
                and audit logging as live — only the execution destination changes.
            </p>

            <div class={styles.paperTabBar}>
                <button class="{styles.paperTab} {paperTab === 'positions' ? styles.paperTabActive : ''}" onclick={() => paperTab = 'positions'}>Positions</button>
                <button class="{styles.paperTab} {paperTab === 'orders' ? styles.paperTabActive : ''}" onclick={() => paperTab = 'orders'}>Orders</button>
                <button class="{styles.paperTab} {paperTab === 'history' ? styles.paperTabActive : ''}" onclick={() => paperTab = 'history'}>History</button>
            </div>

            {#if paperTab === 'positions'}
                <table class={styles.table}>
                    <thead>
                        <tr>
                            <th>Market</th>
                            <th>Side</th>
                            <th class={styles.tdRight}>Size</th>
                            <th class={styles.tdRight}>Entry</th>
                            <th class={styles.tdRight}>Mark</th>
                            <th class={styles.tdRight}>Liq Price</th>
                            <th class={styles.tdRight}>Margin</th>
                            <th class={styles.tdRight}>P&L</th>
                            <th class={styles.tdRight}>ROI</th>
                        </tr>
                    </thead>
                    <tbody>
                        <tr>
                            <td class={styles.tdMono}>BTC-USDT</td>
                            <td class={styles.statPositive}>LONG</td>
                            <td class={styles.tdRight}>0.12</td>
                            <td class={styles.tdRight}>$68,420</td>
                            <td class={styles.tdRight}>$69,150</td>
                            <td class={styles.tdRight}>$62,811</td>
                            <td class={styles.tdRight}>$821.04</td>
                            <td class="tdRight" style="color:#4caf50">+$87.60</td>
                            <td class="tdRight" style="color:#4caf50">+1.07%</td>
                        </tr>
                    </tbody>
                </table>
            {:else if paperTab === 'orders'}
                <table class={styles.table}>
                    <thead>
                        <tr>
                            <th>Type</th>
                            <th>Direction</th>
                            <th class={styles.tdRight}>Price</th>
                            <th class={styles.tdRight}>Size</th>
                            <th>Created</th>
                        </tr>
                    </thead>
                    <tbody>
                        <tr>
                            <td class={styles.tdMono}>LIMIT</td>
                            <td class={styles.statPositive}>BUY</td>
                            <td class={styles.tdRight}>$67,500</td>
                            <td class={styles.tdRight}>25%</td>
                            <td>{fmtTs(Date.now() - 1800000)}</td>
                        </tr>
                        <tr>
                            <td class={styles.tdMono}>LIMIT</td>
                            <td class={styles.statPositive}>BUY</td>
                            <td class={styles.tdRight}>$66,800</td>
                            <td class={styles.tdRight}>25%</td>
                            <td>{fmtTs(Date.now() - 3600000)}</td>
                        </tr>
                    </tbody>
                </table>
            {:else}
                <table class={styles.table}>
                    <thead>
                        <tr>
                            <th>Time</th>
                            <th>Market</th>
                            <th>Side</th>
                            <th class={styles.tdRight}>Entry</th>
                            <th class={styles.tdRight}>Exit</th>
                            <th class={styles.tdRight}>P&L</th>
                            <th class={styles.tdRight}>ROI</th>
                            <th>Trigger</th>
                        </tr>
                    </thead>
                    <tbody>
                        <tr>
                            <td>{fmtTs(Date.now() - 7200000)}</td>
                            <td class={styles.tdMono}>ETH-USDT</td>
                            <td class={styles.statNegative}>SHORT</td>
                            <td class={styles.tdRight}>$3,120</td>
                            <td class={styles.tdRight}>$3,008</td>
                            <td class="tdRight" style="color:#4caf50">+$112.00</td>
                            <td class="tdRight" style="color:#4caf50">+3.59%</td>
                            <td>TAKE_PROFIT</td>
                        </tr>
                        <tr>
                            <td>{fmtTs(Date.now() - 18000000)}</td>
                            <td class={styles.tdMono}>BTC-USDT</td>
                            <td class={styles.statPositive}>LONG</td>
                            <td class={styles.tdRight}>$67,800</td>
                            <td class={styles.tdRight}>$66,445</td>
                            <td class="tdRight" style="color:#ef5350">-$135.50</td>
                            <td class="tdRight" style="color:#ef5350">-2.00%</td>
                            <td>STOP_LOSS</td>
                        </tr>
                    </tbody>
                </table>
            {/if}

            <div class={styles.paperAccountBar}>
                <div class={styles.paperAccountItem}>
                    <span class={styles.paperAccountLabel}>Balance</span>
                    <span class={styles.paperAccountValue}>$10,000.00</span>
                </div>
                <div class={styles.paperAccountItem}>
                    <span class={styles.paperAccountLabel}>Available</span>
                    <span class={styles.paperAccountValue}>$9,178.96</span>
                </div>
                <div class={styles.paperAccountItem}>
                    <span class={styles.paperAccountLabel}>Margin Used</span>
                    <span class={styles.paperAccountValue}>$821.04</span>
                </div>
                <div class={styles.paperAccountItem}>
                    <span class={styles.paperAccountLabel}>Leverage</span>
                    <span class={styles.paperAccountValue}>10x</span>
                </div>
            </div>

        {:else if activePanel === 'lifecycle'}
            <h3 class={styles.sectionTitle}>Instance Lifecycle</h3>
            <p class={styles.sectionDesc}>
                Per-instance lifecycle management with automation config.
                Gate 0 in the pre-trade chain — entries admitted only when RUNNING.
                Exits always bypass Gate 0.
            </p>

            <div class={styles.lifecycleGrid}>
                {#each lifecycleInstances as inst (inst.id)}
                    <div class={styles.lifecycleCard}>
                        <div class={styles.lifecycleHeader}>
                            <span class={styles.lifecycleSymbol}>{inst.symbol}</span>
                            <div style="display:flex; gap:0.3rem; align-items:center">
                                <span class={stateBadge(inst.state)}>{inst.state}</span>
                                <span class={stanceBadge(inst.stance)}>{inst.stance.replace('_', ' ')}</span>
                            </div>
                        </div>
                        <div class={styles.lifecycleInfo}>
                            <div class={styles.lifecycleRow}>
                                <span class={styles.lifecycleLabel}>Start</span>
                                <span class={styles.lifecycleValue}>{inst.start_condition ?? '—'}</span>
                            </div>
                            <div class={styles.lifecycleRow}>
                                <span class={styles.lifecycleLabel}>Pause</span>
                                <span class={styles.lifecycleValue}>{inst.pause_condition ?? '—'}</span>
                            </div>
                            <div class={styles.lifecycleRow}>
                                <span class={styles.lifecycleLabel}>Stop</span>
                                <span class={styles.lifecycleValue}>{inst.stop_condition ?? '—'}</span>
                            </div>
                            <div style="margin-top:0.5rem; font-size:0.68rem; color:#5a5f6e; font-style:italic">{inst.automation_summary}</div>
                        </div>
                        <div class={styles.lifecycleControls} style="margin-top:0.75rem">
                            <button class="{styles.lifecycleBtn} {styles.lifecycleBtnStart}" disabled={inst.state === 'RUNNING'}>▶ Start</button>
                            <button class="{styles.lifecycleBtn} {styles.lifecycleBtnPause}" disabled={inst.state !== 'RUNNING'}>⏸ Pause</button>
                            <button class="{styles.lifecycleBtn} {styles.lifecycleBtnStop}" disabled={inst.state === 'STOPPED'}>⏹ Stop</button>
                        </div>
                    </div>
                {/each}
            </div>
        {/if}
    </div>
</div>
