// Shared TypeScript interfaces for the AI Trading Assistant dashboard.

// Re-export statistics types for backward compatibility
export type { CoreStats, DailyActivity, DailyPnl, HourlyWinRate, WeekdayWinRate, DirectionBreakdown, DashboardStats, TradeLedgerRecord, TradeJournalRecord, StyleSegment, TraderStyleBreakdown, StreakMetrics, CalendarDay, PairStat, DailyCommission, FeePnlRatio, MonthlySummary } from './types/stats';

// ================================================================
// 1. AI Orchestrator & Agents
// ================================================================

export interface TrendAnalysis {
    classification: 'trending upwards' | 'trending downwards' | 'sideways';
    structural_reasoning: string;
}

export interface IndicatorAlignment {
    classification: 'supportive' | 'conflicting' | 'neutral';
    observation: string;
}

export interface PositionRecommendation {
    action: 'Hold' | 'Close' | 'Wait' | 'Open Long' | 'Open Short';
    rationale: string;
}

export interface AssistantAnalysis {
    trend_analysis: TrendAnalysis;
    indicator_alignment: IndicatorAlignment;
    position_recommendation: PositionRecommendation;
}

export interface AssistantHistoryRecord {
    id: number;
    created_at: string;
    position: 'None' | 'Long' | 'Short';
    entry_price?: string;
    trend_classification: 'UPWARD' | 'DOWNWARD' | 'SIDEWAYS';
    indicator_alignment: string;
    indicator_synthesis_summary?: string;
    recommended_action: 'Hold' | 'Close' | 'Wait' | 'Open Long' | 'Open Short';
    recommendation_rationale: string;
    price_at_analysis: string;
    support_levels?: string;
    resistance_levels?: string;
    symbol: string;
    trigger_type: 'Manual' | 'Automated';
}

export interface ChatMessage {
    role: 'user' | 'assistant' | 'system';
    content: string;
}

export interface IndicatorRule {
    id: number;
    profile_id: number;
    indicator_name: string;
    weight: number;
    override_status: string;
}

export interface DecisionProfile {
    id: number;
    profile_name: string;
    long_threshold: number;
    short_threshold: number;
    indicators: IndicatorRule[];
}

export interface IndicatorResult {
    indicator_name: string;
    signal: string;
    weight: number;
    weighted_contribution: number;
    override_active: boolean;
}

export interface DecisionScore {
    profile_name: string;
    score: number;
    recommendation: string;
    momentum_bias: number;
    indicator_results: IndicatorResult[];
}

// ================================================================
// 2. Ingestion & Telemetry
// ================================================================

export interface RiskProfile {
    id: number;
    profile_name: string;
    capital: number;
    max_risk_pct: number;
    leverage: number;
    commission_pct: number;
    funding_rate_8h: number;
    spread: number;
}

export interface SystemHeartbeat {
    connected: boolean;
    latency_ms: number;
    journal_mode: string;
    total_allocated_margin: number;
    total_ai_token_costs_usd: number;
    active_pairs_count: number;
}

export interface InstanceSummary {
    id: string;
    pair: string;
    status: 'running' | 'paused' | 'stopped';
    symbol: string;
    initial_capital: number;
    current_equity: number;
    consecutive_losses: number;
    caution_level: 'normal' | 'cautious' | 'suspended' | 'drawdown_stop';
}

export interface DecisionMemoryRow {
    id: number;
    symbol: string;
    timestamp: number;
    regime_classification: string;
    orchestrator_decision: string;
    confidence_score: number;
    eight_factor_score: number;
    portfolio_risk_pct: number;
}

export interface CompletedTradesRow {
    id: number;
    symbol: string;
    direction: string;
    entry_price: number;
    exit_price: number;
    realized_pnl: number;
    roi_pct: number;
    execution_score: number;
    primary_mistake: string;
    closed_at: number;
}

export interface RiskCalculation {
    risk_capital: number;
    price_distance: number;
    position_size_units: number;
    position_notional: number;
    leverage_required: number;
    leverage_selected: number;
    margin_required: number;
    liquidation_price: number;
    risk_reward_ratio: number | null;
    estimated_profit: number;
    total_fees: number;
    net_pnl: number;
}

export interface FeeTableRow {
    exchange_fee_pct: number;
    leverage: number;
    capital: number;
    min_profit_pct_to_cover_fees: number;
    fees_in_dollars: number;
}

