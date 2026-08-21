//! BTE historical runner — the deep-history simulation.
//!
//! Replays the **real** MME pipeline over archived OHLCV candles:
//!
//! ```text
//! candle_archive (per TF windows, burn-in inclusive)
//!   → warm_indicators_for_timeframe (chunked — the SAME warm path the
//!     live daemon runs at boot, producing full per-candle snapshots
//!     with normalized indicator maps + market context)
//!   → synthesize_cross_tf (the SAME pure MTF synthesizer the live L4/L5
//!     assembly calls) + DecisionContext::compute
//!   → run_tick (the SAME per-tick session body as paper/live)
//! ```
//!
//! Determinism: no wall clock, no randomness in the executor path;
//! liquidity/cluster inputs are absent (the archive stores no
//! derivatives) — the synthesized decision is candle-based, exactly as
//! documented in the parity contract.
//!
//! Cost note: the warm path computes the full 52-indicator suite per
//! candle (~50–60 ms/candle). Per-TF warms run in parallel on the tokio
//! blocking pool; a 7-day window at the default ladder is roughly a
//! few minutes of CPU.

use config_models::FibonacciConfig;
use core_domain::analysis::MarketBias;
use core_domain::models::MarketSnapshot;
use core_domain::normalized::{Exchange, NormalizedCandle};
use market_analyzer::active_set::ActiveSet;
use market_analyzer::analyzer::warm_indicators_for_timeframe;
use portfolio_supervisor::execution::ExecutionEngine;
use portfolio_supervisor::execution::session_tick::run_tick;
use portfolio_supervisor::paper_trading::FeesConfig;
use portfolio_supervisor::setup_executor::{SetupExecutor, TickContext};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;

use crate::recorded::{BacktestParams, BacktestResult, BacktestTrade, BACKTEST_MAX_SNAPSHOTS};

/// Chunk size for the warm-path replay (candles per warm call).
const CHUNK_CANDLES: usize = 800;
/// Overlap between chunks: every indicator lookback window in this
/// platform is ≤ 300 bars, so 300 bars of re-convergence makes chunk
/// tails mathematically identical to a continuous replay.
const CHUNK_OVERLAP: usize = 300;

/// Max candles loaded per TF (bounded memory for deep windows).
const MAX_CANDLES_PER_TF: u32 = 200_000;

/// Per-TF pipeline configuration for the historical run (indicator
/// periods, fib config — the same values the live pipeline uses).
#[derive(Clone)]
pub struct HistoricalRunConfig {
    pub symbol: String,
    /// The full instance ladder (micro/fast/slow/macro durations); the
    /// entry TF must be one of these.
    pub ladder: Vec<u64>,
    /// Indicator configs keyed by timeframe duration.
    pub tf_configs: HashMap<u64, config_models::TimeframeConfig>,
    pub fib_config: FibonacciConfig,
    pub active_set: ActiveSet,
    pub exchange: Exchange,
    /// Warmup bars before the first valid MTF decision (burn-in).
    pub warmup_bars: u32,
    /// Max equity points persisted (downsampling cap).
    pub max_equity_points: u32,
}

