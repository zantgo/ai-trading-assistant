use rust_decimal::Decimal;

/// Pivot-point calculation method. Only `Classic` is implemented in V1; the
/// remaining variants are reserved so the config surface and enum are forward
/// compatible without a breaking change when they are added.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PivotMethod {
    #[default]
    Classic,
    Fibonacci,
    Camarilla,
    Woodie,
}

impl PivotMethod {
    pub fn from_str_lenient(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "fibonacci" | "fib" => PivotMethod::Fibonacci,
            "camarilla" => PivotMethod::Camarilla,
            "woodie" => PivotMethod::Woodie,
            _ => PivotMethod::Classic,
        }
    }
}

/// The seven static levels of a session pivot: the central pivot plus three
/// resistances and three supports.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PivotLevels {
    pub pivot: Decimal,
    pub r1: Decimal,
    pub r2: Decimal,
    pub r3: Decimal,
    pub s1: Decimal,
    pub s2: Decimal,
    pub s3: Decimal,
}

/// Session-based Pivot Points.
///
/// Levels are computed once at the start of each new session (UTC day) from the
/// *previous* completed session's High/Low/Close and remain constant until the
/// next session begins. The calculator accumulates the current session's H/L/C
/// as candles arrive and rolls those into the previous-session registers on a
/// day boundary.
#[derive(Debug, Clone)]
pub struct PivotPoints {
    method: PivotMethod,
    // Current (in-progress) session accumulation.
    cur_high: Option<Decimal>,
    cur_low: Option<Decimal>,
    cur_close: Option<Decimal>,
    cur_day: Option<u64>,
    // Levels published for the active session (from the prior completed session).
    levels: Option<PivotLevels>,
}

impl PivotPoints {
    pub fn new(method: PivotMethod) -> Self {
        Self {
            method,
            cur_high: None,
            cur_low: None,
            cur_close: None,
            cur_day: None,
            levels: None,
        }
    }

    /// Feed a completed candle. `day_index` is the UTC-day bucket
    /// (`candle_close_secs / 86400`). On a day rollover the previous session's
    /// accumulated H/L/C is finalized into a fresh set of pivot levels; the new
    /// session's accumulation then begins with this candle.
    ///
    /// Returns the currently active `PivotLevels` (None until the first full
    /// prior session has completed).
    pub fn update(
        &mut self,
        high: f64,
        low: f64,
        close: f64,
        day_index: u64,
    ) -> Option<PivotLevels> {
        let high = Decimal::from_f64_retain(high).unwrap_or(Decimal::ZERO);
        let low = Decimal::from_f64_retain(low).unwrap_or(Decimal::ZERO);
        let close = Decimal::from_f64_retain(close).unwrap_or(Decimal::ZERO);
        match self.cur_day {
            Some(d) if d == day_index => {
                // Same session: extend the accumulation.
                self.cur_high = Some(self.cur_high.map_or(high, |h| h.max(high)));
                self.cur_low = Some(self.cur_low.map_or(low, |l| l.min(low)));
                self.cur_close = Some(close);
            }
            Some(_) => {
                // New session: finalize the prior session into published levels.
                if let (Some(h), Some(l), Some(c)) = (self.cur_high, self.cur_low, self.cur_close) {
                    self.levels = Some(Self::compute(self.method, h, l, c));
                }
                // Begin the new session with this candle.
                self.cur_high = Some(high);
                self.cur_low = Some(low);
                self.cur_close = Some(close);
                self.cur_day = Some(day_index);
            }
            None => {
                // First candle ever: seed the current session.
                self.cur_high = Some(high);
                self.cur_low = Some(low);
                self.cur_close = Some(close);
                self.cur_day = Some(day_index);
            }
        }
        self.levels
    }

    /// The active session's levels, if a prior session has been finalized.
    pub fn levels(&self) -> Option<PivotLevels> {
        self.levels
    }

    /// Compute the seven levels from a session's High/Low/Close.
    fn compute(method: PivotMethod, high: Decimal, low: Decimal, close: Decimal) -> PivotLevels {
        match method {
            // Only Classic is implemented in V1; other methods fall back to it.
            _ => Self::classic(high, low, close),
        }
    }

    fn classic(high: Decimal, low: Decimal, close: Decimal) -> PivotLevels {
        let three = Decimal::from(3);
        let two = Decimal::from(2);
        let p = (high + low + close) / three;
        let range = high - low;
        PivotLevels {
            pivot: p,
            r1: two * p - low,
            s1: two * p - high,
            r2: p + range,
            s2: p - range,
            r3: high + two * (p - low),
            s3: low - two * (high - p),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_none_until_first_session_completes() {
        let mut pp = PivotPoints::new(PivotMethod::Classic);
        // All within day 0 → no prior session yet.
        assert!(pp.update(110.0, 90.0, 100.0, 0).is_none());
        assert!(pp.update(112.0, 95.0, 105.0, 0).is_none());
        // Rollover to day 1 finalizes day 0 → levels published.
        assert!(pp.update(108.0, 100.0, 104.0, 1).is_some());
    }

    #[test]
    fn test_classic_level_ordering() {
        let mut pp = PivotPoints::new(PivotMethod::Classic);
        pp.update(110.0, 90.0, 100.0, 0);
        let lv = pp.update(105.0, 101.0, 104.0, 1).unwrap();
        // S3 < S2 < S1 < P < R1 < R2 < R3
        assert!(lv.s3 < lv.s2);
        assert!(lv.s2 < lv.s1);
        assert!(lv.s1 < lv.pivot);
        assert!(lv.pivot < lv.r1);
        assert!(lv.r1 < lv.r2);
        assert!(lv.r2 < lv.r3);
    }

    #[test]
    fn test_classic_pivot_formula() {
        let mut pp = PivotPoints::new(PivotMethod::Classic);
        pp.update(120.0, 90.0, 105.0, 0);
        let lv = pp.update(100.0, 99.0, 100.0, 1).unwrap();
        // P = (120 + 90 + 105) / 3 = 105
        assert_eq!(lv.pivot, dec!(105));
        // R1 = 2P - Low = 210 - 90 = 120
        assert_eq!(lv.r1, dec!(120));
        // S1 = 2P - High = 210 - 120 = 90
        assert_eq!(lv.s1, dec!(90));
        // R2 = P + (H-L) = 105 + 30 = 135
        assert_eq!(lv.r2, dec!(135));
        // S2 = P - (H-L) = 105 - 30 = 75
        assert_eq!(lv.s2, dec!(75));
    }

    #[test]
    fn test_levels_constant_within_session() {
        let mut pp = PivotPoints::new(PivotMethod::Classic);
        pp.update(110.0, 90.0, 100.0, 0);
        let a = pp.update(105.0, 101.0, 104.0, 1).unwrap();
        // Further candles within day 1 must not change published levels.
        let b = pp.update(120.0, 95.0, 118.0, 1).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn test_method_from_str() {
        assert_eq!(
            PivotMethod::from_str_lenient("classic"),
            PivotMethod::Classic
        );
        assert_eq!(
            PivotMethod::from_str_lenient("Fibonacci"),
            PivotMethod::Fibonacci
        );
        assert_eq!(
            PivotMethod::from_str_lenient("garbage"),
            PivotMethod::Classic
        );
    }
}
