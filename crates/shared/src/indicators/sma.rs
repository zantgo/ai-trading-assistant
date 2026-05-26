use rust_decimal::Decimal;

/// Simple Moving Average
#[derive(Debug, Clone)]
pub struct Sma {
    period: usize,
    values: Vec<Decimal>,
}

impl Sma {
    pub fn new(period: usize) -> Self {
        Self { period, values: Vec::new() }
    }

    pub fn update(&mut self, val: Decimal) -> Option<Decimal> {
        self.values.push(val);
        if self.values.len() > self.period {
            self.values.remove(0);
        }
        if self.values.len() == self.period {
            let sum: Decimal = self.values.iter().sum();
            Some(sum / Decimal::from(self.period))
        } else {
            None
        }
    }
}
