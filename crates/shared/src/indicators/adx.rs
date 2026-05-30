use rust_decimal::Decimal;
use super::ema::Ema;

/// Average Directional Index (using standard True Range & Wilder's smoothing)
#[derive(Debug, Clone)]
pub struct Adx {
    prev_high: Option<Decimal>,
    prev_low: Option<Decimal>,
    prev_close: Option<Decimal>,
    tr_ema: Ema,
    plus_dm_ema: Ema,
    minus_dm_ema: Ema,
    dx_ema: Ema,
}

impl Adx {
    pub fn new(period: usize) -> Self {
        Self {
            prev_high: None,
            prev_low: None,
            prev_close: None,
            tr_ema: Ema::new(period),
            plus_dm_ema: Ema::new(period),
            minus_dm_ema: Ema::new(period),
            dx_ema: Ema::new(period),
        }
    }

    pub fn update(&mut self, high: Decimal, low: Decimal, close: Decimal) -> Option<(Decimal, Decimal, Decimal)> {
        let (p_high, p_low, p_close) = match (self.prev_high, self.prev_low, self.prev_close) {
            (Some(h), Some(l), Some(c)) => (h, l, c),
            _ => {
                self.prev_high = Some(high);
                self.prev_low = Some(low);
                self.prev_close = Some(close);
                return None;
            }
        };

        self.prev_high = Some(high);
        self.prev_low = Some(low);
        self.prev_close = Some(close);

        let r1 = high - low;
        let r2 = (high - p_close).abs();
        let r3 = (low - p_close).abs();
        let tr = r1.max(r2).max(r3);

        let up_move = high - p_high;
        let down_move = p_low - low;

        let plus_dm = if up_move > down_move && up_move > Decimal::ZERO { up_move } else { Decimal::ZERO };
        let minus_dm = if down_move > up_move && down_move > Decimal::ZERO { down_move } else { Decimal::ZERO };

        let tr_smooth = self.tr_ema.update(tr);
        let plus_dm_smooth = self.plus_dm_ema.update(plus_dm);
        let minus_dm_smooth = self.minus_dm_ema.update(minus_dm);

        if tr_smooth == Decimal::ZERO {
            return None;
        }

        let plus_di = (plus_dm_smooth / tr_smooth) * Decimal::from(100);
        let minus_di = (minus_dm_smooth / tr_smooth) * Decimal::from(100);

        let di_sum = plus_di + minus_di;
        if di_sum == Decimal::ZERO {
            return None;
        }

        let dx = ((plus_di - minus_di).abs() / di_sum) * Decimal::from(100);
        let adx = self.dx_ema.update(dx);

        Some((adx, plus_di, minus_di))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_first_update_returns_none() {
        let mut adx = Adx::new(14);
        assert_eq!(adx.update(dec!(100.00), dec!(95.00), dec!(98.00)), None);
    }

    #[test]
    fn test_strong_up_trend_plus_di_above_minus_di() {
        let mut adx = Adx::new(14);
        let mut high = dec!(100.00);
        let mut low = dec!(95.00);
        let mut close = dec!(98.00);
        adx.update(high, low, close);

        for _ in 0..20 {
            high += dec!(2.00);
            low += dec!(1.50);
            close += dec!(2.00);
            adx.update(high, low, close);
        }

        let (_adx_val, plus_di, minus_di) = adx.update(high + dec!(2.00), low + dec!(1.50), close + dec!(2.00)).unwrap();
        assert!(plus_di > minus_di, "Strong uptrend: +DI should exceed -DI");
    }

    #[test]
    fn test_zero_movement_periods_produce_symmetric_di() {
        let mut adx = Adx::new(14);
        let price = dec!(100.00);
        adx.update(price, price, price);
        for _ in 0..20 {
            let result = adx.update(price, price, price);
            if let Some((_adx, plus_di, minus_di)) = result {
                assert!((plus_di - minus_di).abs() < dec!(1.00),
                    "Zero movement: +DI and -DI should be near equal");
            }
        }
    }
}
