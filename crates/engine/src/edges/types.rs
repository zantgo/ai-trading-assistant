use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EdgeArchetype {
    TrendFollowing,
    MeanReversion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegimeGates {
    pub trending: bool,
    pub compression: bool,
    pub expansion: bool,
    pub range: bool,
}

impl Default for RegimeGates {
    fn default() -> Self {
        Self {
            trending: true,
            compression: false,
            expansion: false,
            range: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TriggerRule {
    Crossover,
    OverboughtOversold,
    Divergence,
    SlopeDirection,
    ThresholdAbove,
    ThresholdBelow,
    Release,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndicatorConfig {
    pub name: String,
    pub weight: f64,
    pub trigger_rule: TriggerRule,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SizingModel {
    Fixed,
    VolatilityTargeting,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum StopLossModel {
    AtrVolatilityStop,
    StructuralPivot,
    FixedPercentage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TriggerPhase {
    ExecuteOnTrigger,
    ExecuteOnConfirmedClose,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SizingConfig {
    pub model: SizingModel,
    #[serde(default = "default_daily_vol_target")]
    pub daily_vol_target_pct: f64,
    #[serde(default = "default_max_leverage")]
    pub max_leverage: f64,
}

fn default_daily_vol_target() -> f64 { 2.0 }
fn default_max_leverage() -> f64 { 20.0 }

impl Default for SizingConfig {
    fn default() -> Self {
        Self {
            model: SizingModel::Fixed,
            daily_vol_target_pct: 2.0,
            max_leverage: 20.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopLossConfig {
    pub model: StopLossModel,
    #[serde(default = "default_atr_multiplier")]
    pub atr_multiplier: f64,
}

fn default_atr_multiplier() -> f64 { 2.0 }

impl Default for StopLossConfig {
    fn default() -> Self {
        Self {
            model: StopLossModel::AtrVolatilityStop,
            atr_multiplier: 2.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TakeProfitConfig {
    pub tp1_multiplier: f64,
    pub tp2_multiplier: f64,
    pub tp3_multiplier: f64,
}

impl Default for TakeProfitConfig {
    fn default() -> Self {
        Self {
            tp1_multiplier: 2.5,
            tp2_multiplier: 5.0,
            tp3_multiplier: 8.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfig {
    #[serde(default = "default_min_rvol")]
    pub min_rvol: f64,
    #[serde(default = "default_climax_rvol")]
    pub climax_rvol: f64,
    pub trigger_phase: TriggerPhase,
    #[serde(default)]
    pub vwap_filter: bool,
}

fn default_min_rvol() -> f64 { 1.5 }
fn default_climax_rvol() -> f64 { 3.0 }

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            min_rvol: 1.5,
            climax_rvol: 3.0,
            trigger_phase: TriggerPhase::ExecuteOnConfirmedClose,
            vwap_filter: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeConfig {
    pub archetype: EdgeArchetype,
    #[serde(default)]
    pub regime_gates: RegimeGates,
    #[serde(default = "default_quorum_threshold")]
    pub quorum_threshold: f64,
    #[serde(default)]
    pub mtf_quorum: Vec<String>,
    #[serde(default)]
    pub indicators: Vec<IndicatorConfig>,
    #[serde(default)]
    pub sizing: SizingConfig,
    #[serde(default)]
    pub stop_loss: StopLossConfig,
    #[serde(default)]
    pub take_profit: TakeProfitConfig,
    #[serde(default)]
    pub execution: ExecutionConfig,
    #[serde(default = "default_backtest_depth")]
    pub backtest_depth: usize,
}

fn default_quorum_threshold() -> f64 { 60.0 }
fn default_backtest_depth() -> usize { 10000 }

impl Default for EdgeConfig {
    fn default() -> Self {
        Self {
            archetype: EdgeArchetype::TrendFollowing,
            regime_gates: RegimeGates::default(),
            quorum_threshold: 60.0,
            mtf_quorum: vec!["micro".to_string(), "fast".to_string()],
            indicators: Vec::new(),
            sizing: SizingConfig::default(),
            stop_loss: StopLossConfig::default(),
            take_profit: TakeProfitConfig::default(),
            execution: ExecutionConfig::default(),
            backtest_depth: 10000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeLog {
    pub entry_index: usize,
    pub exit_index: usize,
    pub direction: String,
    pub entry_price: f64,
    pub exit_price: f64,
    pub pnl_pct: f64,
    pub pnl_absolute: f64,
    pub exit_reason: String,
    pub regime_at_entry: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalMetrics {
    pub total_trades: usize,
    pub win_rate: f64,
    pub profit_factor: f64,
    pub net_sharpe_ratio: f64,
    pub max_drawdown_pct: f64,
    pub max_drawdown_duration: usize,
    pub total_return_pct: f64,
    pub avg_trade_return_pct: f64,
    pub avg_win_pct: f64,
    pub avg_loss_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquityPoint {
    pub trade_index: usize,
    pub cumulative_return_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestOutput {
    pub trade_logs: Vec<TradeLog>,
    pub equity_curve: Vec<EquityPoint>,
    pub in_sample_equity: Vec<EquityPoint>,
    pub out_of_sample_equity: Vec<EquityPoint>,
    pub metrics: HistoricalMetrics,
    pub trade_returns: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonteCarloPath {
    pub path_index: usize,
    pub equity_points: Vec<f64>,
    pub max_drawdown_pct: f64,
    pub final_return_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonteCarloResult {
    pub paths: Vec<MonteCarloPath>,
    pub avg_final_return_pct: f64,
    pub median_max_drawdown_pct: f64,
    pub worst_case_drawdown_pct: f64,
    pub drawdown_distribution: Vec<DrawdownBucket>,
    pub probability_of_ruin_pct: f64,
    pub confidence_95_drawdown_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrawdownBucket {
    pub bucket_pct: f64,
    pub frequency: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapResult {
    pub p_value: f64,
    pub is_significant: bool,
    pub mean_return: f64,
    pub confidence_95_lower: f64,
    pub confidence_95_upper: f64,
    pub iterations: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeAnalysisResponse {
    pub edge_id: i64,
    pub edge_name: String,
    pub symbol: String,
    pub timeframe_secs: u64,
    pub backtest_depth: usize,
    pub historical_metrics: HistoricalMetrics,
    pub backtest_curve: BacktestCurveData,
    pub bootstrap_p_value: f64,
    pub bootstrap_significant: bool,
    pub monte_carlo_paths: Vec<MonteCarloPath>,
    pub drawdown_distribution: Vec<DrawdownBucket>,
    pub probability_of_ruin_pct: f64,
    pub confidence_95_drawdown_pct: f64,
    pub skewness: f64,
    pub cached: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestCurveData {
    pub in_sample: Vec<EquityPoint>,
    pub out_of_sample: Vec<EquityPoint>,
    pub combined: Vec<EquityPoint>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EdgeSaveRequest {
    pub name: String,
    pub pair_key: String,
    #[serde(default)]
    pub description: String,
    pub config: EdgeConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EdgeAnalyzeRequest {
    pub edge_id: i64,
    pub symbol: String,
    #[serde(default = "default_timeframe")]
    pub timeframe_secs: u64,
}

fn default_timeframe() -> u64 { 60 }

#[derive(Debug, Clone, Serialize)]
pub struct SavedEdge {
    pub id: i64,
    pub name: String,
    pub pair_key: String,
    pub description: Option<String>,
    pub config: EdgeConfig,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SavedEdgeRow {
    pub id: i64,
    pub name: String,
    pub pair_key: String,
    pub description: Option<String>,
    pub config_payload: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CachedAnalyticsRow {
    pub edge_id: i64,
    pub historical_metrics: String,
    pub monte_carlo_paths: String,
    pub bootstrap_results: String,
    pub generated_at: String,
}
