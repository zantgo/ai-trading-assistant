use rust_decimal_macros::dec;
use shared::models::MarketSnapshot;
use shared::normalized::Exchange;
use shared::TriggerType;

#[test]
fn test_market_snapshot_json_roundtrip() {
    let snap = MarketSnapshot {
        exchange: Some(Exchange::Hyperliquid),
        timeframe_secs: 60,
        timestamp: 1718000000,
        symbol: "BTC".to_string(),
        is_completed: Some(true),
        mid_price: dec!(50000.00),
        bid_price: dec!(49999.50),
        ask_price: dec!(50000.50),
        bid_size: Some(dec!(1.5)),
        ask_size: Some(dec!(2.0)),
        funding_rate: Some(dec!(0.0001)),
        open: Some(dec!(49800.00)),
        high: Some(dec!(50200.00)),
        low: Some(dec!(49750.00)),
        close: Some(dec!(50000.00)),
        volume: Some(dec!(150.0)),
        average_volume: Some(dec!(120.0)),
        rvol: Some(dec!(1.25)),
        bb_upper: Some(dec!(51000.00)),
        bb_middle: Some(dec!(50000.00)),
        bb_lower: Some(dec!(49000.00)),
        atr_14: Some(dec!(250.0)),
        atr_slope: Some(dec!(5.0)),
        atr_volatility_regime: Some("Stable".to_string()),
        atr_stop_loss_level: Some(dec!(49750.00)),
        atr_take_profit_level: Some(dec!(50500.00)),
        vwap: Some(dec!(50010.00)),
        vwap_bias: Some("Equilibrium".to_string()),
        adx_14: Some(dec!(25.0)),
        adx_plus: Some(dec!(28.0)),
        adx_minus: Some(dec!(18.0)),
        ema_fast: Some(dec!(50010.00)),
        ema_medium: Some(dec!(49990.00)),
        ema_slow: Some(dec!(49850.00)),
        ema_long: Some(dec!(49500.00)),
        ema_stack_state: Some("Bullish".to_string()),
        rsi_14: Some(dec!(65.0)),
        macd_line: Some(dec!(20.0)),
        macd_signal: Some(dec!(15.0)),
        macd_hist: Some(dec!(5.0)),
        squeeze_on: Some(false),
        squeeze_momentum: Some(dec!(0.15)),
        squeeze_duration: Some(0),
        squeeze_release_trigger: Some(true),
        squeeze_momentum_direction: Some("BullishAcceleration".to_string()),
        bbwp: Some(dec!(45.0)),
        support_levels: Some("[\"49500.00\",\"49000.00\"]".to_string()),
        resistance_levels: Some("[\"50500.00\",\"51000.00\"]".to_string()),
        sr_flip_events: None,
        chart_pattern: Some("BullishTriangle".to_string()),
        chart_pattern_confidence: Some(dec!(35.0)),
        fib_golden_pocket_low: Some(dec!(49800.00)),
        fib_golden_pocket_high: Some(dec!(49900.00)),
        fib_extension_1618: Some(dec!(52000.00)),
        fib_extension_2618: Some(dec!(54000.00)),
        swing_high: Some(dec!(51000.00)),
        swing_low: Some(dec!(48000.00)),
        rsi_divergence_status: Some("Potential".to_string()),
        rsi_divergence_coords: Some("[[49500,55,5],[49000,60,10]]".to_string()),
        macd_divergence_status: Some("None".to_string()),
        macd_divergence_coords: None,
        macd_histogram_peak: Some(dec!(8.0)),
        macd_trend_state: Some("Accelerating".to_string()),
        macd_crossover_detected: Some(false),
        macd_crossover_direction: None,
        adx_slope: Some(dec!(1.5)),
        adx_peak: Some(dec!(28.0)),
        adx_regime: Some("Emerging".to_string()),
        adx_di_crossover_detected: Some(true),
        adx_di_crossover_direction: Some("Bullish".to_string()),
    };

    let json = serde_json::to_string(&snap).expect("serialization should succeed");
    let parsed: MarketSnapshot =
        serde_json::from_str(&json).expect("deserialization should succeed");

    assert_eq!(parsed.symbol, "BTC");
    assert_eq!(parsed.mid_price, dec!(50000.00));
    assert_eq!(parsed.exchange, Some(Exchange::Hyperliquid));
    assert_eq!(parsed.rsi_14, Some(dec!(65.0)));
    assert_eq!(parsed.macd_line, Some(dec!(20.0)));
    assert_eq!(parsed.macd_signal, Some(dec!(15.0)));
    assert_eq!(parsed.macd_hist, Some(dec!(5.0)));
    assert_eq!(parsed.bbwp, Some(dec!(45.0)));
    assert_eq!(parsed.squeeze_on, Some(false));
    assert_eq!(parsed.squeeze_momentum, Some(dec!(0.15)));
    assert_eq!(parsed.fib_golden_pocket_low, Some(dec!(49800.00)));
    assert_eq!(parsed.fib_golden_pocket_high, Some(dec!(49900.00)));
    assert_eq!(parsed.fib_extension_1618, Some(dec!(52000.00)));
    assert_eq!(parsed.fib_extension_2618, Some(dec!(54000.00)));
    assert_eq!(parsed.atr_volatility_regime, Some("Stable".to_string()));
    assert_eq!(parsed.chart_pattern, Some("BullishTriangle".to_string()));
    assert_eq!(parsed.chart_pattern_confidence, Some(dec!(35.0)));
}

