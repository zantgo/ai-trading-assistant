use rust_decimal::Decimal;

/// Weighted Moving Average helper.
fn wma(values: &[Decimal]) -> Decimal {
    let n = values.len();
    if n == 0 {
        return Decimal::ZERO;
    }
    let mut num = Decimal::ZERO;
    let mut den = Decimal::ZERO;
    for (i, v) in values.iter().enumerate() {
        let w = Decimal::from(i + 1);
        num += *v * w;
        den += w;
    }
    num / den
}

/// Hull Moving Average — a near-zero-lag weighted moving average designed to
/// reduce the lag present in traditional moving averages while maintaining
/// smoothness.
#[derive(Debug, Clone)]
pub struct HullMA {
    period: usize,
    sqrt_period: usize,
    values: Vec<Decimal>,
    diff_buffer: Vec<Decimal>,
}

impl HullMA {
    pub fn new(period: usize) -> Self {
        let sqrt = (period as f64).sqrt().round() as usize;
        Self {
            period,
            sqrt_period: sqrt.max(1),
            values: Vec::with_capacity(period * 2),
            diff_buffer: Vec::with_capacity(period * 2),
        }
    }

    pub fn update(&mut self, price: f64) -> Option<Decimal> {
        let price = Decimal::from_f64_retain(price).unwrap_or(Decimal::ZERO);
        self.values.push(price);
        let n2 = self.period / 2;
        let n = self.period;
        if self.values.len() < n {
            return None;
        }
        let recent: Vec<Decimal> = self
            .values
            .iter()
            .rev()
            .take(n)
            .copied()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        let wma_half = wma(&recent[n - n2..]);
        let wma_full = wma(&recent);
        let diff = Decimal::from(2) * wma_half - wma_full;
        self.diff_buffer.push(diff);
        let sqrt_n = self.sqrt_period.min(self.diff_buffer.len());
        let wma_diff = wma(&self.diff_buffer[self.diff_buffer.len() - sqrt_n..]);
        Some(wma_diff)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_none_before_period() {
        let mut hma = HullMA::new(16);
        for _ in 0..15 {
            assert!(hma.update(100.0).is_none());
        }
    }

    #[test]
    fn test_produces_output_after_period() {
        let mut hma = HullMA::new(16);
        for _ in 0..16 {
            hma.update(100.0);
        }
        assert!(hma.update(100.0).is_some());
    }
}
