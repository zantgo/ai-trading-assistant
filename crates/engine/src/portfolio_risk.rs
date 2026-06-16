use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use sqlx::SqlitePool;
use crate::db;

#[derive(Debug, Clone)]
pub struct PortfolioRiskState {
    pub max_daily_drawdown_pct: f64,
    pub max_portfolio_exposure_pct: f64,
    pub max_correlation: f64,
    pub max_single_pair_exposure_pct: f64,
    pub total_capital: f64,
}

impl Default for PortfolioRiskState {
    fn default() -> Self {
        Self {
            max_daily_drawdown_pct: 5.0,
            max_portfolio_exposure_pct: 50.0,
            max_correlation: 0.8,
            max_single_pair_exposure_pct: 20.0,
            total_capital: 10_000.0,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PortfolioValidation {
    pub allowed: bool,
    pub reason: String,
    pub current_daily_pnl: f64,
    pub current_drawdown_pct: f64,
    pub current_exposure_pct: f64,
    pub pairwise_correlations: Vec<(String, f64)>,
}

pub async fn validate_new_position(
    state: &PortfolioRiskState,
    pool: &SqlitePool,
    symbol: &str,
    new_exposure_pct: f64,
    existing_positions: &[db::ActivePaperPosition],
    pair_close_histories: &Arc<RwLock<HashMap<String, Vec<f64>>>>,
) -> PortfolioValidation {
    let mut reasons = Vec::new();

    let daily_pnl = db::get_daily_pnl(pool).await.unwrap_or(0.0);
    let drawdown_pct = if state.total_capital > 0.0 && daily_pnl < 0.0 {
        (daily_pnl.abs() / state.total_capital) * 100.0
    } else {
        0.0
    };
    let daily_loss_ok = drawdown_pct < state.max_daily_drawdown_pct;
    if !daily_loss_ok {
        reasons.push(format!(
            "Daily loss {:2.1}% exceeds max {:2.1}%",
            drawdown_pct, state.max_daily_drawdown_pct
        ));
    }

    let current_exposure: f64 = existing_positions
        .iter()
        .map(|p| p.allocated_usd)
        .sum();
    let current_exposure_pct = if state.total_capital > 0.0 {
        (current_exposure / state.total_capital) * 100.0
    } else {
        0.0
    };
    let new_total_exposure_pct = current_exposure_pct + new_exposure_pct;
    let exposure_ok = new_total_exposure_pct <= state.max_portfolio_exposure_pct;
    if !exposure_ok {
        reasons.push(format!(
            "Total exposure {:2.1}% would exceed max {:2.1}%",
            new_total_exposure_pct, state.max_portfolio_exposure_pct
        ));
    }

    let pair_exposure_ok = new_exposure_pct <= state.max_single_pair_exposure_pct;
    if !pair_exposure_ok {
        reasons.push(format!(
            "Pair exposure {:2.1}% exceeds max {:2.1}%",
            new_exposure_pct, state.max_single_pair_exposure_pct
        ));
    }

    let mut pairwise_correlations = Vec::new();
    let histories = pair_close_histories.read().await;
    if let Some(target_history) = histories.get(symbol) {
        for pos in existing_positions {
            if pos.symbol == symbol {
                continue;
            }
            if let Some(other_history) = histories.get(&pos.symbol) {
                if let Some(corr) = pearson_correlation(target_history, other_history) {
                    pairwise_correlations.push((pos.symbol.clone(), corr));
                    if corr.abs() > state.max_correlation {
                        reasons.push(format!(
                            "High correlation ({:.2}) with existing {} position",
                            corr, pos.symbol
                        ));
                    }
                }
            }
        }
    }

    let allowed = daily_loss_ok
        && exposure_ok
        && pair_exposure_ok
        && !pairwise_correlations
            .iter()
            .any(|(_, c)| c.abs() > state.max_correlation);

    PortfolioValidation {
        allowed,
        reason: if allowed {
            "All portfolio risk checks passed".into()
        } else {
            reasons.join("; ")
        },
        current_daily_pnl: daily_pnl,
        current_drawdown_pct: drawdown_pct,
        current_exposure_pct: new_total_exposure_pct,
        pairwise_correlations,
    }
}

pub async fn query_all_active_positions(pool: &SqlitePool) -> Vec<db::ActivePaperPosition> {
    use sqlx::Row;
    let rows = match sqlx::query(
        "SELECT id, symbol, direction, entry_price, size, allocated_usd, entry_timestamp,
                average_entry_price, current_portions, final_invalidation_level, target_profit_ratio
         FROM active_positions"
    )
    .fetch_all(pool)
    .await
    {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };

    rows.iter()
        .map(|r| db::ActivePaperPosition {
            id: r.get(0),
            symbol: r.get(1),
            direction: r.get(2),
            entry_price: r.get(3),
            size: r.get(4),
            allocated_usd: r.get(5),
            entry_timestamp: r.get(6),
            average_entry_price: r.get(7),
            current_portions: r.get(8),
            final_invalidation_level: r.get(9),
            target_profit_ratio: r.get(10),
        })
        .collect()
}

fn pearson_correlation(x: &[f64], y: &[f64]) -> Option<f64> {
    let n = x.len().min(y.len());
    if n < 10 {
        return None;
    }
    let x_slice = &x[x.len() - n..];
    let y_slice = &y[y.len() - n..];
    let mean_x = x_slice.iter().sum::<f64>() / n as f64;
    let mean_y = y_slice.iter().sum::<f64>() / n as f64;
    let mut cov = 0.0;
    let mut var_x = 0.0;
    let mut var_y = 0.0;
    for i in 0..n {
        let dx = x_slice[i] - mean_x;
        let dy = y_slice[i] - mean_y;
        cov += dx * dy;
        var_x += dx * dx;
        var_y += dy * dy;
    }
    if var_x == 0.0 || var_y == 0.0 {
        return None;
    }
    Some(cov / (var_x.sqrt() * var_y.sqrt()))
}
