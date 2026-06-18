// Global reactive state using Svelte 5 runes

export interface TrendAnalysis {
    classification: string;
    structural_reasoning: string;
}

export interface IndicatorAlignment {
    classification: string;
    observation: string;
}

export interface PositionRecommendation {
    action: string;
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
    position: string;
    entry_price?: string;
    trend_classification: string;
    indicator_alignment: string;
    indicator_synthesis_summary?: string;
    recommended_action: string;
    recommendation_rationale: string;
    price_at_analysis: string;
    support_levels?: string;
    resistance_levels?: string;
    symbol: string;
    trigger_type: string;
}

export interface ChatMessage {
    role: string;
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
    status: string;
    symbol: string;
    initial_capital: number;
    current_equity: number;
    consecutive_losses: number;
    caution_level: string;
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

export interface CoreStats {
    total_pnl: number;
    win_rate: number;
    avg_loss: number;
    avg_gain: number;
    expectancy: number;
    avg_risk_reward_ratio: number;
    profit_factor: number;
    largest_loss: number;
    largest_gain: number;
    total_trades: number;
    wins: number;
    losses: number;
}

export interface DailyActivity {
    date: string;
    longs: number;
    shorts: number;
    win_rate: number;
}

export interface DailyPnl { date: string; pnl: number; }
export interface HourlyWinRate { hour: number; win_rate: number; volume: number; }
export interface WeekdayWinRate { weekday: string; win_rate: number; volume: number; }
export interface DirectionBreakdown { longs: number; shorts: number; long_expectancy: number; short_expectancy: number; }
export interface StyleSegment { count: number; avg_duration_minutes: number; win_rate: number; }
export interface TraderStyleBreakdown { scalper: StyleSegment; day_trader: StyleSegment; swing_trader: StyleSegment; }
export interface StreakMetrics { avg_streak_length: number; max_consecutive_value: number; max_streak_length: number; }
export interface CalendarDay { date: string; pnl: number; month: number; day: number; }
export interface PairStat { symbol: string; value: number; }
export interface DailyCommission { date: string; fees: number; }
export interface FeePnlRatio { date: string; ratio: number; }
export interface MonthlySummary { month: string; net_pnl: number; win_rate: number; trade_count: number; }

export interface DashboardStats {
    core_stats: CoreStats;
    equity_curve: [number, number][];
    daily_activity: DailyActivity[];
    daily_pnl: DailyPnl[];
    win_rate_by_hour: HourlyWinRate[];
    win_rate_by_weekday: WeekdayWinRate[];
    direction_breakdown: DirectionBreakdown;
    trader_style: TraderStyleBreakdown;
    winning_streaks: StreakMetrics;
    losing_streaks: StreakMetrics;
    post_loss_recovery_pct: number;
    pnl_calendar: CalendarDay[];
    pair_volume: PairStat[];
    top_pairs_profitability: PairStat[];
    bottom_pairs_profitability: PairStat[];
    daily_commissions: DailyCommission[];
    cumulative_commissions: [number, number][];
    fee_pnl_ratio: FeePnlRatio[];
    monthly_summary: MonthlySummary[];
}

export interface TradeLedgerRecord {
    id: number;
    exchange: string;
    symbol: string;
    direction: string;
    entry_timestamp: number;
    exit_timestamp: number;
    entry_price: number;
    exit_price: number;
    size: number;
    commission_fees: number;
    funding_fees: number;
    realized_pnl: number;
    roi_percentage: number;
    trigger_source: string;
}

export interface TradeJournalRecord {
    id: number;
    trade_id: number;
    entry_date: string;
    exit_date: string;
    asset: string;
    direction: string;
    entry_reason: string;
    roe_percentage: number;
    final_analysis: string;
    execution_score: number;
    human_notes: string;
    created_at: string;
    symbol: string;
    realized_pnl: number;
    roi_percentage: number;
}

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
    position_recommendation: { action: string; rationale: string; };
}

export interface MultiAgentAnalysis {
    phase_one: IndividualIndicatorResult[];
    phase_two: MasterOrchestratorResult;
}

export interface AgentProgress {
    name: string;
    status: 'pending' | 'running' | 'complete' | 'failed';
}

// ─── Multi-Timeframe Telemetry ────────────────────────────────