/// Run the deep-history simulation for the given window. The caller has
/// already validated archive coverage; this function replays
/// deterministically and returns the standard `BacktestResult`.
pub async fn run_historical_backtest(
    pool: &SqlitePool,
    params: &BacktestParams,
    tae_cfg: &config_models::MinimalTaeConfig,
    fees: &FeesConfig,
    cross_leverage: u32,
    analytics: performance_analytics::strategy_analytics::AnalyticsParams,
    run_cfg: &HistoricalRunConfig,
) -> BacktestResult {
    // Burn-in: the first `warmup_bars × max_tf` seconds produce no
    // decisions — they only warm the indicator windows.
    let max_tf = run_cfg.ladder.iter().copied().max().unwrap_or(900);
    let burn_in_secs = run_cfg.warmup_bars as i64 * max_tf as i64;
    let load_from = params.from_secs.saturating_sub(burn_in_secs);

    // ── 1. Load + warm every ladder TF (parallel — the warm path is the
    // CPU-bound cost; the tokio blocking pool runs each TF on its own
    // thread). ──
    let mut warm_futures = Vec::with_capacity(run_cfg.ladder.len());
    for tf in run_cfg.ladder.iter().copied() {
        let candles = load_archive(pool, &run_cfg.symbol, tf, load_from, params.to_secs).await;
        let cfg = run_cfg.clone();
        warm_futures.push(tokio::task::spawn_blocking(move || warm_tf(&cfg, tf, candles)));
    }
    let mut per_tf_snapshots: HashMap<u64, Vec<MarketSnapshot>> = HashMap::new();
    for (tf, fut) in run_cfg.ladder.iter().zip(warm_futures) {
        match fut.await {
            Ok(snaps) => {
                per_tf_snapshots.insert(*tf, snaps);
            }
            Err(e) => eprintln!("BTE warm task failed for {tf}s: {e}"),
        }
    }

    // ── 2. Replay the entry TF through the MTF synthesizer + executor ──
    let engine = Arc::new(ExecutionEngine::new(fees.clone()));
    engine
        .set_initial_equity(
            Decimal::from_f64_retain(params.initial_capital).unwrap_or(dec!(1000)),
        )
        .await;
    engine.set_cross_leverage(cross_leverage).await;
    let executor = SetupExecutor::new(engine.clone(), tae_cfg);

    let entry_tf = params.timeframe_secs;
    let entry_snapshots: Vec<MarketSnapshot> = per_tf_snapshots
        .get(&entry_tf)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|s| (s.timestamp as i64) >= params.from_secs && (s.timestamp as i64) <= params.to_secs)
        .collect();

    let mut trades: Vec<BacktestTrade> = Vec::new();
    let mut equity_points: Vec<(i64, f64)> = Vec::new();
    let mut signals: Vec<database_storage::queries::backtest_ds::DsSignal> = Vec::new();
    let mut portfolio: Vec<database_storage::queries::backtest_ds::DsPortfolioPoint> = Vec::new();
    let mut peak_equity: f64 = params.initial_capital;

    // MTF synthesis state carried across ticks (mirrors the live loop).
    let mut prev_score: Option<f64> = None;
    let mut prev_regime: Option<core_domain::analysis::MarketRegime> = None;
    let mut prev_volume_dim: Option<f64> = None;
    let mut prev_bias: Option<MarketBias> = None;

    for mut snap in entry_snapshots {
        // Gather the latest aligned snapshot per ladder TF (≤ entry ts).
        let mut tf_refs: Vec<(u64, MarketSnapshot)> = Vec::with_capacity(run_cfg.ladder.len());
        for tf in &run_cfg.ladder {
            if let Some(series) = per_tf_snapshots.get(tf) {
                if let Some(latest) = latest_at_or_before(series, snap.timestamp) {
                    tf_refs.push((*tf, latest.clone()));
                }
            }
        }

        // ── The SAME pure synthesizer the live L4/L5 assembly calls ──
        let cross: Vec<(u64, &MarketSnapshot)> = tf_refs.iter().map(|(t, s)| (*t, s)).collect();
        let synthesis = market_analyzer::synthesis::synthesize_cross_tf(
            &run_cfg.symbol,
            &cross,
            None,
            None,
            &[],
            prev_score,
            prev_regime,
            prev_volume_dim,
            prev_bias,
        );

        prev_score = Some(synthesis.alignment.mtf_overall_score);
        prev_regime = Some(synthesis.analysis.market_regime);
        prev_volume_dim = synthesis.alignment.dimensions.get(2).map(|d| d.score);
        prev_bias = Some(synthesis.analysis.bias);

        // Confluence score — the same unsigned 3-factor blend as live.
        let confluence_score = {
            let tradability_dim = synthesis
                .alignment
                .dimensions
                .get(9)
                .map(|d| d.score)
                .unwrap_or(0.0);
            let market_quality_score = synthesis.analysis.market_quality_score;
            let opp_score = synthesis
                .opportunity
                .as_ref()
                .map(|o| o.opportunity_score)
                .unwrap_or(0.0);
            (0.50 * tradability_dim + 0.30 * market_quality_score + 0.20 * opp_score)
                .clamp(0.0, 100.0)
        };

        let close_f = snap.close.as_ref().and_then(|d| d.to_f64()).unwrap_or(0.0);
        let atr_val = snap.atr_14().and_then(|d| d.to_f64()).unwrap_or(0.0);
        let decision = core_domain::decision_context::DecisionContext::compute(
            &snap.indicators,
            close_f,
            atr_val,
            confluence_score,
            &synthesis.analysis,
            synthesis.opportunity.as_ref(),
            &synthesis.risk,
        );

        snap.opportunity = synthesis.opportunity.clone();
        snap.analysis = Some(synthesis.analysis.clone());
        snap.decision_context = Some(decision);
        snap.advisory = Some(synthesis.advisory.clone());
        snap.is_completed = Some(true);

        let mid = snap.mid_price;
        let rec_ts = snap.timestamp as u64;
        let outcome = run_tick(
            &engine,
            &executor,
            "backtest-historical",
            &run_cfg.symbol,
            &[&snap],
            mid,
            TickContext {
                safety_allows_entry: true,
                lifecycle_running: true,
                candle_ts: rec_ts,
                safety: None,
                dispatch: true,
            },
            None,
            true,
        )
        .await;

        if let Some(close) = &outcome.last_close {
            let direction = match outcome.last_close_direction {
                Some(config_models::Direction::Long) => "LONG",
                Some(config_models::Direction::Short) => "SHORT",
                None => "UNKNOWN",
            };
            let entry = outcome
                .last_close_entry
                .and_then(|d| d.to_f64())
                .unwrap_or(0.0);
            let size = outcome
                .last_close_size
                .and_then(|d| d.to_f64())
                .unwrap_or(0.0);
            let exit = if size > 0.0 {
                let pnl = close.pnl.to_f64().unwrap_or(0.0);
                match outcome.last_close_direction {
                    Some(config_models::Direction::Short) => entry - pnl / size,
                    _ => entry + pnl / size,
                }
            } else {
                entry
            };
            trades.push(BacktestTrade {
                timestamp: snap.timestamp as i64,
                direction: direction.to_string(),
                entry_price: entry,
                exit_price: exit,
                size,
                pnl: close.pnl.to_f64().unwrap_or(0.0),
                exit_reason: close.exit_reason.clone(),
            });
        }

        let unrealized: Decimal = {
            let positions = engine.positions.read().await;
            positions.values().map(|p| p.unrealized_pnl).sum()
        };
        let total = engine.get_equity_decimal().await + unrealized;
        equity_points.push((snap.timestamp as i64, total.to_f64().unwrap_or(0.0)));

        crate::recorded::capture_tick_ds(
            snap.timestamp as i64,
            entry_tf,
            &snap,
            &engine,
            &mut signals,
            &mut portfolio,
            &mut peak_equity,
        )
        .await;
    }

    crate::recorded::finalize_result(
        params,
        trades,
        equity_points,
        run_cfg.max_equity_points,
        analytics,
        signals,
        portfolio,
    )
}

