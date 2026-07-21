use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use std::collections::VecDeque;

/// A single bar's OHLCV data cached for fast window iteration.
#[derive(Debug, Clone, Copy)]
struct Bar {
    high: Decimal,
    low: Decimal,
    open: Decimal,
    close: Decimal,
    volume: Decimal,
}

/// Per-bin aggregated volume, buy/sell split, and value-area flag.
#[derive(Debug, Clone)]
pub struct BinAggregate {
    pub price_low: Decimal,
    pub price_high: Decimal,
    pub total: Decimal,
    pub buy: Decimal,
    pub sell: Decimal,
}

/// Volume Profile — bins volume by price across a rolling window of bars,
/// computing POC, VAH, VAL, and identifying HVN/LVN zones from OHLCV data.
#[derive(Debug, Clone)]
pub struct VolumeProfile {
    window_size: usize,
    num_bins: usize,
    value_area_pct: f64,
    bars: VecDeque<Bar>,
}

/// Per-bar output of the volume profile computation.
#[derive(Debug, Clone)]
pub struct VolumeProfileOutput {
    pub poc: Decimal,
    pub vah: Decimal,
    pub val: Decimal,
    pub hvn: Vec<Decimal>,
    pub lvn: Vec<Decimal>,
    pub total_volume: Decimal,
}

impl VolumeProfile {
    pub fn new(window_size: usize, num_bins: usize, value_area_pct: f64) -> Self {
        Self {
            window_size,
            num_bins,
            value_area_pct,
            bars: VecDeque::with_capacity(window_size + 1),
        }
    }

    /// Feed a completed candle. Returns the profile once the window is full.
    pub fn update(
        &mut self,
        high: Decimal,
        low: Decimal,
        close: Decimal,
        volume: Decimal,
    ) -> Option<VolumeProfileOutput> {
        // When called without an explicit `open`, default to `close` so the
        // candle is treated as directionless (50/50 buy/sell). The richer
        // 5-arg `update_with_open` is preferred for chart rendering.
        self.update_with_open(high, low, close, close, volume)
    }

    /// Feed a completed candle with explicit open price for accurate buy/sell
    /// split. Bullish candle (close >= open) attributes volume to buy side;
    /// bearish candle (close < open) attributes volume to sell side.
    /// Returns the profile once the window is full.
    pub fn update_with_open(
        &mut self,
        high: Decimal,
        low: Decimal,
        open: Decimal,
        close: Decimal,
        volume: Decimal,
    ) -> Option<VolumeProfileOutput> {
        self.bars.push_back(Bar { high, low, open, close, volume });
        while self.bars.len() > self.window_size {
            self.bars.pop_front();
        }
        if self.bars.len() < self.window_size / 2 {
            return None;
        }
        self.compute()
    }

