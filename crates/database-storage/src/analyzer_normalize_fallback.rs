//! Fallback normalizer for the snapshot DB reconstruction path.
//!
//! When a `MarketSnapshot` row was inserted before the `aux_json` migration
//! landed (or has NULL `aux_json` for any reason), we still need to
//! populate the `indicators` map so downstream HTTP responses carry a
//! usable shape. This is a *subset* of the full
//! `market_analyzer::analyzer::normalize::build_indicator_map_from_scalars`
//! logic — enough to reconstruct the 8 primary scored indicators from the
//! legacy per-row scalar columns.
//!
//! Lives here (rather than in `market-analyzer`) to break the
//! `database-storage <-> market-analyzer` dependency cycle.

use core_domain::indicator_dtos::NormalizedIndicatorValue;
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct RawScalarInputs {
    pub close: f64,
    pub rsi: f64,
    pub macd_line: f64,
    pub macd_signal: f64,
    pub macd_hist: f64,
    pub adx: f64,
    pub adx_plus_di: f64,
    pub adx_minus_di: f64,
    pub bbwp: f64,
    pub squeeze: f64,
    pub atr: f64,
    pub ema_fast: f64,
    pub ema_medium: f64,
    pub ema_slow: f64,
    pub ema_long: f64,
    pub vwap: f64,
    pub rvol: f64,
    pub stoch_k: f64,
    pub stoch_d: f64,
    pub chandemo: f64,
    pub obv: f64,
    pub cmf: f64,
    pub mfi: f64,
    pub hv: f64,
    pub aroon_up: f64,
    pub aroon_down: f64,
    pub choppiness: f64,
}

pub fn build_indicator_map_from_scalars(s: RawScalarInputs) -> HashMap<String, NormalizedIndicatorValue> {
    let mut m: HashMap<String, NormalizedIndicatorValue> = HashMap::new();
    m.insert(
        "rsi".into(),
        NormalizedIndicatorValue::scalar(s.rsi, (50.0 - s.rsi) / 50.0, "RECONSTRUCTED"),
    );
    let mut macd_vals = HashMap::new();
    macd_vals.insert("line".into(), s.macd_line);
    macd_vals.insert("signal".into(), s.macd_signal);
    macd_vals.insert("histogram".into(), s.macd_hist);
    m.insert(
        "macd".into(),
        NormalizedIndicatorValue::with_values(
            s.macd_hist,
            s.macd_hist.signum(),
            "RECONSTRUCTED",
            macd_vals,
        ),
    );
    let mut adx_vals = HashMap::new();
    adx_vals.insert("adx".into(), s.adx);
    adx_vals.insert("plus_di".into(), s.adx_plus_di);
    adx_vals.insert("minus_di".into(), s.adx_minus_di);
    m.insert(
        "adx".into(),
        NormalizedIndicatorValue::with_values(s.adx, 0.0, "RECONSTRUCTED", adx_vals),
    );
    m.insert(
        "bbwp".into(),
        NormalizedIndicatorValue::scalar(s.bbwp, 0.0, "RECONSTRUCTED"),
    );
    m.insert(
        "squeeze".into(),
        NormalizedIndicatorValue::scalar(s.squeeze, s.squeeze.signum(), "RECONSTRUCTED"),
    );
    let mut atr_vals = HashMap::new();
    atr_vals.insert("atr_14".into(), s.atr);
    m.insert(
        "atr".into(),
        NormalizedIndicatorValue::with_values(s.atr, 0.0, "RECONSTRUCTED", atr_vals),
    );
    let mut ema_vals = HashMap::new();
    ema_vals.insert("fast".into(), s.ema_fast);
    ema_vals.insert("medium".into(), s.ema_medium);
    ema_vals.insert("slow".into(), s.ema_slow);
    ema_vals.insert("long".into(), s.ema_long);
    let ema_norm = if s.close > 0.0 && s.ema_long > 0.0 {
        ((s.close - s.ema_long) / s.ema_long).clamp(-1.0, 1.0)
    } else {
        0.0
    };
    m.insert(
        "ema_stack".into(),
        NormalizedIndicatorValue::with_values(s.close, ema_norm, "RECONSTRUCTED", ema_vals),
    );
    m.insert(
        "vwap".into(),
        NormalizedIndicatorValue::scalar(s.vwap, 0.0, "RECONSTRUCTED"),
    );
    m.insert(
        "rvol".into(),
        NormalizedIndicatorValue::scalar(s.rvol, 0.0, "RECONSTRUCTED"),
    );
    m
}
