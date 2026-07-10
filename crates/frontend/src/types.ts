// Shared TypeScript interfaces for the Market Monitor dashboard.

// Re-export statistics types for backward compatibility
export type { CoreStats, DailyActivity, DailyPnl, HourlyWinRate, WeekdayWinRate, DirectionBreakdown, DashboardStats, TradeLedgerRecord, TradeJournalRecord, StyleSegment, TraderStyleBreakdown, StreakMetrics, CalendarDay, PairStat, DailyCommission, FeePnlRatio, MonthlySummary } from './types/stats';

// ================================================================
// 1. Decision Profiles & Scoring
// ================================================================

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
// 2. Risk & Commission
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

// ================================================================
// 3. Ingestion & Telemetry
// ================================================================

/** Dual-representation normalized indicator DTO (mirrors the Rust NormalizedIndicatorValue). */
export interface IndicatorDto {
    raw_value: number;
    normalized: number;
    state_label: string;
    values?: Record<string, number> | null;
    signals?: IndicatorSignal[];
    confidence?: number;
}

// ── Signals (mirror Rust shared::indicators::normalized signal model) ──
export type SignalKind =
    | 'Divergence' | 'Crossover' | 'Threshold' | 'Breakout' | 'BandTouch'
    | 'ZeroLineCross' | 'CompressionRelease' | 'LevelTest' | 'TrendFlip'
    | 'VolumeClimax' | 'StackChange' | 'PatternForming';
export type SignalDirection = 'Bullish' | 'Bearish' | 'Neutral';
export type SignalStatus = 'Potential' | 'Confirmed' | 'Active';
export interface SignalPoint { time: number; value: number; }
export interface IndicatorSignal {
    kind: SignalKind;
    direction: SignalDirection;
    status: SignalStatus;
    label: string;
    strength: number;
    age_bars?: number;
    points?: SignalPoint[] | null;
}

// ── Market context + Terminal Monitor (meta-intelligence) ──
export interface ContextDimension {
    score: number;
    confidence: number;
    label: string;
}
export interface MarketContext {
    trend: ContextDimension;
    momentum: ContextDimension;
    volatility: ContextDimension;
    volume: ContextDimension;
    liquidity: ContextDimension;
    regime: string;
    overall_score: number;
    overall_label: string;
}
export interface MonitorTimeframe {
    label: string;
    timeframe_secs: number;
    regime: string;
    overall_score: number;
    overall_label: string;
    confluence_score: number;
}
export interface MtfIndicatorRow {
    key: string;
    display_name: string;
    per_tf: number[];
    agreement: number;
}
export interface MtfConfirmation {
    trend_agreement_pct: number;
    structural_trend: string;
    rows: MtfIndicatorRow[];
}
export interface MonitorResponse {
    symbol: string;
    timeframes: MonitorTimeframe[];
    mtf: MtfConfirmation;
    market_context: MarketContext | null;
}

// ── Indicator registry manifest (mirror Rust shared::indicators::registry) ──
export type IndicatorGroup =
    | 'Trend' | 'Momentum' | 'Volume' | 'Volatility' | 'Structure' | 'Regime' | 'Advanced';
export type IndicatorClass = 'Leading' | 'Hybrid' | 'Lagging';
export type RenderKind = 'Pane' | 'PriceOverlay' | 'PriceLevels' | 'Marker';
export interface IndicatorMeta {
    key: string;
    display_name: string;
    group: IndicatorGroup;
    class: IndicatorClass;
    render: RenderKind;
    directional: boolean;
    supports_divergence: boolean;
    signal_types: SignalKind[];
    default_weight: number;
    default_enabled: boolean;
    config_params: string[];
    value_format: string;
    value_source: string;
    color: string;
    guide_section: string;
}

export type IndicatorMap = Record<string, IndicatorDto>;

export function emptyIndicator(): IndicatorDto {
    return { raw_value: 0, normalized: 0, state_label: 'UNKNOWN', values: null };
}

export interface TimeframeTelemetry {
    symbol: string;
    exchange: string;
    barDurationSec: number;
    indicators: IndicatorMap;
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
    showStochastic: boolean;
    showChandeMo: boolean;
    showSupertrend: boolean;
    showKeltner: boolean;
    showDonchian: boolean;
    showObv: boolean;
    showCmf: boolean;
    showMfi: boolean;
    showHv: boolean;
    showAroon: boolean;
    showChoppiness: boolean;
    showLinregSlope: boolean;
    showZscore: boolean;
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
    stochKPeriodVal: number;
    stochDPeriodVal: number;
    stochSPeriodVal: number;
    chandemoPeriodVal: number;
    supertrendPeriodVal: number;
    supertrendMultiplierVal: number;
    keltnerEmaPeriodVal: number;
    keltnerAtrPeriodVal: number;
    keltnerMultiplierVal: number;
    donchianPeriodVal: number;
    obvSmoothingVal: number;
    cmfPeriodVal: number;
    mfiPeriodVal: number;
    hvPeriodVal: number;
    aroonPeriodVal: number;
    chopPeriodVal: number;
    linregPeriodVal: number;
    zscorePeriodVal: number;
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

/** All feature-panel view keys mountable inside an instance workspace. */
export type CurrentView = 'terminal' | 'monitor' | 'decision' | 'commission' | 'performance' | 'analytics' | 'ledger' | 'observability' | 'settings' | 'timeframe_settings';

export interface InstanceState {
    symbol: string;
    exchange: string;
    isConnected: boolean;
    microTerm: TimeframeTelemetry;
    fastTerm: TimeframeTelemetry;
    slowTerm: TimeframeTelemetry;
    macroTerm: TimeframeTelemetry;
    historyLatestClose: string;
    currentView: CurrentView;
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

// ================================================================
// 4. Supported Timeframe Spectrum (14-tier)
// ================================================================

export interface TimeframeOption {
    label: string;
    seconds: number;
}

export const TIMEFRAME_OPTIONS: TimeframeOption[] = [
    { label: '1 sec', seconds: 1 },
    { label: '3 sec', seconds: 3 },
    { label: '5 sec', seconds: 5 },
    { label: '15 sec', seconds: 15 },
    { label: '30 sec', seconds: 30 },
    { label: '1 min', seconds: 60 },
    { label: '3 min', seconds: 180 },
    { label: '5 min', seconds: 300 },
    { label: '15 min', seconds: 900 },
    { label: '30 min', seconds: 1800 },
    { label: '1 h', seconds: 3600 },
    { label: '4 h', seconds: 14400 },
    { label: '12 h', seconds: 43200 },
    { label: '1 day', seconds: 86400 },
];
