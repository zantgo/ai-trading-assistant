pub mod scoring;

pub use scoring::{calculate_eight_factor_score, calculate_opposite_score, EightFactorScore};

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

pub fn indicator_to_snapshot_values(
    snap: &crate::server::types::IndicatorSnapshot,
) -> SnapshotValues {
    SnapshotValues {
        rsi: snap.rsi, squeeze_on: snap.squeeze_on, squeeze_momentum: snap.squeeze_momentum,
        squeeze_duration: snap.squeeze_duration, squeeze_release_trigger: snap.squeeze_release_trigger,
        squeeze_momentum_direction: snap.squeeze_momentum_direction.clone(),
        chart_pattern: snap.chart_pattern.clone(), chart_pattern_confidence: snap.chart_pattern_confidence,
        bbwp: snap.bbwp, macd_line: snap.macd_line, macd_signal: snap.macd_signal,
        macd_hist: snap.macd_histogram, adx: snap.adx, adx_plus: snap.adx_plus,
        adx_minus: snap.adx_minus, bb_upper: snap.bb_upper, bb_middle: snap.bb_middle,
        bb_lower: snap.bb_lower, atr: snap.atr,
        ema_fast: snap.ema_fast, ema_medium: snap.ema_medium, ema_slow: snap.ema_slow,
        ema_long: snap.ema_long, ema_stack_state: snap.ema_stack_state.clone(),
        vwap: snap.vwap, vwap_bias: snap.vwap_bias.clone(),
        close: snap.current_price, volume: snap.volume, average_volume: snap.average_volume,
        rvol: snap.rvol, current_price: snap.current_price.unwrap_or(0.0),
        rsi_divergence_status: None, macd_divergence_status: None,
        macd_trend_state: snap.macd_trend_state.clone(),
        macd_crossover_detected: snap.macd_crossover_detected,
        macd_crossover_direction: snap.macd_crossover_direction.clone(),
        macd_histogram_peak: snap.macd_histogram_peak,
        atr_volatility_regime: snap.atr_volatility_regime.clone(),
        adx_slope: None, adx_regime: None,
        adx_di_crossover_detected: None, adx_di_crossover_direction: None,
    }
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
            if snap.macd_crossover_detected.unwrap_or(false) {
                match snap.macd_crossover_direction.as_deref() {
                    Some("BULLISH") => {
                        if let Some(line) = snap.macd_line {
                            if line < 0.0 { "BULLISH" } else { "SIDEWAYS" }
                        } else { "SIDEWAYS" }
                    }
                    Some("BEARISH") => {
                        if let Some(line) = snap.macd_line {
                            if line > 0.0 { "BEARISH" } else { "SIDEWAYS" }
                        } else { "SIDEWAYS" }
                    }
                    _ => {
                        match (snap.macd_line, snap.macd_signal) {
                            (Some(line), Some(sig)) if line > sig => "BULLISH",
                            (Some(line), Some(sig)) if line < sig => "BEARISH",
                            _ => "SIDEWAYS",
                        }
                    }
                }
            } else {
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
            if snap.chart_pattern.is_some() && snap.chart_pattern.as_deref() != Some("None") {
                match snap.chart_pattern.as_deref() {
                    Some("FallingWedge") | Some("BullishTriangle") | Some("AscendingChannel") => "BULLISH",
                    Some("RisingWedge") | Some("BearishTriangle") | Some("DescendingChannel") => "BEARISH",
                    _ => "SIDEWAYS",
                }
            } else {
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
    pub micro_aligned: bool,
    pub slow_aligned: bool,
    pub structural_trend: String,
}

pub fn evaluate_mtf_alignment(
    micro: &SnapshotValues,
    fast: &SnapshotValues,
    slow_snap: &SnapshotValues,
    macro_snap: &SnapshotValues,
) -> MtfTrendAlignment {
    let structural_trend = match (macro_snap.ema_long, macro_snap.close) {
        (Some(ema), Some(close)) if close > ema => "BULLISH".to_string(),
        (Some(ema), Some(close)) if close < ema => "BEARISH".to_string(),
        _ => "NEUTRAL".to_string(),
    };

    let micro_aligned = micro.ema_stack_state == fast.ema_stack_state;
    let slow_aligned = fast.ema_stack_state == slow_snap.ema_stack_state;

    MtfTrendAlignment {
        micro_aligned,
        slow_aligned,
        structural_trend,
    }
}
