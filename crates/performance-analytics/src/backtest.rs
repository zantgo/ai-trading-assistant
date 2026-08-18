//! PAE Layer 5 — Backtest.
//!
//! Deterministic replay of **recorded MME decisions** through the unchanged
//! TAE setup executor + unified paper engine: completed `market_snapshots`
//! (with their persisted `opportunity_json` / `decision_context_json` /
//! `analysis_json` / `advisory_json` matrices) are fed in time order to the
//! same code the live executor runs. The simulated trades then get the full
//! NHST treatment (t-test, 10k Monte Carlo, α = 0.05, edge classification).
//!
//! See docs/engines/performance-analytics-engine/03-05-06-pae-layer5-backtest.md.

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

use crate::strategy_analytics::compute_setup_analytics;

/// Backtest window + capital parameters (the API request body).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestParams {
    pub symbol: String,
    pub timeframe_secs: u64,
    pub from_ms: i64,
    pub to_ms: i64,
    #[serde(default = "default_capital")]
    pub initial_capital: f64,
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
    /// NHST block (t-statistic, p-value, Monte Carlo p, α, significance,
    /// edge classification) over the simulated trades.
    pub stats: StrategyAnalyticsRow,
    pub trades: Vec<BacktestTrade>,
    /// `(timestamp_ms, total_equity)` sampled per snapshot (downsampled).
    pub equity_curve: Vec<(i64, f64)>,
}

/// Maximum number of recorded snapshots replayed per run (keeps the
/// synchronous endpoint bounded).
pub const BACKTEST_MAX_SNAPSHOTS: u32 = 50_000;

fn snap_to_market(symbol: &str, rec: &RecordedSnapshot) -> core_domain::models::MarketSnapshot {
    let mid = Decimal::from_f64_retain(rec.mid_price).unwrap_or_default();
    let mut snap = core_domain::models::MarketSnapshot::default();
    snap.symbol = symbol.to_string();
    snap.timeframe_secs = rec.timeframe_secs as u64;
    snap.timestamp = rec.timestamp as u64;
    snap.is_completed = Some(true);
    snap.mid_price = mid;
    snap.bid_price = mid;
    snap.ask_price = mid;
    snap.close = rec.close.and_then(|c| Decimal::from_f64_retain(c));
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
) -> BacktestResult {
    let records = database_storage::queries::snapshots::query_backtest_snapshots(
        pool,
        &params.symbol,
        params.timeframe_secs,
        params.from_ms,
        params.to_ms,
        BACKTEST_MAX_SNAPSHOTS,
    )
    .await;

    let engine = ExecutionEngine::new(fees.clone());
    engine
        .set_initial_equity(Decimal::from_f64_retain(params.initial_capital).unwrap_or(dec!(1000)))
        .await;
    engine.set_cross_leverage(cross_leverage).await;
    let engine = std::sync::Arc::new(engine);
    let executor = SetupExecutor::new(engine.clone(), tae_cfg);

    let mut trades: Vec<BacktestTrade> = Vec::new();
    let mut equity_points: Vec<(i64, f64)> = Vec::new();

    for rec in &records {
        let snap = snap_to_market(&params.symbol, rec);
        let mid = snap.mid_price;

        let prev_position = engine.get_position(&params.symbol).await;

        if prev_position.is_some() {
            engine.mark_to_market(&params.symbol, mid).await;
        }
        engine.evaluate_order_fills(mid).await;

        // Record the close (the executor consumes `take_last_close` — the
        // None here simply means we already took it).
        if let Some(outcome) = engine.take_last_close(&params.symbol).await {
            if let Some(pos) = &prev_position {
                let direction = match pos.direction {
                    config_models::Direction::Long => "LONG",
                    config_models::Direction::Short => "SHORT",
                };
                let entry = pos.entry_price.to_f64().unwrap_or(0.0);
                let size = pos.size.to_f64().unwrap_or(0.0);
                let exit = if size > 0.0 {
                    if pos.direction == config_models::Direction::Long {
                        entry + outcome.pnl.to_f64().unwrap_or(0.0) / size
                    } else {
                        entry - outcome.pnl.to_f64().unwrap_or(0.0) / size
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
                    pnl: outcome.pnl.to_f64().unwrap_or(0.0),
                    exit_reason: outcome.exit_reason.clone(),
                });
            }
        }

        executor
            .tick(
                "backtest",
                &params.symbol,
                vec![&snap],
                mid,
                TickContext {
                    safety_allows_entry: true,
                    lifecycle_running: true,
                    candle_ts: rec.timestamp as u64,
                    safety: None,
                },
            )
            .await;

        // Equity curve sample: ledger + unrealized.
        let unrealized: Decimal = {
            let positions = engine.positions.read().await;
            positions.values().map(|p| p.unrealized_pnl).sum()
        };
        let total = engine.get_equity_decimal().await + unrealized;
        equity_points.push((rec.timestamp, total.to_f64().unwrap_or(0.0)));
    }

    // Down-sample the equity curve to ≤ 2000 points.
    if equity_points.len() > 2000 {
        let step = equity_points.len() / 2000;
        equity_points = equity_points.iter().step_by(step.max(1)).cloned().collect();
        if let Some(last) = equity_points.last().cloned() {
            equity_points.push(last);
        }
    }

    // ── Classic + NHST statistics over the simulated trades ──
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
    let stats = compute_setup_analytics("BACKTEST", &records.iter().collect::<Vec<_>>());

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
        stats,
        trades,
        equity_curve: equity_points,
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
            risk_per_trade_pct: 1.0,
            min_net_rr: 1.0,
            max_position_size_usd: None,
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
            from_ms: 0,
            to_ms: 10_000,
            initial_capital: 1000.0,
        };
        let result = run_backtest(&pool, &params, &tae_cfg(), &FeesConfig::default(), 20).await;

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
        let mut snap = MarketSnapshot::default();
        snap.symbol = "BTC-USDC".to_string();
        snap.timeframe_secs = 60;
        snap.timestamp = 1000;
        snap.is_completed = Some(true);
        snap.mid_price = dec!(100);
        snap.bid_price = dec!(100);
        snap.ask_price = dec!(100);
        snap.close = Some(dec!(100));
        database_storage::insert_snapshot_internal(&pool, &snap).await;

        let params = BacktestParams {
            symbol: "BTC-USDC".to_string(),
            timeframe_secs: 60,
            from_ms: 0,
            to_ms: 10_000,
            initial_capital: 1000.0,
        };
        let result = run_backtest(&pool, &params, &tae_cfg(), &FeesConfig::default(), 20).await;
        assert_eq!(result.total_trades, 0);
        assert_eq!(result.equity_curve.len(), 1);
        assert_eq!(
            result.stats.classification,
            core_domain::performance::PerformanceClassification::InsufficientData
        );
    }
}
