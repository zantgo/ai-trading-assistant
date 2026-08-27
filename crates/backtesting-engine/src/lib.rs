//! # Backtesting Engine (BTE)
//!
//! The sixth logical engine. The BTE replays the **same** MME/TAE/PME code
//! paths the live session runs, over two historical data sources:
//!
//! - **Recorded** — completed `market_snapshots` (with their persisted
//!   decision matrices) replayed through the unchanged setup executor
//!   ("record today, replay tomorrow").
//! - **Historical** — archived OHLCV candles (`candle_archive`, written by
//!   the live pipeline and the on-demand backfill job) fed through the full
//!   MME pipeline (indicators → normalizer → signal derivation → MTF
//!   synthesis) and then the executor — a true deep-history simulation.
//!
//! Parity contract (see
//! `docs/engines/backtesting-engine/08-04-parity-contract.md`): paper and
//! backtest share `portfolio_supervisor::execution::run_tick` — they differ
//! only in the tick source and config object, never in decision code.
//!
//! Modules:
//! - `backfill` — on-demand exchange pagination into `candle_archive`.
//! - `registry` — single-run lock + backfill progress registry.
//! - `recorded` — recorded-decision replay (moved from PAE L5).
//! - `historical` — full-pipeline replay over archived candles.
//! - `runner` — mode dispatch + DS persistence orchestration.

pub mod backfill;
pub mod historical;
pub mod recorded;
pub mod registry;
