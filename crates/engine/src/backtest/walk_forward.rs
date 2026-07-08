use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::backtest::engine::{BacktestConfig, BacktestEngine, BacktestResult};
use crate::backtest::metrics::compute_sharpe;

#[derive(Debug, Clone)]
pub struct ParameterRange {
    pub name: String,
    pub values: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalkForwardResult {
    pub best_params: Vec<ParameterValue>,
    pub oos_sharpe: f64,
    pub oos_metrics: Option<BacktestResult>,
    pub all_trials: Vec<ParameterTrial>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterValue {
    pub name: String,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterTrial {
    pub params: Vec<ParameterValue>,
    pub in_sample_sharpe: f64,
    pub out_of_sample_sharpe: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalkForwardConfig {
    pub in_sample_ratio: f64,
    pub min_window_candles: usize,
    pub risk_free_rate: f64,
}

impl Default for WalkForwardConfig {
    fn default() -> Self {
        Self {
            in_sample_ratio: 0.7,
            min_window_candles: 100,
            risk_free_rate: 0.02,
        }
    }
}

pub struct WalkForwardOptimizer {
    pool: SqlitePool,
    symbol: String,
    start_ts: i64,
    end_ts: i64,
    backtest_config: BacktestConfig,
    wf_config: WalkForwardConfig,
    parameter_grid: Vec<ParameterRange>,
}

impl WalkForwardOptimizer {
    pub fn new(
        pool: SqlitePool,
        symbol: String,
        start_ts: i64,
        end_ts: i64,
        parameter_grid: Vec<ParameterRange>,
    ) -> Self {
        Self {
            pool,
            symbol,
            start_ts,
            end_ts,
            backtest_config: BacktestConfig::default(),
            wf_config: WalkForwardConfig::default(),
            parameter_grid,
        }
    }

    pub fn with_backtest_config(mut self, config: BacktestConfig) -> Self {
        self.backtest_config = config;
        self
    }

    pub fn with_walk_forward_config(mut self, config: WalkForwardConfig) -> Self {
        self.wf_config = config;
        self
    }

    fn generate_combinations(&self) -> Vec<Vec<ParameterValue>> {
        generate_param_combinations(&self.parameter_grid)
    }

    pub async fn optimize(&mut self) -> Result<WalkForwardResult, String> {
        let total_duration = self.end_ts - self.start_ts;
        let split_ts = self.start_ts
            + (total_duration as f64 * self.wf_config.in_sample_ratio) as i64;

        let combinations = self.generate_combinations();

        if combinations.is_empty() {
            return Err("No parameter combinations to test".to_string());
        }

        let mut best_result: Option<WalkForwardResult> = None;
        let mut best_oos_sharpe = f64::NEG_INFINITY;
        let mut all_trials = Vec::new();

        for combo in &combinations {
            let quorum_threshold = combo
                .iter()
                .find(|p| p.name == "quorum_threshold")
                .map(|p| p.value)
                .unwrap_or(60.0);

            let (is_sharpe, oos_sharpe) = self
                .run_split_trial(split_ts, quorum_threshold)
                .await?;

            all_trials.push(ParameterTrial {
                params: combo.clone(),
                in_sample_sharpe: is_sharpe,
                out_of_sample_sharpe: oos_sharpe,
            });

            if oos_sharpe > best_oos_sharpe {
                best_oos_sharpe = oos_sharpe;

                let mut engine = BacktestEngine::new(
                    self.pool.clone(),
                    self.symbol.clone(),
                    split_ts,
                    self.end_ts,
                    self.backtest_config.clone(),
                );
                let oos_run = engine.run().await.ok();

                best_result = Some(WalkForwardResult {
                    best_params: combo.clone(),
                    oos_sharpe,
                    oos_metrics: oos_run,
                    all_trials: all_trials.clone(),
                });
            }
        }

        best_result.ok_or("No valid trials completed".to_string())
    }

    async fn run_split_trial(
        &mut self,
        split_ts: i64,
        quorum_threshold: f64,
    ) -> Result<(f64, f64), String> {
        let mut is_engine = BacktestEngine::new(
            self.pool.clone(),
            self.symbol.clone(),
            self.start_ts,
            split_ts,
            BacktestConfig {
                quorum_threshold,
                ..self.backtest_config.clone()
            },
        );

        let mut oos_engine = BacktestEngine::new(
            self.pool.clone(),
            self.symbol.clone(),
            split_ts,
            self.end_ts,
            BacktestConfig {
                quorum_threshold,
                ..self.backtest_config.clone()
            },
        );

        let is_result = is_engine.run().await?;
        let oos_result = oos_engine.run().await?;

        let is_trade_returns: Vec<f64> = is_result
            .trades
            .iter()
            .map(|t| t.pnl_pct / 100.0)
            .collect();

        let oos_trade_returns: Vec<f64> = oos_result
            .trades
            .iter()
            .map(|t| t.pnl_pct / 100.0)
            .collect();

        let is_sharpe = compute_sharpe(&is_trade_returns, self.wf_config.risk_free_rate);
        let oos_sharpe = compute_sharpe(&oos_trade_returns, self.wf_config.risk_free_rate);

        Ok((is_sharpe, oos_sharpe))
    }
}

pub fn generate_param_combinations(grid: &[ParameterRange]) -> Vec<Vec<ParameterValue>> {
    if grid.is_empty() {
        return vec![vec![]];
    }

    let mut combinations: Vec<Vec<ParameterValue>> = vec![vec![]];

    for param in grid {
        let mut next = Vec::new();
        for combo in &combinations {
            for value in &param.values {
                let mut new_combo = combo.clone();
                new_combo.push(ParameterValue {
                    name: param.name.clone(),
                    value: *value,
                });
                next.push(new_combo);
            }
        }
        combinations = next;
    }

    combinations
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_combinations_single_param() {
        let grid = vec![ParameterRange {
            name: "quorum_threshold".to_string(),
            values: vec![50.0, 60.0, 70.0],
        }];
        let combos = generate_param_combinations(&grid);
        assert_eq!(combos.len(), 3);
        assert_eq!(combos[0][0].value, 50.0);
        assert_eq!(combos[2][0].value, 70.0);
    }

    #[test]
    fn test_generate_combinations_empty() {
        let combos = generate_param_combinations(&[]);
        assert_eq!(combos.len(), 1);
        assert!(combos[0].is_empty());
    }

    #[test]
    fn test_default_wf_config() {
        let cfg = WalkForwardConfig::default();
        assert_eq!(cfg.in_sample_ratio, 0.7);
        assert_eq!(cfg.min_window_candles, 100);
    }
}
