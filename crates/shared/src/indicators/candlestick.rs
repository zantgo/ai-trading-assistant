use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use std::collections::VecDeque;

/// The 29 supported candlestick patterns across four families.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandlestickPattern {
    None,
    // ── Single candle (11) ──
    Doji,
    LongLeggedDoji,
    DragonflyDoji,
    GravestoneDoji,
    Hammer,
    InvertedHammer,
    HangingMan,
    ShootingStar,
    BullishMarubozu,
    BearishMarubozu,
    SpinningTop,
    // ── Two candle (8) ──
    BullishEngulfing,
    BearishEngulfing,
    PiercingLine,
    DarkCloudCover,
    TweezerBottom,
    TweezerTop,
    BullishHarami,
    BearishHarami,
    // ── Three candle (8) ──
    MorningStar,
    EveningStar,
    ThreeWhiteSoldiers,
    ThreeBlackCrows,
    ThreeInsideUp,
    ThreeInsideDown,
    ThreeOutsideUp,
    ThreeOutsideDown,
    // ── Continuation (2) ──
    RisingThreeMethods,
    FallingThreeMethods,
}

impl CandlestickPattern {
    pub fn name(self) -> &'static str {
        use CandlestickPattern::*;
        match self {
            None => "NONE",
            Doji => "DOJI",
            LongLeggedDoji => "LONG_LEGGED_DOJI",
            DragonflyDoji => "DRAGONFLY_DOJI",
            GravestoneDoji => "GRAVESTONE_DOJI",
            Hammer => "HAMMER",
            InvertedHammer => "INVERTED_HAMMER",
            HangingMan => "HANGING_MAN",
            ShootingStar => "SHOOTING_STAR",
            BullishMarubozu => "BULLISH_MARUBOZU",
            BearishMarubozu => "BEARISH_MARUBOZU",
            SpinningTop => "SPINNING_TOP",
            BullishEngulfing => "BULLISH_ENGULFING",
            BearishEngulfing => "BEARISH_ENGULFING",
            PiercingLine => "PIERCING_LINE",
            DarkCloudCover => "DARK_CLOUD_COVER",
            TweezerBottom => "TWEEZER_BOTTOM",
            TweezerTop => "TWEEZER_TOP",
            BullishHarami => "BULLISH_HARAMI",
            BearishHarami => "BEARISH_HARAMI",
            MorningStar => "MORNING_STAR",
            EveningStar => "EVENING_STAR",
            ThreeWhiteSoldiers => "THREE_WHITE_SOLDIERS",
            ThreeBlackCrows => "THREE_BLACK_CROWS",
            ThreeInsideUp => "THREE_INSIDE_UP",
            ThreeInsideDown => "THREE_INSIDE_DOWN",
            ThreeOutsideUp => "THREE_OUTSIDE_UP",
            ThreeOutsideDown => "THREE_OUTSIDE_DOWN",
            RisingThreeMethods => "RISING_THREE_METHODS",
            FallingThreeMethods => "FALLING_THREE_METHODS",
        }
    }

    /// Directional bias: +1 bullish, -1 bearish, 0 neutral/indecision.
    pub fn direction(self) -> i8 {
        use CandlestickPattern::*;
        match self {
            Hammer | InvertedHammer | BullishMarubozu | BullishEngulfing | PiercingLine
            | TweezerBottom | BullishHarami | MorningStar | ThreeWhiteSoldiers | ThreeInsideUp
            | ThreeOutsideUp | RisingThreeMethods | DragonflyDoji => 1,
            HangingMan | ShootingStar | BearishMarubozu | BearishEngulfing | DarkCloudCover
            | TweezerTop | BearishHarami | EveningStar | ThreeBlackCrows | ThreeInsideDown
            | ThreeOutsideDown | FallingThreeMethods | GravestoneDoji => -1,
            _ => 0,
        }
    }

    /// Family category for filtering / telemetry.
    pub fn category(self) -> &'static str {
        use CandlestickPattern::*;
        match self {
            Doji | LongLeggedDoji | DragonflyDoji | GravestoneDoji | Hammer | InvertedHammer
            | HangingMan | ShootingStar | BullishMarubozu | BearishMarubozu | SpinningTop => "single",
            BullishEngulfing | BearishEngulfing | PiercingLine | DarkCloudCover | TweezerBottom
            | TweezerTop | BullishHarami | BearishHarami => "two",
            MorningStar | EveningStar | ThreeWhiteSoldiers | ThreeBlackCrows | ThreeInsideUp
            | ThreeInsideDown | ThreeOutsideUp | ThreeOutsideDown => "three",
            RisingThreeMethods | FallingThreeMethods => "continuation",
            None => "none",
        }
    }
}