export interface FeeBreakdown {
    maker_fee_pct: number;
    taker_fee_pct: number;
    order_type: string;
    effective_fee_pct: number;
    entry_1_fees: number;
    entry_2_fees: number;
    total_fees: number;
    funding_rate_8h: number;
    funding_cost: number;
}

export interface EntryMetrics {
    entry_number: number;
    entry_price: number;
    stop_loss_price: number;
    take_profit_price: number;
    capital_allocated: number;
    capital_pct: number;
    position_size_units: number;
    position_notional: number;
    margin_required: number;
    risk_amount: number;
    potential_profit: number;
    fees: number;
    net_profit: number;
}

export interface CommissionProjection {
    direction: string;
    leverage: number;
    total_capital: number;
    total_position_notional: number;
    total_margin_required: number;
    weighted_avg_entry: number;
    effective_stop_loss: number;
    effective_take_profit: number;
    total_risk_amount: number;
    fee_breakdown: FeeBreakdown;
    entry_1: EntryMetrics;
    entry_2: EntryMetrics;
    max_gain_scenario: number;
    max_loss_scenario: number;
    max_gain_net_after_fees: number;
    max_loss_net_after_fees: number;
    trade_viable: boolean;
    viability_reason: string;
    min_profit_pct_to_cover_fees: number;
    required_price_move_pct: number;
}

export interface ExchangeAccount {
    id: number;
    exchange: string;
    account_name: string;
    api_key: string;
    api_secret: string;
    passphrase: string;
    referred_uid: string;
    is_active: boolean;
    last_sync_timestamp: number | null;
}

// ================================================================
// 3. Portfolio & Paper Trading
// ================================================================

export interface IndividualIndicatorResult {
    indicator_name: string;
    signal: 'BULLISH' | 'BEARISH' | 'SIDEWAYS' | 'UNAVAILABLE';
    reason: string;
    divergence_status?: 'none' | 'potential' | 'confirmed';
    divergence_type?: 'bullish' | 'bearish' | null;
    is_confirmed?: boolean;
}

export interface SupportResistance {
    detected_support_levels: string[];
    detected_resistance_levels: string[];
    structural_analysis: string;
}

export interface IndicatorSynthesis {
    summary_count: string;
    evaluation: string;
}

export interface MasterOrchestratorResult {
    general_trend: 'UPWARD' | 'DOWNWARD' | 'SIDEWAYS';
    support_and_resistance: SupportResistance;
    indicator_synthesis: IndicatorSynthesis;
    position_recommendation: { action: 'Hold' | 'Close' | 'Wait' | 'Open Long' | 'Open Short'; rationale: string; };
}

export interface MultiAgentAnalysis {
    phase_one: IndividualIndicatorResult[];
    phase_two: MasterOrchestratorResult;
}

export interface AgentProgress {
    name: string;
    status: 'pending' | 'running' | 'complete' | 'failed';
}

// ================================================================
// 4. Instance & WebSocket Telemetry
// ================================================================

/** Dual-representation normalized indicator DTO (mirrors the Rust NormalizedIndicatorValue). */
export interface IndicatorDto {
    raw_value: number;
    normalized: number;
    state_label: string;
    values?: Record<string, number> | null;
}

export type IndicatorMap = Record<string, IndicatorDto>;

/** Safe sentinel for a missing indicator: neutral, unknown. */
export function emptyIndicator(): IndicatorDto {
    return { raw_value: 0, normalized: 0, state_label: 'UNKNOWN', values: null };
}

