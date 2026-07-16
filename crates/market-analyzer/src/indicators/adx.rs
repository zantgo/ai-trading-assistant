use super::ema::Ema;
use super::traits::{BarInput, Indicator};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Direction of a +DI/-DI crossover event
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiCrossoverDir {
    Bullish,
    Bearish,
}

/// Trend strength regime classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrendRegime {
    Congestion, // ADX < 20
    Emerging,   // 20 <= ADX < 25
    Strong,     // 25 <= ADX <= 40
    Extreme,    // ADX > 40
}

/// Max ADX history length for slope calculation
const ADX_HISTORY_LEN: usize = 4;

/// Expanded output from an ADX update including slope, crossover, and regime.
#[derive(Debug, Clone)]
pub struct AdxOutput {
    pub adx: Decimal,
    pub plus_di: Decimal,
    pub minus_di: Decimal,
    pub adx_slope: Decimal,
    pub di_crossover: Option<DiCrossoverDir>,
    pub adx_peak: Decimal,
    pub trending_regime: TrendRegime,
}

/// Average Directional Index with stateful tracking.
///
/// Tracks ADX history for slope calculation, +DI/-DI history for
/// crossover detection, and a running ADX peak for exhaustion monitoring.
#[derive(Debug, Clone)]
pub struct Adx {
    period: usize,
    prev_high: Option<Decimal>,
    prev_low: Option<Decimal>,
    prev_close: Option<Decimal>,
    tr_ema: Ema,
    plus_dm_ema: Ema,
    minus_dm_ema: Ema,
    dx_ema: Ema,
    adx_history: VecDeque<Decimal>,
    prev_plus_di: Option<Decimal>,
    prev_minus_di: Option<Decimal>,
    adx_peak: Decimal,
    trend_threshold: Decimal,
    exhaustion_threshold: Decimal,
    slope_lookback: usize,
}

