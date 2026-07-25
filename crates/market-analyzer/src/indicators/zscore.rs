use super::traits::{BarInput, Indicator};
use std::collections::VecDeque;

/// Z-Score: number of standard deviations the close is from its N-bar mean.
/// Mean-reversion oriented — extreme positive = statistically stretched high.
#[derive(Debug, Clone)]
pub struct ZScore {
    period: usize,
    closes: VecDeque<f64>,
}

impl ZScore {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            closes: VecDeque::with_capacity(period),
        }
    }

    pub fn update(&mut self, close: f64) -> Option<f64> {
        if self.period < 2 {
            return None;
        }
        let c = close;
        self.closes.push_back(c);
        if self.closes.len() > self.period {
            self.closes.pop_front();
        }
        if self.closes.len() < self.period {
            return None;
        }
        let n = self.closes.len() as f64;
        let mean = self.closes.iter().sum::<f64>() / n;
        let var = self.closes.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n;
        let std = var.sqrt();
        if std < f64::EPSILON {
            return Some(0.0);
        }
        Some((c - mean) / std)
    }
}

impl Indicator for ZScore {
    type Output = Option<f64>;
    fn update(&mut self, bar: &BarInput) -> Self::Output {
        self.update(bar.close)
    }
    fn reset(&mut self) {
        *self = ZScore::new(self.period);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spike_positive_z() {
        let mut z = ZScore::new(10);
        for _ in 0..10 {
            z.update(100.0);
        }
        let v = z.update(120.0).unwrap();
        assert!(v > 1.0, "upward spike → positive z-score, got {}", v);
    }

    #[test]
    fn test_flat_zero() {
        let mut z = ZScore::new(5);
        let mut last = None;
        for _ in 0..8 {
            last = z.update(100.0);
        }
        assert_eq!(last.unwrap(), 0.0);
    }

    #[test]
    fn test_none_before_period() {
        let mut z = ZScore::new(5);
        assert!(z.update(100.0).is_none());
    }
}
