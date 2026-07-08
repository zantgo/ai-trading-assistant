//! The six deterministic risk-category scorers (Section 5) plus the injected
//! inputs (liquidity from OHLC, behavioral from realized performance).
//!
//! Every scorer produces a `[0,1]` risk score and is strictly
//! direction-independent: it never inspects trade direction (Principle 3).

use std::collections::HashMap;

use crate::decision_context::DecisionContext;
use crate::indicators::normalized::NormalizedIndicatorValue;
use crate::market_context::MarketContext;
use crate::risk::object::{clamp01, RiskObject};
use crate::statistics::statistical_context::StatisticalContext;

type IndMap = HashMap<String, NormalizedIndicatorValue>;

/// Weighted mean helper ignoring NaNs; empty → 0.
fn wmean(pairs: &[(f64, f64)]) -> f64 {
    let mut num = 0.0;
    let mut den = 0.0;
    for &(v, w) in pairs {
        if v.is_nan() {
            continue;
        }
        num += clamp01(v) * w;
        den += w;
    }
    if den <= 0.0 {
        0.0
    } else {
        clamp01(num / den)
    }
}

// ─────────────────────────── A. Market Risk ────────────────────────────────

/// How dangerous the market currently is: volatility, trend instability,
/// choppiness, regime uncertainty, structural (statistical) unpredictability.
pub fn market_risk(
    market: Option<&MarketContext>,
    decision: Option<&DecisionContext>,
    stats: Option<&StatisticalContext>,
) -> RiskObject {
    let mut factors: Vec<(f64, f64)> = Vec::new();
    let mut conf: Vec<f64> = Vec::new();

    if let Some(m) = market {
        factors.push((m.volatility.score.abs(), 1.0));
        conf.push(m.volatility.confidence);
        conf.push(m.trend.confidence);
    }
    if let Some(d) = decision {
        factors.push((1.0 - clamp01(d.trend_persistence), 1.0)); // trend instability
        factors.push((1.0 - clamp01(d.consensus), 1.0)); // choppiness / fragmentation
        factors.push((1.0 - clamp01(d.regime_confidence), 0.8)); // regime uncertainty
    }
    if let Some(s) = stats {
        factors.push(((s.volatility_percentile / 100.0), 0.8));
        factors.push((1.0 - clamp01(s.market_predictability), 1.0));
    }

    let score = wmean(&factors);
    let confidence = if conf.is_empty() {
        0.5
    } else {
        clamp01(conf.iter().sum::<f64>() / conf.len() as f64)
    };
    let regime = market.map(|m| m.regime.as_str()).unwrap_or("UNKNOWN");
    RiskObject::new(
        score,
        confidence,
        format!(
            "Regime {regime}; volatility & trend-instability composite of {} factors",
            factors.len()
        ),
    )
}

// ─────────────────────────── B. Structural Risk ────────────────────────────

/// Confidence in market structure: logical stop presence, structural clarity.
pub fn structural_risk(decision: Option<&DecisionContext>, market: Option<&MarketContext>) -> RiskObject {
    let mut factors: Vec<(f64, f64)> = Vec::new();
    let mut expl = String::from("Structural clarity");
    if let Some(d) = decision {
        factors.push((1.0 - clamp01(d.market_quality), 1.0));
        factors.push((1.0 - clamp01(d.trade_quality), 0.8));
        // No logical stop → elevated structural risk.
        if d.recommended_stop <= 0.0 {
            factors.push((1.0, 1.0));
            expl.push_str("; no logical stop identified");
        }
    }
    if let Some(m) = market {
        // Weak/uncertain trend alignment raises structural risk.
        factors.push((1.0 - clamp01(m.trend.confidence), 0.6));
    }
    let score = wmean(&factors);
    let confidence = decision.map(|d| clamp01(d.regime_confidence)).unwrap_or(0.5);
    RiskObject::new(score, confidence, expl)
}

// ─────────────────────────── C. Momentum Risk ──────────────────────────────

