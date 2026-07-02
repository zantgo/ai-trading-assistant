//! Context-dependent normalization mappers.
//!
//! These mappers require directional/positional context (trend bias, price
//! vs VWAP, DI crossover sign, active position, RVOL) supplied via
//! [`NormalizationContext`]. They remain pure/static and side-effect free.

use super::{clamp_unit, pick, NormalizationContext, NormalizationEngine, NormalizedIndicatorValue};
use std::collections::HashMap;

impl NormalizationEngine {
    /// ADX: non-directional strength signed by the +DI/-DI crossover bias.
    pub fn normalize_adx(
        adx: f64,
        plus_di: f64,
        minus_di: f64,
        adx_slope: f64,
        consecutive_deceleration: bool,
    ) -> NormalizedIndicatorValue {
        let sign = if plus_di >= minus_di { 1.0 } else { -1.0 };
        let (norm, label): (f64, String) = if adx < 20.0 {
            (0.0, "TRENDLESS_CONGESTION".into())
        } else if adx > 40.0 && consecutive_deceleration {
            // Hard hook exit on 2-bar consecutive deceleration of extreme trend.
            (
                sign * 0.10,
                if sign > 0.0 {
                    "BULL_TREND_EXHAUSTION_HOOK".into()
                } else {
                    "BEAR_TREND_EXHAUSTION_HOOK".into()
                },
            )
        } else if adx <= 25.0 {
            (
                sign * 0.30,
                pick(sign, "EMERGING_BULL_TREND", "EMERGING_BEAR_TREND"),
            )
        } else if adx <= 40.0 {
            let mag = 0.50 + ((adx - 25.0) / 15.0) * 0.30;
            (
                sign * mag,
                pick(sign, "STRONG_BULL_TREND", "STRONG_BEAR_TREND"),
            )
        } else {
            let mag = 0.90 + ((adx - 40.0) / 20.0).min(1.0) * 0.10;
            (
                sign * mag,
                pick(sign, "CLIMACTIC_BULL_TREND", "CLIMACTIC_BEAR_TREND"),
            )
        };

        let mut values = HashMap::new();
        values.insert("adx".to_string(), adx);
        values.insert("plus_di".to_string(), plus_di);
        values.insert("minus_di".to_string(), minus_di);
        values.insert("adx_slope".to_string(), adx_slope);
        NormalizedIndicatorValue::with_values(adx, clamp_unit(norm), label, values)
    }

    /// BBWP: volatility width percentile signed by prevailing bias.
    pub fn normalize_bbwp(bbwp: f64, bias: i8) -> NormalizedIndicatorValue {
        if bbwp < 10.0 {
            return NormalizedIndicatorValue::scalar(bbwp, 0.0, "MAX_VOLATILITY_COMPRESSION");
        }
        if bbwp > 90.0 {
            // Volatility exhaustion climax → mean-reversion penalty against bias.
            let norm = -0.1 * bias as f64;
            return NormalizedIndicatorValue::scalar(
                bbwp,
                norm,
                "VOLATILITY_EXHAUSTION_REVERSION_WARNING",
            );
        }
        if bias == 0 {
            return NormalizedIndicatorValue::scalar(bbwp, 0.0, "MAX_VOLATILITY_COMPRESSION");
        }
        let (mag, label) = if bbwp <= 30.0 {
            (
                0.2 + ((bbwp - 10.0) / 20.0) * 0.2,
                "LOW_VOLATILITY_BULL_CYCLE",
            )
        } else if bbwp <= 70.0 {
            (
                0.5 + ((bbwp - 30.0) / 40.0) * 0.2,
                "NORMAL_VOLATILITY_BULL_CYCLE",
            )
        } else {
            (0.8, "HIGH_VOLATILITY_BULL_EXPANSION")
        };
        NormalizedIndicatorValue::scalar(bbwp, clamp_unit(mag * bias as f64), label)
    }

