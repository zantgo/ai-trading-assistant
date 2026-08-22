//! BTE recorded-decision replay (moved from PAE L5).
//!
//! Deterministic replay of **recorded MME decisions** through the unchanged
//! TAE setup executor + unified paper engine: completed `market_snapshots`
//! (with their persisted `opportunity_json` / `decision_context_json` /
//! `analysis_json` / `advisory_json` matrices) are fed in time order to the
//! same code the live executor runs. The simulated trades then get the full
//! NHST treatment (t-test, 10k Monte Carlo, α = 0.05, edge classification).
//!
//! See docs/engines/backtesting-engine/08-03-historical-runner.md.

use config_models::MinimalTaeConfig;
use core_domain::performance::{StrategyAnalyticsRow, TradeAnalyticsRecord};
use database_storage::queries::snapshots::RecordedSnapshot;
use portfolio_supervisor::execution::ExecutionEngine;
use portfolio_supervisor::paper_trading::FeesConfig;
use portfolio_supervisor::setup_executor::{SetupExecutor, TickContext};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::sync::Arc;

use performance_analytics::strategy_analytics::compute_setup_analytics;

/// Backtest window + capital parameters.
///
/// Unit contract: the API accepts milliseconds (`from_ms` / `to_ms`) and the
/// gateway converts to **Unix seconds** before constructing these params —
/// `market_snapshots.timestamp` and the candle archive are stored in seconds.
/// `from_secs`/`to_secs` are inclusive bounds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestParams {
    pub symbol: String,
    pub timeframe_secs: u64,
    pub from_secs: i64,
    pub to_secs: i64,
    #[serde(default = "default_capital")]
    #[serde(alias = "initial_capital")]
    /// v9 (F-07): ONE capital dial — the simulated account seed
    /// (`portfolio_capital_usd`, same field name as paper/live).
    pub portfolio_capital_usd: f64,
}

fn default_capital() -> f64 {
    1000.0
}

/// One simulated close.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestTrade {
    pub timestamp: i64,
    pub direction: String,
    pub entry_price: f64,
    pub exit_price: f64,
    pub size: f64,
    pub pnl: f64,
    pub exit_reason: String,
}

/// The full backtest result: classic metrics + the NHST block + trades +
/// equity curve. Persisted to `backtest_runs`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestResult {
    pub params: BacktestParams,
    pub total_trades: u32,
    pub win_count: u32,
    pub loss_count: u32,
    pub win_rate: f64,
    pub gross_profit: f64,
    pub gross_loss: f64,
    pub profit_factor: Option<f64>,
    pub expectancy: f64,
    pub max_drawdown_pct: f64,
    /// v8.2: true when the run was aborted via the cancel flag (no
    /// persistence should happen for cancelled runs).
    #[serde(default)]
    pub cancelled: bool,
    /// NHST block (t-statistic, p-value, Monte Carlo p, α, significance,
    /// edge classification) over the simulated trades.
    pub stats: StrategyAnalyticsRow,
    pub trades: Vec<BacktestTrade>,
    /// `(timestamp_ms, total_equity)` sampled per snapshot (downsampled).
    pub equity_curve: Vec<(i64, f64)>,
    /// Per-tick synthesized decision snapshots (DS persistence; not on
    /// the wire).
    #[serde(default, skip_serializing)]
    pub signals: Vec<database_storage::queries::backtest_ds::DsSignal>,
    /// Per-tick capital/exposure/drawdown samples (DS persistence; not on
    /// the wire).
    #[serde(default, skip_serializing)]
    pub portfolio: Vec<database_storage::queries::backtest_ds::DsPortfolioPoint>,
}

/// Maximum number of recorded snapshots replayed per run (keeps the
/// synchronous endpoint bounded).
pub const BACKTEST_MAX_SNAPSHOTS: u32 = 50_000;

fn snap_to_market(symbol: &str, rec: &RecordedSnapshot) -> core_domain::models::MarketSnapshot {
    let mid = Decimal::from_f64_retain(rec.mid_price).unwrap_or_default();
    let mut snap = core_domain::models::MarketSnapshot {
        symbol: symbol.to_string(),
        timeframe_secs: rec.timeframe_secs as u64,
        timestamp: rec.timestamp as u64,
        is_completed: Some(true),
        mid_price: mid,
        bid_price: mid,
        ask_price: mid,
        close: rec.close.and_then(Decimal::from_f64_retain),
        ..core_domain::models::MarketSnapshot::default()
    };
    snap.opportunity = rec
        .opportunity_json
        .as_deref()
        .and_then(|j| serde_json::from_str(j).ok());
    snap.decision_context = rec
        .decision_context_json
        .as_deref()
        .and_then(|j| serde_json::from_str(j).ok());
    snap.analysis = rec
        .analysis_json
        .as_deref()
        .and_then(|j| serde_json::from_str(j).ok());
    snap.advisory = rec
        .advisory_json
        .as_deref()
        .and_then(|j| serde_json::from_str(j).ok());
    snap
}

