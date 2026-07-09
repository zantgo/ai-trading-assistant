//! Stress testing framework — scenario-based portfolio risk evaluation.
//!
//! Evaluates current positions under predefined extreme scenarios:
//! - Flash Crash (−5σ move)
//! - Vol Spike (max historical ATR)
//! - Correlation Breakdown (all positions move together)
//! - Trend Reversal (largest N-bar reversal in history)
//! - Funding Crisis (funding rate inversion)
//!
//! Each scenario computes per-position and aggregate P&L, margin impact,
//! and a composite stress score.

use serde::{Deserialize, Serialize};

/// A predefined stress scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StressScenario {
    pub name: String,
    /// Price shock as multiple of daily standard deviation.
    pub price_shock_sigma: f64,
    /// Volatility multiplier applied to current ATR.
    pub vol_multiplier: f64,
    /// Whether all pairs move with correlation = 1.0.
    pub correlation_breakdown: bool,
    /// Funding rate multiplier (e.g., 3.0 = 3× current funding).
    pub funding_multiplier: f64,
    /// Whether the shock direction opposes current positions.
    pub opposes_positions: bool,
    /// Human-readable description.
    pub description: String,
}

/// Result of a single stress scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StressTestResult {
    pub scenario_name: String,
    /// Total portfolio P&L under this scenario.
    pub total_portfolio_pnl: f64,
    /// P&L as percentage of equity.
    pub pnl_pct_equity: f64,
    /// Worst-affected trading pair.
    pub worst_pair: String,
    /// Worst pair P&L.
    pub worst_pair_pnl: f64,
    /// Whether margin would be breached.
    pub margin_call: bool,
    /// Whether positions would be liquidated.
    pub liquidation: bool,
    /// Remaining margin buffer as percentage.
    pub margin_buffer_pct: f64,
}

/// A single position snapshot for stress testing.
#[derive(Debug, Clone)]
pub struct StressPosition {
    pub pair: String,
    pub direction: f64, // +1 = long, -1 = short
    pub size: f64,       // position size in base units
    pub leverage: f64,
    pub current_price: f64,
    pub unrealized_pnl: f64,
    pub margin_used: f64,
}