/// Tunable geometric thresholds (fractions of range / body).
#[derive(Debug, Clone, Copy)]
pub struct CandlestickConfig {
    /// Body ≤ this fraction of the candle range → doji-class.
    pub doji_body_max: f64,
    /// Long wick ≥ this multiple of the body (hammer/shooting-star tails).
    pub long_wick_body_mult: f64,
    /// Opposite wick ≤ this fraction of range for hammer/shooting-star.
    pub small_wick_max: f64,
    /// Marubozu: wick ≤ this fraction of range on both ends.
    pub marubozu_wick_max: f64,
    /// Spinning-top body ≤ this fraction of range (but above doji).
    pub spinning_body_max: f64,
    /// Tweezer high/low equality tolerance as fraction of price.
    pub tweezer_eq_tol: f64,
}

impl Default for CandlestickConfig {
    fn default() -> Self {
        Self {
            doji_body_max: 0.1,
            long_wick_body_mult: 2.0,
            small_wick_max: 0.15,
            marubozu_wick_max: 0.05,
            spinning_body_max: 0.3,
            tweezer_eq_tol: 0.001,
        }
    }
}

/// A single candle reduced to f64 geometry.
#[derive(Debug, Clone, Copy)]
struct C {
    o: f64,
    h: f64,
    l: f64,
    c: f64,
}

impl C {
    fn range(&self) -> f64 {
        (self.h - self.l).max(f64::EPSILON)
    }
    fn body(&self) -> f64 {
        (self.c - self.o).abs()
    }
    fn upper_wick(&self) -> f64 {
        self.h - self.o.max(self.c)
    }
    fn lower_wick(&self) -> f64 {
        self.o.min(self.c) - self.l
    }
    fn bullish(&self) -> bool {
        self.c > self.o
    }
    fn bearish(&self) -> bool {
        self.c < self.o
    }
    fn body_top(&self) -> f64 {
        self.o.max(self.c)
    }
    fn body_bottom(&self) -> f64 {
        self.o.min(self.c)
    }
    fn mid(&self) -> f64 {
        (self.o + self.c) / 2.0
    }
}

/// A geometrically-detected pattern (Stage 1 output).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DetectedPattern {
    pub pattern: CandlestickPattern,
    pub direction: i8,
    /// Geometric quality in [0,1] — how cleanly the shape matches its template.
    pub quality: f64,
}

/// A pattern that has formed and is awaiting next-candle confirmation (Stage 3).
#[derive(Debug, Clone, Copy)]
pub struct PendingPattern {
    pub pattern: CandlestickPattern,
    pub direction: i8,
    pub quality: f64,
    /// Reference price the confirmation candle must break beyond.
    pub trigger_price: f64,
    /// Bars waited so far.
    pub age: u32,
}

/// Final classification of a candlestick reading on the current bar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CandlestickStatus {
    /// No pattern this bar.
    None,
    /// Pattern geometrically formed this bar (Stage 1 pass, awaiting confirmation).
    Formed,
    /// A previously-formed pattern was confirmed by this candle (Stage 3 pass).
    Confirmed,
    /// A previously-formed pattern was invalidated by this candle.
    Invalidated,
}

