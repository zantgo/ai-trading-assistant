use super::SnapshotValues;

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

/// Calculate the 8-factor point score for a given trading bias using
/// weighted neural-style scoring per Section 3.1.
///
/// Weights: RSI=10(±1), RSI Div=20(±2), MACD=10(±1), MACD Div=10(±1),
///          S/R=10(±1), Trend=20(±2), 200EMA=10(±1), Patterns=10(±1).
/// Total possible: ±90.
///
/// Allocation: <40→1%, 40–59→2%, ≥60→3%.
pub fn calculate_eight_factor_score(
    bias: &str,
    snap: &SnapshotValues,
    support_levels: &[f64],
    resistance_levels: &[f64],
    _macro_trend: &str,
) -> EightFactorScore {
    let is_bullish = bias == "BULLISH";

    // ─── 1. RSI (Overbought / Oversold) — Weight 10, ±1 ───
    let rsi_aligned = match snap.rsi {
        Some(r) if is_bullish && r < 30.0 => true,
        Some(r) if !is_bullish && r > 70.0 => true,
        _ => false,
    };
    let rsi_points: i32 = if rsi_aligned { if is_bullish { 10 } else { -10 } } else { 0 };

    // ─── 2. RSI Divergence — Weight 20, ±2 ───
    let is_bullish_rsi_div = matches!(snap.rsi_divergence_status.as_deref(),
        Some("confirmed_bullish") | Some("potential_bullish"));
    let is_bearish_rsi_div = matches!(snap.rsi_divergence_status.as_deref(),
        Some("confirmed_bearish") | Some("potential_bearish"));
    let rsi_divergence_aligned = if is_bullish { is_bullish_rsi_div } else { is_bearish_rsi_div };
    let rsi_divergence_points: i32 = if rsi_divergence_aligned { if is_bullish { 20 } else { -20 } } else { 0 };

    // ─── 3. MACD Crossover — Weight 10, ±1 ───
    let macd_crossover = match (snap.macd_crossover_detected, snap.macd_crossover_direction.as_deref()) {
        (Some(true), Some("BULLISH")) => {
            let line_below_zero = snap.macd_line.map_or(false, |l| l < 0.0);
            let rsi_support = snap.rsi_divergence_status.as_deref()
                .map(|s| s == "confirmed_bullish" || s == "potential_bullish")
                .unwrap_or(false)
                || snap.rsi.map_or(false, |r| r > 30.0);
            is_bullish && line_below_zero && rsi_support
        }
        (Some(true), Some("BEARISH")) => {
            let line_above_zero = snap.macd_line.map_or(false, |l| l > 0.0);
            let rsi_support = snap.rsi_divergence_status.as_deref()
                .map(|s| s == "confirmed_bearish" || s == "potential_bearish")
                .unwrap_or(false)
                || snap.rsi.map_or(false, |r| r < 70.0);
            !is_bullish && line_above_zero && rsi_support
        }
        _ => {
            match (snap.macd_line, snap.macd_signal) {
                (Some(line), Some(sig)) if is_bullish && line > sig => true,
                (Some(line), Some(sig)) if !is_bullish && line < sig => true,
                _ => false,
            }
        }
    };
    let macd_points: i32 = if macd_crossover {
        let contracting = snap.macd_trend_state.as_deref() == Some("decelerating");
        if contracting {
            if is_bullish { 9 } else { -9 }
        } else {
            if is_bullish { 10 } else { -10 }
        }
    } else { 0 };

    // ─── 4. MACD Divergence — Weight 10, ±1 ───
    let is_bullish_macd_div = matches!(snap.macd_divergence_status.as_deref(),
        Some("confirmed_bullish") | Some("potential_bullish"));
    let is_bearish_macd_div = matches!(snap.macd_divergence_status.as_deref(),
        Some("confirmed_bearish") | Some("potential_bearish"));
    let macd_divergence_aligned = if is_bullish { is_bullish_macd_div } else { is_bearish_macd_div };
    let macd_divergence_points: i32 = if macd_divergence_aligned { if is_bullish { 10 } else { -10 } } else { 0 };

    // ─── 5. Support / Resistance — Weight 10, ±1 ───
    let support_aligned = support_levels.iter().any(|&s| {
        let dist = (snap.current_price - s).abs();
        dist < s * 0.005
    });
    let resistance_aligned = resistance_levels.iter().any(|&r| {
        let dist = (snap.current_price - r).abs();
        dist < r * 0.005
    });
    let sr_aligned = if is_bullish { support_aligned } else { resistance_aligned };
    let support_resistance_points: i32 = if sr_aligned { if is_bullish { 10 } else { -10 } } else { 0 };

    // ─── 6. Trend (EMA stack check) — Weight 20, ±2 ───
    let trend_aligned = match snap.ema_stack_state.as_deref() {
        Some("bullish") if is_bullish => true,
        Some("bearish") if !is_bullish => true,
        _ => false,
    };
    let trend_points: i32 = if trend_aligned { if is_bullish { 20 } else { -20 } } else { 0 };

    // ─── 7. 200 EMA Position — Weight 10, ±1 ───
    let ema200_aligned = match snap.ema_long {
        Some(e) if is_bullish && snap.current_price > e => true,
        Some(e) if !is_bullish && snap.current_price < e => true,
        _ => false,
    };
    let ema200_points: i32 = if ema200_aligned { if is_bullish { 10 } else { -10 } } else { 0 };

    // ─── 8. Chart Patterns — Weight 10, ±1 ───
    let pattern_aligned = if snap.chart_pattern.is_some() && snap.chart_pattern.as_deref() != Some("None") {
        match snap.chart_pattern.as_deref() {
            Some(p) if is_bullish && (p == "FallingWedge" || p == "BullishTriangle" || p == "AscendingChannel") => true,
            Some(p) if !is_bullish && (p == "RisingWedge" || p == "BearishTriangle" || p == "DescendingChannel") => true,
            _ => false,
        }
    } else {
        match snap.squeeze_momentum_direction.as_deref() {
            Some("BullishAcceleration") if is_bullish => true,
            Some("BearishAcceleration") if !is_bullish => true,
            _ => false,
        }
    };
    let pattern_points: i32 = if pattern_aligned {
        if is_bullish { 10 } else { -10 }
    } else {
        match snap.squeeze_momentum_direction.as_deref() {
            Some("BullishDeceleration") if is_bullish => { -5 }
            Some("BearishDeceleration") if !is_bullish => { 5 }
            _ => 0,
        }
    };

    let signals = EightFactorSignals {
        rsi_aligned,
        rsi_divergence_aligned,
        macd_crossover,
        macd_divergence_aligned,
        support_aligned,
        resistance_aligned,
        trend_aligned,
        ema200_aligned,
        pattern_aligned,
    };

    // ─── ADX Regime Gate ───────────────────────────────────
    let regime_penalty_pct: f64 = match snap.adx_regime.as_deref() {
        Some("congestion") => 0.5,
        Some("extreme") => {
            if snap.adx_slope.map_or(false, |s| s < 0.0) {
                0.0
            } else {
                0.3
            }
        }
        Some("emerging") => 0.7,
        _ => 1.0,
    };

    // ─── RVOL Volume Confirmation Gate ─────────────────────
    let rvol = snap.rvol.unwrap_or(1.0);
    let has_breakout_signal = pattern_aligned || macd_crossover || sr_aligned;
    let rvol_penalty: f64 = if has_breakout_signal {
        if rvol < 1.0 { 0.3 }
        else if rvol < 1.5 { 0.6 }
        else { 1.0 }
    } else {
        1.0
    };

    // ─── BBWP Volatility Percentile Adjustment ──────────────
    let bbwp_multiplier: f64 = match snap.bbwp {
        Some(b) if b < 10.0 => 1.2,
        Some(b) if b > 90.0 => 0.5,
        _ => 1.0,
    };

    let total_score = ((rsi_points
        + rsi_divergence_points
        + macd_points
        + macd_divergence_points
        + support_resistance_points
        + trend_points
        + ema200_points
        + pattern_points) as f64 * regime_penalty_pct * rvol_penalty * bbwp_multiplier) as i32;

    // ─── ATR Volatility Regime Boost ─────────────────────────
    let atr_boost = match snap.atr_volatility_regime.as_deref() {
        Some("expanding") => {
            let breakout_points = pattern_points.abs() + trend_points.abs();
            ((breakout_points as f64) * 0.1) as i32
        }
        Some("contracting") => {
            let breakout_points = pattern_points.abs() + trend_points.abs();
            -((breakout_points as f64) * 0.2) as i32
        }
        _ => 0,
    };

    let adjusted_score = total_score + atr_boost;

    let abs_score = adjusted_score.abs() as u32;
    let allocated_capital_pct = match abs_score {
        0..=39 => 1.0,
        40..=59 => 2.0,
        _ => 3.0,
    };

    EightFactorScore {
        total_score: adjusted_score,
        max_score: 90,
        signals,
        allocated_capital_pct,
        weighted_contributions: EightFactorContributions {
            rsi_points,
            rsi_divergence_points,
            macd_points,
            macd_divergence_points,
            support_resistance_points,
            trend_points,
            ema200_points,
            pattern_points,
        },
    }
}

/// Calculate the opposite-signal score (signals against current position).
/// Used to determine if an opposite-signal exit should trigger.
pub fn calculate_opposite_score(
    position_direction: &str,
    snap: &SnapshotValues,
    support_levels: &[f64],
    resistance_levels: &[f64],
    macro_trend: &str,
) -> u32 {
    let opposite_bias = if position_direction == "LONG" { "BEARISH" } else { "BULLISH" };
    let score = calculate_eight_factor_score(opposite_bias, snap, support_levels, resistance_levels, macro_trend);
    score.total_score.abs() as u32
}