/// Run a backtest over recorded snapshots with the unchanged executor +
/// unified paper engine. Deterministic: identical inputs ⇒ identical result.
pub async fn run_backtest(
    pool: &SqlitePool,
    params: &BacktestParams,
    tae_cfg: &MinimalTaeConfig,
    fees: &FeesConfig,
    cross_leverage: u32,
    analytics: performance_analytics::strategy_analytics::AnalyticsParams,
) -> BacktestResult {
    let records = database_storage::queries::snapshots::query_backtest_snapshots(
        pool,
        &params.symbol,
        params.timeframe_secs,
        params.from_secs,
        params.to_secs,
        BACKTEST_MAX_SNAPSHOTS,
    )
    .await;

    let engine = ExecutionEngine::new(fees.clone());
    engine
        .set_initial_equity(Decimal::from_f64_retain(params.portfolio_capital_usd).unwrap_or(dec!(1000)))
        .await;
    engine.set_cross_leverage(cross_leverage).await;
    let engine = std::sync::Arc::new(engine);
    let executor = SetupExecutor::new(engine.clone(), tae_cfg);

    let mut trades: Vec<BacktestTrade> = Vec::new();
    let mut equity_points: Vec<(i64, f64)> = Vec::new();
    let mut signals: Vec<database_storage::queries::backtest_ds::DsSignal> = Vec::new();
    let mut portfolio: Vec<database_storage::queries::backtest_ds::DsPortfolioPoint> = Vec::new();
    let mut peak_equity: f64 = params.portfolio_capital_usd;

    for rec in &records {
        let snap = snap_to_market(&params.symbol, rec);
        let mid = snap.mid_price;

        // BTE v8 parity: the replay drives the SAME per-tick session body
        // the daemon runs (`run_tick`); `capture_last_close` snapshots the
        // close outcome between fills and the executor tick so the trade
        // log records every simulated close. The executor consumes
        // `take_last_close` afterwards exactly as it does live.
        let outcome = portfolio_supervisor::execution::session_tick::run_tick(
            &engine,
            &executor,
            "backtest",
            &params.symbol,
            &[&snap],
            mid,
            TickContext {
                safety_allows_entry: true,
                lifecycle_running: true,
            market_filter_allows_entry: true,
            entry_block_reason: None,
                candle_ts: rec.timestamp as u64,
                safety: None,
                dispatch: true,
                allocation_pct: None,
                strategy: None,
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
                timestamp: rec.timestamp,
                direction: direction.to_string(),
                entry_price: entry,
                exit_price: exit,
                size,
                pnl: close.pnl.to_f64().unwrap_or(0.0),
                exit_reason: close.exit_reason.clone(),
            });
        }

        // Equity curve sample: ledger + unrealized.
        let unrealized: Decimal = {
            let positions = engine.positions.read().await;
            positions.values().map(|p| p.unrealized_pnl).sum()
        };
        let total = engine.get_equity_decimal().await + unrealized;
        equity_points.push((rec.timestamp, total.to_f64().unwrap_or(0.0)));

        capture_tick_ds(
            rec.timestamp,
            params.timeframe_secs,
            &snap,
            &engine,
            &mut signals,
            &mut portfolio,
            &mut peak_equity,
        )
        .await;
    }

    // ── Classic + NHST statistics over the simulated trades ──
    finalize_result(
        params,
        trades,
        equity_points,
        2000,
        analytics,
        signals,
        portfolio,
    )
}