export interface TimeframeTelemetry {
    symbol: string;
    exchange: string;
    barDurationSec: number;
    /**
     * Authoritative nested normalized indicator map, keyed by indicator name
     * (rsi, macd, squeeze, adx, bbwp, rvol, ema_stack, vwap, fibonacci,
     * patterns, support_resistance, atr, bollinger, rsi_divergence,
     * macd_divergence). The legacy flat *Text fields below are derived from
     * this map for backwards compatibility with existing components.
     */
    indicators: IndicatorMap;
    // Core (non-indicator) market data retained as flat text.
    priceText: string;
    volText: string;
    avgVolText: string;
    showPatterns: boolean;
    isCompleted: boolean;
    latestSnapshot: Record<string, unknown> | null;
    historyPrices: number[];
    showEmas: boolean;
    showBb: boolean;
    showVwap: boolean;
    showVolume: boolean;
    showAdx: boolean;
    showAtr: boolean;
    showRsi: boolean;
    showMacd: boolean;
    showSqueeze: boolean;
    showBbwp: boolean;
    showFib: boolean;
    showRvol: boolean;
    emaFastVal: number;
    emaMediumVal: number;
    emaSlowVal: number;
    emaLongVal: number;
    rsiPeriodVal: number;
    macdFastVal: number;
    macdSlowVal: number;
    macdSignalVal: number;
    adxPeriodVal: number;
    atrPeriodVal: number;
    squeezePeriodVal: number;
    bbwpPeriodVal: number;
    bbwpLookbackVal: number;
    analysisLimit: number;
    macdExtremeHighVal: number;
    macdExtremeLowVal: number;
    macdContractionVal: number;
    adxTrendThresholdVal: number;
    adxExhaustionThresholdVal: number;
    adxSlopeLookbackVal: number;
    squeezeMinDurationVal: number;
    squeezeBbPeriodVal: number;
    squeezeBbStdDevVal: number;
    squeezeKcPeriodVal: number;
    squeezeKcAtrMultVal: number;
    atrMultiplierVal: number;
    atrTargetRRVal: number;
    volumeAvgPeriodVal: number;
    rvolInstitutionalVal: number;
    rvolClimaxVal: number;
}

export interface InstanceState {
    symbol: string;
    exchange: string;
    isConnected: boolean;
    microTerm: TimeframeTelemetry;
    fastTerm: TimeframeTelemetry;
    slowTerm: TimeframeTelemetry;
    macroTerm: TimeframeTelemetry;
    assistantHistory: AssistantHistoryRecord[];
    chatHistory: ChatMessage[];
    currentPosition: 'None' | 'Long' | 'Short';
    entryPriceVal: string;
    stopLossVal: string;
    assistantLoading: boolean;
    assistantError: string | null;
    assistantResponse: AssistantAnalysis | null;
    multiAgentResponse: MultiAgentAnalysis | null;
    analysisPhase: 'idle' | 'phase1' | 'phase2' | 'complete';
    individualResults: IndividualIndicatorResult[];
    agentProgress: AgentProgress[];
    historyLatestClose: string;
    isAssistantModalOpen: boolean;
    chatInputText: string;
    isChatLoading: boolean;
    currentView: 'terminal' | 'assistant' | 'positions' | 'performance' | 'settings' | 'decision' | 'risk' | 'commission' | 'exchange' | 'analytics' | 'ledger' | 'costs' | 'observability' | 'timeframe_settings' | 'edge_builder' | 'edge_analyzer';
    automationEnabled: boolean;
    automationIntervalValue: number;
    automationIntervalUnit: 'seconds' | 'minutes' | 'hours';
    slowIntervalSecs: number;
    normalIntervalSecs: number;
    fastIntervalSecs: number;
    nextEvaluationIn: string;
    totalPointsScore: number;
    allocatedCapitalPct: number;
    activeOppositeSignalsCount: number;
    markedSupportLevels: number[];
    markedResistanceLevels: number[];
    srFlipEvents: string;
    priceLineMode: boolean;
    showEmaFast: boolean;
    showEmaMedium: boolean;
    showEmaSlow: boolean;
    showEmaLong: boolean;
}

export interface ScaleInPortion {
    id: number;
    entry_price: number;
    size: number;
    allocated_usd: number;
    portion_number: number;
}

export interface PositionSlot {
    id: number;
    position_id: number;
    symbol: string;
    direction: 'LONG' | 'SHORT';
    slot_index: number;
    is_active: boolean;
    entry_price: number;
    size: number;
    allocated_usd: number;
    realized_pnl: number;
    timestamp: number;
}

export interface SlotState {
    slot_index: number;
    is_active: boolean;
    entry_price: number;
    size: number;
    allocated_usd: number;
}

export interface EquitySnapshot {
    timestamp: number;
    equity_value: number;
    cash_balance: number;
    unrealized_pnl: number;
}

export interface TakeProfitTarget {
    id: number;
    target_price: number;
    size_fraction: number;
    is_hit: boolean;
}

