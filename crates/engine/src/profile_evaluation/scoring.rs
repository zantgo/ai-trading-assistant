use super::SnapshotValues;
use crate::config::ScoringConfig;
use std::collections::HashMap;

// ─── Legacy hardcoded fallback weights (used when no config provided) ──
const W_RSI_FB: f64 = 10.0;
const W_RSI_DIV_FB: f64 = 20.0;
const W_MACD_FB: f64 = 10.0;
const W_MACD_DIV_FB: f64 = 10.0;
const W_SR_FB: f64 = 10.0;
const W_TREND_FB: f64 = 20.0;
const W_EMA200_FB: f64 = 10.0;
const W_PATTERN_FB: f64 = 10.0;

/// Maximum possible magnitude of the confluence score (±90).
pub const MAX_CONFLUENCE_SCORE: f64 = 90.0;

/// Opposite-signal exit threshold: 60% of the maximum ±90 score.
pub const OPPOSITE_EXIT_THRESHOLD: f64 = 54.0;

/// Resolved weights for a confluence run.
struct ResolvedWeights {
    w_rsi: f64,
    w_rsi_div: f64,
    w_macd: f64,
    w_macd_div: f64,
    w_sr: f64,
    w_trend: f64,
    w_ema200: f64,
    w_pattern: f64,
}

