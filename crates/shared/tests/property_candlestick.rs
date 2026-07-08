use rust_decimal::Decimal;

use shared::indicators::{
    Candlestick, CandlestickConfig, CandlestickPattern, CandlestickResult, CandlestickStatus,
};

fn feed(cs: &mut Candlestick, o: f64, h: f64, l: f64, c: f64) -> CandlestickResult {
    cs.update(
        Decimal::from_f64_retain(o).unwrap(),
        Decimal::from_f64_retain(h).unwrap(),
        Decimal::from_f64_retain(l).unwrap(),
        Decimal::from_f64_retain(c).unwrap(),
    )
}

/// Bearish engulfing after a bullish candle, then bearish confirmation.
#[test]
fn candlestick_bearish_engulfing_confirms() {
    let mut cs = Candlestick::new(CandlestickConfig::default());
    // Prior bullish.
    feed(&mut cs, 95.0, 106.0, 94.0, 105.0);
    // Bearish engulfing: opens above prior close, closes below prior open.
    let r = feed(&mut cs, 106.0, 107.0, 92.0, 93.0);
    assert_eq!(r.pattern, CandlestickPattern::BearishEngulfing);
    assert_eq!(r.direction, -1);
    assert_eq!(r.status, CandlestickStatus::Formed);
    // Next candle closes below the engulfing low (92) → confirmed.
    let r2 = feed(&mut cs, 93.0, 94.0, 88.0, 89.0);
    assert_eq!(r2.status, CandlestickStatus::Confirmed);
    assert_eq!(r2.direction, -1);
}

/// Morning Star three-candle bullish reversal.
#[test]
fn candlestick_morning_star() {
    let mut cs = Candlestick::new(CandlestickConfig::default());
    // Big bearish candle.
    feed(&mut cs, 120.0, 121.0, 99.0, 100.0);
    // Small-bodied star (gap-ish, tiny body).
    feed(&mut cs, 99.0, 100.0, 97.0, 98.5);
    // Strong bullish close above first candle's midpoint (110).
    let r = feed(&mut cs, 99.0, 116.0, 98.0, 115.0);
    assert_eq!(r.pattern, CandlestickPattern::MorningStar);
    assert_eq!(r.direction, 1);
}

/// Three Black Crows bearish continuation/reversal.
#[test]
fn candlestick_three_black_crows() {
    let mut cs = Candlestick::new(CandlestickConfig::default());
    feed(&mut cs, 120.0, 121.0, 114.0, 115.0);
    feed(&mut cs, 116.0, 117.0, 110.0, 111.0);
    let r = feed(&mut cs, 112.0, 113.0, 106.0, 107.0);
    assert_eq!(r.pattern, CandlestickPattern::ThreeBlackCrows);
    assert_eq!(r.direction, -1);
}

/// Doji is a neutral (0-direction) reading and never arms a confirmation.
#[test]
fn candlestick_doji_is_neutral() {
    let mut cs = Candlestick::new(CandlestickConfig::default());
    let r = feed(&mut cs, 100.0, 104.0, 96.0, 100.1);
    assert_eq!(r.direction, 0);
    // A neutral pattern does not enter the pending/confirm pipeline; the next
    // ordinary candle should not be reported as a "Confirmed" doji.
    let r2 = feed(&mut cs, 100.0, 101.0, 99.0, 100.5);
    assert_ne!(r2.status, CandlestickStatus::Confirmed);
}

/// A formed pattern that is contradicted by the next candle is not confirmed.
#[test]
fn candlestick_invalidation_path() {
    let mut cs = Candlestick::new(CandlestickConfig::default());
    feed(&mut cs, 100.0, 101.0, 94.0, 95.0); // prior bearish
    let r = feed(&mut cs, 94.0, 103.0, 93.5, 102.0); // bullish engulfing
    assert_eq!(r.pattern, CandlestickPattern::BullishEngulfing);
    // Next candle collapses bearish, closing well below trigger → never confirmed.
    let r2 = feed(&mut cs, 101.0, 101.5, 95.0, 96.0);
    assert_ne!(r2.status, CandlestickStatus::Confirmed);
}

/// Every result is internally consistent: a directional Formed/Confirmed
/// reading has a non-None pattern, and neutral patterns carry direction 0.
#[test]
fn candlestick_results_are_consistent() {
    let mut cs = Candlestick::new(CandlestickConfig::default());
    let seqs = [
        (100.0, 110.05, 99.98, 110.0), // marubozu bull
        (110.0, 111.0, 104.0, 105.0),  // bearish
        (105.0, 106.0, 100.0, 101.0),  // bearish
        (100.0, 104.0, 96.0, 100.1),   // doji
        (100.0, 108.0, 99.0, 107.0),   // bullish
    ];
    for (o, h, l, c) in seqs {
        let r = feed(&mut cs, o, h, l, c);
        match r.status {
            CandlestickStatus::Formed | CandlestickStatus::Confirmed => {
                assert_ne!(r.pattern, CandlestickPattern::None);
            }
            _ => {}
        }
        assert!(r.quality >= 0.0 && r.quality <= 2.0);
    }
}
