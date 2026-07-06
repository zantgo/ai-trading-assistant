//! Extended normalization mappers for Phase 1A/1B indicators (Stochastic,
//! ChandeMO, Supertrend, Keltner, Donchian, OBV, CMF, MFI, Historical
//! Volatility, Aroon, Choppiness, Linear Regression Slope, Z-Score).
//!
//! Split out of context.rs to keep each file within the 500-line limit.

use super::{clamp_unit, NormalizationEngine, NormalizedIndicatorValue};
use std::collections::HashMap;

impl NormalizationEngine {
    pub fn normalize_stochastic(k: f64, d: f64) -> NormalizedIndicatorValue {
        // Map the [0, 100] scale onto the [-1.0, 1.0] unit interval centered at 50.
        let norm_k = ((k - 50.0) / 50.0).clamp(-1.0, 1.0);
        let label = if k >= 80.0 {
            "OVERBOUGHT_DISTRIBUTION"
        } else if k <= 20.0 {
            "OVERSOLD_ACCUMULATION"
        } else if k > d {
            "BULLISH_MOMENTUM_ALIGNMENT"
        } else {
            "BEARISH_MOMENTUM_ALIGNMENT"
        };

        let mut values = HashMap::new();
        values.insert("k_line".to_string(), k);
        values.insert("d_line".to_string(), d);
        NormalizedIndicatorValue::with_values(k, norm_k, label.to_string(), values)
    }

    /// Chande Momentum Oscillator: raw momentum ratio, natively `[-100, 100]`.
    pub fn normalize_chandemo(cmo: f64) -> NormalizedIndicatorValue {
        let norm = (cmo / 100.0).clamp(-1.0, 1.0);
        let label = if cmo >= 50.0 {
            "CLIMACTIC_BULL_EXHAUSTION"
        } else if cmo <= -50.0 {
            "CLIMACTIC_BEAR_EXHAUSTION"
        } else if cmo > 0.0 {
            "EMERGING_BULL_MOMENTUM"
        } else {
            "EMERGING_BEAR_MOMENTUM"
        };
        NormalizedIndicatorValue::scalar(cmo, norm, label.to_string())
    }

    /// Supertrend: trend direction with distance-scaled conviction.
    pub fn normalize_supertrend(price: f64, line: f64, direction: i8) -> NormalizedIndicatorValue {
        let dist = if line.abs() > f64::EPSILON {
            ((price - line) / line).abs()
        } else {
            0.0
        };
        let mag = 0.6 + 0.4 * (dist * 12.0).tanh();
        let norm = clamp_unit(direction as f64 * mag);
        let label = if direction > 0 {
            "SUPERTREND_BULLISH"
        } else {
            "SUPERTREND_BEARISH"
        };
        let mut values = HashMap::new();
        values.insert("line".to_string(), line);
        values.insert("direction".to_string(), direction as f64);
        NormalizedIndicatorValue::with_values(line, norm, label.to_string(), values)
    }

    /// Keltner Channels: price position within / breakout beyond the channel.
    pub fn normalize_keltner(price: f64, upper: f64, middle: f64, lower: f64) -> NormalizedIndicatorValue {
        let (norm, label) = if price >= upper {
            (1.0, "KELTNER_UPPER_BREAKOUT")
        } else if price <= lower {
            (-1.0, "KELTNER_LOWER_BREAKOUT")
        } else {
            let half = (upper - middle).max(f64::EPSILON);
            let n = (price - middle) / half;
            (
                clamp_unit(n * 0.8),
                if n >= 0.0 { "KELTNER_UPPER_HALF" } else { "KELTNER_LOWER_HALF" },
            )
        };
        let mut values = HashMap::new();
        values.insert("upper".to_string(), upper);
        values.insert("middle".to_string(), middle);
        values.insert("lower".to_string(), lower);
        NormalizedIndicatorValue::with_values(middle, norm, label.to_string(), values)
    }

    /// Donchian Channels: breakout at the extremes, else position within range.
    pub fn normalize_donchian(price: f64, upper: f64, middle: f64, lower: f64) -> NormalizedIndicatorValue {
        let (norm, label) = if price >= upper {
            (1.0, "DONCHIAN_UPPER_BREAKOUT")
        } else if price <= lower {
            (-1.0, "DONCHIAN_LOWER_BREAKOUT")
        } else {
            let half = (upper - middle).max(f64::EPSILON);
            let n = (price - middle) / half;
            (
                clamp_unit(n * 0.7),
                if n >= 0.0 { "DONCHIAN_UPPER_RANGE" } else { "DONCHIAN_LOWER_RANGE" },
            )
        };
        let mut values = HashMap::new();
        values.insert("upper".to_string(), upper);
        values.insert("middle".to_string(), middle);
        values.insert("lower".to_string(), lower);
        NormalizedIndicatorValue::with_values(upper, norm, label.to_string(), values)
    }

    /// OBV: accumulation/distribution from OBV slope vs its smoothed baseline.
    pub fn normalize_obv(obv: f64, obv_sma: f64) -> NormalizedIndicatorValue {
        let diff = obv - obv_sma;
        let denom = obv.abs().max(obv_sma.abs()).max(1.0);
        let norm = clamp_unit((diff / denom * 2.5).tanh() + 0.0);
        let label = if norm > 0.1 {
            "OBV_ACCUMULATION"
        } else if norm < -0.1 {
            "OBV_DISTRIBUTION"
        } else {
            "OBV_NEUTRAL"
        };
        let mut values = HashMap::new();
        values.insert("obv".to_string(), obv);
        values.insert("obv_sma".to_string(), obv_sma);
        NormalizedIndicatorValue::with_values(obv, norm, label.to_string(), values)
    }