fn resolve_weights(
    overrides: Option<&HashMap<String, i32>>,
    global: Option<&ScoringConfig>,
) -> ResolvedWeights {
    let w = |key: &str, fb: f64| -> f64 {
        if let Some(map) = overrides {
            if let Some(&v) = map.get(key) {
                return v as f64;
            }
        }
        if let Some(cfg) = global {
            match key {
                "rsi" => return cfg.rsi_weight as f64,
                "rsi_divergence" => return cfg.rsi_divergence_weight as f64,
                "macd" => return cfg.macd_weight as f64,
                "macd_divergence" => return cfg.macd_divergence_weight as f64,
                "support_resistance" => return cfg.support_resistance_weight as f64,
                "ema_stack" => return cfg.trend_weight as f64,
                "ema200" => return cfg.ema200_weight as f64,
                "patterns" => return cfg.pattern_weight as f64,
                _ => {}
            }
        }
        fb
    };
    ResolvedWeights {
        w_rsi: w("rsi", W_RSI_FB),
        w_rsi_div: w("rsi_divergence", W_RSI_DIV_FB),
        w_macd: w("macd", W_MACD_FB),
        w_macd_div: w("macd_divergence", W_MACD_DIV_FB),
        w_sr: w("support_resistance", W_SR_FB),
        w_trend: w("ema_stack", W_TREND_FB),
        w_ema200: w("ema200", W_EMA200_FB),
        w_pattern: w("patterns", W_PATTERN_FB),
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct EightFactorScore {
    pub total_score: i32,
    pub max_score: i32,
    pub signals: EightFactorSignals,
    pub allocated_capital_pct: f64,
    pub weighted_contributions: EightFactorContributions,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct EightFactorContributions {
    pub rsi_points: i32,
    pub rsi_divergence_points: i32,
    pub macd_points: i32,
    pub macd_divergence_points: i32,
    pub support_resistance_points: i32,
    pub trend_points: i32,
    pub ema200_points: i32,
    pub pattern_points: i32,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct EightFactorSignals {
    pub rsi_aligned: bool,
    pub rsi_divergence_aligned: bool,
    pub macd_crossover: bool,
    pub macd_divergence_aligned: bool,
    pub support_aligned: bool,
    pub resistance_aligned: bool,
    pub trend_aligned: bool,
    pub ema200_aligned: bool,
    pub pattern_aligned: bool,
}

/// Per-factor continuous contributions, before bias projection.
struct FactorContributions {
    rsi: f64,
    rsi_div: f64,
    macd: f64,
    macd_div: f64,
    sr: f64,
    trend: f64,
    ema200: f64,
    pattern: f64,
}

/// Derived normalized `[-1.0, 1.0]` for the 200 EMA factor.
fn ema200_normalized(snap: &SnapshotValues) -> f64 {
    match snap.sub("ema_stack", "long") {
        Some(ema) if ema > 0.0 => {
            let rel = (snap.current_price - ema) / ema;
            (rel / 0.005).clamp(-1.0, 1.0)
        }
        _ => 0.0,
    }
}

/// Compute the continuous per-factor contributions with resolved weights.
fn compute_factor_contributions(snap: &SnapshotValues, w: &ResolvedWeights) -> FactorContributions {
    let rsi = snap.norm("rsi");
    let rsi_div = snap.norm("rsi_divergence");
    let macd = snap.norm("macd");
    let macd_div = snap.norm("macd_divergence");
    let sr = snap.norm("support_resistance");
    let trend = snap.norm("ema_stack");
    let ema200 = ema200_normalized(snap);
    let pattern = snap.norm("patterns");

    let adx_congested = snap.raw("adx").is_some_and(|a| a < 20.0)
        || snap.label("adx") == "TRENDLESS_CONGESTION";
    let trend_gate = if adx_congested { 0.0 } else { 1.0 };

    let rvol = snap.raw("rvol").unwrap_or(1.0);
    let breakout_active = snap.label("support_resistance").contains("FLIP")
        || snap.label("squeeze").ends_with("VOLATILITY_RELEASE")
        || snap.norm("patterns").abs() >= 1.0;
    let breakout_gate = if breakout_active && rvol < 1.5 { 0.3 } else { 1.0 };

    let bbwp_climax = snap.raw("bbwp").is_some_and(|b| b > 90.0);
    let bias = trend.signum();
    let climax_drag = if bbwp_climax { -0.1 * bias } else { 0.0 };

    FactorContributions {
        rsi: rsi * w.w_rsi,
        rsi_div: rsi_div * w.w_rsi_div,
        macd: macd * w.w_macd,
        macd_div: macd_div * w.w_macd_div,
        sr: sr * breakout_gate * w.w_sr,
        trend: (trend * trend_gate + climax_drag) * w.w_trend,
        ema200: (ema200 * trend_gate) * w.w_ema200,
        pattern: pattern * breakout_gate * w.w_pattern,
    }
}

/// Evaluate the allocation percentage from a confluence score using the given curve model.
pub fn evaluate_allocation_curve(
    abs_score: u32,
    base_pct: f64,
    max_pct: f64,
    base_threshold: u32,
    micro_threshold: u32,
    model: &crate::config::AllocationCurveModel,
    exponent: f64,
) -> f64 {
    match model {
        crate::config::AllocationCurveModel::Stepped => {
            if abs_score < base_threshold {
                base_pct
            } else if abs_score < micro_threshold {
                (base_pct + max_pct) / 2.0
            } else {
                max_pct
            }
        }
        crate::config::AllocationCurveModel::Linear => {
            if abs_score == 0 {
                return base_pct;
            }
            let ratio = (abs_score as f64) / (micro_threshold as f64).max(1.0);
            base_pct + (max_pct - base_pct) * ratio.min(1.0)
        }
        crate::config::AllocationCurveModel::Exponential => {
            if abs_score == 0 {
                return base_pct;
            }
            let ratio = (abs_score as f64) / (micro_threshold as f64).max(1.0);
            base_pct + (max_pct - base_pct) * ratio.min(1.0).powf(exponent)
        }
    }
}

/// Calculate the continuous 8-factor confluence score for a given bias.
pub fn calculate_eight_factor_score(
    bias: &str,
    snap: &SnapshotValues,
    _support_levels: &[f64],
    _resistance_levels: &[f64],
    _macro_trend: &str,
) -> EightFactorScore {
    calculate_eight_factor_score_with_weights(bias, snap, _support_levels, _resistance_levels, _macro_trend, None, None)
}

/// Calculate the score with optional weight overrides and global defaults.
pub fn calculate_eight_factor_score_with_weights(
    bias: &str,
    snap: &SnapshotValues,
    _support_levels: &[f64],
    _resistance_levels: &[f64],
    _macro_trend: &str,
    weight_overrides: Option<&HashMap<String, i32>>,
    scoring_config: Option<&ScoringConfig>,
) -> EightFactorScore {
    let w = resolve_weights(weight_overrides, scoring_config);
    let is_bullish = bias == "BULLISH";
    let c = compute_factor_contributions(snap, &w);

    let signed_total = c.rsi + c.rsi_div + c.macd + c.macd_div + c.sr + c.trend + c.ema200 + c.pattern;
    let projected = if is_bullish { signed_total } else { -signed_total };
    let clamped = projected.clamp(-MAX_CONFLUENCE_SCORE, MAX_CONFLUENCE_SCORE);
    let total_score = clamped.round() as i32;

    let abs_score = total_score.unsigned_abs();

    let (base_pct, max_pct, base_th, micro_th, model) = if let Some(cfg) = scoring_config {
        (
            cfg.base_allocation_pct,
            cfg.max_allocation_pct,
            cfg.base_score_threshold,
            cfg.micro_score_threshold,
            &crate::config::AllocationCurveModel::Stepped,
        )
    } else {
        (1.0, 3.0, 40u32, 60u32, &crate::config::AllocationCurveModel::Stepped)
    };
    let allocated_capital_pct = evaluate_allocation_curve(abs_score, base_pct, max_pct, base_th, micro_th, model, 2.0);

    let aligned = |contribution: f64| -> bool {
        if is_bullish { contribution > 0.0 } else { contribution < 0.0 }
    };
    let signals = EightFactorSignals {
        rsi_aligned: aligned(c.rsi),
        rsi_divergence_aligned: aligned(c.rsi_div),
        macd_crossover: aligned(c.macd),
        macd_divergence_aligned: aligned(c.macd_div),
        support_aligned: aligned(c.sr),
        resistance_aligned: aligned(c.sr),
        trend_aligned: aligned(c.trend),
        ema200_aligned: aligned(c.ema200),
        pattern_aligned: aligned(c.pattern),
    };

    let proj = |v: f64| -> i32 {
        (if is_bullish { v } else { -v }).round() as i32
    };

    EightFactorScore {
        total_score,
        max_score: MAX_CONFLUENCE_SCORE as i32,
        signals,
        allocated_capital_pct,
        weighted_contributions: EightFactorContributions {
            rsi_points: proj(c.rsi),
            rsi_divergence_points: proj(c.rsi_div),
            macd_points: proj(c.macd),
            macd_divergence_points: proj(c.macd_div),
            support_resistance_points: proj(c.sr),
            trend_points: proj(c.trend),
            ema200_points: proj(c.ema200),
            pattern_points: proj(c.pattern),
        },
    }
}

/// Continuous opposite-signal score with optional weight overrides.
pub fn calculate_opposite_score(
    position_direction: &str,
    snap: &SnapshotValues,
    _support_levels: &[f64],
    _resistance_levels: &[f64],
    _macro_trend: &str,
) -> u32 {
    calculate_opposite_score_with_weights(position_direction, snap, _support_levels, _resistance_levels, _macro_trend, None, None)
}

pub fn calculate_opposite_score_with_weights(
    position_direction: &str,
    snap: &SnapshotValues,
    _support_levels: &[f64],
    _resistance_levels: &[f64],
    _macro_trend: &str,
    weight_overrides: Option<&HashMap<String, i32>>,
    scoring_config: Option<&ScoringConfig>,
) -> u32 {
    let w = resolve_weights(weight_overrides, scoring_config);
    let c = compute_factor_contributions(snap, &w);
    let holding_long = position_direction == "LONG";
    let all = [
        c.rsi, c.rsi_div, c.macd, c.macd_div, c.sr, c.trend, c.ema200, c.pattern,
    ];
    let opposing_sum: f64 = all
        .iter()
        .filter(|&&v| if holding_long { v < 0.0 } else { v > 0.0 })
        .sum();
    opposing_sum.abs().round() as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::indicators::normalized::NormalizedIndicatorValue;
    use std::collections::HashMap;

    fn niv(norm: f64, label: &str) -> NormalizedIndicatorValue {
        NormalizedIndicatorValue::scalar(norm, norm, label)
    }

    fn snap_with(entries: &[(&str, f64, &str)], price: f64) -> SnapshotValues {
        let mut map = HashMap::new();
        for (k, n, l) in entries {
            map.insert((*k).to_string(), niv(*n, l));
        }
        SnapshotValues::from_map(map, price)
    }

    fn niv_raw(raw: f64, norm: f64, label: &str) -> NormalizedIndicatorValue {
        NormalizedIndicatorValue::scalar(raw, norm, label)
    }

    #[test]
    fn continuous_bullish_alignment_scores_high() {
        let mut map = HashMap::new();
        map.insert("rsi".into(), niv(0.8, "OVERSOLD_ACCUMULATION"));
        map.insert("rsi_divergence".into(), niv(1.0, "CONFIRMED_BULLISH_DIVERGENCE"));
        map.insert("macd".into(), niv(0.9, "BULLISH_CROSSOVER_ACCELERATING"));
        map.insert("support_resistance".into(), niv(1.0, "SUPPORT_DEMAND_ZONE"));
        map.insert("patterns".into(), niv(1.0, "BULLISH_PATTERN_BREAKOUT"));
        map.insert("adx".into(), niv_raw(30.0, 0.7, "STRONG_BULL_TREND"));
        map.insert("rvol".into(), niv_raw(2.0, 0.8, "INSTITUTIONAL_BREAKOUT_VOLUME"));
        let mut ema_vals = HashMap::new();
        ema_vals.insert("long".to_string(), 49000.0);
        map.insert(
            "ema_stack".into(),
            NormalizedIndicatorValue::with_values(50000.0, 1.0, "ESTABLISHED_BULLISH_STACK", ema_vals),
        );
        let snap = SnapshotValues::from_map(map, 50000.0);

        let score = calculate_eight_factor_score("BULLISH", &snap, &[], &[], "BULLISH");
        assert!(score.total_score >= 60, "expected >=60, got {}", score.total_score);
        assert_eq!(score.allocated_capital_pct, 3.0);
        assert!(score.total_score <= 90);
    }

    #[test]
    fn continuous_score_is_gradient_not_cliff() {
        let weak = snap_with(&[("rsi", 0.3, "BULLISH_DISCOUNT")], 100.0);
        let strong = snap_with(&[("rsi", 0.9, "OVERSOLD_ACCUMULATION")], 100.0);
        let ws = calculate_eight_factor_score("BULLISH", &weak, &[], &[], "");
        let ss = calculate_eight_factor_score("BULLISH", &strong, &[], &[], "");
        assert!(ss.total_score > ws.total_score);
    }

    #[test]
    fn adx_congestion_gate_dampens_trend() {
        let entries = [
            ("ema_stack", 1.0, "ESTABLISHED_BULLISH_STACK"),
            ("adx", 0.0, "TRENDLESS_CONGESTION"),
        ];
        let mut map = HashMap::new();
        for (k, n, l) in entries {
            map.insert(k.to_string(), niv(n, l));
        }
        map.insert(
            "adx".to_string(),
            NormalizedIndicatorValue::scalar(15.0, 0.0, "TRENDLESS_CONGESTION"),
        );
        let snap = SnapshotValues::from_map(map, 100.0);
        let score = calculate_eight_factor_score("BULLISH", &snap, &[], &[], "");
        assert_eq!(score.total_score, 0);
    }

    #[test]
    fn opposite_score_triggers_exit_above_threshold() {
        let snap = snap_with(
            &[
                ("rsi", -0.8, "OVERBOUGHT_DISTRIBUTION"),
                ("rsi_divergence", -1.0, "CONFIRMED_BEARISH_DIVERGENCE"),
                ("macd", -0.9, "BEARISH_CROSSOVER_ACCELERATING"),
                ("ema_stack", -1.0, "ESTABLISHED_BEARISH_STACK"),
                ("support_resistance", -1.0, "RESISTANCE_SUPPLY_ZONE"),
            ],
            50000.0,
        );
        let opp = calculate_opposite_score("LONG", &snap, &[], &[], "");
        assert!(opp > OPPOSITE_EXIT_THRESHOLD as u32, "expected >54, got {}", opp);
    }

    #[test]
    fn opposite_score_minor_signals_no_exit() {
        let snap = snap_with(&[("rsi", -0.2, "BEARISH_PREMIUM")], 50000.0);
        let opp = calculate_opposite_score("LONG", &snap, &[], &[], "");
        assert!(opp <= OPPOSITE_EXIT_THRESHOLD as u32);
    }

    #[test]
    fn custom_weights_scale_score() {
        let snap = snap_with(
            &[
                ("rsi", 1.0, "OVERSOLD_ACCUMULATION"),
                ("ema_stack", 1.0, "ESTABLISHED_BULLISH_STACK"),
            ],
            100.0,
        );
        let mut overrides = HashMap::new();
        overrides.insert("rsi".to_string(), 50i32);
        overrides.insert("ema_stack".to_string(), 0i32);
        let score = calculate_eight_factor_score_with_weights(
            "BULLISH", &snap, &[], &[], "",
            Some(&overrides), None,
        );
        assert!(score.total_score > 0, "RSI with weight 50 should dominate");
        // With ema_stack weight = 0, trend contribution should be zero
        assert_eq!(score.weighted_contributions.trend_points, 0);
    }

    #[test]
    fn zero_weight_removes_indicator_influence() {
        let snap = snap_with(&[("ema_stack", 1.0, "ESTABLISHED_BULLISH_STACK")], 100.0);
        let mut overrides = HashMap::new();
        overrides.insert("ema_stack".to_string(), 0i32);
        let score = calculate_eight_factor_score_with_weights(
            "BULLISH", &snap, &[], &[], "",
            Some(&overrides), None,
        );
        assert_eq!(score.weighted_contributions.trend_points, 0);

        let default_score = calculate_eight_factor_score("BULLISH", &snap, &[], &[], "");
        assert!(default_score.weighted_contributions.trend_points > 0);
    }

    #[test]
    fn allocation_stepped_is_backward_compatible() {
        use crate::config::AllocationCurveModel;
        assert_eq!(evaluate_allocation_curve(20, 1.0, 3.0, 40, 60, &AllocationCurveModel::Stepped, 2.0), 1.0);
        assert_eq!(evaluate_allocation_curve(40, 1.0, 3.0, 40, 60, &AllocationCurveModel::Stepped, 2.0), 2.0);
        assert_eq!(evaluate_allocation_curve(55, 1.0, 3.0, 40, 60, &AllocationCurveModel::Stepped, 2.0), 2.0);
        assert_eq!(evaluate_allocation_curve(60, 1.0, 3.0, 40, 60, &AllocationCurveModel::Stepped, 2.0), 3.0);
    }

    #[test]
    fn allocation_linear_interpolates() {
        use crate::config::AllocationCurveModel;
        let pct = evaluate_allocation_curve(30, 1.0, 3.0, 0, 60, &AllocationCurveModel::Linear, 2.0);
        assert!((pct - 2.0).abs() < 0.01, "linear at 50% of max score should give midpoint; got {}", pct);
    }

    #[test]
    fn allocation_exponential_front_loads() {
        use crate::config::AllocationCurveModel;
        let pct_linear = evaluate_allocation_curve(30, 1.0, 3.0, 0, 60, &AllocationCurveModel::Linear, 2.0);
        let pct_exp = evaluate_allocation_curve(30, 1.0, 3.0, 0, 60, &AllocationCurveModel::Exponential, 3.0);
        assert!(pct_exp <= pct_linear, "exponential (exp=3) at 50% score should be <= linear; exp={} linear={}", pct_exp, pct_linear);
    }
}
