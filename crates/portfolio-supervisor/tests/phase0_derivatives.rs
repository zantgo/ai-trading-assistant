//! Phase 0 tests: derivatives telemetry plumbing.
//!
//! Validates the dormant plumbing that the Phase 0 work activates:
//!  - `MarkPriceEvent` / `LiquidationEvent` deserialization
//!  - `LiquidityConfig` default values
//!  - `hl_derivatives_poller::lookup_ctx` symbol matching
//!  - `bitget_derivatives::ticker_to_mark_price` and `funding_to_event`

use rust_decimal_macros::dec;
use core_domain::normalized::{
    Exchange, FundingRateEvent, LiquidationEvent, LiquidationSide, MarkPriceEvent, NormalizedEvent,
    OpenInterestEvent,
};

#[test]
fn mark_price_event_round_trip() {
    let ev = MarkPriceEvent {
        symbol: "BTC-USDT".to_string(),
        mark_px: dec!(50100.50),
        index_px: Some(dec!(50000.00)),
        timestamp_ms: 1700000000000,
    };
    let json = serde_json::to_string(&ev).unwrap();
    let parsed: MarkPriceEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.symbol, "BTC-USDT");
    assert_eq!(parsed.mark_px, dec!(50100.50));
    assert_eq!(parsed.index_px, Some(dec!(50000.00)));
    assert_eq!(parsed.timestamp_ms, 1700000000000);
}

#[test]
fn mark_price_event_omits_optional_index() {
    let ev = MarkPriceEvent {
        symbol: "ETH-USD".to_string(),
        mark_px: dec!(3000.00),
        index_px: None,
        timestamp_ms: 0,
    };
    let json = serde_json::to_string(&ev).unwrap();
    // index_px is None — must be skipped
    assert!(
        !json.contains("index_px"),
        "index_px should be skipped: {}",
        json
    );
    let parsed: MarkPriceEvent = serde_json::from_str(&json).unwrap();
    assert!(parsed.index_px.is_none());
}

#[test]
fn funding_rate_event_round_trip() {
    let ev = FundingRateEvent {
        symbol: "BTC-USDT".to_string(),
        rate: dec!(0.00015),
    };
    let json = serde_json::to_string(&ev).unwrap();
    let parsed: FundingRateEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.rate, dec!(0.00015));
}

#[test]
fn open_interest_event_with_prev_oi() {
    let ev = OpenInterestEvent {
        symbol: "BTC-USDT".to_string(),
        oi: dec!(12345.67),
        prev_oi: Some(dec!(12300.00)),
    };
    let json = serde_json::to_string(&ev).unwrap();
    let parsed: OpenInterestEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.prev_oi, Some(dec!(12300.00)));
    let delta = parsed.oi - parsed.prev_oi.unwrap();
    assert_eq!(delta, dec!(45.67));
}

#[test]
fn open_interest_event_omits_optional_prev() {
    let ev = OpenInterestEvent {
        symbol: "BTC-USDT".to_string(),
        oi: dec!(1000.00),
        prev_oi: None,
    };
    let json = serde_json::to_string(&ev).unwrap();
    assert!(
        !json.contains("prev_oi"),
        "prev_oi should be skipped: {}",
        json
    );
}

#[test]
fn liquidation_event_long_side() {
    let ev = LiquidationEvent {
        exchange: Exchange::Hyperliquid,
        symbol: "BTC-USDT".to_string(),
        side: LiquidationSide::Long,
        price: dec!(50000.00),
        size: dec!(0.5),
        timestamp_ms: 1700000000000,
        venue_order_id: Some("0xabc123".to_string()),
    };
    let json = serde_json::to_string(&ev).unwrap();
    assert!(
        json.contains("\"side\":\"LONG\""),
        "Long side should serialize as LONG: {}",
        json
    );
    let parsed: LiquidationEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.side, LiquidationSide::Long);
    assert_eq!(parsed.exchange, Exchange::Hyperliquid);
}

#[test]
fn liquidation_event_short_side() {
    let ev = LiquidationEvent {
        exchange: Exchange::Bitget,
        symbol: "ETH-USDT".to_string(),
        side: LiquidationSide::Short,
        price: dec!(3000.00),
        size: dec!(2.0),
        timestamp_ms: 0,
        venue_order_id: None,
    };
    let json = serde_json::to_string(&ev).unwrap();
    assert!(
        json.contains("\"side\":\"SHORT\""),
        "Short side should serialize as SHORT: {}",
        json
    );
}

