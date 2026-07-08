use sqlx::SqlitePool;
use tokio::sync::mpsc;
use rust_decimal::prelude::*;
use rust_decimal::Decimal;

use crate::db;
use crate::profile_evaluation::SnapshotValues;
use std::collections::HashMap;

#[inline]
fn dec(v: f64) -> Decimal {
    Decimal::from_f64(v).unwrap_or(Decimal::ZERO)
}

#[inline]
fn f64_from_dec(d: Decimal) -> f64 {
    d.to_f64().unwrap_or(0.0)
}

// ─── Result Types ──────────────────────────────────────────────────

pub struct PaperTradeResult {
    pub success: bool,
    pub message: String,
    pub entry_price: Option<f64>,
    pub size: Option<f64>,
    pub allocated_usd: Option<f64>,
    pub position_pct: Option<f64>,
}

pub struct PaperPositionOpResult {
    pub success: bool,
    pub message: String,
    pub direction: String,
    pub position_pct: f64,
    pub free_balance_pct: f64,
    pub entry_price: f64,
    pub size: f64,
    pub allocated_usd: f64,
}

#[derive(Debug)]
#[allow(dead_code)]
enum RiskError {
    InsufficientCapital { required: Decimal, available: Decimal },
    PositionTooLarge { max: Decimal, requested: Decimal },
    InvalidStopLoss { reason: String },
}

impl std::fmt::Display for RiskError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InsufficientCapital { required, available } => {
                write!(f, "Insufficient capital: required ${:.2}, available ${:.2}", required, available)
            }
            Self::PositionTooLarge { max, requested } => {
                write!(f, "Position too large: max {:.4} units, requested {:.4}", max, requested)
            }
            Self::InvalidStopLoss { reason } => {
                write!(f, "Invalid stop loss: {}", reason)
            }
        }
    }
}

impl std::error::Error for RiskError {}

struct PositionRiskValidator {
    capital: Decimal,
    _max_risk_pct: Decimal,
    _leverage: i32,
}

impl PositionRiskValidator {
    fn new(capital: f64, max_risk_pct: f64, leverage: i32) -> Self {
        Self {
            capital: dec(capital),
            _max_risk_pct: dec(max_risk_pct),
            _leverage: leverage,
        }
    }

    fn validate_trade_cost(&self, cost: Decimal) -> Result<(), RiskError> {
        if cost > self.capital {
            return Err(RiskError::InsufficientCapital {
                required: cost,
                available: self.capital,
            });
        }
        Ok(())
    }

    fn _validate_stop_loss(&self, _entry: Decimal, _stop: Decimal, _direction: &str) -> Result<(), RiskError> {
        Ok(())
    }
}

// ─── Core Percentage-Based Position Engine ─────────────────────────

/// Calculate position sizing from a percentage of total balance.
/// `pct` is 0-100 representing the percentage of initial_usd to allocate.
fn calc_position_from_pct(balance: &db::paper::queries::PaperBalance, pct: f64) -> (Decimal, Decimal) {
    let init = dec(balance.initial_usd);
    let alloc_pct = dec(pct);
    let allocated = init * alloc_pct / dec(100.0);
    (allocated, alloc_pct)
}