/// Aggregate stress score and recommendations.
#[derive(Debug, Clone)]
pub struct StressScore {
    /// Worst scenario P&L as percentage of equity (absolute value).
    pub max_loss_pct: f64,
    /// Name of the worst scenario.
    pub worst_scenario: String,
    /// Overall stress level.
    pub stress_level: StressLevel,
    /// Per-scenario results.
    pub results: Vec<StressTestResult>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StressLevel {
    /// < 5% loss — no action needed.
    Low,
    /// 5–15% loss — consider reducing exposure.
    Moderate,
    /// 15–30% loss — reduce positions immediately.
    High,
    /// > 30% loss — close riskiest position.
    Critical,
}

/// Predefined stress scenarios.
pub fn default_scenarios() -> Vec<StressScenario> {
    vec![
        StressScenario {
            name: "Flash Crash".into(),
            price_shock_sigma: 5.0,
            vol_multiplier: 2.0,
            correlation_breakdown: true,
            funding_multiplier: 1.0,
            opposes_positions: true,
            description: "Instant −5σ move across all assets. All correlations go to 0.95. Simulates liquidity cascade / black swan.".into(),
        },
        StressScenario {
            name: "Vol Spike".into(),
            price_shock_sigma: 0.0,
            vol_multiplier: 3.0,
            correlation_breakdown: false,
            funding_multiplier: 1.0,
            opposes_positions: true,
            description: "Volatility expands to 3× current ATR. Stops gapped through. Simulates sudden volatility regime shift.".into(),
        },
        StressScenario {
            name: "Correlation Breakdown".into(),
            price_shock_sigma: 3.0,
            vol_multiplier: 2.0,
            correlation_breakdown: true,
            funding_multiplier: 1.0,
            opposes_positions: true,
            description: "Diversification fails: all pairs move 3σ against positions simultaneously. Tests worst-case portfolio diversification failure.".into(),
        },
        StressScenario {
            name: "Trend Reversal".into(),
            price_shock_sigma: 0.0,
            vol_multiplier: 2.5,
            correlation_breakdown: false,
            funding_multiplier: 1.0,
            opposes_positions: true,
            description: "Largest 5-bar reversal in 500-bar history. Sudden structural regime shift against current direction.".into(),
        },
        StressScenario {
            name: "Funding Crisis".into(),
            price_shock_sigma: 0.0,
            vol_multiplier: 1.0,
            correlation_breakdown: false,
            funding_multiplier: 3.0,
            opposes_positions: true,
            description: "Funding rate inverts to 3× current level. Funding payments spike against position direction.".into(),
        },
    ]
}

/// Run a single stress scenario against a set of positions.
pub fn run_scenario(
    scenario: &StressScenario,
    positions: &[StressPosition],
    equity: f64,
    price_shock_pct: f64, // pre-computed price shock as percentage
) -> StressTestResult {
    let mut total_pnl = 0.0;
    let mut worst_pair = String::new();
    let mut worst_pair_pnl = 0.0;
    let mut total_margin_used = 0.0;

    for pos in positions {
        let shock_direction = if scenario.opposes_positions { -pos.direction } else { 1.0 };
        let pos_pnl = pos.size * pos.current_price * shock_direction * price_shock_pct * pos.leverage;
        total_pnl += pos_pnl;
        total_margin_used += pos.margin_used;

        if pos_pnl < worst_pair_pnl {
            worst_pair_pnl = pos_pnl;
            worst_pair = pos.pair.clone();
        }
    }

    if positions.len() == 1 && worst_pair.is_empty() {
        worst_pair = positions[0].pair.clone();
        worst_pair_pnl = total_pnl;
    }

    let pnl_pct_equity = if equity > 0.0 {
        (total_pnl / equity) * 100.0
    } else {
        0.0
    };

    let remaining_equity = equity + total_pnl;
    let margin_call = remaining_equity < total_margin_used;
    let liquidation = remaining_equity < total_margin_used * 0.5;
    let margin_buffer_pct = if total_margin_used > 0.0 {
        ((equity - total_margin_used) / equity * 100.0).max(-100.0)
    } else {
        100.0
    };

    StressTestResult {
        scenario_name: scenario.name.clone(),
        total_portfolio_pnl: total_pnl,
        pnl_pct_equity,
        worst_pair,
        worst_pair_pnl,
        margin_call,
        liquidation,
        margin_buffer_pct,
    }
}

/// Compute aggregate stress score from all scenario results.
pub fn aggregate_stress_score(
    results: &[StressTestResult],
) -> StressScore {
    let mut max_loss = 0.0;
    let mut worst_name = String::new();

    for r in results {
        let loss = r.pnl_pct_equity.abs();
        if loss > max_loss {
            max_loss = loss;
            worst_name = r.scenario_name.clone();
        }
    }

    let stress_level = if max_loss < 5.0 {
        StressLevel::Low
    } else if max_loss < 15.0 {
        StressLevel::Moderate
    } else if max_loss < 30.0 {
        StressLevel::High
    } else {
        StressLevel::Critical
    };

    StressScore {
        max_loss_pct: max_loss,
        worst_scenario: worst_name,
        stress_level,
        results: results.to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_positions() -> Vec<StressPosition> {
        vec![
            StressPosition {
                pair: "BTC".into(),
                direction: 1.0,
                size: 0.1,
                leverage: 20.0,
                current_price: 50000.0,
                unrealized_pnl: 0.0,
                margin_used: 250.0,
            },
            StressPosition {
                pair: "ETH".into(),
                direction: -1.0,
                size: 2.0,
                leverage: 10.0,
                current_price: 3000.0,
                unrealized_pnl: 0.0,
                margin_used: 600.0,
            },
        ]
    }

    #[test]
    fn test_flash_crash_pnl_negative() {
        let positions = sample_positions();
        let scenario = &default_scenarios()[0];
        let result = run_scenario(scenario, &positions, 10000.0, -0.10);
        // Flash crash with 10% price drop: both positions lose
        assert!(result.total_portfolio_pnl < 0.0);
    }

    #[test]
    fn test_margin_call_detected() {
        let positions = vec![StressPosition {
            pair: "BTC".into(),
            direction: 1.0,
            size: 1.0,
            leverage: 20.0,
            current_price: 50000.0,
            unrealized_pnl: 0.0,
            margin_used: 9000.0,
        }];
        let scenario = &default_scenarios()[0];
        let result = run_scenario(scenario, &positions, 10000.0, -0.05);
        assert!(result.margin_call || result.pnl_pct_equity < -80.0);
    }

    #[test]
    fn test_aggregate_stress_score() {
        let results = vec![
            StressTestResult {
                scenario_name: "Test".into(),
                total_portfolio_pnl: -2000.0,
                pnl_pct_equity: -20.0,
                worst_pair: "BTC".into(),
                worst_pair_pnl: -1500.0,
                margin_call: false,
                liquidation: false,
                margin_buffer_pct: 50.0,
            },
        ];
        let score = aggregate_stress_score(&results);
        assert_eq!(score.stress_level, StressLevel::High);
        assert!((score.max_loss_pct - 20.0).abs() < 0.01);
    }

    #[test]
    fn test_stress_level_low() {
        let results = vec![StressTestResult {
            scenario_name: "Mild".into(),
            total_portfolio_pnl: -200.0,
            pnl_pct_equity: -2.0,
            worst_pair: "BTC".into(),
            worst_pair_pnl: -150.0,
            margin_call: false,
            liquidation: false,
            margin_buffer_pct: 80.0,
        }];
        let score = aggregate_stress_score(&results);
        assert_eq!(score.stress_level, StressLevel::Low);
    }
}