/// Directional stability: divergence, momentum weakening, trend persistence.
pub fn momentum_risk(map: &IndMap, decision: Option<&DecisionContext>, market: Option<&MarketContext>) -> RiskObject {
    let mut factors: Vec<(f64, f64)> = Vec::new();

    // Divergence: any indicator whose key/label signals divergence.
    let mut divergence = 0.0f64;
    for (key, v) in map.iter() {
        let is_div = key.contains("divergence")
            || v.state_label.to_uppercase().contains("DIVERGEN");
        if is_div {
            divergence = divergence.max(v.normalized.abs());
        }
    }
    factors.push((divergence, 1.0));

    if let Some(d) = decision {
        factors.push((1.0 - clamp01(d.trend_persistence), 1.0));
        factors.push((1.0 - clamp01(d.consensus), 0.8));
    }
    let mut confidence = 0.5;
    if let Some(m) = market {
        confidence = clamp01(m.momentum.confidence.max(0.3));
    }
    let score = wmean(&factors);
    RiskObject::new(
        score,
        confidence,
        format!("Divergence {:.2}; trend-persistence & consensus decay", divergence),
    )
}

// ─────────────────────────── D. Volatility Risk ────────────────────────────

/// Abnormal movement: ATR/HV percentile, BBWP, squeeze, expected volatility.
pub fn volatility_risk(map: &IndMap, decision: Option<&DecisionContext>, stats: Option<&StatisticalContext>) -> RiskObject {
    let mut factors: Vec<(f64, f64)> = Vec::new();
    let raw = |k: &str| map.get(k).map(|v| v.raw_value).unwrap_or(f64::NAN);

    if let Some(s) = stats {
        factors.push((s.volatility_percentile / 100.0, 1.2));
        factors.push((clamp01(s.volatility_shock_prob), 0.8));
    }
    let bbwp = raw("bbwp");
    if !bbwp.is_nan() {
        factors.push((bbwp / 100.0, 1.0));
    }
    if let Some(d) = decision {
        // expected_volatility is an annualized-ish magnitude; normalize softly.
        factors.push(((d.expected_volatility / 100.0).min(1.0), 0.6));
    }
    let score = wmean(&factors);
    let confidence = if stats.is_some() { 0.8 } else { 0.5 };
    let pctl = stats.map(|s| s.volatility_percentile).unwrap_or(0.0);
    RiskObject::new(
        score,
        confidence,
        format!("Volatility percentile {:.0}; BBWP {:.0}", pctl, if bbwp.is_nan() { 0.0 } else { bbwp }),
    )
}

// ─────────────────────────── E. Liquidity Risk ─────────────────────────────

/// OHLC-derived liquidity proxy inputs (Section 5E). Computed by the engine
/// from recent candle geometry and passed into the profile.
#[derive(Debug, Clone, Default)]
pub struct LiquidityInputs {
    pub large_candle_freq: f64,
    pub gap_freq: f64,
    pub wick_intensity: f64,
    pub rejection_freq: f64,
    pub range_instability: f64,
    pub sample: usize,
}

impl LiquidityInputs {
    /// Estimate liquidity-proxy metrics from recent `(open, high, low, close)`
    /// candles and the current ATR. Pure and deterministic.
    pub fn from_ohlc(candles: &[(f64, f64, f64, f64)], atr: f64) -> Self {
        let n = candles.len();
        if n == 0 {
            return Self::default();
        }
        let mut large = 0.0;
        let mut gaps = 0.0;
        let mut wick_sum = 0.0;
        let mut rejection = 0.0;
        let mut ranges: Vec<f64> = Vec::with_capacity(n);
        let mut prev_close: Option<f64> = None;
        let atr_ref = if atr > 0.0 { atr } else {
            // fallback: mean range
            let mr: f64 = candles.iter().map(|(_, h, l, _)| (h - l).abs()).sum::<f64>() / n as f64;
            if mr > 0.0 { mr } else { 1.0 }
        };
        for &(o, h, l, c) in candles {
            let range = (h - l).abs();
            ranges.push(range);
            if range > 1.5 * atr_ref {
                large += 1.0;
            }
            let body = (c - o).abs();
            let wick = (range - body).max(0.0);
            if range > 0.0 {
                wick_sum += wick / range;
                if wick / range > 0.66 {
                    rejection += 1.0;
                }
            }
            if let Some(pc) = prev_close {
                let gap = (o - pc).abs();
                if gap > 0.5 * atr_ref {
                    gaps += 1.0;
                }
            }
            prev_close = Some(c);
        }
        let nf = n as f64;
        let mean_range = ranges.iter().sum::<f64>() / nf;
        let var = if nf > 1.0 {
            ranges.iter().map(|r| (r - mean_range).powi(2)).sum::<f64>() / nf
        } else {
            0.0
        };
        let range_instability = if mean_range > 0.0 {
            (var.sqrt() / mean_range).min(1.0)
        } else {
            0.0
        };
        Self {
            large_candle_freq: large / nf,
            gap_freq: gaps / nf,
            wick_intensity: (wick_sum / nf).min(1.0),
            rejection_freq: rejection / nf,
            range_instability,
            sample: n,
        }
    }
}