/// Per-bar candlestick recognition result.
#[derive(Debug, Clone, Copy)]
pub struct CandlestickResult {
    pub pattern: CandlestickPattern,
    pub direction: i8,
    pub quality: f64,
    pub status: CandlestickStatus,
}

impl CandlestickResult {
    fn none() -> Self {
        Self {
            pattern: CandlestickPattern::None,
            direction: 0,
            quality: 0.0,
            status: CandlestickStatus::None,
        }
    }
}

/// Candlestick pattern recognizer.
///
/// Stage 1 (Geometric Detection) runs in `update`, scanning the rolling window
/// for the highest-quality pattern. Stage 3 (Confirmation) tracks the most
/// recent formed pattern and confirms/invalidates it against the *next* candle.
/// Stage 2 (Context Validation) is applied downstream in normalization where the
/// full indicator map is available.
#[derive(Debug, Clone)]
pub struct Candlestick {
    cfg: CandlestickConfig,
    window: VecDeque<C>,
    pending: Option<PendingPattern>,
    max_confirm_age: u32,
}

impl Candlestick {
    pub fn new(cfg: CandlestickConfig) -> Self {
        Self {
            cfg,
            window: VecDeque::with_capacity(6),
            pending: None,
            max_confirm_age: 3,
        }
    }

    /// Feed a completed candle. Returns the recognition result for this bar.
    pub fn update(&mut self, open: Decimal, high: Decimal, low: Decimal, close: Decimal) -> CandlestickResult {
        let cur = C {
            o: open.to_f64().unwrap_or(0.0),
            h: high.to_f64().unwrap_or(0.0),
            l: low.to_f64().unwrap_or(0.0),
            c: close.to_f64().unwrap_or(0.0),
        };
        self.window.push_back(cur);
        while self.window.len() > 6 {
            self.window.pop_front();
        }

        // ── Stage 3: confirmation of a previously-formed pattern ──
        if let Some(mut p) = self.pending.take() {
            let confirmed = match p.direction {
                1 => cur.c > p.trigger_price,
                -1 => cur.c < p.trigger_price,
                _ => false,
            };
            let invalidated = match p.direction {
                1 => cur.c < cur.o && cur.c < p.trigger_price,
                -1 => cur.c > cur.o && cur.c > p.trigger_price,
                _ => true,
            };
            if confirmed {
                return CandlestickResult {
                    pattern: p.pattern,
                    direction: p.direction,
                    quality: p.quality,
                    status: CandlestickStatus::Confirmed,
                };
            } else if invalidated || p.age + 1 >= self.max_confirm_age {
                // Give up on this pending pattern; fall through to fresh detection.
                let inval = CandlestickStatus::Invalidated;
                // Try to detect a fresh pattern on this bar anyway.
                if let Some(det) = self.detect() {
                    self.arm(det, cur);
                    return CandlestickResult {
                        pattern: det.pattern,
                        direction: det.direction,
                        quality: det.quality,
                        status: CandlestickStatus::Formed,
                    };
                }
                return CandlestickResult {
                    pattern: p.pattern,
                    direction: p.direction,
                    quality: p.quality,
                    status: inval,
                };
            } else {
                // Still pending: age it and keep waiting.
                p.age += 1;
                self.pending = Some(p);
                return CandlestickResult::none();
            }
        }

        // ── Stage 1: fresh geometric detection ──
        if let Some(det) = self.detect() {
            self.arm(det, cur);
            return CandlestickResult {
                pattern: det.pattern,
                direction: det.direction,
                quality: det.quality,
                status: CandlestickStatus::Formed,
            };
        }
        CandlestickResult::none()
    }

    /// Arm a detected pattern for next-candle confirmation.
    fn arm(&mut self, det: DetectedPattern, cur: C) {
        // Bullish patterns confirm on a close above the signal candle's high;
        // bearish on a close below its low. Neutral patterns don't confirm.
        let trigger_price = match det.direction {
            1 => cur.h,
            -1 => cur.l,
            _ => cur.c,
        };
        if det.direction != 0 {
            self.pending = Some(PendingPattern {
                pattern: det.pattern,
                direction: det.direction,
                quality: det.quality,
                trigger_price,
                age: 0,
            });
        }
    }

