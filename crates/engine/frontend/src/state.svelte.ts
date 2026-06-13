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

export interface PairState {
    symbol: string;
    exchange: string;
    isConnected: boolean;
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
    assistantHistory: AssistantHistoryRecord[];
    chatHistory: ChatMessage[];
    historyPrices: number[];
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

    // Per-pair workspace view tab
    currentView: 'terminal' | 'performance' | 'settings' | 'positions' | 'decision' | 'risk' | 'exchange' | 'analytics' | 'ledger';

    // Analysis lookback
    analysisLimit: number;

    // Per-pair configuration
    barDurationSec: number;
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

    // Per-pair visibility
    showEmas: boolean;
    showBb: boolean;
    showVwap: boolean;
    showVolume: boolean;
    showAdx: boolean;
    showAtr: boolean;
    showRsi: boolean;
    showMacd: boolean;
    showSqueeze: boolean;

    // Automation scheduling
    automationEnabled: boolean;
    automationIntervalValue: number;
    automationIntervalUnit: 'seconds' | 'minutes' | 'hours';
    nextEvaluationIn: string;

    // Paper trading
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

function createPairState(symbol: string, exchange: string): PairState {
    return {
        symbol,
        exchange,
        isConnected: false,
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
        assistantHistory: [],
        chatHistory: [],
        historyPrices: [],
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

        analysisLimit: globalCandlesConfig.analysis_limit ?? 100,

        barDurationSec: globalCandlesConfig.duration_seconds,
        emaFastVal: globalIndicatorsConfig.ema_fast,
        emaMediumVal: globalIndicatorsConfig.ema_medium,
        emaSlowVal: globalIndicatorsConfig.ema_slow,
        emaLongVal: globalIndicatorsConfig.ema_long,
        rsiPeriodVal: globalIndicatorsConfig.rsi_period,
        macdFastVal: globalIndicatorsConfig.macd_fast,
        macdSlowVal: globalIndicatorsConfig.macd_slow,
        macdSignalVal: globalIndicatorsConfig.macd_signal,
        adxPeriodVal: globalIndicatorsConfig.adx_period,
        atrPeriodVal: globalIndicatorsConfig.atr_period,
        squeezePeriodVal: globalIndicatorsConfig.squeeze_period,

        showEmas: true,
        showBb: true,
        showVwap: true,
        showVolume: true,
        showAdx: true,
        showAtr: true,
        showRsi: true,
        showMacd: true,
        showSqueeze: true,

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

// --- Per-pair state map ---
let pairsMap = $state<Record<string, PairState>>({});
let activeTab = $state<string>('Hyperliquid-BTC');

// --- Global configuration ---
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

// Labels (global for display)
let emaFastLabel = $state('EMA-10');
let emaMediumLabel = $state('EMA-50');
let emaSlowLabel = $state('EMA-100');
let emaLongLabel = $state('EMA-200');
let rsiLabel = $state('RSI (14)');
let adxLabel = $state('ADX (14)');
let atrLabel = $state('ATR (14)');
let macdLabel = $state('MACD (12,26,9)');

// ─── Decision Trading ───────────────────────────────────────────
let activeDecisionProfileId = $state(1);
let decisionProfiles = $state<DecisionProfile[]>([]);
let calculatedDecisionScore = $state<DecisionScore | null>(null);
let decisionLoading = $state(false);

// ─── Risk Management ────────────────────────────────────────────
let activeRiskProfileId = $state(1);
let riskProfiles = $state<RiskProfile[]>([]);
let riskDirection = $state<'LONG' | 'SHORT'>('LONG');
let riskEntryPrice = $state('0');
let riskStopLoss = $state('0');
let riskTakeProfit = $state('0');
let riskCalculation = $state<RiskCalculation | null>(null);
let riskCalculating = $state(false);

// ─── Exchange Accounts ──────────────────────────────────────────
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

// ─── Dashboard ──────────────────────────────────────────────────
let dashboardStats = $state<DashboardStats | null>(null);
let dashboardActiveFilter = $state('summary');
let dashboardPeriod = $state('Todo');
let dashboardOrigin = $state('Todos');
let tradeLedgerRecords = $state<TradeLedgerRecord[]>([]);

// --- Helper to get/set active pair ---
function activePair(): PairState {
    if (!pairsMap[activeTab]) {
        const parts = activeTab.split('-');
        pairsMap[activeTab] = createPairState(parts[1] || 'BTC', parts[0] || 'Hyperliquid');
    }
    return pairsMap[activeTab];
}

export function initPair(symbol: string, exchange: string = 'Hyperliquid') {
    const key = `${exchange}-${symbol}`;
    if (!pairsMap[key]) {
        pairsMap[key] = createPairState(symbol, exchange);
    } else {
        const pair = pairsMap[key];
        pair.barDurationSec = globalCandlesConfig.duration_seconds;
        pair.emaFastVal = globalIndicatorsConfig.ema_fast;
        pair.emaMediumVal = globalIndicatorsConfig.ema_medium;
        pair.emaSlowVal = globalIndicatorsConfig.ema_slow;
        pair.emaLongVal = globalIndicatorsConfig.ema_long;
        pair.rsiPeriodVal = globalIndicatorsConfig.rsi_period;
        pair.macdFastVal = globalIndicatorsConfig.macd_fast;
        pair.macdSlowVal = globalIndicatorsConfig.macd_slow;
        pair.macdSignalVal = globalIndicatorsConfig.macd_signal;
        pair.adxPeriodVal = globalIndicatorsConfig.adx_period;
        pair.atrPeriodVal = globalIndicatorsConfig.atr_period;
        pair.squeezePeriodVal = globalIndicatorsConfig.squeeze_period;
        pair.analysisLimit = globalCandlesConfig.analysis_limit ?? 100;
    }
}

export function removePair(key: string) {
    delete pairsMap[key];
}

export function switchTab(key: string) {
    activeTab = key;
}

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

function autoLogTrade(pair: PairState, oldPosition: 'Long' | 'Short') {
    const entryPrice = parseFloat(pair.entryPriceVal);
    const exitPrice = parseFloat(pair.priceText);

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
    if (oldPosition === 'Long') {
        pnl = exitPrice - entryPrice;
    } else {
        pnl = entryPrice - exitPrice;
    }

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
        } else {
            console.error("❌ Auto-Logger Error: API server rejected the trade record.");
        }
    })
    .catch(err => {
        console.error("❌ Auto-Logger Network Error:", err);
    });
}