/// Shared per-tick DS capture for both runners: decision snapshots +
/// the capital/exposure/drawdown sample.
pub async fn capture_tick_ds(
    ts_secs: i64,
    timeframe_secs: u64,
    snap: &core_domain::models::MarketSnapshot,
    engine: &Arc<ExecutionEngine>,
    signals: &mut Vec<database_storage::queries::backtest_ds::DsSignal>,
    portfolio: &mut Vec<database_storage::queries::backtest_ds::DsPortfolioPoint>,
    peak_equity: &mut f64,
) {
    if let Some(dec) = snap.decision_context.as_ref() {
        signals.push(database_storage::queries::backtest_ds::DsSignal {
            ts_secs,
            timeframe_secs,
            label: "decision".to_string(),
            kind: "bias".to_string(),
            value: dec.bias.clone(),
        });
        signals.push(database_storage::queries::backtest_ds::DsSignal {
            ts_secs,
            timeframe_secs,
            label: "decision".to_string(),
            kind: "trade_readiness".to_string(),
            value: dec.trade_readiness.clone(),
        });
    }
    if let Some(opp) = snap.opportunity.as_ref() {
        signals.push(database_storage::queries::backtest_ds::DsSignal {
            ts_secs,
            timeframe_secs,
            label: "opportunity".to_string(),
            kind: "score".to_string(),
            value: format!("{:.2}", opp.opportunity_score),
        });
    }

    let (margin_used, unrealized): (Decimal, Decimal) = {
        let positions = engine.positions.read().await;
        let margin: Decimal = positions.values().map(|p| p.size * p.entry_price).sum();
        let unreal: Decimal = positions.values().map(|p| p.unrealized_pnl).sum();
        (margin, unreal)
    };
    let ledger = engine.get_equity_decimal().await;
    let equity = ledger + unrealized;
    let equity_f = equity.to_f64().unwrap_or(0.0);
    let initial = *peak_equity;
    if equity_f > *peak_equity {
        *peak_equity = equity_f;
    }
    let drawdown_pct = if *peak_equity > 0.0 {
        ((*peak_equity - equity_f) / *peak_equity * 100.0).max(0.0)
    } else {
        0.0
    };
    let margin_f = margin_used.to_f64().unwrap_or(0.0);
    let exposure_pct = if initial > 0.0 {
        (margin_f / initial * 100.0).max(0.0)
    } else {
        0.0
    };
    portfolio.push(database_storage::queries::backtest_ds::DsPortfolioPoint {
        ts_secs,
        equity: equity_f,
        cash: ledger.to_f64().unwrap_or(0.0),
        margin_used: margin_f,
        exposure_pct,
        drawdown_pct,
        positions_open: engine.positions.read().await.len() as u32,
    });
}

/// Shared result assembly for the recorded and historical runners:
/// equity downsampling + classic metrics + the NHST block.
pub fn finalize_result(
    params: &BacktestParams,
    trades: Vec<BacktestTrade>,
    mut equity_points: Vec<(i64, f64)>,
    max_equity_points: u32,
    analytics: performance_analytics::strategy_analytics::AnalyticsParams,
    signals: Vec<database_storage::queries::backtest_ds::DsSignal>,
    portfolio: Vec<database_storage::queries::backtest_ds::DsPortfolioPoint>,
) -> BacktestResult {
    // Down-sample the equity curve to the configured cap.
    let cap = max_equity_points.max(10) as usize;
    if equity_points.len() > cap {
        let step = equity_points.len() / cap;
        equity_points = equity_points.iter().step_by(step.max(1)).cloned().collect();
        if equity_points.len() < 2 {
            if let Some(last) = equity_points.last().cloned() {
                equity_points.push(last);
            }
        }
    }

    let total = trades.len() as u32;
    let win_count = trades.iter().filter(|t| t.pnl > 0.0).count() as u32;
    let loss_count = total - win_count;
    let win_rate = if total > 0 {
        win_count as f64 / total as f64 * 100.0
    } else {
        0.0
    };
    let gross_profit: f64 = trades.iter().filter(|t| t.pnl > 0.0).map(|t| t.pnl).sum();
    let gross_loss: f64 = trades
        .iter()
        .filter(|t| t.pnl < 0.0)
        .map(|t| t.pnl.abs())
        .sum();
    let profit_factor = if gross_loss > 0.0 {
        Some(gross_profit / gross_loss)
    } else if gross_profit > 0.0 {
        Some(1_000_000.0)
    } else {
        None
    };
    let expectancy = if total > 0 {
        trades.iter().map(|t| t.pnl).sum::<f64>() / total as f64
    } else {
        0.0
    };

    let max_drawdown_pct = max_drawdown(&equity_points);

    let records: Vec<TradeAnalyticsRecord> = trades
        .iter()
        .map(|t| TradeAnalyticsRecord {
            trade_id: format!("bt_{}_{}", t.timestamp, t.direction),
            symbol: params.symbol.clone(),
            direction: t.direction.clone(),
            entry_timestamp: t.timestamp,
            exit_timestamp: t.timestamp,
            hold_time_seconds: 0,
            entry_price: t.entry_price,
            exit_price: t.exit_price,
            size: t.size,
            gross_pnl: t.pnl,
            net_pnl: t.pnl,
            roi_pct: 0.0,
            execution_slippage: 0.0,
            mfe: 0.0,
            mae: 0.0,
            trigger_source: "BACKTEST".to_string(),
            exit_reason: t.exit_reason.clone(),
            flat_trade: false,
        })
        .collect();
    let stats = compute_setup_analytics("BACKTEST", &records.iter().collect::<Vec<_>>(), analytics);

    BacktestResult {
        params: params.clone(),
        total_trades: total,
        win_count,
        loss_count,
        win_rate,
        gross_profit,
        gross_loss,
        profit_factor,
        expectancy,
        max_drawdown_pct,
        cancelled: false,
        stats,
        trades,
        equity_curve: equity_points,
        signals,
        portfolio,
    }
}

