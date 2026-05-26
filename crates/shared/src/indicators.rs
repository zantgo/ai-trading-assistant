//! # Technical Indicators Module
//!
//! Implements mathematical indicators: SMA, EMA, ATR, RSI, MACD, ADX, and Squeeze Momentum.
//! It utilizes decimal precision to prevent standard rounding errors.

use rust_decimal::Decimal;
use rust_decimal::prelude::{ToPrimitive, FromPrimitive};

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

/// Average True Range
#[derive(Debug, Clone)]
pub struct Atr {
    prev_close: Option<Decimal>,
    tr_ema: Ema,
}

impl Atr {
    pub fn new(period: usize) -> Self {
        Self {
            prev_close: None,
            tr_ema: Ema::new(period),
        }
    }

    pub fn update(&mut self, high: Decimal, low: Decimal, close: Decimal) -> Option<Decimal> {
        let tr = match self.prev_close {
            None => high - low,
            Some(prev) => {
                let r1 = high - low;
                let r2 = (high - prev).abs();
                let r3 = (low - prev).abs();
                r1.max(r2).max(r3)
            }
        };
        self.prev_close = Some(close);
        Some(self.tr_ema.update(tr))
    }
}

/// Relative Strength Index (using Wilder's Smoothing)
#[derive(Debug, Clone)]
pub struct Rsi {
    period: usize,
    prev_close: Option<Decimal>,
    avg_gain: Option<Decimal>,
    avg_loss: Option<Decimal>,
}

impl Rsi {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            prev_close: None,
            avg_gain: None,
            avg_loss: None,
        }
    }

    pub fn update(&mut self, close: Decimal) -> Option<Decimal> {
        let prev = match self.prev_close {
            None => {
                self.prev_close = Some(close);
                return None;
            }
            Some(p) => p,
        };
        self.prev_close = Some(close);

        let change = close - prev;
        let gain = if change > Decimal::ZERO { change } else { Decimal::ZERO };
        let loss = if change < Decimal::ZERO { change.abs() } else { Decimal::ZERO };

        match (self.avg_gain, self.avg_loss) {
            (Some(ag), Some(al)) => {
                let p_dec = Decimal::from(self.period);
                let p_minus_1 = p_dec - Decimal::ONE;
                
                let next_ag = (ag * p_minus_1 + gain) / p_dec;
                let next_al = (al * p_minus_1 + loss) / p_dec;
                
                self.avg_gain = Some(next_ag);
                self.avg_loss = Some(next_al);

                if next_al == Decimal::ZERO {
                    Some(Decimal::from(100))
                } else {
                    let rs = next_ag / next_al;
                    let rsi = Decimal::from(100) - (Decimal::from(100) / (Decimal::ONE + rs));
                    Some(rsi)
                }
            }
            _ => {
                // Catch-all: If either is None, perform the baseline initial set
                self.avg_gain = Some(gain);
                self.avg_loss = Some(loss);
                None
            }
        }
    }
}

/// Moving Average Convergence Divergence
#[derive(Debug, Clone)]
pub struct Macd {
    fast_ema: Ema,
    slow_ema: Ema,
    signal_ema: Ema,
}

impl Macd {
    pub fn new() -> Self {
        Self {
            fast_ema: Ema::new(12),
            slow_ema: Ema::new(26),
            signal_ema: Ema::new(9),
        }
    }

    pub fn update(&mut self, close: Decimal) -> Option<(Decimal, Decimal, Decimal)> {
        let fast = self.fast_ema.update(close);
        let slow = self.slow_ema.update(close);
        let macd_line = fast - slow;
        let signal_line = self.signal_ema.update(macd_line);
        let histogram = macd_line - signal_line;
        Some((macd_line, signal_line, histogram))
    }
}

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

    pub fn update(&mut self, high: Decimal, low: Decimal, close: Decimal) -> Option<Decimal> {
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
        Some(self.dx_ema.update(dx))
    }
}

/// Squeeze Momentum Indicator (John Carter / LazyBear implementation)
/// Detects compression using Bollinger Bands vs. Keltner Channels,
/// combined with a linear-regression momentum oscillator.
#[derive(Debug, Clone)]
pub struct SqueezeMomentum {
    sma_20: Sma,
    ema_20: Ema,
    atr_20: Atr,
    prices_history: Vec<Decimal>,
    high_history: Vec<Decimal>,
    low_history: Vec<Decimal>,
    val_history: Vec<Decimal>,
}

