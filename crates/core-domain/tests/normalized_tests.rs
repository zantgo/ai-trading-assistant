use core_domain::normalized::{
    Exchange, NormalizedEvent, NormalizedTrade, SymbolMapper, TradeSide,
};
use market_analyzer::candle_generator::CandleGenerator;
use rust_decimal_macros::dec;

#[tokio::test]
async fn test_symbol_mapper_register_and_lookup() {
    let mapper = SymbolMapper::new();
    mapper
        .register(Exchange::Hyperliquid, "BTC", "BTC-USD")
        .await;

    let normalized = mapper.normalize(Exchange::Hyperliquid, "BTC").await;
    assert_eq!(normalized, Some("BTC-USD".to_string()));

    let raw = mapper.get_raw(Exchange::Hyperliquid, "BTC-USD").await;
    assert_eq!(raw, Some("BTC".to_string()));
}

#[tokio::test]
async fn test_symbol_mapper_duplicate_registration_overwrites() {
    let mapper = SymbolMapper::new();
    mapper
        .register(Exchange::Hyperliquid, "ETH", "ETH-USD")
        .await;
    // Registering the same pair again should overwrite (no error)
    mapper
        .register(Exchange::Hyperliquid, "ETH", "ETH-USD")
        .await;
    let normalized = mapper.normalize(Exchange::Hyperliquid, "ETH").await;
    assert_eq!(normalized, Some("ETH-USD".to_string()));

    // Should still only return one result for the exchange
    let symbols = mapper
        .get_normalized_for_exchange(Exchange::Hyperliquid)
        .await;
    assert_eq!(symbols.len(), 1);
}

#[tokio::test]
async fn test_symbol_mapper_missing_returns_none() {
    let mapper = SymbolMapper::new();
    let result = mapper.normalize(Exchange::Hyperliquid, "NONEXIST").await;
    assert!(result.is_none());
}

#[test]
fn test_candle_generator_first_trade_seeds_open() {
    let mut gen = CandleGenerator::new("BTC-USD", 60, Exchange::Hyperliquid);
    let trade = NormalizedTrade {
        exchange: Exchange::Hyperliquid,
        symbol: "BTC-USD".to_string(),
        price: dec!(50000.00),
        size: dec!(1.0),
        side: TradeSide::Buy,
        timestamp_ms: 60000,
        trade_id: "t1".to_string(),
    };
    let (closed, live) = gen.process_trade(&trade);
    assert!(
        closed.is_none(),
        "First trade should not produce a closed candle"
    );
    assert_eq!(live.open, dec!(50000.00));
    assert_eq!(live.high, dec!(50000.00));
    assert_eq!(live.low, dec!(50000.00));
    assert_eq!(live.close, dec!(50000.00));
    assert_eq!(live.volume, dec!(1.0));
    assert_eq!(live.trades_count, 1);
    assert_eq!(live.start_time_ms, 60000);
}

#[test]
fn test_candle_generator_multi_trade_aggregation() {
    let mut gen = CandleGenerator::new("ETH-USD", 60, Exchange::Hyperliquid);
    let t1 = NormalizedTrade {
        exchange: Exchange::Hyperliquid,
        symbol: "ETH-USD".to_string(),
        price: dec!(3000.00),
        size: dec!(10.0),
        side: TradeSide::Buy,
        timestamp_ms: 120000,
        trade_id: "t1".to_string(),
    };
    gen.process_trade(&t1);

    let t2 = NormalizedTrade {
        exchange: Exchange::Hyperliquid,
        symbol: "ETH-USD".to_string(),
        price: dec!(3010.00),
        size: dec!(5.0),
        side: TradeSide::Buy,
        timestamp_ms: 130000,
        trade_id: "t2".to_string(),
    };
    let (closed, live) = gen.process_trade(&t2);
    assert!(
        closed.is_none(),
        "Trades within same candle window should not close"
    );
    assert_eq!(live.open, dec!(3000.00));
    assert_eq!(live.high, dec!(3010.00));
    assert_eq!(live.low, dec!(3000.00));
    assert_eq!(live.close, dec!(3010.00));
    assert_eq!(live.volume, dec!(15.0));
    assert_eq!(live.trades_count, 2);
}

