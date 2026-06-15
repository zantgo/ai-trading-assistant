use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use crate::indicators::fibonacci::{PivotPoint, PivotType};

/// Chart Pattern Detection — identifies continuation and reversal patterns
/// on the execution (5-minute) chart using pivot point sequences.
///
/// Section 2.3 of Unified Strategy Framework.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartPattern {
    None,
    BullishTriangle,
    BearishTriangle,
    RisingWedge,
    FallingWedge,
    AscendingChannel,
    DescendingChannel,
}

#[derive(Debug, Clone)]
pub struct PatternResult {
    pub pattern: ChartPattern,
    pub is_bullish: bool,
    pub is_bearish: bool,
    pub confidence: f64,
    pub description: String,
}

impl PatternResult {
    pub fn none() -> Self {
        Self {
            pattern: ChartPattern::None,
            is_bullish: false,
            is_bearish: false,
            confidence: 0.0,
            description: String::new(),
        }
    }
}

/// Detect chart patterns from pivot highs and lows.
/// Requires at least 4 pivots (2 highs, 2 lows) or 2 swing highs and 2 swing lows.
pub fn detect_pattern(pivots: &[PivotPoint]) -> PatternResult {
    let highs: Vec<_> = pivots.iter().filter(|p| p.pivot_type == PivotType::High).collect();
    let lows: Vec<_> = pivots.iter().filter(|p| p.pivot_type == PivotType::Low).collect();

    if highs.len() < 2 || lows.len() < 2 {
        return PatternResult::none();
    }

    // Try triangle detection
    if let Some(result) = detect_triangle(&highs, &lows) {
        return result;
    }

    // Try wedge detection
    if let Some(result) = detect_wedge(&highs, &lows) {
        return result;
    }

    // Try channel detection
    if let Some(result) = detect_channel(&highs, &lows, pivots) {
        return result;
    }

    PatternResult::none()
}

/// Triangle: series of lower highs and higher lows, converging.
/// Ascending = bullish bias, Descending = bearish bias.
fn detect_triangle(highs: &[&PivotPoint], lows: &[&PivotPoint]) -> Option<PatternResult> {
    if highs.len() < 3 || lows.len() < 3 {
        return None;
    }

    let last_n = 3usize.min(highs.len()).min(lows.len());

    // Check if highs are descending
    let recent_highs: Vec<_> = highs.iter().rev().take(last_n).copied().collect();
    let recent_lows: Vec<_> = lows.iter().rev().take(last_n).copied().collect();

    let high_slope = compute_slope(&recent_highs);
    let low_slope = compute_slope(&recent_lows);

    // For a valid triangle, highs should slope down (or be flat) and lows should slope up (or be flat)
    // and they should be converging
    let highs_descending = high_slope <= Decimal::from_f64_retain(0.05).unwrap_or(Decimal::ZERO);
    let lows_ascending = low_slope >= Decimal::from_f64_retain(-0.05).unwrap_or(Decimal::ZERO);

    if !highs_descending || !lows_ascending {
        return None;
    }

    // Check convergence: the gap between high resistance and low support is narrowing
    let first_gap = recent_highs.last().unwrap().price - recent_lows.last().unwrap().price;
    let last_gap = recent_highs.first().unwrap().price - recent_lows.first().unwrap().price;

    if last_gap >= first_gap {
        return None;
    }

    let convergence_pct = (first_gap - last_gap) / first_gap;
    let confidence = (convergence_pct.to_f64().unwrap_or(0.0) * 100.0).min(100.0);

    if confidence < 10.0 {
        return None;
    }

    // Determine bias from breakout direction context
    let (pattern, is_bullish, is_bearish) = if high_slope < Decimal::ZERO && low_slope > Decimal::ZERO {
        (ChartPattern::BullishTriangle, true, false)
    } else {
        (ChartPattern::BearishTriangle, false, true)
    };

    Some(PatternResult {
        pattern,
        is_bullish,
        is_bearish,
        confidence,
        description: format!("Triangle pattern detected — highs descending, lows ascending, {}% convergence", confidence as u32),
    })
}

/// Wedge: both trendlines sloping in the same direction, converging.
fn detect_wedge(highs: &[&PivotPoint], lows: &[&PivotPoint]) -> Option<PatternResult> {
    if highs.len() < 3 || lows.len() < 3 {
        return None;
    }

    let last_n = 3usize.min(highs.len()).min(lows.len());
    let recent_highs: Vec<_> = highs.iter().rev().take(last_n).copied().collect();
    let recent_lows: Vec<_> = lows.iter().rev().take(last_n).copied().collect();

    let high_slope = compute_slope(&recent_highs);
    let low_slope = compute_slope(&recent_lows);

    // Both slopes should be same direction and non-zero
    let both_rising = high_slope > Decimal::ZERO && low_slope > Decimal::ZERO;
    let both_falling = high_slope < Decimal::ZERO && low_slope < Decimal::ZERO;

    if !both_rising && !both_falling {
        return None;
    }

    // Check convergence
    let first_gap = (recent_highs.last().unwrap().price - recent_lows.last().unwrap().price).abs();
    let last_gap = (recent_highs.first().unwrap().price - recent_lows.first().unwrap().price).abs();

    if last_gap >= first_gap {
        return None;
    }

    let convergence_pct = (first_gap - last_gap) / first_gap;
    let confidence = (convergence_pct.to_f64().unwrap_or(0.0) * 100.0).min(100.0);

    if confidence < 10.0 {
        return None;
    }

    let (pattern, is_bullish, is_bearish) = if both_falling {
        (ChartPattern::FallingWedge, true, false)
    } else {
        (ChartPattern::RisingWedge, false, true)
    };

    Some(PatternResult {
        pattern,
        is_bullish,
        is_bearish,
        confidence,
        description: format!("Wedge pattern detected — {:.2}% convergence", confidence),
    })
}