    /// Chaikin Money Flow: buying vs selling pressure (native [-1,1], amplified).
    pub fn normalize_cmf(cmf: f64) -> NormalizedIndicatorValue {
        let norm = clamp_unit(cmf * 3.0);
        let label = if cmf >= 0.2 {
            "CMF_STRONG_BUYING"
        } else if cmf >= 0.05 {
            "CMF_BUYING_PRESSURE"
        } else if cmf <= -0.2 {
            "CMF_STRONG_SELLING"
        } else if cmf <= -0.05 {
            "CMF_SELLING_PRESSURE"
        } else {
            "CMF_NEUTRAL_FLOW"
        };
        NormalizedIndicatorValue::scalar(cmf, norm, label.to_string())
    }

    /// Money Flow Index: volume-weighted RSI mapping (mean-reversion at extremes).
    pub fn normalize_mfi(mfi: f64) -> NormalizedIndicatorValue {
        let norm = if mfi <= 20.0 {
            0.7 + ((20.0 - mfi) / 20.0) * 0.3
        } else if mfi >= 80.0 {
            -0.7 - ((mfi - 80.0) / 20.0) * 0.3
        } else if mfi <= 50.0 {
            ((50.0 - mfi) / 30.0) * 0.7
        } else {
            -((mfi - 50.0) / 30.0) * 0.7
        };
        let label = if mfi >= 80.0 {
            "MFI_OVERBOUGHT_DISTRIBUTION"
        } else if mfi <= 20.0 {
            "MFI_OVERSOLD_ACCUMULATION"
        } else if mfi >= 50.0 {
            "MFI_BULLISH_FLOW"
        } else {
            "MFI_BEARISH_FLOW"
        };
        NormalizedIndicatorValue::scalar(mfi, clamp_unit(norm), label.to_string())
    }

    /// Historical Volatility: non-directional volatility gate (normalized 0.0).
    pub fn normalize_hv(hv: f64) -> NormalizedIndicatorValue {
        let label = if hv >= 100.0 {
            "EXTREME_VOLATILITY"
        } else if hv >= 60.0 {
            "HIGH_VOLATILITY"
        } else if hv <= 20.0 {
            "LOW_VOLATILITY"
        } else {
            "NORMAL_VOLATILITY"
        };
        NormalizedIndicatorValue::scalar(hv, 0.0, label.to_string())
    }

    /// Aroon Oscillator: trend emergence (+) vs consolidation/reversal (-).
    pub fn normalize_aroon(up: f64, down: f64) -> NormalizedIndicatorValue {
        let osc = up - down;
        let norm = clamp_unit(osc / 100.0);
        let label = if up >= 70.0 && down <= 30.0 {
            "AROON_STRONG_UPTREND"
        } else if down >= 70.0 && up <= 30.0 {
            "AROON_STRONG_DOWNTREND"
        } else if osc > 0.0 {
            "AROON_BULLISH_BIAS"
        } else if osc < 0.0 {
            "AROON_BEARISH_BIAS"
        } else {
            "AROON_CONSOLIDATION"
        };
        let mut values = HashMap::new();
        values.insert("up".to_string(), up);
        values.insert("down".to_string(), down);
        NormalizedIndicatorValue::with_values(osc, norm, label.to_string(), values)
    }

    /// Choppiness Index: non-directional regime gate (normalized 0.0). High =
    /// choppy/range (dampen conviction); low = trending.
    pub fn normalize_choppiness(chop: f64) -> NormalizedIndicatorValue {
        let label = if chop >= 61.8 {
            "CHOP_CONSOLIDATION_RANGE"
        } else if chop <= 38.2 {
            "CHOP_STRONG_TREND"
        } else {
            "CHOP_TRANSITIONAL"
        };
        NormalizedIndicatorValue::scalar(chop, 0.0, label.to_string())
    }

    /// Linear Regression Slope: directional trend from the least-squares slope,
    /// scaled by price into a per-bar percentage then saturated.
    pub fn normalize_linreg_slope(slope: f64, price: f64) -> NormalizedIndicatorValue {
        let pct_per_bar = if price.abs() > f64::EPSILON {
            slope / price * 100.0
        } else {
            0.0
        };
        let norm = clamp_unit((pct_per_bar * 3.0).tanh());
        let label = if norm > 0.1 {
            "LINREG_RISING_TREND"
        } else if norm < -0.1 {
            "LINREG_FALLING_TREND"
        } else {
            "LINREG_FLAT"
        };
        NormalizedIndicatorValue::scalar(slope, norm, label.to_string())
    }

    /// Z-Score: mean-reversion — statistically stretched high (+z) is bearish
    /// (distribution), stretched low (-z) is bullish (accumulation).
    pub fn normalize_zscore(z: f64) -> NormalizedIndicatorValue {
        let norm = clamp_unit(-z / 3.0);
        let label = if z >= 2.0 {
            "ZSCORE_OVEREXTENDED_HIGH"
        } else if z <= -2.0 {
            "ZSCORE_OVEREXTENDED_LOW"
        } else if z > 0.0 {
            "ZSCORE_ABOVE_MEAN"
        } else if z < 0.0 {
            "ZSCORE_BELOW_MEAN"
        } else {
            "ZSCORE_AT_MEAN"
        };
        NormalizedIndicatorValue::scalar(z, norm, label.to_string())
    }
}
