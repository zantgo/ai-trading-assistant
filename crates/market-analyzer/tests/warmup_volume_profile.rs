//! Regression test: warm-up snapshots must carry a populated `volume_profile`
//! from the same bar the live analyzer does (i.e. the 250th bar in for the
//! default `volume_profile_window=500`). Before this fix the warm-up snapshot
//! always serialised `volume_profile: None`, so any dashboard that fetched
//! `/api/history?…` before the first live candle close saw nothing for fast /
//! slow / macro TFs (micro recovered fastest because its 250-bar gate clears
//! within minutes, the others sat empty until their first live close).

use config_models::{FibonacciConfig, TimeframeConfig};
use core_domain::normalized::{Exchange, NormalizedCandle};
use rust_decimal::Decimal;

fn make_test_config(window: usize, bins: usize) -> TimeframeConfig {
    use config_models::IndicatorsConfig;
    TimeframeConfig {
        candles: config_models::CandlesConfig {
            duration_seconds: 60,
            analysis_limit: 500,
        },
        indicators: IndicatorsConfig {
            ema_fast: 10,
            ema_medium: 50,
            ema_slow: 100,
            ema_long: 200,
            rsi_period: 14,
            macd_fast: 12,
            macd_slow: 26,
            macd_signal: 9,
            adx_period: 14,
            atr_period: 14,
            squeeze_period: 20,
            squeeze_bb_period: 20,
            squeeze_bb_std_dev: 2.0,
            squeeze_kc_period: 20,
            stoch_k_period: 18,
            stoch_d_period: 5,
            stoch_s_period: 9,
            chandemo_period: 12,
            supertrend_period: 10,
            supertrend_multiplier: 3.0,
            keltner_ema_period: 20,
            keltner_atr_period: 10,
            keltner_multiplier: 2.0,
            donchian_period: 20,
            obv_smoothing: 20,
            cmf_period: 20,
            mfi_period: 20,
            hv_period: 20,
            aroon_period: 25,
            chop_period: 14,
            linreg_period: 20,
            zscore_period: 20,
            bbwp_lookback: 252,
            bbwp_period: 20,
            macd_extreme_high_threshold: 1000.0,
            macd_extreme_low_threshold: -1000.0,
            macd_histogram_contraction_threshold: 0.3,
            adx_trend_threshold: 20,
            adx_exhaustion_threshold: 40,
            adx_slope_lookback: 3,
            squeeze_min_duration: 5,
            squeeze_kc_atr_multiplier: 1.5,
            atr_multiplier_coefficient: 2.0,
            atr_target_rr_ratio: 2.5,
            volume_average_period: 20,
            rvol_threshold_institutional: 1.5,
            rvol_threshold_climax: 3.0,
            ichimoku_tenkan: 9,
            ichimoku_kijun: 26,
            ichimoku_senkou_b: 52,
            ichimoku_displacement: 26,
            cci_period: 20,
            psar_af_step: 0.02,
            psar_af_max: 0.2,
            williams_r_period: 14,
            hull_ma_period: 21,
            force_index_smoothing: 13,
            stddev_channel_period: 20,
            smc_lookback: 20,
            volume_profile_bins: bins,
            volume_profile_window: window,
            volume_profile_value_area: 0.7,
        },
    }
}

fn fib_config() -> FibonacciConfig {
    FibonacciConfig {
        swing_lookback: 5,
        swing_scan_range: 50,
        retracement_coefficients: vec![0.236, 0.382, 0.5, 0.618, 0.786],
        extension_coefficients: vec![1.0, 1.272, 1.618],
    }
}

/// Build `count` synthetic candles spanning a realistic walk. Each candle has
/// open/high/low/close around a slowly-trending midpoint and a synthetic
/// volume that's higher for bullish candles (so the buy/sell split is
/// meaningful for later parity checks).
fn synth_candles(count: usize, secs_per_candle: u64) -> Vec<NormalizedCandle> {
    let base_ts: u64 = 1_700_000_000_000;
    let mut candles = Vec::with_capacity(count);
    for i in 0..count {
        let ts = base_ts + (i as u64) * secs_per_candle * 1000;
        let drift = (i as f64) * 0.05;
        let mid = 50_000.0 + drift;
        let span = 12.0 + (i as f64 % 7.0);
        let high = mid + span;
        let low = mid - span;
        // Bullish every 3rd bar, bearish otherwise — exercises buy/sell split.
        let (open, close) = if i % 3 == 0 { (low, high) } else { (high, low) };
        let vol = 10.0 + (i as f64 % 13.0);
        candles.push(NormalizedCandle {
            exchange: Exchange::Hyperliquid,
            symbol: "BTC-USDC".to_string(),
            start_time_ms: ts,
            duration_ms: secs_per_candle * 1000,
            trades_count: 10,
            reconstructed: None,
            open: Decimal::from_f64_retain(open).unwrap(),
            high: Decimal::from_f64_retain(high).unwrap(),
            low: Decimal::from_f64_retain(low).unwrap(),
            close: Decimal::from_f64_retain(close).unwrap(),
            volume: Decimal::from_f64_retain(vol).unwrap(),
        });
    }
    candles
}

