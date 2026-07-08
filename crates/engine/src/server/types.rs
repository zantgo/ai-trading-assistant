use crate::db::{CompletedTradesBufferRow, DecisionMemoryBufferRow};
use crate::llm::{AnalystDocument, ChatMessage, TraderDecision};
use serde::{Deserialize, Serialize};
use shared::indicators::normalized::NormalizedIndicatorValue;
use std::collections::{BTreeSet, HashMap};

#[derive(Debug, Deserialize)]
pub struct AnalyzeRequest {
    pub position: String,
    #[serde(default)]
    pub entry_price: String,
    pub historical_prices: Vec<f64>,
    pub indicators: IndicatorSnapshot,
    #[serde(default)]
    pub symbol: String,
    #[serde(default)]
    pub timeframes: Option<MultiTimeframeIndicators>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MultiTimeframeIndicators {
    pub micro_term: IndicatorSnapshot,
    pub fast_term: IndicatorSnapshot,
    #[serde(default)]
    pub slow_term: Option<IndicatorSnapshot>,
    #[serde(default)]
    pub macro_term: Option<IndicatorSnapshot>,
}

#[derive(Debug, Deserialize)]
pub struct SetKeyRequest {
    pub api_key: String,
}

#[derive(Debug, Deserialize)]
pub struct SetRulesRequest {
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct RulesResponse {
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct ConfigResponse {
    pub api_key_configured: bool,
    pub symbols: Vec<String>,
    pub candles: crate::config::CandlesConfig,
    pub indicators: crate::config::IndicatorsConfig,
    pub instances: std::collections::HashMap<String, crate::config::InstanceSpecificConfig>,
    /// Authoritative indicator manifest (single source of truth) consumed by the
    /// frontend to drive the telemetry matrix, toggles, and scoring UI.
    pub indicator_registry: Vec<shared::indicators::IndicatorMeta>,
}

#[derive(Debug, Deserialize)]
pub struct AssistantRecordsQuery {
    #[serde(default)]
    pub trigger_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    #[serde(default)]
    pub symbol: String,
    #[serde(default)]
    pub timeframe_secs: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct WsQuery {
    #[serde(default)]
    pub symbol: String,
    #[serde(default)]
    pub timeframe_secs: Option<u64>,
}

/// Nested dual-representation indicator DTO (v2.0). Carries the normalized
/// indicator map plus non-indicator market context. Legacy flat accessor
/// methods reconstruct scalar values from the map for existing consumers.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct IndicatorSnapshot {
    #[serde(default)]
    pub indicators: HashMap<String, NormalizedIndicatorValue>,
    #[serde(default)]
    pub current_price: Option<f64>,
    #[serde(default)]
    pub volume: Option<f64>,
    #[serde(default)]
    pub average_volume: Option<f64>,
}

impl IndicatorSnapshot {
    pub fn new(indicators: HashMap<String, NormalizedIndicatorValue>, current_price: Option<f64>) -> Self {
        Self { indicators, current_price, volume: None, average_volume: None }
    }

    fn raw(&self, k: &str) -> Option<f64> {
        self.indicators.get(k).map(|v| v.raw_value)
    }
    fn sub(&self, k: &str, s: &str) -> Option<f64> {
        self.indicators
            .get(k)
            .and_then(|v| v.values.as_ref())
            .and_then(|m| m.get(s))
            .copied()
    }
    fn lbl(&self, k: &str) -> Option<&str> {
        self.indicators.get(k).map(|v| v.state_label.as_str())
    }
    fn norm(&self, k: &str) -> Option<f64> {
        self.indicators.get(k).map(|v| v.normalized)
    }

    // ── Scalar accessors (flat-equivalent) ──
    pub fn rsi(&self) -> Option<f64> { self.raw("rsi") }
    pub fn stoch_k(&self) -> Option<f64> { self.sub("stochastic", "k_line") }
    pub fn stoch_d(&self) -> Option<f64> { self.sub("stochastic", "d_line") }
    pub fn chandemo(&self) -> Option<f64> { self.raw("chandemo") }
    pub fn supertrend(&self) -> Option<f64> { self.sub("supertrend", "line") }
    pub fn keltner_middle(&self) -> Option<f64> { self.sub("keltner", "middle") }
    pub fn donchian_upper(&self) -> Option<f64> { self.sub("donchian", "upper") }
    pub fn obv(&self) -> Option<f64> { self.raw("obv") }
    pub fn cmf(&self) -> Option<f64> { self.raw("cmf") }
    pub fn mfi(&self) -> Option<f64> { self.raw("mfi") }
    pub fn hv(&self) -> Option<f64> { self.raw("hv") }
    pub fn macd_line(&self) -> Option<f64> { self.sub("macd", "line") }
    pub fn macd_signal(&self) -> Option<f64> { self.sub("macd", "signal") }
    pub fn macd_histogram(&self) -> Option<f64> { self.sub("macd", "histogram") }
    pub fn macd_histogram_peak(&self) -> Option<f64> { self.sub("macd", "histogram_peak") }
    pub fn adx(&self) -> Option<f64> { self.sub("adx", "adx") }
    pub fn adx_plus(&self) -> Option<f64> { self.sub("adx", "plus_di") }
    pub fn adx_minus(&self) -> Option<f64> { self.sub("adx", "minus_di") }
    pub fn adx_slope(&self) -> Option<f64> { self.sub("adx", "adx_slope") }
    pub fn atr(&self) -> Option<f64> { self.sub("atr", "atr_14") }
    pub fn bb_upper(&self) -> Option<f64> { self.sub("bollinger", "upper") }
    pub fn bb_middle(&self) -> Option<f64> { self.sub("bollinger", "middle") }
    pub fn bb_lower(&self) -> Option<f64> { self.sub("bollinger", "lower") }
    pub fn bbwp(&self) -> Option<f64> { self.raw("bbwp") }
    pub fn rvol(&self) -> Option<f64> { self.raw("rvol") }
    pub fn vwap(&self) -> Option<f64> { self.sub("vwap", "vwap") }
    pub fn squeeze_momentum(&self) -> Option<f64> { self.raw("squeeze") }
    pub fn ema_fast(&self) -> Option<f64> { self.sub("ema_stack", "fast") }
    pub fn ema_medium(&self) -> Option<f64> { self.sub("ema_stack", "medium") }
    pub fn ema_slow(&self) -> Option<f64> { self.sub("ema_stack", "slow") }
    pub fn ema_long(&self) -> Option<f64> { self.sub("ema_stack", "long") }
    pub fn chart_pattern_confidence(&self) -> Option<f64> { self.raw("patterns") }

    // ── Boolean accessors ──
    pub fn squeeze_on(&self) -> Option<bool> {
        self.lbl("squeeze").map(|l| l == "COMPRESSION_COILING")
    }
    pub fn squeeze_release_trigger(&self) -> Option<bool> {
        self.lbl("squeeze").map(|l| l.ends_with("VOLATILITY_RELEASE"))
    }
    pub fn macd_crossover_detected(&self) -> Option<bool> {
        self.lbl("macd").map(|l| l.contains("CROSSOVER"))
    }

    // ── State-string accessors (legacy vocabulary) ──
    pub fn ema_stack_state(&self) -> Option<String> {
        self.lbl("ema_stack").map(|l| {
            if l.contains("BULLISH") { "bullish".into() }
            else if l.contains("BEARISH") { "bearish".into() }
            else { "tangled".into() }
        })
    }
    pub fn vwap_bias(&self) -> Option<String> {
        self.lbl("vwap").map(|l| {
            if l.contains("PREMIUM") { "premium".into() }
            else if l.contains("DISCOUNT") { "discount".into() }
            else { "equilibrium".into() }
        })
    }
    pub fn adx_regime(&self) -> Option<String> {
        self.lbl("adx").map(|l| {
            if l.contains("CONGESTION") { "congestion".into() }
            else if l.contains("EMERGING") { "emerging".into() }
            else if l.contains("CLIMACTIC") { "extreme".into() }
            else if l.contains("STRONG") { "strong".into() }
            else { "congestion".into() }
        })
    }
    pub fn macd_crossover_direction(&self) -> Option<String> {
        let v = self.indicators.get("macd")?;
        if !v.state_label.contains("CROSSOVER") { return None; }
        Some(if v.normalized >= 0.0 { "BULLISH" } else { "BEARISH" }.to_string())
    }
    pub fn macd_trend_state(&self) -> Option<String> {
        let hist = self.sub("macd", "histogram")?.abs();
        let peak = self.sub("macd", "histogram_peak")?.abs();
        Some(if peak > 0.0 && hist < peak { "decelerating".into() } else { "accelerating".into() })
    }
    pub fn squeeze_momentum_direction(&self) -> Option<String> {
        self.indicators.get("squeeze").map(|v| {
            let l = v.state_label.as_str();
            if l.contains("BULLISH") && v.normalized >= 0.5 { "BullishAcceleration".into() }
            else if l.contains("BULLISH") { "BullishDeceleration".into() }
            else if l.contains("BEARISH") && v.normalized <= -0.5 { "BearishAcceleration".into() }
            else if l.contains("BEARISH") { "BearishDeceleration".into() }
            else { "Flat".into() }
        })
    }
    pub fn chart_pattern(&self) -> Option<String> {
        self.indicators.get("patterns").and_then(|v| {
            if v.normalized > 0.0 { Some("BullishPattern".to_string()) }
            else if v.normalized < 0.0 { Some("BearishPattern".to_string()) }
            else { None }
        })
    }
    pub fn rsi_divergence_status(&self) -> Option<String> {
        divergence_status(self.lbl("rsi_divergence"))
    }
    pub fn macd_divergence_status(&self) -> Option<String> {
        divergence_status(self.lbl("macd_divergence"))
    }
    pub fn norm_of(&self, key: &str) -> f64 { self.norm(key).unwrap_or(0.0) }

    // ── Fields not preserved in the normalized map (return None) ──
    pub fn squeeze_duration(&self) -> Option<u32> { None }
    pub fn atr_trend(&self) -> Option<String> { None }
    pub fn atr_volatility_regime(&self) -> Option<String> { None }
    pub fn macd_histogram_trend(&self) -> Option<String> { None }
    pub fn adx_di_crossover_detected(&self) -> Option<bool> { None }
    pub fn adx_di_crossover_direction(&self) -> Option<String> { None }
}

fn divergence_status(label: Option<&str>) -> Option<String> {
    match label {
        Some("CONFIRMED_BULLISH_DIVERGENCE") => Some("confirmed_bullish".into()),
        Some("POTENTIAL_BULLISH_DIVERGENCE") => Some("potential_bullish".into()),
        Some("CONFIRMED_BEARISH_DIVERGENCE") => Some("confirmed_bearish".into()),
        Some("POTENTIAL_BEARISH_DIVERGENCE") => Some("potential_bearish".into()),
        _ => None,
    }
}

#[derive(Debug, Serialize)]
pub struct WizardAnalysisResponse {
    pub analyst_document: AnalystDocument,
    pub trader_decision: TraderDecision,
}

#[derive(Debug, Serialize)]
pub struct HistoryCandle {
    pub time: u64,
    pub open: String,
    pub high: String,
    pub low: String,
    pub close: String,
    pub volume: String,
}

/// Parallel time-series arrays for a single indicator (aligned to `times`).
/// `values` carries multi-line sub-series (e.g. macd line/signal, bollinger
/// bands, ema ribbon) each aligned to `times`.
#[derive(Debug, Default, Serialize)]
pub struct HistoricalIndicatorArrays {
    pub raw: Vec<Option<f64>>,
    pub normalized: Vec<Option<f64>>,
    pub state_label: Vec<Option<String>>,
    pub values: HashMap<String, Vec<Option<f64>>>,
}

impl HistoricalIndicatorArrays {
    /// Initialize with pre-known multi-line sub-keys so `values` sub-series
    /// stay aligned even when some snapshots omit them.
    pub fn with_value_keys(value_keys: &BTreeSet<String>) -> Self {
        let mut values = HashMap::new();
        for k in value_keys {
            values.insert(k.clone(), Vec::new());
        }
        Self {
            raw: Vec::new(),
            normalized: Vec::new(),
            state_label: Vec::new(),
            values,
        }
    }

    /// Append a present indicator value (raw/normalized/label + each sub-value).
    pub fn push_value(&mut self, v: &NormalizedIndicatorValue) {
        self.raw.push(Some(v.raw_value));
        self.normalized.push(Some(v.normalized));
        self.state_label.push(Some(v.state_label.clone()));
        for (k, series) in self.values.iter_mut() {
            let sv = v.values.as_ref().and_then(|m| m.get(k)).copied();
            series.push(sv);
        }
    }

    /// Append a null (missing) slot, preserving parallel alignment.
    pub fn push_none(&mut self) {
        self.raw.push(None);
        self.normalized.push(None);
        self.state_label.push(None);
        for series in self.values.values_mut() {
            series.push(None);
        }
    }
}

#[derive(Debug, Serialize)]
pub struct IndicatorHistoryArrays {
    pub symbol: String,
    pub timeframe_secs: u64,
    pub times: Vec<u64>,
    pub indicators: HashMap<String, HistoricalIndicatorArrays>,
}

#[derive(Debug, Serialize)]
pub struct HistoryResponse {
    pub prices: Vec<String>,
    pub candles: Vec<HistoryCandle>,
    pub indicator_history: IndicatorHistoryArrays,
}

// ─── Terminal Monitor (cross-timeframe meta-intelligence) ──────

/// Signed per-indicator contribution to a timeframe's confluence score.
#[derive(Debug, Serialize)]
pub struct ContributionDto {
    pub key: String,
    pub display_name: String,
    /// Signed contribution `weight × normalized` (bull-bias frame).
    pub contribution: f64,
}

#[derive(Debug, Serialize)]
pub struct MonitorTimeframe {
    pub label: String,
    pub timeframe_secs: u64,
    pub regime: String,
    pub overall_score: i32,
    pub overall_label: String,
    /// Registry confluence score in [-100,100] for this timeframe (bull bias).
    pub confluence_score: i32,
    /// Bias-projected confluence in [-1,1] (pre-scaling of confluence_score).
    pub confluence_normalized: f64,
    /// Total active weight of enabled/present directional indicators.
    pub active_weight: f64,
    /// Non-directional regime gate applied this run (choppiness × adx).
    pub regime_gate: f64,
    /// Per-indicator signed contributions driving the confluence score.
    pub contributions: Vec<ContributionDto>,
    /// Opposite-signal exit score if holding LONG (sum of opposing |contrib| × 100).
    pub opposite_score_long: u32,
    /// Opposite-signal exit score if holding SHORT.
    pub opposite_score_short: u32,
    /// Registry opposite-signal exit threshold (conviction bar, 0-100 scale).
    pub opposite_exit_threshold: f64,
}

/// Per-indicator agreement across the four timeframes.
#[derive(Debug, Serialize)]
pub struct MtfIndicatorRow {
    pub key: String,
    pub display_name: String,
    /// Signed direction (+1/0/-1) per timeframe: [micro, fast, slow, macro].
    pub per_tf: Vec<i8>,
    /// Fraction of timeframes agreeing with the dominant direction (0-1).
    pub agreement: f64,
}

#[derive(Debug, Serialize)]
pub struct MtfConfirmation {
    /// Overall trend-agreement across indicators & timeframes (0-100%).
    pub trend_agreement_pct: f64,
    pub structural_trend: String,
    pub rows: Vec<MtfIndicatorRow>,
}

#[derive(Debug, Serialize)]
pub struct MonitorResponse {
    pub symbol: String,
    pub timeframes: Vec<MonitorTimeframe>,
    pub mtf: MtfConfirmation,
    /// Macro-timeframe market-context synthesis (falls back to micro).
    pub market_context: Option<shared::market_context::MarketContext>,
}

// ── Active Trades Monitoring (IMOL) ────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ActiveTradesResponse {
    pub symbol: String,
    pub has_active_position: bool,
    pub direction: Option<String>,
    pub average_entry_price: Option<f64>,
    pub total_size: f64,
    pub unrealized_pnl: f64,
    pub unrealized_roi_pct: f64,
    pub margin_used: f64,
    pub account_value: f64,
    pub slots: Vec<ActiveTradeDto>,
    pub break_even_trail: BreakEvenTrailDto,
    pub exit_signals: ExitSignalsDto,
    pub safety_state: SafetyStateDto,
}

#[derive(Debug, Serialize)]
pub struct ActiveTradeDto {
    pub slot_id: i64,
    pub direction: String,
    pub entry_price: f64,
    pub size: f64,
    pub allocated_usd: f64,
    pub unrealized_pnl: f64,
    pub unrealized_pnl_pct: f64,
    pub stop_loss_price: Option<f64>,
    pub take_profit_prices: Vec<f64>,
}

#[derive(Debug, Serialize)]
pub struct BreakEvenTrailDto {
    pub enabled: bool,
    pub trail_price: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct ExitSignalsDto {
    pub opposite_score_long: u32,
    pub opposite_score_short: u32,
    pub opposite_exit_threshold: f64,
    pub invalidation_level: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct SafetyStateDto {
    pub consecutive_losses: u32,
    pub caution_threshold: u32,
    pub suspend_threshold: u32,
}

#[derive(Debug, Serialize)]
pub struct ChatReplResponse {
    pub reply: String,
}

#[derive(Debug, Deserialize)]
pub struct ChatHistoryRequest {
    pub history: Vec<ChatMessage>,
}

#[derive(Debug, Deserialize)]
pub struct AddTradeRequest {
    pub symbol: String,
    pub direction: String,
    pub outcome: String,
    pub risk_multiplier: f64,
    pub reward_multiplier: f64,
}

#[derive(Debug, Serialize)]
pub struct MasterRecordJson {
    pub id: i64,
    pub created_at: String,
    pub position: String,
    pub entry_price: Option<String>,
    pub trend_classification: String,
    pub indicator_alignment: String,
    pub indicator_synthesis_summary: String,
    pub recommended_action: String,
    pub recommendation_rationale: String,
    pub price_at_analysis: String,
    pub support_levels: String,
    pub resistance_levels: String,
    pub symbol: String,
    pub trigger_type: String,
}

#[derive(Debug, Serialize)]
pub struct MasterHistoryResponse {
    pub records: Vec<MasterRecordJson>,
    pub latest_close: String,
}

#[derive(Debug, Serialize)]
pub struct AnalyzeAcceptedResponse {
    pub master_id: i64,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemStatusResponse {
    pub connected: bool,
    pub latency_ms: u64,
    pub journal_mode: String,
    pub total_allocated_margin: f64,
    pub total_ai_token_costs_usd: f64,
    pub active_pairs_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ObservabilityBuffersResponse {
    pub symbol: String,
    pub recent_decisions: Vec<DecisionMemoryBufferRow>,
    pub completed_trades: Vec<CompletedTradesBufferRow>,
}

#[derive(Debug, Serialize)]
pub struct CostEstimateResponse {
    pub price_per_1m_input_tokens: f64,
    pub price_per_1m_output_tokens: f64,
    pub interval_seconds: u64,
    pub runs_per_day: f64,
    pub input_tokens_per_run: u64,
    pub output_tokens_per_run: u64,
    pub projected_daily_cost: f64,
    pub projected_weekly_cost: f64,
    pub projected_monthly_cost: f64,
    pub actual_input_tokens_used: u64,
    pub actual_output_tokens_used: u64,
    pub actual_total_cost: f64,
}

#[derive(Debug, Deserialize)]
pub struct CostEstimateQuery {
    pub pair_key: Option<String>,
}

// ─── Paper Trading ───────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct PaperStatusQuery { #[serde(default)] pub symbol: String }

#[derive(Debug, Deserialize)]
pub struct PaperConfigRequest {
    pub symbol: String, pub initial_usd: f64, pub allocation_pct: f64,
    pub auto_execute: bool, #[serde(default = "default_max_risk_pct")] pub max_risk_pct: f64,
    #[serde(default = "default_leverage")] pub leverage: i32,
    #[serde(default = "default_auto_execute_intervals")] pub auto_execute_intervals: i32,
    #[serde(default = "default_lookback_trades")] pub lookback_trades: i32,
    #[serde(default)] pub break_even_trail_enabled: bool,
}
fn default_max_risk_pct() -> f64 { 2.0 }
fn default_leverage() -> i32 { 20 }
fn default_auto_execute_intervals() -> i32 { 15 }
fn default_lookback_trades() -> i32 { 10 }

#[derive(Debug, Deserialize)]
pub struct PaperResetRequest { pub symbol: String }

#[derive(Debug, Deserialize)]
pub struct PaperOrderRequest { pub symbol: String, pub direction: String, pub action: String }

#[derive(Debug, Deserialize)]
pub struct PaperPositionPctRequest { pub symbol: String, pub direction: String, #[serde(default)] pub pct: f64 }

#[derive(Debug, Deserialize)]
pub struct PaperTpSlRequest { pub symbol: String, pub targets: Vec<TpSlTarget> }

#[derive(Debug, Deserialize)]
pub struct TpSlTarget { pub pct: f64, pub price: f64 }

#[derive(Debug, Deserialize)]
pub struct PaperPerformanceQuery { #[serde(default)] pub symbol: Option<String> }

#[derive(Debug, Deserialize)]
pub struct PlaceOrderRequest {
    pub symbol: String,
    pub order_type: String,
    pub direction: String,
    #[serde(default)]
    pub price: Option<f64>,
    #[serde(default)]
    pub trigger_price: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct CancelOrderRequest {
    pub symbol: String,
    pub order_id: i64,
}

#[derive(Debug, Serialize)]
pub struct PlaceOrderResponse {
    pub success: bool,
    pub message: String,
    pub order_id: Option<i64>,
}

// ─── Portion Slot Operations ──────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct PaperPortionOpenRequest {
    pub symbol: String,
    pub direction: String,
}

#[derive(Debug, Serialize)]
pub struct PaperPortionOpenResponse {
    pub success: bool,
    pub message: String,
    pub slot_index: i32,
    pub size: f64,
    pub allocated_usd: f64,
}

#[derive(Debug, Deserialize)]
pub struct PaperPortionCloseRequest {
    pub symbol: String,
}

#[derive(Debug, Serialize)]
pub struct PaperPortionCloseResponse {
    pub success: bool,
    pub message: String,
    pub slot_index: i32,
    pub realized_pnl: f64,
    pub refunded_usd: f64,
}

#[derive(Debug, Deserialize)]
pub struct PaperEquityHistoryQuery {
    #[serde(default)]
    pub symbol: String,
    #[serde(default = "default_equity_limit")]
    pub limit: i64,
}
fn default_equity_limit() -> i64 { 200 }

// ─── Decision Profiles ────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct DecisionProfileCreate { pub profile_name: String, #[serde(default = "default_long_threshold")] pub long_threshold: i32, #[serde(default = "default_short_threshold")] pub short_threshold: i32 }
fn default_long_threshold() -> i32 { 15 }
fn default_short_threshold() -> i32 { -15 }

#[derive(Debug, Deserialize)]
pub struct DecisionProfileUpdate { pub profile_name: String, pub long_threshold: i32, pub short_threshold: i32 }

#[derive(Debug, Deserialize)]
pub struct ProfileIndicatorAdd { pub indicator_name: String, #[serde(default = "default_weight")] pub weight: i32, pub override_status: String }
fn default_weight() -> i32 { 10 }

#[derive(Debug, Deserialize)]
pub struct ProfileIndicatorUpdate { pub weight: i32, pub override_status: String }

#[derive(Debug, Deserialize)]
pub struct EvaluateRequest {
    pub symbol: String, pub latest_snapshot: Option<serde_json::Value>,
    #[serde(default)] pub historical_prices: Option<Vec<f64>>,
    #[serde(default)] pub rsi: Option<f64>, #[serde(default)] pub squeeze_on: Option<bool>,
    #[serde(default)] pub squeeze_momentum: Option<f64>, #[serde(default)] pub macd_line: Option<f64>,
    #[serde(default)] pub macd_signal: Option<f64>, #[serde(default)] pub macd_hist: Option<f64>,
    #[serde(default)] pub adx: Option<f64>, #[serde(default)] pub adx_plus: Option<f64>,
    #[serde(default)] pub adx_minus: Option<f64>, #[serde(default)] pub bb_upper: Option<f64>,
    #[serde(default)] pub bb_middle: Option<f64>, #[serde(default)] pub bb_lower: Option<f64>,
    #[serde(default)] pub atr: Option<f64>, #[serde(default)] pub ema_fast: Option<f64>,
    #[serde(default)] pub ema_medium: Option<f64>, #[serde(default)] pub ema_slow: Option<f64>,
    #[serde(default)] pub ema_long: Option<f64>, #[serde(default)] pub ema_stack_state: Option<String>,
    #[serde(default)] pub vwap: Option<f64>, #[serde(default)] pub close: Option<f64>,
    #[serde(default)] pub volume: Option<f64>, #[serde(default)] pub average_volume: Option<f64>,
    #[serde(default)] pub rvol: Option<f64>, #[serde(default)] pub vwap_bias: Option<String>,
    #[serde(default)] pub rsi_divergence_status: Option<String>, #[serde(default)] pub macd_divergence_status: Option<String>,
    #[serde(default)] pub macd_trend_state: Option<String>, #[serde(default)] pub macd_crossover_detected: Option<bool>,
    #[serde(default)] pub macd_crossover_direction: Option<String>, #[serde(default)] pub macd_histogram_peak: Option<f64>,
    #[serde(default)] pub squeeze_duration: Option<u32>, #[serde(default)] pub squeeze_release_trigger: Option<bool>,
    #[serde(default)] pub squeeze_momentum_direction: Option<String>, #[serde(default)] pub chart_pattern: Option<String>,
    #[serde(default)] pub chart_pattern_confidence: Option<f64>, #[serde(default)] pub bbwp: Option<f64>,
    #[serde(default)] pub atr_volatility_regime: Option<String>, #[serde(default)] pub current_price: Option<f64>,
    #[serde(default)] pub adx_slope: Option<f64>, #[serde(default)] pub adx_regime: Option<String>,
    #[serde(default)] pub adx_di_crossover_detected: Option<bool>, #[serde(default)] pub adx_di_crossover_direction: Option<String>,
}

// ─── Risk ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct RiskProfileCreate {
    pub profile_name: String, #[serde(default = "default_capital")] pub capital: f64,
    #[serde(default = "default_max_risk")] pub max_risk_pct: f64, pub leverage: i32,
    #[serde(default)] pub commission_pct: f64, #[serde(default)] pub funding_rate_8h: f64,
    #[serde(default)] pub spread: f64,
}
fn default_capital() -> f64 { 10000.0 }
fn default_max_risk() -> f64 { 2.0 }

#[derive(Debug, Deserialize)]
pub struct RiskCalculateRequest {
    pub profile_id: i64, pub direction: String, pub entry_price: f64,
    pub stop_loss: f64, pub take_profit: f64,
    #[serde(default)] pub max_risk_pct: Option<f64>, #[serde(default)] pub leverage: Option<i32>,
    #[serde(default)] pub commission_pct: Option<f64>, #[serde(default)] pub funding_rate_8h: Option<f64>,
    #[serde(default)] pub spread: Option<f64>, #[serde(default)] pub use_dynamic_atr: Option<bool>,
    #[serde(default)] pub atr_value: Option<f64>, #[serde(default)] pub capital: Option<f64>,
    #[serde(default)] pub stop_loss_price: Option<f64>, #[serde(default)] pub take_profit_price: Option<f64>,
    #[serde(default)] pub atr_multiplier: Option<f64>, #[serde(default)] pub atr_target_rr: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct CommissionProjectionPayload {
    pub profile_id: i64, pub direction: String, pub entry_1: f64, pub entry_2: f64,
    pub stop_loss_1: f64, pub stop_loss_2: f64, pub take_profit_1: f64, pub take_profit_2: f64,
    pub capital_entry_1_pct: f64, pub order_type: String,
    #[serde(default)] pub max_risk_pct: Option<f64>, #[serde(default)] pub leverage: Option<i32>,
    #[serde(default)] pub commission_pct: Option<f64>, #[serde(default)] pub funding_rate_8h: Option<f64>,
    #[serde(default)] pub capital: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct FeeTableQuery {
    pub order_type: String,
    #[serde(default)] pub capitals: Option<Vec<f64>>,
    #[serde(default)] pub leverages: Option<Vec<i32>>,
}

// ─── Exchange Keys ─────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct ExchangeKeyRequest {
    pub exchange: String, pub account_name: String, pub api_key: String,
    pub api_secret: String, #[serde(default)] pub passphrase: String,
    #[serde(default)] pub referred_uid: String, #[serde(default = "default_is_active")] pub is_active: bool,
}
fn default_is_active() -> bool { true }

// ─── Dashboard / Journal ───────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct StatsQuery { #[serde(default)] pub initial_capital: Option<f64> }

#[derive(Debug, Deserialize)]
pub struct TradeJournalQuery { #[serde(default = "default_journal_limit")] pub limit: u32 }
fn default_journal_limit() -> u32 { 50 }

#[derive(Debug, Deserialize)]
pub struct UpdateJournalNotesRequest { pub human_notes: String, pub execution_score: f64 }

#[derive(Debug, Deserialize)]
pub struct TradeLedgerQuery { #[serde(default = "default_limit")] pub limit: u32 }
fn default_limit() -> u32 { 200 }

#[derive(Debug, Deserialize)]
pub struct TradeTelemetryRequest {
    pub exchange: String, pub symbol: String, pub direction: String,
    pub entry_timestamp: i64, pub exit_timestamp: i64, pub entry_price: f64,
    pub exit_price: f64, pub size: f64, pub commission_fees: f64,
    pub funding_fees: f64, pub realized_pnl: f64, pub roi_percentage: f64,
    #[serde(default = "default_trigger")] pub trigger_source: String,
}
fn default_trigger() -> String { "MANUAL".to_string() }

// ─── Session ───────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct SessionInitRequest { pub mode: String, pub currency: String, pub exchange: String, pub capital: f64, #[serde(default)] pub user_name: Option<String> }

#[derive(Debug, Serialize)]
pub struct SessionStatusResponse { pub active: bool, pub mode: Option<String>, pub currency: Option<String>, pub exchange: Option<String>, pub capital: Option<f64>, pub instance_count: usize, pub max_instances: usize, pub user_name: Option<String>, pub wallet_address: Option<String> }

// ─── Profile Settings ──────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ProfileSettingsRequest {
    #[serde(default)]
    pub user_name: Option<String>,
    #[serde(default)]
    pub wallet_address: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ProfileSettingsResponse {
    pub user_name: Option<String>,
    pub wallet_address: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MaxInstancesRequest {
    pub max_instances: usize,
}

// ─── Instance ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct AddInstanceRequest { pub base: String, pub quote: String }

#[derive(Debug, Serialize)]
pub struct InstanceListResponse { pub instances: Vec<crate::registry::InstanceSummary>, pub total_count: usize, pub max_count: usize }

#[derive(Debug, Deserialize)]
pub struct InstanceDetailQuery { #[serde(default)] pub id: String, #[serde(default)] pub pair_key: Option<String> }

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstanceConfigPayload {
    #[serde(default)]
    pub micro_term: Option<crate::config::TimeframeConfig>,
    #[serde(default)]
    pub fast_term: Option<crate::config::TimeframeConfig>,
    #[serde(default)]
    pub slow_term: Option<crate::config::TimeframeConfig>,
    #[serde(default)]
    pub macro_term: Option<crate::config::TimeframeConfig>,
    #[serde(default)]
    pub automation: Option<crate::config::AutomationConfig>,
    #[serde(default)]
    pub operational_mode: Option<String>,
    #[serde(default)]
    pub weight_overrides: Option<std::collections::HashMap<String, i32>>,
    #[serde(default)]
    pub position_scaling: Option<crate::config::PositionScalingConfig>,
    #[serde(default)]
    pub ai_trigger: Option<crate::config::AiTriggerConfig>,
}

#[derive(Debug, Deserialize)]
pub struct InstanceManualRequest { pub action: String, pub direction: Option<String>, #[serde(default)] pub price: Option<f64> }

#[derive(Debug, Deserialize)]
pub struct InstanceApiKeyRequest { pub api_key: String, #[serde(default)] pub base_url: Option<String>, #[serde(default)] pub model: Option<String> }

#[derive(Debug, Serialize)]
pub struct InstanceUsageResponse {
    pub id: String, pub pair: String, pub symbol: String, pub status: String,
    pub initial_capital: f64, pub current_equity: f64, pub paper_balance: f64,
    pub paper_equity: f64, pub paper_unrealized_pnl: f64,
    pub consecutive_losses: u32,
    pub caution_level: String, pub instance_id: String, pub consecutive_failures: u32,
    pub failover_active: bool, pub failover_source: Option<String>,
    pub input_tokens: u64, pub output_tokens: u64,
}

#[derive(Debug, Deserialize)]
pub struct InstanceIntervalsRequest { pub slow_seconds: i64, pub normal_seconds: i64, pub fast_seconds: i64 }

#[derive(Debug, Deserialize)]
pub struct BackupApiKeyRequest { pub api_key: String, pub label: Option<String> }

#[derive(Debug, Deserialize)]
pub struct InstanceChatRequest { pub message: String, #[serde(default)] pub context: Option<String>, #[serde(default)] pub history: Vec<crate::llm::ChatMessage> }
