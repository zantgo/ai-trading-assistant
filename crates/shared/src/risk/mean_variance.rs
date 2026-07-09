//! Markowitz Mean-Variance Optimization (MVO).
//!
//! Computes the efficient frontier, tangency portfolio, and minimum-variance
//! portfolio from a returns matrix.  Complements the existing Kelly Criterion
//! and Risk Parity allocation methods.
//!
//! For small portfolios (2-10 assets), the efficient frontier is computed
//! via the two-fund separation theorem without a quadratic programming solver:
//!
//!   - Any frontier portfolio is a convex combination of two frontier portfolios.
//!   - Grid search over α ∈ [0, 1] produces the full frontier.
//!
//! Tangency portfolio: max Sharpe ratio portfolio on the efficient frontier.

/// A point on the efficient frontier.
#[derive(Debug, Clone)]
pub struct FrontierPoint {
    /// Expected return (annualized % if using annualized inputs).
    pub expected_return: f64,
    /// Expected volatility (annualized %).
    pub expected_vol: f64,
    /// Sharpe ratio.
    pub sharpe_ratio: f64,
    /// Portfolio weights.
    pub weights: Vec<f64>,
}

/// Complete efficient frontier.
#[derive(Debug, Clone)]
pub struct EfficientFrontier {
    /// Frontier points (sorted by return, ascending).
    pub points: Vec<FrontierPoint>,
    /// Minimum-variance portfolio.
    pub min_variance: FrontierPoint,
    /// Tangency portfolio (max Sharpe ratio, None if risk-free rate makes all Sharpe ratios negative).
    pub tangency: Option<FrontierPoint>,
    /// Number of assets.
    pub n_assets: usize,
}

impl EfficientFrontier {
    /// Compute the efficient frontier from a returns matrix.
    ///
    /// `returns` is a Vec<Vec<f64>> where returns[i] = series of returns for asset i.
    /// `n_points` is the number of frontier points to compute.
    /// `risk_free_rate` is the annualized risk-free rate for Sharpe ratio.
    pub fn compute(
        returns: &[Vec<f64>],
        n_points: usize,
        risk_free_rate: f64,
    ) -> Option<Self> {
        let n_assets = returns.len();
        if n_assets < 2 {
            return None;
        }

        let n_periods = returns[0].len();
        if n_periods < 5 {
            return None;
        }

        // Verify all series have same length
        for r in returns.iter().skip(1) {
            if r.len() != n_periods {
                return None;
            }
        }

        // Compute mean returns and covariance matrix
        let mean_returns: Vec<f64> = returns
            .iter()
            .map(|r| r.iter().sum::<f64>() / n_periods as f64)
            .collect();

        let mut cov_matrix = vec![vec![0.0; n_assets]; n_assets];
        for i in 0..n_assets {
            for j in 0..n_assets {
                let cov: f64 = returns[i]
                    .iter()
                    .zip(returns[j].iter())
                    .map(|(ri, rj)| (ri - mean_returns[i]) * (rj - mean_returns[j]))
                    .sum::<f64>()
                    / (n_periods - 1) as f64;
                cov_matrix[i][j] = cov;
            }
        }

        // Compute inverse of covariance matrix via Gaussian elimination (for n ≤ 10).
        let inv_cov = invert_matrix(&cov_matrix)?;

        // Two-fund separation:
        // w_g = Σ⁻¹·1 / (1′Σ⁻¹·1)  (global minimum variance)
        // w_h = Σ⁻¹·μ / (1′Σ⁻¹·μ)  (maximum return direction)
        let ones: Vec<f64> = vec![1.0; n_assets];
        let sigma_inv_ones = mat_vec_mul(&inv_cov, &ones);
        let sigma_inv_mu = mat_vec_mul(&inv_cov, &mean_returns);

        let denom_g: f64 = ones.iter().zip(sigma_inv_ones.iter()).map(|(o, s)| o * s).sum();
        if denom_g.abs() < 1e-12 {
            return None;
        }

        let w_g: Vec<f64> = sigma_inv_ones.iter().map(|s| s / denom_g).collect();
        let denom_h: f64 = ones.iter().zip(sigma_inv_mu.iter()).map(|(o, s)| o * s).sum();
        let w_h: Vec<f64> = if denom_h.abs() > 1e-12 {
            sigma_inv_mu.iter().map(|s| s / denom_h).collect()
        } else {
            w_g.clone()
        };

        // Generate frontier points via convex combinations
        let mut points = Vec::with_capacity(n_points);
        let mut best_sharpe = f64::NEG_INFINITY;
        let mut tangency_idx = 0;

        for k in 0..n_points {
            let alpha = k as f64 / (n_points - 1) as f64;
            let weights: Vec<f64> = w_g
                .iter()
                .zip(w_h.iter())
                .map(|(g, h)| alpha * g + (1.0 - alpha) * h)
                .collect();

            let (exp_ret, exp_vol) = portfolio_metrics(&weights, &mean_returns, &cov_matrix);
            let sharpe = if exp_vol > 1e-12 {
                (exp_ret - risk_free_rate) / exp_vol
            } else {
                0.0
            };

            if sharpe > best_sharpe {
                best_sharpe = sharpe;
                tangency_idx = points.len();
            }

            points.push(FrontierPoint {
                expected_return: exp_ret,
                expected_vol: exp_vol,
                sharpe_ratio: sharpe,
                weights,
            });
        }

        let min_variance = FrontierPoint {
            expected_return: portfolio_return(&w_g, &mean_returns),
            expected_vol: portfolio_vol(&w_g, &cov_matrix),
            sharpe_ratio: points.first().map(|p| p.sharpe_ratio).unwrap_or(0.0),
            weights: w_g,
        };

        let tangency = if points.len() > tangency_idx {
            Some(points[tangency_idx].clone())
        } else {
            None
        };

        Some(EfficientFrontier {
            points,
            min_variance,
            tangency,
            n_assets,
        })
    }

