//! AC-L3-4 (03-01-04 §7.1): median = 0 (venue reset) bypasses the filter for
//! that tick; bypassed ticks are accepted and counted under
//! `outliers_bypassed`, not `outliers_rejected`.

use config_models::QualityConfig;
use network_adapters::median_filter::{FilterVerdict, MedianPriceFilter};

fn zero_median_filter(bypass: bool) -> MedianPriceFilter {
    let cfg = QualityConfig {
        bypass_on_zero_median: bypass,
        ..QualityConfig::default()
    };
    let mut filter = MedianPriceFilter::new(&cfg);
    // Fill the warm-up window with zeros so the rolling median is exactly 0.
    for _ in 0..20 {
        filter.evaluate(0.0);
    }
    filter
}

#[test]
fn zero_median_bypasses_when_enabled() {
    let mut filter = zero_median_filter(true);
    assert_eq!(filter.evaluate(123.45), FilterVerdict::Bypassed);
    assert_eq!(filter.outliers_bypassed(), 1);
    assert_eq!(filter.outliers_rejected(), 0);
}

#[test]
fn bypassed_tick_enters_window_and_recovers_median() {
    let mut filter = zero_median_filter(true);
    // Repeated bypasses gradually replace the zero window with real prices.
    for _ in 0..20 {
        let v = filter.evaluate(100.0);
        assert_ne!(v, FilterVerdict::Rejected, "bypass path never rejects");
    }
    // Median is now 100 → normal evaluation resumes.
    assert_eq!(filter.evaluate(104.0), FilterVerdict::Accepted);
    assert_eq!(filter.evaluate(200.0), FilterVerdict::Rejected);
}

#[test]
fn zero_median_without_bypass_rejects_nonzero_tick() {
    let mut filter = zero_median_filter(false);
    assert_eq!(filter.evaluate(123.45), FilterVerdict::Rejected);
    assert_eq!(filter.evaluate(0.0), FilterVerdict::Accepted);
    assert_eq!(filter.outliers_bypassed(), 0);
    assert_eq!(filter.outliers_rejected(), 1);
}
