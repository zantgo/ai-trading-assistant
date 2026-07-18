//! AC-L3-1 (03-01-04 §7.1): median warm-up accepts every tick for the first
//! `median_window_size = 20` ticks; from tick 21 onward the filter evaluates
//! against the prior 20.

use config_models::QualityConfig;
use network_adapters::median_filter::{FilterVerdict, MedianPriceFilter};

fn default_filter() -> MedianPriceFilter {
    MedianPriceFilter::new(&QualityConfig::default())
}

#[test]
fn first_20_ticks_accepted_unfiltered_even_wild_values() {
    let mut filter = default_filter();
    // Alternate wildly between magnitudes — warm-up must accept everything.
    for i in 0..20u32 {
        let price = if i % 2 == 0 { 1.0 } else { 1_000_000.0 };
        assert_eq!(
            filter.evaluate(price),
            FilterVerdict::Accepted,
            "warm-up tick {i} must be accepted"
        );
    }
    assert_eq!(filter.outliers_rejected(), 0);
    assert_eq!(filter.window_len(), 20, "window filled during warm-up");
}

#[test]
fn tick_21_is_evaluated_against_prior_20() {
    let mut filter = default_filter();
    for _ in 0..20 {
        filter.evaluate(100.0);
    }
    // Tick 21: within 5% of the median (100.0) → accepted.
    assert_eq!(filter.evaluate(104.9), FilterVerdict::Accepted);
    // Far outside 5% → rejected.
    assert_eq!(filter.evaluate(150.0), FilterVerdict::Rejected);
    assert_eq!(filter.outliers_rejected(), 1);
}

#[test]
fn current_tick_appended_after_filter_check() {
    // A rejected tick must NOT enter the window: after a rejection the median
    // is unchanged, so a subsequent in-tolerance tick is still accepted.
    let mut filter = default_filter();
    for _ in 0..20 {
        filter.evaluate(100.0);
    }
    assert_eq!(filter.evaluate(500.0), FilterVerdict::Rejected);
    assert_eq!(
        filter.evaluate(101.0),
        FilterVerdict::Accepted,
        "median must still be 100.0 — the rejected tick was not appended"
    );
}

#[test]
fn configured_window_size_is_respected() {
    let cfg = QualityConfig {
        median_window_size: 5,
        ..QualityConfig::default()
    };
    let mut filter = MedianPriceFilter::new(&cfg);
    for i in 0..5u32 {
        assert_eq!(
            filter.evaluate(100.0 + i as f64),
            FilterVerdict::Accepted,
            "warm-up tick {i}"
        );
    }
    // Tick 6 is evaluated (median ≈ 102) — a wild value is rejected.
    assert_eq!(filter.evaluate(400.0), FilterVerdict::Rejected);
}
