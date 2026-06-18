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

export interface PairState {
    symbol: string;
    exchange: string;
    isConnected: boolean;
    midTerm: TimeframeTelemetry;
    longTerm: TimeframeTelemetry;
    macroTerm: TimeframeTelemetry;
    supermacroTerm: TimeframeTelemetry;
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

function createTimeframeTelemetry(symbol: string, exchange: string, barDurationSec: number): TimeframeTelemetry {
    return {
        symbol,
        exchange,
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

function createPairState(symbol: string, exchange: string): PairState {
    return {
        symbol,
        exchange,
        isConnected: false,
        midTerm: createTimeframeTelemetry(symbol, exchange, 60),
        longTerm: createTimeframeTelemetry(symbol, exchange, 300),
        macroTerm: createTimeframeTelemetry(symbol, exchange, 900),
        supermacroTerm: createTimeframeTelemetry(symbol, exchange, 3600),
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

let pairsMap = $state<Record<string, PairState>>({});
let activeTab = $state<string>('Hyperliquid-BTC');

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

function activePair(): PairState {
    if (!pairsMap[activeTab]) {
        const parts = activeTab.split('-');
        pairsMap[activeTab] = createPairState(parts[1] || 'BTC', parts[0] || 'Hyperliquid');
    }
    return pairsMap[activeTab];
}

// Helper: get the mid-term state (default for backward-compatible accessors)
function mid(): TimeframeTelemetry { return activePair().midTerm; }

export function initPair(symbol: string, exchange: string = 'Hyperliquid') {
    const key = `${exchange}-${symbol}`;
    if (!pairsMap[key]) {
        pairsMap[key] = createPairState(symbol, exchange);
    } else {
        const pair = pairsMap[key];
        pair.midTerm.barDurationSec = 60;
        pair.midTerm.emaFastVal = globalIndicatorsConfig.ema_fast;
        pair.midTerm.emaMediumVal = globalIndicatorsConfig.ema_medium;
        pair.midTerm.emaSlowVal = globalIndicatorsConfig.ema_slow;
        pair.midTerm.emaLongVal = globalIndicatorsConfig.ema_long;
        pair.midTerm.rsiPeriodVal = globalIndicatorsConfig.rsi_period;
        pair.midTerm.macdFastVal = globalIndicatorsConfig.macd_fast;
        pair.midTerm.macdSlowVal = globalIndicatorsConfig.macd_slow;
        pair.midTerm.macdSignalVal = globalIndicatorsConfig.macd_signal;
        pair.midTerm.adxPeriodVal = globalIndicatorsConfig.adx_period;
        pair.midTerm.atrPeriodVal = globalIndicatorsConfig.atr_period;
        pair.midTerm.squeezePeriodVal = globalIndicatorsConfig.squeeze_period;
        pair.midTerm.analysisLimit = globalCandlesConfig.analysis_limit ?? 100;

        pair.longTerm.barDurationSec = 300;
        pair.longTerm.emaFastVal = globalIndicatorsConfig.ema_fast;
        pair.longTerm.emaMediumVal = globalIndicatorsConfig.ema_medium;
        pair.longTerm.emaSlowVal = globalIndicatorsConfig.ema_slow;
        pair.longTerm.emaLongVal = globalIndicatorsConfig.ema_long;
        pair.longTerm.rsiPeriodVal = globalIndicatorsConfig.rsi_period;
        pair.longTerm.macdFastVal = globalIndicatorsConfig.macd_fast;
        pair.longTerm.macdSlowVal = globalIndicatorsConfig.macd_slow;
        pair.longTerm.macdSignalVal = globalIndicatorsConfig.macd_signal;
        pair.longTerm.adxPeriodVal = globalIndicatorsConfig.adx_period;
        pair.longTerm.atrPeriodVal = globalIndicatorsConfig.atr_period;
        pair.longTerm.squeezePeriodVal = globalIndicatorsConfig.squeeze_period;
        pair.longTerm.analysisLimit = globalCandlesConfig.analysis_limit ?? 100;

        pair.macroTerm.barDurationSec = 900;
        pair.macroTerm.emaFastVal = globalIndicatorsConfig.ema_fast;
        pair.macroTerm.emaMediumVal = globalIndicatorsConfig.ema_medium;
        pair.macroTerm.emaSlowVal = globalIndicatorsConfig.ema_slow;
        pair.macroTerm.emaLongVal = globalIndicatorsConfig.ema_long;
        pair.macroTerm.rsiPeriodVal = globalIndicatorsConfig.rsi_period;
        pair.macroTerm.macdFastVal = globalIndicatorsConfig.macd_fast;
        pair.macroTerm.macdSlowVal = globalIndicatorsConfig.macd_slow;
        pair.macroTerm.macdSignalVal = globalIndicatorsConfig.macd_signal;
        pair.macroTerm.adxPeriodVal = globalIndicatorsConfig.adx_period;
        pair.macroTerm.atrPeriodVal = globalIndicatorsConfig.atr_period;
        pair.macroTerm.squeezePeriodVal = globalIndicatorsConfig.squeeze_period;
        pair.macroTerm.analysisLimit = globalCandlesConfig.analysis_limit ?? 100;

        pair.supermacroTerm.barDurationSec = 3600;
        pair.supermacroTerm.emaFastVal = globalIndicatorsConfig.ema_fast;
        pair.supermacroTerm.emaMediumVal = globalIndicatorsConfig.ema_medium;
        pair.supermacroTerm.emaSlowVal = globalIndicatorsConfig.ema_slow;
        pair.supermacroTerm.emaLongVal = globalIndicatorsConfig.ema_long;
        pair.supermacroTerm.rsiPeriodVal = globalIndicatorsConfig.rsi_period;
        pair.supermacroTerm.macdFastVal = globalIndicatorsConfig.macd_fast;
        pair.supermacroTerm.macdSlowVal = globalIndicatorsConfig.macd_slow;
        pair.supermacroTerm.macdSignalVal = globalIndicatorsConfig.macd_signal;
        pair.supermacroTerm.adxPeriodVal = globalIndicatorsConfig.adx_period;
        pair.supermacroTerm.atrPeriodVal = globalIndicatorsConfig.atr_period;
        pair.supermacroTerm.squeezePeriodVal = globalIndicatorsConfig.squeeze_period;
        pair.supermacroTerm.analysisLimit = globalCandlesConfig.analysis_limit ?? 100;
    }
}

export function removePair(key: string) {
    delete pairsMap[key];
}

export function switchTab(key: string) {
    activeTab = key;
}

function autoLogTrade(pair: PairState, oldPosition: 'Long' | 'Short') {
    const entryPrice = parseFloat(pair.entryPriceVal);
    const exitPrice = parseFloat(pair.midTerm.priceText);

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
        initPair(symbol: string, exchange: string = 'Hyperliquid') { initPair(symbol, exchange); },
        removePair(key: string) { removePair(key); },
        get pairsMap() { return pairsMap; },
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
        get midTerm() { return activePair().midTerm; },
        get longTerm() { return activePair().longTerm; },
        get macroTerm() { return activePair().macroTerm; },
        get supermacroTerm() { return activePair().supermacroTerm; },

        // Backward-compatible accessors (proxied to mid-term by default)
        get activeSymbol() { return activePair().symbol; },
        get activeExchange() { return activePair().exchange; },
        get isConnected() { return activePair().isConnected; },
        set isConnected(v: boolean) { activePair().isConnected = v; },

        get priceText() { return mid().priceText; },
        set priceText(v: string) { mid().priceText = v; },
        get vwapText() { return mid().vwapText; },
        set vwapText(v: string) { mid().vwapText = v; },
        get avgVolText() { return mid().avgVolText; },
        set avgVolText(v: string) { mid().avgVolText = v; },
        get emaFastText() { return mid().emaFastText; },
        set emaFastText(v: string) { mid().emaFastText = v; },
        get emaMediumText() { return mid().emaMediumText; },
        set emaMediumText(v: string) { mid().emaMediumText = v; },
        get emaSlowText() { return mid().emaSlowText; },
        set emaSlowText(v: string) { mid().emaSlowText = v; },
        get emaLongText() { return mid().emaLongText; },
        set emaLongText(v: string) { mid().emaLongText = v; },
        get adxText() { return mid().adxText; },
        set adxText(v: string) { mid().adxText = v; },
        get adxPlusText() { return mid().adxPlusText; },
        set adxPlusText(v: string) { mid().adxPlusText = v; },
        get adxMinusText() { return mid().adxMinusText; },
        set adxMinusText(v: string) { mid().adxMinusText = v; },
        get atrText() { return mid().atrText; },
        set atrText(v: string) { mid().atrText = v; },
        get rsiText() { return mid().rsiText; },
        set rsiText(v: string) { mid().rsiText = v; },
        get macdLineText() { return mid().macdLineText; },
        set macdLineText(v: string) { mid().macdLineText = v; },
        get macdSigText() { return mid().macdSigText; },
        set macdSigText(v: string) { mid().macdSigText = v; },
        get macdHistText() { return mid().macdHistText; },
        set macdHistText(v: string) { mid().macdHistText = v; },
        get sqzValText() { return mid().sqzValText; },
        set sqzValText(v: string) { mid().sqzValText = v; },
        get sqzStatusText() { return mid().sqzStatusText; },
        set sqzStatusText(v: string) { mid().sqzStatusText = v; },
        get isSqueezeOn() { return mid().isSqueezeOn; },
        set isSqueezeOn(v: boolean) { mid().isSqueezeOn = v; },
        get volText() { return mid().volText; },
        set volText(v: string) { mid().volText = v; },
        get lastMacdHist() { return mid().lastMacdHist; },
        set lastMacdHist(v: number) { mid().lastMacdHist = v; },
        get lastSqzMom() { return mid().lastSqzMom; },
        set lastSqzMom(v: number) { mid().lastSqzMom = v; },
        get latestSnapshot() { return mid().latestSnapshot; },
        set latestSnapshot(v: Record<string, unknown> | null) { mid().latestSnapshot = v; },
        get historyPrices() { return mid().historyPrices; },
        set historyPrices(v: number[]) { mid().historyPrices = v; },

        // Show/hide toggles
        get showEmas() { return mid().showEmas; }, set showEmas(v: boolean) { mid().showEmas = v; },
        get showBb() { return mid().showBb; }, set showBb(v: boolean) { mid().showBb = v; },
        get showVwap() { return mid().showVwap; }, set showVwap(v: boolean) { mid().showVwap = v; },
        get showVolume() { return mid().showVolume; }, set showVolume(v: boolean) { mid().showVolume = v; },
        get showAdx() { return mid().showAdx; }, set showAdx(v: boolean) { mid().showAdx = v; },
        get showAtr() { return mid().showAtr; }, set showAtr(v: boolean) { mid().showAtr = v; },
        get showRsi() { return mid().showRsi; }, set showRsi(v: boolean) { mid().showRsi = v; },
        get showMacd() { return mid().showMacd; }, set showMacd(v: boolean) { mid().showMacd = v; },
        get showSqueeze() { return mid().showSqueeze; }, set showSqueeze(v: boolean) { mid().showSqueeze = v; },

        // Config values
        get barDurationSec() { return mid().barDurationSec; }, set barDurationSec(v: number) { mid().barDurationSec = v; },
        get emaFastVal() { return mid().emaFastVal; }, set emaFastVal(v: number) { mid().emaFastVal = v; },
        get emaMediumVal() { return mid().emaMediumVal; }, set emaMediumVal(v: number) { mid().emaMediumVal = v; },
        get emaSlowVal() { return mid().emaSlowVal; }, set emaSlowVal(v: number) { mid().emaSlowVal = v; },
        get emaLongVal() { return mid().emaLongVal; }, set emaLongVal(v: number) { mid().emaLongVal = v; },
        get rsiPeriodVal() { return mid().rsiPeriodVal; }, set rsiPeriodVal(v: number) { mid().rsiPeriodVal = v; },
        get macdFastVal() { return mid().macdFastVal; }, set macdFastVal(v: number) { mid().macdFastVal = v; },
        get macdSlowVal() { return mid().macdSlowVal; }, set macdSlowVal(v: number) { mid().macdSlowVal = v; },
        get macdSignalVal() { return mid().macdSignalVal; }, set macdSignalVal(v: number) { mid().macdSignalVal = v; },
        get adxPeriodVal() { return mid().adxPeriodVal; }, set adxPeriodVal(v: number) { mid().adxPeriodVal = v; },
        get atrPeriodVal() { return mid().atrPeriodVal; }, set atrPeriodVal(v: number) { mid().atrPeriodVal = v; },
        get squeezePeriodVal() { return mid().squeezePeriodVal; }, set squeezePeriodVal(v: number) { mid().squeezePeriodVal = v; },
        get analysisLimit() { return mid().analysisLimit; }, set analysisLimit(v: number) { mid().analysisLimit = v; },
        get candleTimeframeLabel() { const sec = mid().barDurationSec; if (sec % 3600 === 0) return `${sec / 3600}h`; if (sec % 60 === 0) return `${sec / 60}m`; return `${sec}s`; },

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
        get assistantHistory() { return activePair().assistantHistory; },
        set assistantHistory(v: AssistantHistoryRecord[]) { activePair().assistantHistory = v; },
        get chatHistory() { return activePair().chatHistory; },
        set chatHistory(v: ChatMessage[]) { activePair().chatHistory = v; },
        get currentPosition() { return activePair().currentPosition; },
        set currentPosition(v: 'None' | 'Long' | 'Short') {
            const pair = activePair();
            const oldVal = pair.currentPosition;
            if (oldVal !== 'None' && v === 'None') {
                autoLogTrade(pair, oldVal);
                pair.entryPriceVal = '';
                pair.stopLossVal = '';
            }
            pair.currentPosition = v;
        },
        get entryPriceVal() { return activePair().entryPriceVal; },
        set entryPriceVal(v: string) { activePair().entryPriceVal = v; },
        get stopLossVal() { return activePair().stopLossVal; },
        set stopLossVal(v: string) { activePair().stopLossVal = v; },
        get assistantLoading() { return activePair().assistantLoading; },
        set assistantLoading(v: boolean) { activePair().assistantLoading = v; },
        get assistantError() { return activePair().assistantError; },
        set assistantError(v: string | null) { activePair().assistantError = v; },
        get assistantResponse() { return activePair().assistantResponse; },
        set assistantResponse(v: AssistantAnalysis | null) { activePair().assistantResponse = v; },
        get multiAgentResponse() { return activePair().multiAgentResponse; },
        set multiAgentResponse(v: MultiAgentAnalysis | null) { activePair().multiAgentResponse = v; },
        get analysisPhase() { return activePair().analysisPhase; },
        set analysisPhase(v: 'idle' | 'phase1' | 'phase2' | 'complete') { activePair().analysisPhase = v; },
        get individualResults() { return activePair().individualResults; },
        set individualResults(v: IndividualIndicatorResult[]) { activePair().individualResults = v; },
        get agentProgress() { return activePair().agentProgress; },
        set agentProgress(v: AgentProgress[]) { activePair().agentProgress = v; },
        get historyLatestClose() { return activePair().historyLatestClose; },
        set historyLatestClose(v: string) { activePair().historyLatestClose = v; },
        get isAssistantModalOpen() { return activePair().isAssistantModalOpen; },
        set isAssistantModalOpen(v: boolean) { activePair().isAssistantModalOpen = v; },
        get chatInputText() { return activePair().chatInputText; },
        set chatInputText(v: string) { activePair().chatInputText = v; },
        get isChatLoading() { return activePair().isChatLoading; },
        set isChatLoading(v: boolean) { activePair().isChatLoading = v; },
        get currentView() { return activePair().currentView; },
        set currentView(v) { activePair().currentView = v; },
        get userTrades() { return userTrades; },
        set userTrades(v: UserTrade[]) { userTrades = v; },

        get automationEnabled() { return activePair().automationEnabled; },
        set automationEnabled(v: boolean) { activePair().automationEnabled = v; },
        get automationIntervalValue() { return activePair().automationIntervalValue; },
        set automationIntervalValue(v: number) { activePair().automationIntervalValue = v; },
        get automationIntervalUnit() { return activePair().automationIntervalUnit; },
        set automationIntervalUnit(v: 'seconds' | 'minutes' | 'hours') { activePair().automationIntervalUnit = v; },
        get slowIntervalSecs() { return activePair().slowIntervalSecs; },
        set slowIntervalSecs(v: number) { activePair().slowIntervalSecs = v; },
        get normalIntervalSecs() { return activePair().normalIntervalSecs; },
        set normalIntervalSecs(v: number) { activePair().normalIntervalSecs = v; },
        get fastIntervalSecs() { return activePair().fastIntervalSecs; },
        set fastIntervalSecs(v: number) { activePair().fastIntervalSecs = v; },
        get tpLevels() { return activePair().tpLevels; },
        set tpLevels(v: number) { activePair().tpLevels = v; },
        get slLevels() { return activePair().slLevels; },
        set slLevels(v: number) { activePair().slLevels = v; },
        get nextEvaluationIn() { return activePair().nextEvaluationIn; },
        set nextEvaluationIn(v: string) { activePair().nextEvaluationIn = v; },

        get paperCashBalance() { return activePair().paperCashBalance; },
        set paperCashBalance(v: number) { activePair().paperCashBalance = v; },
        get paperInitialUSD() { return activePair().paperInitialUSD; },
        set paperInitialUSD(v: number) { activePair().paperInitialUSD = v; },
        get paperAllocationPct() { return activePair().paperAllocationPct; },
        set paperAllocationPct(v: number) { activePair().paperAllocationPct = v; },
        get paperAutoExecute() { return activePair().paperAutoExecute; },
        set paperAutoExecute(v: boolean) { activePair().paperAutoExecute = v; },
        get activePaperPosition() { return activePair().activePaperPosition; },
        set activePaperPosition(v: Record<string, unknown> | null) { activePair().activePaperPosition = v; },
        get paperUnrealizedPnl() { return activePair().paperUnrealizedPnl; },
        set paperUnrealizedPnl(v: number) { activePair().paperUnrealizedPnl = v; },
        get paperUnrealizedRoi() { return activePair().paperUnrealizedRoi; },
        set paperUnrealizedRoi(v: number) { activePair().paperUnrealizedRoi = v; },
        get paperTotalAccountValue() { return activePair().paperTotalAccountValue; },
        set paperTotalAccountValue(v: number) { activePair().paperTotalAccountValue = v; },
        get paperMarginUsed() { return activePair().paperMarginUsed; },
        set paperMarginUsed(v: number) { activePair().paperMarginUsed = v; },
        get paperMaxTrades() { return activePair().paperMaxTrades; },
        set paperMaxTrades(v: number) { activePair().paperMaxTrades = v; },
        get paperActiveTrades() { return activePair().paperActiveTrades; },
        set paperActiveTrades(v: number) { activePair().paperActiveTrades = v; },
        get paperAvailableTrades() { return activePair().paperAvailableTrades; },
        set paperAvailableTrades(v: number) { activePair().paperAvailableTrades = v; },
        get paperHistory() { return activePair().paperHistory; },
        set paperHistory(v: Record<string, unknown>[]) { activePair().paperHistory = v; },
        get paperLoading() { return activePair().paperLoading; },
        set paperLoading(v: boolean) { activePair().paperLoading = v; },

        get totalPointsScore() { return activePair().totalPointsScore; },
        set totalPointsScore(v: number) { activePair().totalPointsScore = v; },
        get allocatedCapitalPct() { return activePair().allocatedCapitalPct; },
        set allocatedCapitalPct(v: number) { activePair().allocatedCapitalPct = v; },
        get activeOppositeSignalsCount() { return activePair().activeOppositeSignalsCount; },
        set activeOppositeSignalsCount(v: number) { activePair().activeOppositeSignalsCount = v; },
        get markedSupportLevels() { return activePair().markedSupportLevels; },
        set markedSupportLevels(v: number[]) { activePair().markedSupportLevels = v; },
        get markedResistanceLevels() { return activePair().markedResistanceLevels; },
        set markedResistanceLevels(v: number[]) { activePair().markedResistanceLevels = v; },
        get srFlipEvents() { return activePair().srFlipEvents; },
        set srFlipEvents(v: string) { activePair().srFlipEvents = v; },

        get systemHeartbeat() { return systemHeartbeat; },
        get recentDecisions() { return recentDecisions; },
        get completedTrades() { return completedTrades; },

        get costPriceInput() { return activePair().costPriceInput; },
        set costPriceInput(v: number) { activePair().costPriceInput = v; },
        get costPriceOutput() { return activePair().costPriceOutput; },
        set costPriceOutput(v: number) { activePair().costPriceOutput = v; },
        get costIntervalSecs() { return activePair().costIntervalSecs; },
        set costIntervalSecs(v: number) { activePair().costIntervalSecs = v; },
        get costRunsPerDay() { return activePair().costRunsPerDay; },
        set costRunsPerDay(v: number) { activePair().costRunsPerDay = v; },
        get costTokensPerRunInput() { return activePair().costTokensPerRunInput; },
        set costTokensPerRunInput(v: number) { activePair().costTokensPerRunInput = v; },
        get costTokensPerRunOutput() { return activePair().costTokensPerRunOutput; },
        set costTokensPerRunOutput(v: number) { activePair().costTokensPerRunOutput = v; },
        get costDailyProjected() { return activePair().costDailyProjected; },
        set costDailyProjected(v: number) { activePair().costDailyProjected = v; },
        get costWeeklyProjected() { return activePair().costWeeklyProjected; },
        set costWeeklyProjected(v: number) { activePair().costWeeklyProjected = v; },
        get costMonthlyProjected() { return activePair().costMonthlyProjected; },
        set costMonthlyProjected(v: number) { activePair().costMonthlyProjected = v; },
        get costActualInputTokens() { return activePair().costActualInputTokens; },
        set costActualInputTokens(v: number) { activePair().costActualInputTokens = v; },
        get costActualOutputTokens() { return activePair().costActualOutputTokens; },
        set costActualOutputTokens(v: number) { activePair().costActualOutputTokens = v; },
        get costActualTotal() { return activePair().costActualTotal; },
        set costActualTotal(v: number) { activePair().costActualTotal = v; },
        get costLoading() { return activePair().costLoading; },
        set costLoading(v: boolean) { activePair().costLoading = v; },

        async fetchCostEstimate() {
            const pair = activePair();
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
            const pair = activePair();
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
            const pair = activePair();
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
            const pair = activePair();
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
                const pair = activePair();
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
            const pair = activePair();
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
                const pair = activePair();
                const res = await fetch(`/api/decision-profiles/${profileId}/evaluate`, {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        symbol: activeTab,
                        latest_snapshot: pair.midTerm.latestSnapshot,
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
