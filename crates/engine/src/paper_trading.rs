use sqlx::SqlitePool;
use tokio::sync::mpsc;

use crate::db;

pub struct PaperTradeResult {
    pub success: bool,
    pub message: String,
    pub entry_price: Option<f64>,
    pub size: Option<f64>,
    pub allocated_usd: Option<f64>,
}

pub async fn verify_margin_and_open(
    pool: &SqlitePool,
    telemetry_tx: &mpsc::Sender<db::TelemetryMsg>,
    symbol: &str,
    direction: &str,
    current_price: f64,
) -> PaperTradeResult {
    let balance = db::paper_get_balance(pool, symbol).await;

    let position = db::paper_get_active_position(pool, symbol).await;
    if position.is_some() {
        return PaperTradeResult {
            success: false,
            message: format!("{} already has an active {} position", symbol, position.unwrap().direction),
            entry_price: None,
            size: None,
            allocated_usd: None,
        };
    }

    if current_price <= 0.0 {
        return PaperTradeResult {
            success: false,
            message: format!("Invalid current price for {}: ${:.4}", symbol, current_price),
            entry_price: None,
            size: None,
            allocated_usd: None,
        };
    }

    let trade_cost = balance.current_cash * (balance.allocation_pct / 100.0);

    if trade_cost <= 0.0 {
        return PaperTradeResult {
            success: false,
            message: format!(
                "Insufficient margin for {}. Cash: ${:.2}, Allocation: {:.1}% → Trade Cost: ${:.2}",
                symbol, balance.current_cash, balance.allocation_pct, trade_cost
            ),
            entry_price: None,
            size: None,
            allocated_usd: None,
        };
    }

    if trade_cost > balance.current_cash {
        return PaperTradeResult {
            success: false,
            message: format!(
                "Trade cost ${:.2} exceeds available cash ${:.2} for {}",
                trade_cost, balance.current_cash, symbol
            ),
            entry_price: None,
            size: None,
            allocated_usd: None,
        };
    }

    let position_size = trade_cost / current_price;

    if let Err(e) = sqlx::query(
        "UPDATE paper_balances SET current_cash = current_cash - ?2 WHERE symbol = ?1"
    )
    .bind(symbol)
    .bind(trade_cost)
    .execute(&*pool)
    .await
    {
        return PaperTradeResult {
            success: false,
            message: format!("Failed to deduct margin for {}: {}", symbol, e),
            entry_price: None,
            size: None,
            allocated_usd: None,
        };
    }

    let _ = telemetry_tx.send(db::TelemetryMsg::PaperOpenPosition {
        symbol: symbol.to_string(),
        direction: direction.to_string(),
        entry_price: current_price,
        size: position_size,
        allocated_usd: trade_cost,
    }).await;

    PaperTradeResult {
        success: true,
        message: format!(
            "OPEN {} {} | Entry: ${:.2} | Size: {:.4} units | Allocated: ${:.2} ({:.0}% of ${:.2})",
            symbol, direction, current_price, position_size, trade_cost,
            balance.allocation_pct, balance.current_cash + trade_cost
        ),
        entry_price: Some(current_price),
        size: Some(position_size),
        allocated_usd: Some(trade_cost),
    }
}

pub async fn close_paper_position(
    pool: &SqlitePool,
    telemetry_tx: &mpsc::Sender<db::TelemetryMsg>,
    symbol: &str,
    exit_price: f64,
    trigger: &str,
) -> PaperTradeResult {
    let position = db::paper_get_active_position(pool, symbol).await;

    match position {
        Some(pos) => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64;

            let realized_pnl = if pos.direction == "LONG" {
                (exit_price - pos.entry_price) * pos.size
            } else {
                (pos.entry_price - exit_price) * pos.size
            };

            let _ = telemetry_tx.send(db::TelemetryMsg::PaperClosePosition {
                symbol: symbol.to_string(),
                exit_price,
                exit_timestamp: now,
                trigger: trigger.to_string(),
            }).await;

            PaperTradeResult {
                success: true,
                message: format!(
                    "CLOSE {} {} | Exit: ${:.2} | PnL: ${:.2} | Entry: ${:.2} | Size: {:.4}",
                    symbol, pos.direction, exit_price, realized_pnl, pos.entry_price, pos.size
                ),
                entry_price: Some(pos.entry_price),
                size: Some(pos.size),
                allocated_usd: Some(pos.allocated_usd),
            }
        }
        None => PaperTradeResult {
            success: false,
            message: format!("No active position to close for {}", symbol),
            entry_price: None,
            size: None,
            allocated_usd: None,
        },
    }
}