    /// RVOL: volume validation gate for structural breakouts.
    pub fn normalize_rvol(rvol: f64) -> NormalizedIndicatorValue {
        let (norm, label) = if rvol < 1.0 {
            (-0.5, "CONSOLIDATION_VOLUME")
        } else if rvol < 1.5 {
            (0.2, "NORMAL_PARTICIPATION_VOLUME")
        } else if rvol < 3.0 {
            (0.8, "INSTITUTIONAL_BREAKOUT_VOLUME")
        } else {
            (-1.0, "EXHAUSTION_CLIMAX_VOLUME")
        };
        NormalizedIndicatorValue::scalar(rvol, norm, label)
    }

    /// EMA stacking & price location across the ribbon.
    pub fn normalize_ema_stack(ctx: &NormalizationContext) -> NormalizedIndicatorValue {
        let state = ctx
            .ema_stack_state
            .as_deref()
            .unwrap_or("tangled")
            .to_lowercase();
        let retest = matches!((ctx.ema_medium, ctx.price), (Some(m), p)
            if p > 0.0 && (p - m).abs() / p < 0.005);
        let (norm, label) = match state.as_str() {
            "bullish" => {
                if retest {
                    (0.8, "DYNAMIC_BULLISH_SUPPORT_RETEST")
                } else {
                    (1.0, "ESTABLISHED_BULLISH_STACK")
                }
            }
            "bearish" => {
                if retest {
                    (-0.8, "DYNAMIC_BEARISH_RESISTANCE_RETEST")
                } else {
                    (-1.0, "ESTABLISHED_BEARISH_STACK")
                }
            }
            _ => (0.0, "CONSOLIDATED_TANGLED_STACK"),
        };
        NormalizedIndicatorValue::scalar(ctx.price, norm, label)
    }

    /// VWAP fair-value baseline (premium/discount reversion zones).
    pub fn normalize_vwap(price: f64, vwap: f64) -> NormalizedIndicatorValue {
        if vwap <= 0.0 {
            return NormalizedIndicatorValue::scalar(price, 0.0, "INTRA_DAY_VALUE_EQUILIBRIUM");
        }
        let ratio = price / vwap;
        let (norm, label) = if ratio > 1.01 {
            (-0.8, "EXTREME_PREMIUM_REVERSION_ZONE")
        } else if ratio > 1.001 {
            (0.5, "INSTITUTIONAL_BULL_VALUE_PULLBACK")
        } else if ratio < 0.99 {
            (0.8, "EXTREME_DISCOUNT_REVERSION_ZONE")
        } else if ratio < 0.999 {
            (-0.5, "INSTITUTIONAL_BEAR_VALUE_PULLBACK")
        } else {
            (0.0, "INTRA_DAY_VALUE_EQUILIBRIUM")
        };
        let mut values = HashMap::new();
        values.insert("vwap".to_string(), vwap);
        values.insert("price".to_string(), price);
        NormalizedIndicatorValue::with_values(vwap, norm, label, values)
    }