    /// Stage 1: scan the window and return the highest-quality pattern (three →
    /// two → single precedence, so larger formations win ties).
    fn detect(&self) -> Option<DetectedPattern> {
        let n = self.window.len();
        let w: Vec<C> = self.window.iter().copied().collect();

        // Continuation (5 candles) — highest specificity.
        if n >= 5 {
            if let Some(p) = self.detect_continuation(&w[n - 5..]) {
                return Some(p);
            }
        }
        // Three-candle.
        if n >= 3 {
            if let Some(p) = self.detect_three(&w[n - 3..]) {
                return Some(p);
            }
        }
        // Two-candle.
        if n >= 2 {
            if let Some(p) = self.detect_two(&w[n - 2..]) {
                return Some(p);
            }
        }
        // Single-candle.
        if n >= 1 {
            if let Some(p) = self.detect_single(&w[n - 1]) {
                return Some(p);
            }
        }
        Option::None
    }

    // ───────────────────────── Single-candle ─────────────────────────
    fn detect_single(&self, c: &C) -> Option<DetectedPattern> {
        use CandlestickPattern::*;
        let cfg = &self.cfg;
        let range = c.range();
        let body = c.body();
        let uw = c.upper_wick();
        let lw = c.lower_wick();
        let body_frac = body / range;

        // Doji family: very small body.
        if body_frac <= cfg.doji_body_max {
            let uw_f = uw / range;
            let lw_f = lw / range;
            // Dragonfly: long lower wick, negligible upper.
            if lw_f >= 0.6 && uw_f <= cfg.small_wick_max {
                return Some(DetectedPattern { pattern: DragonflyDoji, direction: DragonflyDoji.direction(), quality: lw_f });
            }
            // Gravestone: long upper wick, negligible lower.
            if uw_f >= 0.6 && lw_f <= cfg.small_wick_max {
                return Some(DetectedPattern { pattern: GravestoneDoji, direction: GravestoneDoji.direction(), quality: uw_f });
            }
            // Long-legged: both wicks substantial.
            if uw_f >= 0.3 && lw_f >= 0.3 {
                return Some(DetectedPattern { pattern: LongLeggedDoji, direction: 0, quality: 1.0 - body_frac });
            }
            return Some(DetectedPattern { pattern: Doji, direction: 0, quality: 1.0 - body_frac });
        }

        // Marubozu: full body, negligible wicks.
        if uw / range <= cfg.marubozu_wick_max && lw / range <= cfg.marubozu_wick_max {
            let q = body_frac;
            if c.bullish() {
                return Some(DetectedPattern { pattern: BullishMarubozu, direction: 1, quality: q });
            } else {
                return Some(DetectedPattern { pattern: BearishMarubozu, direction: -1, quality: q });
            }
        }

        // Hammer / Hanging Man: long lower wick, small body near top, small upper.
        if lw >= cfg.long_wick_body_mult * body && uw / range <= cfg.small_wick_max && body_frac <= 0.35 {
            let q = (lw / range).min(1.0);
            // Bias by prior candle trend if available; default hammer (bullish reversal).
            let prior_down = self.prior_trend() < 0;
            if prior_down {
                return Some(DetectedPattern { pattern: Hammer, direction: 1, quality: q });
            } else {
                return Some(DetectedPattern { pattern: HangingMan, direction: -1, quality: q });
            }
        }

        // Inverted Hammer / Shooting Star: long upper wick, small body near bottom.
        if uw >= cfg.long_wick_body_mult * body && lw / range <= cfg.small_wick_max && body_frac <= 0.35 {
            let q = (uw / range).min(1.0);
            let prior_up = self.prior_trend() > 0;
            if prior_up {
                return Some(DetectedPattern { pattern: ShootingStar, direction: -1, quality: q });
            } else {
                return Some(DetectedPattern { pattern: InvertedHammer, direction: 1, quality: q });
            }
        }

        // Spinning Top: small body, wicks on both sides.
        if body_frac <= cfg.spinning_body_max && uw / range >= 0.2 && lw / range >= 0.2 {
            return Some(DetectedPattern { pattern: SpinningTop, direction: 0, quality: 1.0 - body_frac });
        }

        Option::None
    }

