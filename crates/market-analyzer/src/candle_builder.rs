use rust_decimal::Decimal;

#[derive(Debug, Clone)]
pub struct CandleBuilder {
    pub current_time: Option<u64>,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: Decimal,
}

impl CandleBuilder {
    pub fn new() -> Self {
        Self {
            current_time: None,
            open: Decimal::ZERO,
            high: Decimal::ZERO,
            low: Decimal::ZERO,
            close: Decimal::ZERO,
            volume: Decimal::ZERO,
        }
    }

    pub fn initialize(&mut self, rounded_time: u64, price: Decimal, vol: Decimal) {
        self.current_time = Some(rounded_time);
        self.open = price;
        self.high = price;
        self.low = price;
        self.close = price;
        self.volume = vol;
    }

    pub fn update_same_candle(&mut self, price: Decimal, vol: Decimal) {
        self.high = self.high.max(price);
        self.low = self.low.min(price);
        self.close = price;
        self.volume += vol;
    }

    pub fn reset_to(&mut self, rounded_time: u64, price: Decimal, vol: Decimal) {
        self.initialize(rounded_time, price, vol);
    }
}