/// Open a position using percentage of balance. Supports netting.
/// If opening opposite direction, automatically closes existing position first.
pub async fn open_position_pct(
    pool: &SqlitePool,
    telemetry_tx: &mpsc::Sender<db::TelemetryMsg>,
    symbol: &str,
    direction: &str,
    pct: f64,
    current_price: f64,
) -> PaperPositionOpResult {
    let balance = db::paper_get_balance(pool, symbol).await;
    let existing = db::paper_get_active_position(pool, symbol).await;

    // Validate percentage — 25% portions
    if !(25.0..=100.0).contains(&pct) || pct % 25.0 != 0.0 {
        return PaperPositionOpResult {
            success: false,
            message: format!("Position percentage must be 25-100 in steps of 25, got {}", pct),
            direction: direction.to_string(),
            position_pct: existing.as_ref().map(|p| {
                if p.allocated_usd > 0.0 && balance.initial_usd > 0.0 {
                    (p.allocated_usd / balance.initial_usd) * 100.0
                } else { 0.0 }
            }).unwrap_or(0.0),
            free_balance_pct: 0.0,
            entry_price: 0.0,
            size: 0.0,
            allocated_usd: 0.0,
        };
    }

    let existing_pct = existing.as_ref().map(|p| {
        if p.allocated_usd > 0.0 && balance.initial_usd > 0.0 {
            (p.allocated_usd / balance.initial_usd) * 100.0
        } else { 0.0 }
    }).unwrap_or(0.0);

    // Netting: if opposite direction exists, close it first
    match &existing {
        Some(pos) if pos.direction != direction => {
            // Close existing position first (netting at current price)
            let close_result = close_paper_position(pool, telemetry_tx, symbol, current_price, "NET").await;
            if !close_result.success {
                return PaperPositionOpResult {
                    success: false,
                    message: format!("Failed to close existing {}: {}", pos.direction, close_result.message),
                    direction: direction.to_string(),
                    position_pct: 0.0,
                    free_balance_pct: 0.0,
                    entry_price: 0.0,
                    size: 0.0,
                    allocated_usd: 0.0,
                };
            }
            // After closing, open the net remainder (new - old)
            let net_pct = (pct - existing_pct).max(25.0);
            let balance2 = db::paper_get_balance(pool, symbol).await;
            let (allocated, _) = calc_position_from_pct(&balance2, net_pct);
            let allocated_f64 = f64_from_dec(allocated);
            let size = allocated_f64 / current_price;
            let now2 = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as i64;
            if let Ok(mut tx2) = pool.begin().await {
                sqlx::query(
                    "INSERT OR REPLACE INTO active_positions (symbol, direction, entry_price, size, allocated_usd, entry_timestamp, average_entry_price, current_portions, final_invalidation_level)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?3, 1, 0)"
                ).bind(symbol).bind(direction).bind(current_price).bind(size).bind(allocated_f64).bind(now2)
                .execute(&mut *tx2).await.ok();
                sqlx::query("UPDATE paper_balances SET current_cash = current_cash - ?2 WHERE symbol = ?1")
                    .bind(symbol).bind(allocated_f64).execute(&mut *tx2).await.ok();
                let _ = tx2.commit().await;
            }
            PaperPositionOpResult {
                success: true,
                message: format!("Netted: closed {}%, opened {}% {:.0} at ${:.2}", existing_pct, net_pct, direction, current_price),
                direction: direction.to_string(),
                position_pct: net_pct,
                free_balance_pct: 100.0 - net_pct,
                entry_price: current_price, size, allocated_usd: allocated_f64,
            }
        }
        Some(pos) => {
            // Same direction — scale in (add more)
            let (new_allocated, _) = calc_position_from_pct(&balance, pct);
            let new_size = new_allocated / dec(current_price);
            let total_allocated = pos.allocated_usd + f64_from_dec(new_allocated);
            let total_pct = (total_allocated / balance.initial_usd) * 100.0;

            if total_pct > 100.0 {
                return PaperPositionOpResult {
                    success: false,
                    message: format!("Total position would be {:.0}% — exceeds 100% limit", total_pct),
                    direction: direction.to_string(),
                    position_pct: existing_pct,
                    free_balance_pct: 100.0 - existing_pct,
                    entry_price: pos.average_entry_price.unwrap_or(pos.entry_price),
                    size: pos.size,
                    allocated_usd: pos.allocated_usd,
                };
            }

            let validator = PositionRiskValidator::new(balance.current_cash, balance.max_risk_pct, balance.leverage);
            if let Err(e) = validator.validate_trade_cost(new_allocated) {
                return PaperPositionOpResult {
                    success: false,
                    message: format!("Insufficient capital: {}", e),
                    direction: direction.to_string(),
                    position_pct: existing_pct,
                    free_balance_pct: 100.0 - existing_pct,
                    entry_price: pos.average_entry_price.unwrap_or(pos.entry_price),
                    size: pos.size,
                    allocated_usd: pos.allocated_usd,
                };
            }

            let old_avg = dec(pos.average_entry_price.unwrap_or(pos.entry_price));
            let old_size = dec(pos.size);
            let new_avg = ((old_avg * old_size) + (dec(current_price) * new_size)) / (old_size + new_size);
            let total_size = f64_from_dec(old_size + new_size);

            let new_allocated_f64 = f64_from_dec(new_allocated);
            sqlx::query("UPDATE paper_balances SET current_cash = current_cash - ?2 WHERE symbol = ?1")
                .bind(symbol)
                .bind(new_allocated_f64)
                .execute(pool)
                .await
                .ok();

            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as i64;
            let current_portions = pos.current_portions.unwrap_or(0) + 1;
            sqlx::query(
                "UPDATE active_positions SET size = ?2, allocated_usd = ?3, average_entry_price = ?4, current_portions = ?5, entry_timestamp = ?6 WHERE symbol = ?1"
            )
            .bind(symbol)
            .bind(total_size)
            .bind(total_allocated)
            .bind(f64_from_dec(new_avg))
            .bind(current_portions)
            .bind(now)
            .execute(pool)
            .await
            .ok();

            // Also create position_slots entry for backward compat
            let slot_idx = current_portions - 1 ;
            sqlx::query(
                "INSERT OR IGNORE INTO position_slots (position_id, symbol, direction, slot_index, is_active, entry_price, size, allocated_usd, realized_pnl, timestamp)
                 VALUES (?1, ?2, ?3, ?4, 1, ?5, ?6, ?7, 0.0, ?8)"
            )
            .bind(pos.id).bind(symbol).bind(direction).bind(slot_idx)
            .bind(current_price).bind(f64_from_dec(new_size)).bind(new_allocated_f64)
            .bind(now)
            .execute(pool)
            .await
            .ok();

            let free_pct = 100.0 - total_pct;
            PaperPositionOpResult {
                success: true,
                message: format!("Scaled in {} {:.0}% at ${:.2}. Total: {:.0}%", direction, pct, current_price, total_pct),
                direction: direction.to_string(),
                position_pct: total_pct,
                free_balance_pct: free_pct,
                entry_price: f64_from_dec(new_avg),
                size: total_size,
                allocated_usd: total_allocated,
            }
        }
        None => {
            // No existing position — fresh open
            let (allocated, _) = calc_position_from_pct(&balance, pct);
            let allocated_f64 = f64_from_dec(allocated);
            let size = allocated_f64 / current_price;

            let validator = PositionRiskValidator::new(balance.current_cash, balance.max_risk_pct, balance.leverage);
            if let Err(e) = validator.validate_trade_cost(allocated) {
                return PaperPositionOpResult {
                    success: false,
                    message: format!("{}", e),
                    direction: direction.to_string(),
                    position_pct: 0.0,
                    free_balance_pct: 100.0,
                    entry_price: 0.0,
                    size: 0.0,
                    allocated_usd: 0.0,
                };
            }

            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as i64;

            if let Ok(mut tx) = pool.begin().await {
                sqlx::query(
                    "INSERT OR REPLACE INTO active_positions (symbol, direction, entry_price, size, allocated_usd, entry_timestamp, average_entry_price, current_portions, final_invalidation_level, initial_allocated_margin, realized_pnl_accumulator)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?3, 1, 0, ?5, 0.0)"
                )
                .bind(symbol).bind(direction).bind(current_price).bind(size).bind(allocated_f64).bind(now)
                .execute(&mut *tx).await.ok();

                sqlx::query("UPDATE paper_balances SET current_cash = current_cash - ?2 WHERE symbol = ?1")
                    .bind(symbol).bind(allocated_f64).execute(&mut *tx).await.ok();

                // Also create position_slots entry
                let pos_id: i64 = sqlx::query_scalar("SELECT id FROM active_positions WHERE symbol = ?1")
                    .bind(symbol).fetch_one(&mut *tx).await.unwrap_or(0);
                if pos_id > 0 {
                    sqlx::query(
                        "INSERT OR IGNORE INTO position_slots (position_id, symbol, direction, slot_index, is_active, entry_price, size, allocated_usd, realized_pnl, timestamp)
                         VALUES (?1, ?2, ?3, 0, 1, ?4, ?5, ?6, 0.0, ?7)"
                    ).bind(pos_id).bind(symbol).bind(direction).bind(current_price).bind(size).bind(allocated_f64).bind(now)
                    .execute(&mut *tx).await.ok();
                }
                let _ = tx.commit().await;
            }

            let free_pct = 100.0 - pct;
            PaperPositionOpResult {
                success: true,
                message: format!("Opened {} {:.0}% at ${:.2}", direction, pct, current_price),
                direction: direction.to_string(),
                position_pct: pct,
                free_balance_pct: free_pct,
                entry_price: current_price,
                size,
                allocated_usd: allocated_f64,
            }
        }
    }
}

