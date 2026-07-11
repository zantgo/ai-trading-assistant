use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MarketStructure {
    Bullish,
    Bearish,
    Neutral,
}

#[derive(Debug, Clone)]
pub struct SmcOutput {
    pub structure: MarketStructure,
    pub bos_bullish: bool,
    pub bos_bearish: bool,
    pub choch_bullish: bool,
    pub choch_bearish: bool,
    pub liquidity_sweep_buy: bool,
    pub liquidity_sweep_sell: bool,
    pub active_ob_bullish_high: Option<Decimal>,
    pub active_ob_bullish_low: Option<Decimal>,
    pub active_ob_bearish_high: Option<Decimal>,
    pub active_ob_bearish_low: Option<Decimal>,
    pub fvg_top: Option<Decimal>,
    pub fvg_bottom: Option<Decimal>,
    pub fvg_bullish: bool,
    pub premium_discount: f64,
}

#[derive(Debug, Clone)]
pub struct SmartMoney {
    lookback: usize,
    prices: VecDeque<(Decimal, Decimal, Decimal, Decimal)>,
    prev_structure: MarketStructure,
    ob_bullish_high: Option<Decimal>,
    ob_bullish_low: Option<Decimal>,
    ob_bearish_high: Option<Decimal>,
    ob_bearish_low: Option<Decimal>,
    fvg_top: Option<Decimal>,
    fvg_bottom: Option<Decimal>,
    fvg_bullish: bool,
}

impl SmartMoney {
    pub fn new(lookback: usize) -> Self {
        Self {
            lookback,
            prices: VecDeque::with_capacity(lookback + 5),
            prev_structure: MarketStructure::Neutral,
            ob_bullish_high: None,
            ob_bullish_low: None,
            ob_bearish_high: None,
            ob_bearish_low: None,
            fvg_top: None,
            fvg_bottom: None,
            fvg_bullish: false,
        }
    }