export function getState() {
    const app = {
        initPair(symbol: string, exchange: string = 'Hyperliquid') { initPair(symbol, exchange); },
        removePair(key: string) { removePair(key); },
        get pairsMap() { return pairsMap; },
        get activeTab() { return activeTab; },
        set activeTab(v: string) { activeTab = v; },

        // Global
        get apiKeyConfigured() { return apiKeyConfigured; },
        set apiKeyConfigured(v: boolean) { apiKeyConfigured = v; },
        get rulesContent() { return rulesContent; },
        set rulesContent(v: string) { rulesContent = v; },

        // Visibility — proxied per-pair
        get showEmas() { return activePair().showEmas; }, set showEmas(v: boolean) { activePair().showEmas = v; },
        get showBb() { return activePair().showBb; }, set showBb(v: boolean) { activePair().showBb = v; },
        get showVwap() { return activePair().showVwap; }, set showVwap(v: boolean) { activePair().showVwap = v; },
        get showVolume() { return activePair().showVolume; }, set showVolume(v: boolean) { activePair().showVolume = v; },
        get showAdx() { return activePair().showAdx; }, set showAdx(v: boolean) { activePair().showAdx = v; },
        get showAtr() { return activePair().showAtr; }, set showAtr(v: boolean) { activePair().showAtr = v; },
        get showRsi() { return activePair().showRsi; }, set showRsi(v: boolean) { activePair().showRsi = v; },
        get showMacd() { return activePair().showMacd; }, set showMacd(v: boolean) { activePair().showMacd = v; },
        get showSqueeze() { return activePair().showSqueeze; }, set showSqueeze(v: boolean) { activePair().showSqueeze = v; },

        // Indicator config — proxied per-pair
        get barDurationSec() { return activePair().barDurationSec; }, set barDurationSec(v: number) { activePair().barDurationSec = v; },
        get emaFastVal() { return activePair().emaFastVal; }, set emaFastVal(v: number) { activePair().emaFastVal = v; },
        get emaMediumVal() { return activePair().emaMediumVal; }, set emaMediumVal(v: number) { activePair().emaMediumVal = v; },
        get emaSlowVal() { return activePair().emaSlowVal; }, set emaSlowVal(v: number) { activePair().emaSlowVal = v; },
        get emaLongVal() { return activePair().emaLongVal; }, set emaLongVal(v: number) { activePair().emaLongVal = v; },
        get rsiPeriodVal() { return activePair().rsiPeriodVal; }, set rsiPeriodVal(v: number) { activePair().rsiPeriodVal = v; },
        get macdFastVal() { return activePair().macdFastVal; }, set macdFastVal(v: number) { activePair().macdFastVal = v; },
        get macdSlowVal() { return activePair().macdSlowVal; }, set macdSlowVal(v: number) { activePair().macdSlowVal = v; },
        get macdSignalVal() { return activePair().macdSignalVal; }, set macdSignalVal(v: number) { activePair().macdSignalVal = v; },
        get adxPeriodVal() { return activePair().adxPeriodVal; }, set adxPeriodVal(v: number) { activePair().adxPeriodVal = v; },
        get atrPeriodVal() { return activePair().atrPeriodVal; }, set atrPeriodVal(v: number) { activePair().atrPeriodVal = v; },
        get squeezePeriodVal() { return activePair().squeezePeriodVal; }, set squeezePeriodVal(v: number) { activePair().squeezePeriodVal = v; },

        // Dynamic timeframe label
        get candleTimeframeLabel() {
            const sec = activePair().barDurationSec;
            if (sec % 3600 === 0) return `${sec / 3600}h`;
            if (sec % 60 === 0) return `${sec / 60}m`;
            return `${sec}s`;
        },

        // Labels
        get emaFastLabel() { return emaFastLabel; }, set emaFastLabel(v: string) { emaFastLabel = v; },
        get emaMediumLabel() { return emaMediumLabel; }, set emaMediumLabel(v: string) { emaMediumLabel = v; },
        get emaSlowLabel() { return emaSlowLabel; }, set emaSlowLabel(v: string) { emaSlowLabel = v; },
        get emaLongLabel() { return emaLongLabel; }, set emaLongLabel(v: string) { emaLongLabel = v; },
        get rsiLabel() { return rsiLabel; }, set rsiLabel(v: string) { rsiLabel = v; },
        get adxLabel() { return adxLabel; }, set adxLabel(v: string) { adxLabel = v; },
        get atrLabel() { return atrLabel; }, set atrLabel(v: string) { atrLabel = v; },
        get macdLabel() { return macdLabel; }, set macdLabel(v: string) { macdLabel = v; },

        // Proxied per-pair fields
        get activeSymbol() { return activePair().symbol; },
        get activeExchange() { return activePair().exchange; },
        get isConnected() { return activePair().isConnected; },
        set isConnected(v: boolean) { activePair().isConnected = v; },
        get priceText() { return activePair().priceText; },
        set priceText(v: string) { activePair().priceText = v; },
        get vwapText() { return activePair().vwapText; },
        set vwapText(v: string) { activePair().vwapText = v; },
        get avgVolText() { return activePair().avgVolText; },
        set avgVolText(v: string) { activePair().avgVolText = v; },
        get emaFastText() { return activePair().emaFastText; },
        set emaFastText(v: string) { activePair().emaFastText = v; },
        get emaMediumText() { return activePair().emaMediumText; },
        set emaMediumText(v: string) { activePair().emaMediumText = v; },
        get emaSlowText() { return activePair().emaSlowText; },
        set emaSlowText(v: string) { activePair().emaSlowText = v; },
        get emaLongText() { return activePair().emaLongText; },
        set emaLongText(v: string) { activePair().emaLongText = v; },
        get adxText() { return activePair().adxText; },
        set adxText(v: string) { activePair().adxText = v; },
        get adxPlusText() { return activePair().adxPlusText; },
        set adxPlusText(v: string) { activePair().adxPlusText = v; },
        get adxMinusText() { return activePair().adxMinusText; },
        set adxMinusText(v: string) { activePair().adxMinusText = v; },
        get atrText() { return activePair().atrText; },
        set atrText(v: string) { activePair().atrText = v; },
        get rsiText() { return activePair().rsiText; },
        set rsiText(v: string) { activePair().rsiText = v; },
        get macdLineText() { return activePair().macdLineText; },
        set macdLineText(v: string) { activePair().macdLineText = v; },
        get macdSigText() { return activePair().macdSigText; },
        set macdSigText(v: string) { activePair().macdSigText = v; },
        get macdHistText() { return activePair().macdHistText; },
        set macdHistText(v: string) { activePair().macdHistText = v; },
        get sqzValText() { return activePair().sqzValText; },
        set sqzValText(v: string) { activePair().sqzValText = v; },
        get sqzStatusText() { return activePair().sqzStatusText; },
        set sqzStatusText(v: string) { activePair().sqzStatusText = v; },
        get isSqueezeOn() { return activePair().isSqueezeOn; },
        set isSqueezeOn(v: boolean) { activePair().isSqueezeOn = v; },
        get volText() { return activePair().volText; },
        set volText(v: string) { activePair().volText = v; },
        get lastMacdHist() { return activePair().lastMacdHist; },
        set lastMacdHist(v: number) { activePair().lastMacdHist = v; },
        get lastSqzMom() { return activePair().lastSqzMom; },
        set lastSqzMom(v: number) { activePair().lastSqzMom = v; },
        get latestSnapshot() { return activePair().latestSnapshot; },
        set latestSnapshot(v: Record<string, unknown> | null) { activePair().latestSnapshot = v; },
        get assistantHistory() { return activePair().assistantHistory; },
        set assistantHistory(v: AssistantHistoryRecord[]) { activePair().assistantHistory = v; },
        get chatHistory() { return activePair().chatHistory; },
        set chatHistory(v: ChatMessage[]) { activePair().chatHistory = v; },
        get historyPrices() { return activePair().historyPrices; },
        set historyPrices(v: number[]) { activePair().historyPrices = v; },
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
        set currentView(v: 'terminal' | 'performance' | 'settings' | 'positions' | 'decision' | 'risk' | 'exchange' | 'analytics' | 'ledger') { activePair().currentView = v; },
        get analysisLimit() { return activePair().analysisLimit; },
        set analysisLimit(v: number) { activePair().analysisLimit = v; },
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
                const res = await fetch(`/api/paper/status?symbol=${encodeURIComponent(app.activeTab)}`);
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
                    body: JSON.stringify({ symbol: app.activeTab, direction, action: 'OPEN' }),
                });
                if (res.ok) {
                    await (app as any).fetchPaperStatus();
                }
            } catch (_) {} finally {
                pair.paperLoading = false;
            }
        },

        async closePaperPosition() {
            const pair = activePair();
            pair.paperLoading = true;
            try {
                const res = await fetch('/api/paper/order', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ symbol: app.activeTab, direction: '', action: 'CLOSE' }),
                });
                if (res.ok) {
                    await (app as any).fetchPaperStatus();
                }
            } catch (_) {} finally {
                pair.paperLoading = false;
            }
        },

        async resetPaperAccount() {
            const pair = activePair();
            try {
                await fetch('/api/paper/reset', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ symbol: app.activeTab }),
                });
                await (app as any).fetchPaperStatus();
            } catch (_) {}
        },

        async savePaperConfig(initialUSD: number, allocationPct: number, autoExecute: boolean) {
            try {
                await fetch('/api/paper/config', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        symbol: app.activeTab,
                        initial_usd: initialUSD,
                        allocation_pct: allocationPct,
                        auto_execute: autoExecute,
                    }),
                });
                await (app as any).fetchPaperStatus();
            } catch (_) {}
        },

        async fetchPaperHistory(symbol?: string) {
            const pair = activePair();
            try {
                const url = symbol
                    ? `/api/paper/performance?symbol=${encodeURIComponent(symbol)}`
                    : '/api/paper/performance';
                const res = await fetch(url);
                if (res.ok) {
                    const data = await res.json();
                    pair.paperHistory = data.trades || [];
                }
            } catch (_) {}
        },

        async fetchTrades() {
            try {
                const res = await fetch(`/api/trades?_=${Date.now()}`);
                if (res.ok) {
                    const data = await res.json();
                    userTrades = data || [];
                }
            } catch (e) {
                console.error("Failed to fetch user trades:", e);
            }
        },

        get globalCandlesConfig() { return globalCandlesConfig; },
        set globalCandlesConfig(v) { globalCandlesConfig = v; },
        get globalIndicatorsConfig() { return globalIndicatorsConfig; },
        set globalIndicatorsConfig(v) { globalIndicatorsConfig = v; },

        // ─── Decision Trading Accessors ────────────────────────
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
                const res = await fetch('/api/decision-profiles', {
                    method: 'POST', headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ profile_name: name, long_threshold: longT, short_threshold: shortT }),
                });
                if (res.ok) { await (app as any).fetchDecisionProfiles(); return true; }
            } catch (_) {}
            return false;
        },

        async deleteDecisionProfile(id: number) {
            try {
                await fetch(`/api/decision-profiles/${id}`, { method: 'DELETE' });
                await (app as any).fetchDecisionProfiles();
            } catch (_) {}
        },

        async updateDecisionProfile(id: number, name: string, longT: number, shortT: number) {
            try {
                await fetch(`/api/decision-profiles/${id}`, {
                    method: 'POST', headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ profile_name: name, long_threshold: longT, short_threshold: shortT }),
                });
                await (app as any).fetchDecisionProfiles();
            } catch (_) {}
        },

        async addProfileIndicator(profileId: number, name: string, weight: number, overrideStatus: string) {
            try {
                await fetch(`/api/decision-profiles/${profileId}/indicators`, {
                    method: 'POST', headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ indicator_name: name, weight, override_status: overrideStatus }),
                });
                await (app as any).fetchDecisionProfiles();
            } catch (_) {}
        },

        async updateProfileIndicator(profileId: number, indicatorId: number, weight: number, overrideStatus: string) {
            try {
                await fetch(`/api/decision-profiles/${profileId}/indicators/${indicatorId}`, {
                    method: 'POST', headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ weight, override_status: overrideStatus }),
                });
                await (app as any).fetchDecisionProfiles();
            } catch (_) {}
        },

        async deleteProfileIndicator(profileId: number, indicatorId: number) {
            try {
                await fetch(`/api/decision-profiles/${profileId}/indicators/${indicatorId}`, { method: 'DELETE' });
                await (app as any).fetchDecisionProfiles();
            } catch (_) {}
        },

        async evaluateDecision(profileId: number, snap: Record<string, unknown>, historyPrices: number[]) {
            decisionLoading = true;
            try {
                const res = await fetch(`/api/decision-profiles/${profileId}/evaluate`, {
                    method: 'POST', headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        rsi: snap.rsi_14 ? parseFloat(String(snap.rsi_14)) : null,
                        squeeze_on: snap.squeeze_on ?? null,
                        squeeze_momentum: snap.squeeze_momentum ? parseFloat(String(snap.squeeze_momentum)) : null,
                        macd_line: snap.macd_line ? parseFloat(String(snap.macd_line)) : null,
                        macd_signal: snap.macd_signal ? parseFloat(String(snap.macd_signal)) : null,
                        macd_hist: snap.macd_hist ? parseFloat(String(snap.macd_hist)) : null,
                        adx: snap.adx_14 ? parseFloat(String(snap.adx_14)) : null,
                        adx_plus: snap.adx_plus ? parseFloat(String(snap.adx_plus)) : null,
                        adx_minus: snap.adx_minus ? parseFloat(String(snap.adx_minus)) : null,
                        bb_upper: snap.bb_upper ? parseFloat(String(snap.bb_upper)) : null,
                        bb_middle: snap.bb_middle ? parseFloat(String(snap.bb_middle)) : null,
                        bb_lower: snap.bb_lower ? parseFloat(String(snap.bb_lower)) : null,
                        atr: snap.atr_14 ? parseFloat(String(snap.atr_14)) : null,
                        ema_fast: snap.ema_fast ? parseFloat(String(snap.ema_fast)) : null,
                        ema_medium: snap.ema_medium ? parseFloat(String(snap.ema_medium)) : null,
                        ema_slow: snap.ema_slow ? parseFloat(String(snap.ema_slow)) : null,
                        ema_long: snap.ema_long ? parseFloat(String(snap.ema_long)) : null,
                        vwap: snap.vwap ? parseFloat(String(snap.vwap)) : null,
                        close: snap.close ? parseFloat(String(snap.close)) : null,
                        volume: snap.volume ? parseFloat(String(snap.volume)) : null,
                        average_volume: snap.average_volume ? parseFloat(String(snap.average_volume)) : null,
                        current_price: snap.mid_price ? parseFloat(String(snap.mid_price)) : 0,
                        historical_prices: historyPrices,
                    }),
                });
                if (res.ok) { calculatedDecisionScore = await res.json(); }
            } catch (_) {} finally {
                decisionLoading = false;
            }
        },

        // ─── Risk Management Accessors ─────────────────────────
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

        async createRiskProfile(name: string, capital: number, maxRisk: number, leverage: number, commission: number, funding: number, spread: number) {
            try {
                const res = await fetch('/api/risk-profiles', {
                    method: 'POST', headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ profile_name: name, capital, max_risk_pct: maxRisk, leverage, commission_pct: commission, funding_rate_8h: funding, spread }),
                });
                if (res.ok) { await (app as any).fetchRiskProfiles(); return true; }
            } catch (_) {}
            return false;
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
                    method: 'POST', headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        direction: riskDirection,
                        entry_price: parseFloat(riskEntryPrice) || 0,
                        stop_loss_price: parseFloat(riskStopLoss) || 0,
                        take_profit_price: parseFloat(riskTakeProfit) || 0,
                        profile_id: activeRiskProfileId || null,
                    }),
                });
                if (res.ok) { riskCalculation = await res.json(); }
            } catch (_) {} finally {
                riskCalculating = false;
            }
        },

        // ─── Exchange Accounts Accessors ────────────────────────
        get exchangeAccounts() { return exchangeAccounts; },
        set exchangeAccounts(v: ExchangeAccount[]) { exchangeAccounts = v; },
        get exchangeActiveCount() { return exchangeActiveCount; },
        set exchangeActiveCount(v: number) { exchangeActiveCount = v; },
        get exchangeMaxAccounts() { return exchangeMaxAccounts; },
        get exchangeFormDraft() { return exchangeFormDraft; },
        set exchangeFormDraft(v: typeof exchangeFormDraft) { exchangeFormDraft = v; },

        async fetchExchangeAccounts() {
            try {
                const res = await fetch('/api/exchange-keys');
                if (res.ok) {
                    const data = await res.json();
                    exchangeAccounts = data.accounts || [];
                    exchangeActiveCount = data.active_count || 0;
                    exchangeMaxAccounts = data.max_accounts || 3;
                }
            } catch (_) {}
        },

        async addExchangeAccount() {
            try {
                const res = await fetch('/api/exchange-keys', {
                    method: 'POST', headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify(exchangeFormDraft),
                });
                if (res.ok) {
                    await (app as any).fetchExchangeAccounts();
                    exchangeFormDraft = { exchange: 'Bitget', account_name: '', api_key: '', api_secret: '', passphrase: '', referred_uid: '', is_active: true };
                }
            } catch (_) {}
        },

        async deleteExchangeAccount(id: number) {
            try {
                await fetch(`/api/exchange-keys/${id}`, { method: 'DELETE' });
                await (app as any).fetchExchangeAccounts();
            } catch (_) {}
        },

        // ─── Dashboard Accessors ────────────────────────────────
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

        async fetchTradeLedger(limit: number = 200) {
            try {
                const res = await fetch(`/api/trade-ledger?limit=${limit}`);
                if (res.ok) { tradeLedgerRecords = await res.json(); }
            } catch (_) {}
        },
    };

    return app;
}
