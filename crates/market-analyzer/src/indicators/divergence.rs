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
        price: f64,
        rsi: f64,
        macd_histogram: f64,
    ) -> DivergenceResult {
        let price = Decimal::from_f64_retain(price).unwrap_or(Decimal::ZERO);
        let rsi = Decimal::from_f64_retain(rsi).unwrap_or(Decimal::ZERO);
        let macd_histogram = Decimal::from_f64_retain(macd_histogram).unwrap_or(Decimal::ZERO);
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

        let has_bullish = matches!(
            rsi_div,
            DivergenceType::RsiBullish | DivergenceType::RsiBullishHidden
        ) || matches!(
            macd_div,
            DivergenceType::MacdBullish | DivergenceType::MacdBullishHidden
        );
        let has_bearish = matches!(
            rsi_div,
            DivergenceType::RsiBearish | DivergenceType::RsiBearishHidden
        ) || matches!(
            macd_div,
            DivergenceType::MacdBearish | DivergenceType::MacdBearishHidden
        );

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

        let rsi_hidden = matches!(
            rsi_div,
            DivergenceType::RsiBullishHidden | DivergenceType::RsiBearishHidden
        );
        let macd_hidden = matches!(
            macd_div,
            DivergenceType::MacdBullishHidden | DivergenceType::MacdBearishHidden
        );

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
        close_price: f64,
        support_level: Option<f64>,
        resistance_level: Option<f64>,
    ) -> DivergenceResult {
        let close_price = Decimal::from_f64_retain(close_price).unwrap_or(Decimal::ZERO);
        let support_level =
            support_level.map(|s| Decimal::from_f64_retain(s).unwrap_or(Decimal::ZERO));
        let resistance_level =
            resistance_level.map(|r| Decimal::from_f64_retain(r).unwrap_or(Decimal::ZERO));
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
            find_extrema(
                &self.price_history,
                &self.macd_hist_history,
                half..len,
                false,
            ),
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
            find_extrema(
                &self.price_history,
                &self.macd_hist_history,
                half..len,
                true,
            ),
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
            find_extrema(
                &self.price_history,
                &self.macd_hist_history,
                half..len,
                false,
            ),
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
            find_extrema(
                &self.price_history,
                &self.macd_hist_history,
                half..len,
                true,
            ),
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

    pub fn update(&mut self, price: f64, value: f64) -> SeriesDivergenceResult {
        let price = Decimal::from_f64_retain(price).unwrap_or(Decimal::ZERO);
        let value = Decimal::from_f64_retain(value).unwrap_or(Decimal::ZERO);
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
                    coords: Some(DivergenceCoords {
                        first_extreme: first,
                        second_extreme: last,
                    }),
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
                    coords: Some(DivergenceCoords {
                        first_extreme: first,
                        second_extreme: last,
                    }),
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
                    coords: Some(DivergenceCoords {
                        first_extreme: first,
                        second_extreme: last,
                    }),
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
                    coords: Some(DivergenceCoords {
                        first_extreme: first,
                        second_extreme: last,
                    }),
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

    #[test]
    fn test_returns_none_before_warmup() {
        let mut det = DivergenceDetector::new(10);
        for _ in 0..9 {
            let r = det.update_full(100.0, 50.0, 0.0);
            assert_eq!(r.rsi_divergence, DivergenceType::None);
            assert_eq!(r.macd_divergence, DivergenceType::None);
        }
    }

    #[test]
    fn test_bullish_rsi_divergence() {
        let mut det = DivergenceDetector::new(10);
        det.update_full(105.0, 40.0, 0.0);
        det.update_full(104.0, 42.0, 0.0);
        det.update_full(103.0, 44.0, 0.0);
        det.update_full(102.0, 46.0, 0.0);
        det.update_full(101.0, 48.0, 0.0);
        det.update_full(100.0, 50.0, 0.0);
        det.update_full(99.0, 52.0, 0.0);
        det.update_full(98.0, 54.0, 0.0);
        det.update_full(97.0, 56.0, 0.0);
        let result = det.update_full(96.0, 58.0, 0.0);
        assert_eq!(result.rsi_divergence, DivergenceType::RsiBullish);
    }

    #[test]
    fn test_bearish_rsi_divergence() {
        let mut det = DivergenceDetector::new(10);
        det.update_full(100.0, 70.0, 0.0);
        det.update_full(101.0, 68.0, 0.0);
        det.update_full(102.0, 66.0, 0.0);
        det.update_full(103.0, 64.0, 0.0);
        det.update_full(104.0, 62.0, 0.0);
        det.update_full(105.0, 60.0, 0.0);
        det.update_full(106.0, 58.0, 0.0);
        det.update_full(107.0, 56.0, 0.0);
        det.update_full(109.0, 52.0, 0.0);
        let result = det.update_full(110.0, 50.0, 0.0);
        assert_eq!(result.rsi_divergence, DivergenceType::RsiBearish);
    }

    #[test]
    fn test_no_divergence_on_aligned_movement() {
        let mut det = DivergenceDetector::new(10);
        for _ in 0..5 {
            det.update_full(100.0, 50.0, 0.0);
        }
        for _ in 0..5 {
            det.update_full(105.0, 60.0, 0.0);
        }
        let result = det.update_full(105.0, 60.0, 0.0);
        assert!(!result.has_bullish);
        assert!(!result.has_bearish);
    }

    #[test]
    fn test_macd_bullish_divergence() {
        let mut det = DivergenceDetector::new(10);
        for i in 0..10 {
            let price = 100.0 - i as f64;
            let rsi = 50.0 + i as f64;
            let macd = -5.0 + i as f64;
            det.update_full(price, rsi, macd);
        }
        let result = det.update_full(89.0, 60.0, 6.0);
        assert!(result.macd_divergence == DivergenceType::MacdBullish || result.has_bullish);
    }

    #[test]
    fn test_update_full_returns_structured_result() {
        let mut det = DivergenceDetector::new(10);
        for _ in 0..9 {
            det.update_full(100.0, 50.0, 0.0);
        }
        let result = det.update_full(100.0, 50.0, 0.0);
        assert!(!result.has_bullish);
        assert!(!result.has_bearish);
    }

    #[test]
    fn test_divergence_status_potential_on_detection() {
        let mut det = DivergenceDetector::new(10);
        det.update_full(100.0, 40.0, 0.0);
        det.update_full(99.0, 42.0, 0.0);
        det.update_full(98.0, 44.0, 0.0);
        det.update_full(97.0, 46.0, 0.0);
        det.update_full(96.0, 48.0, 0.0);
        det.update_full(95.0, 50.0, 0.0);
        det.update_full(94.0, 52.0, 0.0);
        det.update_full(93.0, 54.0, 0.0);
        det.update_full(92.0, 56.0, 0.0);
        let result = det.update_full(91.0, 58.0, 0.0);
        assert_eq!(result.rsi_status, DivergenceStatus::Potential);
        assert!(result.rsi_coords.is_some());
    }

    #[test]
    fn test_confirmation_on_support_break() {
        let mut det = DivergenceDetector::new(10);
        for i in 0..9 {
            let price = 100.0 - i as f64;
            let rsi = 40.0 + i as f64;
            det.update_full(price, rsi, 0.0);
        }
        let result = det.update_full(91.0, 58.0, 0.0);
        assert_eq!(result.rsi_status, DivergenceStatus::Potential);

        let confirmed = det.check_divergence_confirmation(
            &result,
            89.50,
            Some(90.00),
            None,
        );
        assert_eq!(confirmed.rsi_status, DivergenceStatus::Confirmed);
    }

    #[test]
    fn test_no_confirmation_without_break() {
        let mut det = DivergenceDetector::new(10);
        for i in 0..9 {
            let price = 100.0 - i as f64;
            let rsi = 40.0 + i as f64;
            det.update_full(price, rsi, 0.0);
        }
        let result = det.update_full(91.0, 58.0, 0.0);
        assert_eq!(result.rsi_status, DivergenceStatus::Potential);

        let still_potential = det.check_divergence_confirmation(
            &result,
            90.10,
            Some(90.00),
            None,
        );
        assert_eq!(still_potential.rsi_status, DivergenceStatus::Potential);
    }

    #[test]
    fn test_bearish_confirmation_on_resistance_break() {
        let mut det = DivergenceDetector::new(10);
        for i in 0..9 {
            let price = 110.0 + i as f64;
            let rsi = 70.0 - i as f64;
            det.update_full(price, rsi, 0.0);
        }
        let result = det.update_full(119.0, 52.0, 0.0);
        assert_eq!(result.rsi_status, DivergenceStatus::Potential);
        assert_eq!(result.rsi_divergence, DivergenceType::RsiBearish);

        let confirmed = det.check_divergence_confirmation(
            &result,
            120.50,
            None,
            Some(120.00),
        );
        assert_eq!(confirmed.rsi_status, DivergenceStatus::Confirmed);
    }

    #[test]
    fn test_coords_contain_indices() {
        let mut det = DivergenceDetector::new(10);
        for i in 0..9 {
            let price = 100.0 - i as f64;
            let rsi = 40.0 + i as f64;
            det.update_full(price, rsi, 0.0);
        }
        let result = det.update_full(91.0, 58.0, 0.0);
        let coords = result.rsi_coords.unwrap();
        assert!(coords.first_extreme.index <= 4);
        assert!(coords.second_extreme.index >= 5);
        assert!(coords.second_extreme.price < coords.first_extreme.price);
        assert!(coords.second_extreme.indicator_value > coords.first_extreme.indicator_value);
    }
}