/// Close a percentage of the current position.
/// `close_pct` is 0-100 representing what percentage of the open position to close.
pub async fn close_position_pct(
    pool: &SqlitePool,
    telemetry_tx: &mpsc::Sender<db::TelemetryMsg>,
    symbol: &str,
    close_pct: f64,
    current_price: f64,
) -> PaperPositionOpResult {
    let balance = db::paper_get_balance(pool, symbol).await;
    let position = db::paper_get_active_position(pool, symbol).await;

    let pos = match position {
        Some(ref p) => p,
        None => {
            return PaperPositionOpResult {
                success: false,
                message: "No active position to close".to_string(),
                direction: String::new(),
                position_pct: 0.0,
                free_balance_pct: 100.0,
                entry_price: 0.0,
                size: 0.0,
                allocated_usd: 0.0,
            };
        }
    };

    if !(10.0..=100.0).contains(&close_pct) || close_pct % 10.0 != 0.0 {
        return PaperPositionOpResult {
            success: false,
            message: format!("Close percentage must be 10-100 in steps of 10, got {}", close_pct),
            direction: pos.direction.clone(),
            position_pct: (pos.allocated_usd / balance.initial_usd) * 100.0,
            free_balance_pct: 100.0 - (pos.allocated_usd / balance.initial_usd) * 100.0,
            entry_price: pos.average_entry_price.unwrap_or(pos.entry_price),
            size: pos.size,
            allocated_usd: pos.allocated_usd,
        };
    }

    let close_fraction = close_pct / 100.0;
    let close_allocated = pos.allocated_usd * close_fraction;
    let close_size = pos.size * close_fraction;

    let realized_pnl = if pos.direction == "LONG" {
        (current_price - pos.average_entry_price.unwrap_or(pos.entry_price)) * close_size
    } else {
        (pos.average_entry_price.unwrap_or(pos.entry_price) - current_price) * close_size
    };

    let remaining_allocated = pos.allocated_usd - close_allocated;
    let remaining_size = pos.size - close_size;
    let remaining_pct = if balance.initial_usd > 0.0 { (remaining_allocated / balance.initial_usd) * 100.0 } else { 0.0 };

    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as i64;

    if remaining_size <= 0.0 || remaining_pct < 1.0 {
        // Fully closed
        let alloc = pos.allocated_usd;
        let entry_price = pos.average_entry_price.unwrap_or(pos.entry_price);
        db::paper::paper_close_position_internal(pool, symbol, current_price, now, "MANUAL").await.ok();
        let _ = telemetry_tx.send(db::TelemetryMsg::JournalTrade {
            symbol: symbol.to_string(),
            direction: pos.direction.clone(),
            entry_price,
            exit_price: current_price,
            entry_timestamp: pos.entry_timestamp,
            exit_timestamp: now,
            size: pos.size,
            realized_pnl,
            roi_pct: if alloc > 0.0 { (realized_pnl / alloc) * 100.0 } else { 0.0 },
            allocated_usd: alloc,
            trigger: "MANUAL".to_string(),
        }).await;

        PaperPositionOpResult {
            success: true,
            message: format!("Position fully closed. PnL: ${:.2}", realized_pnl),
            direction: String::new(),
            position_pct: 0.0,
            free_balance_pct: 100.0,
            entry_price: 0.0,
            size: 0.0,
            allocated_usd: 0.0,
        }
    } else {
        // Partial close
        sqlx::query("UPDATE active_positions SET size = ?2, allocated_usd = ?3 WHERE symbol = ?1")
            .bind(symbol)
            .bind(remaining_size)
            .bind(remaining_allocated)
            .execute(pool)
            .await
            .ok();

        sqlx::query("UPDATE paper_balances SET current_cash = current_cash + ?2 WHERE symbol = ?1")
            .bind(symbol)
            .bind(close_allocated + realized_pnl)
            .execute(pool)
            .await
            .ok();

        let free_pct = 100.0 - remaining_pct;
        PaperPositionOpResult {
            success: true,
            message: format!("Closed {:.0}% of {}. PnL: ${:.2}. Remaining: {:.0}%", close_pct, pos.direction, realized_pnl, remaining_pct),
            direction: pos.direction.clone(),
            position_pct: remaining_pct,
            free_balance_pct: free_pct,
            entry_price: pos.average_entry_price.unwrap_or(pos.entry_price),
            size: remaining_size,
            allocated_usd: remaining_allocated,
        }
    }
}

// ─── TP/SL Management (25% Portion Model, via open_orders) ─────────

/// Set take-profit targets. Max X brackets where X = active slot count.
pub async fn set_take_profit_targets(
    pool: &SqlitePool,
    symbol: &str,
    targets: &[(f64, f64)],  // (pct, price) pairs
) -> Result<String, String> {
    let position = db::paper_get_active_position(pool, symbol).await;
    let pos = position.ok_or("No active position".to_string())?;

    let active_count = db::paper_get_active_slot_count(pool, symbol).await;
    let max_brackets = active_count.max(1) as usize;

    let (tp_count, _) = db::paper_count_brackets_by_type(pool, pos.id).await;
    if tp_count + targets.len() as i32 > max_brackets as i32 {
        return Err(format!("Maximum {} Take-Profit brackets allowed (one per active slot)", max_brackets));
    }

    let total_tp_pct: f64 = targets.iter().map(|(p, _)| p).sum();
    if total_tp_pct > 100.0 {
        return Err(format!("Total TP percentage ({:.0}%) exceeds 100%", total_tp_pct));
    }

    for (pct, _price) in targets {
        if *pct < 25.0 || *pct > 100.0 || *pct % 25.0 != 0.0 {
            return Err(format!("TP percentage must be 25-100 in steps of 25, got {}", pct));
        }
    }

    let direction = if pos.direction == "LONG" { "SELL" } else { "BUY" };
    for (pct, price) in targets {
        crate::db::paper::operations::paper_insert_open_order(
            pool, symbol, "LIMIT", direction, Some(*price), None, *pct, true, Some(pos.id), None,
        ).await.map_err(|e| format!("DB error: {}", e))?;
    }

    Ok(format!("{} TP targets set", targets.len()))
}

/// Set stop-loss levels. Max X brackets where X = active slot count.
pub async fn set_stop_loss_levels(
    pool: &SqlitePool,
    symbol: &str,
    stops: &[(f64, f64)],  // (pct, price) pairs
) -> Result<String, String> {
    let position = db::paper_get_active_position(pool, symbol).await;
    let pos = position.ok_or("No active position".to_string())?;
    let entry = pos.average_entry_price.unwrap_or(pos.entry_price);

    let active_count = db::paper_get_active_slot_count(pool, symbol).await;
    let max_brackets = active_count.max(1) as usize;

    let (_, sl_count) = db::paper_count_brackets_by_type(pool, pos.id).await;
    if sl_count + stops.len() as i32 > max_brackets as i32 {
        return Err(format!("Maximum {} Stop-Loss brackets allowed (one per active slot)", max_brackets));
    }

    let total_sl_pct: f64 = stops.iter().map(|(p, _)| p).sum();
    if total_sl_pct > 100.0 {
        return Err(format!("Total SL percentage ({:.0}%) exceeds 100%", total_sl_pct));
    }

    for (pct, _price) in stops {
        if *pct < 25.0 || *pct > 100.0 || *pct % 25.0 != 0.0 {
            return Err(format!("SL percentage must be 25-100 in steps of 25, got {}", pct));
        }
    }

    if pos.direction == "LONG" {
        for (_pct, price) in stops {
            if *price >= entry {
                return Err("Long SL must be below entry price".to_string());
            }
        }
    } else {
        for (_pct, price) in stops {
            if *price <= entry {
                return Err("Short SL must be above entry price".to_string());
            }
        }
    }

    let direction = if pos.direction == "LONG" { "SELL" } else { "BUY" };
    for (pct, price) in stops {
        crate::db::paper::operations::paper_insert_open_order(
            pool, symbol, "STOP", direction, None, Some(*price), *pct, true, Some(pos.id), None,
        ).await.map_err(|e| format!("DB error: {}", e))?;
    }

    // Cache worst-case SL in active_positions
    let worst_sl = if pos.direction == "LONG" {
        stops.iter().map(|(_, p)| p).fold(f64::INFINITY, |a, &b| a.min(b))
    } else {
        stops.iter().map(|(_, p)| p).fold(f64::NEG_INFINITY, |a, &b| a.max(b))
    };
    sqlx::query("UPDATE active_positions SET final_invalidation_level = ?2 WHERE symbol = ?1")
        .bind(symbol).bind(worst_sl).execute(pool).await
        .map_err(|e| format!("DB error: {}", e))?;

    Ok(format!("{} SL levels set", stops.len()))
}

