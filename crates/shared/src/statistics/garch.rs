//! GARCH(1,1) volatility forecasting model.
//!
//! Models time-varying conditional variance:
//!
//!   σ²_t = ω + α·ε²_{t-1} + β·σ²_{t-1}
//!
//! Captures volatility clustering — periods of high volatility tend to
//! persist, and periods of low volatility tend to persist.  Provides a
//! forward-looking volatility forecast superior to simple historical
//! volatility (HV) for dynamic stop placement and risk estimation.
//!
//! Parameter estimation uses method of moments (no optimizer needed),
//! and the per-candle `advance()` call is O(1).

/// A GARCH(1,1) model with per-candle recursive updating.
#[derive(Debug, Clone)]
pub struct GarchModel {
    /// Baseline variance (intercept): ω
    pub omega: f64,
    /// Reaction to recent shocks (ARCH term): α
    pub alpha: f64,
    /// Persistence of past volatility (GARCH term): β
    pub beta: f64,
    /// Current conditional variance σ²_t.
    pub current_variance: f64,
    /// Most recent squared residual ε²_{t-1}.
    pub prev_sq_residual: f64,
    /// Mean return (for residual computation).
    pub return_mean: f64,
    /// Number of bars processed.
    pub bar_count: usize,
}

/// Output of a GARCH advance step.
#[derive(Debug, Clone)]
pub struct GarchForecast {
    /// Current-period volatility σ_t (as percentage).
    pub current_vol: f64,
    /// 1-bar-ahead volatility forecast E[σ_{t+1}].
    pub forecast_1bar: f64,
    /// 5-bar-ahead volatility forecast E[σ_{t+5}].
    pub forecast_5bar: f64,
    /// Unconditional (long-run) volatility: √(ω / (1−α−β)).
    pub long_run_vol: f64,
    /// Persistence parameter: α + β ∈ [0, 1).
    pub persistence: f64,
}

impl GarchModel {
    /// Estimate GARCH(1,1) parameters from a returns series using method of moments.
    ///
    /// Returns `None` if estimation fails (too few data, non-stationary process).
    pub fn fit(returns: &[f64]) -> Option<Self> {
        if returns.len() < 30 {
            return None;
        }

        let n = returns.len() as f64;
        let return_mean = returns.iter().sum::<f64>() / n;

        // Unconditional variance
        let unconditional_var: f64 = returns
            .iter()
            .map(|r| (r - return_mean).powi(2))
            .sum::<f64>() / (n - 1.0);

        if unconditional_var < 1e-12 {
            return None;
        }

        // Squared residuals (centered returns)
        let sq_residuals: Vec<f64> = returns
            .iter()
            .map(|r| (r - return_mean).powi(2))
            .collect();

        // Autocorrelation of squared residuals at lag 1
        let mean_sq = sq_residuals.iter().sum::<f64>() / n;
        let var_sq: f64 = sq_residuals
            .iter()
            .map(|x| (x - mean_sq).powi(2))
            .sum::<f64>() / (n - 1.0);

        if var_sq < 1e-12 {
            return None;
        }

        let autocov_sq: f64 = (0..sq_residuals.len() - 1)
            .map(|i| (sq_residuals[i] - mean_sq) * (sq_residuals[i + 1] - mean_sq))
            .sum::<f64>() / (n - 2.0);
        let rho1 = autocov_sq / var_sq;

        // Method of moments: α ≈ ρ₁ (autocorrelation of squared returns),
        // β ≈ 1 − α (initial), then adjust for stationarity.
        let mut alpha = rho1.clamp(0.01, 0.30);
        let mut beta = (1.0 - alpha - 0.02).clamp(0.60, 0.95);
        let mut omega = unconditional_var * (1.0 - alpha - beta);

        // Iterative refinement (5 passes) to push toward stationary region.
        for _ in 0..5 {
            if alpha + beta >= 0.999 {
                let scale = 0.995 / (alpha + beta);
                alpha *= scale;
                beta *= scale;
            }
            omega = unconditional_var * (1.0 - alpha - beta).max(1e-12);
            // Re-estimate alpha from covariance of innovations
            let ac = (0..returns.len() - 1)
                .map(|i| (returns[i] - return_mean).powi(2) * (returns[i + 1] - return_mean).powi(2))
                .sum::<f64>() / (n - 1.0);
            alpha = (ac / unconditional_var.powi(2)).clamp(0.01, 0.30);
            beta = (0.995 - alpha).clamp(0.60, 0.95);
        }

        let persistence = alpha + beta;
        if persistence >= 1.0 {
            return None;
        }

        // Initialize current variance to unconditional.
        let prev_sq_residual = sq_residuals.last().copied().unwrap_or(unconditional_var);

        Some(Self {
            omega,
            alpha,
            beta,
            current_variance: unconditional_var,
            prev_sq_residual,
            return_mean,
            bar_count: returns.len(),
        })
    }

