use super::SnapshotValues;

// ─── Confluence factor weights (Section 3.1) ───────────────────────
// Continuous model: each factor contributes `normalized × weight`.
const W_RSI: f64 = 10.0;
const W_RSI_DIV: f64 = 20.0;
const W_MACD: f64 = 10.0;
const W_MACD_DIV: f64 = 10.0;
const W_SR: f64 = 10.0;
const W_TREND: f64 = 20.0;
const W_EMA200: f64 = 10.0;
const W_PATTERN: f64 = 10.0;

/// Maximum possible magnitude of the confluence score (±90).
pub const MAX_CONFLUENCE_SCORE: f64 = 90.0;

/// Opposite-signal exit threshold: 60% of the maximum ±90 score.
pub const OPPOSITE_EXIT_THRESHOLD: f64 = 54.0;

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

/// Derived normalized `[-1.0, 1.0]` for the 200 EMA factor: sign of the
/// distance between price and the long EMA, softly scaled.
fn ema200_normalized(snap: &SnapshotValues) -> f64 {
    match snap.sub("ema_stack", "long") {
        Some(ema) if ema > 0.0 => {
            let rel = (snap.current_price - ema) / ema;
            // ±0.5% band saturates to full conviction.
            (rel / 0.005).clamp(-1.0, 1.0)
        }
        _ => 0.0,
    }
}

/// Compute the continuous per-factor contributions (each `normalized × weight`)
/// with ADX / RVOL / BBWP gating multipliers applied to the relevant factors.
fn compute_factor_contributions(snap: &SnapshotValues) -> FactorContributions {
    // Raw normalized inputs read directly from the map.
    let rsi = snap.norm("rsi");
    let rsi_div = snap.norm("rsi_divergence");
    let macd = snap.norm("macd");
    let macd_div = snap.norm("macd_divergence");
    let sr = snap.norm("support_resistance");
    let trend = snap.norm("ema_stack");
    let ema200 = ema200_normalized(snap);
    let pattern = snap.norm("patterns");

    // ── ADX Congestion Gate ──
    // When ADX signals congestion (< 20 → normalized 0.0 / TRENDLESS label),
    // dampen trend-following factors toward zero.
    let adx_congested = snap.raw("adx").is_some_and(|a| a < 20.0)
        || snap.label("adx") == "TRENDLESS_CONGESTION";
    let trend_gate = if adx_congested { 0.0 } else { 1.0 };

    // ── RVOL Breakout Gate ──
    // If a breakout setup is active (S/R flip or squeeze release) but volume is
    // unconfirmed (rvol < 1.5), dampen the breakout-driven factors by 0.3.
    let rvol = snap.raw("rvol").unwrap_or(1.0);
    let breakout_active = snap.label("support_resistance").contains("FLIP")
        || snap.label("squeeze").ends_with("VOLATILITY_RELEASE")
        || snap.norm("patterns").abs() >= 1.0;
    let breakout_gate = if breakout_active && rvol < 1.5 { 0.3 } else { 1.0 };

    // ── BBWP Volatility Climax Drag ──
    // At volatility exhaustion (BBWP > 90%), apply a counter-bias drag against
    // trend continuation to discourage new positions in overextended markets.
    let bbwp_climax = snap.raw("bbwp").is_some_and(|b| b > 90.0);
    let bias = trend.signum();
    let climax_drag = if bbwp_climax { -0.1 * bias } else { 0.0 };

    FactorContributions {
        rsi: rsi * W_RSI,
        rsi_div: rsi_div * W_RSI_DIV,
        macd: macd * W_MACD,
        macd_div: macd_div * W_MACD_DIV,
        sr: sr * breakout_gate * W_SR,
        trend: (trend * trend_gate + climax_drag) * W_TREND,
        ema200: (ema200 * trend_gate) * W_EMA200,
        pattern: pattern * breakout_gate * W_PATTERN,
    }
}

/// Calculate the continuous 8-factor confluence score for a given bias.
///
/// Each factor contributes `normalized × weight`, gated by ADX/RVOL/BBWP
/// multipliers, then projected onto the requested bias direction and clamped
/// to `[-90, +90]`. Allocation: `<40 → 1%`, `40–59 → 2%`, `≥60 → 3%`.
pub fn calculate_eight_factor_score(
    bias: &str,
    snap: &SnapshotValues,
    _support_levels: &[f64],
    _resistance_levels: &[f64],
    _macro_trend: &str,
) -> EightFactorScore {
    let is_bullish = bias == "BULLISH";
    let c = compute_factor_contributions(snap);

    // Signed sum of continuous contributions (positive = bullish conviction).
    let signed_total = c.rsi + c.rsi_div + c.macd + c.macd_div + c.sr + c.trend + c.ema200 + c.pattern;

    // Project onto the requested bias: bullish keeps the signed value, bearish
    // inverts it so a "BEARISH" query returns positive magnitude for bearish
    // alignment (mirrors the legacy contract used by the opposite-exit engine).
    let projected = if is_bullish { signed_total } else { -signed_total };
    let clamped = projected.clamp(-MAX_CONFLUENCE_SCORE, MAX_CONFLUENCE_SCORE);
    let total_score = clamped.round() as i32;

    let abs_score = total_score.unsigned_abs();
    let allocated_capital_pct = match abs_score {
        0..=39 => 1.0,
        40..=59 => 2.0,
        _ => 3.0,
    };

    // Directional alignment flags (per-factor sign matches the query bias).
    let aligned = |contribution: f64| -> bool {
        if is_bullish {
            contribution > 0.0
        } else {
            contribution < 0.0
        }
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

    // Bias-projected integer contributions for telemetry/logging.
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

/// Continuous opposite-signal score: the absolute weighted sum of all factors
/// pointing against the active position direction.
///
/// `Opposite Score = |Σ (normalized_i × weight_i)|` over opposing factors.
pub fn calculate_opposite_score(
    position_direction: &str,
    snap: &SnapshotValues,
    _support_levels: &[f64],
    _resistance_levels: &[f64],
    _macro_trend: &str,
) -> u32 {
    let c = compute_factor_contributions(snap);
    // Positive contribution = bullish conviction. Opposing a LONG means bearish
    // (negative) contributions; opposing a SHORT means bullish (positive) ones.
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
        // Realistic raws: ADX 30 (not congested), RVOL 2.0 (volume-confirmed).
        map.insert("adx".into(), niv_raw(30.0, 0.7, "STRONG_BULL_TREND"));
        map.insert("rvol".into(), niv_raw(2.0, 0.8, "INSTITUTIONAL_BREAKOUT_VOLUME"));
        // EMA stack with a long line below price so the 200-EMA factor is bullish.
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
        // A mid-range RSI produces a proportional, non-binary contribution.
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
        // Force raw adx below 20 to activate the congestion gate.
        map.insert(
            "adx".to_string(),
            NormalizedIndicatorValue::scalar(15.0, 0.0, "TRENDLESS_CONGESTION"),
        );
        let snap = SnapshotValues::from_map(map, 100.0);
        let score = calculate_eight_factor_score("BULLISH", &snap, &[], &[], "");
        // Trend factor is gated to ~0, so score stays near zero.
        assert_eq!(score.total_score, 0);
    }

    #[test]
    fn opposite_score_triggers_exit_above_threshold() {
        // Holding LONG while structure turns strongly bearish.
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
}