/// Place a pending limit or stop entry order (25% portion).
pub async fn place_pending_order(
    pool: &SqlitePool,
    symbol: &str,
    order_type: &str,
    direction: &str,
    price: Option<f64>,
    trigger_price: Option<f64>,
) -> Result<i64, String> {
    if order_type != "LIMIT" && order_type != "STOP" {
        return Err("order_type must be LIMIT or STOP".to_string());
    }
    if direction != "BUY" && direction != "SELL" {
        return Err("direction must be BUY or SELL".to_string());
    }
    if order_type == "LIMIT" && price.is_none() {
        return Err("LIMIT orders require a price".to_string());
    }
    if order_type == "STOP" && trigger_price.is_none() {
        return Err("STOP orders require a trigger_price".to_string());
    }

    let active_count = db::paper_get_active_slot_count(pool, symbol).await;
    let pending_count = db::paper_get_open_orders(pool, symbol).await.len() as i32;
    if pending_count + 1 > 4 - active_count {
        return Err(format!(
            "Cannot place pending entry: {} pending + 1 new exceeds {} available slots (4 max, {} active)",
            pending_count, 4 - active_count, active_count
        ));
    }

    crate::db::paper::operations::paper_insert_open_order(
        pool, symbol, order_type, direction, price, trigger_price, 25.0, false, None, None,
    ).await.map_err(|e| format!("DB error: {}", e))
}

/// Cancel a pending order by ID.
pub async fn cancel_pending_order(pool: &SqlitePool, order_id: i64) -> Result<bool, String> {
    crate::db::paper::operations::paper_delete_open_order(pool, order_id)
        .await
        .map_err(|e| format!("DB error: {}", e))
}

// ─── Legacy Position Functions (Automation Compat) ─────────────────

pub async fn verify_margin_and_open(
    pool: &SqlitePool,
    telemetry_tx: &mpsc::Sender<db::TelemetryMsg>,
    symbol: &str,
    direction: &str,
    current_price: f64,
) -> PaperTradeResult {
    verify_margin_and_open_with_alloc(pool, telemetry_tx, symbol, direction, current_price, None).await
}

pub async fn verify_margin_and_open_with_alloc(
    pool: &SqlitePool,
    _telemetry_tx: &mpsc::Sender<db::TelemetryMsg>,
    symbol: &str,
    direction: &str,
    current_price: f64,
    allocation_pct_override: Option<f64>,
) -> PaperTradeResult {
    let balance = db::paper_get_balance(pool, symbol).await;
    let position = db::paper_get_active_position(pool, symbol).await;

    if let Some(pos) = &position {
        return PaperTradeResult {
            success: false,
            message: format!("{} already has an active {} position", symbol, pos.direction),
            entry_price: None, size: None, allocated_usd: None, position_pct: None,
        };
    }

    let price_dec = dec(current_price);
    if price_dec <= Decimal::ZERO {
        return PaperTradeResult {
            success: false,
            message: format!("Invalid current price for {}: ${:.4}", symbol, current_price),
            entry_price: None, size: None, allocated_usd: None, position_pct: None,
        };
    }

    let init_dec = dec(balance.initial_usd);
    let alloc_pct = allocation_pct_override.unwrap_or(balance.allocation_pct);
    let alloc_pct_dec = dec(alloc_pct);
    let total_allocation = init_dec * alloc_pct_dec / dec(100.0);

    let effective_leverage = if balance.leverage_mode == "VolatilityScaled" && balance.leverage_cap > 0 {
        let atr = db::query_latest_snapshot(pool, symbol, 60)
            .await
            .and_then(|s| {
                s.indicators
                    .get("atr")
                    .and_then(|v| v.values.as_ref())
                    .and_then(|m| m.get("atr_14"))
                    .copied()
            })
            .unwrap_or(0.0);
        if atr > 0.0 && current_price > 0.0 {
            let vol_ratio = atr / current_price;
            let optimal = if vol_ratio > 0.0 {
                let target_margin = if balance.atr_leverage_multiplier > 0.0 {
                    balance.atr_leverage_multiplier
                } else {
                    0.02
                };
                (target_margin / vol_ratio).clamp(1.0, balance.leverage_cap as f64)
            } else {
                balance.leverage as f64
            };
            optimal as i32
        } else {
            balance.leverage
        }
    } else {
        balance.leverage
    };

    let validator = PositionRiskValidator::new(balance.current_cash, balance.max_risk_pct, effective_leverage);
    if let Err(e) = validator.validate_trade_cost(total_allocation) {
        return PaperTradeResult {
            success: false,
            message: format!("{}", e),
            entry_price: None, size: None, allocated_usd: None, position_pct: None,
        };
    }

    let size = total_allocation / price_dec;
    let alloc_f64 = f64_from_dec(total_allocation);

    let _now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as i64;
    let _ = db::paper::paper_open_position_internal(pool, symbol, direction, current_price, f64_from_dec(size), alloc_f64).await;

    sqlx::query("UPDATE paper_balances SET current_cash = current_cash - ?2 WHERE symbol = ?1")
        .bind(symbol)
        .bind(alloc_f64)
        .execute(pool)
        .await
        .ok();

    PaperTradeResult {
        success: true,
        message: format!("Opened {} {} at ${:.2}", direction, symbol, current_price),
        entry_price: Some(current_price),
        size: Some(f64_from_dec(size)),
        allocated_usd: Some(alloc_f64),
        position_pct: Some(alloc_pct),
    }
}

pub async fn close_paper_position(
    pool: &SqlitePool,
    telemetry_tx: &mpsc::Sender<db::TelemetryMsg>,
    symbol: &str,
    current_price: f64,
    trigger: &str,
) -> PaperTradeResult {
    let position = db::paper_get_active_position(pool, symbol).await;
    match position {
        Some(pos) => {
            let pnl = if pos.direction == "LONG" {
                (current_price - pos.average_entry_price.unwrap_or(pos.entry_price)) * pos.size
            } else {
                (pos.average_entry_price.unwrap_or(pos.entry_price) - current_price) * pos.size
            };
            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as i64;
            let alloc = pos.allocated_usd;
            db::paper::paper_close_position_internal(pool, symbol, current_price, now, trigger).await.ok();
            let _ = telemetry_tx.send(db::TelemetryMsg::JournalTrade {
                symbol: symbol.to_string(),
                direction: pos.direction.clone(),
                entry_price: pos.average_entry_price.unwrap_or(pos.entry_price),
                exit_price: current_price,
                entry_timestamp: pos.entry_timestamp,
                exit_timestamp: now,
                size: pos.size,
                realized_pnl: pnl,
                roi_pct: if alloc > 0.0 { (pnl / alloc) * 100.0 } else { 0.0 },
                allocated_usd: alloc,
                trigger: trigger.to_string(),
            }).await;

            PaperTradeResult {
                success: true,
                message: format!("Closed {} position. PnL: ${:.2}", pos.direction, pnl),
                entry_price: None, size: None, allocated_usd: None, position_pct: None,
            }
        }
        None => PaperTradeResult {
            success: false,
            message: format!("No active position to close for {}", symbol),
            entry_price: None, size: None, allocated_usd: None, position_pct: None,
        },
    }
}