export interface TimeframeTelemetry {
    symbol: string;
    exchange: string;
    barDurationSec: number;
    priceText: string;
    vwapText: string;
    vwapBias: string;
    avgVolText: string;
    emaFastText: string;
    emaMediumText: string;
    emaSlowText: string;
    emaLongText: string;
    emaStackState: string;
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
    // Divergence tracking
    rsiDivergenceStatus: 'none' | 'potential' | 'confirmed';
    macdDivergenceStatus: 'none' | 'potential' | 'confirmed';
    rsiDivergenceCoords: string | null;
    macdDivergenceCoords: string | null;
    // MACD momentum tracking
    macdHistPeak: number;
    macdContractionTriggered: boolean;
    macdCrossoverDetected: boolean;
    macdCrossoverDirection: string;
    // ADX trend tracking
    adxSlope: number;
    adxTrendingRegime: string;
    adxExhaustionReached: boolean;
    adxDiCrossoverDetected: boolean;
    adxDiCrossoverDirection: string;
    // Squeeze momentum tracking
    squeezeDuration: number;
    squeezeReleaseTrigger: boolean;
    squeezeMomentumDirection: string;
    // Chart pattern tracking
    activePattern: string;
    patternConfidence: number;
    showPatterns: boolean;
    // ATR volatility tracking
    atrVolatilityRegime: string;
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
    // Indicator config
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
    currentView: 'terminal' | 'performance' | 'settings' | 'positions' | 'decision' | 'risk' | 'commission' | 'exchange' | 'analytics' | 'ledger' | 'costs' | 'observability';
    automationEnabled: boolean;
    automationIntervalValue: number;
    automationIntervalUnit: 'seconds' | 'minutes' | 'hours';
    slowIntervalSecs: number;
    normalIntervalSecs: number;
    fastIntervalSecs: number;
    tpLevels: number;
    slLevels: number;
    nextEvaluationIn: string;
    paperCashBalance: number;
    paperInitialUSD: number;
    paperAllocationPct: number;
    paperAutoExecute: boolean;
    activePaperPosition: Record<string, unknown> | null;
    paperUnrealizedPnl: number;
    paperUnrealizedRoi: number;
    paperTotalAccountValue: number;
    paperMarginUsed: number;
    paperMaxTrades: number;
    paperActiveTrades: number;
    paperAvailableTrades: number;
    paperHistory: Record<string, unknown>[];
    paperLoading: boolean;
    paperScaleInPortions: ScaleInPortion[];
    paperTakeProfitTargets: TakeProfitTarget[];
    paperAvgEntryPrice: number;
    paperInvalidationLevel: number;
    paperFilledPortions: number;
    paperMaxRiskPct: number;
    paperLeverage: number;
    paperAutoExecuteIntervals: number;
    paperLookbackTrades: number;
    totalPointsScore: number;
    allocatedCapitalPct: number;
    activeOppositeSignalsCount: number;
    markedSupportLevels: number[];
    markedResistanceLevels: number[];
    srFlipEvents: string;
    // Token cost tracking
    costPriceInput: number;
    costPriceOutput: number;
    costIntervalSecs: number;
    costRunsPerDay: number;
    costTokensPerRunInput: number;
    costTokensPerRunOutput: number;
    costDailyProjected: number;
    costWeeklyProjected: number;
    costMonthlyProjected: number;
    costActualInputTokens: number;
    costActualOutputTokens: number;
    costActualTotal: number;
    costLoading: boolean;
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

function createTimeframeTelemetry(symbol: string, barDurationSec: number): TimeframeTelemetry {
    return {
        symbol,
        exchange: 'Hyperliquid',
        barDurationSec,
        priceText: '--',
        vwapText: '--',
        vwapBias: 'equilibrium',
        avgVolText: '--',
        emaFastText: '--',
        emaMediumText: '--',
        emaSlowText: '--',
        emaLongText: '--',
        emaStackState: 'tangled',
        adxText: '--',
        adxPlusText: '--',
        adxMinusText: '--',
        atrText: '--',
        rsiText: '--',
        macdLineText: '--',
        macdSigText: '--',
        macdHistText: '--',
        sqzValText: '--',
        sqzStatusText: '--',
        isSqueezeOn: false,
        volText: '--',
        bbwpText: '--',
        fibGoldenLowText: '--',
        fibGoldenHighText: '--',
        fibExt1618Text: '--',
        fibExt2618Text: '--',
        lastMacdHist: 0,
        lastSqzMom: 0,
        lastBbwp: 0,
        rsiDivergenceStatus: 'none' as const,
        macdDivergenceStatus: 'none' as const,
        rsiDivergenceCoords: null,
        macdDivergenceCoords: null,
        macdHistPeak: 0,
        macdContractionTriggered: false,
        macdCrossoverDetected: false,
        macdCrossoverDirection: 'NONE',
        adxSlope: 0,
        adxTrendingRegime: 'congestion',
        adxExhaustionReached: false,
        adxDiCrossoverDetected: false,
        adxDiCrossoverDirection: 'NONE',
        squeezeDuration: 0,
        squeezeReleaseTrigger: false,
        squeezeMomentumDirection: 'Flat',
        activePattern: 'None',
        patternConfidence: 0,
        showPatterns: true,
        atrVolatilityRegime: 'stable',
        atrSlope: 0,
        rvol: 0,
        isCompleted: false,
        latestSnapshot: null,
        historyPrices: [],
        showEmas: true,
        showBb: true,
        showVwap: true,
        showVolume: true,
        showAdx: true,
        showAtr: true,
        showRsi: true,
        showMacd: true,
        showSqueeze: true,
        showBbwp: true,
        showFib: true,
        emaFastVal: 10,
        emaMediumVal: 50,
        emaSlowVal: 100,
        emaLongVal: 200,
        rsiPeriodVal: 14,
        macdFastVal: 12,
        macdSlowVal: 26,
        macdSignalVal: 9,
        adxPeriodVal: 14,
        atrPeriodVal: 14,
        squeezePeriodVal: 20,
        bbwpPeriodVal: 20,
        bbwpLookbackVal: 252,
        analysisLimit: 100,
    };
}

function createInstanceState(symbol: string): InstanceState {
    return {
        symbol,
        exchange: 'Hyperliquid',
        isConnected: false,
        microTerm: createTimeframeTelemetry(symbol, 60),
        smallTerm: createTimeframeTelemetry(symbol, 300),
        mediumTerm: createTimeframeTelemetry(symbol, 900),
        largeTerm: createTimeframeTelemetry(symbol, 3600),
        assistantHistory: [],
        chatHistory: [],
        currentPosition: 'None',
        entryPriceVal: '',
        stopLossVal: '',
        assistantLoading: false,
        assistantError: null,
        assistantResponse: null,
        multiAgentResponse: null,
        analysisPhase: 'idle',
        individualResults: [],
        agentProgress: [],
        historyLatestClose: '0',
        isAssistantModalOpen: false,
        chatInputText: '',
        isChatLoading: false,
        currentView: 'terminal',
        automationEnabled: false,
        automationIntervalValue: 15,
        automationIntervalUnit: 'minutes',
        slowIntervalSecs: 3600,
        normalIntervalSecs: 900,
        fastIntervalSecs: 300,
        tpLevels: 1,
        slLevels: 1,
        nextEvaluationIn: '--',
        paperCashBalance: 10000,
        paperInitialUSD: 10000,
        paperAllocationPct: 10,
        paperAutoExecute: false,
        activePaperPosition: null,
        paperUnrealizedPnl: 0,
        paperUnrealizedRoi: 0,
        paperTotalAccountValue: 10000,
        paperMarginUsed: 0,
        paperMaxTrades: 10,
        paperActiveTrades: 0,
        paperAvailableTrades: 10,
        paperHistory: [],
        paperLoading: false,
        paperScaleInPortions: [],
        paperTakeProfitTargets: [],
        paperAvgEntryPrice: 0,
        paperInvalidationLevel: 0,
        paperFilledPortions: 0,
        paperMaxRiskPct: 2.0,
        paperLeverage: 20,
        paperAutoExecuteIntervals: 15,
        paperLookbackTrades: 10,
        totalPointsScore: 0,
        allocatedCapitalPct: 0,
        activeOppositeSignalsCount: 0,
        markedSupportLevels: [],
        markedResistanceLevels: [],
        srFlipEvents: '[]',
        costPriceInput: 0.27,
        costPriceOutput: 1.10,
        costIntervalSecs: 900,
        costRunsPerDay: 0,
        costTokensPerRunInput: 0,
        costTokensPerRunOutput: 0,
        costDailyProjected: 0,
        costWeeklyProjected: 0,
        costMonthlyProjected: 0,
        costActualInputTokens: 0,
        costActualOutputTokens: 0,
        costActualTotal: 0,
        costLoading: false,
    };
}

// ─── Session State ────────────────────────────────────────────────

let sessionActive = $state(false);
let sessionMode = $state<string>('paper');
let sessionCurrency = $state<string>('USDT');
let sessionExchange = $state<string>('Hyperliquid');
let sessionCapital = $state(0);
let sessionInstanceCount = $state(0);
let sessionMaxInstances = $state(100);
let sessionLoading = $state(false);
let sessionChecked = $state(false);
let sessionError = $state<string | null>(null);

async function fetchSessionStatus() {
    try {
        const res = await fetch('/api/session/status');
        if (res.ok) {
            const data = await res.json();
            sessionActive = data.active;
            sessionMode = data.mode || 'paper';
            sessionCurrency = data.currency || 'USDT';
            sessionExchange = data.exchange || 'Hyperliquid';
            sessionCapital = data.capital || 0;
            sessionInstanceCount = data.instance_count || 0;
            sessionMaxInstances = data.max_instances || 100;
        }
    } catch (_) {
        // Backend may not be ready yet
    } finally {
        sessionChecked = true;
    }
}

async function initSession(mode: string, currency: string, exchange: string, capital: number): Promise<{ success: boolean; error?: string }> {
    sessionLoading = true;
    sessionError = null;
    try {
        const res = await fetch('/api/session/init', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ mode, currency, exchange, capital }),
        });
        const data = await res.json();
        if (res.ok && data.success) {
            sessionActive = true;
            sessionMode = mode;
            sessionCurrency = currency;
            sessionExchange = exchange;
            sessionCapital = capital;
            sessionLoading = false;
            return { success: true };
        } else {
            sessionError = data.error || 'Session initialization failed';
            sessionLoading = false;
            return { success: false, error: sessionError || undefined };
        }
    } catch (e: any) {
        sessionError = e.message || 'Network error';
        sessionLoading = false;
        return { success: false, error: sessionError || undefined };
    }
}

async function quitSession(): Promise<boolean> {
    sessionLoading = true;
    try {
        const res = await fetch('/api/session/quit', { method: 'POST' });
        const data = await res.json();
        if (res.ok && data.success) {
            sessionActive = false;
            sessionMode = 'paper';
            sessionCurrency = 'USDT';
            sessionExchange = 'Hyperliquid';
            sessionCapital = 0;
            sessionInstanceCount = 0;
            sessionLoading = false;
            return true;
        }
        sessionLoading = false;
        return false;
    } catch (_) {
        sessionLoading = false;
        return false;
    }
}

let instancesMap = $state<Record<string, InstanceState>>({});
let activeTab = $state<string>('BTC-USDT');

let apiKeyConfigured = $state(true);
let rulesContent = $state('');

