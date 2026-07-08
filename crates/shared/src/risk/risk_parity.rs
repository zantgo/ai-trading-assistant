//! Risk Parity allocation — pure mathematics (Portfolio Optimization).
//!
//! Instead of allocating capital equally, risk-parity schemes allocate so that
//! each asset contributes a comparable amount of portfolio risk. With no
//! correlation assumptions, the inverse-volatility weighting gives every asset
//! (approximately) equal risk contribution; risk budgeting generalizes this by
//! assigning each asset a target share of total risk.

/// Equal Risk Contribution weights: `w_i = (1/sigma_i) / sum(1/sigma_j)`.
pub fn equal_risk_contribution(volatilities: &[f64]) -> Vec<f64> {
    let inv_vols: Vec<f64> = volatilities.iter().map(|&v| 1.0 / v.max(1e-10)).collect();
    let total: f64 = inv_vols.iter().sum();
    if total <= 0.0 {
        return vec![0.0; volatilities.len()];
    }
    inv_vols.iter().map(|&v| v / total).collect()
}

/// Risk-budgeted weights: `w_i = budget_i / sigma_i`, normalized.
pub fn risk_budgeted(volatilities: &[f64], budgets: &[f64]) -> Vec<f64> {
    let raw: Vec<f64> = volatilities
        .iter()
        .zip(budgets.iter())
        .map(|(&v, &b)| b / v.max(1e-10))
        .collect();
    let total: f64 = raw.iter().sum();
    if total <= 0.0 {
        return vec![0.0; raw.len()];
    }
    raw.iter().map(|&v| v / total).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn erc_weights_sum_to_one() {
        let w = equal_risk_contribution(&[0.1, 0.2, 0.4]);
        let sum: f64 = w.iter().sum();
        assert!(approx(sum, 1.0));
    }

    #[test]
    fn erc_favors_lower_volatility() {
        // Lower vol -> higher weight.
        let w = equal_risk_contribution(&[0.1, 0.2]);
        assert!(w[0] > w[1]);
        // 1/0.1=10, 1/0.2=5, total 15 -> [2/3, 1/3].
        assert!(approx(w[0], 2.0 / 3.0));
        assert!(approx(w[1], 1.0 / 3.0));
    }

    #[test]
    fn erc_equal_vols_equal_weights() {
        let w = equal_risk_contribution(&[0.2, 0.2, 0.2, 0.2]);
        for x in &w {
            assert!(approx(*x, 0.25));
        }
    }

    #[test]
    fn erc_guards_zero_volatility() {
        // Zero vol floored at 1e-10 -> finite, no NaN/inf.
        let w = equal_risk_contribution(&[0.0, 0.2]);
        assert!(w.iter().all(|x| x.is_finite()));
        let sum: f64 = w.iter().sum();
        assert!(approx(sum, 1.0));
    }

    #[test]
    fn erc_empty_is_empty() {
        assert!(equal_risk_contribution(&[]).is_empty());
    }

    #[test]
    fn budgeted_weights_sum_to_one() {
        let w = risk_budgeted(&[0.1, 0.2, 0.4], &[0.5, 0.3, 0.2]);
        let sum: f64 = w.iter().sum();
        assert!(approx(sum, 1.0));
    }

    #[test]
    fn budgeted_equal_budgets_matches_erc() {
        let vols = [0.1, 0.2, 0.4];
        let budgeted = risk_budgeted(&vols, &[1.0, 1.0, 1.0]);
        let erc = equal_risk_contribution(&vols);
        for (a, b) in budgeted.iter().zip(erc.iter()) {
            assert!(approx(*a, *b));
        }
    }

    #[test]
    fn budgeted_higher_budget_higher_weight() {
        // Same vol, higher budget -> higher weight.
        let w = risk_budgeted(&[0.2, 0.2], &[0.75, 0.25]);
        assert!(w[0] > w[1]);
        assert!(approx(w[0], 0.75));
        assert!(approx(w[1], 0.25));
    }
}
