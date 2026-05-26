use rust_decimal::Decimal;

/// Exponential Moving Average
#[derive(Debug, Clone)]
pub struct Ema {
    period: usize,
    current_value: Option<Decimal>,
}

impl Ema {
    pub fn new(period: usize) -> Self {
        Self { period, current_value: None }
    }

    pub fn update(&mut self, price: Decimal) -> Decimal {
        match self.current_value {
            None => {
                self.current_value = Some(price);
                price
            }
            Some(prev_ema) => {
                let p_dec = Decimal::from(self.period);
                let multiplier = Decimal::from(2) / (p_dec + Decimal::ONE);
                let next_ema = (price - prev_ema) * multiplier + prev_ema;
                self.current_value = Some(next_ema);
                next_ema
            }
        }
    }
}