let globalCandlesConfig = $state({ duration_seconds: 60, analysis_limit: 100 });
let globalIndicatorsConfig = $state({
    ema_fast: 10,
    ema_medium: 50,
    ema_slow: 100,
    ema_long: 200,
    rsi_period: 14,
    macd_fast: 12,
    macd_slow: 26,
    macd_signal: 9,
    adx_period: 14,
    atr_period: 14,
    squeeze_period: 20,
});

let emaFastLabel = $state('EMA-10');
let emaMediumLabel = $state('EMA-50');
let emaSlowLabel = $state('EMA-100');
let emaLongLabel = $state('EMA-200');
let rsiLabel = $state('RSI (14)');
let adxLabel = $state('ADX (14)');
let atrLabel = $state('ATR (14)');
let macdLabel = $state('MACD (12,26,9)');

let activeDecisionProfileId = $state(1);
let decisionProfiles = $state<DecisionProfile[]>([]);
let calculatedDecisionScore = $state<DecisionScore | null>(null);
let decisionLoading = $state(false);

let activeRiskProfileId = $state(1);
let riskProfiles = $state<RiskProfile[]>([]);
let riskDirection = $state<'LONG' | 'SHORT'>('LONG');
let riskEntryPrice = $state('0');
let riskStopLoss = $state('0');
let riskTakeProfit = $state('0');
let riskCalculation = $state<RiskCalculation | null>(null);
let riskCalculating = $state(false);
let useDynamicAtr = $state(false);
let atrValue = $state(0);

let commissionDirection = $state<'LONG' | 'SHORT'>('LONG');
let commissionEntry1 = $state('');
let commissionEntry2 = $state('');
let commissionSL1 = $state('');
let commissionSL2 = $state('');
let commissionTP1 = $state('');
let commissionTP2 = $state('');
let commissionCapitalSplit = $state(50);
let commissionOrderType = $state<'maker' | 'taker'>('taker');
let commissionProjection = $state<CommissionProjection | null>(null);
let commissionLoading = $state(false);
let feeTable = $state<FeeTableRow[]>([]);
let feeTableLoading = $state(false);

let exchangeAccounts = $state<ExchangeAccount[]>([]);
let exchangeActiveCount = $state(0);
let exchangeMaxAccounts = $state(3);
let exchangeFormDraft = $state({
    exchange: 'Bitget',
    account_name: '',
    api_key: '',
    api_secret: '',
    passphrase: '',
    referred_uid: '',
    is_active: true,
});

let dashboardStats = $state<DashboardStats | null>(null);
let dashboardActiveFilter = $state('summary');
let dashboardPeriod = $state('Todo');
let dashboardOrigin = $state('Todos');
let tradeLedgerRecords = $state<TradeLedgerRecord[]>([]);

let tradeJournalRecords = $state<TradeJournalRecord[]>([]);
let journalLookbackDepth = $state(10);

let userTrades = $state<UserTrade[]>([]);

let systemHeartbeat = $state<SystemHeartbeat | null>(null);
let recentDecisions = $state<DecisionMemoryRow[]>([]);
let completedTrades = $state<CompletedTradesRow[]>([]);

export interface UserTrade {
    id: number;
    timestamp: number;
    symbol: string;
    direction: string;
    outcome: 'WIN' | 'LOSS';
    risk_multiplier: number;
    reward_multiplier: number;
}

function activeInstance(): InstanceState {
    if (!instancesMap[activeTab]) {
        const parts = activeTab.split('-');
        instancesMap[activeTab] = createInstanceState(parts[0] || 'BTC');
    }
    return instancesMap[activeTab];
}

// Helper: get the micro-term state (default for backward-compatible accessors)
function micro(): TimeframeTelemetry { return activeInstance().microTerm; }

export function initInstance(symbol: string, exchange?: string) {
    const key = `${symbol}-USDT`;
    if (!instancesMap[key]) {
        instancesMap[key] = createInstanceState(symbol);
    } else {
        const pair = instancesMap[key];
        pair.microTerm.barDurationSec = 60;
        pair.microTerm.emaFastVal = globalIndicatorsConfig.ema_fast;
        pair.microTerm.emaMediumVal = globalIndicatorsConfig.ema_medium;
        pair.microTerm.emaSlowVal = globalIndicatorsConfig.ema_slow;
        pair.microTerm.emaLongVal = globalIndicatorsConfig.ema_long;
        pair.microTerm.rsiPeriodVal = globalIndicatorsConfig.rsi_period;
        pair.microTerm.macdFastVal = globalIndicatorsConfig.macd_fast;
        pair.microTerm.macdSlowVal = globalIndicatorsConfig.macd_slow;
        pair.microTerm.macdSignalVal = globalIndicatorsConfig.macd_signal;
        pair.microTerm.adxPeriodVal = globalIndicatorsConfig.adx_period;
        pair.microTerm.atrPeriodVal = globalIndicatorsConfig.atr_period;
        pair.microTerm.squeezePeriodVal = globalIndicatorsConfig.squeeze_period;
        pair.microTerm.analysisLimit = globalCandlesConfig.analysis_limit ?? 100;

        pair.smallTerm.barDurationSec = 300;
        pair.smallTerm.emaFastVal = globalIndicatorsConfig.ema_fast;
        pair.smallTerm.emaMediumVal = globalIndicatorsConfig.ema_medium;
        pair.smallTerm.emaSlowVal = globalIndicatorsConfig.ema_slow;
        pair.smallTerm.emaLongVal = globalIndicatorsConfig.ema_long;
        pair.smallTerm.rsiPeriodVal = globalIndicatorsConfig.rsi_period;
        pair.smallTerm.macdFastVal = globalIndicatorsConfig.macd_fast;
        pair.smallTerm.macdSlowVal = globalIndicatorsConfig.macd_slow;
        pair.smallTerm.macdSignalVal = globalIndicatorsConfig.macd_signal;
        pair.smallTerm.adxPeriodVal = globalIndicatorsConfig.adx_period;
        pair.smallTerm.atrPeriodVal = globalIndicatorsConfig.atr_period;
        pair.smallTerm.squeezePeriodVal = globalIndicatorsConfig.squeeze_period;
        pair.smallTerm.analysisLimit = globalCandlesConfig.analysis_limit ?? 100;

        pair.mediumTerm.barDurationSec = 900;
        pair.mediumTerm.emaFastVal = globalIndicatorsConfig.ema_fast;
        pair.mediumTerm.emaMediumVal = globalIndicatorsConfig.ema_medium;
        pair.mediumTerm.emaSlowVal = globalIndicatorsConfig.ema_slow;
        pair.mediumTerm.emaLongVal = globalIndicatorsConfig.ema_long;
        pair.mediumTerm.rsiPeriodVal = globalIndicatorsConfig.rsi_period;
        pair.mediumTerm.macdFastVal = globalIndicatorsConfig.macd_fast;
        pair.mediumTerm.macdSlowVal = globalIndicatorsConfig.macd_slow;
        pair.mediumTerm.macdSignalVal = globalIndicatorsConfig.macd_signal;
        pair.mediumTerm.adxPeriodVal = globalIndicatorsConfig.adx_period;
        pair.mediumTerm.atrPeriodVal = globalIndicatorsConfig.atr_period;
        pair.mediumTerm.squeezePeriodVal = globalIndicatorsConfig.squeeze_period;
        pair.mediumTerm.analysisLimit = globalCandlesConfig.analysis_limit ?? 100;

        pair.largeTerm.barDurationSec = 3600;
        pair.largeTerm.emaFastVal = globalIndicatorsConfig.ema_fast;
        pair.largeTerm.emaMediumVal = globalIndicatorsConfig.ema_medium;
        pair.largeTerm.emaSlowVal = globalIndicatorsConfig.ema_slow;
        pair.largeTerm.emaLongVal = globalIndicatorsConfig.ema_long;
        pair.largeTerm.rsiPeriodVal = globalIndicatorsConfig.rsi_period;
        pair.largeTerm.macdFastVal = globalIndicatorsConfig.macd_fast;
        pair.largeTerm.macdSlowVal = globalIndicatorsConfig.macd_slow;
        pair.largeTerm.macdSignalVal = globalIndicatorsConfig.macd_signal;
        pair.largeTerm.adxPeriodVal = globalIndicatorsConfig.adx_period;
        pair.largeTerm.atrPeriodVal = globalIndicatorsConfig.atr_period;
        pair.largeTerm.squeezePeriodVal = globalIndicatorsConfig.squeeze_period;
        pair.largeTerm.analysisLimit = globalCandlesConfig.analysis_limit ?? 100;
    }
}

