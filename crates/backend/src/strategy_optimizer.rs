// Strategy Optimizer: scheduler stub.
//
// Per-regime trade analytics depended on the removed `paper_trades` table
// and `ClosedTradeRow` type. With the paper-trading matching engine gone,
// there is no closed-trade dataset to analyze. The optimizer retains its
// scheduler loop so the platform can re-task it when an automated trade
// source becomes available.

use tokio_util::sync::CancellationToken;

pub struct OptimizerConfig {
    pub cancel: CancellationToken,
    pub interval_secs: u64,
}

pub async fn run_strategy_optimizer(cfg: OptimizerConfig) {
    println!(
        "🧠 Strategy Optimizer: No-op stub running (interval: {}s)...",
        cfg.interval_secs
    );
    cfg.cancel.cancelled().await;
    println!("🛑 Strategy Optimizer: Terminated.");
}
