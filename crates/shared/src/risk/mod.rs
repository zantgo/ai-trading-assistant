//! # Institutional Risk Management Layer (IRML) — deterministic core.
//!
//! A read-only enrichment layer that converts existing analysis
//! (`MarketContext`, `DecisionContext`, `StatisticalContext`, indicators) plus
//! injected behavioral / liquidity inputs into a structured `RiskProfile`.
//!
//! Pure mathematics, no I/O, no AI (Principle 5). The stateful engine
//! (`crates/engine/src/risk_engine.rs`) supplies the behavioral / drawdown /
//! reward-risk inputs and enriches percentiles & trends from persisted history.
//!
//! See `docs/institutional-risk-management-layer.md`.

pub mod categories;
pub mod kelly;
pub mod object;
pub mod risk_parity;
pub mod rr;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::decision_context::DecisionContext;
use crate::indicators::normalized::NormalizedIndicatorValue;
use crate::market_context::MarketContext;
use crate::statistics::statistical_context::StatisticalContext;

pub use categories::{BehavioralInputs, LiquidityInputs};
pub use object::{RiskLevel, RiskObject, RiskTrend};
use object::clamp01;
pub use rr::RewardRiskRecommendation;

// ─────────────────────────── Enums ─────────────────────────────────────────

/// Adaptive exposure tier (Section 8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExposureTier {
    Maximum,
    Normal,
    Reduced,
    Minimal,
    Zero,
}

impl ExposureTier {
    pub fn as_str(self) -> &'static str {
        match self {
            ExposureTier::Maximum => "Maximum",
            ExposureTier::Normal => "Normal",
            ExposureTier::Reduced => "Reduced",
            ExposureTier::Minimal => "Minimal",
            ExposureTier::Zero => "Zero",
        }
    }
    /// Fraction of the pair's base allocation permitted for this tier.
    pub fn scaling_factor(self) -> f64 {
        match self {
            ExposureTier::Maximum => 1.00,
            ExposureTier::Normal => 0.75,
            ExposureTier::Reduced => 0.50,
            ExposureTier::Minimal => 0.25,
            ExposureTier::Zero => 0.0,
        }
    }
}

/// Drawdown-protection state machine (Section 9). Supplied by the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DrawdownState {
    Normal,
    Recovery,
    Defensive,
    Critical,
    Shutdown,
}

impl DrawdownState {
    pub fn as_str(self) -> &'static str {
        match self {
            DrawdownState::Normal => "Normal",
            DrawdownState::Recovery => "Recovery",
            DrawdownState::Defensive => "Defensive",
            DrawdownState::Critical => "Critical",
            DrawdownState::Shutdown => "Shutdown",
        }
    }
}

impl Default for DrawdownState {
    fn default() -> Self {
        DrawdownState::Normal
    }
}

/// Final trading-permission gate (Section 4.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradePermission {
    Allowed,
    Restricted,
    HighCaution,
    Suspended,
    EmergencyStop,
}

impl TradePermission {
    pub fn as_str(self) -> &'static str {
        match self {
            TradePermission::Allowed => "Allowed",
            TradePermission::Restricted => "Restricted",
            TradePermission::HighCaution => "High Caution",
            TradePermission::Suspended => "Suspended",
            TradePermission::EmergencyStop => "Emergency Stop",
        }
    }
}

// ─────────────────────────── Profile ───────────────────────────────────────

/// Complete Position Risk Profile attached to a snapshot (Section 7).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskProfile {
    pub overall_risk: f64,
    pub overall_confidence: f64,
    pub overall_level: RiskLevel,

    pub market: RiskObject,
    pub structural: RiskObject,
    pub momentum: RiskObject,
    pub volatility: RiskObject,
    pub liquidity: RiskObject,
    pub behavioral: RiskObject,

    pub exposure: ExposureTier,
    pub recommended_allocation_pct: f64,
    pub drawdown_state: DrawdownState,
    pub permission: TradePermission,

    pub opportunity_score: f64,
    pub reward_risk: RewardRiskRecommendation,

    pub explanation: String,
}

