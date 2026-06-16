use sqlx::SqlitePool;
use crate::db;

#[derive(Debug, Clone, serde::Serialize)]
pub struct DecisionScore {
    pub profile_name: String,
    pub score: i32,
    pub recommendation: String,
    pub momentum_bias: f32,
    pub indicator_results: Vec<IndicatorResult>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct IndicatorResult {
    pub indicator_name: String,
    pub signal: String,
    pub weight: i32,
    pub weighted_contribution: i32,
    pub override_active: bool,
}

pub struct SnapshotValues {
    pub rsi: Option<f64>,
    pub squeeze_on: Option<bool>,
    pub squeeze_momentum: Option<f64>,
    pub macd_line: Option<f64>,
    pub macd_signal: Option<f64>,
    pub macd_hist: Option<f64>,
    pub adx: Option<f64>,
    pub adx_plus: Option<f64>,
    pub adx_minus: Option<f64>,
    pub bb_upper: Option<f64>,
    pub bb_middle: Option<f64>,
    pub bb_lower: Option<f64>,
    pub atr: Option<f64>,
    pub ema_fast: Option<f64>,
    pub ema_medium: Option<f64>,
    pub ema_slow: Option<f64>,
    pub ema_long: Option<f64>,
    pub ema_stack_state: Option<String>,
    pub vwap: Option<f64>,
    pub vwap_bias: Option<String>,
    pub close: Option<f64>,
    pub volume: Option<f64>,
    pub average_volume: Option<f64>,
    pub rvol: Option<f64>,
    pub current_price: f64,
    pub rsi_divergence_status: Option<String>,
    pub macd_divergence_status: Option<String>,
    pub macd_trend_state: Option<String>,
    pub macd_crossover_detected: Option<bool>,
    pub macd_crossover_direction: Option<String>,
    pub macd_histogram_peak: Option<f64>,
    pub squeeze_duration: Option<u32>,
    pub squeeze_release_trigger: Option<bool>,
    pub squeeze_momentum_direction: Option<String>,
    pub chart_pattern: Option<String>,
    pub chart_pattern_confidence: Option<f64>,
    pub atr_volatility_regime: Option<String>,
    pub bbwp: Option<f64>,
    pub adx_slope: Option<f64>,
    pub adx_regime: Option<String>,
    pub adx_di_crossover_detected: Option<bool>,
    pub adx_di_crossover_direction: Option<String>,
}

pub async fn evaluate_profile(
    pool: &SqlitePool,
    profile_id: i64,
    snap: &SnapshotValues,
    _historical_prices: &[f64],
) -> DecisionScore {
    let profiles = db::decision_profiles_list(pool).await;
    let profile = match profiles.iter().find(|p| p.id == profile_id) {
        Some(p) => p.clone(),
        None => profiles.first().cloned().unwrap(),
    };

    let mut total_score: i32 = 0;
    let mut max_possible: i32 = 0;
    let mut indicator_results = Vec::new();

    for ind in &profile.indicators {
        let signal = evaluate_indicator_signal(ind.indicator_name.as_str(), snap);
        let override_active = ind.override_status != "NONE";
        let effective_signal = if override_active {
            ind.override_status.as_str()
        } else {
            signal
        };

        let contribution = match effective_signal {
            "BULLISH" => ind.weight,
            "BEARISH" => -ind.weight,
            _ => 0,
        };

        max_possible += ind.weight;
        total_score += contribution;

        indicator_results.push(IndicatorResult {
            indicator_name: ind.indicator_name.clone(),
            signal: effective_signal.to_string(),
            weight: ind.weight,
            weighted_contribution: contribution,
            override_active,
        });
    }

    let recommendation = if total_score >= profile.long_threshold {
        "BUY".to_string()
    } else if total_score <= profile.short_threshold {
        "SELL".to_string()
    } else {
        "WAIT".to_string()
    };

    let momentum_bias = if max_possible > 0 {
        (total_score as f32 / max_possible as f32) * 40.0
    } else {
        0.0
    };

    DecisionScore {
        profile_name: profile.profile_name,
        score: total_score,
        recommendation,
        momentum_bias,
        indicator_results,
    }
}

fn evaluate_indicator_signal(name: &str, snap: &SnapshotValues) -> &'static str {
    match name {
        "RSI (Oversold/Overbought)" => {
            match snap.rsi {
                Some(r) if r < 30.0 => "BULLISH",
                Some(r) if r > 70.0 => "BEARISH",
                _ => "SIDEWAYS",
            }
        }
        "RSI (Divergence)" => {
            match snap.rsi_divergence_status.as_deref() {
                Some("confirmed_bullish") | Some("potential_bullish") => "BULLISH",
                Some("confirmed_bearish") | Some("potential_bearish") => "BEARISH",
                _ => "SIDEWAYS",
            }
        }
        "MACD (Crossovers)" => {
            // Zero-line filtered crossover logic
            if snap.macd_crossover_detected.unwrap_or(false) {
                match snap.macd_crossover_direction.as_deref() {
                    Some("BULLISH") => {
                        // Valid only if macd_line < 0 (cross below zero)
                        if let Some(line) = snap.macd_line {
                            if line < 0.0 {
                                "BULLISH"
                            } else {
                                "SIDEWAYS" // Extreme high / above-zero crossover rejected
                            }
                        } else {
                            "SIDEWAYS"
                        }
                    }
                    Some("BEARISH") => {
                        // Valid only if macd_line > 0 (cross above zero)
                        if let Some(line) = snap.macd_line {
                            if line > 0.0 {
                                "BEARISH"
                            } else {
                                "SIDEWAYS" // Below-zero bearish crossover rejected
                            }
                        } else {
                            "SIDEWAYS"
                        }
                    }
                    _ => {
                        // No crossover, fall back to basic line comparison
                        match (snap.macd_line, snap.macd_signal) {
                            (Some(line), Some(sig)) if line > sig => "BULLISH",
                            (Some(line), Some(sig)) if line < sig => "BEARISH",
                            _ => "SIDEWAYS",
                        }
                    }
                }
            } else {
                // No crossover: standard line comparison
                match (snap.macd_line, snap.macd_signal) {
                    (Some(line), Some(sig)) if line > sig => "BULLISH",
                    (Some(line), Some(sig)) if line < sig => "BEARISH",
                    _ => "SIDEWAYS",
                }
            }
        }
        "MACD (Divergence)" => {
            match snap.macd_divergence_status.as_deref() {
                Some("confirmed_bullish") | Some("potential_bullish") => "BULLISH",
                Some("confirmed_bearish") | Some("potential_bearish") => "BEARISH",
                _ => "SIDEWAYS",
            }
        }
        "Support/Resistance" => {
            match (snap.bb_middle, snap.current_price) {
                (Some(bb), cp) if cp > bb => "BULLISH",
                (Some(bb), cp) if cp < bb => "BEARISH",
                _ => "SIDEWAYS",
            }
        }
        "Trend" => {
            match snap.ema_stack_state.as_deref() {
                Some("bullish") => "BULLISH",
                Some("bearish") => "BEARISH",
                _ => "SIDEWAYS",
            }
        }
        "ATR" => {
            match snap.atr_volatility_regime.as_deref() {
                Some("expanding") => "BULLISH",
                Some("contracting") => "SIDEWAYS",
                _ => "SIDEWAYS",
            }
        }
        "Patterns" => {
            // Use actual chart pattern detection result if available
            if snap.chart_pattern.is_some() && snap.chart_pattern.as_deref() != Some("None") {
                match snap.chart_pattern.as_deref() {
                    Some("FallingWedge") | Some("BullishTriangle") | Some("AscendingChannel") => "BULLISH",
                    Some("RisingWedge") | Some("BearishTriangle") | Some("DescendingChannel") => "BEARISH",
                    _ => "SIDEWAYS",
                }
            } else {
                // Fallback to squeeze momentum direction
                match snap.squeeze_momentum_direction.as_deref() {
                    Some("BullishAcceleration") | Some("BullishDeceleration") => "BULLISH",
                    Some("BearishAcceleration") | Some("BearishDeceleration") => "BEARISH",
                    _ => "SIDEWAYS",
                }
            }
        }
        "ADX" => {
            match snap.adx_regime.as_deref() {
                Some("congestion") => "SIDEWAYS",
                Some("extreme") => "SIDEWAYS",
                _ => {
                    if snap.adx_di_crossover_detected.unwrap_or(false) {
                        match snap.adx_di_crossover_direction.as_deref() {
                            Some("BULLISH") => "BULLISH",
                            Some("BEARISH") => "BEARISH",
                            _ => {
                                match (snap.adx_plus, snap.adx_minus) {
                                    (Some(p), Some(m)) if p > m => "BULLISH",
                                    (Some(p), Some(m)) if m > p => "BEARISH",
                                    _ => "SIDEWAYS",
                                }
                            }
                        }
                    } else {
                        match (snap.adx_plus, snap.adx_minus) {
                            (Some(p), Some(m)) if p > m => "BULLISH",
                            (Some(p), Some(m)) if m > p => "BEARISH",
                            _ => "SIDEWAYS",
                        }
                    }
                }
            }
        }
        "Volume" => {
            match snap.rvol {
                Some(r) if r >= 1.5 => "BULLISH",
                Some(r) if r < 1.0 => "BEARISH",
                _ => "SIDEWAYS",
            }
        }
        "BBWP" => {
            match snap.bbwp {
                Some(b) if b < 10.0 => "BULLISH",
                Some(b) if b > 90.0 => "BEARISH",
                _ => "SIDEWAYS",
            }
        }
        "VWAP" => {
            match snap.vwap_bias.as_deref() {
                Some("premium") => "BULLISH",
                Some("discount") => "BEARISH",
                _ => "SIDEWAYS",
            }
        }
        _ => "SIDEWAYS",
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
    // Zero-line filter: bullish valid only below zero, bearish only above zero
    let macd_crossover = match (snap.macd_crossover_detected, snap.macd_crossover_direction.as_deref()) {
        (Some(true), Some("BULLISH")) => {
            // Valid bullish: crossover below zero line AND RSI confluence
            let line_below_zero = snap.macd_line.map_or(false, |l| l < 0.0);
            let rsi_support = snap.rsi_divergence_status.as_deref()
                .map(|s| s == "confirmed_bullish" || s == "potential_bullish")
                .unwrap_or(false)
                || snap.rsi.map_or(false, |r| r > 30.0); // Exiting oversold
            is_bullish && line_below_zero && rsi_support
        }
        (Some(true), Some("BEARISH")) => {
            let line_above_zero = snap.macd_line.map_or(false, |l| l > 0.0);
            let rsi_support = snap.rsi_divergence_status.as_deref()
                .map(|s| s == "confirmed_bearish" || s == "potential_bearish")
                .unwrap_or(false)
                || snap.rsi.map_or(false, |r| r < 70.0); // Exiting overbought
            !is_bullish && line_above_zero && rsi_support
        }
        _ => {
            // No crossover: fall back to line comparison
            match (snap.macd_line, snap.macd_signal) {
                (Some(line), Some(sig)) if is_bullish && line > sig => true,
                (Some(line), Some(sig)) if !is_bullish && line < sig => true,
                _ => false,
            }
        }
    };
    let macd_points: i32 = if macd_crossover {
        // If contracting, reduce score by 1
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
        // Fallback to squeeze momentum direction
        match snap.squeeze_momentum_direction.as_deref() {
            Some("BullishAcceleration") if is_bullish => true,
            Some("BearishAcceleration") if !is_bullish => true,
            _ => false,
        }
    };
    let pattern_points: i32 = if pattern_aligned {
        if is_bullish { 10 } else { -10 }
    } else {
        // Deceleration while holding is an exit signal — give negative points
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
    // Congestion (<20): halve all trend-following allocation
    // Extreme (>40) with negative slope: block new entries entirely
    let regime_penalty_pct: f64 = match snap.adx_regime.as_deref() {
        Some("congestion") => 0.5,  // halve allocation
        Some("extreme") => {
            if snap.adx_slope.map_or(false, |s| s < 0.0) {
                0.0  // block entirely — exhaustion hook
            } else {
                0.3  // severe penalty for extreme regime
            }
        }
        Some("emerging") => 0.7,  // reduced confidence
        _ => 1.0,  // strong or unclassified — full allocation
    };

    // ─── RVOL Volume Confirmation Gate ─────────────────────
    // Breakout/momentum signals require RVOL ≥ 1.5 for full score
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
    // Expanding volatility favors breakout signals; contracting penalizes them
    let atr_boost = match snap.atr_volatility_regime.as_deref() {
        Some("expanding") => {
            // Boost breakout-adjacent signals: squeeze patterns, trend
            let breakout_points = pattern_points.abs() + trend_points.abs();
            ((breakout_points as f64) * 0.1) as i32
        }
        Some("contracting") => {
            // Penalize breakouts, slight boost to mean-reverting indicators
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
/// Returns the absolute score count of opposing signals.
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

// ─── Market Regime Classification ──────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MarketRegime {
    Trending,
    Compression,
    Expansion,
    Range,
}

impl MarketRegime {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Trending => "TRENDING",
            Self::Compression => "COMPRESSION",
            Self::Expansion => "EXPANSION",
            Self::Range => "RANGE",
        }
    }
}

pub fn classify_market_regime(snap: &SnapshotValues) -> MarketRegime {
    let adx = snap.adx.unwrap_or(0.0);
    let bbwp = snap.bbwp.unwrap_or(50.0);
    let squeeze_on = snap.squeeze_on.unwrap_or(false);

    if bbwp < 10.0 || squeeze_on {
        return MarketRegime::Compression;
    }

    if snap.squeeze_release_trigger.unwrap_or(false)
        || (bbwp > 90.0 && snap.atr_volatility_regime.as_deref() == Some("expanding"))
    {
        return MarketRegime::Expansion;
    }

    if adx >= 25.0 && snap.ema_stack_state.as_deref() != Some("tangled") {
        return MarketRegime::Trending;
    }

    MarketRegime::Range
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MtfTrendAlignment {
    pub short_aligned: bool,
    pub mid_aligned: bool,
    pub macro_aligned: bool,
    pub structural_trend: String,
}

pub fn evaluate_mtf_alignment(
    short: &SnapshotValues,
    mid: &SnapshotValues,
    long: &SnapshotValues,
    macro_snap: &SnapshotValues,
    super_snap: &SnapshotValues,
) -> MtfTrendAlignment {
    let structural_trend = match (super_snap.ema_long, super_snap.close) {
        (Some(ema), Some(close)) if close > ema => "BULLISH".to_string(),
        (Some(ema), Some(close)) if close < ema => "BEARISH".to_string(),
        _ => "NEUTRAL".to_string(),
    };

    let short_aligned = short.ema_stack_state == mid.ema_stack_state;
    let mid_aligned = mid.ema_stack_state == long.ema_stack_state;
    let macro_aligned = long.ema_stack_state == macro_snap.ema_stack_state;

    MtfTrendAlignment {
        short_aligned,
        mid_aligned,
        macro_aligned,
        structural_trend,
    }
}
