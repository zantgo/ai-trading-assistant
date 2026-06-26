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

export interface TimeframeTelemetry {
    symbol: string;
    exchange: string;
    barDurationSec: number;
    priceText: string;
    vwapText: string;
    vwapBias: 'premium' | 'discount' | 'equilibrium';
    avgVolText: string;
    emaFastText: string;
    emaMediumText: string;
    emaSlowText: string;
    emaLongText: string;
    emaStackState: 'bullish' | 'bearish' | 'tangled';
    adxText: string;
    adxPlusText: string;
    adxMinusText: string;
    atrText: string;
    rsiText: string;
    macdLineText: string;
    macdSigText: string;
    macdHistText: string;
    sqzValText: string;
    sqzStatusText: string;
    isSqueezeOn: boolean;
    volText: string;
    bbwpText: string;
    fibGoldenLowText: string;
    fibGoldenHighText: string;
    fibExt1618Text: string;
    fibExt2618Text: string;
    lastMacdHist: number;
    lastSqzMom: number;
    lastBbwp: number;
    rsiDivergenceStatus: 'none' | 'potential' | 'confirmed';
    macdDivergenceStatus: 'none' | 'potential' | 'confirmed';
    rsiDivergenceCoords: string | null;
    macdDivergenceCoords: string | null;
    macdHistPeak: number;
    macdContractionTriggered: boolean;
    macdCrossoverDetected: boolean;
    macdCrossoverDirection: 'BULLISH' | 'BEARISH' | 'NONE';
    adxSlope: number;
    adxTrendingRegime: 'congestion' | 'emerging' | 'strong' | 'extreme';
    adxExhaustionReached: boolean;
    adxDiCrossoverDetected: boolean;
    adxDiCrossoverDirection: 'BULLISH' | 'BEARISH' | 'NONE';
    squeezeDuration: number;
    squeezeReleaseTrigger: boolean;
    squeezeMomentumDirection: 'BullishAcceleration' | 'BullishDeceleration' | 'BearishAcceleration' | 'BearishDeceleration' | 'Flat';
    activePattern: 'None' | 'BullishTriangle' | 'BearishTriangle' | 'RisingWedge' | 'FallingWedge' | 'AscendingChannel' | 'DescendingChannel';
    patternConfidence: number;
    showPatterns: boolean;
    atrVolatilityRegime: 'expanding' | 'contracting' | 'stable';
    atrSlope: number;
    rvol: number;
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
}

export interface InstanceState {
    symbol: string;
    exchange: string;
    isConnected: boolean;
    microTerm: TimeframeTelemetry;
    smallTerm: TimeframeTelemetry;
    mediumTerm: TimeframeTelemetry;
    largeTerm: TimeframeTelemetry;
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
    currentView: 'terminal' | 'assistant' | 'positions' | 'performance' | 'settings' | 'decision' | 'risk' | 'commission' | 'exchange' | 'analytics' | 'ledger' | 'costs' | 'observability';
    automationEnabled: boolean;
    automationIntervalValue: number;
    automationIntervalUnit: 'seconds' | 'minutes' | 'hours';
    slowIntervalSecs: number;
    normalIntervalSecs: number;
    fastIntervalSecs: number;
    tpLevels: number;
    slLevels: number;
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

export interface TakeProfitTarget {
    id: number;
    target_price: number;
    size_fraction: number;
    is_hit: boolean;
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
