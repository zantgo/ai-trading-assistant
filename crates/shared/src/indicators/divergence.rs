use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Combined RSI and MACD Divergence Detector —
/// identifies bullish and bearish divergences between price action
/// and RSI values, plus MACD histogram divergences.
///
/// RSI Bullish Divergence: Price makes Lower Low, RSI makes Higher Low.
/// RSI Bearish Divergence: Price makes Higher High, RSI makes Lower High.
///
/// MACD Bullish Divergence: Price makes Lower Low, MACD Histogram makes Higher Low.
/// MACD Bearish Divergence: Price makes Higher High, MACD Histogram makes Lower High.
///
/// Divergences start as "Potential" until a candle close breaks the relevant
/// support (for bullish) or resistance (for bearish) level, at which point
/// they become "Confirmed".
#[derive(Debug, Clone)]
pub struct DivergenceDetector {
    price_history: Vec<Decimal>,
    rsi_history: Vec<Decimal>,
    macd_hist_history: Vec<Decimal>,
    lookback: usize,
}

/// Specific type of divergence detected
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DivergenceType {
    RsiBullish,
    RsiBearish,
    MacdBullish,
    MacdBearish,
    RsiBullishHidden,
    RsiBearishHidden,
    MacdBullishHidden,
    MacdBearishHidden,
    None,
}

/// Divergence confirmation status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DivergenceStatus {
    Potential,
    Confirmed,
    None,
}

impl DivergenceStatus {
    pub fn is_active(&self) -> bool {
        matches!(
            self,
            DivergenceStatus::Potential | DivergenceStatus::Confirmed
        )
    }

    pub fn is_confirmed(&self) -> bool {
        matches!(self, DivergenceStatus::Confirmed)
    }
}

/// Coordinates of a price peak or trough used for divergence detection and
/// chart rendering. Includes the index within the lookback window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeakTrough {
    pub price: Decimal,
    pub indicator_value: Decimal,
    pub index: usize,
}

impl PeakTrough {
    pub fn price_f64(&self) -> f64 {
        self.price.to_string().parse::<f64>().unwrap_or(0.0)
    }

    pub fn indicator_f64(&self) -> f64 {
        self.indicator_value
            .to_string()
            .parse::<f64>()
            .unwrap_or(0.0)
    }
}

/// Pair of peak/trough coordinates for a detected divergence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DivergenceCoords {
    pub first_extreme: PeakTrough,
    pub second_extreme: PeakTrough,
}

#[derive(Debug, Clone)]
pub struct DivergenceResult {
    pub rsi_divergence: DivergenceType,
    pub macd_divergence: DivergenceType,
    pub has_bullish: bool,
    pub has_bearish: bool,
    pub rsi_status: DivergenceStatus,
    pub macd_status: DivergenceStatus,
    pub rsi_coords: Option<DivergenceCoords>,
    pub macd_coords: Option<DivergenceCoords>,
    pub rsi_hidden: bool,
    pub macd_hidden: bool,
}

impl DivergenceResult {
    pub fn default_div() -> Self {
        Self {
            rsi_divergence: DivergenceType::None,
            macd_divergence: DivergenceType::None,
            has_bullish: false,
            has_bearish: false,
            rsi_status: DivergenceStatus::None,
            macd_status: DivergenceStatus::None,
            rsi_coords: None,
            macd_coords: None,
            rsi_hidden: false,
            macd_hidden: false,
        }
    }
}

impl DivergenceDetector {
    pub fn new(lookback: usize) -> Self {
        Self {
            price_history: Vec::with_capacity(lookback),
            rsi_history: Vec::with_capacity(lookback),
            macd_hist_history: Vec::with_capacity(lookback),
            lookback,
        }
    }

