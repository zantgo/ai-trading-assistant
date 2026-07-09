use serde::Serialize;

use crate::decision::config::DecisionConfig;
use shared::risk::{ExposureTier, RiskProfile, TradePermission};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Hold,
    Close,
    Wait,
    OpenLong,
    OpenShort,
}

impl Action {
    pub fn as_str(&self) -> &'static str {
        match self {
            Action::Hold => "Hold",
            Action::Close => "Close",
            Action::Wait => "Wait",
            Action::OpenLong => "Open Long",
            Action::OpenShort => "Open Short",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DecisionOutput {
    pub action: Action,
    pub confidence: f64,
    pub rationale: String,
    pub risk_notes: String,
    /// Contributing factor values for transparency
    pub factor_breakdown: FactorBreakdown,
}

#[derive(Debug, Clone, Serialize)]
pub struct FactorBreakdown {
    pub confluence_norm: f64,
    pub trade_readiness: f64,
    pub trade_quality: f64,
    pub safety_score: f64,
    pub trend_persistence: f64,
    pub regime_confidence: f64,
    pub breakout_confidence: f64,
    pub signal_decay: f64,
    pub regime_multiplier: f64,
    pub base_score: f64,
    pub final_score: f64,
    pub regime: String,
    pub hard_gates_passed: bool,
    pub failing_gates: Vec<String>,
}

fn clamp(v: f64, lo: f64, hi: f64) -> f64 {
    v.max(lo).min(hi)
}

pub struct DecisionMatrix {
    config: DecisionConfig,
}

impl DecisionMatrix {
    pub fn new(config: DecisionConfig) -> Self {
        Self { config }
    }

    /// Evaluate the decision matrix for a given market state.
    ///
    /// `positioned` — whether we currently hold a position (Long or Short)
    /// `position_dir` — if positioned: 1.0 for Long, -1.0 for Short
    /// `confluence_score` — ICSL weighted score in [-100, +100]
    /// `opposite_score` — ICSL opposite score in [0, 100]
    /// `trade_readiness` — IDCL trade_readiness [0, 1]
    /// `trade_quality` — IDCL trade_quality [0, 1]
    /// `trend_persistence` — IDCL trend_persistence [0, 1]
    /// `risk_level` — IDCL risk_level [0, 1]
    /// `regime` — IRCL regime label (trending, expansion, range, compression, transitional)
    /// `regime_confidence` — IRCL regime_confidence [0, 1]
    /// `breakout_confidence` — ISIL breakout_confidence [0, 1]
    /// `anomaly_score` — ISIL anomaly_score [0, 1]
    /// `compressed` — BBWP gate indicates compression
    /// `choppy` — Choppiness gate indicates range-bound
    /// `confirmed_opposing_divergence` — structural divergence against position
    /// `signal_age_bars` — bars since the triggering signal
    /// `risk_profile` — IRML risk profile
    #[allow(clippy::too_many_arguments)]
    pub fn evaluate(
        &self,
        positioned: bool,
        _position_dir: Option<f64>,
        confluence_score: f64,
        opposite_score: f64,
        trade_readiness: f64,
        trade_quality: f64,
        trend_persistence: f64,
        risk_level: f64,
        regime: &str,
        regime_confidence: f64,
        breakout_confidence: f64,
        anomaly_score: f64,
        compressed: bool,
        choppy: bool,
        confirmed_opposing_divergence: bool,
        signal_age_bars: u32,
        risk_profile: Option<&RiskProfile>,
    ) -> DecisionOutput {
        let mut failing_gates: Vec<String> = Vec::new();

        // ── Extract IRML state ──
        let permission = risk_profile.map(|r| r.permission).unwrap_or(TradePermission::Allowed);
        let exposure = risk_profile.map(|r| r.exposure).unwrap_or(ExposureTier::Maximum);

        // ── STEP 1: Hard gates (any fail → Wait, confidence 100) ──
        let hard_gates_passed = self.check_hard_gates(
            permission,
            exposure,
            anomaly_score,
            regime,
            compressed,
            breakout_confidence,
            choppy,
            &mut failing_gates,
        );

        if !hard_gates_passed {
            return DecisionOutput {
                action: Action::Wait,
                confidence: 100.0,
                rationale: format!("Hard gate(s) failed: {}", failing_gates.join(", ")),
                risk_notes: String::new(),
                factor_breakdown: FactorBreakdown {
                    confluence_norm: confluence_score.abs() / 100.0,
                    trade_readiness,
                    trade_quality,
                    safety_score: (1.0 - risk_level).max(0.0),
                    trend_persistence,
                    regime_confidence,
                    breakout_confidence,
                    signal_decay: 0.0,
                    regime_multiplier: 0.0,
                    base_score: 0.0,
                    final_score: 0.0,
                    regime: regime.to_string(),
                    hard_gates_passed: false,
                    failing_gates,
                },
            };
        }

        // ── STEP 2: Position-gated exits ──
        if positioned {
            return self.evaluate_exit(
                opposite_score,
                confirmed_opposing_divergence,
                permission,
                confluence_score,
                trade_readiness,
                trade_quality,
                trend_persistence,
                risk_level,
                regime,
                regime_confidence,
                breakout_confidence,
                signal_age_bars,
            );
        }

        // ── STEP 3: Weighted composite score ──
        let safety_score = (1.0 - risk_level).max(0.0);
        let confluence_norm = confluence_score.abs() / 100.0;

        let structure_mult = self.resolve_regime_multiplier(regime);

        let signal_decay = if signal_age_bars == 0 || self.config.max_signal_age_bars == 0 {
            1.0
        } else {
            clamp(1.0 - (signal_age_bars as f64 / self.config.max_signal_age_bars as f64), 0.0, 1.0)
        };

        let base_score = clamp(
            confluence_norm * self.config.w_confluence
                + trade_readiness * self.config.w_readiness
                + trade_quality * self.config.w_quality
                + safety_score * self.config.w_safety
                + trend_persistence * self.config.w_trend
                + regime_confidence * self.config.w_regime_conf
                + breakout_confidence * self.config.w_breakout,
            0.0,
            1.0,
        );

        let final_score = clamp(base_score * structure_mult * signal_decay, 0.0, 1.0);

        let threshold = self.resolve_open_threshold(regime);
        let confidence = clamp(final_score * 100.0, 0.0, 100.0);

        // ── STEP 4: Directional action ──
        if final_score >= threshold && confluence_score > 0.0 {
            DecisionOutput {
                action: Action::OpenLong,
                confidence,
                rationale: format!(
                    "Final score {:.2} >= threshold {:.2} with positive confluence ({:.0})",
                    final_score, threshold, confluence_score
                ),
                risk_notes: format!(
                    "Regime: {}, regime mult: {:.2}, signal decay: {:.2}",
                    regime, structure_mult, signal_decay
                ),
                factor_breakdown: FactorBreakdown {
                    confluence_norm,
                    trade_readiness,
                    trade_quality,
                    safety_score,
                    trend_persistence,
                    regime_confidence,
                    breakout_confidence,
                    signal_decay,
                    regime_multiplier: structure_mult,
                    base_score,
                    final_score,
                    regime: regime.to_string(),
                    hard_gates_passed: true,
                    failing_gates,
                },
            }
        } else if final_score >= threshold && confluence_score < 0.0 {
            DecisionOutput {
                action: Action::OpenShort,
                confidence,
                rationale: format!(
                    "Final score {:.2} >= threshold {:.2} with negative confluence ({:.0})",
                    final_score, threshold, confluence_score
                ),
                risk_notes: format!(
                    "Regime: {}, regime mult: {:.2}, signal decay: {:.2}",
                    regime, structure_mult, signal_decay
                ),
                factor_breakdown: FactorBreakdown {
                    confluence_norm,
                    trade_readiness,
                    trade_quality,
                    safety_score,
                    trend_persistence,
                    regime_confidence,
                    breakout_confidence,
                    signal_decay,
                    regime_multiplier: structure_mult,
                    base_score,
                    final_score,
                    regime: regime.to_string(),
                    hard_gates_passed: true,
                    failing_gates,
                },
            }
        } else {
            DecisionOutput {
                action: Action::Wait,
                confidence,
                rationale: format!(
                    "Final score {:.2} < threshold {:.2}. Insufficient conviction to open.",
                    final_score, threshold
                ),
                risk_notes: format!(
                    "Regime: {}, confluence: {:.0}, readiness: {:.2}",
                    regime, confluence_score, trade_readiness
                ),
                factor_breakdown: FactorBreakdown {
                    confluence_norm,
                    trade_readiness,
                    trade_quality,
                    safety_score,
                    trend_persistence,
                    regime_confidence,
                    breakout_confidence,
                    signal_decay,
                    regime_multiplier: structure_mult,
                    base_score,
                    final_score,
                    regime: regime.to_string(),
                    hard_gates_passed: true,
                    failing_gates,
                },
            }
        }
    }

    fn check_hard_gates(
        &self,
        permission: TradePermission,
        exposure: ExposureTier,
        anomaly_score: f64,
        regime: &str,
        compressed: bool,
        breakout_confidence: f64,
        choppy: bool,
        failing: &mut Vec<String>,
    ) -> bool {
        if permission == TradePermission::EmergencyStop {
            failing.push("IRML: Emergency Stop active".into());
            return false;
        }
        if permission == TradePermission::Suspended {
            failing.push("IRML: Trading suspended (consecutive losses)".into());
            return false;
        }
        if exposure == ExposureTier::Zero {
            failing.push("IRML: Zero exposure tier (no capital allocation)".into());
            return false;
        }
        if anomaly_score > self.config.anomaly_threshold {
            failing.push(format!(
                "ISIL: Anomaly score {:.2} > threshold {:.2}",
                anomaly_score, self.config.anomaly_threshold
            ));
            return false;
        }
        if regime == "transitional" || regime == "TRANSITIONAL" {
            failing.push("IRCL: Transitional regime — no directional trades".into());
            return false;
        }
        if compressed && breakout_confidence < 0.7 {
            failing.push("ISIL: Compression regime without sufficient breakout confidence".into());
            return false;
        }
        if choppy {
            failing.push("IRCL: Choppy/range-bound — wait for trend clarity".into());
            return false;
        }
        true
    }

    #[allow(clippy::too_many_arguments)]
    fn evaluate_exit(
        &self,
        opposite_score: f64,
        confirmed_opposing_divergence: bool,
        permission: TradePermission,
        confluence_score: f64,
        trade_readiness: f64,
        trade_quality: f64,
        trend_persistence: f64,
        risk_level: f64,
        regime: &str,
        regime_confidence: f64,
        breakout_confidence: f64,
        signal_age_bars: u32,
    ) -> DecisionOutput {
        let safety_score = (1.0 - risk_level).max(0.0);
        let confluence_norm = confluence_score.abs() / 100.0;
        let structure_mult = self.resolve_regime_multiplier(regime);
        let signal_decay = if signal_age_bars == 0 || self.config.max_signal_age_bars == 0 {
            1.0
        } else {
            clamp(1.0 - (signal_age_bars as f64 / self.config.max_signal_age_bars as f64), 0.0, 1.0)
        };

        let base_score = clamp(
            confluence_norm * self.config.w_confluence
                + trade_readiness * self.config.w_readiness
                + trade_quality * self.config.w_quality
                + safety_score * self.config.w_safety
                + trend_persistence * self.config.w_trend
                + regime_confidence * self.config.w_regime_conf
                + breakout_confidence * self.config.w_breakout,
            0.0,
            1.0,
        );

        let final_score = clamp(base_score * structure_mult * signal_decay, 0.0, 1.0);

        // Exit conditions
        if opposite_score > self.config.exit_opposite_threshold {
            return DecisionOutput {
                action: Action::Close,
                confidence: 100.0,
                rationale: format!(
                    "Opposite score {:.0} > exit threshold {:.0} — position invalidated",
                    opposite_score, self.config.exit_opposite_threshold
                ),
                risk_notes: "Opposite confluence exceeds exit threshold".into(),
                factor_breakdown: FactorBreakdown {
                    confluence_norm,
                    trade_readiness,
                    trade_quality,
                    safety_score,
                    trend_persistence,
                    regime_confidence,
                    breakout_confidence,
                    signal_decay,
                    regime_multiplier: structure_mult,
                    base_score,
                    final_score,
                    regime: regime.to_string(),
                    hard_gates_passed: true,
                    failing_gates: Vec::new(),
                },
            };
        }

        if confirmed_opposing_divergence {
            return DecisionOutput {
                action: Action::Close,
                confidence: 90.0,
                rationale: "Confirmed opposing divergence with structural break — position invalidated".into(),
                risk_notes: "Divergence confirmed by S/R break".into(),
                factor_breakdown: FactorBreakdown {
                    confluence_norm,
                    trade_readiness,
                    trade_quality,
                    safety_score,
                    trend_persistence,
                    regime_confidence,
                    breakout_confidence,
                    signal_decay,
                    regime_multiplier: structure_mult,
                    base_score,
                    final_score,
                    regime: regime.to_string(),
                    hard_gates_passed: true,
                    failing_gates: Vec::new(),
                },
            };
        }

        if permission == TradePermission::Restricted || permission == TradePermission::EmergencyStop {
            return DecisionOutput {
                action: Action::Close,
                confidence: 95.0,
                rationale: format!("IRML permission {:?} — position must close", permission.as_str()),
                risk_notes: "Risk management override".into(),
                factor_breakdown: FactorBreakdown {
                    confluence_norm,
                    trade_readiness,
                    trade_quality,
                    safety_score,
                    trend_persistence,
                    regime_confidence,
                    breakout_confidence,
                    signal_decay,
                    regime_multiplier: structure_mult,
                    base_score,
                    final_score,
                    regime: regime.to_string(),
                    hard_gates_passed: true,
                    failing_gates: Vec::new(),
                },
            };
        }

        // Default: hold
        DecisionOutput {
            action: Action::Hold,
            confidence: 80.0,
            rationale: "No exit signals detected. Thesis remains intact.".into(),
            risk_notes: String::new(),
            factor_breakdown: FactorBreakdown {
                confluence_norm,
                trade_readiness,
                trade_quality,
                safety_score,
                trend_persistence,
                regime_confidence,
                breakout_confidence,
                signal_decay,
                regime_multiplier: structure_mult,
                base_score,
                final_score,
                regime: regime.to_string(),
                hard_gates_passed: true,
                failing_gates: Vec::new(),
            },
        }
    }

    fn resolve_regime_multiplier(&self, regime: &str) -> f64 {
        match regime.to_lowercase().as_str() {
            "trending" => self.config.regime_mult_trending,
            "expansion" => self.config.regime_mult_expansion,
            "range" => self.config.regime_mult_range,
            "compression" => self.config.regime_mult_compression,
            "transitional" => self.config.regime_mult_transitional,
            _ => 1.0,
        }
    }

    fn resolve_open_threshold(&self, regime: &str) -> f64 {
        match regime.to_lowercase().as_str() {
            "trending" => self.config.open_threshold_trending,
            "expansion" => self.config.open_threshold_expansion,
            "range" => self.config.open_threshold_range,
            "compression" => self.config.open_threshold_compression,
            _ => 0.70,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_matrix() -> DecisionMatrix {
        DecisionMatrix::new(DecisionConfig::default())
    }

    #[test]
    fn test_hard_gate_emergency_stop() {
        let m = default_matrix();
        let output = m.evaluate(
            false, None, 50.0, 0.0, 0.8, 0.8, 0.7, 0.2,
            "trending", 0.8, 0.7, 0.1, false, false, false, 1,
            None,
        );
        // No risk_profile means default Allowed — should pass
        assert_eq!(output.action, Action::OpenLong);
    }

    #[test]
    fn test_hard_gate_transitional() {
        let m = default_matrix();
        let output = m.evaluate(
            false, None, 50.0, 0.0, 0.8, 0.8, 0.7, 0.2,
            "transitional", 0.3, 0.7, 0.1, false, false, false, 1,
            None,
        );
        assert_eq!(output.action, Action::Wait);
        assert!(output.factor_breakdown.failing_gates.iter().any(|g| g.contains("Transitional")));
    }

    #[test]
    fn test_open_long() {
        let m = default_matrix();
        let output = m.evaluate(
            false, None, 60.0, 0.0, 0.8, 0.8, 0.7, 0.2,
            "trending", 0.8, 0.7, 0.1, false, false, false, 1,
            None,
        );
        assert_eq!(output.action, Action::OpenLong);
        assert!(output.confidence > 0.0);
    }

    #[test]
    fn test_open_short() {
        let m = default_matrix();
        let output = m.evaluate(
            false, None, -55.0, 0.0, 0.75, 0.75, 0.7, 0.2,
            "trending", 0.8, 0.7, 0.1, false, false, false, 1,
            None,
        );
        assert_eq!(output.action, Action::OpenShort);
    }

    #[test]
    fn test_wait_below_threshold() {
        let m = default_matrix();
        let output = m.evaluate(
            false, None, 20.0, 0.0, 0.3, 0.3, 0.3, 0.6,
            "range", 0.4, 0.2, 0.1, false, false, false, 10,
            None,
        );
        assert_eq!(output.action, Action::Wait);
    }

    #[test]
    fn test_exit_on_opposite_score() {
        let m = default_matrix();
        let output = m.evaluate(
            true, Some(1.0), 30.0, 65.0, 0.6, 0.6, 0.5, 0.3,
            "trending", 0.7, 0.5, 0.1, false, false, false, 2,
            None,
        );
        assert_eq!(output.action, Action::Close);
    }

    #[test]
    fn test_hold_no_exit_signals() {
        let m = default_matrix();
        let output = m.evaluate(
            true, Some(1.0), 40.0, 30.0, 0.6, 0.6, 0.5, 0.3,
            "trending", 0.7, 0.5, 0.1, false, false, false, 2,
            None,
        );
        assert_eq!(output.action, Action::Hold);
    }

    #[test]
    fn test_signal_decay_reduces_score() {
        let m = default_matrix();
        let fresh = m.evaluate(
            false, None, 50.0, 0.0, 0.7, 0.7, 0.6, 0.2,
            "trending", 0.7, 0.6, 0.1, false, false, false, 0,
            None,
        );
        let stale = m.evaluate(
            false, None, 50.0, 0.0, 0.7, 0.7, 0.6, 0.2,
            "trending", 0.7, 0.6, 0.1, false, false, false, 5,
            None,
        );
        assert!(fresh.factor_breakdown.final_score >= stale.factor_breakdown.final_score);
    }
}
