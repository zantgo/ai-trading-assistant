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
    pub fn update(&mut self, high: f64, low: f64, close: f64) -> Option<IchimokuOutput> {
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

    /// Soft-floor variant: produces a partial Ichimoku reading once at least
    /// `min_bars` candles have been seen, even when the buffer has not yet
    /// reached `tenkan_period` (9) / `kijun_period` (26) / `senkou_b_period`
    /// (52). The effective window for each base line is clamped to
    /// `min(buffer.len(), configured_period)`, so the reading converges to the
    /// strict `update()` output once the buffer reaches `senkou_b_period`.
    ///
    /// Mirrors the precedent set by Volume Profile's `compute_with_min_bars(25)`
    /// (see `crates/market-analyzer/src/analyzer/warm.rs:256`) and Hull MA's
    /// `update_with_min_bars`. Returns `None` if `buffer.len() < min_bars`.
    pub fn update_with_min_bars(
        &mut self,
        high: f64,
        low: f64,
        close: f64,
        min_bars: usize,
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

        let avail = self.highs.len();
        if avail < min_bars {
            return None;
        }

        // Effective periods: clamp each configured period to whatever the
        // buffer can support. Once avail reaches the configured window, this
        // collapses to the strict window and the output is identical to
        // `update()`.
        let eff_tenkan = self.tenkan_period.min(avail).max(1);
        let eff_kijun = self.kijun_period.min(avail).max(1);
        let eff_senkou_b = self.senkou_b_period.min(avail).max(1);

        let tenkan = self.midpoint(eff_tenkan)?;
        let kijun = self.midpoint(eff_kijun)?;
        let senkou_b = self.midpoint(eff_senkou_b)?;
        let senkou_a = (tenkan + kijun) / Decimal::from(2);

        self.projection_queue.push_back((senkou_a, senkou_b));
        let (senkou_a_current, senkou_b_current) =
            if self.projection_queue.len() > self.displacement {
                self.projection_queue
                    .pop_front()
                    .unwrap_or((senkou_a, senkou_b))
            } else {
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

    #[test]
    fn test_soft_floor_none_below_min_bars() {
        let mut ich = Ichimoku::new(9, 26, 52, 26);
        // Feed fewer than 9 candles; even soft-floor should refuse.
        let out = feed_ramp_with(&mut ich, 5, |i| (100.0 + i as f64, 100.0 + i as f64, 100.0 + i as f64), 9);
        assert!(out.is_none());
    }

    #[test]
    fn test_soft_floor_partial_reading_below_52() {
        let mut ich = Ichimoku::new(9, 26, 52, 26);
        // Feed 20 bars — strict `update()` returns None, soft-floor (min_bars=9)
        // must produce *something* with a Tenkan value.
        let out = feed_ramp_with(
            &mut ich,
            20,
            |i| (100.0 + i as f64, 100.0 + i as f64, 100.0 + i as f64),
            9,
        )
        .expect("soft-floor should yield a reading with 20 bars");
        // Tenkan window (9) is fully populated, so it should be defined.
        assert!(out.tenkan > dec!(0));
    }

    #[test]
    fn test_soft_floor_converges_to_strict_after_52() {
        let mut ich_soft = Ichimoku::new(9, 26, 52, 26);
        let mut ich_strict = Ichimoku::new(9, 26, 52, 26);
        // Feed the same 80-bar ramp into both, then compare.
        for i in 0..80 {
            let h = 100.0 + i as f64;
            let l = 100.0 + i as f64;
            let c = 100.0 + i as f64;
            ich_soft.update_with_min_bars(h, l, c, 9);
            ich_strict.update(h, l, c);
        }
        let soft = ich_soft
            .update_with_min_bars(180.0, 180.0, 180.0, 9)
            .unwrap();
        let strict = ich_strict.update(180.0, 180.0, 180.0).unwrap();
        // Once buffer ≥ 52 the soft-floor reading is identical to the strict
        // reading (both windows are full).
        assert_eq!(soft.tenkan, strict.tenkan);
        assert_eq!(soft.kijun, strict.kijun);
        assert_eq!(soft.senkou_a, strict.senkou_a);
        assert_eq!(soft.senkou_b, strict.senkou_b);
    }

    fn feed_ramp_with<F: Fn(usize) -> (f64, f64, f64)>(
        ich: &mut Ichimoku,
        n: usize,
        f: F,
        min_bars: usize,
    ) -> Option<IchimokuOutput> {
        let mut out = None;
        for i in 0..n {
            let (h, l, c) = f(i);
            out = ich.update_with_min_bars(h, l, c, min_bars);
        }
        out
    }
}