    pub fn update(
        &mut self,
        open: Decimal,
        high: Decimal,
        low: Decimal,
        close: Decimal,
    ) -> Option<SmcOutput> {
        self.prices.push_back((open, high, low, close));
        while self.prices.len() > self.lookback + 3 {
            self.prices.pop_front();
        }
        if self.prices.len() < 5 {
            return None;
        }

        let n = self.prices.len();
        let bars: Vec<(Decimal, Decimal, Decimal, Decimal)> =
            self.prices.iter().copied().collect();

        let mut structure = MarketStructure::Neutral;
        let mut bos_bull = false;
        let mut bos_bear = false;
        let mut choch_bull = false;
        let mut choch_bear = false;

        let mut sh_idx = Vec::new();
        let mut sl_idx = Vec::new();
        for i in 1..n - 1 {
            let h = bars[i].1;
            let l = bars[i].2;
            if h >= bars[i - 1].1 && h >= bars[i + 1].1 {
                sh_idx.push((i, h));
            }
            if l <= bars[i - 1].2 && l <= bars[i + 1].2 {
                sl_idx.push((i, l));
            }
        }
        if sh_idx.len() >= 2 && sl_idx.len() >= 2 {
            let last_sh = sh_idx.last().unwrap().1;
            let prev_sh = sh_idx[sh_idx.len() - 2].1;
            let last_sl = sl_idx.last().unwrap().1;
            let prev_sl = sl_idx[sl_idx.len() - 2].1;
            if last_sh > prev_sh {
                bos_bull = true;
                structure = MarketStructure::Bullish;
            }
            if last_sl < prev_sl {
                bos_bear = true;
                structure = MarketStructure::Bearish;
            }
            if last_sh < prev_sh && self.prev_structure == MarketStructure::Bullish {
                choch_bear = true;
                structure = MarketStructure::Bearish;
            }
            if last_sl > prev_sl && self.prev_structure == MarketStructure::Bearish {
                choch_bull = true;
                structure = MarketStructure::Bullish;
            }
            if structure == MarketStructure::Neutral {
                structure = self.prev_structure;
            }
            self.prev_structure = structure;
        }

        let mut liq_sweep_buy = false;
        let mut liq_sweep_sell = false;
        let last = bars.last().unwrap();
        if !sh_idx.is_empty() {
            let prev_high = sh_idx.last().unwrap().1;
            if last.1 > prev_high && last.3 < prev_high {
                liq_sweep_sell = true;
            }
        }
        if !sl_idx.is_empty() {
            let prev_low = sl_idx.last().unwrap().1;
            if last.2 < prev_low && last.3 > prev_low {
                liq_sweep_buy = true;
            }
        }

        if n >= 3 {
            let a = bars[n - 3];
            let c = bars[n - 1];
            let bull_fvg = c.2 > a.1;
            let bear_fvg = c.1 < a.2;
            if bull_fvg {
                self.fvg_top = Some(c.2);
                self.fvg_bottom = Some(a.1);
                self.fvg_bullish = true;
            } else if bear_fvg {
                self.fvg_top = Some(a.2);
                self.fvg_bottom = Some(c.1);
                self.fvg_bullish = false;
            } else if let (Some(t), Some(b)) = (self.fvg_top, self.fvg_bottom) {
                if close > t || close < b {
                    self.fvg_top = None;
                    self.fvg_bottom = None;
                }
            } else {
                self.fvg_top = None;
                self.fvg_bottom = None;
            }
        }

        if bos_bull && n >= 2 {
            let prev = bars[n - 2];
            if prev.3 < prev.0 {
                self.ob_bullish_high = Some(prev.0);
                self.ob_bullish_low = Some(prev.2);
            }
        }
        if bos_bear && n >= 2 {
            let prev = bars[n - 2];
            if prev.3 > prev.0 {
                self.ob_bearish_high = Some(prev.1);
                self.ob_bearish_low = Some(prev.0);
            }
        }
        if let (Some(_h), Some(l)) = (self.ob_bullish_high, self.ob_bullish_low) {
            if close < l {
                self.ob_bullish_high = None;
                self.ob_bullish_low = None;
            }
        }
        if let (Some(h), Some(_l)) = (self.ob_bearish_high, self.ob_bearish_low) {
            if close > h {
                self.ob_bearish_high = None;
                self.ob_bearish_low = None;
            }
        }

        let pd = if !sh_idx.is_empty() && !sl_idx.is_empty() {
            let sh = sh_idx
                .iter()
                .map(|x| x.1)
                .fold(Decimal::MIN, |a, b| a.max(b));
            let sl = sl_idx
                .iter()
                .map(|x| x.1)
                .fold(Decimal::MAX, |a, b| a.min(b));
            let range = sh - sl;
            if range > Decimal::ZERO {
                ((close - sl) / range * Decimal::from(2) - Decimal::ONE)
                    .to_f64()
                    .unwrap_or(0.0)
            } else {
                0.0
            }
        } else {
            0.0
        };

        Some(SmcOutput {
            structure,
            bos_bullish: bos_bull,
            bos_bearish: bos_bear,
            choch_bullish: choch_bull,
            choch_bearish: choch_bear,
            liquidity_sweep_buy: liq_sweep_buy,
            liquidity_sweep_sell: liq_sweep_sell,
            active_ob_bullish_high: self.ob_bullish_high,
            active_ob_bullish_low: self.ob_bullish_low,
            active_ob_bearish_high: self.ob_bearish_high,
            active_ob_bearish_low: self.ob_bearish_low,
            fvg_top: self.fvg_top,
            fvg_bottom: self.fvg_bottom,
            fvg_bullish: self.fvg_bullish,
            premium_discount: pd,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_none_before_min_bars() {
        let mut smc = SmartMoney::new(50);
        let out = smc.update(dec!(100), dec!(110), dec!(90), dec!(105));
        assert!(out.is_none());
    }

    #[test]
    fn test_produces_output_after_five_bars() {
        let mut smc = SmartMoney::new(50);
        for _ in 0..5 {
            smc.update(dec!(100), dec!(110), dec!(90), dec!(105));
        }
        assert!(smc.update(dec!(100), dec!(110), dec!(90), dec!(105)).is_some());
    }

    #[test]
    fn test_bos_bull_detected_on_higher_high() {
        let mut smc = SmartMoney::new(50);
        // Create a clear swing pattern: higher lows, higher highs
        // Bar pattern: price makes a low at 90, then rallies to 110, pulls back to 95,
        // then rallies to a new higher high at 150.
        smc.update(dec!(100), dec!(105), dec!(95), dec!(100));
        smc.update(dec!(100), dec!(102), dec!(92), dec!(95));
        smc.update(dec!(95), dec!(103), dec!(90), dec!(101));
        smc.update(dec!(101), dec!(110), dec!(101), dec!(108));
        smc.update(dec!(108), dec!(112), dec!(100), dec!(103));
        smc.update(dec!(103), dec!(104), dec!(95), dec!(98));
        smc.update(dec!(98), dec!(105), dec!(94), dec!(103));
        smc.update(dec!(103), dec!(120), dec!(103), dec!(118));
        smc.update(dec!(118), dec!(122), dec!(110), dec!(113));
        smc.update(dec!(113), dec!(114), dec!(105), dec!(108));
        // Final bar: breakout to new high
        let out = smc.update(dec!(108), dec!(150), dec!(108), dec!(145));
        assert!(out.is_some());
        let o = out.unwrap();
        assert!(o.bos_bullish);
    }
}
