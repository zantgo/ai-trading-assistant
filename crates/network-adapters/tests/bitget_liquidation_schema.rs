//! Regression tests for the Bitget `fill`-channel liquidation parser
//! (Phase 1) and the OI × mark-price unit conversion.
//!
//! Two failure modes are pinned here:
//!
//! 1. **OI unit conversion** — Bitget's `open-interest` channel emits a
//!    base-asset-quantity field (contracts on USDT-M perps = base asset).
//!    The cluster estimator downstream expects USD notional. Without
//!    conversion, `total_oi_usd ≈ 39_925 BTC` is misinterpreted as
//!    `≈ $39,925`, poisoning cluster confidence to ~4% and pushing every
//!    estimated bin below the `$50,000` noise threshold.
//!
//! 2. **Liquidation fill parsing** — the `fill` channel's
//!    `execType == "L"` rows must be emitted as
//!    `NormalizedEvent::Liquidation` with the correct side semantics
//!    (`side == "buy"` → `LiquidationSide::Short` (short squeeze);
//!    `side == "sell"` → `LiquidationSide::Long` (long liquidation)).

use core_domain::normalized::{LiquidationSide, NormalizedEvent};
use network_adapters::adapters::bitget::{
    emit_bitget_fill_liquidations_for_test, open_interest_event_for_test,
};

#[test]
fn bitget_oi_event_is_usd_notional_when_mark_known() {
    // 1000 contracts (base) at mark 65_000 → $65M USD notional.
    let ev = open_interest_event_for_test(1000.0, 65_000.0, "BTC-USDT")
        .expect("OI event should be emitted when mark is known");
    let NormalizedEvent::OpenInterest(oi) = ev else {
        panic!("expected OpenInterest event");
    };
    let usd: f64 = oi.oi.to_string().parse().unwrap();
    assert!(
        (usd - 65_000_000.0).abs() < 1.0,
        "OI should be $65M USD notional, got ${}",
        usd
    );
    assert_eq!(oi.symbol, "BTC-USDT");
    assert!(oi.prev_oi.is_none());
}

#[test]
fn bitget_oi_event_skips_when_mark_missing() {
    // Without a mark price we cannot convert — emit `None` so the
    // dispatcher skips, rather than poisoning the downstream USD series
    // with a base-asset value.
    let ev = open_interest_event_for_test(1000.0, 0.0, "BTC-USDT");
    assert!(
        ev.is_none(),
        "OI conversion must skip when mark price is missing"
    );
}

#[tokio::test]
async fn bitget_liquidation_long_fill_emits_liquidation_event() {
    // execType="L", side="sell" → a long was closed.
    let payload = serde_json::json!([
        {
            "tradeId": "t-123",
            "price": "65000",
            "size": "2",
            "side": "sell",
            "ts": "1700000000000",
            "execType": "L"
        }
    ]);
    let (tx, mut rx) = tokio::sync::mpsc::channel::<NormalizedEvent>(64);
    emit_bitget_fill_liquidations_for_test("BTC-USDT", &payload, &tx).await;
    let received = rx.try_recv().expect("a liquidation event should be queued");
    let NormalizedEvent::Liquidation(liq) = received else {
        panic!("expected Liquidation event");
    };
    assert_eq!(liq.symbol, "BTC-USDT");
    assert_eq!(liq.side, LiquidationSide::Long);
    assert_eq!(liq.price.to_string(), "65000");
    assert_eq!(liq.size.to_string(), "2");
    assert_eq!(liq.timestamp_ms, 1_700_000_000_000);
    assert_eq!(liq.venue_order_id.as_deref(), Some("t-123"));
}

#[tokio::test]
async fn bitget_liquidation_short_fill_emits_short_side() {
    // execType="L", side="buy" → a short was closed (short squeeze).
    let payload = serde_json::json!([
        {
            "tradeId": "t-124",
            "price": "65010",
            "size": "1.5",
            "side": "buy",
            "ts": "1700000001000",
            "execType": "L"
        }
    ]);
    let (tx, mut rx) = tokio::sync::mpsc::channel::<NormalizedEvent>(64);
    emit_bitget_fill_liquidations_for_test("BTC-USDT", &payload, &tx).await;
    let received = rx.try_recv().expect("a liquidation event should be queued");
    let NormalizedEvent::Liquidation(liq) = received else {
        panic!("expected Liquidation event");
    };
    assert_eq!(liq.side, LiquidationSide::Short);
}

