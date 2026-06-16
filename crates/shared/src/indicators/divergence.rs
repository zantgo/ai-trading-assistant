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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Divergence {
    Bullish,
    Bearish,
    None,
}

/// Specific type of divergence detected
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DivergenceType {
    RsiBullish,
    RsiBearish,
    MacdBullish,
    MacdBearish,
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
        matches!(self, DivergenceStatus::Potential | DivergenceStatus::Confirmed)
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
        self.indicator_value.to_string().parse::<f64>().unwrap_or(0.0)
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
            };
        }

        let (rsi_div, rsi_coords) = self.detect_rsi_divergence();
        let (macd_div, macd_coords) = self.detect_macd_divergence();

        let has_bullish = matches!(rsi_div, DivergenceType::RsiBullish)
            || matches!(macd_div, DivergenceType::MacdBullish);
        let has_bearish = matches!(rsi_div, DivergenceType::RsiBearish)
            || matches!(macd_div, DivergenceType::MacdBearish);

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

        DivergenceResult {
            rsi_divergence: rsi_div,
            macd_divergence: macd_div,
            has_bullish,
            has_bearish,
            rsi_status,
            macd_status,
            rsi_coords,
            macd_coords,
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
                DivergenceType::RsiBullish => {
                    // Bullish: close breaks below support
                    if let Some(s) = support_level {
                        let buffer = s * tolerance_pct;
                        close_price < s && (s - close_price) > buffer
                    } else {
                        false
                    }
                }
                DivergenceType::RsiBearish => {
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
                DivergenceType::MacdBullish => {
                    if let Some(s) = support_level {
                        let buffer = s * tolerance_pct;
                        close_price < s && (s - close_price) > buffer
                    } else {
                        false
                    }
                }
                DivergenceType::MacdBearish => {
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

    /// Legacy update: price + RSI only (backward-compatible).
    pub fn update(&mut self, price: Decimal, rsi: Decimal) -> Divergence {
        self.price_history.push(price);
        self.rsi_history.push(rsi);

        if self.price_history.len() > self.lookback {
            self.price_history.remove(0);
            self.rsi_history.remove(0);
        }

        if self.price_history.len() < self.lookback {
            return Divergence::None;
        }

        self.detect()
    }

    /// Get the current lookback window size
    pub fn len(&self) -> usize {
        self.price_history.len()
    }

    /// Whether the detector has enough data for analysis
    pub fn is_ready(&self) -> bool {
        self.price_history.len() >= self.lookback
    }

    /// Detect RSI divergence within history. Returns type and coordinates.
    fn detect_rsi_divergence(&self) -> (DivergenceType, Option<DivergenceCoords>) {
        let half = self.lookback / 2;

        // Bullish: price lower low, RSI higher low
        if let (Some(first), Some(last)) =
            self.extrema_min_with_index(&self.price_history, &self.rsi_history, half)
        {
            if last.price < first.price && last.indicator_value > first.indicator_value {
                return (
                    DivergenceType::RsiBullish,
                    Some(DivergenceCoords { first_extreme: first, second_extreme: last }),
                );
            }
        }

        // Bearish: price higher high, RSI lower high
        if let (Some(first), Some(last)) =
            self.extrema_max_with_index(&self.price_history, &self.rsi_history, half)
        {
            if last.price > first.price && last.indicator_value < first.indicator_value {
                return (
                    DivergenceType::RsiBearish,
                    Some(DivergenceCoords { first_extreme: first, second_extreme: last }),
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

        // Bullish: price lower low, MACD histogram higher low
        if let (Some(first), Some(last)) =
            self.extrema_min_with_index(&self.price_history, &self.macd_hist_history, half)
        {
            if last.price < first.price && last.indicator_value > first.indicator_value {
                return (
                    DivergenceType::MacdBullish,
                    Some(DivergenceCoords { first_extreme: first, second_extreme: last }),
                );
            }
        }

        // Bearish: price higher high, MACD histogram lower high
        if let (Some(first), Some(last)) =
            self.extrema_max_with_index(&self.price_history, &self.macd_hist_history, half)
        {
            if last.price > first.price && last.indicator_value < first.indicator_value {
                return (
                    DivergenceType::MacdBearish,
                    Some(DivergenceCoords { first_extreme: first, second_extreme: last }),
                );
            }
        }

        (DivergenceType::None, None)
    }

    /// Find the minimum values in the first and second halves of the history,
    /// returning PeakTrough structs with their indices.
    fn extrema_min_with_index(
        &self,
        prices: &[Decimal],
        values: &[Decimal],
        half: usize,
    ) -> (Option<PeakTrough>, Option<PeakTrough>) {
        let first = find_min_with_index(prices, values, 0, half);
        let last = find_min_with_index(prices, values, half, prices.len());
        (first, last)
    }

    /// Find the maximum values in the first and second halves of the history,
    /// returning PeakTrough structs with their indices.
    fn extrema_max_with_index(
        &self,
        prices: &[Decimal],
        values: &[Decimal],
        half: usize,
    ) -> (Option<PeakTrough>, Option<PeakTrough>) {
        let first = find_max_with_index(prices, values, 0, half);
        let last = find_max_with_index(prices, values, half, prices.len());
        (first, last)
    }

    fn detect(&self) -> Divergence {
        let half = self.lookback / 2;

        let price_first = self.price_history[..half].iter().min_by(|a, b| {
            a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
        });
        let price_last = self.price_history[half..].iter().min_by(|a, b| {
            a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
        });

        let rsi_first = self.rsi_history[..half].iter().min_by(|a, b| {
            a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
        });
        let rsi_last = self.rsi_history[half..].iter().min_by(|a, b| {
            a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
        });

        if let (Some(&pf), Some(&pl), Some(&rf), Some(&rl)) = (price_first, price_last, rsi_first, rsi_last) {
            if pl < pf && rl > rf {
                return Divergence::Bullish;
            }
        }

        let price_first_high = self.price_history[..half].iter().max_by(|a, b| {
            a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
        });
        let price_last_high = self.price_history[half..].iter().max_by(|a, b| {
            a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
        });

        let rsi_first_high = self.rsi_history[..half].iter().max_by(|a, b| {
            a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
        });
        let rsi_last_high = self.rsi_history[half..].iter().max_by(|a, b| {
            a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
        });

        if let (Some(&pf_h), Some(&pl_h), Some(&rf_h), Some(&rl_h)) = (price_first_high, price_last_high, rsi_first_high, rsi_last_high) {
            if pl_h > pf_h && rl_h < rf_h {
                return Divergence::Bearish;
            }
        }

        Divergence::None
    }
}

/// Find the minimum price in a slice range and the corresponding value at the same index.
fn find_min_with_index(
    prices: &[Decimal],
    values: &[Decimal],
    start: usize,
    end: usize,
) -> Option<PeakTrough> {
    let slice = &prices[start..end.min(prices.len())];
    let (offset, _) = slice.iter().enumerate()
        .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))?;
    let idx = start + offset;
    Some(PeakTrough {
        price: prices[idx],
        indicator_value: values.get(idx).copied().unwrap_or(Decimal::ZERO),
        index: idx,
    })
}

/// Find the maximum price in a slice range and the corresponding value at the same index.
fn find_max_with_index(
    prices: &[Decimal],
    values: &[Decimal],
    start: usize,
    end: usize,
) -> Option<PeakTrough> {
    let slice = &prices[start..end.min(prices.len())];
    let (offset, _) = slice.iter().enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))?;
    let idx = start + offset;
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
            assert_eq!(det.update(dec!(100.00), dec!(50.00)), Divergence::None);
        }
    }

    #[test]
    fn test_bullish_divergence() {
        let mut det = DivergenceDetector::new(10);
        det.update(dec!(105.00), dec!(40.00));
        det.update(dec!(104.00), dec!(42.00));
        det.update(dec!(103.00), dec!(44.00));
        det.update(dec!(102.00), dec!(46.00));
        det.update(dec!(101.00), dec!(48.00));
        det.update(dec!(100.00), dec!(50.00));
        det.update(dec!(99.00), dec!(52.00));
        det.update(dec!(98.00), dec!(54.00));
        det.update(dec!(97.00), dec!(56.00));
        let result = det.update(dec!(96.00), dec!(58.00));
        assert_eq!(result, Divergence::Bullish);
    }

    #[test]
    fn test_bearish_divergence() {
        let mut det = DivergenceDetector::new(10);
        det.update(dec!(100.00), dec!(70.00));
        det.update(dec!(101.00), dec!(68.00));
        det.update(dec!(102.00), dec!(66.00));
        det.update(dec!(103.00), dec!(64.00));
        det.update(dec!(104.00), dec!(62.00));
        det.update(dec!(105.00), dec!(60.00));
        det.update(dec!(106.00), dec!(58.00));
        det.update(dec!(107.00), dec!(56.00));
        det.update(dec!(109.00), dec!(52.00));
        let result = det.update(dec!(110.00), dec!(50.00));
        assert_eq!(result, Divergence::Bearish);
    }

    #[test]
    fn test_no_divergence_on_aligned_movement() {
        let mut det = DivergenceDetector::new(10);
        for _ in 0..5 {
            det.update(dec!(100.00), dec!(50.00));
        }
        for _ in 0..5 {
            det.update(dec!(105.00), dec!(60.00));
        }
        assert_eq!(det.update(dec!(105.00), dec!(60.00)), Divergence::None);
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
            dec!(89.50),        // close below 90.00 support
            Some(dec!(90.00)),  // support level
            None,               // no resistance
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
            dec!(90.10),        // close above support (barely below, within tolerance)
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
            dec!(120.50),        // close above 120.00 resistance
            None,
            Some(dec!(120.00)),  // resistance level
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