export function removeInstance(key: string) {
    delete instancesMap[key];
}

export function switchTab(key: string) {
    activeTab = key;
}

function autoLogTrade(pair: InstanceState, oldPosition: 'Long' | 'Short') {
    const entryPrice = parseFloat(pair.entryPriceVal);
    const exitPrice = parseFloat(pair.microTerm.priceText);

    if (isNaN(entryPrice) || isNaN(exitPrice) || entryPrice <= 0 || exitPrice <= 0) {
        console.warn("⚠️ Trade Logger Bypassed: Entry Price or Current Market Price is invalid.");
        return;
    }

    const stopLoss = parseFloat(pair.stopLossVal);
    let riskDistance = 0;
    if (!isNaN(stopLoss) && stopLoss > 0 && stopLoss !== entryPrice) {
        riskDistance = Math.abs(entryPrice - stopLoss);
    } else {
        riskDistance = entryPrice * 0.01;
    }

    let pnl = 0;
    if (oldPosition === 'Long') { pnl = exitPrice - entryPrice; }
    else { pnl = entryPrice - exitPrice; }

    const outcome = pnl >= 0 ? 'WIN' : 'LOSS';
    const rewardDistance = Math.abs(pnl);
    const rewardMultiplier = riskDistance > 0 ? (rewardDistance / riskDistance) : 1.0;

    const payload = {
        symbol: pair.symbol.toUpperCase(),
        direction: oldPosition,
        outcome,
        risk_multiplier: 1.0,
        reward_multiplier: parseFloat(rewardMultiplier.toFixed(2)),
    };

    fetch('/api/trades', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload),
    })
    .then(res => {
        if (res.ok) {
            console.log(`✅ Auto-Logged Trade: ${payload.symbol} ${payload.direction} ${payload.outcome} (R:R Ratio 1:${payload.reward_multiplier})`);
            fetch(`/api/trades?_=${Date.now()}`)
                .then(r => r.json())
                .then(data => { userTrades = data || []; })
                .catch(() => {});
        }
    })
    .catch(err => console.error("❌ Auto-Logger Network Error:", err));
}