    /// Direction of the candle immediately before the latest one (-1/0/+1).
    fn prior_trend(&self) -> i8 {
        let n = self.window.len();
        if n < 2 {
            return 0;
        }
        let prev = self.window[n - 2];
        if prev.bearish() {
            -1
        } else if prev.bullish() {
            1
        } else {
            0
        }
    }

    // ───────────────────────── Two-candle ─────────────────────────
    fn detect_two(&self, w: &[C]) -> Option<DetectedPattern> {
        use CandlestickPattern::*;
        let cfg = &self.cfg;
        let a = w[0]; // prior
        let b = w[1]; // current
        let a_body = a.body();
        let b_body = b.body();

        // Engulfing: current body fully engulfs prior body, opposite color.
        if a.bearish() && b.bullish() && b.body_top() >= a.body_top() && b.body_bottom() <= a.body_bottom() && b_body > a_body {
            return Some(DetectedPattern { pattern: BullishEngulfing, direction: 1, quality: (b_body / a_body.max(f64::EPSILON)).min(2.0) / 2.0 });
        }
        if a.bullish() && b.bearish() && b.body_top() >= a.body_top() && b.body_bottom() <= a.body_bottom() && b_body > a_body {
            return Some(DetectedPattern { pattern: BearishEngulfing, direction: -1, quality: (b_body / a_body.max(f64::EPSILON)).min(2.0) / 2.0 });
        }

        // Piercing Line: prior bearish, current bullish opens below prior low, closes above prior midpoint but below prior open.
        if a.bearish() && b.bullish() && b.o < a.l && b.c > a.mid() && b.c < a.o {
            return Some(DetectedPattern { pattern: PiercingLine, direction: 1, quality: 0.7 });
        }
        // Dark Cloud Cover: prior bullish, current bearish opens above prior high, closes below prior midpoint but above prior open.
        if a.bullish() && b.bearish() && b.o > a.h && b.c < a.mid() && b.c > a.o {
            return Some(DetectedPattern { pattern: DarkCloudCover, direction: -1, quality: 0.7 });
        }

        // Tweezer Bottom/Top: matching lows/highs across two candles.
        let eq = |x: f64, y: f64| (x - y).abs() / x.max(f64::EPSILON) <= cfg.tweezer_eq_tol;
        if eq(a.l, b.l) && a.bearish() && b.bullish() {
            return Some(DetectedPattern { pattern: TweezerBottom, direction: 1, quality: 0.6 });
        }
        if eq(a.h, b.h) && a.bullish() && b.bearish() {
            return Some(DetectedPattern { pattern: TweezerTop, direction: -1, quality: 0.6 });
        }

        // Harami: current small body contained within prior large body, opposite color.
        if a.bearish() && b.bullish() && a_body > b_body && b.body_top() <= a.body_top() && b.body_bottom() >= a.body_bottom() {
            return Some(DetectedPattern { pattern: BullishHarami, direction: 1, quality: 1.0 - (b_body / a_body.max(f64::EPSILON)) });
        }
        if a.bullish() && b.bearish() && a_body > b_body && b.body_top() <= a.body_top() && b.body_bottom() >= a.body_bottom() {
            return Some(DetectedPattern { pattern: BearishHarami, direction: -1, quality: 1.0 - (b_body / a_body.max(f64::EPSILON)) });
        }

        Option::None
    }