/// Execution-difficulty proxy from candle geometry.
pub fn liquidity_risk(inputs: &LiquidityInputs) -> RiskObject {
    let score = wmean(&[
        (inputs.large_candle_freq, 1.0),
        (inputs.gap_freq, 1.0),
        (inputs.wick_intensity, 0.8),
        (inputs.rejection_freq, 1.0),
        (inputs.range_instability, 0.6),
    ]);
    // Confidence scales with sample size (needs a reasonable window).
    let confidence = clamp01(inputs.sample as f64 / 50.0);
    RiskObject::new(
        score,
        confidence,
        format!(
            "Large-candle {:.0}%, gaps {:.0}%, wick {:.0}% over {} candles",
            inputs.large_candle_freq * 100.0,
            inputs.gap_freq * 100.0,
            inputs.wick_intensity * 100.0,
            inputs.sample
        ),
    )
}

// ─────────────────────────── F. Behavioral Risk ────────────────────────────

/// System-health inputs derived from realized performance (Section 5F).
/// Injected by the engine to keep the compute core stateless.
#[derive(Debug, Clone, Default)]
pub struct BehavioralInputs {
    pub consecutive_losses: u32,
    pub consecutive_wins: u32,
    pub recent_win_rate: f64,
    pub recent_trade_count: u32,
    pub drawdown_pct: f64,
    pub suspend_threshold: u32,
    pub drawdown_limit_pct: f64,
    pub suspended: bool,
}

/// System health / overtrading protection. Worst-case biased.
pub fn behavioral_risk(inputs: &BehavioralInputs) -> RiskObject {
    if inputs.suspended {
        return RiskObject::new(1.0, 0.95, "System suspended by safety manager".to_string());
    }
    let suspend = inputs.suspend_threshold.max(1) as f64;
    let dd_limit = if inputs.drawdown_limit_pct > 0.0 {
        inputs.drawdown_limit_pct
    } else {
        30.0
    };
    let streak_risk = (inputs.consecutive_losses as f64 / suspend).min(1.0);
    let dd_risk = (inputs.drawdown_pct / dd_limit).clamp(0.0, 1.0);
    // Performance deficit vs 50% baseline (only if win rate below breakeven).
    let perf_risk = clamp01((0.5 - inputs.recent_win_rate) * 2.0);
    // Worst-case bias — the most dangerous behavioral signal dominates.
    let score = streak_risk.max(dd_risk).max(perf_risk * 0.8);
    let confidence = clamp01(inputs.recent_trade_count as f64 / 10.0).max(0.3);
    RiskObject::new(
        score,
        confidence,
        format!(
            "{} consecutive losses; drawdown {:.1}%; recent win-rate {:.0}%",
            inputs.consecutive_losses,
            inputs.drawdown_pct,
            inputs.recent_win_rate * 100.0
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn behavioral_suspended_is_max() {
        let mut b = BehavioralInputs::default();
        b.suspended = true;
        assert_eq!(behavioral_risk(&b).score, 1.0);
    }

    #[test]
    fn behavioral_streak_monotonic() {
        let base = BehavioralInputs { suspend_threshold: 7, recent_win_rate: 0.5, ..Default::default() };
        let mut a = base.clone();
        a.consecutive_losses = 2;
        let mut b = base.clone();
        b.consecutive_losses = 6;
        assert!(behavioral_risk(&b).score >= behavioral_risk(&a).score);
    }

    #[test]
    fn liquidity_empty_is_zero() {
        let li = LiquidityInputs::from_ohlc(&[], 0.0);
        assert_eq!(liquidity_risk(&li).score, 0.0);
    }

    #[test]
    fn liquidity_detects_large_candles() {
        // atr=1; one huge 10-range candle out of 4 → large_candle_freq > 0.
        let candles = [
            (100.0, 100.5, 99.5, 100.2),
            (100.2, 110.0, 100.0, 109.0),
            (109.0, 109.3, 108.7, 109.1),
            (109.1, 109.4, 108.8, 109.0),
        ];
        let li = LiquidityInputs::from_ohlc(&candles, 1.0);
        assert!(li.large_candle_freq > 0.0);
    }
}