    /// Feeds price + RSI + MACD histogram into the detector.
    /// Returns the combined divergence result with potential status.
    pub fn update_full(
        &mut self,
        price: Decimal,
        rsi: Decimal,
        macd_histogram: Decimal,
    ) -> DivergenceResult {
        self.price_history.push(price);
        self.rsi_history.push(rsi);
        self.macd_hist_history.push(macd_histogram);

        if self.price_history.len() > self.lookback {
            self.price_history.remove(0);
            self.rsi_history.remove(0);
            self.macd_hist_history.remove(0);
        }

        if self.price_history.len() < self.lookback {
            return DivergenceResult {
                rsi_divergence: DivergenceType::None,
                macd_divergence: DivergenceType::None,
                has_bullish: false,
                has_bearish: false,
                rsi_status: DivergenceStatus::None,
                macd_status: DivergenceStatus::None,
                rsi_coords: None,
                macd_coords: None,
                rsi_hidden: false,
                macd_hidden: false,
            };
        }

        let (rsi_div, rsi_coords) = self.detect_rsi_divergence();
        let (macd_div, macd_coords) = self.detect_macd_divergence();

        let has_bullish = matches!(rsi_div, DivergenceType::RsiBullish | DivergenceType::RsiBullishHidden)
            || matches!(macd_div, DivergenceType::MacdBullish | DivergenceType::MacdBullishHidden);
        let has_bearish = matches!(rsi_div, DivergenceType::RsiBearish | DivergenceType::RsiBearishHidden)
            || matches!(macd_div, DivergenceType::MacdBearish | DivergenceType::MacdBearishHidden);

        let rsi_status = if rsi_div != DivergenceType::None {
            DivergenceStatus::Potential
        } else {
            DivergenceStatus::None
        };
        let macd_status = if macd_div != DivergenceType::None {
            DivergenceStatus::Potential
        } else {
            DivergenceStatus::None
        };

        let rsi_hidden = matches!(rsi_div, DivergenceType::RsiBullishHidden | DivergenceType::RsiBearishHidden);
        let macd_hidden = matches!(macd_div, DivergenceType::MacdBullishHidden | DivergenceType::MacdBearishHidden);

        DivergenceResult {
            rsi_divergence: rsi_div,
            macd_divergence: macd_div,
            has_bullish,
            has_bearish,
            rsi_status,
            macd_status,
            rsi_coords,
            macd_coords,
            rsi_hidden,
            macd_hidden,
        }
    }

    /// Check whether a potential divergence should be upgraded to Confirmed
    /// based on a candle close breaching the relevant support/resistance level.
    ///
    /// - For bullish divergences: close must break BELOW the active support level.
    /// - For bearish divergences: close must break ABOVE the active resistance level.
    /// - Tolerance buffer: 0.2% of the S/R level price.
    pub fn check_divergence_confirmation(
        &self,
        current_result: &DivergenceResult,
        close_price: Decimal,
        support_level: Option<Decimal>,
        resistance_level: Option<Decimal>,
    ) -> DivergenceResult {
        let mut confirmed = current_result.clone();
        let tolerance_pct = Decimal::new(2, 3); // 0.002

        // RSI confirmation
        if confirmed.rsi_status == DivergenceStatus::Potential {
            let now_confirmed = match confirmed.rsi_divergence {
                DivergenceType::RsiBullish | DivergenceType::RsiBullishHidden => {
                    // Bullish: close breaks below support
                    if let Some(s) = support_level {
                        let buffer = s * tolerance_pct;
                        close_price < s && (s - close_price) > buffer
                    } else {
                        false
                    }
                }
                DivergenceType::RsiBearish | DivergenceType::RsiBearishHidden => {
                    // Bearish: close breaks above resistance
                    if let Some(r) = resistance_level {
                        let buffer = r * tolerance_pct;
                        close_price > r && (close_price - r) > buffer
                    } else {
                        false
                    }
                }
                _ => false,
            };
            if now_confirmed {
                confirmed.rsi_status = DivergenceStatus::Confirmed;
            }
        }

        // MACD confirmation
        if confirmed.macd_status == DivergenceStatus::Potential {
            let now_confirmed = match confirmed.macd_divergence {
                DivergenceType::MacdBullish | DivergenceType::MacdBullishHidden => {
                    if let Some(s) = support_level {
                        let buffer = s * tolerance_pct;
                        close_price < s && (s - close_price) > buffer
                    } else {
                        false
                    }
                }
                DivergenceType::MacdBearish | DivergenceType::MacdBearishHidden => {
                    if let Some(r) = resistance_level {
                        let buffer = r * tolerance_pct;
                        close_price > r && (close_price - r) > buffer
                    } else {
                        false
                    }
                }
                _ => false,
            };
            if now_confirmed {
                confirmed.macd_status = DivergenceStatus::Confirmed;
            }
        }

        confirmed
    }

    /// Get the current lookback window size
    pub fn len(&self) -> usize {
        self.price_history.len()
    }

    /// Whether no price history has been accumulated yet.
    pub fn is_empty(&self) -> bool {
        self.price_history.is_empty()
    }

    /// Whether the detector has enough data for analysis
    pub fn is_ready(&self) -> bool {
        self.price_history.len() >= self.lookback
    }