pub async fn invalidate_position(
    pool: &SqlitePool,
    telemetry_tx: &mpsc::Sender<db::TelemetryMsg>,
    symbol: &str,
    exit_price: f64,
    reason: &str,
) -> PaperTradeResult {
    let position = db::paper_get_active_position(pool, symbol).await;
    match position {
        Some(pos) => {
            let loss = if pos.direction == "LONG" {
                (pos.average_entry_price.unwrap_or(pos.entry_price) - exit_price) * pos.size
            } else {
                (exit_price - pos.average_entry_price.unwrap_or(pos.entry_price)) * pos.size
            };
            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as i64;
            let alloc = pos.allocated_usd;
            let entry_price = pos.average_entry_price.unwrap_or(pos.entry_price);
            db::paper::paper_invalidate_position_internal(pool, symbol, exit_price, now, loss, reason).await.ok();
            let _ = telemetry_tx.send(db::TelemetryMsg::JournalTrade {
                symbol: symbol.to_string(),
                direction: pos.direction.clone(),
                entry_price,
                exit_price,
                entry_timestamp: pos.entry_timestamp,
                exit_timestamp: now,
                size: pos.size,
                realized_pnl: -loss,
                roi_pct: if alloc > 0.0 { ((-loss) / alloc) * 100.0 } else { 0.0 },
                allocated_usd: alloc,
                trigger: format!("INVALIDATION:{}", reason),
            }).await;

            PaperTradeResult {
                success: true,
                message: format!("Invalidated {} position at ${:.2}. Loss: ${:.2}", pos.direction, exit_price, loss),
                entry_price: None, size: None, allocated_usd: None, position_pct: None,
            }
        }
        None => PaperTradeResult {
            success: false,
            message: format!("No active position to invalidate for {}", symbol),
            entry_price: None, size: None, allocated_usd: None, position_pct: None,
        },
    }
}

/// Check if a break-even trailing update is needed.
/// No-op when break_even_trail_enabled is false in paper_balances.
pub async fn check_break_even_trail(pool: &SqlitePool, symbol: &str, current_price: f64) -> bool {
    let balance = db::paper_get_balance(pool, symbol).await;
    if !balance.break_even_trail_enabled {
        return false;
    }
    let position = db::paper_get_active_position(pool, symbol).await;
    let pos = match position {
        Some(ref p) => p,
        None => return false,
    };

    let entry = pos.average_entry_price.unwrap_or(pos.entry_price);
    let targets = sqlx::query_as::<_, (i64, f64)>(
        "SELECT id, COALESCE(price, trigger_price) as target_price FROM open_orders
         WHERE associated_position_id = ?1 AND is_reduce_only = 1 AND order_type = 'LIMIT'
         ORDER BY target_price ASC",
    )
    .bind(pos.id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    for (order_id, target_price) in &targets {
        let tp_hit = if pos.direction == "LONG" {
            current_price >= *target_price
        } else {
            current_price <= *target_price
        };

        if tp_hit {
            if let Ok(mut tx) = pool.begin().await {
                sqlx::query("DELETE FROM open_orders WHERE id = ?1")
                    .bind(order_id).execute(&mut *tx).await.ok();
                sqlx::query("UPDATE active_positions SET final_invalidation_level = ?2 WHERE symbol = ?1")
                    .bind(symbol).bind(entry).execute(&mut *tx).await.ok();
                let _ = tx.commit().await;
            }

            println!("📄 Break-Even Trail: {} TP1 hit at ${:.2}. SL moved to entry ${:.2}", symbol, target_price, entry);
            return true;
        }
    }
    false
}

/// Apply break-even trailing immediately after a TP fill (called from order matcher).
/// Moves stop-loss to the weighted average entry price of remaining active slots.
/// No-op when break_even_trail_enabled is false in paper_balances.
pub async fn apply_break_even_trail(pool: &SqlitePool, symbol: &str) {
    let balance = db::paper_get_balance(pool, symbol).await;
    if !balance.break_even_trail_enabled {
        return;
    }
    let position = db::paper_get_active_position(pool, symbol).await;
    let pos = match position {
        Some(ref p) => p,
        None => return,
    };
    let entry = pos.average_entry_price.unwrap_or(pos.entry_price);
    let _ = sqlx::query("UPDATE active_positions SET final_invalidation_level = ?2 WHERE symbol = ?1")
        .bind(symbol)
        .bind(entry)
        .execute(pool)
        .await;
    println!(
        "📄 Break-Even Trail: {} TP filled — SL moved to entry ${:.2} (avg entry of remaining slots)",
        symbol, entry
    );
}

/// Evaluate whether the cumulative opposite-signal score warrants an exit.
///
/// Uses the registry-driven confluence model: exit triggers when the opposing-
/// indicator weighted contribution sum exceeds the calibrated threshold on the
/// unified ±100 scale (60% conviction bar).
pub fn evaluate_opposite_exit(
    position_direction: &str,
    snap: &SnapshotValues,
    _support_levels: &[f64],
    _resistance_levels: &[f64],
    _macro_trend: &str,
    _max_opposite: u32,
    regime_multipliers: Option<&HashMap<String, HashMap<String, f64>>>,
) -> (bool, u32) {
    let opposite_score = crate::profile_evaluation::calculate_registry_opposite_score(
        position_direction,
        snap,
        &std::collections::HashMap::new(),
        &std::collections::HashMap::new(),
        regime_multipliers,
    );
    let threshold = crate::profile_evaluation::scoring::REGISTRY_OPPOSITE_EXIT_THRESHOLD as u32;
    (opposite_score > threshold, opposite_score)
}

pub async fn verify_margin_and_open_with_alloc_and_pct(
    pool: &SqlitePool,
    telemetry_tx: &mpsc::Sender<db::TelemetryMsg>,
    symbol: &str,
    direction: &str,
    current_price: f64,
    allocation_pct_override: Option<f64>,
) -> Result<PaperTradeResult, String> {
    Ok(verify_margin_and_open_with_alloc(pool, telemetry_tx, symbol, direction, current_price, allocation_pct_override).await)
}

// ─── 4-Portion Dynamic Margin State Machine ─────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SlotState {
    pub slot_index: usize,
    pub is_active: bool,
    pub entry_price: f64,
    pub size: f64,
    pub allocated_usd: f64,
}

/// Result type for portion (slot) operations.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PaperSlotOpResult {
    pub success: bool,
    pub message: String,
    pub slot_index: i32,
    pub size: f64,
    pub allocated_usd: f64,
    pub realized_pnl: f64,
    pub refunded_usd: f64,
    pub active_count: i32,
    pub direction: String,
}

/// Calculate the dynamic margin for a newly opened portion slot.
/// C_cycle = initial_allocated_margin + realized_pnl_accumulator
/// K_new = (C_cycle - sum(K_active)) / N_vacant, capped by available_cash.
///
/// When N_vacant == 1 (final slot), sweeps all remaining cash to complete the cycle.
pub fn calculate_slot_margin(
    total_cycle_capital: f64,
    active_slots: &[SlotState],
    available_cash: f64,
) -> Result<f64, String> {
    if available_cash <= 0.0 {
        return Err("No cash available for allocation".into());
    }
    let active_count = active_slots.iter().filter(|s| s.is_active).count();
    if active_count >= 4 {
        return Err("Position fully allocated (4/4 portions active)".into());
    }
    let vacant_count = 4 - active_count;
    let locked_margin: f64 = active_slots.iter().filter(|s| s.is_active).map(|s| s.allocated_usd).sum();
    let unallocated_capital = total_cycle_capital - locked_margin;

    if unallocated_capital <= 0.0 {
        return Err("No unallocated margin remains in this position cycle".into());
    }

    if vacant_count == 1 {
        Ok(available_cash)
    } else {
        let required = unallocated_capital / vacant_count as f64;
        Ok(required.min(available_cash))
    }
}

