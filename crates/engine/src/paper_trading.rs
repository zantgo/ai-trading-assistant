use sqlx::SqlitePool;
use tokio::sync::mpsc;
use rust_decimal::prelude::*;
use rust_decimal::Decimal;

use crate::db;
use crate::profile_evaluation::{calculate_opposite_score, SnapshotValues};

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

    fn validate_position_size(&self, _size: Decimal) -> Result<(), RiskError> {
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

    // Validate percentage
    if pct < 10.0 || pct > 100.0 || pct % 10.0 != 0.0 {
        return PaperPositionOpResult {
            success: false,
            message: format!("Position percentage must be 10-100 in steps of 10, got {}", pct),
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
            let net_pct = (pct - existing_pct).max(10.0);
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
            return PaperPositionOpResult {
                success: true,
                message: format!("Netted: closed {}%, opened {}% {:.0} at ${:.2}", existing_pct, net_pct, direction, current_price),
                direction: direction.to_string(),
                position_pct: net_pct,
                free_balance_pct: 100.0 - net_pct,
                entry_price: current_price, size, allocated_usd: allocated_f64,
            };
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
                    "INSERT OR REPLACE INTO active_positions (symbol, direction, entry_price, size, allocated_usd, entry_timestamp, average_entry_price, current_portions, final_invalidation_level)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?3, 1, 0)"
                )
                .bind(symbol)
                .bind(direction)
                .bind(current_price)
                .bind(size)
                .bind(allocated_f64)
                .bind(now)
                .execute(&mut *tx)
                .await
                .ok();

                sqlx::query("UPDATE paper_balances SET current_cash = current_cash - ?2 WHERE symbol = ?1")
                    .bind(symbol)
                    .bind(allocated_f64)
                    .execute(&mut *tx)
                    .await
                    .ok();

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

    if close_pct < 10.0 || close_pct > 100.0 || close_pct % 10.0 != 0.0 {
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

// ─── TP/SL Management (Percentage-Based) ───────────────────────────

/// Set take-profit targets as percentages of the current position.
/// Each target is a percentage (10-100, step 10) with a price.
pub async fn set_take_profit_targets(
    pool: &SqlitePool,
    symbol: &str,
    targets: &[(f64, f64)],  // (pct, price) pairs
) -> Result<String, String> {
    let position = db::paper_get_active_position(pool, symbol).await;
    let pos = position.ok_or("No active position".to_string())?;
    let balance = db::paper_get_balance(pool, symbol).await;
    let position_pct = if balance.initial_usd > 0.0 { (pos.allocated_usd / balance.initial_usd) * 100.0 } else { 0.0 };

    let total_tp_pct: f64 = targets.iter().map(|(p, _)| p).sum();
    if total_tp_pct > position_pct + 0.1 {
        return Err(format!("Total TP percentage ({:.0}%) exceeds position size ({:.0}%)", total_tp_pct, position_pct));
    }

    for (pct, _price) in targets {
        if *pct < 10.0 || *pct > 100.0 || *pct % 10.0 != 0.0 {
            return Err(format!("TP percentage must be 10-100 in steps of 10, got {}", pct));
        }
    }

    // Clear existing targets
    sqlx::query("DELETE FROM position_take_profit_targets WHERE symbol = ?1")
        .bind(symbol)
        .execute(pool)
        .await
        .map_err(|e| format!("DB error: {}", e))?;

    // Insert new targets
    for (pct, price) in targets {
        sqlx::query(
            "INSERT INTO position_take_profit_targets (symbol, target_price, size_fraction) VALUES (?1, ?2, ?3)"
        )
        .bind(symbol)
        .bind(price)
        .bind(pct / 100.0)
        .execute(pool)
        .await
        .map_err(|e| format!("DB error: {}", e))?;
    }

    Ok(format!("{} TP targets set", targets.len()))
}

/// Set stop-loss levels as percentages of the current position.
pub async fn set_stop_loss_levels(
    pool: &SqlitePool,
    symbol: &str,
    stops: &[(f64, f64)],  // (pct, price) pairs
) -> Result<String, String> {
    let position = db::paper_get_active_position(pool, symbol).await;
    let pos = position.ok_or("No active position".to_string())?;
    let balance = db::paper_get_balance(pool, symbol).await;
    let position_pct = if balance.initial_usd > 0.0 { (pos.allocated_usd / balance.initial_usd) * 100.0 } else { 0.0 };

    let total_sl_pct: f64 = stops.iter().map(|(p, _)| p).sum();
    if total_sl_pct > position_pct + 0.1 {
        return Err(format!("Total SL percentage ({:.0}%) exceeds position size ({:.0}%)", total_sl_pct, position_pct));
    }

    for (pct, _price) in stops {
        if *pct < 10.0 || *pct > 100.0 || *pct % 10.0 != 0.0 {
            return Err(format!("SL percentage must be 10-100 in steps of 10, got {}", pct));
        }
    }

    let direction = &pos.direction;
    for (_pct, price) in stops {
        if direction == "LONG" && *price >= pos.entry_price {
            return Err("Long SL must be below entry price".to_string());
        }
        if direction == "SHORT" && *price <= pos.entry_price {
            return Err("Short SL must be above entry price".to_string());
        }
    }

    // Store SL info in final_invalidation_level (use the closest/worst stop)
    let worst_sl = if direction == "LONG" {
        stops.iter().map(|(_, p)| p).fold(f64::INFINITY, |a, &b| a.min(b))
    } else {
        stops.iter().map(|(_, p)| p).fold(f64::NEG_INFINITY, |a, &b| a.max(b))
    };

    sqlx::query("UPDATE active_positions SET final_invalidation_level = ?2 WHERE symbol = ?1")
        .bind(symbol)
        .bind(worst_sl)
        .execute(pool)
        .await
        .map_err(|e| format!("DB error: {}", e))?;

    Ok(format!("{} SL levels set", stops.len()))
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

    let validator = PositionRiskValidator::new(balance.current_cash, balance.max_risk_pct, balance.leverage);
    if let Err(e) = validator.validate_trade_cost(total_allocation) {
        return PaperTradeResult {
            success: false,
            message: format!("{}", e),
            entry_price: None, size: None, allocated_usd: None, position_pct: None,
        };
    }

    let size = total_allocation / price_dec;
    let alloc_f64 = f64_from_dec(total_allocation);

    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as i64;
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
pub async fn check_break_even_trail(pool: &SqlitePool, symbol: &str, current_price: f64) -> bool {
    let position = db::paper_get_active_position(pool, symbol).await;
    let pos = match position {
        Some(ref p) => p,
        None => return false,
    };

    let entry = pos.average_entry_price.unwrap_or(pos.entry_price);
    let targets = sqlx::query_as::<_, (i64, f64, f64, i64)>(
        "SELECT id, target_price, size_fraction, is_hit FROM position_take_profit_targets
         WHERE symbol = ?1 AND is_hit = 0 ORDER BY target_price ASC",
    )
    .bind(symbol)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    for (target_id, target_price, _size_fraction, _is_hit) in &targets {
        let tp_hit = if pos.direction == "LONG" {
            current_price >= *target_price
        } else {
            current_price <= *target_price
        };

        if tp_hit {
            sqlx::query("UPDATE position_take_profit_targets SET is_hit = 1 WHERE id = ?1")
                .bind(target_id)
                .execute(pool)
                .await
                .ok();

            let new_invalidation = entry;
            sqlx::query("UPDATE active_positions SET final_invalidation_level = ?2 WHERE symbol = ?1")
                .bind(symbol)
                .bind(new_invalidation)
                .execute(pool)
                .await
                .ok();

            println!("📄 Break-Even Trail: {} TP1 hit at ${:.2}. SL moved to entry ${:.2}", symbol, target_price, entry);
            return true;
        }
    }
    false
}

pub fn evaluate_opposite_exit(
    position_direction: &str,
    snap: &SnapshotValues,
    support_levels: &[f64],
    resistance_levels: &[f64],
    macro_trend: &str,
    max_opposite: u32,
) -> (bool, u32) {
    let opposite_score = calculate_opposite_score(position_direction, snap, support_levels, resistance_levels, macro_trend);
    (opposite_score > max_opposite, opposite_score)
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