/// All inputs required to compute a `RiskProfile`. Deterministic sources come
/// from the snapshot; behavioral / liquidity / drawdown / reward_risk are
/// injected by the stateful engine.
pub struct RiskComputeParams<'a> {
    pub indicators: &'a HashMap<String, NormalizedIndicatorValue>,
    pub market: Option<&'a MarketContext>,
    pub decision: Option<&'a DecisionContext>,
    pub stats: Option<&'a StatisticalContext>,
    pub liquidity: &'a LiquidityInputs,
    pub behavioral: &'a BehavioralInputs,
    pub drawdown_state: DrawdownState,
    pub reward_risk: RewardRiskRecommendation,
    /// Per-category weights (market, structural, momentum, volatility, liquidity, behavioral).
    pub category_weights: [f64; 6],
    /// Worst-case bias factor λ (Section 7.1).
    pub worst_case_lambda: f64,
    /// Pair base allocation percentage (from scoring config).
    pub base_allocation_pct: f64,
}

/// Opportunity score `[0,1]` derived from existing decision/statistical layers
/// (Section 11). Higher = more attractive setup.
fn opportunity_score(decision: Option<&DecisionContext>, stats: Option<&StatisticalContext>) -> f64 {
    let mut num = 0.0;
    let mut den = 0.0;
    if let Some(d) = decision {
        num += clamp01(d.trade_quality) * 1.0;
        num += clamp01(d.trade_readiness) * 1.0;
        num += (d.confluence.abs() / 100.0).min(1.0) * 0.8;
        den += 2.8;
    }
    if let Some(s) = stats {
        num += clamp01(s.expected_opportunity) * 0.8;
        num += clamp01(s.breakout_confidence) * 0.6;
        den += 1.4;
    }
    if den <= 0.0 {
        0.0
    } else {
        clamp01(num / den)
    }
}

/// Derive the trade-permission gate from overall risk + states (Sections 4.6, 9).
fn derive_permission(overall_risk: f64, dd: DrawdownState, behavioral_suspended: bool) -> TradePermission {
    if dd == DrawdownState::Shutdown {
        return TradePermission::EmergencyStop;
    }
    if behavioral_suspended {
        return TradePermission::Suspended;
    }
    match dd {
        DrawdownState::Critical => return TradePermission::Restricted,
        DrawdownState::Defensive => {
            if overall_risk > 0.6 {
                return TradePermission::Restricted;
            }
            return TradePermission::HighCaution;
        }
        _ => {}
    }
    if overall_risk > 0.80 {
        TradePermission::Suspended
    } else if overall_risk > 0.60 {
        TradePermission::Restricted
    } else if overall_risk > 0.40 {
        TradePermission::HighCaution
    } else {
        TradePermission::Allowed
    }
}

/// Map overall risk + permission to an exposure tier (Section 8).
fn derive_exposure(overall_risk: f64, permission: TradePermission) -> ExposureTier {
    match permission {
        TradePermission::Suspended | TradePermission::EmergencyStop => return ExposureTier::Zero,
        TradePermission::Restricted => return ExposureTier::Minimal,
        TradePermission::HighCaution => return ExposureTier::Reduced,
        _ => {}
    }
    if overall_risk <= 0.20 {
        ExposureTier::Maximum
    } else if overall_risk <= 0.40 {
        ExposureTier::Normal
    } else if overall_risk <= 0.60 {
        ExposureTier::Reduced
    } else if overall_risk <= 0.80 {
        ExposureTier::Minimal
    } else {
        ExposureTier::Zero
    }
}

