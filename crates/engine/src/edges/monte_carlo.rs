use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;

use crate::edges::types::{DrawdownBucket, MonteCarloPath, MonteCarloResult};

pub struct MonteCarloConfig {
    pub num_paths: usize,
    pub trades_per_path: usize,
    pub seed: Option<u64>,
}

impl Default for MonteCarloConfig {
    fn default() -> Self {
        Self {
            num_paths: 1000,
            trades_per_path: 100,
            seed: Some(42),
        }
    }
}

pub fn run_monte_carlo(
    trade_returns: &[f64],
    ruin_threshold_pct: f64,
    cfg: &MonteCarloConfig,
) -> MonteCarloResult {
    if trade_returns.is_empty() {
        return MonteCarloResult {
            paths: Vec::new(),
            avg_final_return_pct: 0.0,
            median_max_drawdown_pct: 0.0,
            worst_case_drawdown_pct: 0.0,
            drawdown_distribution: Vec::new(),
            probability_of_ruin_pct: 0.0,
            confidence_95_drawdown_pct: 0.0,
        };
    }

    let mut rng = if let Some(seed) = cfg.seed {
        StdRng::seed_from_u64(seed)
    } else {
        StdRng::from_entropy()
    };

    let mut paths: Vec<MonteCarloPath> = Vec::with_capacity(cfg.num_paths);
    let mut all_drawdowns: Vec<f64> = Vec::with_capacity(cfg.num_paths);
    let mut all_final_returns: Vec<f64> = Vec::with_capacity(cfg.num_paths);
    let mut ruin_count = 0_usize;

    for path_idx in 0..cfg.num_paths {
        let mut equity = 0.0_f64;
        let mut peak = 0.0_f64;
        let mut max_dd_pct = 0.0_f64;
        let mut equity_points: Vec<f64> = Vec::with_capacity(cfg.trades_per_path + 1);
        equity_points.push(0.0);
        let mut hit_ruin = false;

        for _ in 0..cfg.trades_per_path {
            let r = *trade_returns.choose(&mut rng).unwrap_or(&0.0);
            equity += r;

            if equity > peak {
                peak = equity;
            }

            let dd = peak - equity;
            if dd > max_dd_pct {
                max_dd_pct = dd;
            }

            equity_points.push(equity);

            if max_dd_pct.abs() >= ruin_threshold_pct.abs() && !hit_ruin {
                hit_ruin = true;
            }
        }

        if hit_ruin {
            ruin_count += 1;
        }

        all_drawdowns.push(max_dd_pct);
        all_final_returns.push(equity);

        paths.push(MonteCarloPath {
            path_index: path_idx,
            equity_points,
            max_drawdown_pct: max_dd_pct,
            final_return_pct: equity,
        });
    }

    all_drawdowns.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let avg_final = all_final_returns.iter().sum::<f64>() / cfg.num_paths as f64;
    let median_dd = all_drawdowns[all_drawdowns.len() / 2];
    let worst_dd = all_drawdowns[all_drawdowns.len() - 1];
    let dd_95_idx = ((0.95 * cfg.num_paths as f64) as usize).min(all_drawdowns.len() - 1);
    let confidence_95_dd = all_drawdowns[dd_95_idx];
    let ruin_pct = (ruin_count as f64 / cfg.num_paths as f64) * 100.0;

    let drawdown_distribution = build_drawdown_distribution(&all_drawdowns, 20);

    MonteCarloResult {
        paths,
        avg_final_return_pct: avg_final,
        median_max_drawdown_pct: median_dd,
        worst_case_drawdown_pct: worst_dd,
        drawdown_distribution,
        probability_of_ruin_pct: ruin_pct,
        confidence_95_drawdown_pct: confidence_95_dd,
    }
}

fn build_drawdown_distribution(drawdowns: &[f64], num_buckets: usize) -> Vec<DrawdownBucket> {
    if drawdowns.is_empty() {
        return Vec::new();
    }

    let min_dd = drawdowns.first().copied().unwrap_or(0.0);
    let max_dd = drawdowns.last().copied().unwrap_or(0.0);

    if (max_dd - min_dd).abs() < 1e-9 {
        return vec![DrawdownBucket {
            bucket_pct: max_dd,
            frequency: drawdowns.len(),
        }];
    }

    let bucket_width = (max_dd - min_dd) / num_buckets as f64;

    let mut buckets: Vec<DrawdownBucket> = Vec::with_capacity(num_buckets);

    for i in 0..num_buckets {
        let lower = min_dd + bucket_width * i as f64;
        let upper = if i == num_buckets - 1 {
            max_dd + 0.001
        } else {
            lower + bucket_width
        };

        let count = drawdowns
            .iter()
            .filter(|&&dd| dd >= lower && dd < upper)
            .count();

        let mid = (lower + upper) / 2.0;

        buckets.push(DrawdownBucket {
            bucket_pct: mid,
            frequency: count,
        });
    }

    buckets
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monte_carlo_empty() {
        let result = run_monte_carlo(&[], 10.0, &MonteCarloConfig::default());
        assert!(result.paths.is_empty());
        assert_eq!(result.probability_of_ruin_pct, 0.0);
    }

    #[test]
    fn test_monte_carlo_basic() {
        let returns = vec![2.0, -1.0, 3.0, -0.5, 1.0, -2.0, 0.5, 4.0, -1.5, 0.0];
        let cfg = MonteCarloConfig {
            num_paths: 500,
            trades_per_path: 50,
            seed: Some(42),
        };
        let result = run_monte_carlo(&returns, 15.0, &cfg);
        assert_eq!(result.paths.len(), 500);
        assert!(result.median_max_drawdown_pct >= 0.0);
        assert!(result.drawdown_distribution.len() > 0);
    }

    #[test]
    fn test_monte_carlo_reproducible() {
        let returns = vec![1.0, -0.5, 2.0, -1.0, 0.5];
        let cfg = MonteCarloConfig {
            num_paths: 100,
            trades_per_path: 20,
            seed: Some(42),
        };
        let result1 = run_monte_carlo(&returns, 10.0, &cfg);
        let result2 = run_monte_carlo(&returns, 10.0, &cfg);
        assert_eq!(result1.avg_final_return_pct, result2.avg_final_return_pct);
        assert_eq!(result1.median_max_drawdown_pct, result2.median_max_drawdown_pct);
    }

    #[test]
    fn test_drawdown_distribution_empty() {
        let dist = build_drawdown_distribution(&[], 5);
        assert!(dist.is_empty());
    }

    #[test]
    fn test_drawdown_distribution_single() {
        let dist = build_drawdown_distribution(&[5.0, 5.0, 5.0], 5);
        assert!(!dist.is_empty());
        let total: usize = dist.iter().map(|b| b.frequency).sum();
        assert_eq!(total, 3);
    }
}
