//! Phase 1 tests: real liquidation event ingestion.
//!
//! Validates the per-candle flow aggregation logic and the exchange-specific
//! payload parsers. Real WS connection is not exercised here — that is
//! covered by the integration tests in `tests/engine/`.

use core_domain::liquidity::{CascadeState, LiquidityEventAccumulator, LiquidityFlow};
use core_domain::normalized::{Exchange, LiquidationEvent, LiquidationSide};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

fn ev(side: LiquidationSide, price: f64, size: f64, ts_ms: u64) -> LiquidationEvent {
    LiquidationEvent {
        exchange: Exchange::Hyperliquid,
        symbol: "BTC-USDT".to_string(),
        side,
        price: Decimal::from_f64_retain(price).unwrap_or(Decimal::ZERO),
        size: Decimal::from_f64_retain(size).unwrap_or(Decimal::ZERO),
        timestamp_ms: ts_ms,
        venue_order_id: None,
    }
}

#[test]
fn flow_default_is_zero() {
    let flow = LiquidityFlow::default();
    assert_eq!(flow.event_count, 0);
    assert_eq!(flow.long_liquidations_usd, 0.0);
    assert_eq!(flow.short_liquidations_usd, 0.0);
    assert_eq!(flow.cascade_state, CascadeState::None);
    assert_eq!(flow.cascade_intensity, 0.0);
    assert!(flow.largest_event_price.is_none());
    assert!(flow.largest_event_side.is_none());
}

#[test]
fn flow_net_sign_convention() {
    // Long liquidations = longs dumped = bearish pressure.
    let mut acc = LiquidityEventAccumulator::new("BTC-USDT");
    acc.record_event(ev(LiquidationSide::Long, 50_000.0, 1.0, 1));
    let flow = acc.flush_to_flow();
    assert!(
        flow.net_liquidation_usd > 0.0,
        "net should be positive for long liqs (sign convention)"
    );

    let mut acc = LiquidityEventAccumulator::new("BTC-USDT");
    acc.record_event(ev(LiquidationSide::Short, 50_000.0, 1.0, 1));
    let flow = acc.flush_to_flow();
    assert!(
        flow.net_liquidation_usd < 0.0,
        "net should be negative for short liqs"
    );
}

#[test]
fn flow_largest_event_tracks_max() {
    let mut acc = LiquidityEventAccumulator::new("BTC-USDT");
    acc.record_event(ev(LiquidationSide::Long, 50_000.0, 0.5, 1));
    acc.record_event(ev(LiquidationSide::Long, 51_000.0, 2.0, 2));
    acc.record_event(ev(LiquidationSide::Short, 49_000.0, 0.1, 3));
    let flow = acc.flush_to_flow();
    // Largest: 51000 × 2.0 = 102_000 USD.
    assert!((flow.largest_event_usd - 102_000.0).abs() < 1.0);
    assert_eq!(flow.largest_event_price, Some(51_000.0));
    assert_eq!(flow.largest_event_side, Some(LiquidationSide::Long));
}

#[test]
fn flow_event_count_matches_recorded() {
    let mut acc = LiquidityEventAccumulator::new("BTC-USDT");
    for i in 0..5 {
        acc.record_event(ev(LiquidationSide::Long, 50_000.0, 0.1, i));
    }
    let flow = acc.flush_to_flow();
    assert_eq!(flow.event_count, 5);
    assert!((flow.long_liquidations_usd - 25_000.0).abs() < 1.0); // 5 * 5000
}

#[test]
fn flow_bounded_history_drops_oldest() {
    let mut acc = LiquidityEventAccumulator::with_config("BTC-USDT", 3, 2.5, 5, 3);
    for i in 0..10 {
        acc.record_event(ev(LiquidationSide::Long, 50_000.0, 0.1, i));
    }
    assert_eq!(acc.buffered_event_count(), 3, "history must cap at 3");
}

#[test]
fn flow_flush_resets_per_bar() {
    let mut acc = LiquidityEventAccumulator::new("BTC-USDT");
    acc.record_event(ev(LiquidationSide::Long, 50_000.0, 1.0, 1));
    let first = acc.flush_to_flow();
    assert_eq!(first.event_count, 1);
    // Second flush with no events in between → empty.
    let second = acc.flush_to_flow();
    assert_eq!(second.event_count, 0);
    assert_eq!(second.long_liquidations_usd, 0.0);
}

#[test]
fn flow_cascade_state_starts_none() {
    let mut acc = LiquidityEventAccumulator::new("BTC-USDT");
    // No history, no events → None.
    let flow = acc.flush_to_flow();
    assert_eq!(flow.cascade_state, CascadeState::None);
}

#[test]
fn flow_cascade_state_detected_after_large_event() {
    let mut acc = LiquidityEventAccumulator::with_config("BTC-USDT", 100, 1.5, 5, 3);
    // Warm baseline with 3 small bars.
    for i in 0..3 {
        acc.record_event(ev(LiquidationSide::Long, 50_000.0, 0.01, i * 1000));
        let _ = acc.flush_to_flow();
    }
    // Now a large event.
    acc.record_event(ev(LiquidationSide::Long, 50_000.0, 10.0, 9999));
    let flow = acc.flush_to_flow();
    assert!(
        matches!(
            flow.cascade_state,
            CascadeState::Detected | CascadeState::Sustained
        ),
        "large event should escalate cascade state, got {:?}",
        flow.cascade_state
    );
}

