//! # Risk Matrix — Market Risk Assessment
//!
//! The Risk Matrix evaluates market-derived risk factors for a single symbol
//! by consuming the Alignment Matrix and per-timeframe Metrics. It answers:
//! *given the current market conditions, how should risk be managed?*
//!
//! It does NOT know about portfolio state, account balance, or position size.
//!
//! Layer: L4.25 in the architecture (between Alignment and Analysis).

use crate::alignment::AlignmentMatrix;
use crate::indicators::normalized::NormalizedIndicatorValue;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Market risk level classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskLevel {
    VeryLow,
    Low,
    Moderate,
    High,
    Extreme,
}

impl std::fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RiskLevel::VeryLow => write!(f, "VERY_LOW"),
            RiskLevel::Low => write!(f, "LOW"),
            RiskLevel::Moderate => write!(f, "MODERATE"),
            RiskLevel::High => write!(f, "HIGH"),
            RiskLevel::Extreme => write!(f, "EXTREME"),
        }
    }
}

/// Trend stability assessment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrendStability {
    Weak,
    Developing,
    Healthy,
    Strong,
    Exhausted,
}

impl std::fmt::Display for TrendStability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TrendStability::Weak => write!(f, "WEAK"),
            TrendStability::Developing => write!(f, "DEVELOPING"),
            TrendStability::Healthy => write!(f, "HEALTHY"),
            TrendStability::Strong => write!(f, "STRONG"),
            TrendStability::Exhausted => write!(f, "EXHAUSTED"),
        }
    }
}

/// Signal reliability assessment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignalReliability {
    Poor,
    Fair,
    Good,
    Excellent,
}

impl std::fmt::Display for SignalReliability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SignalReliability::Poor => write!(f, "POOR"),
            SignalReliability::Fair => write!(f, "FAIR"),
            SignalReliability::Good => write!(f, "GOOD"),
            SignalReliability::Excellent => write!(f, "EXCELLENT"),
        }
    }
}

/// Suggested stop-loss method.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StopMethod {
    ATR,
    SwingLow,
    SwingHigh,
    Support,
    Resistance,
    VWAP,
    Supertrend,
    StructureBased,
}

impl std::fmt::Display for StopMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StopMethod::ATR => write!(f, "ATR"),
            StopMethod::SwingLow => write!(f, "SWING_LOW"),
            StopMethod::SwingHigh => write!(f, "SWING_HIGH"),
            StopMethod::Support => write!(f, "SUPPORT"),
            StopMethod::Resistance => write!(f, "RESISTANCE"),
            StopMethod::VWAP => write!(f, "VWAP"),
            StopMethod::Supertrend => write!(f, "SUPERTREND"),
            StopMethod::StructureBased => write!(f, "STRUCTURE_BASED"),
        }
    }
}

/// Suggested take-profit method.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetMethod {
    Fibonacci,
    SwingHigh,
    SwingLow,
    ATRMultiple,
    Resistance,
    Support,
    Donchian,
}

impl std::fmt::Display for TargetMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TargetMethod::Fibonacci => write!(f, "FIBONACCI"),
            TargetMethod::SwingHigh => write!(f, "SWING_HIGH"),
            TargetMethod::SwingLow => write!(f, "SWING_LOW"),
            TargetMethod::ATRMultiple => write!(f, "ATR_MULTIPLE"),
            TargetMethod::Resistance => write!(f, "RESISTANCE"),
            TargetMethod::Support => write!(f, "SUPPORT"),
            TargetMethod::Donchian => write!(f, "DONCHIAN"),
        }
    }
}

/// Market risk assessment for a single symbol.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RiskMatrix {
    pub symbol: String,
    pub overall_market_risk: RiskLevel,
    pub volatility_risk: RiskLevel,
    pub liquidity_risk: RiskLevel,
    pub trend_stability: TrendStability,
    pub structural_risk: RiskLevel,
    pub signal_reliability: SignalReliability,
    pub suggested_stop_method: StopMethod,
    pub suggested_stop_distance: f64,
    pub suggested_target_method: TargetMethod,
    pub expected_rr: f64,
}

impl RiskMatrix {
    pub fn empty(symbol: &str) -> Self {
        Self {
            symbol: symbol.to_string(),
            overall_market_risk: RiskLevel::Moderate,
            volatility_risk: RiskLevel::Moderate,
            liquidity_risk: RiskLevel::Moderate,
            trend_stability: TrendStability::Developing,
            structural_risk: RiskLevel::Moderate,
            signal_reliability: SignalReliability::Fair,
            suggested_stop_method: StopMethod::ATR,
            suggested_stop_distance: 2.0,
            suggested_target_method: TargetMethod::ATRMultiple,
            expected_rr: 2.0,
        }
    }
}

