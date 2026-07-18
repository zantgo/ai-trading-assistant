use core_domain::performance::{PerformanceMatrixRow, RegimeCompatibility, TradeAnalyticsRecord};
use sqlx::SqlitePool;

/// Compute regime-performance matrix by joining trade data with market snapshots.
/// Implements docs:03-05-05-pae-layer4-performance.md
pub async fn compute_performance_matrix(
    pool: &SqlitePool,
    trades: &[TradeAnalyticsRecord],
) -> Vec<PerformanceMatrixRow> {
    if trades.is_empty() {
        return vec![];
    }

    let mut by_policy: std::collections::HashMap<String, Vec<&TradeAnalyticsRecord>> =
        std::collections::HashMap::new();
    for t in trades {
        by_policy
            .entry(t.trigger_source.clone())
            .or_default()
            .push(t);
    }

    let mut results = Vec::new();

    for (policy_id, policy_trades) in &by_policy {
        let mut by_regime: std::collections::HashMap<String, Vec<&TradeAnalyticsRecord>> =
            std::collections::HashMap::new();
        for t in policy_trades {
            let regime = resolve_regime_for_trade(pool, t).await;
            by_regime.entry(regime).or_default().push(*t);
        }

        for (regime, regime_trades) in &by_regime {
            let trade_count = regime_trades.len() as u32;
            let wins = regime_trades
                .iter()
                .filter(|t| t.net_pnl > 0.0)
                .count();
            let win_rate = if trade_count > 0 {
                wins as f64 / trade_count as f64
            } else {
                0.0
            };

            let gross_profit: f64 = regime_trades
                .iter()
                .filter(|t| t.net_pnl > 0.0)
                .map(|t| t.net_pnl)
                .sum();
            let gross_loss: f64 = regime_trades
                .iter()
                .filter(|t| t.net_pnl < 0.0)
                .map(|t| t.net_pnl.abs())
                .sum();
            let profit_factor = if gross_loss > 0.0 {
                let v = gross_profit / gross_loss;
                if v.is_finite() {
                    Some(v)
                } else {
                    None
                }
            } else if gross_profit > 0.0 {
                None
            } else {
                Some(0.0)
            };

            let total_pnl: f64 = regime_trades.iter().map(|t| t.net_pnl).sum();
            let r_multiples: Vec<f64> = regime_trades
                .iter()
                .filter_map(|t| {
                    let allocated = t.entry_price * t.size;
                    if allocated > 0.0 {
                        Some(t.net_pnl / allocated)
                    } else {
                        None
                    }
                })
                .collect();
            let avg_r_multiple = if !r_multiples.is_empty() {
                r_multiples.iter().sum::<f64>() / r_multiples.len() as f64
            } else {
                0.0
            };

            let compatibility_label =
                classify_regime_compatibility(profit_factor, win_rate, trade_count);

            results.push(PerformanceMatrixRow {
                policy_id: policy_id.clone(),
                regime: regime.clone(),
                trade_count,
                win_rate,
                profit_factor,
                avg_r_multiple,
                total_pnl,
                compatibility_label,
            });
        }
    }

    results
}

fn classify_regime_compatibility(
    profit_factor: Option<f64>,
    win_rate: f64,
    trade_count: u32,
) -> RegimeCompatibility {
    if trade_count < 3 {
        return RegimeCompatibility::Avoid;
    }
    let pf = profit_factor.unwrap_or(f64::INFINITY);
    if pf >= 1.5 && win_rate >= 0.55 && trade_count >= 5 {
        RegimeCompatibility::Strong
    } else if pf >= 1.2 && win_rate >= 0.45 {
        RegimeCompatibility::Favorable
    } else if pf >= 1.0 {
        RegimeCompatibility::Marginal
    } else {
        RegimeCompatibility::Avoid
    }
}

async fn resolve_regime_for_trade(
    pool: &SqlitePool,
    trade: &TradeAnalyticsRecord,
) -> String {
    let row: Result<Option<String>, _> = sqlx::query_scalar(
        "SELECT market_regime FROM market_snapshots
         WHERE symbol = ?1 AND timestamp <= ?2
         ORDER BY timestamp DESC LIMIT 1",
    )
    .bind(&trade.symbol)
    .bind(trade.entry_timestamp)
    .fetch_optional(pool)
    .await;

    match row {
        Ok(Some(regime)) => regime,
        _ => "UNKNOWN".to_string(),
    }
}
