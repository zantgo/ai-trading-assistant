// Performance Evaluator: scheduler stub.
//
// The forward-testing accuracy tracker (which tracked price deviation at
// 1h/4h/24h horizons for stored decision records) is reduced to a
// no-op stub. The per-trade performance signal can be re-attached when a
// non-decision-record source becomes available.

use tokio_util::sync::CancellationToken;

pub struct EvaluatorConfig {
    pub cancel: CancellationToken,
    pub eval_interval_secs: u64,
}

pub async fn run_performance_evaluator(cfg: EvaluatorConfig) {
    println!(
        "📊 Performance Evaluator: Stub running (interval: {}s)...",
        cfg.eval_interval_secs
    );
    cfg.cancel.cancelled().await;
    println!("🛑 Performance Evaluator: Terminated.");
}
