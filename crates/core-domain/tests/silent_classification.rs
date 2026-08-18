//! Tests for the Layer-1 silent-classification helpers on
//! `NormalizedIndicatorValue`. Drives the frontend's SILENT ⚡ pill
//! rendering in `IndicatorsView.svelte`.

use core_domain::indicator_dtos::{
    IndicatorSignal, NormalizedIndicatorValue, SignalDirection, SignalKind, SignalStatus,
};

fn make() -> NormalizedIndicatorValue {
    NormalizedIndicatorValue::scalar(0.0, 0.0, "")
}

#[test]
fn is_silent_when_no_signal_no_label_zero_value() {
    let v = make();
    assert!(v.is_silent(), "default-empty value with zero raw is silent");
}

#[test]
fn not_silent_when_signal_present() {
    let mut v = make();
    v = v.push_signal(IndicatorSignal::new(
        SignalKind::Crossover,
        SignalDirection::Bullish,
        SignalStatus::Active,
        "GOLDEN_CROSS",
    ));
    assert!(!v.is_silent(), "entry with a signal is not silent");
    assert!(v.has_signals());
}

#[test]
fn not_silent_when_state_label_present() {
    let v = NormalizedIndicatorValue::scalar(0.0, 0.0, "BULLISH");
    assert!(!v.is_silent(), "non-empty state_label defeats silence");
}

#[test]
fn not_silent_when_raw_value_nonzero() {
    // Even with no signal and no state_label, a non-zero raw value
    // indicates the calculator produced a meaningful reading, which the
    // dashboard should surface as LIVE rather than SILENT (the raw
    // scalar is itself a "data emission").
    let v = NormalizedIndicatorValue::scalar(0.05, 0.0, "");
    assert!(
        !v.is_silent(),
        "non-zero raw_value is itself a published reading"
    );
}

#[test]
fn indicator_lifecycle_status_carries_silent_flag() {
    use core_domain::indicator_dtos::{IndicatorLifecycleState, IndicatorLifecycleStatus};

    let live =
        IndicatorLifecycleStatus::live(200, 200, 1_700_000_000_000, 300, /* silent = */ true);
    assert!(live.silent);
    assert_eq!(live.state, IndicatorLifecycleState::Live);

    let loading = IndicatorLifecycleStatus::loading(200, 300);
    assert!(!loading.silent, "Loading state is not silent");
}

#[test]
fn has_signals_only_when_signals_vec_nonempty() {
    let mut v = make();
    assert!(!v.has_signals());
    v = v.push_signal(IndicatorSignal::new(
        SignalKind::Threshold,
        SignalDirection::Bearish,
        SignalStatus::Potential,
        "OVERSOLD",
    ));
    assert!(v.has_signals());
}