impl RiskProfile {
    /// Compute the full deterministic risk profile.
    pub fn compute(p: &RiskComputeParams) -> Self {
        let market = categories::market_risk(p.market, p.decision, p.stats);
        let structural = categories::structural_risk(p.decision, p.market);
        let momentum = categories::momentum_risk(p.indicators, p.decision, p.market);
        let volatility = categories::volatility_risk(p.indicators, p.decision, p.stats);
        let liquidity = categories::liquidity_risk(p.liquidity);
        let behavioral = categories::behavioral_risk(p.behavioral);

        let cats = [&market, &structural, &momentum, &volatility, &liquidity, &behavioral];

        // Confidence-weighted mean (Section 7.1).
        let mut num = 0.0;
        let mut den = 0.0;
        let mut conf_num = 0.0;
        let mut conf_den = 0.0;
        let mut worst = 0.0f64;
        for (i, c) in cats.iter().enumerate() {
            let w = p.category_weights[i];
            num += c.score * c.confidence * w;
            den += c.confidence * w;
            conf_num += c.confidence * w;
            conf_den += w;
            worst = worst.max(c.score);
        }
        let weighted_mean = if den > 0.0 { num / den } else { 0.0 };
        let overall_risk = clamp01(weighted_mean.max(p.worst_case_lambda * worst));
        let overall_confidence = if conf_den > 0.0 {
            clamp01(conf_num / conf_den)
        } else {
            0.5
        };
        let overall_level = RiskLevel::from_score(overall_risk);

        let permission = derive_permission(overall_risk, p.drawdown_state, p.behavioral.suspended);
        let exposure = derive_exposure(overall_risk, permission);
        let recommended_allocation_pct = exposure.scaling_factor() * p.base_allocation_pct.max(0.0);
        let opportunity = opportunity_score(p.decision, p.stats);

        let explanation = format!(
            "Overall risk {:.2} ({}); worst category {:.2}; permission {}; exposure {}",
            overall_risk,
            overall_level.as_str(),
            worst,
            permission.as_str(),
            exposure.as_str()
        );

        Self {
            overall_risk,
            overall_confidence,
            overall_level,
            market,
            structural,
            momentum,
            volatility,
            liquidity,
            behavioral,
            exposure,
            recommended_allocation_pct,
            drawdown_state: p.drawdown_state,
            permission,
            opportunity_score: opportunity,
            reward_risk: p.reward_risk.clone(),
            explanation,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_params<'a>(
        li: &'a LiquidityInputs,
        bi: &'a BehavioralInputs,
        map: &'a HashMap<String, NormalizedIndicatorValue>,
    ) -> RiskComputeParams<'a> {
        RiskComputeParams {
            indicators: map,
            market: None,
            decision: None,
            stats: None,
            liquidity: li,
            behavioral: bi,
            drawdown_state: DrawdownState::Normal,
            reward_risk: RewardRiskRecommendation::default(),
            category_weights: [1.0, 1.0, 1.0, 1.2, 0.8, 1.0],
            worst_case_lambda: 0.5,
            base_allocation_pct: 4.0,
        }
    }

    #[test]
    fn low_risk_yields_allowed_and_allocation() {
        let li = LiquidityInputs::default();
        let bi = BehavioralInputs { suspend_threshold: 7, recent_win_rate: 0.5, ..Default::default() };
        let map = HashMap::new();
        let p = base_params(&li, &bi, &map);
        let profile = RiskProfile::compute(&p);
        assert!(profile.overall_risk >= 0.0 && profile.overall_risk <= 1.0);
        assert_eq!(profile.permission, TradePermission::Allowed);
        assert!(profile.recommended_allocation_pct > 0.0);
    }

    #[test]
    fn suspended_behavioral_blocks_trading() {
        let li = LiquidityInputs::default();
        let bi = BehavioralInputs { suspended: true, suspend_threshold: 7, ..Default::default() };
        let map = HashMap::new();
        let p = base_params(&li, &bi, &map);
        let profile = RiskProfile::compute(&p);
        assert_eq!(profile.permission, TradePermission::Suspended);
        assert_eq!(profile.exposure, ExposureTier::Zero);
        assert_eq!(profile.recommended_allocation_pct, 0.0);
    }

    #[test]
    fn shutdown_is_emergency_stop() {
        let li = LiquidityInputs::default();
        let bi = BehavioralInputs { suspend_threshold: 7, recent_win_rate: 0.5, ..Default::default() };
        let map = HashMap::new();
        let mut p = base_params(&li, &bi, &map);
        p.drawdown_state = DrawdownState::Shutdown;
        let profile = RiskProfile::compute(&p);
        assert_eq!(profile.permission, TradePermission::EmergencyStop);
        assert_eq!(profile.exposure, ExposureTier::Zero);
    }

    #[test]
    fn worst_case_bias_pulls_overall_up() {
        // One extreme behavioral category should lift overall via lambda*max.
        let li = LiquidityInputs::default();
        let bi = BehavioralInputs {
            consecutive_losses: 7,
            suspend_threshold: 7,
            recent_win_rate: 0.5,
            recent_trade_count: 10,
            ..Default::default()
        };
        let map = HashMap::new();
        let p = base_params(&li, &bi, &map);
        let profile = RiskProfile::compute(&p);
        assert!(profile.behavioral.score >= 0.99);
        assert!(profile.overall_risk >= 0.5 * profile.behavioral.score - 1e-9);
    }
}
