//! Stateless ratio mathematics for the MME Layers 1/3/5/6.
//!
//! These helpers back the four production ratios added in v6.11:
//!   - `price_trend_sharpe` (L1 metrics) — Sharpe of raw price log returns
//!   - `volatility_to_spread_ratio` (L5 risk, computed in `core-domain`)
//!   - `quality_to_risk_ratio` (L6 advisory, computed in `core-domain`)
//!
//! The Sharpe form shares the same annualization convention as the rest of
//! the platform (crypto-native, 365-day continuous market — see
//! `04-02-29-hv.md`): `sqrt(candles_per_day * 365)` where
//! `candles_per_day = 86_400 / timeframe_secs`.

/// Canonical trailing window for the Sharpe ratio — equals
/// `[candle_buffer] size` (300) so `price_trend_sharpe` reaches `Live`
/// exactly when the pipeline buffer fills (no lifecycle lock).
pub const SHARPE_WINDOW: usize = 300;

/// v6.10.21: maximum absolute annualized Sharpe published on the wire.
/// Near-flat series (e.g. an EMA-50 line on a quiet market) have tiny
/// return variance — the classic Sharpe `σ → 0` pathology can produce
/// annualized values like −117, which read as a defect on the dashboard.
/// The raw math stays formula-exact; the published value is clamped to the
/// ±20 band (well outside any actionable regime) so the UI can never
/// render an absurd number. The normalized score already clamps to ±1.
pub const SHARPE_MAX_ABS: f64 = 20.0;

/// Absolute standard-deviation floor (fractional log-return units) below
/// which the series is treated as numerically flat (`None`). Real markets
/// at any timeframe produce per-bar log-return σ well above 1e-9; EMA
/// derivative series on frozen markets can sit below it.
const SHARPE_STDDEV_FLOOR: f64 = 1e-9;

/// Annualization factor scaling a per-candle Sharpe to an annual basis.
///
/// `sqrt((86_400 / timeframe_secs) × 365)` — for a 60 s timeframe this is
/// `sqrt(1_440 × 365) ≈ 724.9`.
pub fn annualization_factor(timeframe_secs: u64) -> f64 {
    let candles_per_day = (86_400.0 / timeframe_secs.max(1) as f64).max(1.0);
    (candles_per_day * 365.0).sqrt()
}

