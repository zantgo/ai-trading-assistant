//! AC-L3-2 (03-01-04 §7.1): a tick whose `|p − median| / median > 0.05` is
//! dropped and counted in `outliers_rejected`.

use config_models::QualityConfig;
use network_adapters::median_filter::{FilterVerdict, MedianPriceFilter};

fn warmed_filter_at(price: f64) -> MedianPriceFilter {
    let mut filter = MedianPriceFilter::new(&QualityConfig::default());
    for _ in 0..20 {
        filter.evaluate(price);
    }
    filter
}

#[test]
fn tick_beyond_tolerance_is_rejected_and_counted() {
    let mut filter = warmed_filter_at(100.0);
    // 5% of 100 = 5. 105.0 is exactly at the boundary (<= tolerance → accept).
    assert_eq!(filter.evaluate(105.0), FilterVerdict::Accepted);
    // 106.0 → |106−~100|/~100 > 0.05 → reject.
    assert_eq!(filter.evaluate(106.0), FilterVerdict::Rejected);
    assert_eq!(filter.outliers_rejected(), 1);
}

#[test]
fn downside_spike_rejected_symmetrically() {
    let mut filter = warmed_filter_at(200.0);
    assert_eq!(filter.evaluate(189.0), FilterVerdict::Rejected); // −5.5%
    assert_eq!(filter.evaluate(191.0), FilterVerdict::Accepted); // −4.5%
    assert_eq!(filter.outliers_rejected(), 1);
}

#[test]
fn fat_finger_print_suppressed_but_genuine_move_passes() {
    // A single wild print is rejected; a genuine fast move (persisting across
    // ticks) shifts the median and passes. With a 20-tick window the median
    // lags ~10 ticks, so steps of +0.4%/tick keep the worst-case deviation
    // (1.004^10 − 1 ≈ 4.1%) inside the 5% tolerance while the price grinds up.
    let mut filter = warmed_filter_at(100.0);
    assert_eq!(filter.evaluate(150.0), FilterVerdict::Rejected);

    let mut price = 100.0;
    for _ in 0..110 {
        price *= 1.004;
        assert_eq!(
            filter.evaluate(price),
            FilterVerdict::Accepted,
            "genuine trending move must not be filtered (price {price})"
        );
    }
    assert!(price > 150.0, "the move eventually exceeds the old spike");
    assert_eq!(filter.outliers_rejected(), 1);
}

#[test]
fn rejection_counter_accumulates() {
    let mut filter = warmed_filter_at(100.0);
    for _ in 0..7 {
        filter.evaluate(1_000.0);
    }
    assert_eq!(filter.outliers_rejected(), 7);
}