/// Assess volatility risk from ATR and BBWP.
fn assess_volatility_risk(indicators: &HashMap<String, NormalizedIndicatorValue>) -> RiskLevel {
    let bbwp = indicators.get("bbwp").map(|v| v.raw_value).unwrap_or(50.0);
    let squeeze_on = indicators.get("squeeze")
        .map(|v| v.state_label.contains("COMPRESSION"))
        .unwrap_or(false);

    if bbwp >= 90.0 {
        RiskLevel::Extreme
    } else if bbwp >= 60.0 || squeeze_on {
        RiskLevel::High
    } else if bbwp <= 20.0 {
        RiskLevel::Low
    } else {
        RiskLevel::Moderate
    }
}

/// Assess liquidity risk from volume, RVOL, and spread.
fn assess_liquidity_risk(indicators: &HashMap<String, NormalizedIndicatorValue>) -> RiskLevel {
    let rvol = indicators.get("rvol").map(|v| v.raw_value).unwrap_or(1.0);
    let spread = indicators.get("spread").map(|v| v.raw_value).unwrap_or(0.0);

    if rvol >= 2.0 && spread < 0.05 {
        RiskLevel::VeryLow
    } else if rvol >= 1.5 {
        RiskLevel::Low
    } else if rvol < 0.5 || spread > 0.2 {
        RiskLevel::High
    } else if rvol < 0.8 {
        RiskLevel::Moderate
    } else {
        RiskLevel::Low
    }
}

/// Assess trend stability from ADX and EMA stack.
fn assess_trend_stability(indicators: &HashMap<String, NormalizedIndicatorValue>) -> TrendStability {
    let adx = indicators.get("adx").map(|v| v.raw_value).unwrap_or(20.0);
    let ema_label = indicators.get("ema_stack")
        .map(|v| v.state_label.as_str())
        .unwrap_or("NEUTRAL");

    let is_bull = ema_label.contains("BULLISH");
    let is_bear = ema_label.contains("BEARISH");
    let is_aligned = is_bull || is_bear;

    if adx >= 40.0 && is_aligned {
        TrendStability::Strong
    } else if adx >= 50.0 {
        TrendStability::Exhausted
    } else if adx >= 25.0 && is_aligned {
        TrendStability::Healthy
    } else if adx >= 15.0 {
        TrendStability::Developing
    } else {
        TrendStability::Weak
    }
}

/// Assess structural risk from S/R proximity.
fn assess_structural_risk(indicators: &HashMap<String, NormalizedIndicatorValue>) -> RiskLevel {
    let sr_state = indicators.get("support_resistance")
        .map(|v| v.state_label.as_str())
        .unwrap_or("NEUTRAL");

    if sr_state.contains("FLIP") || sr_state.contains("BREAKOUT") {
        RiskLevel::High
    } else if sr_state.contains("DEMAND_ZONE") || sr_state.contains("SUPPLY_ZONE") {
        RiskLevel::Low
    } else {
        RiskLevel::Moderate
    }
}

/// Assess signal reliability from the Alignment Matrix's trend agreement.
fn assess_signal_reliability(alignment: &AlignmentMatrix) -> SignalReliability {
    let pct = alignment.trend_agreement_pct;
    let cross_tf = alignment.signal_cross_tf_count;

    if pct >= 90.0 && cross_tf >= 4 {
        SignalReliability::Excellent
    } else if pct >= 75.0 {
        SignalReliability::Good
    } else if pct >= 50.0 {
        SignalReliability::Fair
    } else {
        SignalReliability::Poor
    }
}

/// Determine overall market risk by aggregating individual risk factors.
fn overall_risk(
    vol: RiskLevel, liq: RiskLevel, trend: TrendStability, structural: RiskLevel,
) -> RiskLevel {
    let score = |r: RiskLevel| -> f64 {
        match r {
            RiskLevel::VeryLow => 1.0, RiskLevel::Low => 2.0,
            RiskLevel::Moderate => 3.0, RiskLevel::High => 4.0, RiskLevel::Extreme => 5.0,
        }
    };
    let trend_score = match trend {
        TrendStability::Weak => 4.0, TrendStability::Developing => 3.0,
        TrendStability::Healthy => 2.0, TrendStability::Strong => 1.5, TrendStability::Exhausted => 4.5,
    };
    let avg = (score(vol) + score(liq) + trend_score + score(structural)) / 4.0;
    if avg >= 4.0 { RiskLevel::High }
    else if avg >= 3.0 { RiskLevel::Moderate }
    else if avg >= 2.0 { RiskLevel::Low }
    else { RiskLevel::VeryLow }
}