impl SqueezeMomentum {
    pub fn new() -> Self {
        Self {
            sma_20: Sma::new(20),
            ema_20: Ema::new(20),
            atr_20: Atr::new(20),
            prices_history: Vec::new(),
            high_history: Vec::new(),
            low_history: Vec::new(),
            val_history: Vec::new(),
        }
    }

    pub fn update(&mut self, high: Decimal, low: Decimal, close: Decimal) -> Option<(bool, Decimal)> {
        self.prices_history.push(close);
        self.high_history.push(high);
        self.low_history.push(low);
        
        if self.prices_history.len() > 20 {
            self.prices_history.remove(0);
            self.high_history.remove(0);
            self.low_history.remove(0);
        }

        let sma = self.sma_20.update(close)?;
        let ema = self.ema_20.update(close);
        let atr = self.atr_20.update(high, low, close)?;

        if self.prices_history.len() < 20 {
            return None;
        }

        // Calculate standard deviation of historical closes
        let std_dev = {
            let sum_sq: f64 = self.prices_history.iter()
                .map(|&p| {
                    let diff = (p - sma).to_f64().unwrap_or(0.0);
                    diff * diff
                })
                .sum();
            let variance = sum_sq / 20.0;
            Decimal::from_f64(variance.sqrt()).unwrap_or(Decimal::ZERO)
        };

        let bb_upper = sma + std_dev * Decimal::from(2);
        let bb_lower = sma - std_dev * Decimal::from(2);

        let kc_upper = ema + atr * Decimal::new(15, 1); // 1.5 multiplier
        let kc_lower = ema - atr * Decimal::new(15, 1);

        // Squeeze active: Bollinger Bands are compressed inside Keltner Channels
        let squeeze_on = bb_lower > kc_lower && bb_upper < kc_upper;

        let highest_high = self.high_history.iter().max().copied().unwrap_or(high);
        let lowest_low = self.low_history.iter().min().copied().unwrap_or(low);

        let avg = ((highest_high + lowest_low) / Decimal::from(2) + ema) / Decimal::from(2);
        let val = close - avg;

        self.val_history.push(val);
        if self.val_history.len() > 20 {
            self.val_history.remove(0);
        }

        if self.val_history.len() == 20 {
            // Linear regression of the last 20 'val' points (x: 0..19)
            let n = 20.0;
            let sum_x: f64 = 190.0; // Sum of 0..19
            let sum_x_sq: f64 = 2470.0; // Sum of x^2 for 0..19

            let mut sum_y = 0.0;
            let mut sum_xy = 0.0;

            for (x, &y_dec) in self.val_history.iter().enumerate() {
                let y = y_dec.to_f64().unwrap_or(0.0);
                sum_y += y;
                sum_xy += (x as f64) * y;
            }

            let denominator = n * sum_x_sq - (sum_x * sum_x);
            let b = if denominator != 0.0 {
                (n * sum_xy - sum_x * sum_y) / denominator
            } else {
                0.0
            };

            let a = (sum_y - b * sum_x) / n;
            let momentum_val_f64 = a + b * 19.0;
            let momentum_val = Decimal::from_f64(momentum_val_f64).unwrap_or(Decimal::ZERO);

            Some((squeeze_on, momentum_val))
        } else {
            None
        }
    }
}

// ... (keep all other structures like Ema, Sma, Atr, Rsi, Macd, Adx, SqueezeMomentum exactly as they are)

/// Bollinger Bands Indicator (20 SMA +/- 2 Standard Deviations)
#[derive(Debug, Clone)]
pub struct BollingerBands {
    sma_20: Sma,
    prices_history: Vec<Decimal>,
}

impl BollingerBands {
    pub fn new() -> Self {
        Self {
            sma_20: Sma::new(20),
            prices_history: Vec::new(),
        }
    }

    /// Update with a closed price, returning (upper, middle, lower)
    pub fn update(&mut self, close: Decimal) -> Option<(Decimal, Decimal, Decimal)> {
        self.prices_history.push(close);
        if self.prices_history.len() > 20 {
            self.prices_history.remove(0);
        }

        let sma = self.sma_20.update(close)?;

        if self.prices_history.len() < 20 {
            return None;
        }

        // Standard Deviation
        let std_dev = {
            let sum_sq: f64 = self.prices_history.iter()
                .map(|&p| {
                    let diff = (p - sma).to_f64().unwrap_or(0.0);
                    diff * diff
                })
                .sum();
            let variance = sum_sq / 20.0;
            Decimal::from_f64(variance.sqrt()).unwrap_or(Decimal::ZERO)
        };

        let upper = sma + std_dev * Decimal::from(2);
        let lower = sma - std_dev * Decimal::from(2);

        Some((upper, sma, lower))
    }
}