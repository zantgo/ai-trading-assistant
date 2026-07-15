pub mod scoring;

pub use scoring::{calculate_registry_confluence, RegistryConfluence};

use crate::db;
use sqlx::SqlitePool;

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

use shared::indicators::normalized::{
    DivergenceState, NormalizationContext, NormalizationEngine, NormalizedIndicatorValue,
};
use std::collections::HashMap;

/// Continuous-scale view of a market snapshot. The nested normalized indicator
/// map is the single source of truth; convenience getters expose each of the
/// indicators by string key. `current_price` is retained as non-indicator
/// context needed for structural distance checks.
pub struct SnapshotValues {
    pub indicators: HashMap<String, NormalizedIndicatorValue>,
    pub current_price: f64,
}

impl SnapshotValues {
    /// Construct from an already-computed normalized indicator map.
    pub fn from_map(
        indicators: HashMap<String, NormalizedIndicatorValue>,
        current_price: f64,
    ) -> Self {
        Self {
            indicators,
            current_price,
        }
    }

    /// Fetch an indicator entry, or a neutral `UNKNOWN` default when missing.
    pub fn ind(&self, key: &str) -> NormalizedIndicatorValue {
        self.indicators
            .get(key)
            .cloned()
            .unwrap_or_else(|| NormalizedIndicatorValue::scalar(0.0, 0.0, "UNKNOWN"))
    }

    /// Normalized `[-1.0, 1.0]` score for an indicator (0.0 when missing).
    pub fn norm(&self, key: &str) -> f64 {
        self.indicators
            .get(key)
            .map(|v| v.normalized)
            .unwrap_or(0.0)
    }

    /// Context-aware state label for an indicator ("UNKNOWN" when missing).
    pub fn label(&self, key: &str) -> String {
        self.indicators
            .get(key)
            .map(|v| v.state_label.clone())
            .unwrap_or_else(|| "UNKNOWN".to_string())
    }

    /// Primary raw scalar for an indicator.
    pub fn raw(&self, key: &str) -> Option<f64> {
        self.indicators.get(key).map(|v| v.raw_value)
    }

    /// Auxiliary raw sub-component (e.g. macd `line`, ema_stack `long`).
    pub fn sub(&self, key: &str, sub: &str) -> Option<f64> {
        self.indicators
            .get(key)
            .and_then(|v| v.values.as_ref())
            .and_then(|m| m.get(sub))
            .copied()
    }
}

/// Bridge: build a [`SnapshotValues`] directly from an already-computed
/// normalized indicator map (e.g. from a `MarketSnapshot`).
pub fn indicator_to_snapshot_values(
    indicators: &HashMap<String, NormalizedIndicatorValue>,
    current_price: f64,
) -> SnapshotValues {
    SnapshotValues::from_map(indicators.clone(), current_price)
}

/// Map a divergence status string (as carried on the flat server
/// `IndicatorSnapshot`/`EvaluateRequest`) to a [`DivergenceState`].
fn parse_divergence(status: Option<&str>) -> DivergenceState {
    match status {
        Some("confirmed_bullish") => DivergenceState::ConfirmedBullish,
        Some("potential_bullish") => DivergenceState::PotentialBullish,
        Some("confirmed_bearish") => DivergenceState::ConfirmedBearish,
        Some("potential_bearish") => DivergenceState::PotentialBearish,
        _ => DivergenceState::None,
    }
}

/// Bridge: build a [`SnapshotValues`] from a nested server
/// [`IndicatorSnapshot`] (which already carries the normalized map).
pub fn snapshot_values_from_flat(snap: &crate::server::types::IndicatorSnapshot) -> SnapshotValues {
    SnapshotValues::from_map(snap.indicators.clone(), snap.current_price.unwrap_or(0.0))
}

