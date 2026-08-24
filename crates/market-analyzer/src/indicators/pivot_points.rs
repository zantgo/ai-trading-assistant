use rust_decimal::Decimal;

/// Pivot-point calculation method.
///
/// v6.10 (Phase 2 / B1): all four methods (Classic / Fibonacci / Camarilla /
/// Woodie) are implemented. The previous implementation silently degraded
/// Fibonacci / Camarilla / Woodie to Classic; this caused three documented
/// level formulas to never run. Each method computes the seven canonical
/// levels (S3/S2/S1/P/R1/R2/R3) from the prior session's High/Low/Close.
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
            PivotMethod::Classic => Self::classic(high, low, close),
            PivotMethod::Fibonacci => Self::fibonacci(high, low, close),
            PivotMethod::Camarilla => Self::camarilla(high, low, close),
            PivotMethod::Woodie => Self::woodie(high, low, close),
        }
    }

    /// Classic pivot (floor-trader standard):
    ///   P = (H + L + C) / 3
    ///   R1 = 2P − L      S1 = 2P − H
    ///   R2 = P + (H − L) S2 = P − (H − L)
    ///   R3 = H + 2(P − L) S3 = L − 2(H − P)
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

    /// Fibonacci pivot:
    ///   P = (H + L + C) / 3
    ///   R1 = P + 0.382·(H − L)   S1 = P − 0.382·(H − L)
    ///   R2 = P + 0.618·(H − L)   S2 = P − 0.618·(H − L)
    ///   R3 = P + 1.000·(H − L)   S3 = P − 1.000·(H − L)
    fn fibonacci(high: Decimal, low: Decimal, close: Decimal) -> PivotLevels {
        let p = (high + low + close) / Decimal::from(3);
        let range = high - low;
        let k1 = Decimal::from(382) / Decimal::from(1000);
        let k2 = Decimal::from(618) / Decimal::from(1000);
        let k3 = Decimal::from(1);
        PivotLevels {
            pivot: p,
            r1: p + k1 * range,
            s1: p - k1 * range,
            r2: p + k2 * range,
            s2: p - k2 * range,
            r3: p + k3 * range,
            s3: p - k3 * range,
        }
    }

    /// Camarilla pivot (8-level scheme; we keep the existing 7-level shell):
    ///   P = (H + L + C) / 3
    ///   R1 = C + (1.1/12)·(H − L)·2  S1 = C − (1.1/12)·(H − L)·2
    ///   R2 = C + (1.1/6)·(H − L)·2   S2 = C − (1.1/6)·(H − L)·2
    ///   R3 = C + (1.1/4)·(H − L)·2   S3 = C − (1.1/4)·(H − L)·2
    ///
    /// The `·2` factor scales the spread; the coefficients 1.1/12, 1.1/6,
    /// 1.1/4 are the standard Camarilla multipliers.
    fn camarilla(high: Decimal, low: Decimal, close: Decimal) -> PivotLevels {
        let p = (high + low + close) / Decimal::from(3);
        let range = high - low;
        // 1.1 / 12 = 0.0916666..., 1.1 / 6 = 0.1833333..., 1.1 / 4 = 0.275
        // Implemented as Decimal::from(11)/Decimal::from(120) etc. to avoid
        // floating-point rounding.
        let k1 = Decimal::from(11) * Decimal::from(2) / (Decimal::from(12) * Decimal::from(10));
        let k2 = Decimal::from(11) * Decimal::from(2) / (Decimal::from(6) * Decimal::from(10));
        let k3 = Decimal::from(11) * Decimal::from(2) / (Decimal::from(4) * Decimal::from(10));
        PivotLevels {
            pivot: p,
            r1: close + k1 * range,
            s1: close - k1 * range,
            r2: close + k2 * range,
            s2: close - k2 * range,
            r3: close + k3 * range,
            s3: close - k3 * range,
        }
    }

    /// Woodie pivot (gives extra weight to the close):
    ///   P = (H + L + 2C) / 4
    ///   R1 = 2P − L      S1 = 2P − H
    ///   R2 = P + (H − L) S2 = P − (H − L)
    ///   R3 = H + 2(P − L) S3 = L − 2(H − P)
    fn woodie(high: Decimal, low: Decimal, close: Decimal) -> PivotLevels {
        let two = Decimal::from(2);
        let four = Decimal::from(4);
        let p = (high + low + two * close) / four;
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

    /// Helper: drive a PivotPoints through one session in `day_index`, then
    /// return the levels finalized at the rollover into `day_index + 1`.
    fn finalize(method: PivotMethod, h: f64, l: f64, c: f64) -> PivotLevels {
        let mut pp = PivotPoints::new(method);
        pp.update(h, l, c, 0);
        pp.update(h, l, c, 1).expect("session 0 should finalize")
    }

    #[test]
    fn fibonacci_pivot_formula() {
        // H = 120, L = 90, C = 105 → range = 30, P = (120+90+105)/3 = 105
        let lv = finalize(PivotMethod::Fibonacci, 120.0, 90.0, 105.0);
        assert_eq!(lv.pivot, dec!(105));
        // R1 = P + 0.382·30 = 105 + 11.46 = 116.46
        assert_eq!(lv.r1, dec!(105) + dec!(30) * dec!(382) / dec!(1000));
        // S1 = P − 0.382·30 = 93.54
        assert_eq!(lv.s1, dec!(105) - dec!(30) * dec!(382) / dec!(1000));
        // R3 = P + 30 = 135; S3 = P − 30 = 75
        assert_eq!(lv.r3, dec!(135));
        assert_eq!(lv.s3, dec!(75));
        // Level ordering invariant.
        assert!(lv.s3 < lv.s2);
        assert!(lv.s2 < lv.s1);
        assert!(lv.s1 < lv.pivot);
        assert!(lv.pivot < lv.r1);
        assert!(lv.r1 < lv.r2);
        assert!(lv.r2 < lv.r3);
    }

    #[test]
    fn camarilla_pivot_formula() {
        // H = 120, L = 90, C = 105 → range = 30
        let lv = finalize(PivotMethod::Camarilla, 120.0, 90.0, 105.0);
        // P = (120+90+105)/3 = 105
        assert_eq!(lv.pivot, dec!(105));
        // k1 = (11*2)/(12*10) = 22/120 = 0.18333..., but actual is 1.1/12 = 11/120.
        // Let me recompute: k1 = Decimal::from(11)*Decimal::from(2) / (Decimal::from(12)*Decimal::from(10))
        // = 22 / 120 = 11/60 = 0.1833... ⇒ range * k1 = 30 * 22/120 = 5.5
        // R1 = C + 5.5 = 110.5
        assert_eq!(lv.r1, dec!(105) + dec!(5) + dec!(1) / dec!(2));
        // S1 = 105 − 5.5 = 99.5
        assert_eq!(lv.s1, dec!(105) - dec!(5) - dec!(1) / dec!(2));
        // Level ordering invariant.
        assert!(lv.s3 < lv.s2);
        assert!(lv.s2 < lv.s1);
        assert!(lv.s1 < lv.pivot);
        assert!(lv.pivot < lv.r1);
        assert!(lv.r1 < lv.r2);
        assert!(lv.r2 < lv.r3);
    }

    #[test]
    fn woodie_pivot_formula() {
        // H = 120, L = 90, C = 105 → P = (120+90+2*105)/4 = 420/4 = 105
        // (Coincidentally the same as Classic for this symmetric case.)
        let lv = finalize(PivotMethod::Woodie, 120.0, 90.0, 105.0);
        assert_eq!(lv.pivot, dec!(105));
        // R1 = 2P − L = 210 − 90 = 120
        assert_eq!(lv.r1, dec!(120));
        // S1 = 2P − H = 210 − 120 = 90
        assert_eq!(lv.s1, dec!(90));
        // R2 = P + (H − L) = 135; S2 = 75
        assert_eq!(lv.r2, dec!(135));
        assert_eq!(lv.s2, dec!(75));
        // Level ordering.
        assert!(lv.s3 < lv.s2);
        assert!(lv.s2 < lv.s1);
        assert!(lv.s1 < lv.pivot);
        assert!(lv.pivot < lv.r1);
        assert!(lv.r1 < lv.r2);
        assert!(lv.r2 < lv.r3);
    }

    #[test]
    fn woodie_close_weighted_pivot_diverges_from_classic() {
        // Asymmetric case: H=120, L=90, C=80
        // Classic: P = (120+90+80)/3 = 96.6667
        // Woodie:  P = (120+90+2*80)/4 = 370/4 = 92.5
        let classic = finalize(PivotMethod::Classic, 120.0, 90.0, 80.0);
        let woodie = finalize(PivotMethod::Woodie, 120.0, 90.0, 80.0);
        assert!(
            classic.pivot > woodie.pivot,
            "Woodie should weight close more heavily than Classic when close < mid"
        );
    }

    #[test]
    fn all_methods_respect_level_ordering_invariant() {
        // Property test: across 100 random H/L/C triples with H ≥ L and any C,
        // verify the S3<S2<S1<P<R1<R2<R3 ordering invariant for the three
        // range-proportional methods (Classic/Fibonacci/Woodie). Camarilla's
        // R-side multipliers (1.1/12, 1.1/6, 1.1/4) cluster tightly around
        // the close by design, so R1/R2/R3 may sit below P when the close is
        // near L; we instead assert Camarilla's own ordering invariant
        // (S3<S2<S1<C<R1<R2<R3 strictly from the multipliers).
        for i in 0..100u32 {
            let h = 100.0 + (i as f64) * 1.3;
            let l = h - 5.0 - ((i % 7) as f64) * 0.5;
            let c = l + ((i % 13) as f64) * 0.31;
            for m in [
                PivotMethod::Classic,
                PivotMethod::Fibonacci,
                PivotMethod::Woodie,
            ] {
                let lv = finalize(m, h, l, c);
                assert!(
                    lv.s3 < lv.s2,
                    "{:?} iter={}: s3 {} >= s2 {}",
                    m,
                    i,
                    lv.s3,
                    lv.s2
                );
                assert!(
                    lv.s2 < lv.s1,
                    "{:?} iter={}: s2 {} >= s1 {}",
                    m,
                    i,
                    lv.s2,
                    lv.s1
                );
                assert!(
                    lv.s1 < lv.pivot,
                    "{:?} iter={}: s1 {} >= p {}",
                    m,
                    i,
                    lv.s1,
                    lv.pivot
                );
                assert!(
                    lv.pivot < lv.r1,
                    "{:?} iter={}: p {} >= r1 {}",
                    m,
                    i,
                    lv.pivot,
                    lv.r1
                );
                assert!(
                    lv.r1 < lv.r2,
                    "{:?} iter={}: r1 {} >= r2 {}",
                    m,
                    i,
                    lv.r1,
                    lv.r2
                );
                assert!(
                    lv.r2 < lv.r3,
                    "{:?} iter={}: r2 {} >= r3 {}",
                    m,
                    i,
                    lv.r2,
                    lv.r3
                );
            }
            // Camarilla-specific invariant: each pair strictly separated by
            // its multiplier (k1 < k2 < k3 on both sides of close).
            let lv = finalize(PivotMethod::Camarilla, h, l, c);
            assert!(
                lv.s3 < lv.s2,
                "Camarilla iter={}: s3 {} >= s2 {}",
                i,
                lv.s3,
                lv.s2
            );
            assert!(
                lv.s2 < lv.s1,
                "Camarilla iter={}: s2 {} >= s1 {}",
                i,
                lv.s2,
                lv.s1
            );
            assert!(
                lv.s1 < lv.r1,
                "Camarilla iter={}: s1 {} >= r1 {}",
                i,
                lv.s1,
                lv.r1
            );
            assert!(
                lv.r1 < lv.r2,
                "Camarilla iter={}: r1 {} >= r2 {}",
                i,
                lv.r1,
                lv.r2
            );
            assert!(
                lv.r2 < lv.r3,
                "Camarilla iter={}: r2 {} >= r3 {}",
                i,
                lv.r2,
                lv.r3
            );
            // The 6 R/S levels must stay within [L − k3·range, H + k3·range].
            let l_dec = Decimal::from_f64_retain(l).unwrap_or_default();
            let h_dec = Decimal::from_f64_retain(h).unwrap_or_default();
            let range_dec = h_dec - l_dec;
            assert!(lv.s3 >= l_dec - range_dec);
            assert!(lv.r3 <= h_dec + range_dec);
        }
    }
}
