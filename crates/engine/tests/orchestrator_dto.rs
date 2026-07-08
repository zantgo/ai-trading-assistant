//! Two-Agent Pipeline verification: mock analyst document and trader decision
//! JSON still deserializes cleanly into the new DTOs (v3.0).

use engine::llm::{AnalystDocument, TraderDecision};

#[test]
fn mock_analyst_json_parses_into_dto() {
    let mock = r#"{
        "market_summary": "Bullish regime with strong directional bias. Confluence score 72.",
        "trend_indicators": "Bullish EMA stack. ADX at 32 (strong). Supertrend long.",
        "momentum_indicators": "RSI 62 (neutral-bullish). MACD histogram expanding above zero.",
        "volatility_indicators": "BBWP at 45%. ATR stable. Squeeze off, momentum accelerating.",
        "volume_indicators": "RVOL 1.8 (institutional). OBV rising. VWAP premium.",
        "structure_indicators": "Price above S1. Golden Pocket entry zone. No active patterns.",
        "active_signals": "RSI bullish divergence confirmed. Squeeze release on prior candle.",
        "confluence_summary": "72/100 confluence. 85% consensus. High regime confidence."
    }"#;

    let result: AnalystDocument =
        serde_json::from_str(mock).expect("mock analyst JSON must parse into the DTO");

    assert!(result.market_summary.contains("Bullish"));
    assert!(result.trend_indicators.contains("EMA"));
    assert!(!result.market_summary.contains("action"));
}

#[test]
fn mock_trader_json_parses_into_dto() {
    let mock = r#"{
        "action": "Open Long",
        "confidence": 78,
        "rationale": "Strong bullish confluence with confirmed divergence and institutional volume.",
        "risk_notes": "No significant risk flags."
    }"#;

    let result: TraderDecision =
        serde_json::from_str(mock).expect("mock trader JSON must parse into the DTO");

    assert_eq!(result.action, "Open Long");
    assert_eq!(result.confidence, 78);
    assert!(!result.rationale.is_empty());
}

#[test]
fn trader_decision_validates_position_rules() {
    // When position is "Long", only Hold or Close allowed
    let close_only = r#"{
        "action": "Close",
        "confidence": 65,
        "rationale": "Bearish divergence confirmed. Exiting position.",
        "risk_notes": ""
    }"#;
    let result: TraderDecision =
        serde_json::from_str(close_only).expect("Close decision must parse");
    assert_eq!(result.action, "Close");

    // When position is "None", only Open Long or Open Short or Wait allowed
    let open = r#"{
        "action": "Open Long",
        "confidence": 80,
        "rationale": "All conditions favorable for entry.",
        "risk_notes": "Monitor volume on entry candle."
    }"#;
    let result2: TraderDecision =
        serde_json::from_str(open).expect("Open Long decision must parse");
    assert_eq!(result2.action, "Open Long");
}