#[test]
fn flow_recent_events_returns_newest_first() {
    let mut acc = LiquidityEventAccumulator::new("BTC-USDT");
    acc.record_event(ev(LiquidationSide::Long, 50_000.0, 0.1, 1));
    acc.record_event(ev(LiquidationSide::Long, 50_500.0, 0.2, 2));
    acc.record_event(ev(LiquidationSide::Long, 51_000.0, 0.3, 3));
    let recent = acc.recent_events(2);
    assert_eq!(recent.len(), 2);
    assert_eq!(recent[0].timestamp_ms, 3);
    assert_eq!(recent[1].timestamp_ms, 2);
}

#[test]
fn flow_intensity_capped_at_100() {
    // Force a very high-intensity bar and check the cap.
    let mut acc = LiquidityEventAccumulator::with_config("BTC-USDT", 100, 1.0, 5, 3);
    for i in 0..3 {
        acc.record_event(ev(LiquidationSide::Long, 50_000.0, 0.0001, i * 1000));
        let _ = acc.flush_to_flow();
    }
    // Huge event.
    acc.record_event(ev(LiquidationSide::Long, 50_000.0, 100.0, 9999));
    let flow = acc.flush_to_flow();
    assert!(
        flow.cascade_intensity <= 100.0,
        "intensity must be capped at 100, got {}",
        flow.cascade_intensity
    );
    assert!(flow.cascade_intensity >= 0.0, "intensity must be >= 0");
}

#[test]
fn flow_decimal_serialization() {
    // The LiquidityFlow must round-trip cleanly through JSON because it
    // rides on the WebSocket frame.
    let mut acc = LiquidityEventAccumulator::new("BTC-USDT");
    acc.record_event(ev(LiquidationSide::Long, 50_000.0, 1.0, 1));
    let flow = acc.flush_to_flow();
    let json = serde_json::to_string(&flow).expect("flow serializes");
    let parsed: LiquidityFlow = serde_json::from_str(&json).expect("flow deserializes");
    assert!((parsed.long_liquidations_usd - flow.long_liquidations_usd).abs() < 1e-9);
    assert_eq!(parsed.cascade_state, flow.cascade_state);
    assert_eq!(parsed.event_count, flow.event_count);
}

#[test]
fn flow_handles_zero_size_event() {
    let mut acc = LiquidityEventAccumulator::new("BTC-USDT");
    acc.record_event(ev(LiquidationSide::Long, 50_000.0, 0.0, 1));
    let flow = acc.flush_to_flow();
    assert_eq!(flow.event_count, 1);
    assert_eq!(flow.long_liquidations_usd, 0.0);
}

#[test]
fn flow_multiple_bars_track_separately() {
    let mut acc = LiquidityEventAccumulator::new("BTC-USDT");
    acc.record_event(ev(LiquidationSide::Long, 50_000.0, 1.0, 1));
    let bar1 = acc.flush_to_flow();
    assert_eq!(bar1.event_count, 1);

    acc.record_event(ev(LiquidationSide::Long, 51_000.0, 2.0, 2));
    acc.record_event(ev(LiquidationSide::Short, 52_000.0, 0.5, 3));
    let bar2 = acc.flush_to_flow();
    assert_eq!(bar2.event_count, 2);
    assert!((bar2.long_liquidations_usd - 102_000.0).abs() < 1.0);
    assert!((bar2.short_liquidations_usd - 26_000.0).abs() < 1.0);
}

#[test]
fn flow_serialization_uses_screaming_snake_case_cascade() {
    // Frontend expects CASCADE_*_STATE (uppercase) in the cascade_state field.
    let flow = LiquidityFlow {
        long_liquidations_usd: 1.0,
        short_liquidations_usd: 2.0,
        net_liquidation_usd: -1.0,
        event_count: 1,
        largest_event_usd: 2.0,
        largest_event_price: Some(50_000.0),
        largest_event_side: Some(LiquidationSide::Long),
        cascade_state: CascadeState::Sustained,
        cascade_intensity: 75.0,
    };
    let json = serde_json::to_string(&flow).unwrap();
    assert!(
        json.contains("\"cascade_state\":\"SUSTAINED\""),
        "cascade_state must be SCREAMING_SNAKE_CASE: {}",
        json
    );
    assert!(
        json.contains("\"largest_event_side\":\"LONG\""),
        "LiquidationSide must be SCREAMING_SNAKE_CASE: {}",
        json
    );
}

#[test]
fn flow_serialization_long_side_marker() {
    // LiquidationSide::Long serializes to "LONG" not "Long" — a contract
    // for the frontend.
    let flow = LiquidityFlow {
        largest_event_side: Some(LiquidationSide::Long),
        cascade_state: CascadeState::Detected,
        ..Default::default()
    };
    let json = serde_json::to_string(&flow).unwrap();
    assert!(json.contains("\"LONG\""));
    assert!(json.contains("\"DETECTED\""));
    // Avoid the PascalCase variants we explicitly don't want.
    assert!(!json.contains("\"Long\""));
    assert!(!json.contains("\"Detected\""));
    let _ = dec!(1.0);
}
