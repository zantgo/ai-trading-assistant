//! AUDIT-AIU-114 regression pins: SMC order-block signals must fire ONLY on
//! genuine TEST events (`SMC_OB_*_TEST` — price within 0.5% of the block
//! mid). The legacy branch also emitted on `SMC_OB_*_ACTIVE` — the standing
//! steady state — pushing a LevelTest + `SMC_OB_TRENDFLIP` on EVERY bar for
//! the zone's lifetime, spamming TAE/confluence with a flip that never flips.

use market_analyzer::indicators::normalized::{
    derive_signals, IndicatorSignal, NormalizedIndicatorValue, SignalDirection, SignalKind,
};
use std::collections::HashMap;

fn signals_for(label: &str) -> Vec<IndicatorSignal> {
    let mut map: HashMap<String, NormalizedIndicatorValue> = HashMap::new();
    map.insert(
        "smc_order_blocks".to_string(),
        NormalizedIndicatorValue::scalar(0.0, 0.0, label.to_string()),
    );
    derive_signals(&mut map);
    map.get("smc_order_blocks")
        .map(|v| v.signals.clone())
        .unwrap_or_default()
}

#[test]
fn active_order_block_emits_no_signals() {
    // Standing steady state (zone persists until price closes through it) —
    // must NOT spam LevelTest/TrendFlip every bar.
    for label in ["SMC_OB_BULLISH_ACTIVE", "SMC_OB_BEARISH_ACTIVE"] {
        let sigs = signals_for(label);
        assert!(
            sigs.is_empty(),
            "ACTIVE order block '{label}' must emit no signals, got {:?}",
            sigs
        );
    }
}

#[test]
fn tested_order_block_emits_leveltest_and_trendflip() {
    // Genuine test event (price within 0.5% of the block mid) → one
    // LevelTest + one TrendFlip, direction from the side.
    let sigs = signals_for("SMC_OB_BULLISH_TEST");
    assert_eq!(
        sigs.len(),
        2,
        "TEST block must emit exactly 2 signals: {:?}",
        sigs
    );
    assert_eq!(sigs[0].kind, SignalKind::LevelTest);
    assert_eq!(sigs[0].direction, SignalDirection::Bullish);
    assert_eq!(sigs[1].kind, SignalKind::TrendFlip);
    assert_eq!(sigs[1].label, "SMC_OB_TRENDFLIP");
    assert_eq!(sigs[1].direction, SignalDirection::Bullish);

    let sigs = signals_for("SMC_OB_BEARISH_TEST");
    assert_eq!(
        sigs.len(),
        2,
        "TEST block must emit exactly 2 signals: {:?}",
        sigs
    );
    assert_eq!(sigs[0].kind, SignalKind::LevelTest);
    assert_eq!(sigs[0].direction, SignalDirection::Bearish);
    assert_eq!(sigs[1].kind, SignalKind::TrendFlip);
    assert_eq!(sigs[1].direction, SignalDirection::Bearish);
}

#[test]
fn no_block_emits_no_signals() {
    assert!(signals_for("SMC_OB_NONE").is_empty());
}