/// Determine the suggested stop method based on market conditions.
fn determine_stop_method(indicators: &HashMap<String, NormalizedIndicatorValue>) -> StopMethod {
    let adx = indicators.get("adx").map(|v| v.raw_value).unwrap_or(20.0);
    let squeeze_on = indicators.get("squeeze")
        .map(|v| v.state_label.contains("COMPRESSION"))
        .unwrap_or(false);

    if squeeze_on {
        StopMethod::Supertrend
    } else if adx >= 30.0 {
        StopMethod::ATR
    } else {
        StopMethod::SwingLow
    }
}

/// Determine the suggested take-profit method based on market conditions.
fn determine_target_method(indicators: &HashMap<String, NormalizedIndicatorValue>) -> TargetMethod {
    let fib = indicators.get("fibonacci");
    let has_fib = fib.map(|v| v.state_label != "INACTIVE").unwrap_or(false);
    let adx = indicators.get("adx").map(|v| v.raw_value).unwrap_or(20.0);

    if has_fib && adx >= 25.0 {
        TargetMethod::Fibonacci
    } else if adx >= 25.0 {
        TargetMethod::ATRMultiple
    } else {
        TargetMethod::Resistance
    }
}

/// Compute the Risk Matrix from per-timeframe indicator maps and the
/// Alignment Matrix. Uses the slowest active timeframe for stability
/// assessment and the fastest for volatility/liquidity.
pub fn compute_risk(
    symbol: &str,
    alignment: &AlignmentMatrix,
    fastest_indicators: &HashMap<String, NormalizedIndicatorValue>,
    slowest_indicators: Option<&HashMap<String, NormalizedIndicatorValue>>,
) -> RiskMatrix {
    if alignment.timeframes_present == 0 {
        return RiskMatrix::empty(symbol);
    }

    let stable = slowest_indicators.unwrap_or(fastest_indicators);

    let vol_risk = assess_volatility_risk(fastest_indicators);
    let liq_risk = assess_liquidity_risk(fastest_indicators);
    let trend_stab = assess_trend_stability(stable);
    let struct_risk = assess_structural_risk(fastest_indicators);
    let sig_rel = assess_signal_reliability(alignment);
    let overall = overall_risk(vol_risk, liq_risk, trend_stab, struct_risk);
    let stop_method = determine_stop_method(fastest_indicators);
    let target_method = determine_target_method(fastest_indicators);

    // Stop distance: based on ATR or default 2.0x
    let atr = fastest_indicators.get("atr").map(|v| v.raw_value).unwrap_or(0.0);
    let stop_dist = if atr > 0.0 { 1.8 } else { 2.0 };

    // Expected RR: rough estimate from target/stop ratio
    let expected_rr = match target_method {
        TargetMethod::Fibonacci => 3.0,
        TargetMethod::ATRMultiple => 2.5,
        _ => 2.0,
    };

    RiskMatrix {
        symbol: symbol.to_string(),
        overall_market_risk: overall,
        volatility_risk: vol_risk,
        liquidity_risk: liq_risk,
        trend_stability: trend_stab,
        structural_risk: struct_risk,
        signal_reliability: sig_rel,
        suggested_stop_method: stop_method,
        suggested_stop_distance: stop_dist,
        suggested_target_method: target_method,
        expected_rr,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn niv(raw: f64, label: &str) -> NormalizedIndicatorValue {
        NormalizedIndicatorValue::scalar(raw, raw / 100.0, label)
    }

    fn build_indicators(adx_val: f64, bbwp_val: f64, rvol_val: f64, ema_label: &str) -> HashMap<String, NormalizedIndicatorValue> {
        let mut m = HashMap::new();
        m.insert("adx".into(), niv(adx_val, ""));
        m.insert("bbwp".into(), niv(bbwp_val, ""));
        m.insert("rvol".into(), niv(rvol_val, ""));
        m.insert("ema_stack".into(), NormalizedIndicatorValue::scalar(0.0, 0.0, ema_label));
        m.insert("atr".into(), niv(100.0, ""));
        m
    }

    #[test]
    fn weak_trend_is_developing() {
        let indicators = build_indicators(18.0, 50.0, 1.0, "NEUTRAL");
        let alignment = crate::alignment::AlignmentMatrix::empty("BTC-USD");
        let risk = compute_risk("BTC-USD", &alignment, &indicators, None);
        assert!(matches!(risk.trend_stability, TrendStability::Developing));
    }

    #[test]
    fn empty_returns_moderate() {
        let risk = RiskMatrix::empty("BTC-USD");
        assert!(matches!(risk.overall_market_risk, RiskLevel::Moderate));
        assert_eq!(risk.suggested_stop_distance, 2.0);
    }
}