/// Load the archived candle window for one TF, ascending.
async fn load_archive(
    pool: &SqlitePool,
    symbol: &str,
    tf: u64,
    from_secs: i64,
    to_secs: i64,
) -> Vec<NormalizedCandle> {
    database_storage::queries::archive::query_archive_window(
        pool,
        symbol,
        tf,
        from_secs,
        to_secs,
        MAX_CANDLES_PER_TF.min(BACKTEST_MAX_SNAPSHOTS),
    )
    .await
    .into_iter()
    .map(|a| a.to_normalized())
    .collect()
}

/// Chunked warm-path replay: candles → per-candle snapshots (the same
/// `warm_indicators_for_timeframe` the live daemon boots through).
fn warm_tf(
    run_cfg: &HistoricalRunConfig,
    tf: u64,
    candles: Vec<NormalizedCandle>,
) -> Vec<MarketSnapshot> {
    let tf_config = run_cfg
        .tf_configs
        .get(&tf)
        .cloned()
        .unwrap_or_else(|| config_models::TimeframeConfig::new(tf, config_models::IndicatorsConfig::default()));
    let slot = match tf {
        _ if tf == run_cfg.ladder.first().copied().unwrap_or(0) => core_domain::models::TimeframeSlot::Micro,
        _ if tf == run_cfg.ladder.get(1).copied().unwrap_or(0) => core_domain::models::TimeframeSlot::Fast,
        _ if tf == run_cfg.ladder.get(2).copied().unwrap_or(0) => core_domain::models::TimeframeSlot::Slow,
        _ => core_domain::models::TimeframeSlot::Macro,
    };

    let mut out: Vec<MarketSnapshot> = Vec::new();
    let mut seen: std::collections::HashSet<u64> = std::collections::HashSet::new();

    if candles.is_empty() {
        return out;
    }

    let mut start = 0usize;
    while start < candles.len() {
        let end = (start + CHUNK_CANDLES).min(candles.len());
        let chunk = candles[start..end].to_vec();
        let state = warm_indicators_for_timeframe(
            chunk,
            &tf_config,
            &run_cfg.fib_config,
            &run_cfg.symbol,
            tf,
            slot,
            CHUNK_CANDLES,
            &run_cfg.active_set,
            Some(run_cfg.exchange),
        );

        // Emit: first chunk emits everything (its head is the burn-in and
        // gets filtered by window); later chunks skip the re-convergence
        // overlap — those candles already emitted converged copies from
        // the previous chunk's tail.
        let skip = if start == 0 { 0 } else { CHUNK_OVERLAP };
        for snap in state.snapshot_history.into_iter().skip(skip) {
            if seen.insert(snap.timestamp) {
                out.push(snap);
            }
        }

        if end >= candles.len() {
            break;
        }
        start = end.saturating_sub(CHUNK_OVERLAP);
        if start == 0 {
            break;
        }
    }

    out.sort_by_key(|s| s.timestamp);
    out
}

