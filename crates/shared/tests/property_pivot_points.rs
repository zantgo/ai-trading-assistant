use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use shared::indicators::{PivotMethod, PivotPoints};

/// Levels are None until the first full prior session completes, then published.
#[test]
fn pivots_none_until_first_session_finalizes() {
    let mut pp = PivotPoints::new(PivotMethod::Classic);
    assert!(pp.update(dec!(110), dec!(90), dec!(100), 0).is_none());
    assert!(pp.update(dec!(115), dec!(95), dec!(108), 0).is_none());
    // Day rollover finalizes day 0.
    assert!(pp.update(dec!(109), dec!(101), dec!(105), 1).is_some());
}

/// Classic level ordering S3<S2<S1<P<R1<R2<R3 must always hold.
#[test]
fn pivots_classic_level_ordering() {
    // A spread of prior-session H/L/C shapes.
    let cases = [
        (dec!(120), dec!(80), dec!(100)),
        (dec!(200), dec!(150), dec!(180)),
        (dec!(1.5), dec!(1.0), dec!(1.2)),
        (dec!(50000), dec!(48000), dec!(49500)),
    ];
    for (h, l, c) in cases {
        let mut pp = PivotPoints::new(PivotMethod::Classic);
        pp.update(h, l, c, 0);
        let lv = pp.update(c, c, c, 1).expect("levels published after rollover");
        assert!(lv.s3 < lv.s2, "S3<S2 failed for H={} L={} C={}", h, l, c);
        assert!(lv.s2 < lv.s1, "S2<S1 failed");
        assert!(lv.s1 < lv.pivot, "S1<P failed");
        assert!(lv.pivot < lv.r1, "P<R1 failed");
        assert!(lv.r1 < lv.r2, "R1<R2 failed");
        assert!(lv.r2 < lv.r3, "R2<R3 failed");
    }
}

/// Pivot formula: P = (H+L+C)/3, R1 = 2P-L, S1 = 2P-H.
#[test]
fn pivots_classic_formula_exact() {
    let mut pp = PivotPoints::new(PivotMethod::Classic);
    pp.update(dec!(120), dec!(90), dec!(105), 0);
    let lv = pp.update(dec!(100), dec!(99), dec!(100), 1).unwrap();
    assert_eq!(lv.pivot, dec!(105));
    assert_eq!(lv.r1, dec!(120));
    assert_eq!(lv.s1, dec!(90));
    assert_eq!(lv.r2, dec!(135));
    assert_eq!(lv.s2, dec!(75));
}

/// Levels stay constant across candles within the same session and only change
/// on a new session boundary.
#[test]
fn pivots_constant_within_session_change_across() {
    let mut pp = PivotPoints::new(PivotMethod::Classic);
    pp.update(dec!(110), dec!(90), dec!(100), 0);
    let day1_a = pp.update(dec!(108), dec!(101), dec!(104), 1).unwrap();
    let day1_b = pp.update(dec!(130), dec!(95), dec!(126), 1).unwrap();
    assert_eq!(day1_a, day1_b, "levels must be constant within a session");

    // New session (day 2) recomputes from day 1's accumulated H/L/C.
    let day2 = pp.update(dec!(120), dec!(118), dec!(119), 2).unwrap();
    assert_ne!(day1_a.pivot, day2.pivot, "levels must change across sessions");
}

/// Session high/low accumulation tracks the running extreme, not just the last
/// candle — the finalized pivot uses the true session range.
#[test]
fn pivots_session_accumulates_extremes() {
    let mut pp = PivotPoints::new(PivotMethod::Classic);
    // Day 0: high spikes to 200 mid-session, low dips to 50, closes at 120.
    pp.update(dec!(150), dec!(100), dec!(140), 0);
    pp.update(dec!(200), dec!(90), dec!(160), 0);
    pp.update(dec!(160), dec!(50), dec!(120), 0);
    let lv = pp.update(dec!(120), dec!(119), dec!(120), 1).unwrap();
    // P = (200 + 50 + 120) / 3 = 123.333...
    let expected_p = (dec!(200) + dec!(50) + dec!(120)) / Decimal::from(3);
    assert_eq!(lv.pivot, expected_p);
}