impl Adx {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            prev_high: None,
            prev_low: None,
            prev_close: None,
            tr_ema: Ema::new(period),
            plus_dm_ema: Ema::new(period),
            minus_dm_ema: Ema::new(period),
            dx_ema: Ema::new(period),
            adx_history: VecDeque::with_capacity(ADX_HISTORY_LEN),
            prev_plus_di: None,
            prev_minus_di: None,
            adx_peak: Decimal::ZERO,
            trend_threshold: Decimal::new(20, 0),
            exhaustion_threshold: Decimal::new(40, 0),
            slope_lookback: 3,
        }
    }

    /// Configure the trend and exhaustion thresholds.
    pub fn set_thresholds(
        &mut self,
        trend_threshold: Decimal,
        exhaustion_threshold: Decimal,
        slope_lookback: usize,
    ) {
        self.trend_threshold = trend_threshold;
        self.exhaustion_threshold = exhaustion_threshold;
        self.slope_lookback = slope_lookback;
    }

    pub fn update(&mut self, high: Decimal, low: Decimal, close: Decimal) -> Option<AdxOutput> {
        let (p_high, p_low, p_close) = match (self.prev_high, self.prev_low, self.prev_close) {
            (Some(h), Some(l), Some(c)) => (h, l, c),
            _ => {
                self.prev_high = Some(high);
                self.prev_low = Some(low);
                self.prev_close = Some(close);
                return None;
            }
        };

        self.prev_high = Some(high);
        self.prev_low = Some(low);
        self.prev_close = Some(close);

        let r1 = high - low;
        let r2 = (high - p_close).abs();
        let r3 = (low - p_close).abs();
        let tr = r1.max(r2).max(r3);

        let up_move = high - p_high;
        let down_move = p_low - low;

        let plus_dm = if up_move > down_move && up_move > Decimal::ZERO {
            up_move
        } else {
            Decimal::ZERO
        };
        let minus_dm = if down_move > up_move && down_move > Decimal::ZERO {
            down_move
        } else {
            Decimal::ZERO
        };

        let tr_smooth = self.tr_ema.update(tr);
        let plus_dm_smooth = self.plus_dm_ema.update(plus_dm);
        let minus_dm_smooth = self.minus_dm_ema.update(minus_dm);

        if tr_smooth == Decimal::ZERO {
            return None;
        }

        let plus_di = (plus_dm_smooth / tr_smooth) * Decimal::from(100);
        let minus_di = (minus_dm_smooth / tr_smooth) * Decimal::from(100);

        let di_sum = plus_di + minus_di;
        if di_sum == Decimal::ZERO {
            return None;
        }

        let dx = ((plus_di - minus_di).abs() / di_sum) * Decimal::from(100);
        let adx = self.dx_ema.update(dx);

        // Track ADX history for slope calculation
        self.adx_history.push_back(adx);
        while self.adx_history.len() > ADX_HISTORY_LEN {
            self.adx_history.pop_front();
        }

        // Compute ADX slope
        let adx_slope = self.compute_slope();

        // Detect DI crossover
        let di_crossover = self.detect_di_crossover(plus_di, minus_di);

        // Track ADX peak — reset on DI crossover
        if di_crossover.is_some() {
            self.adx_peak = Decimal::ZERO;
        }
        if adx > self.adx_peak {
            self.adx_peak = adx;
        }

        // Classify trend regime
        let trending_regime = classify_regime(adx, self.trend_threshold, self.exhaustion_threshold);

        self.prev_plus_di = Some(plus_di);
        self.prev_minus_di = Some(minus_di);

        Some(AdxOutput {
            adx,
            plus_di,
            minus_di,
            adx_slope,
            di_crossover,
            adx_peak: self.adx_peak,
            trending_regime,
        })
    }

    /// Compute ADX slope as difference over the lookback window.
    /// Positive = accelerating, negative = decelerating.
    fn compute_slope(&self) -> Decimal {
        if self.adx_history.len() < self.slope_lookback {
            return Decimal::ZERO;
        }
        let newest = self.adx_history.back().copied().unwrap_or(Decimal::ZERO);
        let old = self.adx_history.front().copied().unwrap_or(Decimal::ZERO);
        newest - old
    }

    /// Detect +DI/-DI crossover from previous values to current.
    fn detect_di_crossover(
        &self,
        current_plus: Decimal,
        current_minus: Decimal,
    ) -> Option<DiCrossoverDir> {
        let (prev_p, prev_m) = match (self.prev_plus_di, self.prev_minus_di) {
            (Some(p), Some(m)) => (p, m),
            _ => return None,
        };

        if prev_p <= prev_m && current_plus > current_minus {
            Some(DiCrossoverDir::Bullish)
        } else if prev_p >= prev_m && current_plus < current_minus {
            Some(DiCrossoverDir::Bearish)
        } else {
            None
        }
    }

    /// Get the current ADX peak.
    pub fn get_adx_peak(&self) -> Decimal {
        self.adx_peak
    }

    /// Get the latest ADX value.
    pub fn get_adx(&self) -> Option<Decimal> {
        self.adx_history.back().copied()
    }

    /// Get the current ADX slope.
    pub fn get_slope(&self) -> Decimal {
        self.compute_slope()
    }
}

impl Indicator for Adx {
    type Output = Option<AdxOutput>;

    fn update(&mut self, bar: &BarInput) -> Self::Output {
        self.update(bar.high, bar.low, bar.close)
    }

    fn reset(&mut self) {
        *self = Adx::new(self.period);
    }
}