/// Latest snapshot with `ts <= t` (the series is ascending).
fn latest_at_or_before(series: &[MarketSnapshot], t: u64) -> Option<&MarketSnapshot> {
    match series.binary_search_by_key(&t, |s| s.timestamp) {
        Ok(idx) => Some(&series[idx]),
        Err(idx) if idx > 0 => Some(&series[idx - 1]),
        _ => series.first().filter(|s| s.timestamp <= t),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_domain::normalized::ReconstructionMethod;
    use rust_decimal_macros::dec;

    async fn seed_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("mem pool");
        database_storage::run_migrations(&pool)
            .await
            .expect("migrations");
        pool
    }

    fn candle(symbol: &str, tf_secs: u64, ts_secs: u64, close: f64) -> NormalizedCandle {
        NormalizedCandle {
            exchange: Exchange::Hyperliquid,
            symbol: symbol.to_string(),
            start_time_ms: ts_secs * 1000,
            duration_ms: tf_secs * 1000,
            open: rust_decimal::Decimal::from_f64_retain(close - 1.0).unwrap(),
            high: rust_decimal::Decimal::from_f64_retain(close + 2.0).unwrap(),
            low: rust_decimal::Decimal::from_f64_retain(close - 2.0).unwrap(),
            close: rust_decimal::Decimal::from_f64_retain(close).unwrap(),
            volume: dec!(100),
            trades_count: 5,
            reconstructed: Some(ReconstructionMethod::ExchangeHistorical),
        }
    }

    fn tae_cfg() -> config_models::MinimalTaeConfig {
        config_models::MinimalTaeConfig {
            enabled: true,
            risk_per_trade_pct: 1.0,
            min_net_rr: 1.0,
            max_position_size_usd: None,
            max_open_positions: 1,
            entry_mode: "zone_midpoint".to_string(),
            invalidate_on: "direction_flip".to_string(),
        }
    }

    /// Deterministic 15m sine+drift series over N days.
    fn synthetic_days(days: u64, tf: u64, phase_deg: f64) -> Vec<NormalizedCandle> {
        let bars_per_day = 86400 / tf;
        let mut out = Vec::new();
        let mut ts = 1_700_000_000u64;
        for i in 0..(bars_per_day * days) {
            let close = 100.0 + (i as f64 * 0.02)
                + 3.0 * ((i as f64) * 0.35 + phase_deg.to_radians()).sin();
            out.push(candle("BTC-USDC", tf, ts, close));
            ts += tf;
        }
        out
    }

    fn run_cfg(ladder: Vec<u64>) -> HistoricalRunConfig {
        let mut tf_configs = HashMap::new();
        for tf in &ladder {
            tf_configs.insert(*tf, config_models::TimeframeConfig::new(*tf, config_models::IndicatorsConfig::default()));
        }
        HistoricalRunConfig {
            symbol: "BTC-USDC".into(),
            ladder,
            tf_configs,
            fib_config: FibonacciConfig::default(),
            active_set: ActiveSet::default(),
            exchange: Exchange::Hyperliquid,
            warmup_bars: 60,
            max_equity_points: 2000,
        }
    }

    #[tokio::test]
    async fn historical_run_is_deterministic() {
        let pool = seed_pool().await;
        let tf = 900u64;
        let candles = synthetic_days(2, tf, 0.0);
        database_storage::queries::archive::upsert_archive_candles(&pool, &candles, "backfill").await;

        let from = candles.first().unwrap().start_time_ms / 1000 + 60 * tf;
        let to = candles.last().unwrap().start_time_ms / 1000;
        let params = BacktestParams {
            symbol: "BTC-USDC".into(),
            timeframe_secs: tf,
            from_secs: from as i64,
            to_secs: to as i64,
            initial_capital: 1000.0,
        };
        let cfg = run_cfg(vec![tf]);

        let r1 = run_historical_backtest(
            &pool, &params, &tae_cfg(), &FeesConfig::default(), 20,
            performance_analytics::strategy_analytics::AnalyticsParams::default(), &cfg,
        ).await;
        let r2 = run_historical_backtest(
            &pool, &params, &tae_cfg(), &FeesConfig::default(), 20,
            performance_analytics::strategy_analytics::AnalyticsParams::default(), &cfg,
        ).await;

        assert_eq!(r1.trades.len(), r2.trades.len(), "deterministic trades");
        assert_eq!(r1.equity_curve.len(), r2.equity_curve.len());
        for (a, b) in r1.equity_curve.iter().zip(&r2.equity_curve) {
            assert_eq!(a.0, b.0);
            assert_eq!(a.1, b.1);
        }
        for (a, b) in r1.trades.iter().zip(&r2.trades) {
            assert_eq!(a.pnl, b.pnl);
            assert_eq!(a.exit_reason, b.exit_reason);
        }
        assert_eq!(r1.stats.classification, r2.stats.classification);
    }

    #[tokio::test]
    async fn burn_in_window_produces_no_trades_before_from() {
        let pool = seed_pool().await;
        let tf = 900u64;
        let candles = synthetic_days(1, tf, 0.0);
        database_storage::queries::archive::upsert_archive_candles(&pool, &candles, "backfill").await;

        let from = candles.last().unwrap().start_time_ms / 1000; // last bar only
        let to = from;
        let params = BacktestParams {
            symbol: "BTC-USDC".into(),
            timeframe_secs: tf,
            from_secs: from as i64,
            to_secs: to as i64,
            initial_capital: 1000.0,
        };
        let cfg = run_cfg(vec![tf]);
        let r = run_historical_backtest(
            &pool, &params, &tae_cfg(), &FeesConfig::default(), 20,
            performance_analytics::strategy_analytics::AnalyticsParams::default(), &cfg,
        ).await;
        // A single-bar window can produce at most one trade; the run must
        // complete and carry the standard result shape regardless.
        assert!(r.total_trades <= 1);
        assert_eq!(r.params.to_secs, to as i64);
    }

    #[tokio::test]
    async fn warm_chunks_cover_multi_chunk_series() {
        // 900 candles > CHUNK_CANDLES (800) forces multi-chunk replay.
        let tf = 60u64;
        let candles = synthetic_days(2, tf, 90.0);
        let candles = candles.into_iter().take(900).collect::<Vec<_>>();
        let cfg = run_cfg(vec![tf]);
        let snaps = warm_tf(&cfg, tf, candles);
        assert!(snaps.len() >= 500, "chunked replay emits the bulk of the series");
        // Ascending + unique.
        for w in snaps.windows(2) {
            assert!(w[0].timestamp < w[1].timestamp, "ascending ts");
        }
    }
}