/// Recalculate aggregate position fields from active slot data and update.
pub async fn recalculate_position_aggregates(
    pool: &SqlitePool,
    symbol: &str,
) -> Result<(), sqlx::Error> {
    let active_slots: Vec<db::PositionSlotRecord> = db::paper_get_active_slots(pool, symbol).await;

    if active_slots.is_empty() {
        // No active slots — clean up position
        if let Some(pos) = db::paper_get_active_position(pool, symbol).await {
            let _now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64;
            sqlx::query("DELETE FROM active_positions WHERE id = ?1")
                .bind(pos.id)
                .execute(pool)
                .await?;
            sqlx::query("DELETE FROM position_slots WHERE position_id = ?1")
                .bind(pos.id)
                .execute(pool)
                .await?;
            sqlx::query("DELETE FROM open_orders WHERE associated_position_id = ?1")
                .bind(pos.id)
                .execute(pool)
                .await?;
        }
        return Ok(());
    }

    let total_size: f64 = active_slots.iter().map(|s| s.size).sum();
    let total_allocated: f64 = active_slots.iter().map(|s| s.allocated_usd).sum();
    let weighted_price = if total_size > 0.0 {
        active_slots.iter().map(|s| s.entry_price * s.size).sum::<f64>() / total_size
    } else {
        0.0
    };
    let portion_count = active_slots.len() as i32;
    let _direction = &active_slots[0].direction;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    sqlx::query(
        "UPDATE active_positions SET size = ?2, allocated_usd = ?3, average_entry_price = ?4, current_portions = ?5, entry_timestamp = ?6 WHERE symbol = ?1"
    )
    .bind(symbol)
    .bind(total_size)
    .bind(total_allocated)
    .bind(weighted_price)
    .bind(portion_count)
    .bind(now)
    .execute(pool)
    .await?;

    Ok(())
}

/// Open a portion slot for the given symbol + direction at current market price.
/// Uses dynamic margin allocation: K_new = (C_cycle - sum(K_active)) / N_vacant.
pub async fn open_slot_internal(
    pool: &SqlitePool,
    _telemetry_tx: &mpsc::Sender<db::TelemetryMsg>,
    symbol: &str,
    direction: &str,
    current_price: f64,
) -> PaperSlotOpResult {
    if current_price <= 0.0 {
        return PaperSlotOpResult {
            success: false,
            message: "Invalid market price".into(),
            slot_index: -1, size: 0.0, allocated_usd: 0.0,
            realized_pnl: 0.0, refunded_usd: 0.0,
            active_count: 0, direction: direction.to_string(),
        };
    }

    let active_slots = db::paper_get_active_slots(pool, symbol).await;
    let active_count = active_slots.len() as i32;

    // Netting check: if opposite direction has active slots, close them all first
    if active_count > 0 && active_slots[0].direction != direction {
        return PaperSlotOpResult {
            success: false,
            message: format!("Cannot open {}: active {} position exists. Close it first or open in the same direction.", direction, active_slots[0].direction),
            slot_index: -1, size: 0.0, allocated_usd: 0.0,
            realized_pnl: 0.0, refunded_usd: 0.0,
            active_count, direction: active_slots[0].direction.clone(),
        };
    }

    if active_count >= 4 {
        return PaperSlotOpResult {
            success: false,
            message: "Position fully allocated (4/4 portions active)".into(),
            slot_index: -1, size: 0.0, allocated_usd: 0.0,
            realized_pnl: 0.0, refunded_usd: 0.0,
            active_count, direction: direction.to_string(),
        };
    }

    // Fetch balance once — used for cycle capital calc and cash-capped margin
    let balance = db::paper_get_balance(pool, symbol).await;

    // Determine cycle capital
    let pos = db::paper_get_active_position(pool, symbol).await;
    let cycle_capital = if let Some(ref p) = pos {
        let initial_margin = p.initial_allocated_margin.unwrap_or(p.allocated_usd);
        let realized_accum = p.realized_pnl_accumulator.unwrap_or(0.0);
        initial_margin + realized_accum
    } else {
        // Fresh cycle: use the balance's paper_initial_usd as first allocation
        balance.initial_usd
    };

    let active_state: Vec<SlotState> = active_slots.iter().map(|s| SlotState {
        slot_index: s.slot_index as usize,
        is_active: s.is_active,
        entry_price: s.entry_price,
        size: s.size,
        allocated_usd: s.allocated_usd,
    }).collect();

    let new_margin = match calculate_slot_margin(cycle_capital, &active_state, balance.current_cash) {
        Ok(m) => m,
        Err(e) => {
            return PaperSlotOpResult {
                success: false,
                message: e,
                slot_index: -1, size: 0.0, allocated_usd: 0.0,
                realized_pnl: 0.0, refunded_usd: 0.0,
                active_count, direction: direction.to_string(),
            };
        }
    };

    if balance.current_cash < new_margin {
        return PaperSlotOpResult {
            success: false,
            message: format!("Insufficient cash: need ${:.2}, have ${:.2}", new_margin, balance.current_cash),
            slot_index: -1, size: 0.0, allocated_usd: 0.0,
            realized_pnl: 0.0, refunded_usd: 0.0,
            active_count, direction: direction.to_string(),
        };
    }

    let slot_index = match db::paper_find_vacant_slot(pool, symbol).await {
        Some(idx) => idx,
        None => {
            return PaperSlotOpResult {
                success: false,
                message: "No vacant slot available (all 4 slots occupied)".into(),
                slot_index: -1, size: 0.0, allocated_usd: 0.0,
                realized_pnl: 0.0, refunded_usd: 0.0,
                active_count, direction: direction.to_string(),
            };
        }
    };

    let size = new_margin / current_price;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    let mut tx = match pool.begin().await {
        Ok(t) => t,
        Err(e) => {
            return PaperSlotOpResult {
                success: false,
                message: format!("DB error: {}", e),
                slot_index: -1, size: 0.0, allocated_usd: 0.0,
                realized_pnl: 0.0, refunded_usd: 0.0,
                active_count, direction: direction.to_string(),
            };
        }
    };

    // Ensure active_positions row exists
    let position_id: i64 = if let Some(ref p) = pos {
        p.id
    } else {
        let result = sqlx::query(
            "INSERT INTO active_positions (symbol, direction, entry_price, size, allocated_usd, entry_timestamp, average_entry_price, current_portions, final_invalidation_level, initial_allocated_margin, realized_pnl_accumulator)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?3, 1, 0, ?7, 0.0)"
        )
        .bind(symbol)
        .bind(direction)
        .bind(current_price)
        .bind(size)
        .bind(new_margin)
        .bind(now)
        .bind(cycle_capital)
        .execute(&mut *tx)
        .await;

        match result {
            Ok(_) => {
                let row = sqlx::query_as::<_, (i64,)>("SELECT id FROM active_positions WHERE symbol = ?1")
                    .bind(symbol)
                    .fetch_one(&mut *tx)
                    .await;
                match row {
                    Ok(r) => r.0,
                    Err(e) => { let _ = tx.rollback().await; return PaperSlotOpResult { success: false, message: format!("Failed to get position ID: {}", e), slot_index: -1, size: 0.0, allocated_usd: 0.0, realized_pnl: 0.0, refunded_usd: 0.0, active_count, direction: direction.to_string() }; }
                }
            }
            Err(e) => { let _ = tx.rollback().await; return PaperSlotOpResult { success: false, message: format!("Failed to create position: {}", e), slot_index: -1, size: 0.0, allocated_usd: 0.0, realized_pnl: 0.0, refunded_usd: 0.0, active_count, direction: direction.to_string() }; }
        }
    };

    // Insert the slot record
    if let Err(e) = sqlx::query(
        "INSERT INTO position_slots (position_id, symbol, direction, slot_index, is_active, entry_price, size, allocated_usd, realized_pnl, timestamp)
         VALUES (?1, ?2, ?3, ?4, 1, ?5, ?6, ?7, 0.0, ?8)"
    )
    .bind(position_id)
    .bind(symbol)
    .bind(direction)
    .bind(slot_index)
    .bind(current_price)
    .bind(size)
    .bind(new_margin)
    .bind(now)
    .execute(&mut *tx)
    .await {
        let _ = tx.rollback().await;
        return PaperSlotOpResult {
            success: false,
            message: format!("Failed to insert slot: {}", e),
            slot_index: -1, size: 0.0, allocated_usd: 0.0,
            realized_pnl: 0.0, refunded_usd: 0.0,
            active_count, direction: direction.to_string(),
        };
    }

    // Deduct from paper balance
    if let Err(e) = sqlx::query("UPDATE paper_balances SET current_cash = current_cash - ?2 WHERE symbol = ?1")
        .bind(symbol)
        .bind(new_margin)
        .execute(&mut *tx)
        .await {
        let _ = tx.rollback().await;
        return PaperSlotOpResult {
            success: false,
            message: format!("Failed to deduct balance: {}", e),
            slot_index: -1, size: 0.0, allocated_usd: 0.0,
            realized_pnl: 0.0, refunded_usd: 0.0,
            active_count, direction: direction.to_string(),
        };
    }

    let _ = tx.commit().await;

    // Recalculate aggregates
    let _ = recalculate_position_aggregates(pool, symbol).await;

    PaperSlotOpResult {
        success: true,
        message: format!("Opened {} slot {}: ${:.2} margin, {:.5} size at ${:.2}", direction, slot_index, new_margin, size, current_price),
        slot_index,
        size,
        allocated_usd: new_margin,
        realized_pnl: 0.0,
        refunded_usd: 0.0,
        active_count: active_count + 1,
        direction: direction.to_string(),
    }
}