#[test]
fn liquidity_config_default_is_safe() {
    let cfg = config_models::LiquidityConfig::default();
    // Master switch defaults to enabled.
    assert!(cfg.enabled);
    // Polling cadence is sane (>= 1s to avoid hot loops).
    assert!(cfg.mark_price_poll_ms >= 1000);
    assert!(cfg.funding_refresh_ms >= 1000);
    // Cluster refresh: v6.5 changed the default from 300 s to 0
    // (means "synchronize with TF candle cadence"). Operators may still
    // override with any value ≥ 1, but the v6.5 default is 0 so the
    // cluster refresh runs at every candle close (matching every other
    // MME indicator/signal).
    assert_eq!(cfg.cluster_refresh_secs, 0);
    // Maintenance margin 0.5% (industry standard for perps).
    assert!((cfg.maintenance_margin_rate - 0.005).abs() < 1e-9);
    // Cascade z-score threshold is meaningful (>= 2.0).
    assert!(cfg.cascade_detected_zscore >= 2.0);
    // Funding extreme threshold is sensible.
    assert!(cfg.funding_extreme_pct > 0.0);
    // Magnet activation distance is in 0-5% range.
    assert!(cfg.magnet_activation_distance_pct > 0.0);
    assert!(cfg.magnet_activation_distance_pct < 5.0);
    // Retention defaults match the design.
    assert_eq!(cfg.event_retention_days, 90);
    assert_eq!(cfg.bucket_retention_days, 7);
}

#[test]
fn liquidity_config_default_round_trips_via_toml() {
    let cfg = config_models::LiquidityConfig::default();
    let toml_str = toml::to_string(&cfg).expect("LiquidityConfig should be TOML-serializable");
    let parsed: config_models::LiquidityConfig =
        toml::from_str(&toml_str).expect("LiquidityConfig should round-trip through TOML");
    assert_eq!(parsed.event_retention_days, cfg.event_retention_days);
    assert_eq!(parsed.cluster_refresh_secs, cfg.cluster_refresh_secs);
    assert!((parsed.maintenance_margin_rate - cfg.maintenance_margin_rate).abs() < 1e-9);
}

#[test]
fn hl_derivatives_poller_lookup_ctx_handles_case_variants() {
    use network_adapters::adapters::hl_derivatives_poller::lookup_ctx;
    use network_adapters::adapters::hyperliquid_rest::HlDerivativesCtx;
    use std::collections::HashMap;

    let mut m = HashMap::new();
    m.insert("BTC".to_string(), HlDerivativesCtx::default());
    assert!(lookup_ctx(&m, "BTC").is_some());
    assert!(lookup_ctx(&m, "btc").is_some());
    assert!(lookup_ctx(&m, "ETH").is_none());
    assert!(lookup_ctx(&m, "").is_none());
}

#[test]
fn bitget_ticker_to_mark_price_extracts_fields() {
    use network_adapters::adapters::bitget_derivatives::{
        funding_to_event, ticker_to_mark_price, BitgetFundingData, BitgetTickerData,
    };

    let d = BitgetTickerData {
        mark_price: Some("50100.5".into()),
        index_price: Some("50000.0".into()),
        open_24h: Some("49000.0".into()),
    };
    let ev = ticker_to_mark_price("BTC-USDT", &d).expect("mark present");
    match ev {
        NormalizedEvent::MarkPrice(m) => {
            assert_eq!(m.symbol, "BTC-USDT");
            assert_eq!(m.mark_px, dec!(50100.5));
            assert_eq!(m.index_px, Some(dec!(50000.0)));
        }
        _ => panic!("expected MarkPrice event"),
    }

    // No mark → None.
    let d = BitgetTickerData {
        mark_price: None,
        index_price: Some("50000.0".into()),
        open_24h: None,
    };
    assert!(ticker_to_mark_price("BTC-USDT", &d).is_none());

    // Funding parser.
    let fr = BitgetFundingData {
        fundingRate: Some("0.0001".into()),
        nextUpdate: Some(1700000000000),
    };
    let ev = funding_to_event("BTC-USDT", &fr).expect("rate present");
    match ev {
        NormalizedEvent::FundingRate(f) => assert_eq!(f.rate, dec!(0.0001)),
        _ => panic!("expected FundingRate event"),
    }
}
