// Shared TypeScript interfaces for the Trading Platform dashboard.

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
    /**
     * Decimal fields are serialized as strings from the backend
     * (`#[serde(with = "rust_decimal::serde::str")]`) to preserve full
     * precision. Parse via `parseFloat()` or `new Decimal(value)` before use.
     */
    capital: string;
    max_risk_pct: string;
    leverage: number;
    commission_pct: string;
    funding_rate_8h: string;
    spread: string;
}

export interface RiskCalculation {
    risk_capital: string;
    price_distance: string;
    position_size_units: string;
    position_notional: string;
    leverage_required: string;
    leverage_selected: number;
    margin_required: string;
    liquidation_price: string;
    risk_reward_ratio: string | null;
    estimated_profit: string;
    total_fees: string;
    net_pnl: string;
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

/** Optional per-snapshot statistical context block (L1-native Monte Carlo / z-scores). */
export interface StatisticalContext {
    close_zscore?: number | null;
    rsi_zscore?: number | null;
    macd_zscore?: number | null;
    monte_carlo_expected_return?: number | null;
    monte_carlo_std_dev?: number | null;
    monte_carlo_sample_count?: number | null;
    monte_carlo_p_value?: number | null;
    window_bars?: number | null;
}

/** Candle quality envelope — only present on completed snapshots. */
export type SequenceIntegrity = 'Valid' | 'OutOfOrder' | 'Duplicate';
export interface CandleQualityEnvelope {
    quality_score: number;
    is_valid: boolean;
    is_gap_filled: boolean;
    had_outliers_rejected: boolean;
    spike_detected: boolean;
    is_stale: boolean;
    sequence_integrity: SequenceIntegrity;
    gap_since_last: number;
    validated_at: number;
}

/** Liquidity activation subset (Phase 0/1/2/3 toggles). */
export interface LiquidityActivation {
    enabled: boolean;
    liquidation_feed: boolean;
    cluster_estimation: boolean;
    signals: boolean;
}

/** Optional activation block surfaced from the snapshot. */
export interface MetricsConfig {
    disabled_indicators: string[];
    disabled_signals: Array<[string, string]>;
    disabled_signal_kinds: string[];
    liquidity: LiquidityActivation;
    config_version: number;
}

/**
 * Top-level per-timeframe snapshot envelope (mirrors the Rust `MarketSnapshot`).
 * This is what the WebSocket broadcasts per `(symbol, timeframe_secs)`. Most
 * fields are optional because some only populate on completed candles.
 */
export interface MarketSnapshot {
    exchange?: string | null;
    symbol: string;
    timeframe_secs: number;
    timestamp: number;
    is_completed?: boolean;
    mid_price?: number | string | null;
    bid_price?: number | string | null;
    ask_price?: number | string | null;
    bid_size?: number | string | null;
    ask_size?: number | string | null;
    funding_rate?: number | string | null;
    open?: number | string | null;
    high?: number | string | null;
    low?: number | string | null;
    close?: number | string | null;
    volume?: number | string | null;
    average_volume?: number | string | null;
    indicators: IndicatorMap;
    context?: MarketContext | null;
    alignment?: AlignmentMatrix | null;
    analysis?: AnalysisMatrix | null;
    risk?: RiskMatrix | null;
    advisory?: AdvisoryMatrix | null;
    open_interest?: number | string | null;
    oi_delta_1h?: number | string | null;
    mark_price?: number | string | null;
    index_price?: number | string | null;
    mark_index_spread_pct?: number | null;
    prev_day_px?: number | string | null;
    statistical_context?: StatisticalContext | null;
    decision_context?: Record<string, unknown> | null;
    opportunity?: OpportunityMatrix | null;
    liquidity_signals?: LiquiditySignal[];
    metrics_config?: MetricsConfig | null;
    risk_profile?: number | null;
    liquidity?: LiquidityFlow | null;
    cluster?: LiquidationClusterMatrix | null;
    quality_envelope?: CandleQualityEnvelope | null;
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

// ── Market context + Metrics Panel (meta-intelligence) ──
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

// ── Alignment Matrix (cross-timeframe MTF — 10 dimensions) ──
export interface TfAlignmentInfo {
    timeframe: string;
    timeframe_secs: number;
    trend_score: number;
    momentum_score: number;
    overall_score: number;
    regime: string;
    active_signals: number;
    price: number;
}

export interface AlignmentDimension {
    score: number;
    state: string;
    confidence: number;
}

export interface AlignmentMatrix {
    symbol: string;
    timeframes_present: number;
    dimensions: AlignmentDimension[];
    mtf_trend_alignment: number;
    mtf_momentum_alignment: number;
    mtf_volume_alignment: number;
    mtf_volatility_alignment: number;
    mtf_overall_score: number;
    mtf_overall_label: string;
    timeframe_alignments: TfAlignmentInfo[];
    signal_cross_tf_count: number;
    trend_agreement_pct: number;
}

// ── Analysis Matrix (market interpretation — 10 components) ──
export type MarketBias = 'StrongBullish' | 'Bullish' | 'Neutral' | 'Bearish' | 'StrongBearish';
export type MarketRegime = 'TRENDING_BULL' | 'TRENDING_BEAR' | 'RANGE' | 'ACCUMULATION' | 'DISTRIBUTION' | 'EXPANSION' | 'CONTRACTION' | 'TRANSITION';
export type TrendAssessment = 'Weak' | 'Developing' | 'Healthy' | 'Strong' | 'Exhausted';
export type MomentumAssessment = 'Increasing' | 'Stable' | 'Weakening' | 'Exhausted' | 'Reversing';
export type StructureAssessment = 'Strong' | 'Healthy' | 'Weak' | 'Broken' | 'UNKNOWN';
export type VolatilityAssessment = 'Compressed' | 'Normal' | 'Expanding' | 'Extreme' | 'Unstable';
export type VolumeAssessment = 'Weak' | 'Normal' | 'Strong' | 'Exceptional';
export type OpportunityType = 'TrendContinuation' | 'Breakout' | 'Pullback' | 'MeanReversion' | 'Reversal' | 'LiquiditySqueeze' | 'Scalp' | 'NoClearOpportunity';
export type QualityLevel = 'Poor' | 'Weak' | 'Average' | 'Good' | 'Excellent';

export interface AnalysisMatrix {
    symbol: string;
    bias: MarketBias;
    confidence: number;
    market_regime: MarketRegime;
    trend_assessment: TrendAssessment;
    momentum_assessment: MomentumAssessment;
    structure_assessment: StructureAssessment;
    volatility_assessment: VolatilityAssessment;
    volume_assessment: VolumeAssessment;
    opportunity_analysis: OpportunityType;
    market_quality: QualityLevel;
    market_interpretation: string;
    rationale: string;
    supporting_signals: string[];
    contradicting_signals: string[];
    timeframes_considered: number;
}

// ── Risk Matrix (risk evaluation — 9 dimensions) ──
export type RiskLevel = 'VeryLow' | 'Low' | 'Moderate' | 'High' | 'Extreme';
export type RiskState = 'Stable' | 'Increasing' | 'Elevated' | 'Critical' | 'Improving';

export interface RiskDimension {
    score: number;
    level: RiskLevel;
    state: RiskState;
    confidence: number;
    evidence: string[];
}

export interface RiskMatrix {
    symbol: string;
    market_risk: RiskDimension;
    volatility_risk: RiskDimension;
    /**
     * Phase 3: renamed from `liquidity_risk` for clarity. This dimension
     * covers execution liquidity / market depth, not positional
     * liquidation liquidity.
     */
    execution_liquidity_risk?: RiskDimension;
    structure_risk: RiskDimension;
    momentum_risk: RiskDimension;
    signal_risk: RiskDimension;
    execution_risk: RiskDimension;
    /** Phase 3: cascade risk — danger from forced liquidation cascades. */
    cascade_risk?: RiskDimension;
    overall_risk: RiskDimension;
}

// ── Advisory Matrix (human-facing guidance — 10 components) ──
export type DirectionalGuidance = 'StrongLong' | 'Long' | 'Neutral' | 'Short' | 'StrongShort' | 'AvoidDirectionalExposure';
export type MarketStance = 'Aggressive' | 'Constructive' | 'Neutral' | 'Cautious' | 'Avoid';
export type OpportunityClass = 'TrendContinuation' | 'Breakout' | 'Pullback' | 'MeanReversion' | 'Reversal' | 'NoClearOpportunity';
export type StrategyEnvironment = 'TrendFollowing' | 'Breakout' | 'MeanReversion' | 'HighVolatility' | 'LowActivity' | 'Unfavorable';
export type EntryGuidance = 'Immediate' | 'WaitForConfirmation' | 'Pullback' | 'Breakout' | 'NoEntryContext';
export type ExitGuidance = 'TrendWeakening' | 'MomentumExhaustion' | 'StructureBreakdown' | 'RiskIncreasing' | 'NoWarning';
export type ProtectionStrategy = 'StructureBased' | 'VolatilityBased' | 'ATRBased' | 'SRBased' | 'NoRecommendation';
export type TargetStrategy = 'ResistanceBased' | 'RRBased' | 'VolatilityBased' | 'TrailingMethod' | 'NoRecommendation';

export interface AdvisoryMatrix {
    symbol: string;
    directional_guidance: DirectionalGuidance;
    market_stance: MarketStance;
    opportunity_classification: OpportunityClass;
    strategy_environment: StrategyEnvironment;
    entry_guidance: EntryGuidance;
    exit_guidance: ExitGuidance;
    protection_strategy: ProtectionStrategy;
    target_strategy: TargetStrategy;
    confidence_assessment: number;
    final_recommendation: string;
}

// ── Overview Matrix (global market synthesis — 9 components) ──
export type GlobalBias = 'StrongBullish' | 'Bullish' | 'Neutral' | 'Bearish' | 'StrongBearish' | 'Mixed';
export type MarketBreadth = 'VeryWeak' | 'Weak' | 'Balanced' | 'Positive' | 'StrongPositive' | 'Negative' | 'StrongNegative';
export type SyncLevel = 'HighlySynchronized' | 'Synchronized' | 'Mixed' | 'Fragmented' | 'HighlyFragmented';
export type HealthLevel = 'Poor' | 'Weak' | 'Neutral' | 'Healthy' | 'Strong';

export interface AssetRank {
    symbol: string;
    score: number;
    bias: string;
    confidence: number;
    regime: string;
    risk_level: string;
}

export interface RiskDistribution {
    low_pct: number;
    moderate_pct: number;
    high_pct: number;
    risk_environment: string;
}

export interface OverviewMatrix {
    global_market_bias: GlobalBias;
    market_breadth: MarketBreadth;
    regime_distribution: Record<string, number>;
    opportunity_distribution: Record<string, number>;
    risk_distribution: RiskDistribution;
    asset_ranking: AssetRank[];
    market_synchronization: SyncLevel;
    market_health: HealthLevel;
    global_summary: string;
    instance_count: number;
    active_symbols: string[];
}

// ── Indicator registry manifest (mirror Rust shared::indicators::registry) ──
export type IndicatorGroup =
    | 'Trend' | 'Momentum' | 'Volume' | 'Volatility' | 'Structure' | 'Regime' | 'Institutional' | 'DerivativesData';
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

/// Stable slot identity. The four timeframes are positional; their actual
/// `barDurationSec` may be any positive value the user picked. Slot identity
/// travels on the wire as `timeframe_slot` (`micro`/`fast`/`slow`/`macro`)
/// and is stamped by the analyzer onto every snapshot. Consumers that need
/// to bind a chart to a column should key by slot, never by duration.
export type TimeframeSlotKind = 'micro' | 'fast' | 'slow' | 'macro';

export const TIMEFRAME_SLOT_KINDS: readonly TimeframeSlotKind[] = ['micro', 'fast', 'slow', 'macro'] as const;

export function isTimeframeSlotKind(s: unknown): s is TimeframeSlotKind {
    return s === 'micro' || s === 'fast' || s === 'slow' || s === 'macro';
}

export interface TimeframeTelemetry {
    /// Authoritative slot identity (`micro`/`fast`/`slow`/`macro`). A
    /// `TimeframeTelemetry` lives on a known slot — never derive slot from
    /// `barDurationSec`.
    slot: TimeframeSlotKind;
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
    /** Per-TF synthesis block from the analyzer (L1 MarketContext). */
    context?: MarketContext | null;
    /** Phase 1: per-candle liquidity flow (real liquidation events). */
    liquidity?: LiquidityFlow;
    /** Phase 2: estimated liquidation cluster matrix (5-min refresh). */
    cluster?: LiquidationClusterMatrix;
    /** Phase 3: liquidity signals derived from flow + cluster. */
    liquiditySignals?: LiquiditySignal[];
    /** Volume profile snapshot — per-timeframe aggregated volume distribution. */
    volumeProfile?: VolumeProfileSnapshot;
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
    showIchimoku: boolean;
    showPsar: boolean;
    showStddevChan: boolean;
    showObv: boolean;
    showCmf: boolean;
    showMfi: boolean;
    showHv: boolean;
    showAroon: boolean;
    showChoppiness: boolean;
    showLinregSlope: boolean;
    showZscore: boolean;
    showLiqHeatmap: boolean;
    showVolumeProfile: boolean;
    showWilliamsR: boolean;
    showCci: boolean;
    showForceIdx: boolean;
    showFunding: boolean;
    showOpenInterest: boolean;
    showOiDelta: boolean;
    showOrderFlowDepth: boolean;
    showDerivativeRibbon: boolean;
    showPivotPoints: boolean;
    showSupportResistance: boolean;
    showSmcStructure: boolean;
    showSmcLiquidity: boolean;
    showFvgZones: boolean;
    showOrderBlocks: boolean;
    showAnchoredVwap: boolean;
    showSpread: boolean;
    showChartPatterns: boolean;
    showCandlestickPatterns: boolean;
    showOiPriceDivergence: boolean;
    showAwesome: boolean;
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
export type CurrentView = 'terminal' | 'monitor' | 'alignment' | 'opportunity' | 'risk' | 'analysis' | 'advisory' | 'settings' | 'costs' | 'ledger';

export interface InstanceState {
    symbol: string;
    exchange: string;
    isConnected: boolean;
    /// Backend-assigned instance UUID (`inst_<hex>`). Used as the path
    /// parameter for `/api/instances/{instance_id}/...` endpoints.
    /// Populated lazily from `GET /api/instances` or `POST /api/instances`.
    instanceId?: string;
    microTerm: TimeframeTelemetry;
    fastTerm: TimeframeTelemetry;
    slowTerm: TimeframeTelemetry;
    macroTerm: TimeframeTelemetry;
    historyLatestClose: string;
    currentView: CurrentView;
    alignment: AlignmentMatrix | null;
    analysis: AnalysisMatrix | null;
    risk: RiskMatrix | null;
    advisory: AdvisoryMatrix | null;
    automationEnabled: boolean;
    automationIntervalMode: string;
    automationIntervalValue: number;
    automationIntervalUnit: string;
    priceLineMode: boolean;
    slowIntervalSecs: number;
    normalIntervalSecs: number;
    fastIntervalSecs: number;
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

// ================================================================
// Phase 1-3: Liquidity Intelligence types
// ================================================================

/**
 * Wire-format enum values are SCREAMING_SNAKE_CASE (the Rust side uses
 * `#[serde(rename_all = "SCREAMING_SNAKE_CASE")]` on every liquidity-related
 * enum). Older frontend code compared PascalCase strings — that was a bug;
 * these unions match the actual JSON the server emits.
 */
export type CascadeState = 'NONE' | 'DETECTED' | 'SUSTAINED' | 'EXHAUSTED';
export type LiquidationSide = 'LONG' | 'SHORT';
export type ClusterKind = 'ABOVE_CURRENT_PRICE' | 'BELOW_CURRENT_PRICE' | 'AT_CURRENT_PRICE' | 'DISTANT';
export type LeverageDistributionSource = 'DEFAULT_POWER_LAW' | 'FUNDING_ADAPTIVE' | 'CONFIG_OVERRIDE';

export interface LiquidityFlow {
    long_liquidations_usd: number;
    short_liquidations_usd: number;
    net_liquidation_usd: number;
    event_count: number;
    largest_event_usd: number;
    largest_event_price?: number;
    largest_event_side?: LiquidationSide;
    cascade_state: CascadeState;
    cascade_intensity: number; // 0..100
}

export interface LeverageAssumptions {
    buckets: number[];
    weights: number[];
    funding_modulation_active: boolean;
    funding_extreme_pct: number;
    source: LeverageDistributionSource;
}

export interface LiquidationCluster {
    price_low: number;
    price_high: number;
    peak_price: number;
    notional_usd: number;
    dominant_leverage: number;
    distance_from_mid_pct: number;
    cluster_kind: ClusterKind;
    magnet_strength: number; // 0..100
}

export interface LiquidationClusterMatrix {
    symbol: string;
    generated_at_ms: number;
    valid_until_ms: number;
    mid_price: number;
    leverage_assumptions: LeverageAssumptions;
    short_clusters: LiquidationCluster[];
    long_clusters: LiquidationCluster[];
    cascade_asymmetry: number;       // [-1, +1]
    total_long_oi_usd: number;
    total_short_oi_usd: number;
    estimation_confidence: number;  // 0..1
}

/**
 * Wire-format SCREAMING_SNAKE_CASE values (matches Rust `LiquiditySignalKind`).
 * The frontend previously used PascalCase and silently miscategorized every
 * incoming liquidity signal — see `LiquidityPanel.svelte` for the fix.
 */
export type LiquiditySignalKind =
    | 'CASCADE_DETECTED'
    | 'CASCADE_SUSTAINED'
    | 'CASCADE_EXHAUSTED'
    | 'LIQUIDITY_VACUUM'
    | 'FUNDING_EXTREME'
    | 'OI_FUNDING_DIVERGENCE'
    | 'MAGNET_ACTIVATED'
    | 'CLUSTER_PRESSURE_HIGH'
    | 'CLUSTER_FORWARD_PRESSURE'
    | 'FUNDING_FLIP'
    | 'OI_PRICE_DIVERGENCE';

/** Wire-format LiquidityDirection (was PascalCase; old code broke styling). */
export type LiquidityDirection = 'BULLISH' | 'BEARISH' | 'NEUTRAL';

export interface LiquiditySignal {
    kind: LiquiditySignalKind;
    direction: LiquidityDirection;
    strength: number;     // 0..100
    confidence: number;   // 0..1
    evidence: string[];
}

// ================================================================
// Volume Profile (per-timeframe, computed by market-analyzer)
// ================================================================

export interface VolumeProfileBin {
    price_low: number;
    price_high: number;
    volume: number;
    buy_volume: number;
    sell_volume: number;
    is_poc: boolean;
    is_value_area: boolean;
}

export interface VolumeProfileSnapshot {
    symbol: string;
    timeframe_slot: string;
    timeframe_secs: number;
    bins: VolumeProfileBin[];
    poc_price: number;
    value_area_high: number;
    value_area_low: number;
    total_volume: number;
    range_low: number;
    range_high: number;
    num_bins: number;
    timestamp_ms: number;
}

export type QualityWindow = 'one_hour' | 'six_hour' | 'twenty_four_hour';

export interface ConnectionQualityReport {
    window: QualityWindow;
    window_start_ms: number;
    window_end_ms: number;
    uptime_pct: number;
    disconnect_count: number;
    avg_reconnect_ms: number;
    total_data_loss_secs: number;
    reconstructed_candles: number;
    score: number;
}

export interface ClockStatusResponse {
    within_threshold: boolean;
    drift_us: number | null;
    jitter_rms_us: number | null;
    last_poll_ms: number | null;
    breach_count: number;
    breach_action: string;
    ntp_servers: string[];
    sample_count: number;
    threshold_micros: number;
}

export type ExchangeConnectionState = 'Connecting' | 'Connected' | 'Disconnected' | 'Reconnecting' | 'Disabled';

export interface ExchangeStatus {
    name: string;
    state: ExchangeConnectionState;
    active_pairs: number;
    last_heartbeat_ms: number;
    total_reconnects: number;
    ws_url: string;
}

export interface ExchangeStatusReport {
    exchanges: ExchangeStatus[];
}

export interface PipelineReliabilityMetrics {
    coverage: number;
    gap_count: number;
    outliers_rejected: number;
    out_of_order_dropped: number;
    total_candles_processed: number;
    reconstructed_candles: number;
}

export interface ExchangeAccount {
    id: number;
    exchange: string;
    label: string;
    currency: string;
    testnet: boolean;
    created_at: string;
    is_active: boolean;
    referred_uid: string;
    last_sync_timestamp: number | null;
    api_key: string;
    account_name: string;
}

export interface SystemHeartbeat {
    observation_loop_latency_ms: number;
    ingest_skew_ms: number;
    system_heartbeat_latency_ms: number;
    wal_mode: boolean;
    active_pairs: number;
}

export interface DecisionMemoryRow {
    timestamp: number;
    symbol: string;
    decision_score: number;
    direction: string;
    readiness: string;
}

export interface CompletedTradesRow {
    timestamp: number;
    symbol: string;
    direction: string;
    entry_price: number;
    exit_price: number;
    pnl: number;
    roi_pct: number;
}

export type AllocationCurveModel = 'Stepped' | 'Linear' | 'Exponential';

export interface AllocationCurve {
    model: AllocationCurveModel;
    base_allocation_pct: number;
    max_allocation_pct: number;
    base_score_threshold: number;
    micro_score_threshold: number;
    exponent: number;
}

export interface PositionScalingConfig {
    allocation_curve: AllocationCurve;
    leverage_mode: 'Fixed' | 'VolatilityScaled';
    leverage_cap: number;
    target_margin: number;
}

export type TriggerModeUnion = 'interval' | 'candle_close' | 'event_driven';

export type TriggerModeConfig =
    | { mode: 'interval'; seconds: number }
    | { mode: 'candle_close'; timeframe: string; count: number }
    | { mode: 'event_driven'; events: string[] };

// ── Opportunity Matrix (L4) ──
export type LevelSource = 'FIBONACCI' | 'VOLUME_PROFILE' | 'PIVOT_POINTS' | 'SUPPORT_RESISTANCE' | 'LIQUIDITY_CLUSTER' | 'ATR_FALLBACK';

export interface ConfluentLevel {
    price: number;
    confluence_count: number;
    sources: LevelSource[];
    strength: number;
}

export interface PriceRange {
    low: number;
    high: number;
}

export interface OpportunityMatrix {
    symbol: string;
    primary_opportunity: string;
    opportunity_score: number;
    setup_quality: string;
    profiles: { opportunity_type: string; score: number; preconditions_met: number; preconditions_total: number; notes: string }[];
    forecast_confidence: number;
    contributing_signals: string[];
    invalidation_note: string;
    entry_zone: PriceRange;
    target_zone: PriceRange;
    invalidation_level: number;
    expected_rr_internal: number;
    time_horizon: string;
    confluent_entry_levels: ConfluentLevel[];
    confluent_target_levels: ConfluentLevel[];
    confluent_invalidation_levels: ConfluentLevel[];
}