    // ───────────────────────── Three-candle ─────────────────────────
    fn detect_three(&self, w: &[C]) -> Option<DetectedPattern> {
        use CandlestickPattern::*;
        let a = w[0];
        let b = w[1];
        let c = w[2];
        let a_body = a.body();
        let b_body = b.body();
        let c_body = c.body();
        let small_mid = b_body < a_body * 0.5 && b_body < c_body * 0.5;

        // Morning Star: bearish, small-body star, strong bullish closing above a's midpoint.
        if a.bearish() && small_mid && c.bullish() && c.c > a.mid() {
            return Some(DetectedPattern { pattern: MorningStar, direction: 1, quality: 0.85 });
        }
        // Evening Star: bullish, small star, strong bearish closing below a's midpoint.
        if a.bullish() && small_mid && c.bearish() && c.c < a.mid() {
            return Some(DetectedPattern { pattern: EveningStar, direction: -1, quality: 0.85 });
        }

        // Three White Soldiers: three rising bullish candles with higher closes.
        if a.bullish() && b.bullish() && c.bullish() && b.c > a.c && c.c > b.c && b.o > a.o && c.o > b.o {
            return Some(DetectedPattern { pattern: ThreeWhiteSoldiers, direction: 1, quality: 0.9 });
        }
        // Three Black Crows: three falling bearish candles with lower closes.
        if a.bearish() && b.bearish() && c.bearish() && b.c < a.c && c.c < b.c && b.o < a.o && c.o < b.o {
            return Some(DetectedPattern { pattern: ThreeBlackCrows, direction: -1, quality: 0.9 });
        }

        // Three Inside Up/Down: harami (a,b) then confirmation candle c.
        // Inside Up: a bearish, b bullish harami inside a, c closes above a's open.
        if a.bearish() && b.bullish() && b.body_top() <= a.body_top() && b.body_bottom() >= a.body_bottom() && c.bullish() && c.c > a.o {
            return Some(DetectedPattern { pattern: ThreeInsideUp, direction: 1, quality: 0.8 });
        }
        if a.bullish() && b.bearish() && b.body_top() <= a.body_top() && b.body_bottom() >= a.body_bottom() && c.bearish() && c.c < a.o {
            return Some(DetectedPattern { pattern: ThreeInsideDown, direction: -1, quality: 0.8 });
        }

        // Three Outside Up/Down: engulfing (a,b) then confirmation candle c.
        if a.bearish() && b.bullish() && b.body_top() >= a.body_top() && b.body_bottom() <= a.body_bottom() && c.bullish() && c.c > b.c {
            return Some(DetectedPattern { pattern: ThreeOutsideUp, direction: 1, quality: 0.82 });
        }
        if a.bullish() && b.bearish() && b.body_top() >= a.body_top() && b.body_bottom() <= a.body_bottom() && c.bearish() && c.c < b.c {
            return Some(DetectedPattern { pattern: ThreeOutsideDown, direction: -1, quality: 0.82 });
        }

        Option::None
    }

