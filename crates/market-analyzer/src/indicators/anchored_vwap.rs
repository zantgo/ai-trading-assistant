use rust_decimal::Decimal;

#[derive(Debug, Clone)]
pub struct AnchoredVwap {
    // Weekly accumulator
    wk_tp_vol: Decimal,
    wk_vol: Decimal,
    last_week: Option<u64>,
    // Monthly accumulator
    mo_tp_vol: Decimal,
    mo_vol: Decimal,
    last_month: Option<u64>,
    // Swing-anchored (auto from most recent pivot)
    sw_tp_vol: Decimal,
    sw_vol: Decimal,
    #[allow(dead_code)]
    last_swing_bar: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct AvwapOutput {
    pub vwap_daily: Decimal,
    pub vwap_weekly: Option<Decimal>,
    pub vwap_monthly: Option<Decimal>,
    pub vwap_swing: Option<Decimal>,
}

impl Default for AnchoredVwap {
    fn default() -> Self {
        Self::new()
    }
}

impl AnchoredVwap {
    pub fn new() -> Self {
        Self {
            wk_tp_vol: Decimal::ZERO,
            wk_vol: Decimal::ZERO,
            last_week: None,
            mo_tp_vol: Decimal::ZERO,
            mo_vol: Decimal::ZERO,
            last_month: None,
            sw_tp_vol: Decimal::ZERO,
            sw_vol: Decimal::ZERO,
            last_swing_bar: None,
        }
    }

    pub fn update(
        &mut self,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
        day_index: u64,
        daily_vwap: f64,
    ) -> AvwapOutput {
        let high = Decimal::from_f64_retain(high).unwrap_or(Decimal::ZERO);
        let low = Decimal::from_f64_retain(low).unwrap_or(Decimal::ZERO);
        let close = Decimal::from_f64_retain(close).unwrap_or(Decimal::ZERO);
        let volume = Decimal::from_f64_retain(volume).unwrap_or(Decimal::ZERO);
        let tp = (high + low + close) / Decimal::from(3);

        // Weekly reset
        let week_index = day_index / 7;
        match self.last_week {
            Some(w) if w == week_index => {
                self.wk_tp_vol += tp * volume;
                self.wk_vol += volume;
            }
            _ => {
                self.wk_tp_vol = tp * volume;
                self.wk_vol = volume;
                self.last_week = Some(week_index);
            }
        }

        // Monthly reset (approx 30-day month)
        let month_index = day_index / 30;
        match self.last_month {
            Some(m) if m == month_index => {
                self.mo_tp_vol += tp * volume;
                self.mo_vol += volume;
            }
            _ => {
                self.mo_tp_vol = tp * volume;
                self.mo_vol = volume;
                self.last_month = Some(month_index);
            }
        }

        // Swing-anchored: accumulates from bar 0 onward unless reset by a new swing.
        // The engine resets this externally via reset_swing() when a new swing pivot forms.
        self.sw_tp_vol += tp * volume;
        self.sw_vol += volume;

        AvwapOutput {
            vwap_daily: Decimal::from_f64_retain(daily_vwap).unwrap_or(Decimal::ZERO),
            // When an anchor bucket has not yet accumulated any volume (e.g.
            // the very first bar of a fresh week, or sub-minute timeframes
            // whose candle volume is still rounding to zero), fall back to
            // the current typical price. Returning `None` would cause the
            // normalizer at indicators/normalized/all.rs:381 to skip the
            // entire AVWAP entry, leaving the dashboard indicator stuck in
            // `WARMING` regardless of how many completed candles exist.
            // The frontend's `AVWAP_AT_ACTIVE` label already handles the
            // price-equal case, so this fallback surfaces a coherent
            // "AVWAP tracks price while volume is zero" reading instead of
            // hiding the indicator.
            vwap_weekly: if self.wk_vol > Decimal::ZERO {
                Some(self.wk_tp_vol / self.wk_vol)
            } else {
                Some(tp)
            },
            vwap_monthly: if self.mo_vol > Decimal::ZERO {
                Some(self.mo_tp_vol / self.mo_vol)
            } else {
                Some(tp)
            },
            vwap_swing: if self.sw_vol > Decimal::ZERO {
                Some(self.sw_tp_vol / self.sw_vol)
            } else {
                Some(tp)
            },
        }
    }

    /// Reset the swing-anchored accumulator (called on new swing pivot).
    pub fn reset_swing(&mut self) {
        self.sw_tp_vol = Decimal::ZERO;
        self.sw_vol = Decimal::ZERO;
    }
}