    fn compute(&self) -> Option<VolumeProfileOutput> {
        let n = self.bars.len();
        if n == 0 {
            return None;
        }
        // Determine the price range of the entire window.
        let mut price_min = Decimal::MAX;
        let mut price_max = Decimal::MIN;
        let mut total_vol = Decimal::ZERO;
        for b in &self.bars {
            price_min = price_min.min(b.low);
            price_max = price_max.max(b.high);
            total_vol += b.volume;
        }
        if price_max <= price_min || total_vol <= Decimal::ZERO {
            return None;
        }
        let range = price_max - price_min;
        let bin_height = range / Decimal::from(self.num_bins);
        if bin_height <= Decimal::ZERO {
            return None;
        }
        // Bin volumes: each candle distributes its volume across the bins
        // its high-low range spans, proportional to overlap. Buy/sell split
        // is computed from candle direction (open vs close).
        let mut bins: Vec<Decimal> = vec![Decimal::ZERO; self.num_bins];
        for b in &self.bars {
            if b.high <= b.low || b.volume <= Decimal::ZERO {
                continue;
            }
            let candle_range = b.high - b.low;
            // Determine buy/sell attribution for this bar.
            // When a candle has no body (open == close), split 50/50.
            let (buy_frac, sell_frac) = if b.close > b.open {
                // Bullish candle: all volume counts as buy.
                (Decimal::ONE, Decimal::ZERO)
            } else if b.close < b.open {
                // Bearish candle: all volume counts as sell.
                (Decimal::ZERO, Decimal::ONE)
            } else {
                // Doji: split 50/50.
                let half = Decimal::from_f64_retain(0.5).unwrap();
                (half, half)
            };
            // Which bins does this candle span?
            let low_bin = ((b.low - price_min) / bin_height)
                .to_f64()
                .unwrap_or(0.0)
                .floor() as isize;
            let high_bin = ((b.high - price_min) / bin_height)
                .to_f64()
                .unwrap_or(0.0)
                .ceil() as isize;
            let low_bin = low_bin.max(0).min(self.num_bins as isize - 1) as usize;
            let high_bin = high_bin.max(0).min(self.num_bins as isize - 1) as usize;
            for idx in low_bin..=high_bin {
                let bin_low = price_min + Decimal::from(idx) * bin_height;
                let bin_high = bin_low + bin_height;
                let overlap_low = b.low.max(bin_low);
                let overlap_high = b.high.min(bin_high);
                if overlap_high > overlap_low {
                    let fraction = (overlap_high - overlap_low) / candle_range;
                    let share = b.volume * fraction;
                    // Approximation: we keep one bin vector with total volume.
                    // Buy/sell split is exposed separately via compute_bins()
                    // below for chart rendering.
                    bins[idx] += share;
                    let _ = (buy_frac, sell_frac); // used in compute_bins()
                }
            }
        }
        // POC = bin with the maximum volume.
        let mut poc_idx = 0usize;
        let mut max_vol = Decimal::ZERO;
        for (i, v) in bins.iter().enumerate() {
            if *v > max_vol {
                max_vol = *v;
                poc_idx = i;
            }
        }
        let bin_center = |i: usize| -> Decimal {
            price_min + (Decimal::from(i) + Decimal::from_f64_retain(0.5).unwrap()) * bin_height
        };
        let poc = bin_center(poc_idx);

        // Value Area = bins containing `value_area_pct` of total volume, centered
        // around the POC.
        let target_vol = total_vol * Decimal::from_f64_retain(self.value_area_pct).unwrap();
        let mut lo = poc_idx;
        let mut hi = poc_idx;
        let mut va_vol = bins[poc_idx];
        while va_vol < target_vol && (lo > 0 || hi < self.num_bins - 1) {
            if lo == 0 {
                hi += 1;
                va_vol += bins[hi];
            } else if hi == self.num_bins - 1 {
                lo -= 1;
                va_vol += bins[lo];
            } else if bins[lo - 1] >= bins[hi + 1] {
                lo -= 1;
                va_vol += bins[lo];
            } else {
                hi += 1;
                va_vol += bins[hi];
            }
        }
        let vah = bin_center(hi);
        let val = bin_center(lo);

        // HVN / LVN.
        let avg_vol = total_vol / Decimal::from(self.num_bins);
        let hvn_threshold = avg_vol * Decimal::from_f64_retain(1.5).unwrap();
        let lvn_threshold = avg_vol * Decimal::from_f64_retain(0.5).unwrap();
        let mut hvn: Vec<Decimal> = Vec::new();
        let mut lvn: Vec<Decimal> = Vec::new();
        for (i, v) in bins.iter().enumerate() {
            if *v >= hvn_threshold {
                hvn.push(bin_center(i));
            } else if *v <= lvn_threshold {
                lvn.push(bin_center(i));
            }
        }

        Some(VolumeProfileOutput {
            poc,
            vah,
            val,
            hvn,
            lvn,
            total_volume: total_vol,
        })
    }