export function getState() {
    const app = {
        initInstance(symbol: string, exchange?: string) { initInstance(symbol, exchange); },
        removeInstance(key: string) { removeInstance(key); },
        get instancesMap() { return instancesMap; },
        get activeTab() { return activeTab; },
        set activeTab(v: string) { activeTab = v; },

        get apiKeyConfigured() { return apiKeyConfigured; },
        set apiKeyConfigured(v: boolean) { apiKeyConfigured = v; },
        get rulesContent() { return rulesContent; },

        // Session state
        get sessionActive() { return sessionActive; },
        set sessionActive(v: boolean) { sessionActive = v; },
        get sessionChecked() { return sessionChecked; },
        get sessionMode() { return sessionMode; },
        get sessionCurrency() { return sessionCurrency; },
        get sessionExchange() { return sessionExchange; },
        get sessionCapital() { return sessionCapital; },
        set sessionCapital(v: number) { sessionCapital = v; },
        get sessionInstanceCount() { return sessionInstanceCount; },
        get sessionMaxInstances() { return sessionMaxInstances; },
        get sessionLoading() { return sessionLoading; },
        get sessionError() { return sessionError; },
        set sessionError(v: string | null) { sessionError = v; },
        initSession(mode: string, currency: string, exchange: string, capital: number) {
            return initSession(mode, currency, exchange, capital);
        },
        quitSession() { return quitSession(); },
        fetchSessionStatus() { return fetchSessionStatus(); },
        set rulesContent(v: string) { rulesContent = v; },

        // Multi-timeframe telemetry access
        get microTerm() { return activeInstance().microTerm; },
        get smallTerm() { return activeInstance().smallTerm; },
        get mediumTerm() { return activeInstance().mediumTerm; },
        get largeTerm() { return activeInstance().largeTerm; },

        // Backward-compatible accessors (proxied to micro-term by default)
        get activeSymbol() { return activeInstance().symbol; },
        get activeExchange() { return activeInstance().exchange; },
        get isConnected() { return activeInstance().isConnected; },
        set isConnected(v: boolean) { activeInstance().isConnected = v; },

        get priceText() { return micro().priceText; },
        set priceText(v: string) { micro().priceText = v; },
        get vwapText() { return micro().vwapText; },
        set vwapText(v: string) { micro().vwapText = v; },
        get avgVolText() { return micro().avgVolText; },
        set avgVolText(v: string) { micro().avgVolText = v; },
        get emaFastText() { return micro().emaFastText; },
        set emaFastText(v: string) { micro().emaFastText = v; },
        get emaMediumText() { return micro().emaMediumText; },
        set emaMediumText(v: string) { micro().emaMediumText = v; },
        get emaSlowText() { return micro().emaSlowText; },
        set emaSlowText(v: string) { micro().emaSlowText = v; },
        get emaLongText() { return micro().emaLongText; },
        set emaLongText(v: string) { micro().emaLongText = v; },
        get adxText() { return micro().adxText; },
        set adxText(v: string) { micro().adxText = v; },
        get adxPlusText() { return micro().adxPlusText; },
        set adxPlusText(v: string) { micro().adxPlusText = v; },
        get adxMinusText() { return micro().adxMinusText; },
        set adxMinusText(v: string) { micro().adxMinusText = v; },
        get atrText() { return micro().atrText; },
        set atrText(v: string) { micro().atrText = v; },
        get rsiText() { return micro().rsiText; },
        set rsiText(v: string) { micro().rsiText = v; },
        get macdLineText() { return micro().macdLineText; },
        set macdLineText(v: string) { micro().macdLineText = v; },
        get macdSigText() { return micro().macdSigText; },
        set macdSigText(v: string) { micro().macdSigText = v; },
        get macdHistText() { return micro().macdHistText; },
        set macdHistText(v: string) { micro().macdHistText = v; },
        get sqzValText() { return micro().sqzValText; },
        set sqzValText(v: string) { micro().sqzValText = v; },
        get sqzStatusText() { return micro().sqzStatusText; },
        set sqzStatusText(v: string) { micro().sqzStatusText = v; },
        get isSqueezeOn() { return micro().isSqueezeOn; },
        set isSqueezeOn(v: boolean) { micro().isSqueezeOn = v; },
        get volText() { return micro().volText; },
        set volText(v: string) { micro().volText = v; },
        get lastMacdHist() { return micro().lastMacdHist; },
        set lastMacdHist(v: number) { micro().lastMacdHist = v; },
        get lastSqzMom() { return micro().lastSqzMom; },
        set lastSqzMom(v: number) { micro().lastSqzMom = v; },
        get latestSnapshot() { return micro().latestSnapshot; },
        set latestSnapshot(v: Record<string, unknown> | null) { micro().latestSnapshot = v; },
        get historyPrices() { return micro().historyPrices; },
        set historyPrices(v: number[]) { micro().historyPrices = v; },

        // Show/hide toggles
        get showEmas() { return micro().showEmas; }, set showEmas(v: boolean) { micro().showEmas = v; },
        get showBb() { return micro().showBb; }, set showBb(v: boolean) { micro().showBb = v; },
        get showVwap() { return micro().showVwap; }, set showVwap(v: boolean) { micro().showVwap = v; },
        get showVolume() { return micro().showVolume; }, set showVolume(v: boolean) { micro().showVolume = v; },
        get showAdx() { return micro().showAdx; }, set showAdx(v: boolean) { micro().showAdx = v; },
        get showAtr() { return micro().showAtr; }, set showAtr(v: boolean) { micro().showAtr = v; },
        get showRsi() { return micro().showRsi; }, set showRsi(v: boolean) { micro().showRsi = v; },
        get showMacd() { return micro().showMacd; }, set showMacd(v: boolean) { micro().showMacd = v; },
        get showSqueeze() { return micro().showSqueeze; }, set showSqueeze(v: boolean) { micro().showSqueeze = v; },

        // Config values
        get barDurationSec() { return micro().barDurationSec; }, set barDurationSec(v: number) { micro().barDurationSec = v; },
        get emaFastVal() { return micro().emaFastVal; }, set emaFastVal(v: number) { micro().emaFastVal = v; },
        get emaMediumVal() { return micro().emaMediumVal; }, set emaMediumVal(v: number) { micro().emaMediumVal = v; },
        get emaSlowVal() { return micro().emaSlowVal; }, set emaSlowVal(v: number) { micro().emaSlowVal = v; },
        get emaLongVal() { return micro().emaLongVal; }, set emaLongVal(v: number) { micro().emaLongVal = v; },
        get rsiPeriodVal() { return micro().rsiPeriodVal; }, set rsiPeriodVal(v: number) { micro().rsiPeriodVal = v; },
        get macdFastVal() { return micro().macdFastVal; }, set macdFastVal(v: number) { micro().macdFastVal = v; },
        get macdSlowVal() { return micro().macdSlowVal; }, set macdSlowVal(v: number) { micro().macdSlowVal = v; },
        get macdSignalVal() { return micro().macdSignalVal; }, set macdSignalVal(v: number) { micro().macdSignalVal = v; },
        get adxPeriodVal() { return micro().adxPeriodVal; }, set adxPeriodVal(v: number) { micro().adxPeriodVal = v; },
        get atrPeriodVal() { return micro().atrPeriodVal; }, set atrPeriodVal(v: number) { micro().atrPeriodVal = v; },
        get squeezePeriodVal() { return micro().squeezePeriodVal; }, set squeezePeriodVal(v: number) { micro().squeezePeriodVal = v; },
        get analysisLimit() { return micro().analysisLimit; }, set analysisLimit(v: number) { micro().analysisLimit = v; },
        get candleTimeframeLabel() { const sec = micro().barDurationSec; if (sec % 3600 === 0) return `${sec / 3600}h`; if (sec % 60 === 0) return `${sec / 60}m`; return `${sec}s`; },

        // Labels
        get emaFastLabel() { return emaFastLabel; }, set emaFastLabel(v: string) { emaFastLabel = v; },
        get emaMediumLabel() { return emaMediumLabel; }, set emaMediumLabel(v: string) { emaMediumLabel = v; },
        get emaSlowLabel() { return emaSlowLabel; }, set emaSlowLabel(v: string) { emaSlowLabel = v; },
        get emaLongLabel() { return emaLongLabel; }, set emaLongLabel(v: string) { emaLongLabel = v; },
        get rsiLabel() { return rsiLabel; }, set rsiLabel(v: string) { rsiLabel = v; },
        get adxLabel() { return adxLabel; }, set adxLabel(v: string) { adxLabel = v; },
        get atrLabel() { return atrLabel; }, set atrLabel(v: string) { atrLabel = v; },
        get macdLabel() { return macdLabel; }, set macdLabel(v: string) { macdLabel = v; },

        // Assistant & analysis
        get assistantHistory() { return activeInstance().assistantHistory; },
        set assistantHistory(v: AssistantHistoryRecord[]) { activeInstance().assistantHistory = v; },
        get chatHistory() { return activeInstance().chatHistory; },
        set chatHistory(v: ChatMessage[]) { activeInstance().chatHistory = v; },
        get currentPosition() { return activeInstance().currentPosition; },
        set currentPosition(v: 'None' | 'Long' | 'Short') {
            const pair = activeInstance();
            const oldVal = pair.currentPosition;
            if (oldVal !== 'None' && v === 'None') {
                autoLogTrade(pair, oldVal);
                pair.entryPriceVal = '';
                pair.stopLossVal = '';
            }
            pair.currentPosition = v;
        },
        get entryPriceVal() { return activeInstance().entryPriceVal; },
        set entryPriceVal(v: string) { activeInstance().entryPriceVal = v; },
        get stopLossVal() { return activeInstance().stopLossVal; },
        set stopLossVal(v: string) { activeInstance().stopLossVal = v; },
        get assistantLoading() { return activeInstance().assistantLoading; },
        set assistantLoading(v: boolean) { activeInstance().assistantLoading = v; },
        get assistantError() { return activeInstance().assistantError; },
        set assistantError(v: string | null) { activeInstance().assistantError = v; },
        get assistantResponse() { return activeInstance().assistantResponse; },
        set assistantResponse(v: AssistantAnalysis | null) { activeInstance().assistantResponse = v; },
        get multiAgentResponse() { return activeInstance().multiAgentResponse; },
        set multiAgentResponse(v: MultiAgentAnalysis | null) { activeInstance().multiAgentResponse = v; },
        get analysisPhase() { return activeInstance().analysisPhase; },
        set analysisPhase(v: 'idle' | 'phase1' | 'phase2' | 'complete') { activeInstance().analysisPhase = v; },
        get individualResults() { return activeInstance().individualResults; },
        set individualResults(v: IndividualIndicatorResult[]) { activeInstance().individualResults = v; },
        get agentProgress() { return activeInstance().agentProgress; },
        set agentProgress(v: AgentProgress[]) { activeInstance().agentProgress = v; },
        get historyLatestClose() { return activeInstance().historyLatestClose; },
        set historyLatestClose(v: string) { activeInstance().historyLatestClose = v; },
        get isAssistantModalOpen() { return activeInstance().isAssistantModalOpen; },
        set isAssistantModalOpen(v: boolean) { activeInstance().isAssistantModalOpen = v; },
        get chatInputText() { return activeInstance().chatInputText; },
        set chatInputText(v: string) { activeInstance().chatInputText = v; },
        get isChatLoading() { return activeInstance().isChatLoading; },
        set isChatLoading(v: boolean) { activeInstance().isChatLoading = v; },
        get currentView() { return activeInstance().currentView; },
        set currentView(v) { activeInstance().currentView = v; },
        get userTrades() { return userTrades; },
        set userTrades(v: UserTrade[]) { userTrades = v; },

        get automationEnabled() { return activeInstance().automationEnabled; },
        set automationEnabled(v: boolean) { activeInstance().automationEnabled = v; },
        get automationIntervalValue() { return activeInstance().automationIntervalValue; },
        set automationIntervalValue(v: number) { activeInstance().automationIntervalValue = v; },
        get automationIntervalUnit() { return activeInstance().automationIntervalUnit; },
        set automationIntervalUnit(v: 'seconds' | 'minutes' | 'hours') { activeInstance().automationIntervalUnit = v; },
        get slowIntervalSecs() { return activeInstance().slowIntervalSecs; },
        set slowIntervalSecs(v: number) { activeInstance().slowIntervalSecs = v; },
        get normalIntervalSecs() { return activeInstance().normalIntervalSecs; },
        set normalIntervalSecs(v: number) { activeInstance().normalIntervalSecs = v; },
        get fastIntervalSecs() { return activeInstance().fastIntervalSecs; },
        set fastIntervalSecs(v: number) { activeInstance().fastIntervalSecs = v; },
        get tpLevels() { return activeInstance().tpLevels; },
        set tpLevels(v: number) { activeInstance().tpLevels = v; },
        get slLevels() { return activeInstance().slLevels; },
        set slLevels(v: number) { activeInstance().slLevels = v; },
        get nextEvaluationIn() { return activeInstance().nextEvaluationIn; },
        set nextEvaluationIn(v: string) { activeInstance().nextEvaluationIn = v; },

        get paperCashBalance() { return activeInstance().paperCashBalance; },
        set paperCashBalance(v: number) { activeInstance().paperCashBalance = v; },
        get paperInitialUSD() { return activeInstance().paperInitialUSD; },
        set paperInitialUSD(v: number) { activeInstance().paperInitialUSD = v; },
        get paperAllocationPct() { return activeInstance().paperAllocationPct; },
        set paperAllocationPct(v: number) { activeInstance().paperAllocationPct = v; },
        get paperAutoExecute() { return activeInstance().paperAutoExecute; },
        set paperAutoExecute(v: boolean) { activeInstance().paperAutoExecute = v; },
        get activePaperPosition() { return activeInstance().activePaperPosition; },
        set activePaperPosition(v: Record<string, unknown> | null) { activeInstance().activePaperPosition = v; },
        get paperUnrealizedPnl() { return activeInstance().paperUnrealizedPnl; },
        set paperUnrealizedPnl(v: number) { activeInstance().paperUnrealizedPnl = v; },
        get paperUnrealizedRoi() { return activeInstance().paperUnrealizedRoi; },
        set paperUnrealizedRoi(v: number) { activeInstance().paperUnrealizedRoi = v; },
        get paperTotalAccountValue() { return activeInstance().paperTotalAccountValue; },
        set paperTotalAccountValue(v: number) { activeInstance().paperTotalAccountValue = v; },
        get paperMarginUsed() { return activeInstance().paperMarginUsed; },
        set paperMarginUsed(v: number) { activeInstance().paperMarginUsed = v; },
        get paperMaxTrades() { return activeInstance().paperMaxTrades; },
        set paperMaxTrades(v: number) { activeInstance().paperMaxTrades = v; },
        get paperActiveTrades() { return activeInstance().paperActiveTrades; },
        set paperActiveTrades(v: number) { activeInstance().paperActiveTrades = v; },
        get paperAvailableTrades() { return activeInstance().paperAvailableTrades; },
        set paperAvailableTrades(v: number) { activeInstance().paperAvailableTrades = v; },
        get paperHistory() { return activeInstance().paperHistory; },
        set paperHistory(v: Record<string, unknown>[]) { activeInstance().paperHistory = v; },
        get paperLoading() { return activeInstance().paperLoading; },
        set paperLoading(v: boolean) { activeInstance().paperLoading = v; },

        get totalPointsScore() { return activeInstance().totalPointsScore; },
        set totalPointsScore(v: number) { activeInstance().totalPointsScore = v; },
        get allocatedCapitalPct() { return activeInstance().allocatedCapitalPct; },
        set allocatedCapitalPct(v: number) { activeInstance().allocatedCapitalPct = v; },
        get activeOppositeSignalsCount() { return activeInstance().activeOppositeSignalsCount; },
        set activeOppositeSignalsCount(v: number) { activeInstance().activeOppositeSignalsCount = v; },
        get markedSupportLevels() { return activeInstance().markedSupportLevels; },
        set markedSupportLevels(v: number[]) { activeInstance().markedSupportLevels = v; },
        get markedResistanceLevels() { return activeInstance().markedResistanceLevels; },
        set markedResistanceLevels(v: number[]) { activeInstance().markedResistanceLevels = v; },
        get srFlipEvents() { return activeInstance().srFlipEvents; },
        set srFlipEvents(v: string) { activeInstance().srFlipEvents = v; },

        get systemHeartbeat() { return systemHeartbeat; },
        get recentDecisions() { return recentDecisions; },
        get completedTrades() { return completedTrades; },

        get costPriceInput() { return activeInstance().costPriceInput; },
        set costPriceInput(v: number) { activeInstance().costPriceInput = v; },
        get costPriceOutput() { return activeInstance().costPriceOutput; },
        set costPriceOutput(v: number) { activeInstance().costPriceOutput = v; },
        get costIntervalSecs() { return activeInstance().costIntervalSecs; },
        set costIntervalSecs(v: number) { activeInstance().costIntervalSecs = v; },
        get costRunsPerDay() { return activeInstance().costRunsPerDay; },
        set costRunsPerDay(v: number) { activeInstance().costRunsPerDay = v; },
        get costTokensPerRunInput() { return activeInstance().costTokensPerRunInput; },
        set costTokensPerRunInput(v: number) { activeInstance().costTokensPerRunInput = v; },
        get costTokensPerRunOutput() { return activeInstance().costTokensPerRunOutput; },
        set costTokensPerRunOutput(v: number) { activeInstance().costTokensPerRunOutput = v; },
        get costDailyProjected() { return activeInstance().costDailyProjected; },
        set costDailyProjected(v: number) { activeInstance().costDailyProjected = v; },
        get costWeeklyProjected() { return activeInstance().costWeeklyProjected; },
        set costWeeklyProjected(v: number) { activeInstance().costWeeklyProjected = v; },
        get costMonthlyProjected() { return activeInstance().costMonthlyProjected; },
        set costMonthlyProjected(v: number) { activeInstance().costMonthlyProjected = v; },
        get costActualInputTokens() { return activeInstance().costActualInputTokens; },
        set costActualInputTokens(v: number) { activeInstance().costActualInputTokens = v; },
        get costActualOutputTokens() { return activeInstance().costActualOutputTokens; },
        set costActualOutputTokens(v: number) { activeInstance().costActualOutputTokens = v; },
        get costActualTotal() { return activeInstance().costActualTotal; },
        set costActualTotal(v: number) { activeInstance().costActualTotal = v; },
        get costLoading() { return activeInstance().costLoading; },
        set costLoading(v: boolean) { activeInstance().costLoading = v; },

        async fetchCostEstimate() {
            const pair = activeInstance();
            pair.costLoading = true;
            try {
                const res = await fetch(`/api/cost-estimate?pair_key=${encodeURIComponent(activeTab)}`);
                if (res.ok) {
                    const data = await res.json();
                    pair.costPriceInput = data.price_per_1m_input_tokens ?? 0.27;
                    pair.costPriceOutput = data.price_per_1m_output_tokens ?? 1.10;
                    pair.costIntervalSecs = data.interval_seconds ?? 900;
                    pair.costRunsPerDay = data.runs_per_day ?? 0;
                    pair.costTokensPerRunInput = data.input_tokens_per_run ?? 0;
                    pair.costTokensPerRunOutput = data.output_tokens_per_run ?? 0;
                    pair.costDailyProjected = data.projected_daily_cost ?? 0;
                    pair.costWeeklyProjected = data.projected_weekly_cost ?? 0;
                    pair.costMonthlyProjected = data.projected_monthly_cost ?? 0;
                    pair.costActualInputTokens = data.actual_input_tokens_used ?? 0;
                    pair.costActualOutputTokens = data.actual_output_tokens_used ?? 0;
                    pair.costActualTotal = data.actual_total_cost ?? 0;
                }
            } catch (_) {} finally { pair.costLoading = false; }
        },

        async fetchPaperStatus() {
            const pair = activeInstance();
            try {
                const res = await fetch(`/api/paper/status?symbol=${encodeURIComponent(activeTab)}`);
                if (!res.ok) return;
                const data = await res.json();
                pair.paperCashBalance = data.current_cash ?? 10000;
                pair.paperInitialUSD = data.initial_usd ?? 10000;
                pair.paperAllocationPct = data.allocation_pct ?? 10;
                pair.paperAutoExecute = data.auto_execute ?? false;
                pair.activePaperPosition = data.active_position ?? null;
                pair.paperUnrealizedPnl = data.unrealized_pnl ?? 0;
                pair.paperUnrealizedRoi = data.unrealized_roi_pct ?? 0;
                pair.paperTotalAccountValue = data.total_account_value ?? 10000;
                pair.paperMarginUsed = data.margin_used ?? 0;
                pair.paperMaxTrades = data.max_trades ?? 10;
                pair.paperActiveTrades = data.active_trades ?? 0;
                pair.paperAvailableTrades = data.available_trades ?? 10;
                pair.paperScaleInPortions = data.scale_in_portions ?? [];
                pair.paperTakeProfitTargets = data.take_profit_targets ?? [];
                pair.paperAvgEntryPrice = data.active_position?.average_entry_price ?? data.active_position?.entry_price ?? 0;
                pair.paperInvalidationLevel = data.active_position?.final_invalidation_level ?? 0;
                pair.paperFilledPortions = data.active_position?.current_portions ?? 0;
                pair.paperMaxRiskPct = data.max_risk_pct ?? 2.0;
                pair.paperLeverage = data.leverage ?? 20;
                pair.paperAutoExecuteIntervals = data.auto_execute_intervals ?? 15;
                pair.paperLookbackTrades = data.lookback_trades ?? 10;
            } catch (_) {}
        },

        async openPaperPosition(direction: 'LONG' | 'SHORT') {
            const pair = activeInstance();
            pair.paperLoading = true;
            try {
                const res = await fetch('/api/paper/order', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ symbol: activeTab, direction, action: 'OPEN' }),
                });
                if (res.ok) await (app as any).fetchPaperStatus();
            } catch (_) {} finally { pair.paperLoading = false; }
        },

        async closePaperPosition() {
            const pair = activeInstance();
            pair.paperLoading = true;
            try {
                const res = await fetch('/api/paper/order', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ symbol: activeTab, direction: '', action: 'CLOSE' }),
                });
                if (res.ok) await (app as any).fetchPaperStatus();
            } catch (_) {} finally { pair.paperLoading = false; }
        },

        async resetPaperAccount() {
            try {
                await fetch('/api/paper/reset', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ symbol: activeTab }) });
                await (app as any).fetchPaperStatus();
            } catch (_) {}
        },

        async savePaperConfig(initialUSD: number, allocationPct: number, autoExecute: boolean) {
            try {
                const pair = activeInstance();
                await fetch('/api/paper/config', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        symbol: activeTab,
                        initial_usd: initialUSD,
                        allocation_pct: allocationPct,
                        auto_execute: autoExecute,
                        max_risk_pct: pair.paperMaxRiskPct,
                        leverage: pair.paperLeverage,
                        auto_execute_intervals: pair.paperAutoExecuteIntervals,
                        lookback_trades: pair.paperLookbackTrades,
                    })
                });
                await (app as any).fetchPaperStatus();
            } catch (_) {}
        },

        async fetchPaperHistory(symbol?: string) {
            const pair = activeInstance();
            try {
                const url = symbol ? `/api/paper/performance?symbol=${encodeURIComponent(symbol)}` : '/api/paper/performance';
                const res = await fetch(url);
                if (res.ok) { const data = await res.json(); pair.paperHistory = data.trades || []; }
            } catch (_) {}
        },

        async fetchTrades() {
            try {
                const res = await fetch(`/api/trades?_=${Date.now()}`);
                if (res.ok) { const data = await res.json(); userTrades = data || []; }
            } catch (e) { console.error("Failed to fetch user trades:", e); }
        },

        get globalCandlesConfig() { return globalCandlesConfig; },
        set globalCandlesConfig(v) { globalCandlesConfig = v; },
        get globalIndicatorsConfig() { return globalIndicatorsConfig; },
        set globalIndicatorsConfig(v) { globalIndicatorsConfig = v; },

        get activeDecisionProfileId() { return activeDecisionProfileId; },
        set activeDecisionProfileId(v: number) { activeDecisionProfileId = v; },
        get decisionProfiles() { return decisionProfiles; },
        set decisionProfiles(v: DecisionProfile[]) { decisionProfiles = v; },
        get calculatedDecisionScore() { return calculatedDecisionScore; },
        set calculatedDecisionScore(v: DecisionScore | null) { calculatedDecisionScore = v; },
        get decisionLoading() { return decisionLoading; },
        set decisionLoading(v: boolean) { decisionLoading = v; },

        async fetchDecisionProfiles() {
            try {
                const res = await fetch('/api/decision-profiles');
                if (res.ok) { decisionProfiles = await res.json(); }
            } catch (_) {}
        },

        async createDecisionProfile(name: string, longT: number, shortT: number) {
            try {
                await fetch('/api/decision-profiles', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ profile_name: name, long_threshold: longT, short_threshold: shortT }) });
                await (app as any).fetchDecisionProfiles();
            } catch (_) {}
        },

        async deleteDecisionProfile(id: number) {
            try {
                await fetch(`/api/decision-profiles/${id}`, { method: 'DELETE' });
                await (app as any).fetchDecisionProfiles();
            } catch (_) {}
        },

        async updateDecisionProfileThresholds(id: number, longT: number, shortT: number) {
            try {
                await fetch(`/api/decision-profiles/${id}`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ long_threshold: longT, short_threshold: shortT }) });
                await (app as any).fetchDecisionProfiles();
            } catch (_) {}
        },

        async addProfileIndicator(profileId: number, name: string, weight: number, overrideStatus: string) {
            try {
                await fetch(`/api/decision-profiles/${profileId}/indicators`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ indicator_name: name, weight, override_status: overrideStatus }) });
                await (app as any).fetchDecisionProfiles();
            } catch (_) {}
        },

        async updateProfileIndicator(profileId: number, indicatorId: number, weight: number, overrideStatus: string) {
            try {
                await fetch(`/api/decision-profiles/${profileId}/indicators/${indicatorId}`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ weight, override_status: overrideStatus }) });
                await (app as any).fetchDecisionProfiles();
            } catch (_) {}
        },

        async deleteProfileIndicator(profileId: number, indicatorId: number) {
            try {
                await fetch(`/api/decision-profiles/${profileId}/indicators/${indicatorId}`, { method: 'DELETE' });
                await (app as any).fetchDecisionProfiles();
            } catch (_) {}
        },

        async evaluateDecision(profileId: number) {
            decisionLoading = true;
            try {
                const pair = activeInstance();
                const res = await fetch(`/api/decision-profiles/${profileId}/evaluate`, {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        symbol: activeTab,
                        latest_snapshot: pair.microTerm.latestSnapshot,
                    }),
                });
                if (res.ok) { calculatedDecisionScore = await res.json(); }
            } catch (_) {} finally { decisionLoading = false; }
        },

        get activeRiskProfileId() { return activeRiskProfileId; },
        set activeRiskProfileId(v: number) { activeRiskProfileId = v; },
        get riskProfiles() { return riskProfiles; },
        set riskProfiles(v: RiskProfile[]) { riskProfiles = v; },
        get riskDirection() { return riskDirection; },
        set riskDirection(v: 'LONG' | 'SHORT') { riskDirection = v; },
        get riskEntryPrice() { return riskEntryPrice; },
        set riskEntryPrice(v: string) { riskEntryPrice = v; },
        get riskStopLoss() { return riskStopLoss; },
        set riskStopLoss(v: string) { riskStopLoss = v; },
        get riskTakeProfit() { return riskTakeProfit; },
        set riskTakeProfit(v: string) { riskTakeProfit = v; },
        get riskCalculation() { return riskCalculation; },
        set riskCalculation(v: RiskCalculation | null) { riskCalculation = v; },
        get riskCalculating() { return riskCalculating; },
        get useDynamicAtr() { return useDynamicAtr; }, set useDynamicAtr(v: boolean) { useDynamicAtr = v; },
        get atrValue() { return atrValue; }, set atrValue(v: number) { atrValue = v; },
        set riskCalculating(v: boolean) { riskCalculating = v; },

        async fetchRiskProfiles() {
            try {
                const res = await fetch('/api/risk-profiles');
                if (res.ok) { riskProfiles = await res.json(); }
            } catch (_) {}
        },

        async createRiskProfile(name: string, capital: number, riskPct: number, leverage: number) {
            try {
                await fetch('/api/risk-profiles', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ profile_name: name, capital, max_risk_pct: riskPct, leverage }) });
                await (app as any).fetchRiskProfiles();
            } catch (_) {}
        },

        async deleteRiskProfile(id: number) {
            try {
                await fetch(`/api/risk-profiles/${id}`, { method: 'DELETE' });
                await (app as any).fetchRiskProfiles();
            } catch (_) {}
        },

        async calculateRisk() {
            riskCalculating = true;
            try {
                const res = await fetch('/api/risk/calculate', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        profile_id: activeRiskProfileId,
                        direction: riskDirection,
                        entry_price: parseFloat(riskEntryPrice) || 0,
                        stop_loss: parseFloat(riskStopLoss) || 0,
                        take_profit: parseFloat(riskTakeProfit) || 0,
                    }),
                });
                if (res.ok) { riskCalculation = await res.json(); }
            } catch (_) {} finally { riskCalculating = false; }
        },

        get commissionDirection() { return commissionDirection; },
        set commissionDirection(v: 'LONG' | 'SHORT') { commissionDirection = v; },
        get commissionEntry1() { return commissionEntry1; },
        set commissionEntry1(v: string) { commissionEntry1 = v; },
        get commissionEntry2() { return commissionEntry2; },
        set commissionEntry2(v: string) { commissionEntry2 = v; },
        get commissionSL1() { return commissionSL1; },
        set commissionSL1(v: string) { commissionSL1 = v; },
        get commissionSL2() { return commissionSL2; },
        set commissionSL2(v: string) { commissionSL2 = v; },
        get commissionTP1() { return commissionTP1; },
        set commissionTP1(v: string) { commissionTP1 = v; },
        get commissionTP2() { return commissionTP2; },
        set commissionTP2(v: string) { commissionTP2 = v; },
        get commissionCapitalSplit() { return commissionCapitalSplit; },
        set commissionCapitalSplit(v: number) { commissionCapitalSplit = v; },
        get commissionOrderType() { return commissionOrderType; },
        set commissionOrderType(v: 'maker' | 'taker') { commissionOrderType = v; },
        get commissionProjection() { return commissionProjection; },
        set commissionProjection(v: CommissionProjection | null) { commissionProjection = v; },
        get commissionLoading() { return commissionLoading; },
        set commissionLoading(v: boolean) { commissionLoading = v; },
        get feeTable() { return feeTable; },
        set feeTable(v: FeeTableRow[]) { feeTable = v; },

        async fetchFeeTable() {
            feeTableLoading = true;
            try {
                const res = await fetch(`/api/risk/fee-table?order_type=${commissionOrderType}`);
                if (res.ok) { feeTable = await res.json(); }
            } catch (_) {} finally { feeTableLoading = false; }
        },

        async calculateCommissionProjection() {
            commissionLoading = true;
            try {
                const res = await fetch('/api/risk/commission-projection', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        profile_id: activeRiskProfileId,
                        direction: commissionDirection,
                        entry_1: parseFloat(commissionEntry1) || 0,
                        entry_2: parseFloat(commissionEntry2) || 0,
                        stop_loss_1: parseFloat(commissionSL1) || 0,
                        stop_loss_2: parseFloat(commissionSL2) || 0,
                        take_profit_1: parseFloat(commissionTP1) || 0,
                        take_profit_2: parseFloat(commissionTP2) || 0,
                        capital_entry_1_pct: commissionCapitalSplit,
                        order_type: commissionOrderType,
                    }),
                });
                if (res.ok) { commissionProjection = await res.json(); }
            } catch (_) {} finally { commissionLoading = false; }
        },

        get exchangeAccounts() { return exchangeAccounts; },
        set exchangeAccounts(v: ExchangeAccount[]) { exchangeAccounts = v; },
        get exchangeFormDraft() { return exchangeFormDraft; },
        set exchangeFormDraft(v) { exchangeFormDraft = v; },
        get exchangeMaxAccounts() { return exchangeMaxAccounts; },
        get exchangeActiveCount() { return exchangeActiveCount; },
        set exchangeActiveCount(v: number) { exchangeActiveCount = v; },

        async fetchExchangeKeys() {
            try {
                const res = await fetch('/api/exchange-keys');
                if (res.ok) { exchangeAccounts = await res.json(); exchangeActiveCount = exchangeAccounts.filter(a => a.is_active).length; }
            } catch (_) {}
        },

        async addExchangeKey() {
            try {
                const res = await fetch('/api/exchange-keys', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(exchangeFormDraft) });
                if (res.ok) { exchangeFormDraft = { exchange: 'Bitget', account_name: '', api_key: '', api_secret: '', passphrase: '', referred_uid: '', is_active: true }; await (app as any).fetchExchangeKeys(); }
            } catch (_) {}
        },

        async deleteExchangeKey(id: number) {
            try {
                await fetch(`/api/exchange-keys/${id}`, { method: 'DELETE' });
                await (app as any).fetchExchangeKeys();
            } catch (_) {}
        },

        get dashboardStats() { return dashboardStats; },
        set dashboardStats(v: DashboardStats | null) { dashboardStats = v; },
        get dashboardActiveFilter() { return dashboardActiveFilter; },
        set dashboardActiveFilter(v: string) { dashboardActiveFilter = v; },
        get dashboardPeriod() { return dashboardPeriod; },
        set dashboardPeriod(v: string) { dashboardPeriod = v; },
        get dashboardOrigin() { return dashboardOrigin; },
        set dashboardOrigin(v: string) { dashboardOrigin = v; },
        get tradeLedgerRecords() { return tradeLedgerRecords; },
        set tradeLedgerRecords(v: TradeLedgerRecord[]) { tradeLedgerRecords = v; },

        get tradeJournalRecords() { return tradeJournalRecords; },
        set tradeJournalRecords(v: TradeJournalRecord[]) { tradeJournalRecords = v; },
        get journalLookbackDepth() { return journalLookbackDepth; },
        set journalLookbackDepth(v: number) { journalLookbackDepth = v; },

        async fetchDashboardStats() {
            try {
                const res = await fetch('/api/dashboard/stats');
                if (res.ok) { dashboardStats = await res.json(); }
            } catch (_) {}
        },

        async fetchTradeLedger() {
            try {
                const res = await fetch('/api/trade-ledger');
                if (res.ok) { tradeLedgerRecords = await res.json(); }
            } catch (_) {}
        },

        async fetchTradeJournal(limit: number = 50) {
            try {
                const res = await fetch(`/api/trade-journal?limit=${limit}`);
                if (res.ok) { tradeJournalRecords = await res.json(); }
            } catch (_) {}
        },

        async updateJournalNotes(id: number, notes: string, score: number) {
            try {
                const res = await fetch(`/api/trade-journal/${id}/notes`, {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ human_notes: notes, execution_score: score }),
                });
                if (res.ok) {
                    const idx = tradeJournalRecords.findIndex(r => r.id === id);
                    if (idx >= 0) {
                        tradeJournalRecords[idx].human_notes = notes;
                        tradeJournalRecords[idx].execution_score = score;
                        tradeJournalRecords = [...tradeJournalRecords];
                    }
                }
            } catch (_) {}
        },

        async fetchSystemStatus() {
            try {
                const res = await fetch('/api/system/status');
                if (res.ok) {
                    systemHeartbeat = await res.json();
                }
            } catch (e) {
                console.error("Failed to fetch system heartbeat:", e);
            }
        },

        async fetchObservabilityBuffers(symbol: string) {
            try {
                const res = await fetch(`/api/system/observability?symbol=${encodeURIComponent(symbol)}`);
                if (res.ok) {
                    const data = await res.json();
                    recentDecisions = data.recent_decisions || [];
                    completedTrades = data.completed_trades || [];
                }
            } catch (e) {
                console.error("Failed to fetch observability buffers:", e);
            }
        },

        exportJournalCSV() {
            window.open('/api/trade-journal/export/csv', '_blank');
        },

        exportJournalJSON() {
            window.open('/api/trade-journal/export/json', '_blank');
        },
    };
    return app;
}
