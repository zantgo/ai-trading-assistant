use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── Safety State ──────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SafetyState {
    Normal,
    Warn,
    Cautious,
    Suspended,
    DrawdownStop,
}

impl Default for SafetyState {
    fn default() -> Self {
        SafetyState::Normal
    }
}

impl SafetyState {
    pub fn as_str(&self) -> &'static str {
        match self {
            SafetyState::Normal => "NORMAL",
            SafetyState::Warn => "WARN",
            SafetyState::Cautious => "CAUTIOUS",
            SafetyState::Suspended => "SUSPENDED",
            SafetyState::DrawdownStop => "DRAWDOWN_STOP",
        }
    }
}

// ─── Correlation Map ───────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CorrelationMap {
    pub pairs: HashMap<String, f64>,
}

// ─── Veto Trigger ──────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VetoTrigger {
    pub condition: String,
    pub target_stance: String,
    pub reason: String,
    pub hard_exit: bool,
}

// ─── L1: Position Matrix ───────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PositionMatrix {
    pub position_id: u64,
    pub symbol: String,
    pub direction: String,
    pub entry_price: Decimal,
    pub average_entry_price: Decimal,
    pub size: Decimal,
    pub allocated_usd: Decimal,
    pub entry_timestamp: u64,

    pub current_price: Decimal,
    pub unrealized_pnl: Decimal,
    pub roi_pct: Decimal,
    pub unrealized_pnl_after_fees: Decimal,

    pub stop_loss_price: Option<Decimal>,
    pub take_profit_price: Option<Decimal>,
    pub invalidation_level: Option<Decimal>,
    pub target_profit_ratio: Option<Decimal>,

    pub current_portions: u32,
    pub initial_allocated_margin: Decimal,
    pub realized_pnl_accumulator: Decimal,
}

// ─── L2: Exposure Matrix ───────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExposureMatrix {
    pub gross_exposure: Decimal,
    pub net_exposure: Decimal,
    pub net_exposure_pct: Decimal,
    pub long_exposure: Decimal,
    pub short_exposure: Decimal,
    pub symbol_concentration: HashMap<String, Decimal>,
    pub sector_concentration: HashMap<String, Decimal>,
    pub max_single_pair_pct: Decimal,
    pub correlation_matrix: CorrelationMap,
}

// ─── L3: Capital Matrix ────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CapitalMatrix {
    pub initial_balance: Decimal,
    pub current_equity: Decimal,
    pub available_margin: Decimal,
    pub committed_margin: Decimal,
    pub realized_pnl: Decimal,
    pub unrealized_pnl: Decimal,

    pub margin_usage_ratio: Decimal,
    pub leverage_ratio: Decimal,
    pub max_daily_drawdown_pct: Decimal,
    pub daily_pnl: Decimal,
    pub starting_session_equity: Decimal,
}

// ─── L4: Portfolio Matrix ──────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PortfolioMatrix {
    pub current_equity: Decimal,
    pub realized_pnl: Decimal,
    pub unrealized_pnl: Decimal,
    pub gross_exposure: Decimal,
    pub net_exposure: Decimal,
    pub margin_usage_ratio: Decimal,
    pub leverage_ratio: Decimal,
    pub daily_pnl: Decimal,
    pub max_daily_drawdown_pct: Decimal,
    pub drawdown_limit_pct: Decimal,
    pub peak_equity: Decimal,
    pub safety_state: SafetyState,
    pub systemic_risk_score: f64,
    pub active_stances: HashMap<String, String>,
    pub default_stances: HashMap<String, String>,
    pub consecutive_losses: HashMap<String, u32>,
    pub position_count: u32,
}
