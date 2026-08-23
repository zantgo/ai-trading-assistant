//! # Performance Analytics DTOs
//!
//! Stateless output types for the Performance Analytics Engine (PAE).
//! Consumed by the API gateway and persisted by database-storage.
//! All four PAE layers (L1-L4) are represented.

use serde::{Deserialize, Serialize};

// ─── L1: Trade Analytics Matrix ─────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeAnalyticsRecord {
    pub trade_id: String,
    pub symbol: String,
    pub direction: String,
    pub entry_timestamp: i64,
    pub exit_timestamp: i64,
    pub hold_time_seconds: u64,
    pub entry_price: f64,
    pub exit_price: f64,
    pub size: f64,
    pub gross_pnl: f64,
    pub net_pnl: f64,
    pub roi_pct: f64,
    pub execution_slippage: f64,
    pub mfe: f64,
    pub mae: f64,
    pub trigger_source: String,
    pub exit_reason: String,
    pub flat_trade: bool,
}

// ─── L2: Strategy Analytics Matrix ──────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyAnalyticsRow {
    pub setup_type: String,
    /// Significance bar (0.05 = 5%). `is_significant` requires both the
    /// t-test p-value and the Monte Carlo p-value below this threshold.
    pub alpha: f64,
    pub total_trades: u32,
    pub win_count: u32,
    pub loss_count: u32,
    pub win_rate: f64,
    pub gross_profit: f64,
    pub gross_loss: f64,
    pub profit_factor: Option<f64>,
    pub average_win: f64,
    pub average_loss: f64,
    pub avg_win_loss_ratio: f64,
    pub expectancy: f64,
    pub slippage_overhead: f64,
    pub t_statistic: f64,
    pub p_value: f64,
    pub p_mc: f64,
    pub monte_carlo_runs: u32,
    pub is_significant: bool,
    pub classification: PerformanceClassification,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PerformanceClassification {
    StrongEdge,
    ModerateEdge,
    WeakMarginalEdge,
    NoEdgeNegative,
    InsufficientData,
}

// ─── L3: Risk Analytics Matrix ──────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAnalyticsRow {
    pub maximum_drawdown_pct: f64,
    pub max_drawdown_duration_days: f64,
    pub average_drawdown_pct: f64,
    pub drawdown_count: u32,
    pub sharpe_ratio: Option<f64>,
    pub sortino_ratio: Option<f64>,
    pub ulcer_index: f64,
    pub calmar_ratio: Option<f64>,
    pub daily_volatility: f64,
    pub downside_deviation: f64,
    pub value_at_risk_95: f64,
    pub expected_shortfall_95: f64,
    /// v10.1: Sharpe computed over **log** daily returns (time-additive,
    /// unbiased for skewed/volatile equity curves). `None` when the curve
    /// is too short or flat.
    #[serde(default)]
    pub sharpe_ratio_log: Option<f64>,
}

// ─── v10.1 Direction Symmetry Verdict ────────────────────────────────

/// Welch two-sample t-test comparing LONG vs SHORT per-trade returns.
/// H0: the two directions' returns are statistically equal.
/// Primary statistic = `roi_pct` (size-normalized); USD expectancy is
/// reported as context. Only produced with ≥10 trades per side.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectionSymmetryVerdict {
    pub long_count: u32,
    pub short_count: u32,
    pub long_expectancy_usd: f64,
    pub short_expectancy_usd: f64,
    pub long_win_rate: f64,
    pub short_win_rate: f64,
    pub t_statistic: f64,
    pub degrees_of_freedom: f64,
    pub p_value: f64,
    pub significant: bool,
    /// SYMMETRIC | LONG_BETTER | SHORT_BETTER (significant only).
    pub verdict: String,
}

// ─── L4: Performance Matrix (Regime Compatibility) ──────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMatrixRow {
    pub setup_type: String,
    pub regime: String,
    pub trade_count: u32,
    pub win_rate: f64,
    pub profit_factor: Option<f64>,
    pub avg_r_multiple: f64,
    pub total_pnl: f64,
    pub compatibility_label: RegimeCompatibility,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RegimeCompatibility {
    Strong,
    Favorable,
    Marginal,
    Avoid,
}