/// Annualized Sharpe ratio of the logarithmic returns of a price series.
///
/// ```text
/// mean(ln(x_t / x_{t-1})) / stddev(ln(x_t / x_{t-1})) x annualization_factor
/// ```
///
/// Returns `None` when the series has fewer than 2 points (no return can be
/// formed), the return standard deviation is ≈ 0 (a perfectly flat series
/// — division guard), or σ sits below [`SHARPE_STDDEV_FLOOR`] (numerically
/// flat). The annualized output is clamped to ±[`SHARPE_MAX_ABS`] (v6.10.21
/// — the `σ → 0` pathology on near-flat EMA/price series can otherwise
/// explode to ±100+). The caller supplies the trailing window (the canonical
/// 300-bar buffer); this function is deliberately stateless so it can be
/// unit-tested and reused by both the live pipeline and the warm-up path.
pub fn sharpe_ratio_annualized(series: &[f64], timeframe_secs: u64) -> Option<f64> {
    if series.len() < 2 {
        return None;
    }
    let mut log_returns: Vec<f64> = Vec::with_capacity(series.len() - 1);
    for pair in series.windows(2) {
        let (prev, cur) = (pair[0], pair[1]);
        if prev <= 0.0 || cur <= 0.0 {
            return None;
        }
        log_returns.push((cur / prev).ln());
    }
    let n = log_returns.len() as f64;
    let mean = log_returns.iter().sum::<f64>() / n;
    let variance = log_returns
        .iter()
        .map(|r| {
            let d = r - mean;
            d * d
        })
        .sum::<f64>()
        / n;
    let stddev = variance.sqrt();
    if !stddev.is_finite() || stddev.abs() < SHARPE_STDDEV_FLOOR {
        return None;
    }
    let annualized = mean / stddev * annualization_factor(timeframe_secs);
    if !annualized.is_finite() {
        return None;
    }
    Some(annualized.clamp(-SHARPE_MAX_ABS, SHARPE_MAX_ABS))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_and_single_point_series_yield_none() {
        assert_eq!(sharpe_ratio_annualized(&[], 60), None);
        assert_eq!(sharpe_ratio_annualized(&[100.0], 60), None);
    }

    #[test]
    fn perfectly_flat_series_yields_none() {
        let flat = vec![100.0; 300];
        assert_eq!(sharpe_ratio_annualized(&flat, 60), None);
    }

    #[test]
    fn monotonic_rising_series_yields_high_positive_sharpe() {
        let series: Vec<f64> = (0..300).map(|i| 100.0 + i as f64 * 0.01).collect();
        let s = sharpe_ratio_annualized(&series, 60).expect("rising series must yield a value");
        assert!(s > 0.0, "rising series must be positive, got {s}");
        assert!(
            s > 2.0,
            "consistent rise must clear the significance band, got {s}"
        );
    }

    #[test]
    fn monotonic_falling_series_yields_negative_sharpe() {
        let series: Vec<f64> = (0..300).map(|i| 100.0 - i as f64 * 0.01).collect();
        let s = sharpe_ratio_annualized(&series, 60).expect("falling series must yield a value");
        assert!(s < 0.0, "falling series must be negative, got {s}");
    }

    #[test]
    fn alternating_series_yields_near_zero_sharpe() {
        // 301 points → 300 returns with exactly 150 up-legs and 150
        // down-legs, so the log-return mean is exactly zero (an even
        // return count avoids the odd-sample drift bias).
        let series: Vec<f64> = (0..301)
            .map(|i| if i % 2 == 0 { 100.0 } else { 100.5 })
            .collect();
        let s =
            sharpe_ratio_annualized(&series, 60).expect("alternating series must yield a value");
        assert!(
            s.abs() < 1.0,
            "alternating noise must be near zero, got {s}"
        );
    }

    #[test]
    fn annualization_scales_with_timeframe() {
        // A noisy series (sinusoidal ±1.5 on top of a 0.01 %/bar drift) keeps
        // the annualized Sharpe inside the ±20 clamp band at both timeframes,
        // so the ratio reflects the pure annualization scaling: sqrt(5) for
        // 60 s vs 300 s.
        let series: Vec<f64> = (0..300)
            .map(|i| {
                let i = i as f64;
                100.0 * (1.0 + 0.0001 * i) + (i * 0.7).sin() * 1.5
            })
            .collect();
        let s_60 = sharpe_ratio_annualized(&series, 60).unwrap();
        let s_300 = sharpe_ratio_annualized(&series, 300).unwrap();
        assert!(
            s_60.abs() < SHARPE_MAX_ABS,
            "test series must stay in band: {s_60}"
        );
        assert!(
            (s_60 / s_300 - (5.0_f64).sqrt()).abs() < 1e-6,
            "scale must follow sqrt(candles_per_day): {} vs {}",
            s_60 / s_300,
            (5.0_f64).sqrt()
        );
    }

    #[test]
    fn extremely_smooth_series_clamps_at_max_abs() {
        // v6.10.21: a near-monotonic series (tiny variance → Sharpe σ→0
        // pathology) must clamp at ±SHARPE_MAX_ABS instead of exploding to
        // values like −117 that read as defects on the dashboard.
        let rising: Vec<f64> = (0..300).map(|i| 100.0 + i as f64 * 0.01).collect();
        let s_up =
            sharpe_ratio_annualized(&rising, 60).expect("smooth rising series yields a value");
        assert_eq!(
            s_up, SHARPE_MAX_ABS,
            "smooth rising series must clamp at +20"
        );
        let falling: Vec<f64> = (0..300).map(|i| 100.0 - i as f64 * 0.01).collect();
        let s_down =
            sharpe_ratio_annualized(&falling, 60).expect("smooth falling series yields a value");
        assert_eq!(
            s_down, -SHARPE_MAX_ABS,
            "smooth falling series must clamp at −20"
        );
    }

    #[test]
    fn numerically_flat_series_below_stddev_floor_yields_none() {
        // v6.10.21: sub-1e-9 log-return variance (e.g. an EMA-50 line on a
        // frozen market) is numerically flat — treat as None, not as a
        // gigantic Sharpe.
        let series: Vec<f64> = (0..300).map(|i| 100.0 + i as f64 * 1e-10).collect();
        assert_eq!(sharpe_ratio_annualized(&series, 60), None);
    }

    #[test]
    fn annualization_factor_matches_expected_values() {
        // 60 s → sqrt(1440 × 365) = sqrt(525 600) ≈ 724.98
        assert!((annualization_factor(60) - 724.98).abs() < 0.01);
        // 1 s → sqrt(86 400 × 365) = sqrt(31 536 000) ≈ 5615.69
        assert!((annualization_factor(1) - 5615.69).abs() < 0.01);
        // Identity: factor must equal the closed form.
        assert!((annualization_factor(300) - (288.0_f64 * 365.0).sqrt()).abs() < 1e-12);
    }
}
