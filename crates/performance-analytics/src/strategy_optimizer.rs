use database_storage;
use sqlx::SqlitePool;
use tokio_util::sync::CancellationToken;

use core_domain::performance::{OptimizationReport, RegimePerformanceReport};

pub struct OptimizerConfig {
    pub pool: SqlitePool,
    pub cancel: CancellationToken,
    pub interval_secs: u64,
}

pub async fn run_strategy_optimizer(cfg: OptimizerConfig) {
    println!(
        "🧠 Strategy Optimizer: Started (interval: {}s)...",
        cfg.interval_secs
    );

    loop {
        tokio::select! {
            biased;
            _ = cfg.cancel.cancelled() => {
                println!("🛑 Strategy Optimizer: Cancelled, shutting down.");
                break;
            }
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(cfg.interval_secs)) => {}
        }

        let trades = database_storage::query_all_closed_trades(&cfg.pool).await;
        if trades.is_empty() {
            continue;
        }

        let mut by_regime: std::collections::HashMap<String, Vec<&database_storage::ClosedTradeRow>> =
            std::collections::HashMap::new();
        for t in &trades {
            let regime = t
                .market_regime
                .clone()
                .unwrap_or_else(|| "UNKNOWN".to_string());
            by_regime.entry(regime).or_default().push(t);
        }

        let mut regime_reports = Vec::new();
        let mut recommendations = Vec::new();

        for (regime, regime_trades) in &by_regime {
            let wins = regime_trades
                .iter()
                .filter(|t| t.realized_pnl > 0.0)
                .count();
            let win_rate = if !regime_trades.is_empty() {
                wins as f64 / regime_trades.len() as f64 * 100.0
            } else {
                0.0
            };

            let gross_profit: f64 = regime_trades
                .iter()
                .filter(|t| t.realized_pnl > 0.0)
                .map(|t| t.realized_pnl)
                .sum();
            let gross_loss: f64 = regime_trades
                .iter()
                .filter(|t| t.realized_pnl < 0.0)
                .map(|t| t.realized_pnl.abs())
                .sum();
            let profit_factor = if gross_loss > 0.0 {
                gross_profit / gross_loss
            } else {
                f64::INFINITY
            };

            let total_pnl: f64 = regime_trades.iter().map(|t| t.realized_pnl).sum();
            let valid_r_multiples: Vec<f64> = regime_trades
                .iter()
                .filter_map(|t| {
                    if t.allocated_usd > 0.0 {
                        Some(t.realized_pnl / t.allocated_usd)
                    } else {
                        None
                    }
                })
                .collect();
            let avg_r = if !valid_r_multiples.is_empty() {
                valid_r_multiples.iter().sum::<f64>() / valid_r_multiples.len() as f64
            } else {
                0.0
            };

            regime_reports.push(RegimePerformanceReport {
                regime: regime.clone(),
                trade_count: regime_trades.len() as i64,
                win_rate,
                profit_factor,
                avg_r_multiple: avg_r,
                total_pnl,
            });

            if win_rate < 35.0 && regime_trades.len() > 5 {
                recommendations.push(format!(
                    "REGIME {}: Low win rate ({:.1}%), consider reducing allocation or stricter entry filters",
                    regime, win_rate
                ));
            }
            if profit_factor < 1.0 && profit_factor.is_finite() && regime_trades.len() > 5 {
                recommendations.push(format!(
                    "REGIME {}: Profit factor {:.2} < 1.0 — trend-following in this regime may need review",
                    regime, profit_factor
                ));
            }
        }

        if regime_reports.len() > 1 {
            let best_regime = regime_reports.iter().max_by(|a, b| {
                a.profit_factor
                    .partial_cmp(&b.profit_factor)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            let worst_regime = regime_reports.iter().min_by(|a, b| {
                a.profit_factor
                    .partial_cmp(&b.profit_factor)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            if let (Some(best), Some(worst)) = (best_regime, worst_regime) {
                if best.profit_factor > 2.0 * worst.profit_factor
                    && best.profit_factor.is_finite()
                    && worst.profit_factor.is_finite()
                {
                    recommendations.push(format!(
                        "ALLOCATION BIAS: {} regime PF={:.2} vs {} regime PF={:.2} — consider favoring {} allocations",
                        best.regime, best.profit_factor, worst.regime, worst.profit_factor, best.regime
                    ));
                }
            }
        }

        let report = OptimizationReport {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64,
            total_trades: trades.len() as i64,
            regime_reports,
            recommendations,
        };

        database_storage::insert_optimization_report(&cfg.pool, &report).await;

        println!(
            "📊 Strategy Optimizer: Report persisted — {} trades, {} regimes analyzed",
            report.total_trades,
            by_regime.len()
        );

        for rec in &report.recommendations {
            println!("   📋 {}", rec);
        }
    }
}