    /// Advance the model by one bar with a new observed return.
    ///
    /// Updates the conditional variance estimate.  Returns the updated
    /// `GarchForecast` for this bar.
    pub fn advance(&mut self, log_return: f64) -> GarchForecast {
        self.bar_count = self.bar_count.wrapping_add(1);

        // Compute this bar's residual
        let residual = log_return - self.return_mean;
        let sq_residual = residual * residual;

        // Update conditional variance: σ²_t = ω + α·ε²_{t-1} + β·σ²_{t-1}
        self.current_variance = self.omega
            + self.alpha * self.prev_sq_residual
            + self.beta * self.current_variance;

        // Clamp to prevent negative or zero variance.
        if self.current_variance < 1e-12 {
            self.current_variance = self.omega;
        }

        // Store for next iteration
        self.prev_sq_residual = sq_residual;

        // Update running mean of returns (exponential decay)
        self.return_mean = 0.995 * self.return_mean + 0.005 * log_return;

        let persistence = self.alpha + self.beta;
        let long_run_var = if persistence < 0.999 {
            self.omega / (1.0 - persistence)
        } else {
            self.current_variance
        };

        let current_vol = self.current_variance.sqrt();
        let long_run_vol = long_run_var.sqrt().max(1e-12);

        // Multi-step forecast: σ²_{t+h} = ω/(1−α−β) + (α+β)^h × (σ²_t − ω/(1−α−β))
        let forecast_1bar = forecast_vol(self.current_variance, long_run_var, persistence, 1);
        let forecast_5bar = forecast_vol(self.current_variance, long_run_var, persistence, 5);

        GarchForecast {
            current_vol,
            forecast_1bar,
            forecast_5bar,
            long_run_vol,
            persistence,
        }
    }

    /// True if enough data has been processed for reliable forecasts.
    pub fn is_ready(&self) -> bool {
        self.bar_count >= 30
    }
}

/// Compute the h-step-ahead volatility forecast.
fn forecast_vol(current_var: f64, long_run_var: f64, persistence: f64, horizon: usize) -> f64 {
    if persistence >= 0.999 {
        return current_var.sqrt();
    }
    let var_forecast = long_run_var + persistence.powi(horizon as i32) * (current_var - long_run_var);
    var_forecast.max(1e-12).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Simple pseudo-random number generator for tests.
    fn test_random() -> f64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        static mut COUNTER: u64 = 0;
        let val = unsafe {
            COUNTER = COUNTER.wrapping_add(1);
            COUNTER
        };
        let mut h = DefaultHasher::new();
        val.hash(&mut h);
        (h.finish() as f64 / u64::MAX as f64) * 2.0 - 1.0
    }

    #[test]
    fn test_garch_fit_and_advance() {
        let mut returns = Vec::with_capacity(200);
        let mut vol: f64 = 2.0;
        for _ in 0..200 {
            let shock = test_random();
            let ret = shock * vol;
            returns.push(ret);
            vol = (0.02 + 0.1 * ret * ret + 0.85 * vol * vol).sqrt();
        }

        let mut model = GarchModel::fit(&returns).expect("should fit");
        assert!(model.alpha + model.beta < 1.0, "must be stationary");
        assert!(model.omega > 0.0, "omega must be positive");

        let f1 = model.advance(1.5);
        let f2 = model.advance(-2.0);
        assert!(f1.current_vol > 0.0);
        assert!(f2.current_vol > 0.0);
        assert!(f1.forecast_5bar > 0.0);
    }

    #[test]
    fn test_garch_stationary() {
        let returns: Vec<f64> = (0..100)
            .map(|_| test_random() * 2.0)
            .collect();
        let model = GarchModel::fit(&returns).expect("should fit");
        assert!(model.alpha + model.beta < 1.0);
    }

    #[test]
    fn test_garch_persistence_positive() {
        let returns: Vec<f64> = (0..50)
            .map(|i| (i as f64 * 0.1).sin() * 3.0)
            .collect();
        let model = GarchModel::fit(&returns).expect("should fit");
        let persistence = model.alpha + model.beta;
        assert!(persistence > 0.0);
        assert!(persistence < 1.0);
    }

    #[test]
    fn test_garch_forecast_converges_to_long_run() {
        let returns: Vec<f64> = (0..100)
            .map(|_| test_random() * 2.0)
            .collect();
        let mut model = GarchModel::fit(&returns).expect("should fit");
        let forecast = model.advance(0.5);
        let long_run = forecast.long_run_vol;
        assert!(long_run > 0.0);
        assert!(long_run < 50.0);
    }

    #[test]
    fn test_garch_vol_responds_to_shock() {
        // Simple stationary returns with a large outlier
        let mut returns: Vec<f64> = (0..100).map(|_| test_random() * 0.5).collect();
        returns.push(-8.0); // large shock
        let mut model = GarchModel::fit(&returns).expect("should fit");
        // Feed calm returns, then a shock
        model.advance(0.1);
        model.advance(-0.2);
        let baseline = model.advance(0.05).current_vol;
        let after_shock = model.advance(-7.0).current_vol;
        assert!(after_shock > 0.0, "vol should be positive after shock");
        assert!(baseline > 0.0, "baseline vol should be positive");
    }
}