// ─── Dashboard Stats (moved from performance-analytics) ─────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardStats {
    pub core_stats: CoreStats,
    pub equity_curve: Vec<(i64, f64)>,
    pub compounded_curve: Vec<(i64, f64)>,
    pub daily_activity: Vec<DailyActivity>,
    pub daily_pnl: Vec<DailyPnl>,
    pub win_rate_by_hour: Vec<HourlyWinRate>,
    pub win_rate_by_weekday: Vec<WeekdayWinRate>,
    pub direction_breakdown: DirectionBreakdown,
    pub trader_style: TraderStyleBreakdown,
    pub winning_streaks: StreakMetrics,
    pub losing_streaks: StreakMetrics,
    pub post_loss_recovery_pct: f64,
    pub pnl_calendar: Vec<CalendarDay>,
    pub pair_volume: Vec<PairStat>,
    pub top_pairs_profitability: Vec<PairStat>,
    pub bottom_pairs_profitability: Vec<PairStat>,
    pub daily_commissions: Vec<DailyCommission>,
    pub cumulative_commissions: Vec<(i64, f64)>,
    pub fee_pnl_ratio: Vec<FeePnlRatio>,
    pub monthly_summary: Vec<MonthlySummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreStats {
    pub total_pnl: f64,
    pub win_rate: f64,
    pub avg_loss: f64,
    pub avg_gain: f64,
    pub expectancy: f64,
    pub avg_risk_reward_ratio: f64,
    pub profit_factor: f64,
    pub largest_loss: f64,
    pub largest_gain: f64,
    pub total_trades: usize,
    pub wins: usize,
    pub losses: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyActivity {
    pub date: String,
    pub longs: usize,
    pub shorts: usize,
    pub win_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyPnl {
    pub date: String,
    pub pnl: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HourlyWinRate {
    pub hour: u32,
    pub win_rate: f64,
    pub volume: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeekdayWinRate {
    pub weekday: String,
    pub win_rate: f64,
    pub volume: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectionBreakdown {
    pub longs: usize,
    pub shorts: usize,
    pub long_expectancy: f64,
    pub short_expectancy: f64,
    pub long_wins: usize,
    pub long_losses: usize,
    pub long_win_rate: f64,
    pub long_avg_gain: f64,
    pub long_avg_loss: f64,
    pub short_wins: usize,
    pub short_losses: usize,
    pub short_win_rate: f64,
    pub short_avg_gain: f64,
    pub short_avg_loss: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraderStyleBreakdown {
    pub scalper: StyleSegment,
    pub day_trader: StyleSegment,
    pub swing_trader: StyleSegment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleSegment {
    pub count: usize,
    pub avg_duration_minutes: f64,
    pub win_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreakMetrics {
    pub avg_streak_length: f64,
    pub max_consecutive_value: f64,
    pub max_streak_length: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarDay {
    pub date: String,
    pub pnl: f64,
    pub month: u32,
    pub day: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairStat {
    pub symbol: String,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyCommission {
    pub date: String,
    pub fees: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeePnlRatio {
    pub date: String,
    pub ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonthlySummary {
    pub month: String,
    pub net_pnl: f64,
    pub win_rate: f64,
    pub trade_count: usize,
}

// ─── L4: Performance Matrix Summary (aggregate) ─────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMatrixSummary {
    pub setup_type: String,
    pub total_trades: u32,
    pub overall_profit_factor: Option<f64>,
    pub overall_expectancy: f64,
    pub overall_sharpe: Option<f64>,
    pub overall_sortino: Option<f64>,
    pub max_drawdown_pct: f64,
    pub regime_compatibility: Vec<PerformanceMatrixRow>,
    pub regime_strength_summary: Vec<RegimeStrengthEntry>,
    pub optimization_recommendations: Vec<String>,
    pub overall_rating: OverallRating,
    pub last_evaluated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegimeStrengthEntry {
    pub regime: String,
    pub rank: u32,
    pub strength: RegimeCompatibility,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OverallRating {
    Excellent,
    Good,
    Fair,
    Poor,
    Unrated,
}

// ─── Optimization Report ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegimePerformanceReport {
    pub regime: String,
    pub trade_count: i64,
    pub win_rate: f64,
    pub profit_factor: f64,
    pub avg_r_multiple: f64,
    pub total_pnl: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationReport {
    pub timestamp: i64,
    pub total_trades: i64,
    pub regime_reports: Vec<RegimePerformanceReport>,
    pub recommendations: Vec<String>,
}