    /// Get the efficient frontier as (volatility, return) pairs for charting.
    pub fn frontier_curve(&self) -> Vec<(f64, f64)> {
        self.points
            .iter()
            .map(|p| (p.expected_vol, p.expected_return))
            .collect()
    }
}

/// Compute portfolio expected return.
fn portfolio_return(weights: &[f64], mean_returns: &[f64]) -> f64 {
    weights.iter().zip(mean_returns.iter()).map(|(w, r)| w * r).sum()
}

/// Compute portfolio volatility.
fn portfolio_vol(weights: &[f64], cov_matrix: &[Vec<f64>]) -> f64 {
    let n = weights.len();
    let mut var = 0.0;
    for i in 0..n {
        for j in 0..n {
            var += weights[i] * weights[j] * cov_matrix[i][j];
        }
    }
    var.max(0.0).sqrt()
}

/// Compute portfolio expected return and volatility.
fn portfolio_metrics(weights: &[f64], mean_returns: &[f64], cov_matrix: &[Vec<f64>]) -> (f64, f64) {
    (portfolio_return(weights, mean_returns), portfolio_vol(weights, cov_matrix))
}

/// Matrix-vector multiplication.
fn mat_vec_mul(mat: &[Vec<f64>], vec: &[f64]) -> Vec<f64> {
    let n = mat.len();
    (0..n)
        .map(|i| mat[i].iter().zip(vec.iter()).map(|(a, b)| a * b).sum())
        .collect()
}

/// Matrix inversion via Gaussian elimination with partial pivoting (for n ≤ 10).
fn invert_matrix(mat: &[Vec<f64>]) -> Option<Vec<Vec<f64>>> {
    let n = mat.len();
    if n == 0 || mat[0].len() != n {
        return None;
    }

    // Augmented matrix [A | I]
    let mut aug: Vec<Vec<f64>> = Vec::with_capacity(n);
    for i in 0..n {
        let mut row = vec![0.0; 2 * n];
        for j in 0..n {
            row[j] = mat[i][j];
        }
        row[n + i] = 1.0;
        aug.push(row);
    }

    // Forward elimination
    for col in 0..n {
        // Find pivot
        let mut pivot_row = col;
        let mut pivot_val = aug[col][col].abs();
        for row in (col + 1)..n {
            let val = aug[row][col].abs();
            if val > pivot_val {
                pivot_val = val;
                pivot_row = row;
            }
        }
        if pivot_val < 1e-12 {
            return None; // singular matrix
        }
        aug.swap(col, pivot_row);

        // Normalize pivot row
        let pivot = aug[col][col];
        for j in 0..2 * n {
            aug[col][j] /= pivot;
        }

        // Eliminate other rows
        for row in 0..n {
            if row == col {
                continue;
            }
            let factor = aug[row][col];
            for j in 0..2 * n {
                aug[row][j] -= factor * aug[col][j];
            }
        }
    }

    // Extract inverse from augmented right half
    let inv: Vec<Vec<f64>> = aug.iter().map(|row| row[n..2 * n].to_vec()).collect();
    Some(inv)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_efficient_frontier_two_assets() {
        let returns: Vec<Vec<f64>> = vec![
            vec![0.01, -0.02, 0.03, 0.01, -0.01, 0.02, 0.01, -0.01, 0.02, 0.0,
                 0.03, -0.01, 0.01, 0.02, -0.01, 0.01, 0.02, 0.0, 0.01, -0.02],
            vec![-0.01, 0.02, -0.01, 0.03, 0.01, -0.02, 0.01, 0.03, -0.01, 0.01,
                 0.02, 0.01, -0.02, 0.01, 0.02, -0.01, 0.01, 0.02, -0.01, 0.03],
        ];
        let frontier = EfficientFrontier::compute(&returns, 20, 0.02).unwrap();
        assert_eq!(frontier.points.len(), 20);
        assert!(frontier.points.iter().all(|p| p.weights.iter().sum::<f64>() - 1.0 < 0.01));
        assert!(frontier.min_variance.expected_vol > 0.0);
    }

    #[test]
    fn test_tangency_exists() {
        let returns: Vec<Vec<f64>> = vec![
            (0..100).map(|i| 0.01 + 0.001 * (i as f64 % 5.0 - 2.5)).collect(),
            (0..100).map(|i| -0.005 + 0.002 * (i as f64 % 3.0 - 1.5)).collect(),
        ];
        let frontier = EfficientFrontier::compute(&returns, 10, 0.01).unwrap();
        assert!(frontier.tangency.is_some());
        let t = frontier.tangency.unwrap();
        assert!(t.sharpe_ratio > 0.0);
    }

    #[test]
    fn test_weights_sum_to_one() {
        let returns: Vec<Vec<f64>> = vec![
            (0..50).map(|i| (i as f64).sin() * 0.02).collect(),
            (0..50).map(|i| (i as f64).cos() * 0.01).collect(),
            (0..50).map(|i| (i as f64 * 0.5).sin() * 0.015).collect(),
        ];
        let frontier = EfficientFrontier::compute(&returns, 15, 0.02).unwrap();
        for point in &frontier.points {
            let sum: f64 = point.weights.iter().sum();
            assert!((sum - 1.0).abs() < 0.01, "weights should sum to 1.0, got {}", sum);
        }
    }

    #[test]
    fn test_single_asset_none() {
        let returns: Vec<Vec<f64>> = vec![vec![0.01, -0.02, 0.03]];
        assert!(EfficientFrontier::compute(&returns, 10, 0.02).is_none());
    }
}