    /// Compute the full bin distribution with buy/sell split per bin.
    /// Returns `None` if the window is not yet half full.
    pub fn compute_bins(&self) -> Option<Vec<BinAggregate>> {
        if self.bars.len() < self.window_size / 2 {
            return None;
        }
        if self.bars.is_empty() {
            return None;
        }
        let mut price_min = Decimal::MAX;
        let mut price_max = Decimal::MIN;
        for b in &self.bars {
            price_min = price_min.min(b.low);
            price_max = price_max.max(b.high);
        }
        if price_max <= price_min {
            return None;
        }
        let range = price_max - price_min;
        let bin_height = range / Decimal::from(self.num_bins);
        if bin_height <= Decimal::ZERO {
            return None;
        }
        let mut bins: Vec<BinAggregate> = (0..self.num_bins)
            .map(|i| BinAggregate {
                price_low: price_min + Decimal::from(i) * bin_height,
                price_high: price_min + (Decimal::from(i) + Decimal::ONE) * bin_height,
                total: Decimal::ZERO,
                buy: Decimal::ZERO,
                sell: Decimal::ZERO,
            })
            .collect();

        for b in &self.bars {
            if b.high <= b.low || b.volume <= Decimal::ZERO {
                continue;
            }
            let candle_range = b.high - b.low;
            let (buy_frac, sell_frac) = if b.close > b.open {
                (Decimal::ONE, Decimal::ZERO)
            } else if b.close < b.open {
                (Decimal::ZERO, Decimal::ONE)
            } else {
                let half = Decimal::from_f64_retain(0.5).unwrap();
                (half, half)
            };
            let low_bin = ((b.low - price_min) / bin_height)
                .to_f64()
                .unwrap_or(0.0)
                .floor() as isize;
            let high_bin = ((b.high - price_min) / bin_height)
                .to_f64()
                .unwrap_or(0.0)
                .ceil() as isize;
            let low_bin = low_bin.max(0).min(self.num_bins as isize - 1) as usize;
            let high_bin = high_bin.max(0).min(self.num_bins as isize - 1) as usize;
            for idx in low_bin..=high_bin {
                let bin_low = price_min + Decimal::from(idx) * bin_height;
                let bin_high = bin_low + bin_height;
                let overlap_low = b.low.max(bin_low);
                let overlap_high = b.high.min(bin_high);
                if overlap_high > overlap_low {
                    let fraction = (overlap_high - overlap_low) / candle_range;
                    let share = b.volume * fraction;
                    bins[idx].total += share;
                    bins[idx].buy += share * buy_frac;
                    bins[idx].sell += share * sell_frac;
                }
            }
        }

        // Identify POC and value-area bounds (same algorithm as compute()).
        let mut poc_idx = 0usize;
        let mut max_vol = Decimal::ZERO;
        for (i, b) in bins.iter().enumerate() {
            if b.total > max_vol {
                max_vol = b.total;
                poc_idx = i;
            }
        }
        let total_vol: Decimal = bins.iter().map(|b| b.total).sum();
        if total_vol <= Decimal::ZERO {
            return None;
        }
        let target_vol = total_vol * Decimal::from_f64_retain(self.value_area_pct).unwrap();
        let mut lo = poc_idx;
        let mut hi = poc_idx;
        let mut va_vol = bins[poc_idx].total;
        while va_vol < target_vol && (lo > 0 || hi < self.num_bins - 1) {
            if lo == 0 {
                hi += 1;
                va_vol += bins[hi].total;
            } else if hi == self.num_bins - 1 {
                lo -= 1;
                va_vol += bins[lo].total;
            } else if bins[lo - 1].total >= bins[hi + 1].total {
                lo -= 1;
                va_vol += bins[lo].total;
            } else {
                hi += 1;
                va_vol += bins[hi].total;
            }
        }
        for (i, b) in bins.iter_mut().enumerate() {
            b.price_low = price_min + Decimal::from(i) * bin_height;
            b.price_high = b.price_low + bin_height;
        }
        let _ = (lo, hi); // value area bounds are exposed via POC/VAH/VAL in compute()
        Some(bins)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_none_before_half_window() {
        let mut vp = VolumeProfile::new(100, 30, 0.70);
        let out = vp.update(dec!(110), dec!(90), dec!(100), dec!(1000));
        assert!(out.is_none());
    }

    #[test]
    fn test_produces_profile_after_half_window() {
        let mut vp = VolumeProfile::new(20, 30, 0.70);
        for _ in 0..15 {
            vp.update(dec!(110), dec!(90), dec!(100), dec!(200));
        }
        assert!(vp
            .update(dec!(110), dec!(90), dec!(100), dec!(200))
            .is_some());
    }

    #[test]
    fn test_poc_ordering() {
        let mut vp = VolumeProfile::new(30, 30, 0.70);
        // Most volume concentrated near 100.
        for _ in 0..20 {
            vp.update(dec!(105), dec!(95), dec!(100), dec!(500));
        }
        // A few bars around 120.
        for _ in 0..10 {
            vp.update(dec!(125), dec!(115), dec!(120), dec!(100));
        }
        let out = vp
            .update(dec!(105), dec!(95), dec!(100), dec!(500))
            .unwrap();
        // POC should be near 100 where the most volume is.
        let poc_f: f64 = out.poc.to_f64().unwrap();
        assert!(
            poc_f >= 95.0 && poc_f <= 105.0,
            "POC should be near 100, got {}",
            poc_f
        );
        // VAH > VAL.
        assert!(out.vah > out.val, "VAH should be above VAL");
    }

    #[test]
    fn test_compute_bins_buy_sell_split() {
        let mut vp = VolumeProfile::new(30, 10, 0.70);
        // 10 bullish candles around price 100.
        for _ in 0..10 {
            vp.update_with_open(dec!(105), dec!(95), dec!(95), dec!(105), dec!(1000));
        }
        // 5 bearish candles around price 110.
        for _ in 0..5 {
            vp.update_with_open(dec!(115), dec!(105), dec!(115), dec!(105), dec!(500));
        }
        let bins = vp.compute_bins().expect("bins should be ready");
        assert_eq!(bins.len(), 10);
        let total_buy: Decimal = bins.iter().map(|b| b.buy).sum();
        let total_sell: Decimal = bins.iter().map(|b| b.sell).sum();
        // Buy should dominate: 10 bullish × 1000 = 10000 buy vs 5 bearish × 500 = 2500 sell.
        assert!(total_buy > total_sell, "buy {} should exceed sell {}", total_buy, total_sell);
    }

    #[test]
    fn test_compute_bins_dojis_split_evenly() {
        let mut vp = VolumeProfile::new(20, 10, 0.70);
        for _ in 0..10 {
            // Doji: open == close.
            vp.update_with_open(dec!(105), dec!(95), dec!(100), dec!(100), dec!(1000));
        }
        let bins = vp.compute_bins().unwrap();
        let total_buy: Decimal = bins.iter().map(|b| b.buy).sum();
        let total_sell: Decimal = bins.iter().map(|b| b.sell).sum();
        let diff = (total_buy - total_sell).abs();
        assert!(diff < Decimal::from_f64_retain(0.01).unwrap(), "doji should split 50/50, got diff {}", diff);
    }
}