#[test]
fn warmup_populates_volume_profile_from_gate_bar_onward() {
    // 500 candles is the default `volume_profile_window`; gate clears at 250.
    let candles = synth_candles(500, 60);
    let cfg = make_test_config(500, 50);

    let warmed = market_analyzer::analyzer::warm::warm_indicators_for_timeframe(
        candles.clone(),
        &cfg,
        &fib_config(),
        "BTC-USDC",
        60,
        core_domain::models::TimeframeSlot::Micro,
    );

    assert!(
        !warmed.snapshot_history.is_empty(),
        "warmup should produce at least one snapshot",
    );

    // Pre-gate snapshots should still be None — matches live behaviour when
    // the indicator hasn't yet accumulated enough bars.
    let pre_gate_nones = warmed
        .snapshot_history
        .iter()
        .take(249)
        .filter(|s| s.volume_profile.is_none())
        .count();
    assert!(
        pre_gate_nones >= 248,
        "expected the first ~249 warm-up snapshots to have volume_profile:None (got {}/249)",
        pre_gate_nones,
    );

    // From the 250th snapshot onward the bin-level snapshot must be present.
    let post_gate_snapshots: Vec<_> = warmed
        .snapshot_history
        .iter()
        .skip(249)
        .filter(|s| s.volume_profile.is_some())
        .collect();
    assert!(
        !post_gate_snapshots.is_empty(),
        "no warm-up snapshots past the gate carried volume_profile",
    );

    // Last snapshot must have the bin-level profile (this is what /api/history reads).
    let last_vp = warmed
        .snapshot_history
        .last()
        .and_then(|s| s.volume_profile.as_ref())
        .expect("last warm-up snapshot must carry volume_profile");

    assert!(
        !last_vp.bins.is_empty(),
        "last warm-up volume_profile should have populated bins",
    );
    assert!(
        last_vp.bins.len() <= 50,
        "bin count must be within dynamic_bin_count clamp (got {})",
        last_vp.bins.len(),
    );
    assert!(
        last_vp.poc_price >= last_vp.range_low && last_vp.poc_price <= last_vp.range_high,
        "POC must be within range (got poc={}, range=({}..{}))",
        last_vp.poc_price,
        last_vp.range_low,
        last_vp.range_high,
    );
    // Bins sorted ascending by price_low per doc contract.
    let mut prev_lo = f64::NEG_INFINITY;
    for bin in &last_vp.bins {
        assert!(
            bin.price_low >= prev_lo,
            "bins must be sorted ascending by price_low (got {} after {})",
            bin.price_low,
            prev_lo,
        );
        prev_lo = bin.price_low;
    }
    // POC bin must be the bin with highest volume.
    let poc_bin = last_vp
        .bins
        .iter()
        .find(|b| b.is_poc)
        .expect("exactly one bin must be POC");
    let max_vol = last_vp.bins.iter().map(|b| b.volume).fold(0.0_f64, f64::max);
    assert!(
        (poc_bin.volume - max_vol).abs() < 1e-9,
        "POC bin must be the highest-volume bin",
    );
}

#[test]
fn warmup_sub_minute_timeframes_also_populate() {
    // Sub-minute timeframes (e.g. 5s bars) are supported by `duration_seconds`
    // and must also produce the bin-level snapshot in warm-up, otherwise the
    // dashboard would still show no volume profile until the first live
    // sub-minute close — exactly the bug the user reported.
    let candles = synth_candles(500, 5);
    let cfg = make_test_config(500, 50);

    let warmed = market_analyzer::analyzer::warm::warm_indicators_for_timeframe(
        candles,
        &cfg,
        &fib_config(),
        "BTC-USDC",
        5,
        core_domain::models::TimeframeSlot::Micro,
    );

    let last_vp = warmed
        .snapshot_history
        .last()
        .and_then(|s| s.volume_profile.as_ref())
        .expect("5s-TF warm-up must carry volume_profile on last snapshot");
    assert!(!last_vp.bins.is_empty());
    assert_eq!(last_vp.timeframe_secs, 5);
    assert_eq!(last_vp.timeframe_slot, "micro");
}
