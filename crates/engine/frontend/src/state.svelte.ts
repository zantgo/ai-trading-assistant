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

export interface IndividualIndicatorResult {
    indicator_name: string;
    signal: 'BULLISH' | 'BEARISH' | 'SIDEWAYS' | 'UNAVAILABLE';
    reason: string;
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
    avgVolText: string;
    emaFastText: string;
    emaMediumText: string;
    emaSlowText: string;
    emaLongText: string;
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
    lastMacdHist: number;
    lastSqzMom: number;
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
    analysisLimit: number;
}

export interface PairState {
    symbol: string;
    exchange: string;
    isConnected: boolean;
    shortTerm: TimeframeTelemetry;
    midTerm: TimeframeTelemetry;
    longTerm: TimeframeTelemetry;
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
    currentView: 'terminal' | 'performance' | 'settings' | 'positions' | 'decision' | 'risk' | 'exchange' | 'analytics' | 'ledger';
    automationEnabled: boolean;
    automationIntervalValue: number;
    automationIntervalUnit: 'seconds' | 'minutes' | 'hours';
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
}

function createTimeframeTelemetry(symbol: string, exchange: string, barDurationSec: number): TimeframeTelemetry {
    return {
        symbol,
        exchange,
        barDurationSec,
        priceText: '--',
        vwapText: '--',
        avgVolText: '--',
        emaFastText: '--',
        emaMediumText: '--',
        emaSlowText: '--',
        emaLongText: '--',
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
        lastMacdHist: 0,
        lastSqzMom: 0,
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
        analysisLimit: 100,
    };
}

function createPairState(symbol: string, exchange: string): PairState {
    return {
        symbol,
        exchange,
        isConnected: false,
        shortTerm: createTimeframeTelemetry(symbol, exchange, 15),
        midTerm: createTimeframeTelemetry(symbol, exchange, 60),
        longTerm: createTimeframeTelemetry(symbol, exchange, 300),
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
    };
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

let userTrades = $state<UserTrade[]>([]);

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
        pair.shortTerm.barDurationSec = 15;
        pair.shortTerm.emaFastVal = globalIndicatorsConfig.ema_fast;
        pair.shortTerm.emaMediumVal = globalIndicatorsConfig.ema_medium;
        pair.shortTerm.emaSlowVal = globalIndicatorsConfig.ema_slow;
        pair.shortTerm.emaLongVal = globalIndicatorsConfig.ema_long;
        pair.shortTerm.rsiPeriodVal = globalIndicatorsConfig.rsi_period;
        pair.shortTerm.macdFastVal = globalIndicatorsConfig.macd_fast;
        pair.shortTerm.macdSlowVal = globalIndicatorsConfig.macd_slow;
        pair.shortTerm.macdSignalVal = globalIndicatorsConfig.macd_signal;
        pair.shortTerm.adxPeriodVal = globalIndicatorsConfig.adx_period;
        pair.shortTerm.atrPeriodVal = globalIndicatorsConfig.atr_period;
        pair.shortTerm.squeezePeriodVal = globalIndicatorsConfig.squeeze_period;
        pair.shortTerm.analysisLimit = globalCandlesConfig.analysis_limit ?? 100;

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
        set rulesContent(v: string) { rulesContent = v; },

        // Multi-timeframe telemetry access
        get shortTerm() { return activePair().shortTerm; },
        get midTerm() { return activePair().midTerm; },
        get longTerm() { return activePair().longTerm; },

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
                await fetch('/api/paper/config', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ symbol: activeTab, initial_usd: initialUSD, allocation_pct: allocationPct, auto_execute: autoExecute }) });
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
    };
    return app;
}
