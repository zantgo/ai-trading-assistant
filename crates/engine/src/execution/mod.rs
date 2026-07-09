/// Execution subsystem: order management, position reconciliation, and trade safety.
/// Phase 4 — Real Exchange Execution (extends paper trading to live Hyperliquid orders).
pub mod order_manager;
pub mod position_reconciler;
pub mod exec_safety;
/// Phase 17 — Algorithmic execution: TWAP, VWAP, Implementation Shortfall.
pub mod algo;
