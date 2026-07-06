use super::atr::Atr;
use super::traits::{BarInput, Indicator};
use rust_decimal::Decimal;

#[derive(Debug, Clone)]
pub struct SupertrendOutput {
    /// Active Supertrend line value (the trailing stop level).
    pub line: Decimal,
    /// +1 uptrend (price above line), -1 downtrend.
    pub direction: i8,
    /// True on the bar where the trend direction flipped.
    pub flipped: bool,
}

/// Supertrend: ATR-based trailing stop / trend direction indicator.
#[derive(Debug, Clone)]
pub struct Supertrend {
    period: usize,
    multiplier: Decimal,
    atr: Atr,
    prev_close: Option<Decimal>,
    prev_final_upper: Option<Decimal>,
    prev_final_lower: Option<Decimal>,
    prev_direction: i8,
}

impl Supertrend {
    pub fn new(period: usize, multiplier: f64) -> Self {
        Self {
            period,
            multiplier: Decimal::from_f64_retain(multiplier).unwrap_or(Decimal::from(3)),
            atr: Atr::new(period),
            prev_close: None,
            prev_final_upper: None,
            prev_final_lower: None,
            prev_direction: 1,
        }
    }

    pub fn update(&mut self, high: Decimal, low: Decimal, close: Decimal) -> Option<SupertrendOutput> {
        if self.period == 0 {
            return None;
        }
        let atr = self.atr.update(high, low, close)?.atr_value;
        let hl2 = (high + low) / Decimal::from(2);
        let basic_upper = hl2 + self.multiplier * atr;
        let basic_lower = hl2 - self.multiplier * atr;

        let prev_close = self.prev_close.unwrap_or(close);
        let prev_final_upper = self.prev_final_upper.unwrap_or(basic_upper);
        let prev_final_lower = self.prev_final_lower.unwrap_or(basic_lower);

        let final_upper = if basic_upper < prev_final_upper || prev_close > prev_final_upper {
            basic_upper
        } else {
            prev_final_upper
        };
        let final_lower = if basic_lower > prev_final_lower || prev_close < prev_final_lower {
            basic_lower
        } else {
            prev_final_lower
        };

        // Determine direction: default carry previous, flip on band breach.
        let mut direction = self.prev_direction;
        if self.prev_close.is_some() {
            if self.prev_direction == 1 {
                if close < final_lower {
                    direction = -1;
                }
            } else if close > final_upper {
                direction = 1;
            }
        } else {
            direction = if close >= hl2 { 1 } else { -1 };
        }

        let line = if direction == 1 { final_lower } else { final_upper };
        let flipped = self.prev_close.is_some() && direction != self.prev_direction;

        self.prev_close = Some(close);
        self.prev_final_upper = Some(final_upper);
        self.prev_final_lower = Some(final_lower);
        self.prev_direction = direction;

        Some(SupertrendOutput { line, direction, flipped })
    }
}

impl Indicator for Supertrend {
    type Output = Option<SupertrendOutput>;
    fn update(&mut self, bar: &BarInput) -> Self::Output {
        self.update(bar.high, bar.low, bar.close)
    }
    fn reset(&mut self) {
        let m = self.multiplier.try_into().unwrap_or(3.0);
        *self = Supertrend::new(self.period, m);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn feed(st: &mut Supertrend, h: f64, l: f64, c: f64) -> Option<SupertrendOutput> {
        st.update(
            Decimal::from_f64_retain(h).unwrap(),
            Decimal::from_f64_retain(l).unwrap(),
            Decimal::from_f64_retain(c).unwrap(),
        )
    }

    #[test]
    fn test_zero_period_none() {
        let mut st = Supertrend::new(0, 3.0);
        assert!(feed(&mut st, 10.0, 9.0, 9.5).is_none());
    }

    #[test]
    fn test_uptrend_direction_positive() {
        let mut st = Supertrend::new(3, 2.0);
        let mut price = 100.0;
        let mut last = None;
        for _ in 0..20 {
            price += 2.0;
            last = feed(&mut st, price + 1.0, price - 1.0, price);
        }
        let out = last.unwrap();
        assert_eq!(out.direction, 1, "sustained uptrend should be +1");
        assert!(out.line < dec!(200), "line trails below price in uptrend");
    }

    #[test]
    fn test_downtrend_direction_negative() {
        let mut st = Supertrend::new(3, 2.0);
        let mut price = 200.0;
        let mut last = None;
        for _ in 0..20 {
            price -= 2.0;
            last = feed(&mut st, price + 1.0, price - 1.0, price);
        }
        assert_eq!(last.unwrap().direction, -1);
    }
}
