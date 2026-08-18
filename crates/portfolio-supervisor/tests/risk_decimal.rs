//! Tests for the Decimal-based Position Sizing Protocol.
//! Verifies precision preservation, tick quantization, and the spec example
//! from `docs/engines/trade-automation-engine/03-03-03-tae-layer2-execution.md`.

use portfolio_supervisor::risk_calculator::{
    compute_risk, compute_risk_with_atr, RiskCalculationInput,
};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::str::FromStr;

fn input_long_basic() -> RiskCalculationInput {
    RiskCalculationInput {
        capital: dec!(10000),
        max_risk_pct: dec!(1),
        leverage: 10,
        direction: "LONG".to_string(),
        entry_price: dec!(100),
        stop_loss_price: dec!(99),
        take_profit_price: dec!(103),
        commission_pct: dec!(0.06),
        funding_rate_8h: dec!(0),
        spread: dec!(0),
        atr_value: None,
        atr_multiplier: None,
        atr_target_rr: None,
        use_dynamic_atr: false,
        min_tick_size: None,
    }
}

#[test]
fn spec_example_produces_exact_decimal() {
    let mut input = input_long_basic();
    input.entry_price = dec!(100);
    input.stop_loss_price = dec!(98.5);
    input.max_risk_pct = dec!(1);

    let calc = compute_risk(&input).unwrap();

    assert_eq!(calc.risk_capital, dec!(100));
    assert_eq!(calc.price_distance, dec!(1.5));

    let expected = Decimal::from_str("66.666666666666666666666666667").unwrap();
    assert_eq!(calc.position_size_units, expected);
}

#[test]
fn tick_size_quantization_rounds_down() {
    let mut input = input_long_basic();
    input.entry_price = dec!(100);
    input.stop_loss_price = dec!(98.5);
    input.max_risk_pct = dec!(1);
    input.min_tick_size = Some(dec!(0.01));

    let calc = compute_risk(&input).unwrap();

    assert_eq!(calc.position_size_units, dec!(66.66));
}

#[test]
fn zero_capital_returns_error() {
    let mut input = input_long_basic();
    input.capital = dec!(0);
    assert!(compute_risk(&input).is_err());
}

#[test]
fn zero_price_distance_returns_error() {
    let mut input = input_long_basic();
    input.stop_loss_price = input.entry_price;
    assert!(compute_risk(&input).is_err());
}

#[test]
fn atr_path_produces_decimal_consistent_with_hand_calc() {
    let mut input = input_long_basic();
    input.use_dynamic_atr = true;
    input.atr_value = Some(dec!(2));
    input.atr_multiplier = Some(dec!(2));
    input.atr_target_rr = Some(dec!(3));

    let calc = compute_risk_with_atr(&input).unwrap();

    assert_eq!(calc.price_distance, dec!(4));
    assert_eq!(calc.risk_capital, dec!(100));
    assert_eq!(calc.position_size_units, dec!(25));
}

#[test]
fn short_position_uses_correct_liquidation_direction() {
    let mut input = input_long_basic();
    input.direction = "SHORT".to_string();
    input.entry_price = dec!(100);
    input.stop_loss_price = dec!(101);
    input.take_profit_price = dec!(97);
    input.leverage = 10;

    let calc = compute_risk(&input).unwrap();

    assert_eq!(calc.liquidation_price, dec!(110));
}

#[test]
fn decimal_serialization_round_trips_as_string() {
    let calc = compute_risk(&input_long_basic()).unwrap();
    let json = serde_json::to_string(&calc).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed["position_size_units"].is_string());
}
