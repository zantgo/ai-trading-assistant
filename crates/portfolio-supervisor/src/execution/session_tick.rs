//! The shared per-tick session body — the structural parity contract
//! between the live session, paper trading, and the Backtesting Engine.
//!
//! Every tick of the TAE loop runs the **same** sequence through the
//! **same** code, no matter where the snapshot came from:
//!
//! ```text
//! mark_to_market (if a position is open)
//!   → fills branch (live: poll+apply external; paper/backtest: evaluate)
//!   → SetupExecutor::tick
//! ```
//!
//! The daemon drives this with live WS snapshots; the backtest runner
//! drives it with recorded/archived snapshots. Fills, sizing, safety
//! ladder and fees are the same functions on the same config — so a
//! backtest result equals the paper result by construction (live differs
//! only by venue fills/latency). See
//! `docs/engines/backtesting-engine/08-04-parity-contract.md`.

use core_domain::models::MarketSnapshot;
use rust_decimal::Decimal;
use std::sync::Arc;

use super::engine::{CloseOutcome, ExecutionEngine};
use crate::setup_executor::{SetupExecutor, TickContext};

/// What the tick did (extension point for call-site bookkeeping; the
/// execution itself is identical everywhere).
#[derive(Debug, Default, Clone)]
pub struct SessionTickOutcome {
    /// The candle timestamp the executor saw (`max` over the snapshots).
    pub candle_ts: u64,
    /// Equity after the tick (ledger only; call sites add unrealized).
    pub equity: f64,
    /// The close outcome observed between fills and the executor tick
    /// (only populated when `capture_last_close` is set — the backtest
    /// reads it before the executor's own consumption).
    pub last_close: Option<CloseOutcome>,
    /// Direction of the closed position (for the backtest trade log).
    pub last_close_direction: Option<config_models::Direction>,
    /// Size of the closed position (for the backtest trade log).
    pub last_close_size: Option<Decimal>,
    /// Entry price of the closed position (for the backtest trade log).
    pub last_close_entry: Option<Decimal>,
}

/// Run one session tick. `dispatch` is decided by the call site
/// (observe → false; paper/backtest/live → true); `live_fills` swaps the
/// fill source for live-mode engines (the daemon polls the broker);
/// `capture_last_close` snapshots the close outcome after fills and
/// before the executor consumes it (backtest trade log).
pub async fn run_tick(
    engine: &Arc<ExecutionEngine>,
    executor: &SetupExecutor,
    instance_id: &str,
    symbol: &str,
    snaps: &[&MarketSnapshot],
    mid: Decimal,
    ctx: TickContext,
    live_fills: Option<Vec<crate::execution::backend::Fill>>,
    capture_last_close: bool,
) -> SessionTickOutcome {
    let candle_ts = snaps.iter().map(|s| s.timestamp).max().unwrap_or(0);

    let mut outcome = SessionTickOutcome {
        candle_ts,
        ..SessionTickOutcome::default()
    };

    // The closed-position context (captured before mark-to-market).
    let prev_position = engine.get_position(symbol).await;

    // Mark-to-market (live unrealized PnL for display).
    if prev_position.is_some() {
        engine.mark_to_market(symbol, mid).await;
    }

    // Fills first (entry, TP, SL), then the state machine.
    if ctx.dispatch {
        match live_fills {
            Some(fills) => {
                engine.apply_external_fills(fills).await;
            }
            None => {
                engine.evaluate_order_fills(mid).await;
            }
        }
    }

    // The backtest captures the close (TP/SL fill) before the executor's
    // tick_position consumes it; the daemon never sets this flag.
    if capture_last_close {
        if let Some(close) = engine.take_last_close(symbol).await {
            if let Some(pos) = &prev_position {
                outcome.last_close_direction = Some(pos.direction);
                outcome.last_close_size = Some(pos.size);
                outcome.last_close_entry = Some(pos.entry_price);
            }
            outcome.last_close = Some(close);
        }
    }

    executor
        .tick(instance_id, symbol, snaps.to_vec(), mid, ctx)
        .await;

    outcome.equity = engine.get_equity().await;
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paper_trading::FeesConfig;
    use config_models::MinimalTaeConfig;
    use rust_decimal_macros::dec;

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

    fn plain_snapshot(ts: u64, mid: f64) -> MarketSnapshot {
        MarketSnapshot {
            symbol: "BTC-USDC".to_string(),
            timeframe_secs: 60,
            timestamp: ts,
            is_completed: Some(true),
            mid_price: Decimal::from_f64_retain(mid).unwrap(),
            bid_price: Decimal::from_f64_retain(mid).unwrap(),
            ask_price: Decimal::from_f64_retain(mid).unwrap(),
            close: Some(Decimal::from_f64_retain(mid).unwrap()),
            ..MarketSnapshot::default()
        }
    }

    #[tokio::test]
    async fn run_tick_is_deterministic_and_idempotent_across_engines() {
        // Two fresh engines driven by the same tick sequence must produce
        // byte-identical outcomes — the backtest determinism contract.
        for _ in 0..2 {
            let engine = Arc::new(ExecutionEngine::new(FeesConfig::default()));
            engine.set_initial_equity(dec!(1000)).await;
            let executor = SetupExecutor::new(engine.clone(), &tae_cfg());

            for (ts, mid) in [(1u64, 100.0f64), (2, 100.0), (3, 100.0)] {
                let snap = plain_snapshot(ts, mid);
                let _outcome = run_tick(
                    &engine,
                    &executor,
                    "test",
                    "BTC-USDC",
                    &[&snap],
                    snap.mid_price,
                    TickContext {
                        safety_allows_entry: true,
                        lifecycle_running: true,
                        candle_ts: ts,
                        safety: None,
                        dispatch: true,
                    },
                    None,
                    true,
                )
                .await;
            }
            assert_eq!(engine.get_equity().await, 1000.0, "no fills without decision matrices");
        }
    }

    #[tokio::test]
    async fn run_tick_respects_dispatch_gate() {
        let engine = Arc::new(ExecutionEngine::new(FeesConfig::default()));
        engine.set_initial_equity(dec!(1000)).await;
        let executor = SetupExecutor::new(engine.clone(), &tae_cfg());
        let snap = plain_snapshot(1, 100.0);
        let outcome = run_tick(
            &engine,
            &executor,
            "test",
            "BTC-USDC",
            &[&snap],
            snap.mid_price,
            TickContext {
                safety_allows_entry: true,
                lifecycle_running: true,
                candle_ts: 1,
                safety: None,
                dispatch: false,
            },
            None,
            false,
        )
        .await;
        assert_eq!(outcome.candle_ts, 1);
        assert_eq!(engine.get_equity().await, 1000.0);
    }
}