#[tokio::test]
async fn bitget_non_liquidation_fill_is_dropped() {
    // execType != "L" → must be ignored.
    let payload = serde_json::json!([
        {
            "tradeId": "t-125",
            "price": "65000",
            "size": "0.5",
            "side": "buy",
            "ts": "1700000002000",
            "execType": "T"
        }
    ]);
    let (tx, mut rx) = tokio::sync::mpsc::channel::<NormalizedEvent>(64);
    emit_bitget_fill_liquidations_for_test("BTC-USDT", &payload, &tx).await;
    let maybe = rx.try_recv();
    assert!(maybe.is_err(), "non-liquidation fills must not emit events");
}

#[tokio::test]
async fn bitget_mixed_fill_payload_emits_only_liquidations() {
    let payload = serde_json::json!([
        {
            "tradeId": "t-200",
            "price": "65000",
            "size": "1",
            "side": "sell",
            "ts": "1700000003000",
            "execType": "T"
        },
        {
            "tradeId": "t-201",
            "price": "64990",
            "size": "3",
            "side": "sell",
            "ts": "1700000004000",
            "execType": "L"
        },
        {
            "tradeId": "t-202",
            "price": "65010",
            "size": "0.5",
            "side": "buy",
            "ts": "1700000005000",
            "execType": "L"
        }
    ]);
    let (tx, mut rx) = tokio::sync::mpsc::channel::<NormalizedEvent>(64);
    emit_bitget_fill_liquidations_for_test("BTC-USDT", &payload, &tx).await;
    let a = rx.try_recv().expect("event 1");
    let b = rx.try_recv().expect("event 2");
    assert!(
        rx.try_recv().is_err(),
        "no further events should be emitted"
    );
    let NormalizedEvent::Liquidation(liq_a) = a else {
        panic!("expected Liquidation event 1");
    };
    let NormalizedEvent::Liquidation(liq_b) = b else {
        panic!("expected Liquidation event 2");
    };
    assert_eq!(liq_a.side, LiquidationSide::Long);
    assert_eq!(liq_a.size.to_string(), "3");
    assert_eq!(liq_b.side, LiquidationSide::Short);
    assert_eq!(liq_b.size.to_string(), "0.5");
}

/// Regression: Phase 1.3 of the HL/Bitget parity sweep — when the first
/// `open-interest` frame arrives before the first `ticker` mark price
/// (Bitget's WS serves OI and mark on separate channels; the order is
/// not guaranteed on cold start), the adapter silently drops the OI
/// event to avoid poisoning the USD series with a base-asset value. The
/// parity fix: emit a one-shot `NormalizedEvent::Status` so the operator
/// sees the gap in the Exchange Status panel.
///
/// The actual WS-loop behavior is hard to drive in a unit test (requires
/// a mock WS server + cancellation token). This test pins the contract
/// by reading the source file and asserting the Status event emit path
/// is wired into the OI drop branch.
#[test]
fn first_frame_oi_drop_emits_status_event() {
    use std::fs;
    let src = fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/adapters/bitget.rs"),
    )
    .expect("bitget.rs must be readable");
    // The fix is gated by a local `oi_drop_warned: bool` flag inside
    // `run_for_symbol` — a one-shot to avoid flooding the panel.
    assert!(
        src.contains("oi_drop_warned"),
        "bitget.rs must declare the one-shot flag `oi_drop_warned`"
    );
    // The Status event itself is emitted inside the `continue` branch
    // that handles the missing-mark-price case.
    assert!(
        src.contains("OI conversion deferred — waiting for first mark price"),
        "bitget.rs must emit a Status event with the `OI conversion deferred` \
         message when the first OI frame arrives without a mark price"
    );
    assert!(
        src.contains("NormalizedEvent::Status"),
        "bitget.rs must emit a NormalizedEvent::Status variant"
    );
}
