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
    pub vwap: Option<f64>,
    pub close: Option<f64>,
    pub volume: Option<f64>,
    pub average_volume: Option<f64>,
    pub current_price: f64,
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
            match snap.rsi {
                Some(r) if r < 40.0 => "BULLISH",
                Some(r) if r > 60.0 => "BEARISH",
                _ => "SIDEWAYS",
            }
        }
        "MACD (Crossovers)" => {
            match (snap.macd_line, snap.macd_signal) {
                (Some(line), Some(sig)) if line > sig => "BULLISH",
                (Some(line), Some(sig)) if line < sig => "BEARISH",
                _ => "SIDEWAYS",
            }
        }
        "MACD (Divergence)" => {
            match snap.macd_hist {
                Some(h) if h > 0.0 => "BULLISH",
                Some(h) if h < 0.0 => "BEARISH",
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
            let ema_bullish = snap.ema_fast.zip(snap.ema_slow).map(|(f, s)| f > s).unwrap_or(false);
            let bb_bullish = snap.current_price > snap.bb_middle.unwrap_or(0.0);
            if ema_bullish && bb_bullish {
                "BULLISH"
            } else if bb_bullish {
                "BULLISH"
            } else {
                "BEARISH"
            }
        }
        "Patterns" => {
            match (snap.squeeze_on, snap.squeeze_momentum) {
                (Some(true), Some(m)) if m > 0.0 => "BULLISH",
                (Some(true), Some(m)) if m < 0.0 => "BEARISH",
                _ => "SIDEWAYS",
            }
        }
        _ => "SIDEWAYS",
    }
}