/// Close the oldest active slot (FIFO) for the given symbol.
/// Returns the allocated margin + realized P&L to the paper balance.
pub async fn close_slot_internal(
    pool: &SqlitePool,
    telemetry_tx: &mpsc::Sender<db::TelemetryMsg>,
    symbol: &str,
    current_price: f64,
    trigger: &str,
) -> PaperSlotOpResult {
    if current_price <= 0.0 {
        return PaperSlotOpResult {
            success: false,
            message: "Invalid market price".into(),
            slot_index: -1, size: 0.0, allocated_usd: 0.0,
            realized_pnl: 0.0, refunded_usd: 0.0,
            active_count: 0, direction: String::new(),
        };
    }

    let active_count = db::paper_get_active_slot_count(pool, symbol).await;
    if active_count == 0 {
        return PaperSlotOpResult {
            success: false,
            message: "No active slots to close".into(),
            slot_index: -1, size: 0.0, allocated_usd: 0.0,
            realized_pnl: 0.0, refunded_usd: 0.0,
            active_count: 0, direction: String::new(),
        };
    }

    let oldest = match db::paper_get_oldest_active_slot(pool, symbol).await {
        Some(s) => s,
        None => {
            return PaperSlotOpResult {
                success: false,
                message: "No active slot found".into(),
                slot_index: -1, size: 0.0, allocated_usd: 0.0,
                realized_pnl: 0.0, refunded_usd: 0.0,
                active_count: 0, direction: String::new(),
            };
        }
    };

    let pnl = if oldest.direction == "LONG" {
        (current_price - oldest.entry_price) * oldest.size
    } else {
        (oldest.entry_price - current_price) * oldest.size
    };

    let is_maker = trigger.to_uppercase().contains("TP");
    let fee_rate = if is_maker { 0.0002 } else { 0.0006 };
    let commission_fee = oldest.size * current_price * fee_rate;
    let net_pnl = pnl - commission_fee;
    let refund = oldest.allocated_usd + net_pnl;
    let direction = oldest.direction.clone();
    let slot_index = oldest.slot_index;
    let position_id = oldest.position_id;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    let mut tx = match pool.begin().await {
        Ok(t) => t,
        Err(e) => {
            return PaperSlotOpResult {
                success: false,
                message: format!("DB error: {}", e),
                slot_index, size: 0.0, allocated_usd: 0.0,
                realized_pnl: pnl, refunded_usd: 0.0,
                active_count, direction,
            };
        }
    };

    // Mark slot inactive
    if let Err(e) = sqlx::query("UPDATE position_slots SET is_active = 0, realized_pnl = ?2 WHERE id = ?1")
        .bind(oldest.id)
        .bind(net_pnl)
        .execute(&mut *tx)
        .await {
        let _ = tx.rollback().await;
        return PaperSlotOpResult {
            success: false,
            message: format!("Failed to update slot: {}", e),
            slot_index, size: oldest.size, allocated_usd: oldest.allocated_usd,
            realized_pnl: net_pnl, refunded_usd: 0.0,
            active_count, direction,
        };
    }

    // FIFO bracket cleanup: delete oldest TP and SL orders for this position
    let _ = sqlx::query(
        "DELETE FROM open_orders WHERE id = (SELECT id FROM open_orders WHERE associated_position_id = ?1 AND order_type = 'LIMIT' AND is_reduce_only = 1 ORDER BY created_at ASC LIMIT 1)"
    )
    .bind(position_id)
    .execute(&mut *tx)
    .await;
    let _ = sqlx::query(
        "DELETE FROM open_orders WHERE id = (SELECT id FROM open_orders WHERE associated_position_id = ?1 AND order_type = 'STOP' AND is_reduce_only = 1 ORDER BY created_at ASC LIMIT 1)"
    )
    .bind(position_id)
    .execute(&mut *tx)
    .await;

    // Update realized_pnl_accumulator on active_positions
    if let Some(pos) = db::paper_get_active_position(pool, symbol).await {
        let current_accum = pos.realized_pnl_accumulator.unwrap_or(0.0);
        let _ = sqlx::query("UPDATE active_positions SET realized_pnl_accumulator = ?2 WHERE symbol = ?1")
            .bind(symbol)
            .bind(current_accum + net_pnl)
            .execute(&mut *tx)
            .await;
    }

    // Refund margin + PnL to paper balance
    if let Err(e) = sqlx::query("UPDATE paper_balances SET current_cash = current_cash + ?2 WHERE symbol = ?1")
        .bind(symbol)
        .bind(refund)
        .execute(&mut *tx)
        .await {
        let _ = tx.rollback().await;
        return PaperSlotOpResult {
            success: false,
            message: format!("Failed to refund balance: {}", e),
            slot_index, size: oldest.size, allocated_usd: oldest.allocated_usd,
            realized_pnl: pnl, refunded_usd: 0.0,
            active_count, direction,
        };
    }

    // Record trade
    let _pos = db::paper_get_active_position(pool, symbol).await;
    let roi_pct = if oldest.allocated_usd > 0.0 {
        (net_pnl / oldest.allocated_usd) * 100.0
    } else {
        0.0
    };

    let regime: String = sqlx::query_scalar(
        "SELECT COALESCE(market_regime, 'RANGE') FROM master_assistant_records WHERE symbol = ?1 AND created_at <= ?2 ORDER BY id DESC LIMIT 1"
    )
    .bind(symbol)
    .bind(now / 1000)
    .fetch_optional(pool)
    .await
    .unwrap_or(None)
    .unwrap_or_else(|| "RANGE".to_string());

    sqlx::query(
        "INSERT INTO paper_trades (symbol, direction, entry_price, exit_price, size, realized_pnl, roi_pct, entry_timestamp, exit_timestamp, trigger, market_regime)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)"
    )
    .bind(symbol)
    .bind(&direction)
    .bind(oldest.entry_price)
    .bind(current_price)
    .bind(oldest.size)
    .bind(net_pnl)
    .bind(roi_pct)
    .bind(oldest.timestamp)
    .bind(now)
    .bind(trigger)
    .bind(&regime)
    .execute(&mut *tx)
    .await
    .ok();

    sqlx::query(
        "INSERT INTO trade_telemetry_history (exchange, symbol, direction, entry_timestamp, exit_timestamp, entry_price, exit_price, size, realized_pnl, roi_percentage, trigger_source)
         VALUES ('PAPER', ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"
    )
    .bind(symbol)
    .bind(&direction)
    .bind(oldest.timestamp)
    .bind(now)
    .bind(oldest.entry_price)
    .bind(current_price)
    .bind(oldest.size)
    .bind(net_pnl)
    .bind(roi_pct)
    .bind(trigger)
    .execute(&mut *tx)
    .await
    .ok();

    let _ = tx.commit().await;

    // Recalculate aggregates; will clean up if no active slots remain
    let _ = recalculate_position_aggregates(pool, symbol).await;

    // Journal trade
    let _ = telemetry_tx.send(db::TelemetryMsg::JournalTrade {
        symbol: symbol.to_string(),
        direction: direction.clone(),
        entry_price: oldest.entry_price,
        exit_price: current_price,
        entry_timestamp: oldest.timestamp,
        exit_timestamp: now,
        size: oldest.size,
        realized_pnl: net_pnl,
        roi_pct,
        allocated_usd: oldest.allocated_usd,
        trigger: trigger.to_string(),
    }).await;

    let new_active = db::paper_get_active_slot_count(pool, symbol).await;

    PaperSlotOpResult {
        success: true,
        message: format!("Closed {} slot {}: PnL ${:.2}, refunded ${:.2}", direction, slot_index, net_pnl, refund),
        slot_index,
        size: oldest.size,
        allocated_usd: oldest.allocated_usd,
        realized_pnl: net_pnl,
        refunded_usd: refund,
        active_count: new_active,
        direction,
    }
}

/// Netting close: close all active slots for the symbol (opposite direction entry).
pub async fn close_all_slots(
    pool: &SqlitePool,
    telemetry_tx: &mpsc::Sender<db::TelemetryMsg>,
    symbol: &str,
    current_price: f64,
    trigger: &str,
) -> PaperSlotOpResult {
    let mut total_pnl = 0.0;
    let mut total_refund = 0.0;
    let active_count = db::paper_get_active_slot_count(pool, symbol).await;
    let direction = db::paper_get_active_slots(pool, symbol).await
        .first()
        .map(|s| s.direction.clone())
        .unwrap_or_default();

    if active_count == 0 {
        return PaperSlotOpResult {
            success: true,
            message: "No active slots to close".into(),
            slot_index: -1, size: 0.0, allocated_usd: 0.0,
            realized_pnl: 0.0, refunded_usd: 0.0,
            active_count: 0, direction,
        };
    }

    for _ in 0..active_count {
        let result = close_slot_internal(pool, telemetry_tx, symbol, current_price, trigger).await;
        if result.success {
            total_pnl += result.realized_pnl;
            total_refund += result.refunded_usd;
        }
    }

    PaperSlotOpResult {
        success: true,
        message: format!("Closed all {} slots. Total PnL: ${:.2}, Total refund: ${:.2}", active_count, total_pnl, total_refund),
        slot_index: -1, size: 0.0, allocated_usd: 0.0,
        realized_pnl: total_pnl, refunded_usd: total_refund,
        active_count: 0, direction,
    }
}

/// Get slot states for the given symbol, padded to 4 slots.
pub async fn get_slot_states(pool: &SqlitePool, symbol: &str) -> Vec<SlotState> {
    let active_slots = db::paper_get_active_slots(pool, symbol).await;
    let mut states: Vec<SlotState> = (0..4).map(|i| SlotState {
        slot_index: i,
        is_active: false,
        entry_price: 0.0,
        size: 0.0,
        allocated_usd: 0.0,
    }).collect();

    for slot in &active_slots {
        let idx = slot.slot_index as usize;
        if idx < 4 {
            states[idx] = SlotState {
                slot_index: idx,
                is_active: true,
                entry_price: slot.entry_price,
                size: slot.size,
                allocated_usd: slot.allocated_usd,
            };
        }
    }
    states
}

/// Background funding fee decay tracker — runs every 8 hours.
/// Deducts holding costs from active position slots to prevent capital inflation.
pub async fn run_funding_decay_tracker(
    pool: SqlitePool,
    funding_rate_8h_pct: f64,
    cancel: tokio_util::sync::CancellationToken,
) {
    let interval = tokio::time::Duration::from_secs(8 * 3600);
    println!("💸 Funding Decay Tracker: started (8h interval, rate: {:.4}%)", funding_rate_8h_pct);

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                println!("🛑 Funding Decay Tracker: cancelled, shutting down.");
                break;
            }
            _ = tokio::time::sleep(interval) => {}
        }

        let funding_rate = funding_rate_8h_pct / 100.0;
        let positions = db::paper::queries::paper_get_all_active_positions(&pool).await;

        for pos in &positions {
            let current_price = get_latest_close_price(&pool, &pos.symbol).await;
            if current_price <= 0.0 {
                continue;
            }

            let active_slots = db::paper_get_active_slots(&pool, &pos.symbol).await;
            let mut total_funding_cost = 0.0;

            if let Ok(mut tx) = pool.begin().await {
                for slot in &active_slots {
                    let funding_cost = slot.size * current_price * funding_rate;
                    if funding_cost > 0.0 {
                        let new_allocated = (slot.allocated_usd - funding_cost).max(0.0);
                        let _ = sqlx::query(
                            "UPDATE position_slots SET allocated_usd = ?2 WHERE id = ?1 AND is_active = 1"
                        )
                        .bind(slot.id)
                        .bind(new_allocated)
                        .execute(&mut *tx)
                        .await;
                        total_funding_cost += funding_cost;
                    }
                }

                // Update master position aggregate
                let new_total_allocated = (pos.allocated_usd - total_funding_cost).max(0.0);
                let _ = sqlx::query(
                    "UPDATE active_positions SET allocated_usd = ?2 WHERE id = ?1"
                )
                .bind(pos.id)
                .bind(new_total_allocated)
                .execute(&mut *tx)
                .await;

                let _ = tx.commit().await;
            }

            if total_funding_cost > 0.01 {
                println!(
                    "💸 Funding Decay: {} debited ${:.4} across {} active slots (rate: {:.4}%)",
                    pos.symbol, total_funding_cost, active_slots.len(), funding_rate_8h_pct
                );
            }
        }
    }
}

async fn get_latest_close_price(pool: &SqlitePool, symbol: &str) -> f64 {
    use sqlx::Row;
    let row = sqlx::query(
        "SELECT CAST(close AS REAL) FROM market_snapshots WHERE symbol = ?1 AND close IS NOT NULL AND timeframe_secs = 60 ORDER BY timestamp DESC LIMIT 1"
    )
    .bind(symbol)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();
    row.map(|r| r.get::<f64, _>(0)).unwrap_or(0.0)
}
