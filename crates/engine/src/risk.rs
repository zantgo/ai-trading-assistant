use rust_decimal::Decimal;
use shared::models::MarketSnapshot;

pub fn check(tick: &MarketSnapshot) {
    let current_price = tick.mid_price;
    if current_price < Decimal::from(1000) {
        eprintln!("⚠️ RISK ENGINE ALERT: {} price ({:.2}) below safety margin!", tick.symbol, current_price);
    }
}