    // ───────────────────────── Continuation (5-candle) ─────────────────────────
    fn detect_continuation(&self, w: &[C]) -> Option<DetectedPattern> {
        use CandlestickPattern::*;
        let a = w[0];
        let mid = &w[1..4];
        let e = w[4];

        // Rising Three Methods: strong bullish, three small pullback candles held
        // within a's range, strong bullish close above a's close.
        if a.bullish()
            && mid.iter().all(|m| m.body() < a.body() && m.h <= a.h && m.l >= a.l)
            && e.bullish()
            && e.c > a.c
        {
            return Some(DetectedPattern { pattern: RisingThreeMethods, direction: 1, quality: 0.8 });
        }
        // Falling Three Methods: mirror.
        if a.bearish()
            && mid.iter().all(|m| m.body() < a.body() && m.h <= a.h && m.l >= a.l)
            && e.bearish()
            && e.c < a.c
        {
            return Some(DetectedPattern { pattern: FallingThreeMethods, direction: -1, quality: 0.8 });
        }

        Option::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn feed(cs: &mut Candlestick, o: f64, h: f64, l: f64, c: f64) -> CandlestickResult {
        cs.update(
            Decimal::from_f64_retain(o).unwrap(),
            Decimal::from_f64_retain(h).unwrap(),
            Decimal::from_f64_retain(l).unwrap(),
            Decimal::from_f64_retain(c).unwrap(),
        )
    }

    #[test]
    fn test_doji_detected() {
        let mut cs = Candlestick::new(CandlestickConfig::default());
        let r = feed(&mut cs, 100.0, 105.0, 95.0, 100.2);
        assert!(matches!(
            r.pattern,
            CandlestickPattern::Doji | CandlestickPattern::LongLeggedDoji
        ));
    }

    #[test]
    fn test_bullish_marubozu() {
        let mut cs = Candlestick::new(CandlestickConfig::default());
        let r = feed(&mut cs, 100.0, 110.05, 99.98, 110.0);
        assert_eq!(r.pattern, CandlestickPattern::BullishMarubozu);
        assert_eq!(r.direction, 1);
        assert_eq!(r.status, CandlestickStatus::Formed);
    }

    #[test]
    fn test_bullish_engulfing_and_confirmation() {
        let mut cs = Candlestick::new(CandlestickConfig::default());
        // Prior bearish candle.
        feed(&mut cs, 100.0, 101.0, 94.0, 95.0);
        // Bullish engulfing (opens below prior close, closes above prior open).
        let r = feed(&mut cs, 94.0, 103.0, 93.5, 102.0);
        assert_eq!(r.pattern, CandlestickPattern::BullishEngulfing);
        assert_eq!(r.status, CandlestickStatus::Formed);
        // Next candle closes above the engulfing high (103) → confirmed.
        let r2 = feed(&mut cs, 102.0, 106.0, 101.5, 105.0);
        assert_eq!(r2.status, CandlestickStatus::Confirmed);
        assert_eq!(r2.direction, 1);
    }

    #[test]
    fn test_engulfing_invalidation() {
        let mut cs = Candlestick::new(CandlestickConfig::default());
        feed(&mut cs, 100.0, 101.0, 94.0, 95.0);
        let r = feed(&mut cs, 94.0, 103.0, 93.5, 102.0);
        assert_eq!(r.pattern, CandlestickPattern::BullishEngulfing);
        // Next candle closes strongly bearish below trigger → not confirmed.
        let r2 = feed(&mut cs, 101.0, 101.5, 96.0, 97.0);
        assert_ne!(r2.status, CandlestickStatus::Confirmed);
    }

    #[test]
    fn test_three_white_soldiers() {
        let mut cs = Candlestick::new(CandlestickConfig::default());
        feed(&mut cs, 100.0, 105.0, 99.0, 104.0);
        feed(&mut cs, 103.0, 108.0, 102.0, 107.0);
        let r = feed(&mut cs, 106.0, 111.0, 105.0, 110.0);
        assert_eq!(r.pattern, CandlestickPattern::ThreeWhiteSoldiers);
        assert_eq!(r.direction, 1);
    }

    #[test]
    fn test_flat_no_pattern_eventually() {
        let mut cs = Candlestick::new(CandlestickConfig::default());
        // Identical flat candles → no meaningful pattern (or neutral doji).
        let r = feed(&mut cs, 100.0, 100.5, 99.5, 100.0);
        // A near-doji is acceptable; just ensure it doesn't panic and status valid.
        assert!(matches!(
            r.status,
            CandlestickStatus::None | CandlestickStatus::Formed
        ));
    }

    #[test]
    fn test_decimal_input_smoke() {
        let mut cs = Candlestick::new(CandlestickConfig::default());
        let r = cs.update(dec!(100), dec!(110), dec!(90), dec!(100));
        // Wide range, tiny body centered → long-legged doji.
        assert_eq!(r.direction, 0);
    }
}