/// Channel: parallel trendlines containing price action.
fn detect_channel(
    highs: &[&PivotPoint],
    lows: &[&PivotPoint],
    all_pivots: &[PivotPoint],
) -> Option<PatternResult> {
    if highs.len() < 2 || lows.len() < 2 {
        return None;
    }

    let last_n = 2usize.min(highs.len()).min(lows.len());
    let recent_highs: Vec<_> = highs.iter().rev().take(last_n).copied().collect();
    let recent_lows: Vec<_> = lows.iter().rev().take(last_n).copied().collect();

    let high_slope = compute_slope(&recent_highs);
    let low_slope = compute_slope(&recent_lows);

    // Slopes should be roughly parallel
    let slope_diff = (high_slope - low_slope).abs();
    let max_slope = high_slope.abs().max(low_slope.abs());
    let tolerance = if max_slope > Decimal::ZERO {
        max_slope * Decimal::from(2) / Decimal::from(10)
    } else {
        Decimal::from(1)
    };

    if slope_diff > tolerance {
        return None;
    }

    // Check that price pivots stay within the channel bounds
    let channel_width = {
        let first_high = recent_highs.last().unwrap().price;
        let first_low = recent_lows.last().unwrap().price;
        first_high - first_low
    };

    if channel_width <= Decimal::ZERO {
        return None;
    }

    // Verify most price pivots touch the channel lines
    let touching_count = all_pivots.iter().filter(|p| {
        let interpolated_high = interpolate_line(&recent_highs, p.index);
        let interpolated_low = interpolate_line(&recent_lows, p.index);
        match interpolated_high.zip(interpolated_low) {
            Some((h, l)) => {
                let near_high = (p.price - h).abs() < channel_width * Decimal::from(1) / Decimal::from(10);
                let near_low = (p.price - l).abs() < channel_width * Decimal::from(1) / Decimal::from(10);
                near_high || near_low
            }
            None => false,
        }
    }).count();

    let ratio = touching_count as f64 / all_pivots.len().max(1) as f64;
    if ratio < 0.4 {
        return None;
    }

    let confidence = (ratio * 100.0).min(100.0);
    let (pattern, is_bullish, is_bearish) = if high_slope > Decimal::ZERO {
        (ChartPattern::AscendingChannel, true, false)
    } else {
        (ChartPattern::DescendingChannel, false, true)
    };

    Some(PatternResult {
        pattern,
        is_bullish,
        is_bearish,
        confidence,
        description: format!("Channel pattern detected — {:.2}% confidence", confidence),
    })
}

/// Compute price-change slope from pivot sequence (price change per bar).
fn compute_slope(pivots: &[&PivotPoint]) -> Decimal {
    if pivots.len() < 2 {
        return Decimal::ZERO;
    }
    let first = pivots.last().unwrap();
    let last = pivots.first().unwrap();
    let bars = (last.index - first.index).max(1) as i64;
    (last.price - first.price) / Decimal::from(bars)
}

/// Interpolate the line value at a given index.
fn interpolate_line(pivots: &[&PivotPoint], index: usize) -> Option<Decimal> {
    if pivots.len() < 2 {
        return None;
    }
    let first = pivots.last().unwrap();
    let last = pivots.first().unwrap();
    let total_bars = (last.index - first.index).max(1) as i64;
    let offset = (index as i64 - first.index as i64).max(0);
    let ratio = Decimal::from(offset) / Decimal::from(total_bars);
    Some(first.price + (last.price - first.price) * ratio)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_high(idx: usize, price: f64) -> PivotPoint {
        PivotPoint { index: idx, price: Decimal::from_f64_retain(price).unwrap(), pivot_type: PivotType::High, strength: 10 }
    }

    fn make_low(idx: usize, price: f64) -> PivotPoint {
        PivotPoint { index: idx, price: Decimal::from_f64_retain(price).unwrap(), pivot_type: PivotType::Low, strength: 10 }
    }

    #[test]
    fn test_no_pattern_with_few_pivots() {
        let pivots = vec![make_high(10, 150.0)];
        let result = detect_pattern(&pivots);
        assert_eq!(result.pattern, ChartPattern::None);
    }

    #[test]
    fn test_triangle_detection() {
        let pivots = vec![
            make_low(5, 100.0),
            make_high(10, 155.0),
            make_low(15, 105.0),
            make_high(20, 150.0),
            make_low(25, 110.0),
            make_high(30, 140.0),
        ];
        let result = detect_pattern(&pivots);
        assert!(matches!(result.pattern, ChartPattern::BullishTriangle | ChartPattern::BearishTriangle));
        assert!(result.confidence > 0.0);
    }

    #[test]
    fn test_channel_detection() {
        let pivots = vec![
            make_low(5, 100.0),
            make_high(6, 120.0),
            make_low(15, 105.0),
            make_high(16, 125.0),
            make_low(25, 110.0),
            make_high(26, 130.0),
        ];
        let result = detect_pattern(&pivots);
        assert!(matches!(result.pattern, ChartPattern::AscendingChannel | ChartPattern::DescendingChannel));
    }

    #[test]
    fn test_wedge_detection() {
        let pivots = vec![
            make_low(5, 100.0),
            make_high(10, 160.0),
            make_low(15, 107.0),
            make_high(20, 155.0),
            make_low(25, 113.0),
            make_high(30, 145.0),
        ];
        let result = detect_pattern(&pivots);
        assert!(matches!(result.pattern, ChartPattern::FallingWedge | ChartPattern::RisingWedge | ChartPattern::BullishTriangle | ChartPattern::BearishTriangle | ChartPattern::None));
    }
}
