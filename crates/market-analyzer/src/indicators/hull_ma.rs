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
        if self.values.len() > self.period * 2 {
            self.values.remove(0);
        }
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
        if self.diff_buffer.len() > self.period * 2 {
            self.diff_buffer.remove(0);
        }
        let sqrt_n = self.sqrt_period.min(self.diff_buffer.len());
        let wma_diff = wma(&self.diff_buffer[self.diff_buffer.len() - sqrt_n..]);
        Some(wma_diff)
    }

    /// Soft-floor variant: produces a partial Hull MA reading once at least
    /// `min_bars` values have been seen, using a smaller effective `n` when
    /// the buffer has not yet reached the full `period`. This mirrors the
    /// pattern established by Volume Profile's `compute_with_min_bars(25)`
    /// (see `crates/market-analyzer/src/analyzer/warm.rs:256`) and lets the
    /// indicator surface a Live reading on sub-minute timeframes where the
    /// venue's historical fetch returns fewer bars than the configured
    /// `hull_ma_period`. The reading is mathematically a *partial* Hull MA
    /// until the buffer reaches `period`; afterwards it is identical to
    /// `update()`.
    pub fn update_with_min_bars(&mut self, price: f64, min_bars: usize) -> Option<Decimal> {
        let price = Decimal::from_f64_retain(price).unwrap_or(Decimal::ZERO);
        self.values.push(price);
        // AUDIT-AIU-005: capacity cap — the buffer was previously never
        // trimmed, growing ~1.4 MB/day/symbol-TF at 1 s candles.
        if self.values.len() > self.period * 2 {
            self.values.remove(0);
        }
        let avail = self.values.len();
        if avail < min_bars || self.period == 0 {
            return None;
        }
        let n = avail.min(self.period);
        let n2 = (n / 2).max(1);
        // Need at least n values; avail >= min_bars && min_bars <= period &&
        // n = min(avail, period), so avail >= n by construction.
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
        // AUDIT-AIU-005: same bounded-memory cap as `update()`.
        if self.diff_buffer.len() > self.period * 2 {
            self.diff_buffer.remove(0);
        }
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

    #[test]
    fn test_soft_floor_none_below_min_bars() {
        let mut hma = HullMA::new(16);
        for _ in 0..4 {
            assert!(hma.update_with_min_bars(100.0, 5).is_none());
        }
    }

    #[test]
    fn test_soft_floor_partial_reading_above_min_bars() {
        let mut hma = HullMA::new(16);
        for _ in 0..6 {
            hma.update_with_min_bars(100.0, 5);
        }
        // 6 ≥ 5 (min_bars), < 16 (period) — should produce a partial reading.
        let out = hma.update_with_min_bars(100.0, 5);
        assert!(out.is_some());
    }

    #[test]
    fn test_soft_floor_converges_to_strict_after_period() {
        let mut soft = HullMA::new(16);
        let mut strict = HullMA::new(16);
        for _ in 0..20 {
            soft.update_with_min_bars(100.0, 5);
            strict.update(100.0);
        }
        let soft_val = soft.update_with_min_bars(100.0, 5).unwrap();
        let strict_val = strict.update(100.0).unwrap();
        // Once `values.len() >= period`, both paths are mathematically
        // equivalent — every intermediate value was fed identically.
        assert_eq!(soft_val, strict_val);
    }
}