#[test]
fn test_candle_generator_crosses_boundary_emits_closed() {
    let mut gen = CandleGenerator::new("SOL-USD", 60, Exchange::Hyperliquid);
    let t1 = NormalizedTrade {
        exchange: Exchange::Hyperliquid,
        symbol: "SOL-USD".to_string(),
        price: dec!(100.00),
        size: dec!(1.0),
        side: TradeSide::Buy,
        timestamp_ms: 120000,
        trade_id: "t1".to_string(),
    };
    let (closed1, live1) = gen.process_trade(&t1);
    assert!(closed1.is_none());
    assert_eq!(live1.open, dec!(100.00));

    // Trade in the next candle window (120000+60000 = 180000)
    let t2 = NormalizedTrade {
        exchange: Exchange::Hyperliquid,
        symbol: "SOL-USD".to_string(),
        price: dec!(110.00),
        size: dec!(2.0),
        side: TradeSide::Sell,
        timestamp_ms: 180000,
        trade_id: "t2".to_string(),
    };
    let (closed2, live2) = gen.process_trade(&t2);
    assert!(
        closed2.is_some(),
        "Trade in next window should close previous candle"
    );
    let closed = closed2.unwrap();
    assert_eq!(closed.start_time_ms, 120000);
    assert_eq!(closed.open, dec!(100.00));
    assert_eq!(closed.high, dec!(100.00));
    assert_eq!(closed.low, dec!(100.00));
    assert_eq!(closed.close, dec!(100.00));
    assert_eq!(closed.volume, dec!(1.0));
    assert_eq!(closed.trades_count, 1);

    assert_eq!(live2.open, dec!(110.00));
    assert_eq!(live2.start_time_ms, 180000);
}

#[test]
fn test_normalized_event_trade_serialization() {
    let trade = NormalizedTrade {
        exchange: Exchange::Hyperliquid,
        symbol: "BTC-USD".to_string(),
        price: dec!(50000.00),
        size: dec!(0.5),
        side: TradeSide::Buy,
        timestamp_ms: 1000000,
        trade_id: "abc123".to_string(),
    };
    let event = NormalizedEvent::Trade(trade);

    let json = serde_json::to_string(&event).expect("Trade event serialization should succeed");
    let parsed: NormalizedEvent =
        serde_json::from_str(&json).expect("Trade event deserialization should succeed");

    match parsed {
        NormalizedEvent::Trade(t) => {
            assert_eq!(t.symbol, "BTC-USD");
            assert_eq!(t.price, dec!(50000.00));
            assert_eq!(t.size, dec!(0.5));
            assert_eq!(t.side, TradeSide::Buy);
            assert_eq!(t.timestamp_ms, 1000000);
            assert_eq!(t.trade_id, "abc123");
        }
        _ => panic!("Expected Trade event"),
    }
}

#[test]
fn test_normalized_event_status_serialization() {
    let event = NormalizedEvent::Status {
        exchange: Exchange::Hyperliquid,
        status: core_domain::normalized::ConnectionStatus::Connected,
        message: "WebSocket connected".to_string(),
    };

    let json = serde_json::to_string(&event).expect("Status event serialization should succeed");
    let parsed: NormalizedEvent =
        serde_json::from_str(&json).expect("Status event deserialization should succeed");

    match parsed {
        NormalizedEvent::Status {
            exchange,
            status,
            message,
        } => {
            assert_eq!(exchange, Exchange::Hyperliquid);
            assert_eq!(status, core_domain::normalized::ConnectionStatus::Connected);
            assert_eq!(message, "WebSocket connected");
        }
        _ => panic!("Expected Status event"),
    }
}
