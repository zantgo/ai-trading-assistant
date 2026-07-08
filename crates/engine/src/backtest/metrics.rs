use std::collections::VecDeque;

pub fn compute_sharpe(returns: &[f64], risk_free: f64) -> f64 {
    if returns.is_empty() {
        return 0.0;
    }
    let n = returns.len() as f64;
    let mean: f64 = returns.iter().sum::<f64>() / n;
    let variance: f64 = returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / n;
    let std_dev = variance.sqrt();
    if std_dev < 1e-12 {
        return 0.0;
    }
    let excess = mean - risk_free / 252.0;
    excess / std_dev * (252.0_f64).sqrt()
}

pub fn compute_sortino(returns: &[f64], risk_free: f64) -> f64 {
    if returns.is_empty() {
        return 0.0;
    }
    let n = returns.len() as f64;
    let mean: f64 = returns.iter().sum::<f64>() / n;

    let downside: Vec<f64> = returns
        .iter()
        .filter(|&&r| r < risk_free / 252.0)
        .map(|r| (r - risk_free / 252.0).powi(2))
        .collect();

    if downside.is_empty() {
        return 0.0;
    }
    let downside_variance = downside.iter().sum::<f64>() / n;
    let downside_dev = downside_variance.sqrt();
    if downside_dev < 1e-12 {
        return 0.0;
    }
    let excess = mean - risk_free / 252.0;
    excess / downside_dev * (252.0_f64).sqrt()
}

pub fn compute_max_drawdown(equity: &[(i64, f64)]) -> f64 {
    if equity.is_empty() {
        return 0.0;
    }
    let mut peak = f64::NEG_INFINITY;
    let mut max_dd = 0.0_f64;

    for &(_ts, val) in equity {
        if val > peak {
            peak = val;
        }
        if peak > 0.0 {
            let dd = (peak - val) / peak;
            if dd > max_dd {
                max_dd = dd;
            }
        }
    }
    max_dd * 100.0
}

pub fn compute_cagr(initial: f64, final_val: f64, days: f64) -> f64 {
    if initial <= 0.0 || days <= 0.0 {
        return 0.0;
    }
    let years = days / 365.25;
    (final_val / initial).powf(1.0 / years) - 1.0
}

pub fn deflated_sharpe(sharpe: f64, n_trials: usize, variance: f64) -> f64 {
    if n_trials < 2 {
        return sharpe;
    }
    let euler_mascheroni = 0.5772156649;
    let ln_n = (n_trials as f64).ln();
    let expected_max = euler_mascheroni + ln_n;
    let deflation = expected_max * variance.sqrt();
    (sharpe - deflation).max(0.0)
}

pub fn rolling_sharpe_window(
    equity_curve: &[(i64, f64)],
    window_days: usize,
) -> Vec<(i64, f64)> {
    if equity_curve.len() < window_days + 1 || window_days < 2 {
        return Vec::new();
    }
    let mut result = Vec::new();
    let mut returns: VecDeque<f64> = VecDeque::with_capacity(window_days);

    for i in 1..equity_curve.len() {
        let prev_val = equity_curve[i - 1].1;
        let cur_val = equity_curve[i].1;
        if prev_val > 0.0 {
            returns.push_back((cur_val / prev_val) - 1.0);
        }
        if returns.len() >= window_days {
            let window: Vec<f64> = returns.iter().copied().collect();
            let sh = compute_sharpe(&window, 0.0);
            result.push((equity_curve[i].0, sh));
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sharpe_positive() {
        let returns = vec![0.01, 0.02, -0.005, 0.015, 0.007];
        let s = compute_sharpe(&returns, 0.02);
        assert!(s > 0.0);
    }

    #[test]
    fn test_sharpe_empty() {
        assert_eq!(compute_sharpe(&[], 0.02), 0.0);
    }

    #[test]
    fn test_sortino_positive() {
        let returns = vec![0.01, 0.02, -0.005, 0.015, 0.007, -0.01];
        let s = compute_sortino(&returns, 0.02);
        assert!(s.is_finite());
    }

    #[test]
    fn test_max_drawdown_basic() {
        let equity = vec![
            (1, 100.0),
            (2, 110.0),
            (3, 90.0),
            (4, 105.0),
            (5, 95.0),
        ];
        let dd = compute_max_drawdown(&equity);
        assert!((dd - 18.1818).abs() < 0.1, "got {}", dd);
    }

    #[test]
    fn test_max_drawdown_no_decline() {
        let equity = vec![
            (1, 100.0),
            (2, 110.0),
            (3, 120.0),
        ];
        assert_eq!(compute_max_drawdown(&equity), 0.0);
    }

    #[test]
    fn test_cagr_basic() {
        let c = compute_cagr(100.0, 200.0, 365.25);
        assert!((c - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_cagr_zero_days() {
        assert_eq!(compute_cagr(100.0, 200.0, 0.0), 0.0);
    }

    #[test]
    fn test_deflated_sharpe() {
        let s = deflated_sharpe(1.5, 100, 0.25);
        assert!(s >= 0.0);
        assert!(s < 1.5);
    }
}