    /// Fibonacci golden pocket & extension targets relative to trend bias.
    pub fn normalize_fibonacci(
        price: f64,
        gp_low: Option<f64>,
        gp_high: Option<f64>,
        ext_1618: Option<f64>,
        ext_2618: Option<f64>,
        bias: i8,
    ) -> NormalizedIndicatorValue {
        if let (Some(lo), Some(hi)) = (gp_low, gp_high) {
            let (lo, hi) = if lo <= hi { (lo, hi) } else { (hi, lo) };
            if price >= lo && price <= hi {
                return match bias {
                    b if b > 0 => NormalizedIndicatorValue::scalar(
                        price,
                        1.0,
                        "BULLISH_GOLDEN_POCKET_REBOUND",
                    ),
                    b if b < 0 => NormalizedIndicatorValue::scalar(
                        price,
                        -1.0,
                        "BEARISH_GOLDEN_POCKET_REJECTION",
                    ),
                    _ => NormalizedIndicatorValue::scalar(price, 0.0, "GOLDEN_POCKET_NEUTRAL"),
                };
            }
        }
        if bias > 0 {
            if let Some(e) = ext_2618 {
                if price >= e {
                    return NormalizedIndicatorValue::scalar(
                        price,
                        0.1,
                        "CLIMACTIC_EXTENSION_TARGET_REACHED",
                    );
                }
            }
            if let Some(e) = ext_1618 {
                if price >= e {
                    return NormalizedIndicatorValue::scalar(
                        price,
                        0.2,
                        "PRIMARY_EXTENSION_TARGET_REACHED",
                    );
                }
            }
        } else if bias < 0 {
            if let Some(e) = ext_2618 {
                if price <= e {
                    return NormalizedIndicatorValue::scalar(
                        price,
                        -0.1,
                        "CLIMACTIC_EXTENSION_TARGET_REACHED",
                    );
                }
            }
            if let Some(e) = ext_1618 {
                if price <= e {
                    return NormalizedIndicatorValue::scalar(
                        price,
                        -0.2,
                        "PRIMARY_EXTENSION_TARGET_REACHED",
                    );
                }
            }
        }
        NormalizedIndicatorValue::scalar(price, 0.0, "FIBONACCI_NEUTRAL")
    }

    /// Chart pattern breakout / accumulation gated by RVOL confirmation.
    pub fn normalize_patterns(
        is_bullish: bool,
        is_bearish: bool,
        confidence: f64,
        rvol: f64,
    ) -> NormalizedIndicatorValue {
        let (norm, label) = if is_bullish {
            if rvol >= 1.5 {
                (1.0, "BULLISH_PATTERN_BREAKOUT")
            } else {
                (0.5, "BULLISH_PATTERN_ACCUMULATION")
            }
        } else if is_bearish {
            if rvol >= 1.5 {
                (-1.0, "BEARISH_PATTERN_BREAKOUT")
            } else {
                (-0.5, "BEARISH_PATTERN_DISTRIBUTION")
            }
        } else {
            (0.0, "NO_PATTERN")
        };
        NormalizedIndicatorValue::scalar(confidence, norm, label)
    }

    /// Support & Resistance with proximity + role-reversal breakout detection.
    pub fn normalize_sr(
        price: f64,
        support_levels: &[f64],
        resistance_levels: &[f64],
        rvol: f64,
    ) -> NormalizedIndicatorValue {
        let near = |lvl: f64| lvl > 0.0 && (price - lvl).abs() / lvl <= 0.005;
        if support_levels.iter().copied().any(near) {
            return NormalizedIndicatorValue::scalar(price, 1.0, "SUPPORT_DEMAND_ZONE");
        }
        if resistance_levels.iter().copied().any(near) {
            return NormalizedIndicatorValue::scalar(price, -1.0, "RESISTANCE_SUPPLY_ZONE");
        }
        if rvol >= 1.5 {
            let highest_res = resistance_levels
                .iter()
                .copied()
                .filter(|r| *r > 0.0)
                .fold(None, |acc: Option<f64>, r| Some(acc.map_or(r, |a| a.max(r))));
            if let Some(r) = highest_res {
                if price > r {
                    return NormalizedIndicatorValue::scalar(price, 0.8, "RESISTANCE_FLIP_CONFIRMED");
                }
            }
            let lowest_sup = support_levels
                .iter()
                .copied()
                .filter(|s| *s > 0.0)
                .fold(None, |acc: Option<f64>, s| Some(acc.map_or(s, |a| a.min(s))));
            if let Some(s) = lowest_sup {
                if price < s {
                    return NormalizedIndicatorValue::scalar(price, -0.8, "SUPPORT_FLIP_CONFIRMED");
                }
            }
        }
        NormalizedIndicatorValue::scalar(price, 0.0, "STRUCTURE_NEUTRAL")
    }
}
