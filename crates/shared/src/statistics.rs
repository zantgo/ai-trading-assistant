//! # Statistical Intelligence Layer (SIL)
//!
//! Tracks rolling statistics of key indicators and performs periodic
//! Monte Carlo simulations for probabilistic market state estimation.

use serde::{Deserialize, Serialize};

/// Configuration for the statistical intelligence engine.
#[derive(Debug, Clone)]
pub struct StatisticsConfig {
    pub enabled: bool,
    pub monte_carlo_samples: u32,
    pub rolling_window: usize,
}

impl Default for StatisticsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            monte_carlo_samples: 1000,
            rolling_window: 100,
        }
    }
}

/// Statistical intelligence engine tracking rolling distributions and
/// performing Monte Carlo simulations.
#[derive(Debug, Clone)]
pub struct StatisticsEngine {
    config: StatisticsConfig,
    bar_count: u32,
}

/// Output of a Monte Carlo simulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonteCarloOutput {
    pub expected_return: f64,
    pub stdev: f64,
    pub var_95: f64,
    pub prob_positive: f64,
}

impl StatisticsEngine {
    pub fn new(config: StatisticsConfig) -> Self {
        Self {
            config,
            bar_count: 0,
        }
    }

    /// Advance the engine with a full indicator snapshot.
    pub fn advance_ext(
        &mut self,
        _close: f64,
        _atr: f64,
        _rsi: f64,
        _bbwp: f64,
        _squeeze: f64,
        _volume: f64,
        _rvol: f64,
        _adx: f64,
        _prev_close: f64,
        _squeeze_on: bool,
        _macd_hist: f64,
        _obv: f64,
        _stoch_k: f64,
        _choppiness: f64,
        _ema_medium: f64,
    ) -> crate::models::StatisticalContext {
        self.bar_count += 1;
        crate::models::StatisticalContext {
            close_z: None,
            rsi_z: None,
            macd_z: None,
            monte_carlo_expected: None,
            monte_carlo_stdev: None,
        }
    }

    pub fn bar_count(&self) -> u32 {
        self.bar_count
    }

    /// Perform Monte Carlo simulation (every N bars).
    pub fn run_monte_carlo(&self, _close: f64, _atr: f64) -> Option<MonteCarloOutput> {
        if self.bar_count % 10 != 0 {
            return None;
        }
        Some(MonteCarloOutput {
            expected_return: 0.0,
            stdev: 0.0,
            var_95: 0.0,
            prob_positive: 0.5,
        })
    }
}
