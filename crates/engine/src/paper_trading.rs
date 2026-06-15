use sqlx::SqlitePool;
use tokio::sync::mpsc;

use crate::db;
use crate::profile_evaluation::{self, SnapshotValues, calculate_opposite_score};

pub struct PaperTradeResult {
    pub success: bool,
    pub message: String,
    pub entry_price: Option<f64>,
    pub size: Option<f64>,
    pub allocated_usd: Option<f64>,
}

pub struct PaperScaleInResult {
    pub success: bool,
    pub message: String,
    pub new_average_entry_price: f64,
    pub total_size: f64,
    pub portion_number: i32,
}

pub struct PaperScaleOutResult {
    pub success: bool,
    pub message: String,
    pub realized_pnl: f64,
    pub remaining_size: f64,
}

pub async fn verify_margin_and_open(
    pool: &SqlitePool,
    telemetry_tx: &mpsc::Sender<db::TelemetryMsg>,
    symbol: &str,
    direction: &str,
    current_price: f64,
) -> PaperTradeResult {
    verify_margin_and_open_with_alloc(pool, telemetry_tx, symbol, direction, current_price, None).await
}

/// Open a position with an optional allocation percentage override.
/// When `allocation_pct_override` is `None`, uses the paper_balance default.
pub async fn verify_margin_and_open_with_alloc(
    pool: &SqlitePool,
    telemetry_tx: &mpsc::Sender<db::TelemetryMsg>,
    symbol: &str,
    direction: &str,
    current_price: f64,
    allocation_pct_override: Option<f64>,
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

    let alloc_pct = allocation_pct_override.unwrap_or(balance.allocation_pct);
    let trade_cost = balance.current_cash * (alloc_pct / 100.0);

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
            alloc_pct, balance.current_cash + trade_cost
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

            let entry = pos.average_entry_price.unwrap_or(pos.entry_price);
            let realized_pnl = if pos.direction == "LONG" {
                (exit_price - entry) * pos.size
            } else {
                (entry - exit_price) * pos.size
            };

            let _ = telemetry_tx.send(db::TelemetryMsg::PaperClosePosition {
                symbol: symbol.to_string(),
                exit_price,
                exit_timestamp: now,
                trigger: trigger.to_string(),
            }).await;

            let _ = telemetry_tx.send(db::TelemetryMsg::JournalTrade {
                symbol: symbol.to_string(),
                direction: pos.direction.clone(),
                entry_price: entry,
                exit_price,
                entry_timestamp: pos.entry_timestamp,
                exit_timestamp: now,
                size: pos.size,
                realized_pnl,
                roi_pct: if pos.allocated_usd > 0.0 { (realized_pnl / pos.allocated_usd) * 100.0 } else { 0.0 },
                allocated_usd: pos.allocated_usd,
                trigger: trigger.to_string(),
            }).await;

            PaperTradeResult {
                success: true,
                message: format!(
                    "CLOSE {} {} | Exit: ${:.2} | PnL: ${:.2} | Avg Entry: ${:.2} | Size: {:.4}",
                    symbol, pos.direction, exit_price, realized_pnl, entry, pos.size
                ),
                entry_price: Some(entry),
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

/// Scale into an existing position (portions 2 or 3) or open a new position (portion 1).
/// Uses the trading strategy's 3-part scaling model (33% each).
pub async fn scale_in_portion(
    pool: &SqlitePool,
    telemetry_tx: &mpsc::Sender<db::TelemetryMsg>,
    symbol: &str,
    direction: &str,
    entry_price: f64,
    portion_number: i32,
    final_invalidation_level: f64,
) -> PaperScaleInResult {
    let balance = db::paper_get_balance(pool, symbol).await;
    let existing = db::paper_get_active_position(pool, symbol).await;

    // Portion cost: 1/3 of the total allocation
    let total_allocation = balance.initial_usd * (balance.allocation_pct / 100.0);
    let portion_cost = total_allocation / 3.0;

    if portion_cost <= 0.0 || portion_cost > balance.current_cash {
        return PaperScaleInResult {
            success: false,
            message: format!(
                "Insufficient capital for portion {}. Need ${:.2}, have ${:.2}",
                portion_number, portion_cost, balance.current_cash
            ),
            new_average_entry_price: 0.0,
            total_size: 0.0,
            portion_number,
        };
    }

    match existing {
        Some(pos) if pos.current_portions.unwrap_or(0) >= 3 => PaperScaleInResult {
            success: false,
            message: "All 3 portions already filled".to_string(),
            new_average_entry_price: pos.average_entry_price.unwrap_or(pos.entry_price),
            total_size: pos.size,
            portion_number,
        },
        Some(pos) if pos.direction != direction => PaperScaleInResult {
            success: false,
            message: format!("Cannot {} scale into existing {} position", direction, pos.direction),
            new_average_entry_price: pos.average_entry_price.unwrap_or(pos.entry_price),
            total_size: pos.size,
            portion_number,
        },
        Some(pos) => {
            // Scale into existing position
            let new_size = portion_cost / entry_price;
            let old_avg = pos.average_entry_price.unwrap_or(pos.entry_price);
            let old_size = pos.size;
            let new_avg = ((old_avg * old_size) + (entry_price * new_size)) / (old_size + new_size);
            let total_size = old_size + new_size;

            // Deduct cash
            sqlx::query("UPDATE paper_balances SET current_cash = current_cash - ?2 WHERE symbol = ?1")
                .bind(symbol)
                .bind(portion_cost)
                .execute(&*pool)
                .await
                .ok();

            let _ = telemetry_tx.send(db::TelemetryMsg::PaperScaleInPortion {
                symbol: symbol.to_string(),
                direction: direction.to_string(),
                entry_price,
                size: new_size,
                allocated_usd: portion_cost,
                portion_number,
                new_average_entry_price: new_avg,
                total_size,
                final_invalidation_level,
            }).await;

            PaperScaleInResult {
                success: true,
                message: format!(
                    "SCALE-IN Portion {}/3: {} @ ${:.2} | New Avg: ${:.2} | Total Size: {:.4}",
                    portion_number, symbol, entry_price, new_avg, total_size
                ),
                new_average_entry_price: new_avg,
                total_size,
                portion_number,
            }
        }
        None => {
            // Open new position (portion 1)
            let new_size = portion_cost / entry_price;

            sqlx::query("UPDATE paper_balances SET current_cash = current_cash - ?2 WHERE symbol = ?1")
                .bind(symbol)
                .bind(portion_cost)
                .execute(&*pool)
                .await
                .ok();

            let _ = telemetry_tx.send(db::TelemetryMsg::PaperScaleInPortion {
                symbol: symbol.to_string(),
                direction: direction.to_string(),
                entry_price,
                size: new_size,
                allocated_usd: portion_cost,
                portion_number: 1,
                new_average_entry_price: entry_price,
                total_size: new_size,
                final_invalidation_level,
            }).await;

            PaperScaleInResult {
                success: true,
                message: format!(
                    "OPEN (Portion 1/3): {} {} @ ${:.2} | Size: {:.4} | Allocated: ${:.2}",
                    symbol, direction, entry_price, new_size, portion_cost
                ),
                new_average_entry_price: entry_price,
                total_size: new_size,
                portion_number: 1,
            }
        }
    }
}

/// Scale out of a position (partial take-profit).
/// Closes a fraction of the position at the specified exit price.
pub async fn scale_out_portion(
    pool: &SqlitePool,
    telemetry_tx: &mpsc::Sender<db::TelemetryMsg>,
    symbol: &str,
    exit_price: f64,
    size_fraction: f64,
    target_id: i64,
    trigger: &str,
) -> PaperScaleOutResult {
    let position = db::paper_get_active_position(pool, symbol).await;

    match position {
        Some(pos) => {
            let entry = pos.average_entry_price.unwrap_or(pos.entry_price);
            let close_size = pos.size * size_fraction;
            let remaining_size = pos.size - close_size;

            let realized_pnl = if pos.direction == "LONG" {
                (exit_price - entry) * close_size
            } else {
                (entry - exit_price) * close_size
            };

            if remaining_size <= 0.0 {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as i64;
                let _ = telemetry_tx.send(db::TelemetryMsg::PaperClosePosition {
                    symbol: symbol.to_string(),
                    exit_price,
                    exit_timestamp: now,
                    trigger: trigger.to_string(),
                }).await;
            } else {
                let _ = telemetry_tx.send(db::TelemetryMsg::PaperScaleOutPortion {
                    symbol: symbol.to_string(),
                    exit_price,
                    size_fraction,
                    realized_pnl,
                    remaining_size,
                    target_id,
                }).await;
            }

            PaperScaleOutResult {
                success: true,
                message: format!(
                    "SCALE-OUT: {} @ ${:.2} | {:.0}% closed | PnL: ${:.2} | Remaining: {:.4}",
                    symbol, exit_price, size_fraction * 100.0, realized_pnl, remaining_size.max(0.0)
                ),
                realized_pnl,
                remaining_size: remaining_size.max(0.0),
            }
        }
        None => PaperScaleOutResult {
            success: false,
            message: format!("No active position to scale out for {}", symbol),
            realized_pnl: 0.0,
            remaining_size: 0.0,
        },
    }
}

/// Invalidate (hard stop) a position due to candle-close breach of invalidation level.
pub async fn invalidate_position(
    pool: &SqlitePool,
    telemetry_tx: &mpsc::Sender<db::TelemetryMsg>,
    symbol: &str,
    exit_price: f64,
    reason: &str,
) -> PaperScaleOutResult {
    let position = db::paper_get_active_position(pool, symbol).await;

    match position {
        Some(pos) => {
            let entry = pos.average_entry_price.unwrap_or(pos.entry_price);
            let realized_loss = if pos.direction == "LONG" {
                (exit_price - entry) * pos.size
            } else {
                (entry - exit_price) * pos.size
            };

            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64;

            let _ = telemetry_tx.send(db::TelemetryMsg::PaperInvalidatePosition {
                symbol: symbol.to_string(),
                exit_price,
                exit_timestamp: now,
                realized_loss,
                reason: reason.to_string(),
            }).await;

            let _ = telemetry_tx.send(db::TelemetryMsg::JournalTrade {
                symbol: symbol.to_string(),
                direction: pos.direction.clone(),
                entry_price: entry,
                exit_price,
                entry_timestamp: pos.entry_timestamp,
                exit_timestamp: now,
                size: pos.size,
                realized_pnl: realized_loss,
                roi_pct: if pos.allocated_usd > 0.0 { (realized_loss / pos.allocated_usd) * 100.0 } else { 0.0 },
                allocated_usd: pos.allocated_usd,
                trigger: format!("INVALIDATION:{}", reason),
            }).await;

            PaperScaleOutResult {
                success: true,
                message: format!(
                    "INVALIDATED: {} @ ${:.2} | Loss: ${:.2} | Reason: {}",
                    symbol, exit_price, realized_loss, reason
                ),
                realized_pnl: realized_loss,
                remaining_size: 0.0,
            }
        }
        None => PaperScaleOutResult {
            success: false,
            message: format!("No active position to invalidate for {}", symbol),
            realized_pnl: 0.0,
            remaining_size: 0.0,
        },
    }
}

/// Calculate the dynamic capital allocation percentage based on the 8-factor point score.
/// Returns (allocation_pct, total_score, max_score, signals_json).
pub async fn calculate_allocated_capital(
    _pool: &SqlitePool,
    _symbol: &str,
    bias: &str,
    snap: &SnapshotValues,
    support_levels: &[f64],
    resistance_levels: &[f64],
    macro_trend: &str,
) -> (f64, i32, i32, String) {
    let score = profile_evaluation::calculate_eight_factor_score(
        bias, snap, support_levels, resistance_levels, macro_trend,
    );
    let signals_json = serde_json::to_string(&score.signals).unwrap_or_default();
    (score.allocated_capital_pct, score.total_score, score.max_score, signals_json)
}

/// Evaluate whether an opposite-signal exit should trigger.
/// Returns true if more than `max_opposite` opposite signals are detected.
pub fn evaluate_opposite_exit(
    position_direction: &str,
    snap: &SnapshotValues,
    support_levels: &[f64],
    resistance_levels: &[f64],
    macro_trend: &str,
    max_opposite: u32,
) -> (bool, u32) {
    let opposite_score = calculate_opposite_score(
        position_direction, snap, support_levels, resistance_levels, macro_trend,
    );
    (opposite_score > max_opposite, opposite_score)
}

/// Check if a break-even trailing update is needed.
/// When the current price crosses beyond TP1 (first take-profit target),
/// the stop-loss for the remaining position should be moved to the entry price.
pub async fn check_break_even_trail(
    pool: &SqlitePool,
    symbol: &str,
    current_price: f64,
) -> bool {
    let position = db::paper_get_active_position(pool, symbol).await;
    let pos = match position {
        Some(ref p) => p,
        None => return false,
    };

    let entry = pos.average_entry_price.unwrap_or(pos.entry_price);
    let targets = sqlx::query_as::<_, (i64, f64, f64, i64)>(
        "SELECT id, target_price, size_fraction, is_hit FROM position_take_profit_targets
         WHERE symbol = ?1 AND is_hit = 0
         ORDER BY target_price ASC"
    )
    .bind(symbol)
    .fetch_all(&*pool)
    .await
    .unwrap_or_default();

    for (target_id, target_price, _size_fraction, _is_hit) in &targets {
        let tp_hit = if pos.direction == "LONG" {
            current_price >= *target_price
        } else {
            current_price <= *target_price
        };

        if tp_hit {
            // Mark TP1 as hit
            sqlx::query("UPDATE position_take_profit_targets SET is_hit = 1 WHERE id = ?1")
                .bind(target_id)
                .execute(&*pool)
                .await
                .ok();

            // For TP1, move stop-loss to entry (break-even)
            let new_invalidation = entry;
            sqlx::query(
                "UPDATE active_positions SET final_invalidation_level = ?2 WHERE symbol = ?1"
            )
            .bind(symbol)
            .bind(new_invalidation)
            .execute(&*pool)
            .await
            .ok();

            println!(
                "📄 Break-Even Trail: {} TP1 hit at ${:.2}. SL moved to entry ${:.2}",
                symbol, target_price, entry
            );
            return true;
        }
    }
    false
}
