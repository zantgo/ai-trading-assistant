use crate::db::{CompletedTradesBufferRow, DecisionMemoryBufferRow};
use crate::llm::{ChatMessage, IndividualIndicatorResult};
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Default, Deserialize)]
#[allow(dead_code)]
pub struct IndicatorSnapshot {
    pub rsi: Option<f64>,
    pub squeeze_on: Option<bool>,
    pub squeeze_momentum: Option<f64>,
    pub macd_line: Option<f64>,
    pub macd_signal: Option<f64>,
    pub macd_histogram: Option<f64>,
    pub macd_histogram_trend: Option<String>,
    pub adx: Option<f64>,
    pub adx_plus: Option<f64>,
    pub adx_minus: Option<f64>,
    pub bb_upper: Option<f64>,
    pub bb_middle: Option<f64>,
    pub bb_lower: Option<f64>,
    pub atr: Option<f64>,
    pub atr_trend: Option<String>,
    pub atr_volatility_regime: Option<String>,
    pub current_price: Option<f64>,
    pub volume: Option<f64>,
    pub average_volume: Option<f64>,
    pub rvol: Option<f64>,
    pub ema_fast: Option<f64>,
    pub ema_medium: Option<f64>,
    pub ema_slow: Option<f64>,
    pub ema_long: Option<f64>,
    pub ema_stack_state: Option<String>,
    pub vwap: Option<f64>,
    pub vwap_bias: Option<String>,
    pub rsi_divergence_status: Option<String>,
    pub macd_divergence_status: Option<String>,
    pub macd_trend_state: Option<String>,
    pub macd_crossover_detected: Option<bool>,
    pub macd_crossover_direction: Option<String>,
    pub macd_histogram_peak: Option<f64>,
    pub squeeze_duration: Option<u32>,
    pub squeeze_release_trigger: Option<bool>,
    pub squeeze_momentum_direction: Option<String>,
    pub chart_pattern: Option<String>,
    pub chart_pattern_confidence: Option<f64>,
    pub bbwp: Option<f64>,
    pub adx_slope: Option<f64>,
    pub adx_regime: Option<String>,
    pub adx_di_crossover_detected: Option<bool>,
    pub adx_di_crossover_direction: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SupportResistanceResponse {
    pub detected_support_levels: Vec<String>,
    pub detected_resistance_levels: Vec<String>,
    pub structural_analysis: String,
}

#[derive(Debug, Serialize)]
pub struct IndicatorSynthesisResponse {
    pub summary_count: String,
    pub evaluation: String,
}

#[derive(Debug, Serialize)]
pub struct PositionRecommendationResponse {
    pub action: String,
    pub rationale: String,
}

#[derive(Debug, Serialize)]
pub struct PhaseTwoResponse {
    pub general_trend: String,
    pub support_and_resistance: SupportResistanceResponse,
    pub indicator_synthesis: IndicatorSynthesisResponse,
    pub position_recommendation: PositionRecommendationResponse,
}

#[derive(Debug, Serialize)]
pub struct MultiAgentAnalysisResponse {
    pub phase_one: Vec<IndividualIndicatorResult>,
    pub phase_two: PhaseTwoResponse,
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

#[derive(Debug, Serialize)]
pub struct IndicatorHistoryArrays {
    pub symbol: String,
    pub timeframe_secs: u64,
    pub times: Vec<u64>,
    pub rsi_14: Vec<Option<String>>,
    pub squeeze_on: Vec<Option<bool>>,
    pub squeeze_momentum: Vec<Option<String>>,
    pub macd_line: Vec<Option<String>>,
    pub macd_signal: Vec<Option<String>>,
    pub macd_hist: Vec<Option<String>>,
    pub adx_14: Vec<Option<String>>,
    pub adx_plus: Vec<Option<String>>,
    pub adx_minus: Vec<Option<String>>,
    pub atr_14: Vec<Option<String>>,
    pub ema_fast: Vec<Option<String>>,
    pub ema_medium: Vec<Option<String>>,
    pub ema_slow: Vec<Option<String>>,
    pub ema_long: Vec<Option<String>>,
    pub bbwp: Vec<Option<String>>,
    pub vwap: Vec<Option<String>>,
    pub bb_upper: Vec<Option<String>>,
    pub bb_middle: Vec<Option<String>>,
    pub bb_lower: Vec<Option<String>>,
    pub rvol: Vec<Option<String>>,
}

#[derive(Debug, Serialize)]
pub struct HistoryResponse {
    pub prices: Vec<String>,
    pub candles: Vec<HistoryCandle>,
    pub indicator_history: IndicatorHistoryArrays,
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
pub struct SessionInitRequest { pub mode: String, pub currency: String, pub exchange: String, pub capital: f64 }

#[derive(Debug, Serialize)]
pub struct SessionStatusResponse { pub active: bool, pub mode: Option<String>, pub currency: Option<String>, pub exchange: Option<String>, pub capital: Option<f64>, pub instance_count: usize, pub max_instances: usize }

// ─── Instance ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct AddInstanceRequest { pub base: String, pub quote: String }

#[derive(Debug, Serialize)]
pub struct InstanceListResponse { pub instances: Vec<crate::registry::InstanceSummary>, pub total_count: usize, pub max_count: usize }

#[derive(Debug, Deserialize)]
pub struct InstanceDetailQuery { #[serde(default)] pub id: String, #[serde(default)] pub pair_key: Option<String> }

#[derive(Debug, Deserialize)]
pub struct InstanceConfigPayload { pub micro_term: crate::config::TimeframeConfig, pub fast_term: crate::config::TimeframeConfig, pub slow_term: Option<crate::config::TimeframeConfig>, pub macro_term: Option<crate::config::TimeframeConfig>, pub automation: crate::config::AutomationConfig }

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