fn max_drawdown(points: &[(i64, f64)]) -> f64 {
    if points.is_empty() {
        return 0.0;
    }
    let mut peak = points[0].1;
    let mut max_dd = 0.0;
    for (_, equity) in points {
        if *equity > peak {
            peak = *equity;
        }
        if peak > 0.0 {
            let dd = (peak - equity) / peak * 100.0;
            if dd > max_dd {
                max_dd = dd;
            }
        }
    }
    max_dd
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_domain::analysis::{
        AnalysisMatrix, MarketBias, OpportunityProfile, OpportunityType, PriceRange, SetupQuality,
        TradeViability,
    };
    use core_domain::decision_context::DecisionContext;
    use core_domain::models::MarketSnapshot;
    use core_domain::opportunity::OpportunityMatrix;
    use core_domain::risk::{RiskDimension, RiskLevel, RiskState};
    use rust_decimal_macros::dec;

    fn decision(rr: f64) -> DecisionContext {
        DecisionContext {
            score: 60.0,
            bias: "Bullish".to_string(),
            score_confidence: 0.6,
            entry_danger: RiskDimension {
                score: 20.0,
                level: RiskLevel::Low,
                state: RiskState::Stable,
                confidence: 80.0,
                evidence: vec![],
                volatility_to_spread_ratio: None,
            },
            expected_reward_risk_ratio: rr,
            trade_readiness: "READY".to_string(),
            contributing_indicators: vec![],
            long_probability: 60.0,
            short_probability: 30.0,
            hold_probability: 10.0,
            net_bias_pct: 30.0,
            lean_floor_applied: false,
        }
    }

    fn long_snapshot(ts: u64, mid: f64) -> MarketSnapshot {
        let mut snap = MarketSnapshot::default();
        snap.symbol = "BTC-USDC".to_string();
        snap.timeframe_secs = 60;
        snap.timestamp = ts;
        snap.is_completed = Some(true);
        snap.mid_price = Decimal::from_f64_retain(mid).unwrap();
        snap.bid_price = snap.mid_price;
        snap.ask_price = snap.mid_price;
        snap.close = Some(snap.mid_price);

        let mut a = AnalysisMatrix::empty("BTC-USDC");
        a.bias = MarketBias::Bullish;
        snap.analysis = Some(a);
        snap.decision_context = Some(decision(2.0));
        snap.opportunity = Some(OpportunityMatrix {
            symbol: "BTC-USDC".to_string(),
            primary_opportunity: OpportunityType::TrendContinuation,
            opportunity_score: 60.0,
            setup_quality: SetupQuality::Strong,
            profiles: vec![OpportunityProfile {
                opportunity_type: OpportunityType::TrendContinuation,
                score: 80.0,
                preconditions_met: 4,
                preconditions_total: 4,
                notes: String::new(),
                direction_family: None,
                long_entry_zone: Some(PriceRange {
                    low: 90.0,
                    high: 100.0,
                }),
                long_target_zone: Some(PriceRange {
                    low: 120.0,
                    high: 130.0,
                }),
                long_invalidation_level: Some(85.0),
                short_entry_zone: None,
                short_target_zone: None,
                short_invalidation_level: None,
                long_expected_rr_internal: 2.0,
                short_expected_rr_internal: 0.0,
                trade_viability: Some(TradeViability::Actionable),
                long_geometry_consistent: true,
                short_geometry_consistent: false,
                scoring_factors: None,
                display_score: Some(80.0),
            }],
            forecast_confidence: 0.7,
            contributing_signals: vec![],
            invalidation_note: String::new(),
            entry_zone: PriceRange {
                low: 90.0,
                high: 100.0,
            },
            target_zone: PriceRange {
                low: 120.0,
                high: 130.0,
            },
            time_horizon: "SWING".to_string(),
            long_entry_zone: PriceRange {
                low: 90.0,
                high: 100.0,
            },
            long_target_zone: PriceRange {
                low: 120.0,
                high: 130.0,
            },
            long_invalidation_level: 85.0,
            short_entry_zone: PriceRange {
                low: 0.0,
                high: 0.0,
            },
            short_target_zone: PriceRange {
                low: 0.0,
                high: 0.0,
            },
            short_invalidation_level: 0.0,
            long_expected_rr_internal: 2.0,
            short_expected_rr_internal: 0.0,
            long_gross_rr_internal: 2.0,
            short_gross_rr_internal: 0.0,
            invalidation_level: 85.0,
            direction_family: None,
            long_geometry_consistent: true,
            short_geometry_consistent: false,
            neutral_reference_bracket: None,
            confluent_entry_levels: vec![],
            confluent_target_levels: vec![],
            confluent_invalidation_levels: vec![],
        });
        snap
    }

    async fn seed_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("mem pool");
        database_storage::run_migrations(&pool)
            .await
            .expect("migrations");
        pool
    }

    fn tae_cfg() -> MinimalTaeConfig {
        MinimalTaeConfig {
            enabled: true,
            allocation_pct: 10.0,
            min_net_rr: 1.0,
            max_position_size_pct_of_equity: None,
            max_open_positions: 1,
            entry_mode: "zone_midpoint".to_string(),
            invalidate_on: "direction_flip".to_string(),
        }
    }

    #[tokio::test]
    async fn replay_records_trade_and_stats() {
        let pool = seed_pool().await;
        // Accept at 105 → fill at 94 → TP at 126.
        for (ts, mid) in [(1000u64, 105.0f64), (1001, 94.0), (1002, 126.0)] {
            database_storage::insert_snapshot_internal(&pool, &long_snapshot(ts, mid)).await;
        }

        let params = BacktestParams {
            symbol: "BTC-USDC".to_string(),
            timeframe_secs: 60,
            from_secs: 0,
            to_secs: 10_000,
            portfolio_capital_usd: 1000.0,
        };
        let result = run_backtest(
            &pool,
            &params,
            &tae_cfg(),
            &FeesConfig::default(),
            20,
            performance_analytics::strategy_analytics::AnalyticsParams::default(),
        )
        .await;

        assert_eq!(result.total_trades, 1, "one simulated close expected");
        assert_eq!(result.win_count, 1);
        assert!(result.profit_factor.unwrap() > 1.0);
        assert_eq!(result.equity_curve.len(), 3);
        assert!(result.max_drawdown_pct >= 0.0);
        assert_eq!(result.trades[0].exit_reason, "tp");
        // NHST block present with the fixed α.
        assert_eq!(result.stats.alpha, 0.05);
        assert!(result.stats.total_trades >= 1);
    }

    #[tokio::test]
    async fn no_setups_yields_empty_result() {
        let pool = seed_pool().await;
        // Snapshots without any decision matrix → nothing to trade.
        let snap = MarketSnapshot {
            symbol: "BTC-USDC".to_string(),
            timeframe_secs: 60,
            timestamp: 1000,
            is_completed: Some(true),
            mid_price: dec!(100),
            bid_price: dec!(100),
            ask_price: dec!(100),
            close: Some(dec!(100)),
            ..MarketSnapshot::default()
        };
        database_storage::insert_snapshot_internal(&pool, &snap).await;

        let params = BacktestParams {
            symbol: "BTC-USDC".to_string(),
            timeframe_secs: 60,
            from_secs: 0,
            to_secs: 10_000,
            portfolio_capital_usd: 1000.0,
        };
        let result = run_backtest(
            &pool,
            &params,
            &tae_cfg(),
            &FeesConfig::default(),
            20,
            performance_analytics::strategy_analytics::AnalyticsParams::default(),
        )
        .await;
        assert_eq!(result.total_trades, 0);
        assert_eq!(result.equity_curve.len(), 1);
        assert_eq!(
            result.stats.classification,
            core_domain::performance::PerformanceClassification::InsufficientData
        );
    }
}