/// Bridge: reconstruct a normalized indicator map from the flat
/// [`EvaluateRequest`] payload (frontend decision-profile evaluator),
/// preserving divergence, crossover, squeeze direction, and pattern context so
/// the continuous scoring engine is fully fed.
pub fn snapshot_values_from_evaluate(
    req: &crate::server::types::EvaluateRequest,
) -> SnapshotValues {
    use shared::indicators::squeeze::MomentumDirection;
    use shared::indicators::IndicatorInputs;

    let current_price = req.current_price.or(req.close).unwrap_or(0.0);

    let squeeze_direction = match req.squeeze_momentum_direction.as_deref() {
        Some("BullishAcceleration") => Some(MomentumDirection::BullishAcceleration),
        Some("BullishDeceleration") => Some(MomentumDirection::BullishDeceleration),
        Some("BearishAcceleration") => Some(MomentumDirection::BearishAcceleration),
        Some("BearishDeceleration") => Some(MomentumDirection::BearishDeceleration),
        _ => Some(MomentumDirection::Flat),
    };
    let macd_crossover = if req.macd_crossover_detected.unwrap_or(false) {
        match req.macd_crossover_direction.as_deref() {
            Some("BULLISH") => Some(1i8),
            Some("BEARISH") => Some(-1i8),
            _ => None,
        }
    } else {
        None
    };
    let pattern_bullish = matches!(
        req.chart_pattern.as_deref(),
        Some("FallingWedge")
            | Some("BullishTriangle")
            | Some("AscendingChannel")
            | Some("BullishPattern")
    );
    let pattern_bearish = matches!(
        req.chart_pattern.as_deref(),
        Some("RisingWedge")
            | Some("BearishTriangle")
            | Some("DescendingChannel")
            | Some("BearishPattern")
    );

    let inputs = IndicatorInputs {
        rsi: req.rsi,
        rsi_divergence: parse_divergence(req.rsi_divergence_status.as_deref()),
        macd_divergence: parse_divergence(req.macd_divergence_status.as_deref()),
        macd_line: req.macd_line,
        macd_signal: req.macd_signal,
        macd_histogram: req.macd_hist,
        macd_histogram_peak: req.macd_histogram_peak,
        macd_crossover,
        squeeze_on: req.squeeze_on,
        squeeze_release_trigger: req.squeeze_release_trigger.unwrap_or(false),
        squeeze_momentum: req.squeeze_momentum,
        squeeze_direction,
        adx: req.adx,
        adx_plus_di: req.adx_plus,
        adx_minus_di: req.adx_minus,
        adx_slope: req.adx_slope,
        bbwp: req.bbwp,
        rvol: req.rvol,
        vwap: req.vwap,
        pattern_bullish,
        pattern_bearish,
        pattern_confidence: req.chart_pattern_confidence,
        atr_14: req.atr,
        bb_upper: req.bb_upper,
        bb_middle: req.bb_middle,
        bb_lower: req.bb_lower,
        ..Default::default()
    };

    let trend_bias = match req.ema_stack_state.as_deref() {
        Some("bullish") => 1i8,
        Some("bearish") => -1i8,
        _ => 0i8,
    };
    let ctx = NormalizationContext {
        trend_bias,
        price: current_price,
        vwap: req.vwap,
        ema_stack_state: req.ema_stack_state.clone(),
        ema_medium: req.ema_medium,
        rvol: req.rvol,
        ..Default::default()
    };

    let mut map = NormalizationEngine::normalize_all(&inputs, &ctx);
    if let Some(entry) = map.get_mut("ema_stack") {
        let mut vals = entry.values.take().unwrap_or_default();
        for (k, v) in [
            ("fast", req.ema_fast),
            ("medium", req.ema_medium),
            ("slow", req.ema_slow),
            ("long", req.ema_long),
        ] {
            if let Some(val) = v {
                vals.insert(k.to_string(), val);
            }
        }
        entry.values = Some(vals);
    }

    SnapshotValues::from_map(map, current_price)
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

/// Classify a continuous normalized value into a directional signal.
fn sign_signal(norm: f64) -> &'static str {
    if norm > 0.10 {
        "BULLISH"
    } else if norm < -0.10 {
        "BEARISH"
    } else {
        "SIDEWAYS"
    }
}

fn evaluate_indicator_signal(name: &str, snap: &SnapshotValues) -> &'static str {
    // Map each decision-profile indicator name to its normalized map key and
    // classify by sign against the continuous `[-1.0, 1.0]` scale.
    let key = match name {
        "RSI (Oversold/Overbought)" => "rsi",
        "RSI (Divergence)" => "rsi_divergence",
        "MACD (Crossovers)" => "macd",
        "MACD (Divergence)" => "macd_divergence",
        "Support/Resistance" => "support_resistance",
        "Trend" => "ema_stack",
        "Patterns" => "patterns",
        "ADX" => "adx",
        "Volume" => "rvol",
        "BBWP" => "bbwp",
        "VWAP" => "vwap",
        "Stochastic" => "stochastic",
        "Chande MO" => "chandemo",
        "Supertrend" => "supertrend",
        "Keltner" => "keltner",
        "Donchian" => "donchian",
        "OBV" => "obv",
        "Chaikin MF" => "cmf",
        "Money Flow Index" => "mfi",
        "Aroon" => "aroon",
        "LinReg Slope" => "linreg_slope",
        "Z-Score" => "zscore",
        "ATR" => return "SIDEWAYS", // non-directional volatility gauge
        "Hist. Volatility" => return "SIDEWAYS",
        "Choppiness" => return "SIDEWAYS", // non-directional regime gate
        _ => return "SIDEWAYS",
    };
    sign_signal(snap.norm(key))
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
    let adx = snap.raw("adx").unwrap_or(0.0);
    let bbwp = snap.raw("bbwp").unwrap_or(50.0);
    let squeeze_label = snap.label("squeeze");
    let squeeze_on = squeeze_label == "COMPRESSION_COILING";
    let squeeze_release = squeeze_label.ends_with("VOLATILITY_RELEASE");
    let tangled =
        snap.label("ema_stack").contains("TANGLED") || snap.norm("ema_stack").abs() < 0.10;

    if bbwp < 10.0 || squeeze_on {
        return MarketRegime::Compression;
    }

    if squeeze_release || bbwp > 90.0 {
        return MarketRegime::Expansion;
    }

    if adx >= 25.0 && !tangled {
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

/// Bucket the EMA-stack normalized value into a coarse trend direction.
fn ema_bucket(snap: &SnapshotValues) -> i8 {
    let n = snap.norm("ema_stack");
    if n > 0.10 {
        1
    } else if n < -0.10 {
        -1
    } else {
        0
    }
}

pub fn evaluate_mtf_alignment(
    micro: &SnapshotValues,
    fast: &SnapshotValues,
    slow_snap: &SnapshotValues,
    macro_snap: &SnapshotValues,
) -> MtfTrendAlignment {
    let structural_trend = match (
        macro_snap.sub("ema_stack", "long"),
        macro_snap.current_price,
    ) {
        (Some(ema), close) if close > ema => "BULLISH".to_string(),
        (Some(ema), close) if close < ema => "BEARISH".to_string(),
        _ => "NEUTRAL".to_string(),
    };

    let micro_aligned = ema_bucket(micro) == ema_bucket(fast);
    let slow_aligned = ema_bucket(fast) == ema_bucket(slow_snap);

    MtfTrendAlignment {
        micro_aligned,
        slow_aligned,
        structural_trend,
    }
}
