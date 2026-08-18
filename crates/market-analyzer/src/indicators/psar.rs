use rust_decimal::Decimal;

/// Parabolic SAR — trend-following trailing-stop overlay.
///
/// The SAR dot sits below price in an uptrend and above price in a downtrend.
/// When price crosses the SAR, the trend flips and the dot jumps to the
/// opposite side. The Acceleration Factor (AF) starts at `af_step` and
/// increases by `af_step` up to `af_max` with every new extreme price.
#[derive(Debug, Clone)]
pub struct ParabolicSar {
    af_step: Decimal,
    af_max: Decimal,
    sar: Decimal,
    ep: Decimal,
    af: Decimal,
    direction: i8,
    initialized: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct PsarOutput {
    pub sar: Decimal,
    pub direction: i8,
    /// True on the bar where the trend direction flipped.
    pub flipped: bool,
}

impl ParabolicSar {
    pub fn new(af_step: f64, af_max: f64) -> Self {
        Self {
            af_step: Decimal::from_f64_retain(af_step)
                .unwrap_or(Decimal::from_f64_retain(0.02).unwrap()),
            af_max: Decimal::from_f64_retain(af_max)
                .unwrap_or(Decimal::from_f64_retain(0.20).unwrap()),
            sar: Decimal::ZERO,
            ep: Decimal::ZERO,
            af: Decimal::from_f64_retain(0.02).unwrap(),
            direction: 1,
            initialized: false,
        }
    }

    pub fn update(&mut self, high: f64, low: f64) -> Option<PsarOutput> {
        let high = Decimal::from_f64_retain(high).unwrap_or(Decimal::ZERO);
        let low = Decimal::from_f64_retain(low).unwrap_or(Decimal::ZERO);
        if !self.initialized {
            // Seed: first bar's high/low determine direction.
            self.sar = low;
            self.ep = high;
            self.direction = 1;
            self.af = self.af_step;
            self.initialized = true;
            return Some(PsarOutput {
                sar: self.sar,
                direction: self.direction,
                flipped: false,
            });
        }

        let _prev_dir = self.direction;
        let mut flipped = false;

        if self.direction > 0 {
            // Uptrend: SAR must stay below price.
            let candidate = self.sar + self.af * (self.ep - self.sar);
            self.sar = candidate.min(low);
            if self.sar >= low {
                // Trend reverses to downside.
                self.direction = -1;
                flipped = true;
                // AUDIT-M8: the standard (Wilder/TradingView) reversal
                // anchors the new SAR at the prior trend's EP (the highest
                // high of the completed uptrend), not the current bar's
                // high — the old code could place the new SAR far above
                // the real extreme on sharp reversals, distorting the
                // trailing-stop geometry of the new trend.
                self.sar = self.ep;
                self.ep = low;
                self.af = self.af_step;
            } else {
                if high > self.ep {
                    self.ep = high;
                    self.af = (self.af + self.af_step).min(self.af_max);
                }
            }
        } else {
            // Downtrend: SAR must stay above price.
            let candidate = self.sar + self.af * (self.ep - self.sar);
            self.sar = candidate.max(high);
            if self.sar <= high {
                // Trend reverses to upside.
                self.direction = 1;
                flipped = true;
                // AUDIT-M8: anchor at the prior trend's EP (lowest low).
                self.sar = self.ep;
                self.ep = high;
                self.af = self.af_step;
            } else {
                if low < self.ep {
                    self.ep = low;
                    self.af = (self.af + self.af_step).min(self.af_max);
                }
            }
        }

        Some(PsarOutput {
            sar: self.sar,
            direction: self.direction,
            flipped,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seeds_on_first_bar() {
        let mut psar = ParabolicSar::new(0.02, 0.20);
        let out = psar.update(110.0, 90.0).unwrap();
        assert_eq!(out.sar, Decimal::from_f64_retain(90.0).unwrap());
        assert_eq!(out.direction, 1);
        assert!(!out.flipped);
    }

    #[test]
    fn test_sar_rises_in_uptrend() {
        let mut psar = ParabolicSar::new(0.02, 0.20);
        psar.update(110.0, 90.0);
        let prev = psar.update(112.0, 95.0).unwrap().sar;
        let curr = psar.update(114.0, 97.0).unwrap().sar;
        assert!(curr > prev, "SAR should rise in an uptrend");
    }

    #[test]
    fn test_flips_on_reversal() {
        let mut psar = ParabolicSar::new(0.02, 0.20);
        psar.update(110.0, 90.0);
        // Strong downtrend — price breaks below SAR.
        psar.update(111.0, 99.0);
        let out = psar.update(101.0, 80.0).unwrap();
        assert_eq!(out.direction, -1);
        assert!(out.flipped);
    }

    #[test]
    fn test_reversal_anchors_at_prior_ep_not_current_bar_extreme() {
        // AUDIT-M8: after an uptrend with EP = 114.0, a reversal bar that
        // spiked to 120.0 must anchor the new SAR at the prior EP (114.0),
        // not at the current bar's high (120.0).
        let mut psar = ParabolicSar::new(0.02, 0.20);
        psar.update(110.0, 90.0);
        psar.update(112.0, 95.0);
        psar.update(114.0, 97.0);
        let out = psar.update(120.0, 85.0).unwrap();
        assert_eq!(out.direction, -1);
        assert!(out.flipped);
        assert_eq!(
            out.sar,
            Decimal::from_f64_retain(114.0).unwrap(),
            "new SAR must anchor at the prior trend's EP (highest high)"
        );
    }

    #[test]
    fn test_reversal_anchors_at_prior_ep_for_downside_to_upside() {
        // Construct a downtrend state directly (EP = 80.0 = lowest low of
        // the completed downtrend), then a reversal bar at the same high
        // as the SAR.
        let mut psar = ParabolicSar::new(0.02, 0.20);
        psar.initialized = true;
        psar.direction = -1;
        psar.sar = Decimal::from_f64_retain(95.0).unwrap();
        psar.ep = Decimal::from_f64_retain(80.0).unwrap();
        psar.af = Decimal::from_f64_retain(0.04).unwrap();
        let out = psar.update(95.0, 70.0).unwrap();
        assert_eq!(out.direction, 1);
        assert!(out.flipped);
        assert_eq!(
            out.sar,
            Decimal::from_f64_retain(80.0).unwrap(),
            "new SAR must anchor at the prior trend's EP (lowest low)"
        );
    }

    #[test]
    fn test_af_accelerates_in_trend() {
        let mut psar = ParabolicSar::new(0.02, 0.20);
        psar.update(110.0, 90.0);
        // Strong rally → EP keeps extending, AF keeps stepping up.
        for i in 1..5i64 {
            let h = 110 + i * 3;
            let l = 90 + i * 3;
            psar.update(h as f64, l as f64);
        }
        assert!(psar.af > Decimal::from_f64_retain(0.02).unwrap());
    }
}