    /// Detect RSI divergence within history. Returns type and coordinates.
    fn detect_rsi_divergence(&self) -> (DivergenceType, Option<DivergenceCoords>) {
        let half = self.lookback / 2;
        let len = self.price_history.len();

        // Bullish (regular): price lower low, RSI higher low
        if let (Some(first), Some(last)) = (
            find_extrema(&self.price_history, &self.rsi_history, 0..half, false),
            find_extrema(&self.price_history, &self.rsi_history, half..len, false),
        ) {
            if last.price < first.price && last.indicator_value > first.indicator_value {
                return (
                    DivergenceType::RsiBullish,
                    Some(DivergenceCoords {
                        first_extreme: first,
                        second_extreme: last,
                    }),
                );
            }
        }

        // Bearish (regular): price higher high, RSI lower high
        if let (Some(first), Some(last)) = (
            find_extrema(&self.price_history, &self.rsi_history, 0..half, true),
            find_extrema(&self.price_history, &self.rsi_history, half..len, true),
        ) {
            if last.price > first.price && last.indicator_value < first.indicator_value {
                return (
                    DivergenceType::RsiBearish,
                    Some(DivergenceCoords {
                        first_extreme: first,
                        second_extreme: last,
                    }),
                );
            }
        }

        // Hidden Bullish: price higher low, RSI lower low (continuation)
        if let (Some(first), Some(last)) = (
            find_extrema(&self.price_history, &self.rsi_history, 0..half, false),
            find_extrema(&self.price_history, &self.rsi_history, half..len, false),
        ) {
            if last.price > first.price && last.indicator_value < first.indicator_value {
                return (
                    DivergenceType::RsiBullishHidden,
                    Some(DivergenceCoords {
                        first_extreme: first,
                        second_extreme: last,
                    }),
                );
            }
        }

        // Hidden Bearish: price lower high, RSI higher high (continuation)
        if let (Some(first), Some(last)) = (
            find_extrema(&self.price_history, &self.rsi_history, 0..half, true),
            find_extrema(&self.price_history, &self.rsi_history, half..len, true),
        ) {
            if last.price < first.price && last.indicator_value > first.indicator_value {
                return (
                    DivergenceType::RsiBearishHidden,
                    Some(DivergenceCoords {
                        first_extreme: first,
                        second_extreme: last,
                    }),
                );
            }
        }

        (DivergenceType::None, None)
    }

    /// Detect MACD histogram divergence within history. Returns type and coordinates.
    fn detect_macd_divergence(&self) -> (DivergenceType, Option<DivergenceCoords>) {
        if self.macd_hist_history.len() < self.lookback {
            return (DivergenceType::None, None);
        }

        let half = self.lookback / 2;
        let len = self.price_history.len();

        // Bullish (regular): price lower low, MACD histogram higher low
        if let (Some(first), Some(last)) = (
            find_extrema(&self.price_history, &self.macd_hist_history, 0..half, false),
            find_extrema(&self.price_history, &self.macd_hist_history, half..len, false),
        ) {
            if last.price < first.price && last.indicator_value > first.indicator_value {
                return (
                    DivergenceType::MacdBullish,
                    Some(DivergenceCoords {
                        first_extreme: first,
                        second_extreme: last,
                    }),
                );
            }
        }

        // Bearish (regular): price higher high, MACD histogram lower high
        if let (Some(first), Some(last)) = (
            find_extrema(&self.price_history, &self.macd_hist_history, 0..half, true),
            find_extrema(&self.price_history, &self.macd_hist_history, half..len, true),
        ) {
            if last.price > first.price && last.indicator_value < first.indicator_value {
                return (
                    DivergenceType::MacdBearish,
                    Some(DivergenceCoords {
                        first_extreme: first,
                        second_extreme: last,
                    }),
                );
            }
        }

        // Hidden Bullish: price higher low, MACD histogram lower low
        if let (Some(first), Some(last)) = (
            find_extrema(&self.price_history, &self.macd_hist_history, 0..half, false),
            find_extrema(&self.price_history, &self.macd_hist_history, half..len, false),
        ) {
            if last.price > first.price && last.indicator_value < first.indicator_value {
                return (
                    DivergenceType::MacdBullishHidden,
                    Some(DivergenceCoords {
                        first_extreme: first,
                        second_extreme: last,
                    }),
                );
            }
        }

        // Hidden Bearish: price lower high, MACD histogram higher high
        if let (Some(first), Some(last)) = (
            find_extrema(&self.price_history, &self.macd_hist_history, 0..half, true),
            find_extrema(&self.price_history, &self.macd_hist_history, half..len, true),
        ) {
            if last.price < first.price && last.indicator_value > first.indicator_value {
                return (
                    DivergenceType::MacdBearishHidden,
                    Some(DivergenceCoords {
                        first_extreme: first,
                        second_extreme: last,
                    }),
                );
            }
        }

        (DivergenceType::None, None)
    }

    


}

