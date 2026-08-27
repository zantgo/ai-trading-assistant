//! # Strategy intake gates (v9)
//!
//! The strategy's L7 market filter (`breadth_entry_floor`) and the PME
//! veto wiring (`pme.enforce_systemic_veto`, `enforce_margin_close_only`,
//! exposure caps) are enforced as **TAE intake gates** — PME stays
//! informational, the daemon applies the strategy's flags before any
//! entry. Veto OFF by default (the `default` strategy carries all flags
//! off), configurable per strategy.
//!
//! Exposure + margin gates need live portfolio state (engine exposure,
//! margin usage) and are evaluated by the caller with those inputs; the
//! breadth + systemic gates are pure functions of the strategy and the
//! L7 Overview Matrix.

use config_models::StrategyConfig;

/// Evaluate the breadth-floor + systemic-veto gates (pure of portfolio
/// state). Returns `(allows_entry, block_reason)`.
pub fn evaluate_intake_gates(
    strategy: &StrategyConfig,
    breadth_pct: f64,
    systemic_risk: f64,
) -> (bool, Option<String>) {
    if let Some(floor) = strategy.l7.breadth_entry_floor {
        if breadth_pct < floor {
            return (
                false,
                Some(format!(
                    "MARKET FILTER BLOCKED — breadth {:.0}% below the strategy floor ({:.0}%)",
                    breadth_pct, floor
                )),
            );
        }
    }
    if strategy.pme.enforce_systemic_veto
        && systemic_risk >= strategy.l7.systemic.entry_veto_threshold
    {
        return (
            false,
            Some(format!(
                "BLOCKED — systemic risk {:.0} ≥ strategy threshold {:.0}",
                systemic_risk, strategy.l7.systemic.entry_veto_threshold
            )),
        );
    }
    (true, None)
}

/// Evaluate the portfolio-state gates: exposure caps (when enforced) and
/// the margin close-only band (when enforced). `current_single_pair_pct`
/// is the prospective single-pair exposure AFTER the entry;
/// `current_portfolio_pct` likewise for the gross portfolio;
/// `margin_usage_ratio` is the capital-layer ratio.
#[allow(clippy::too_many_arguments)]
pub fn evaluate_portfolio_gates(
    strategy: &StrategyConfig,
    prospective_single_pair_pct: f64,
    prospective_portfolio_pct: f64,
    margin_usage_ratio: f64,
) -> (bool, Option<String>) {
    let ex = &strategy.pme.exposure;
    if ex.enforce.single_pair && prospective_single_pair_pct > ex.max_single_pair_exposure_pct {
        return (
            false,
            Some(format!(
                "BLOCKED — single-pair exposure limit ({:.0}%)",
                ex.max_single_pair_exposure_pct
            )),
        );
    }
    if ex.enforce.portfolio && prospective_portfolio_pct > ex.max_portfolio_exposure_pct {
        return (
            false,
            Some(format!(
                "BLOCKED — portfolio exposure limit ({:.0}%)",
                ex.max_portfolio_exposure_pct
            )),
        );
    }
    if strategy.pme.capital.enforce_margin_close_only
        && margin_usage_ratio >= strategy.pme.capital.margin_alert_bands.close_only
    {
        return (false, Some("BLOCKED — margin close-only".into()));
    }
    (true, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_strategy_has_no_gates() {
        let s = StrategyConfig::default();
        let (allows, reason) = evaluate_intake_gates(&s, -90.0, 95.0);
        assert!(allows);
        assert!(reason.is_none());
        let (allows2, reason2) = evaluate_portfolio_gates(&s, 99.0, 99.0, 0.99);
        assert!(allows2);
        assert!(reason2.is_none());
    }

    #[test]
    fn breadth_floor_blocks_when_set() {
        let mut s = StrategyConfig::default();
        s.l7.breadth_entry_floor = Some(-20.0);
        let (allows, reason) = evaluate_intake_gates(&s, -45.0, 10.0);
        assert!(!allows);
        assert!(reason.unwrap().contains("MARKET FILTER BLOCKED"));
        let (allows2, _) = evaluate_intake_gates(&s, 30.0, 10.0);
        assert!(allows2);
    }

    #[test]
    fn systemic_veto_only_when_enforced() {
        let mut s = StrategyConfig::default();
        s.pme.enforce_systemic_veto = true;
        s.l7.systemic.entry_veto_threshold = 60.0;
        let (allows, reason) = evaluate_intake_gates(&s, 50.0, 75.0);
        assert!(!allows);
        assert!(reason.unwrap().contains("systemic risk"));
        // With enforcement off the same environment passes.
        let mut s2 = StrategyConfig::default();
        s2.pme.enforce_systemic_veto = false;
        let (allows2, _) = evaluate_intake_gates(&s2, 50.0, 75.0);
        assert!(allows2);
    }

    #[test]
    fn exposure_and_margin_gates_respect_flags() {
        let mut s = StrategyConfig::default();
        s.pme.exposure.enforce.single_pair = true;
        let (allows, reason) = evaluate_portfolio_gates(&s, 25.0, 10.0, 0.1);
        assert!(!allows);
        assert!(reason.unwrap().contains("single-pair"));

        let mut s2 = StrategyConfig::default();
        s2.pme.capital.enforce_margin_close_only = true;
        let (allows2, reason2) = evaluate_portfolio_gates(&s2, 5.0, 10.0, 0.96);
        assert!(!allows2);
        assert!(reason2.unwrap().contains("margin close-only"));
    }
}
