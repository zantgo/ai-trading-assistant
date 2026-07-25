use super::traits::{BarInput, Indicator};
use std::collections::VecDeque;

/// Linear Regression Slope: slope of the least-squares regression line fit over
/// the last N closes. Positive = up-trend, negative = down-trend; magnitude =
/// trend strength. Directional (sign of slope).
#[derive(Debug, Clone)]
pub struct LinRegSlope {
    period: usize,
    closes: VecDeque<f64>,
}

impl LinRegSlope {
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
        self.closes.push_back(close);
        if self.closes.len() > self.period {
            self.closes.pop_front();
        }
        if self.closes.len() < self.period {
            return None;
        }
        let n = self.period as f64;
        let sum_x = (0..self.period).map(|i| i as f64).sum::<f64>();
        let sum_y: f64 = self.closes.iter().sum();
        let sum_xy: f64 = self
            .closes
            .iter()
            .enumerate()
            .map(|(i, y)| i as f64 * y)
            .sum();
        let sum_x2: f64 = (0..self.period).map(|i| (i as f64).powi(2)).sum();
        let denom = n * sum_x2 - sum_x * sum_x;
        if denom.abs() < f64::EPSILON {
            return Some(0.0);
        }
        let slope = (n * sum_xy - sum_x * sum_y) / denom;
        Some(slope)
    }
}

impl Indicator for LinRegSlope {
    type Output = Option<f64>;
    fn update(&mut self, bar: &BarInput) -> Self::Output {
        self.update(bar.close)
    }
    fn reset(&mut self) {
        *self = LinRegSlope::new(self.period);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rising_positive_slope() {
        let mut l = LinRegSlope::new(5);
        let mut p = 100.0;
        let mut last = None;
        for _ in 0..8 {
            p += 2.0;
            last = l.update(p);
        }
        assert!(last.unwrap() > 0.0, "rising series → positive slope");
    }

    #[test]
    fn test_falling_negative_slope() {
        let mut l = LinRegSlope::new(5);
        let mut p = 100.0;
        let mut last = None;
        for _ in 0..8 {
            p -= 2.0;
            last = l.update(p);
        }
        assert!(last.unwrap() < 0.0);
    }

    #[test]
    fn test_flat_zero_slope() {
        let mut l = LinRegSlope::new(5);
        let mut last = None;
        for _ in 0..8 {
            last = l.update(100.0);
        }
        assert_eq!(last.unwrap(), 0.0);
    }
}