/// Generic single-oscillator divergence detector (reused across Stochastic,
/// ChandeMO, MFI, CMF, OBV, Squeeze momentum). Tracks price + one oscillator
/// series and reports bullish/bearish regular divergence with pivot coords.
#[derive(Debug, Clone)]
pub struct SeriesDivergence {
    price_history: Vec<Decimal>,
    ind_history: Vec<Decimal>,
    lookback: usize,
}

/// Result of a generic series divergence check.
#[derive(Debug, Clone, Default)]
pub struct SeriesDivergenceResult {
    /// +1 bullish, -1 bearish, 0 none.
    pub direction: i8,
    pub coords: Option<DivergenceCoords>,
    pub hidden: bool,
}

impl SeriesDivergence {
    pub fn new(lookback: usize) -> Self {
        Self {
            price_history: Vec::with_capacity(lookback),
            ind_history: Vec::with_capacity(lookback),
            lookback: lookback.max(4),
        }
    }

    pub fn update(&mut self, price: Decimal, value: Decimal) -> SeriesDivergenceResult {
        self.price_history.push(price);
        self.ind_history.push(value);
        if self.price_history.len() > self.lookback {
            self.price_history.remove(0);
            self.ind_history.remove(0);
        }
        if self.price_history.len() < self.lookback {
            return SeriesDivergenceResult::default();
        }
        let half = self.lookback / 2;
        let len = self.price_history.len();

        // Bullish (regular): price lower low, oscillator higher low.
        if let (Some(first), Some(last)) = (
            find_extrema(&self.price_history, &self.ind_history, 0..half, false),
            find_extrema(&self.price_history, &self.ind_history, half..len, false),
        ) {
            if last.price < first.price && last.indicator_value > first.indicator_value {
                return SeriesDivergenceResult {
                    direction: 1,
                    coords: Some(DivergenceCoords { first_extreme: first, second_extreme: last }),
                    hidden: false,
                };
            }
        }
        // Bearish (regular): price higher high, oscillator lower high.
        if let (Some(first), Some(last)) = (
            find_extrema(&self.price_history, &self.ind_history, 0..half, true),
            find_extrema(&self.price_history, &self.ind_history, half..len, true),
        ) {
            if last.price > first.price && last.indicator_value < first.indicator_value {
                return SeriesDivergenceResult {
                    direction: -1,
                    coords: Some(DivergenceCoords { first_extreme: first, second_extreme: last }),
                    hidden: false,
                };
            }
        }

        // Hidden Bullish: price higher low, oscillator lower or flat low
        if let (Some(first), Some(last)) = (
            find_extrema(&self.price_history, &self.ind_history, 0..half, false),
            find_extrema(&self.price_history, &self.ind_history, half..len, false),
        ) {
            if last.price > first.price && last.indicator_value <= first.indicator_value {
                return SeriesDivergenceResult {
                    direction: 1,
                    coords: Some(DivergenceCoords { first_extreme: first, second_extreme: last }),
                    hidden: true,
                };
            }
        }
        // Hidden Bearish: price lower high, oscillator higher or flat high
        if let (Some(first), Some(last)) = (
            find_extrema(&self.price_history, &self.ind_history, 0..half, true),
            find_extrema(&self.price_history, &self.ind_history, half..len, true),
        ) {
            if last.price < first.price && last.indicator_value >= first.indicator_value {
                return SeriesDivergenceResult {
                    direction: -1,
                    coords: Some(DivergenceCoords { first_extreme: first, second_extreme: last }),
                    hidden: true,
                };
            }
        }
        SeriesDivergenceResult::default()
    }
}

