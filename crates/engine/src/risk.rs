use rust_decimal::Decimal;
use shared::models::MarketSnapshot;

pub fn check(tick: &MarketSnapshot) {
    let current_price = tick.mid_price;
    if current_price < Decimal::new(1, 2) {
        eprintln!("⚠️ RISK ENGINE ALERT: {} price ({:.4}) below absolute safety margin of $0.01!", tick.symbol, current_price);
    }
}
