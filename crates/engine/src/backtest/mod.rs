pub mod engine;
pub mod metrics;
pub mod walk_forward;

pub use engine::{BacktestEngine, BacktestResult, BacktestTrade};
pub use metrics::{
    compute_cagr, compute_max_drawdown, compute_sharpe, compute_sortino, deflated_sharpe,
};
pub use walk_forward::WalkForwardOptimizer;
