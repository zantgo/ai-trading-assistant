//! Regression tests for the Bitget `fill`-channel liquidation parser
//! (Phase 1) and the V2 ticker-based OI / funding / mark-price
//! extraction.
//!
//! Two failure modes are pinned here:
//!
//! 1. **OI unit conversion** — Bitget V2's `ticker` channel emits
//!    `holdingAmount` as a base-asset quantity (contracts on USDT-M
//!    perps = base asset). The cluster estimator downstream expects
//!    USD notional. Without conversion, `total_oi_usd ≈ 39_925 BTC` is
//!    misinterpreted as `≈ $39,925`, poisoning cluster confidence to
//!    ~4% and pushing every estimated bin below the `$50,000` noise
//!    threshold. The fix: `ticker_to_derivatives_events` multiplies by
//!    the parsed mark price (with a cached override for frames that
//!    lack `markPrice`).
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
use network_adapters::adapters::bitget_derivatives::{
    ticker_to_derivatives_events, BitgetTickerData,
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

#[test]
fn bitget_v2_ticker_payload_extracts_holding_amount_as_oi() {
    // Realistic V2 ticker payload. The single push carries markPrice,
    // holdingAmount (base-asset units), and fundingRate. After
    // `ticker_to_derivatives_events` we should see all three events in
    // order with USD-converted OI.
    let payload = serde_json::json!({
        "instId": "BTCUSDT",
        "markPrice": "65000",
        "indexPrice": "64995",
        "holdingAmount": "1234.5",
        "fundingRate": "0.00012",
        "nextFundingTime": "1700000000000",
        "open24h": "64500"
    });
    let parsed: BitgetTickerData =
        serde_json::from_value(payload).expect("ticker payload should parse");
    let evs = ticker_to_derivatives_events("BTC-USDT", &parsed, None);
    assert_eq!(evs.len(), 3, "expected MarkPrice + OpenInterest + FundingRate");
    // OI: 1234.5 contracts * $65_000 = $80_242_500 USD notional.
    let NormalizedEvent::OpenInterest(oi) = &evs[1] else {
        panic!("expected OpenInterest at index 1");
    };
    let usd: f64 = oi.oi.to_string().parse().unwrap();
    assert!(
        (usd - 80_242_500.0).abs() < 1.0,
        "OI should be ~$80.24M USD notional, got ${}",
        usd
    );
    assert_eq!(oi.symbol, "BTC-USDT");
    assert!(oi.prev_oi.is_none());
}

#[test]
fn bitget_v2_ticker_payload_extracts_funding_rate() {
    let payload = serde_json::json!({
        "markPrice": "65000",
        "fundingRate": "0.0001"
    });
    let parsed: BitgetTickerData =
        serde_json::from_value(payload).expect("ticker payload should parse");
    let evs = ticker_to_derivatives_events("BTC-USDT", &parsed, None);
    assert_eq!(evs.len(), 2);
    let NormalizedEvent::FundingRate(fr) = &evs[1] else {
        panic!("expected FundingRate at index 1");
    };
    assert_eq!(fr.symbol, "BTC-USDT");
    assert_eq!(fr.rate.to_string(), "0.0001");
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
/// event to avoid poisoning the USD series with a base-asset value.
///
/// As of v6.6 the OI channel was removed in V2 and the data now rides on
/// `ticker`. The first-frame race is handled inside
/// `ticker_to_derivatives_events` via the `mark_px_override` argument:
/// the cached mark from a prior ticker frame rescues an OI sample that
/// arrives without its own `markPrice`. This test pins the V2 wire
/// format by reading the source file and asserting the helper is
/// wired into the ticker handler.
#[test]
fn bitget_v2_oi_extraction_uses_ticker_to_derivatives_events() {
    use std::fs;
    let src = fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/adapters/bitget.rs"),
    )
    .expect("bitget.rs must be readable");
    // The ticker arm must call the V2 helper that extracts OI/funding
    // from `holdingAmount` / `fundingRate` fields on the ticker push.
    assert!(
        src.contains("ticker_to_derivatives_events"),
        "bitget.rs must invoke `ticker_to_derivatives_events` from the ticker arm"
    );
    // The subscription must NOT include the dead `open-interest` channel.
    assert!(
        !src.contains("\"channel\": \"open-interest\""),
        "bitget.rs must NOT subscribe to the dead V2 `open-interest` channel"
    );
    // The subscription must NOT include the dead `funding-rate` channel.
    assert!(
        !src.contains("\"channel\": \"funding-rate\""),
        "bitget.rs must NOT subscribe to the dead V2 `funding-rate` channel"
    );
}
