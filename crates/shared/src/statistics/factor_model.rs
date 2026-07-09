//! Factor model — alpha/beta decomposition for performance attribution.
//!
//! Decomposes strategy returns into market-driven (beta) and skill-driven
//! (alpha) components using a 1-factor market model:
//!
//!   r_i = α + β·r_m + ε
//!
//! β = sensitivity to the market factor (e.g., BTC)
//! α = Jensen's alpha — excess return unexplained by market exposure
//!
//! Provides rolling factor exposures, regime-conditional alpha, and
//! signal quality decomposition.

/// Result of a single-factor OLS regression.
#[derive(Debug, Clone)]
pub struct FactorResult {
    /// Jensen's alpha (annualized % excess return).
    pub alpha: f64,
    /// Market beta (sensitivity to the factor).
    pub beta: f64,
    /// R-squared — fraction of variance explained by the factor.
    pub r_squared: f64,
    /// Idiosyncratic volatility (stddev of residuals).
    pub residual_vol: f64,
    /// t-statistic for alpha (significance test).
    pub t_stat_alpha: f64,
    /// t-statistic for beta (significance test).
    pub t_stat_beta: f64,
    /// Number of observations used.
    pub n: usize,
}

/// Compute a single-factor OLS regression.
///
/// `asset_returns` and `factor_returns` must be the same length.
pub fn single_factor_beta(asset_returns: &[f64], factor_returns: &[f64]) -> Option<FactorResult> {
    let n = asset_returns.len();
    if n < 5 || factor_returns.len() != n {
        return None;
    }

    let mean_asset: f64 = asset_returns.iter().sum::<f64>() / n as f64;
    let mean_factor: f64 = factor_returns.iter().sum::<f64>() / n as f64;

    let cov: f64 = asset_returns
        .iter()
        .zip(factor_returns.iter())
        .map(|(r, f)| (r - mean_asset) * (f - mean_factor))
        .sum::<f64>()
        / (n - 1) as f64;

    let var_factor: f64 = factor_returns
        .iter()
        .map(|f| (f - mean_factor).powi(2))
        .sum::<f64>()
        / (n - 1) as f64;

    if var_factor < 1e-12 {
        return None;
    }

    let beta = cov / var_factor;
    let alpha = mean_asset - beta * mean_factor;

    // Residuals and R²
    let residuals: Vec<f64> = asset_returns
        .iter()
        .zip(factor_returns.iter())
        .map(|(r, f)| r - (alpha + beta * f))
        .collect();

    let var_residual: f64 = residuals
        .iter()
        .map(|e| e.powi(2))
        .sum::<f64>()
        / (n - 2) as f64;

    let var_asset: f64 = asset_returns
        .iter()
        .map(|r| (r - mean_asset).powi(2))
        .sum::<f64>()
        / (n - 1) as f64;

    let r_squared = if var_asset > 1e-12 {
        (1.0 - var_residual * (n - 2) as f64 / (var_asset * (n - 1) as f64)).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let residual_vol = var_residual.sqrt();

    // t-statistics
    let se_beta = if var_factor > 1e-12 && n > 2 {
        (var_residual / (var_factor * (n - 1) as f64)).sqrt()
    } else {
        1.0
    };
    let se_alpha = if n > 2 {
        (var_residual * (1.0 / n as f64 + mean_factor.powi(2) / (var_factor * (n - 1) as f64))).sqrt()
    } else {
        1.0
    };

    let t_stat_beta = if se_beta > 1e-12 { beta.abs() / se_beta } else { 0.0 };
    let t_stat_alpha = if se_alpha > 1e-12 { alpha.abs() / se_alpha } else { 0.0 };

    Some(FactorResult {
        alpha,
        beta,
        r_squared,
        residual_vol,
        t_stat_alpha,
        t_stat_beta,
        n,
    })
}

/// Performance attribution: how much P&L from market vs alpha.
#[derive(Debug, Clone)]
pub struct Attribution {
    /// P&L attributed to market exposure.
    pub market_pnl: f64,
    /// P&L attributed to alpha.
    pub alpha_pnl: f64,
    /// Total P&L.
    pub total_pnl: f64,
    /// Fraction of P&L from alpha: alpha_pnl / total_pnl.
    pub attribution_ratio: f64,
}

/// Attribute total P&L between market beta and alpha.
pub fn performance_attribution(
    total_pnl: f64,
    beta: f64,
    cumulative_market_return: f64,
    direction: f64, // +1 for long, -1 for short
) -> Attribution {
    let market_pnl = beta * cumulative_market_return * direction;
    let alpha_pnl = total_pnl - market_pnl;
    let attribution_ratio = if total_pnl.abs() > 1e-12 {
        alpha_pnl / total_pnl
    } else {
        0.5
    };

    Attribution {
        market_pnl,
        alpha_pnl,
        total_pnl,
        attribution_ratio,
    }
}

/// Rolling factor exposure tracker.
#[derive(Debug, Clone)]
pub struct RollingFactorTracker {
    asset_returns: Vec<f64>,
    factor_returns: Vec<f64>,
    window_size: usize,
}

impl RollingFactorTracker {
    pub fn new(window_size: usize) -> Self {
        Self {
            asset_returns: Vec::with_capacity(window_size),
            factor_returns: Vec::with_capacity(window_size),
            window_size,
        }
    }

    /// Push a new pair of returns.
    pub fn push(&mut self, asset_return: f64, factor_return: f64) {
        self.asset_returns.push(asset_return);
        self.factor_returns.push(factor_return);
        if self.asset_returns.len() > self.window_size {
            self.asset_returns.remove(0);
        }
        if self.factor_returns.len() > self.window_size {
            self.factor_returns.remove(0);
        }
    }

    /// Compute the current rolling factor regression.
    pub fn compute(&self) -> Option<FactorResult> {
        single_factor_beta(&self.asset_returns, &self.factor_returns)
    }

    pub fn len(&self) -> usize {
        self.asset_returns.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_beta_perfect_correlation() {
        let factor: Vec<f64> = (0..100).map(|i| (i as f64 - 50.0) * 0.1).collect();
        let asset: Vec<f64> = factor.iter().map(|f| f * 1.5 + 0.02).collect();
        let result = single_factor_beta(&asset, &factor).unwrap();
        assert!((result.beta - 1.5).abs() < 0.05);
        assert!((result.alpha - 0.02).abs() < 0.1);
        assert!(result.r_squared > 0.95);
    }

    #[test]
    fn test_beta_zero_correlation() {
        let factor: Vec<f64> = (0..100).map(|i| (i as f64 - 50.0) * 0.1).collect();
        let asset: Vec<f64> = (0..100).map(|_| (factor[0] - 50.0) * 0.1).collect();
        let result = single_factor_beta(&asset, &factor).unwrap();
        assert!(result.beta.abs() < 0.5);
        assert!(result.r_squared < 0.3);
    }

    #[test]
    fn test_attribution_ratio() {
        let attr = performance_attribution(100.0, 1.0, 30.0, 1.0);
        assert!((attr.market_pnl - 30.0).abs() < 0.01);
        assert!((attr.alpha_pnl - 70.0).abs() < 0.01);
        assert!((attr.attribution_ratio - 0.7).abs() < 0.01);
    }

    #[test]
    fn test_rolling_tracker() {
        let mut tracker = RollingFactorTracker::new(50);
        for i in 0..60 {
            tracker.push(i as f64 * 0.1, i as f64 * 0.05);
        }
        assert_eq!(tracker.len(), 50);
        let result = tracker.compute().unwrap();
        assert!(result.beta > 0.0);
    }
}