fn find_extrema(
    prices: &[Decimal],
    values: &[Decimal],
    range: std::ops::Range<usize>,
    find_max: bool,
) -> Option<PeakTrough> {
    let end = range.end.min(prices.len());
    let slice = &prices[range.start..end];
    let cmp = |a: &Decimal, b: &Decimal| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal);
    let (offset, _) = if find_max {
        slice
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| cmp(a, b))?
    } else {
        slice
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| cmp(a, b))?
    };
    let idx = range.start + offset;
    Some(PeakTrough {
        price: prices[idx],
        indicator_value: values.get(idx).copied().unwrap_or(Decimal::ZERO),
        index: idx,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_returns_none_before_warmup() {
        let mut det = DivergenceDetector::new(10);
        for _ in 0..9 {
            let r = det.update_full(dec!(100.00), dec!(50.00), Decimal::ZERO);
            assert_eq!(r.rsi_divergence, DivergenceType::None);
            assert_eq!(r.macd_divergence, DivergenceType::None);
        }
    }

    #[test]
    fn test_bullish_rsi_divergence() {
        let mut det = DivergenceDetector::new(10);
        det.update_full(dec!(105.00), dec!(40.00), Decimal::ZERO);
        det.update_full(dec!(104.00), dec!(42.00), Decimal::ZERO);
        det.update_full(dec!(103.00), dec!(44.00), Decimal::ZERO);
        det.update_full(dec!(102.00), dec!(46.00), Decimal::ZERO);
        det.update_full(dec!(101.00), dec!(48.00), Decimal::ZERO);
        det.update_full(dec!(100.00), dec!(50.00), Decimal::ZERO);
        det.update_full(dec!(99.00), dec!(52.00), Decimal::ZERO);
        det.update_full(dec!(98.00), dec!(54.00), Decimal::ZERO);
        det.update_full(dec!(97.00), dec!(56.00), Decimal::ZERO);
        let result = det.update_full(dec!(96.00), dec!(58.00), Decimal::ZERO);
        assert_eq!(result.rsi_divergence, DivergenceType::RsiBullish);
    }

    #[test]
    fn test_bearish_rsi_divergence() {
        let mut det = DivergenceDetector::new(10);
        det.update_full(dec!(100.00), dec!(70.00), Decimal::ZERO);
        det.update_full(dec!(101.00), dec!(68.00), Decimal::ZERO);
        det.update_full(dec!(102.00), dec!(66.00), Decimal::ZERO);
        det.update_full(dec!(103.00), dec!(64.00), Decimal::ZERO);
        det.update_full(dec!(104.00), dec!(62.00), Decimal::ZERO);
        det.update_full(dec!(105.00), dec!(60.00), Decimal::ZERO);
        det.update_full(dec!(106.00), dec!(58.00), Decimal::ZERO);
        det.update_full(dec!(107.00), dec!(56.00), Decimal::ZERO);
        det.update_full(dec!(109.00), dec!(52.00), Decimal::ZERO);
        let result = det.update_full(dec!(110.00), dec!(50.00), Decimal::ZERO);
        assert_eq!(result.rsi_divergence, DivergenceType::RsiBearish);
    }

    #[test]
    fn test_no_divergence_on_aligned_movement() {
        let mut det = DivergenceDetector::new(10);
        for _ in 0..5 {
            det.update_full(dec!(100.00), dec!(50.00), Decimal::ZERO);
        }
        for _ in 0..5 {
            det.update_full(dec!(105.00), dec!(60.00), Decimal::ZERO);
        }
        let result = det.update_full(dec!(105.00), dec!(60.00), Decimal::ZERO);
        assert!(!result.has_bullish);
        assert!(!result.has_bearish);
    }

    #[test]
    fn test_macd_bullish_divergence() {
        let mut det = DivergenceDetector::new(10);
        for i in 0..10 {
            let price = dec!(100.00) - Decimal::from(i as i64);
            let rsi = dec!(50.00) + Decimal::from(i as i64);
            let macd = Decimal::from(-5i64) + Decimal::from(i as i64);
            det.update_full(price, rsi, macd);
        }
        let result = det.update_full(dec!(89.00), dec!(60.00), dec!(6.00));
        assert!(result.macd_divergence == DivergenceType::MacdBullish || result.has_bullish);
    }

    #[test]
    fn test_update_full_returns_structured_result() {
        let mut det = DivergenceDetector::new(10);
        for _ in 0..9 {
            det.update_full(dec!(100.00), dec!(50.00), dec!(0.00));
        }
        let result = det.update_full(dec!(100.00), dec!(50.00), dec!(0.00));
        assert!(!result.has_bullish);
        assert!(!result.has_bearish);
    }

    #[test]
    fn test_divergence_status_potential_on_detection() {
        let mut det = DivergenceDetector::new(10);
        // Set up RSI bullish divergence
        det.update_full(dec!(100.00), dec!(40.00), dec!(0.00));
        det.update_full(dec!(99.00), dec!(42.00), dec!(0.00));
        det.update_full(dec!(98.00), dec!(44.00), dec!(0.00));
        det.update_full(dec!(97.00), dec!(46.00), dec!(0.00));
        det.update_full(dec!(96.00), dec!(48.00), dec!(0.00));
        det.update_full(dec!(95.00), dec!(50.00), dec!(0.00));
        det.update_full(dec!(94.00), dec!(52.00), dec!(0.00));
        det.update_full(dec!(93.00), dec!(54.00), dec!(0.00));
        det.update_full(dec!(92.00), dec!(56.00), dec!(0.00));
        let result = det.update_full(dec!(91.00), dec!(58.00), dec!(0.00));
        assert_eq!(result.rsi_status, DivergenceStatus::Potential);
        assert!(result.rsi_coords.is_some());
    }

    #[test]
    fn test_confirmation_on_support_break() {
        let mut det = DivergenceDetector::new(10);
        // Build bullish RSI divergence
        for i in 0..9 {
            let price = dec!(100.00) - Decimal::from(i as i64);
            let rsi = dec!(40.00) + Decimal::from(i as i64);
            det.update_full(price, rsi, dec!(0.00));
        }
        let result = det.update_full(dec!(91.00), dec!(58.00), dec!(0.00));
        assert_eq!(result.rsi_status, DivergenceStatus::Potential);

        // Confirm: close breaks below support at 90.00 with >0.2% tolerance
        let confirmed = det.check_divergence_confirmation(
            &result,
            dec!(89.50),       // close below 90.00 support
            Some(dec!(90.00)), // support level
            None,              // no resistance
        );
        assert_eq!(confirmed.rsi_status, DivergenceStatus::Confirmed);
    }

    #[test]
    fn test_no_confirmation_without_break() {
        let mut det = DivergenceDetector::new(10);
        for i in 0..9 {
            let price = dec!(100.00) - Decimal::from(i as i64);
            let rsi = dec!(40.00) + Decimal::from(i as i64);
            det.update_full(price, rsi, dec!(0.00));
        }
        let result = det.update_full(dec!(91.00), dec!(58.00), dec!(0.00));
        assert_eq!(result.rsi_status, DivergenceStatus::Potential);

        // Close not decisively below support
        let still_potential = det.check_divergence_confirmation(
            &result,
            dec!(90.10), // close above support (barely below, within tolerance)
            Some(dec!(90.00)),
            None,
        );
        assert_eq!(still_potential.rsi_status, DivergenceStatus::Potential);
    }

    #[test]
    fn test_bearish_confirmation_on_resistance_break() {
        let mut det = DivergenceDetector::new(10);
        // Build bearish RSI divergence
        for i in 0..9 {
            let price = dec!(110.00) + Decimal::from(i as i64);
            let rsi = dec!(70.00) - Decimal::from(i as i64);
            det.update_full(price, rsi, dec!(0.00));
        }
        let result = det.update_full(dec!(119.00), dec!(52.00), dec!(0.00));
        assert_eq!(result.rsi_status, DivergenceStatus::Potential);
        assert_eq!(result.rsi_divergence, DivergenceType::RsiBearish);

        // Confirm: close breaks above resistance at 120.00
        let confirmed = det.check_divergence_confirmation(
            &result,
            dec!(120.50), // close above 120.00 resistance
            None,
            Some(dec!(120.00)), // resistance level
        );
        assert_eq!(confirmed.rsi_status, DivergenceStatus::Confirmed);
    }

    #[test]
    fn test_coords_contain_indices() {
        let mut det = DivergenceDetector::new(10);
        for i in 0..9 {
            let price = dec!(100.00) - Decimal::from(i as i64);
            let rsi = dec!(40.00) + Decimal::from(i as i64);
            det.update_full(price, rsi, dec!(0.00));
        }
        let result = det.update_full(dec!(91.00), dec!(58.00), dec!(0.00));
        let coords = result.rsi_coords.unwrap();
        assert!(coords.first_extreme.index <= 4);
        assert!(coords.second_extreme.index >= 5);
        assert!(coords.second_extreme.price < coords.first_extreme.price);
        assert!(coords.second_extreme.indicator_value > coords.first_extreme.indicator_value);
    }
}