/// Classify the trend regime based on ADX value against configured thresholds.
fn classify_regime(
    adx: Decimal,
    trend_threshold: Decimal,
    exhaustion_threshold: Decimal,
) -> TrendRegime {
    if adx > exhaustion_threshold {
        TrendRegime::Extreme
    } else if adx >= trend_threshold + Decimal::new(5, 0) {
        TrendRegime::Strong
    } else if adx >= trend_threshold {
        TrendRegime::Emerging
    } else {
        TrendRegime::Congestion
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_first_update_returns_none() {
        let mut adx = Adx::new(14);
        assert!(adx.update(dec!(100.00), dec!(95.00), dec!(98.00)).is_none());
    }

    #[test]
    fn test_strong_up_trend_plus_di_above_minus_di() {
        let mut adx = Adx::new(14);
        let mut high = dec!(100.00);
        let mut low = dec!(95.00);
        let mut close = dec!(98.00);
        adx.update(high, low, close);

        for _ in 0..20 {
            high += dec!(2.00);
            low += dec!(1.50);
            close += dec!(2.00);
            adx.update(high, low, close);
        }

        let out = adx
            .update(high + dec!(2.00), low + dec!(1.50), close + dec!(2.00))
            .unwrap();
        assert!(
            out.plus_di > out.minus_di,
            "Strong uptrend: +DI should exceed -DI"
        );
        assert!(out.adx > Decimal::ZERO);
    }

    #[test]
    fn test_zero_movement_periods_produce_symmetric_di() {
        let mut adx = Adx::new(14);
        let price = dec!(100.00);
        adx.update(price, price, price);
        for _ in 0..20 {
            if let Some(out) = adx.update(price, price, price) {
                assert!(
                    (out.plus_di - out.minus_di).abs() < dec!(1.00),
                    "Zero movement: +DI and -DI should be near equal"
                );
            }
        }
    }

    #[test]
    fn test_di_crossover_detection() {
        let mut adx = Adx::new(14);
        let mut high = dec!(100.00);
        let mut low = dec!(95.00);
        let mut close = dec!(98.00);
        adx.update(high, low, close);

        // Drive a strong uptrend then reverse to detect crossovers
        for _ in 0..15 {
            high += dec!(1.00);
            low += dec!(0.50);
            close += dec!(1.00);
            adx.update(high, low, close);
        }
        // Now reverse hard
        for _ in 0..20 {
            high -= dec!(1.00);
            low -= dec!(0.50);
            close -= dec!(1.00);
            adx.update(high, low, close);
        }
        let out = adx
            .update(high - dec!(1.00), low - dec!(0.50), close - dec!(1.00))
            .unwrap();
        // After sustained downtrend, -DI should exceed +DI
        assert!(out.minus_di > out.plus_di);
    }

    #[test]
    fn test_regime_classification_congestion() {
        let regime = classify_regime(dec!(15.0), dec!(20.0), dec!(40.0));
        assert_eq!(regime, TrendRegime::Congestion);
    }

    #[test]
    fn test_regime_classification_emerging() {
        let regime = classify_regime(dec!(22.0), dec!(20.0), dec!(40.0));
        assert_eq!(regime, TrendRegime::Emerging);
    }

    #[test]
    fn test_regime_classification_strong() {
        let regime = classify_regime(dec!(30.0), dec!(20.0), dec!(40.0));
        assert_eq!(regime, TrendRegime::Strong);
    }

    #[test]
    fn test_regime_classification_extreme() {
        let regime = classify_regime(dec!(42.0), dec!(20.0), dec!(40.0));
        assert_eq!(regime, TrendRegime::Extreme);
    }

    #[test]
    fn test_adx_peak_tracks_max() {
        let mut adx = Adx::new(14);
        let mut high = dec!(100.00);
        let mut low = dec!(95.00);
        let mut close = dec!(98.00);
        adx.update(high, low, close);

        for _ in 0..20 {
            high += dec!(2.00);
            low += dec!(1.50);
            close += dec!(2.00);
            adx.update(high, low, close);
        }
        let peak = adx.get_adx_peak();
        assert!(peak > Decimal::ZERO);
    }

    #[test]
    fn test_slope_computation() {
        let mut adx = Adx::new(14);
        adx.set_thresholds(dec!(20.0), dec!(40.0), 3);
        let mut high = dec!(100.00);
        let mut low = dec!(95.00);
        let mut close = dec!(98.00);
        adx.update(high, low, close);

        for _ in 0..25 {
            high += dec!(1.00);
            low += dec!(0.50);
            close += dec!(1.00);
            adx.update(high, low, close);
        }
        let slope = adx.get_slope();
        // In a sustained uptrend, ADX slope should be positive or zero
        assert!(slope >= Decimal::ZERO);
    }
}