#[test]
fn test_market_snapshot_option_fields_null() {
    let snap = MarketSnapshot {
        exchange: None,
        timeframe_secs: 0,
        timestamp: 0,
        symbol: "EMPTY".to_string(),
        is_completed: None,
        mid_price: dec!(0.0),
        bid_price: dec!(0.0),
        ask_price: dec!(0.0),
        bid_size: None,
        ask_size: None,
        funding_rate: None,
        open: None,
        high: None,
        low: None,
        close: None,
        volume: None,
        average_volume: None,
        rvol: None,
        bb_upper: None,
        bb_middle: None,
        bb_lower: None,
        atr_14: None,
        atr_slope: None,
        atr_volatility_regime: None,
        atr_stop_loss_level: None,
        atr_take_profit_level: None,
        vwap: None,
        vwap_bias: None,
        adx_14: None,
        adx_plus: None,
        adx_minus: None,
        ema_fast: None,
        ema_medium: None,
        ema_slow: None,
        ema_long: None,
        ema_stack_state: None,
        rsi_14: None,
        macd_line: None,
        macd_signal: None,
        macd_hist: None,
        squeeze_on: None,
        squeeze_momentum: None,
        squeeze_duration: None,
        squeeze_release_trigger: None,
        squeeze_momentum_direction: None,
        bbwp: None,
        support_levels: None,
        resistance_levels: None,
        sr_flip_events: None,
        chart_pattern: None,
        chart_pattern_confidence: None,
        fib_golden_pocket_low: None,
        fib_golden_pocket_high: None,
        fib_extension_1618: None,
        fib_extension_2618: None,
        swing_high: None,
        swing_low: None,
        rsi_divergence_status: None,
        rsi_divergence_coords: None,
        macd_divergence_status: None,
        macd_divergence_coords: None,
        macd_histogram_peak: None,
        macd_trend_state: None,
        macd_crossover_detected: None,
        macd_crossover_direction: None,
        adx_slope: None,
        adx_peak: None,
        adx_regime: None,
        adx_di_crossover_detected: None,
        adx_di_crossover_direction: None,
    };

    let json = serde_json::to_string(&snap).expect("serialization of all-None opts should succeed");
    let parsed: MarketSnapshot =
        serde_json::from_str(&json).expect("deserialization of all-None opts should succeed");

    assert_eq!(parsed.symbol, "EMPTY");
    assert_eq!(parsed.mid_price, dec!(0.0));
    assert!(parsed.exchange.is_none());
    assert!(parsed.rsi_14.is_none());
    assert!(parsed.squeeze_on.is_none());
    assert!(parsed.fib_golden_pocket_low.is_none());
}

#[test]
fn test_trigger_type_manual_serde() {
    let manual = TriggerType::Manual;
    let json = serde_json::to_string(&manual).unwrap();
    assert_eq!(json, "\"Manual\"");
    let parsed: TriggerType = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, TriggerType::Manual);
}

#[test]
fn test_trigger_type_automated_serde() {
    let auto = TriggerType::Automated;
    let json = serde_json::to_string(&auto).unwrap();
    assert_eq!(json, "\"Automated\"");
    let parsed: TriggerType = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, TriggerType::Automated);
}
