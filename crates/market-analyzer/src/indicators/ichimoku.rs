use rust_decimal::Decimal;
use std::collections::VecDeque;

/// Ichimoku Cloud output for a completed candle.
///
/// `senkou_a` / `senkou_b` are the values computed *now* but which belong 26
/// candles in the *future* (rendered with a +displacement offset by the
/// frontend). `chikou` is the current close, rendered 26 candles *back*.
/// `senkou_a_current` / `senkou_b_current` are the cloud values that apply to
/// the current candle (i.e. the projections made `displacement` bars ago),
/// used for price-vs-cloud position and signals.
#[derive(Debug, Clone, Copy)]
pub struct IchimokuOutput {
    pub tenkan: Decimal,
    pub kijun: Decimal,
    /// Forward-projected leading span A (belongs +displacement bars ahead).
    pub senkou_a: Decimal,
    /// Forward-projected leading span B (belongs +displacement bars ahead).
    pub senkou_b: Decimal,
    /// Chikou (lagging) span = current close, plotted -displacement bars back.
    pub chikou: Decimal,
    /// Cloud span A applicable to the current candle.
    pub senkou_a_current: Decimal,
    /// Cloud span B applicable to the current candle.
    pub senkou_b_current: Decimal,
}

/// Ichimoku Kinko Hyo — a complete trend + dynamic S/R system.
///
/// Maintains rolling highs/lows for the Tenkan (9), Kijun (26), and Senkou B
/// (52) windows, plus a displacement queue holding the last `displacement`
/// (26) forward-projected cloud values so the value applicable to the current
/// candle can be retrieved.
#[derive(Debug, Clone)]
pub struct Ichimoku {
    tenkan_period: usize,
    kijun_period: usize,
    senkou_b_period: usize,
    displacement: usize,
    highs: VecDeque<Decimal>,
    lows: VecDeque<Decimal>,
    /// Forward cloud projections awaiting their applicable candle:
    /// each entry is (senkou_a, senkou_b) made `displacement` bars ago.
    projection_queue: VecDeque<(Decimal, Decimal)>,
}

impl Ichimoku {
    pub fn new(
        tenkan_period: usize,
        kijun_period: usize,
        senkou_b_period: usize,
        displacement: usize,
    ) -> Self {
        let cap = senkou_b_period.max(kijun_period).max(tenkan_period) + 2;
        Self {
            tenkan_period,
            kijun_period,
            senkou_b_period,
            displacement,
            highs: VecDeque::with_capacity(cap),
            lows: VecDeque::with_capacity(cap),
            projection_queue: VecDeque::with_capacity(displacement + 2),
        }
    }

    /// Midpoint (highest-high + lowest-low)/2 over the last `period` candles.
    /// Returns None until `period` candles are available.
    fn midpoint(&self, period: usize) -> Option<Decimal> {
        let len = self.highs.len();
        if len < period || period == 0 {
            return None;
        }
        let hi = self
            .highs
            .iter()
            .skip(len - period)
            .copied()
            .fold(Decimal::MIN, |a, b| a.max(b));
        let lo = self
            .lows
            .iter()
            .skip(len - period)
            .copied()
            .fold(Decimal::MAX, |a, b| a.min(b));
        Some((hi + lo) / Decimal::from(2))
    }

    /// Feed a completed candle. Returns the full Ichimoku reading once enough
    /// history exists (needs `senkou_b_period` candles for the base lines and
    /// `displacement` more projections for the current cloud).
    pub fn update(
        &mut self,
        high: f64,
        low: f64,
        close: f64,
    ) -> Option<IchimokuOutput> {
        let high = Decimal::from_f64_retain(high).unwrap_or(Decimal::ZERO);
        let low = Decimal::from_f64_retain(low).unwrap_or(Decimal::ZERO);
        let close = Decimal::from_f64_retain(close).unwrap_or(Decimal::ZERO);
        self.highs.push_back(high);
        self.lows.push_back(low);
        let cap = self.senkou_b_period + 2;
        while self.highs.len() > cap {
            self.highs.pop_front();
            self.lows.pop_front();
        }

        let tenkan = self.midpoint(self.tenkan_period)?;
        let kijun = self.midpoint(self.kijun_period)?;
        let senkou_b = self.midpoint(self.senkou_b_period)?;
        let senkou_a = (tenkan + kijun) / Decimal::from(2);

        // Push this bar's forward projection; the current-applicable cloud is
        // the projection made `displacement` bars ago (front of the queue once
        // it has filled).
        self.projection_queue.push_back((senkou_a, senkou_b));
        let (senkou_a_current, senkou_b_current) =
            if self.projection_queue.len() > self.displacement {
                self.projection_queue
                    .pop_front()
                    .unwrap_or((senkou_a, senkou_b))
            } else {
                // Not enough forward history yet — use the live cloud as a fallback.
                (senkou_a, senkou_b)
            };

        Some(IchimokuOutput {
            tenkan,
            kijun,
            senkou_a,
            senkou_b,
            chikou: close,
            senkou_a_current,
            senkou_b_current,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn feed_ramp(ich: &mut Ichimoku, n: usize, start: f64, step: f64) -> Option<IchimokuOutput> {
        let mut out = None;
        for i in 0..n {
            let base = start + step * i as f64;
            let h = base + 1.0;
            let l = base - 1.0;
            let c = base;
            out = ich.update(h, l, c);
        }
        out
    }

    #[test]
    fn test_none_before_warmup() {
        let mut ich = Ichimoku::new(9, 26, 52, 26);
        // Fewer than 52 candles → None.
        let out = feed_ramp(&mut ich, 40, 100.0, 1.0);
        assert!(out.is_none());
    }

    #[test]
    fn test_produces_output_after_warmup() {
        let mut ich = Ichimoku::new(9, 26, 52, 26);
        let out = feed_ramp(&mut ich, 60, 100.0, 1.0);
        assert!(out.is_some());
    }

    #[test]
    fn test_uptrend_tenkan_above_kijun() {
        let mut ich = Ichimoku::new(9, 26, 52, 26);
        // Steady uptrend: faster Tenkan should sit above slower Kijun.
        let out = feed_ramp(&mut ich, 60, 100.0, 1.0).unwrap();
        assert!(
            out.tenkan > out.kijun,
            "uptrend: Tenkan should exceed Kijun"
        );
    }

    #[test]
    fn test_senkou_a_is_tenkan_kijun_midpoint() {
        let mut ich = Ichimoku::new(9, 26, 52, 26);
        let out = feed_ramp(&mut ich, 60, 100.0, 1.0).unwrap();
        let expected = (out.tenkan + out.kijun) / dec!(2);
        assert_eq!(out.senkou_a, expected);
    }

    #[test]
    fn test_chikou_is_current_close() {
        let mut ich = Ichimoku::new(9, 26, 52, 26);
        // Last close in the ramp of 60 starting at 100 step 1 → 100 + 59 = 159.
        let out = feed_ramp(&mut ich, 60, 100.0, 1.0).unwrap();
        assert_eq!(out.chikou, dec!(159));
    }

    #[test]
    fn test_flat_market_lines_converge() {
        let mut ich = Ichimoku::new(9, 26, 52, 26);
        let out = feed_ramp(&mut ich, 60, 100.0, 0.0).unwrap();
        // Flat prices → all base lines equal the flat level.
        assert_eq!(out.tenkan, out.kijun);
        assert_eq!(out.senkou_a, out.senkou_b);
    }
}