export interface OpenOrder {
    id: number;
    symbol: string;
    order_type: 'LIMIT' | 'STOP';
    direction: 'BUY' | 'SELL';
    price: number | null;
    trigger_price: number | null;
    size: number;
    is_reduce_only: boolean;
    associated_position_id: number | null;
    created_at: number;
}

export interface PlaceOrderPayload {
    order_type: 'LIMIT' | 'STOP';
    direction: 'BUY' | 'SELL';
    price?: number;
    trigger_price?: number;
}

export interface UserTrade {
    id: number;
    timestamp: number;
    symbol: string;
    direction: string;
    outcome: 'WIN' | 'LOSS';
    risk_multiplier: number;
    reward_multiplier: number;
}

// ================================================================
// 5. Edge Builder & Edge Analyzer
// ================================================================

export type EdgeArchetype = 'trend_following' | 'mean_reversion';

export interface RegimeGates {
    trending: boolean;
    compression: boolean;
    expansion: boolean;
    range: boolean;
}

export type TriggerRule = 'crossover' | 'overbought_oversold' | 'divergence' | 'slope_direction' | 'threshold_above' | 'threshold_below' | 'release';

export interface IndicatorConfig {
    name: string;
    weight: number;
    trigger_rule: TriggerRule;
    enabled: boolean;
}

export type SizingModel = 'fixed' | 'volatility_targeting';

export interface SizingConfig {
    model: SizingModel;
    daily_vol_target_pct: number;
    max_leverage: number;
}

export type StopLossModel = 'atr_volatility_stop' | 'structural_pivot' | 'fixed_percentage';

export interface StopLossConfig {
    model: StopLossModel;
    atr_multiplier: number;
}

export interface TakeProfitConfig {
    tp1_multiplier: number;
    tp2_multiplier: number;
    tp3_multiplier: number;
}

export type TriggerPhase = 'execute_on_trigger' | 'execute_on_confirmed_close';

export interface ExecutionConfig {
    min_rvol: number;
    climax_rvol: number;
    trigger_phase: TriggerPhase;
    vwap_filter: boolean;
}

export interface EdgeConfig {
    archetype: EdgeArchetype;
    regime_gates: RegimeGates;
    quorum_threshold: number;
    mtf_quorum: string[];
    indicators: IndicatorConfig[];
    sizing: SizingConfig;
    stop_loss: StopLossConfig;
    take_profit: TakeProfitConfig;
    execution: ExecutionConfig;
    backtest_depth: number;
}

export interface EdgeSaveRequest {
    name: string;
    pair_key: string;
    description: string;
    config: EdgeConfig;
}

export interface EdgeAnalyzeRequest {
    edge_id: number;
    symbol: string;
    timeframe_secs: number;
}

export interface SavedEdge {
    id: number;
    name: string;
    pair_key: string;
    description: string | null;
    config: EdgeConfig;
    created_at: string;
}

export interface HistoricalMetrics {
    total_trades: number;
    win_rate: number;
    profit_factor: number;
    net_sharpe_ratio: number;
    max_drawdown_pct: number;
    max_drawdown_duration: number;
    total_return_pct: number;
    avg_trade_return_pct: number;
    avg_win_pct: number;
    avg_loss_pct: number;
}

export interface EquityPoint {
    trade_index: number;
    cumulative_return_pct: number;
    /** Market regime of the trade producing this point (empty for seed point). */
    regime: string;
}

export interface BacktestCurveData {
    in_sample: EquityPoint[];
    out_of_sample: EquityPoint[];
    combined: EquityPoint[];
}

export interface MonteCarloPath {
    path_index: number;
    equity_points: number[];
    max_drawdown_pct: number;
    final_return_pct: number;
}

export interface DrawdownBucket {
    bucket_pct: number;
    frequency: number;
}

export interface EdgeAnalysisResponse {
    edge_id: number;
    edge_name: string;
    symbol: string;
    timeframe_secs: number;
    backtest_depth: number;
    historical_metrics: HistoricalMetrics;
    backtest_curve: BacktestCurveData;
    bootstrap_p_value: number;
    bootstrap_significant: boolean;
    monte_carlo_paths: MonteCarloPath[];
    drawdown_distribution: DrawdownBucket[];
    probability_of_ruin_pct: number;
    confidence_95_drawdown_pct: number;
    skewness: number;
    cached: boolean;
}
