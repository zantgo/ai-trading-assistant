use sqlx::SqlitePool;
use tokio::sync::mpsc;
use rust_decimal::prelude::*;
use rust_decimal::Decimal;

use crate::db;
use crate::profile_evaluation::{self, calculate_opposite_score, SnapshotValues};

#[inline]
fn dec(v: f64) -> Decimal {
    Decimal::from_f64(v).unwrap_or(Decimal::ZERO)
}

#[inline]
fn f64_from_dec(d: Decimal) -> f64 {
    d.to_f64().unwrap_or(0.0)
}

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

#[derive(Debug)]
enum RiskError {
    InsufficientCapital { required: Decimal, available: Decimal },
    PositionTooLarge { max: Decimal, requested: Decimal },
    InvalidStopLoss { reason: String },
}

impl std::fmt::Display for RiskError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InsufficientCapital { required, available } => {
                write!(
                    f,
                    "Insufficient capital: required ${:.2}, available ${:.2}",
                    required, available
                )
            }
            Self::PositionTooLarge { max, requested } => {
                write!(
                    f,
                    "Position too large: max {:.4} units, requested {:.4}",
                    max, requested
                )
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
    max_risk_pct: Decimal,
    leverage: i32,
}

impl PositionRiskValidator {
    fn new(capital: f64, max_risk_pct: f64, leverage: i32) -> Self {
        Self {
            capital: dec(capital),
            max_risk_pct: dec(max_risk_pct),
            leverage,
        }
    }

    fn validate_trade_cost(&self, cost: Decimal) -> Result<Decimal, RiskError> {
        if cost <= Decimal::ZERO {
            return Err(RiskError::InsufficientCapital {
                required: cost,
                available: self.capital,
            });
        }
        if cost > self.capital {
            return Err(RiskError::InsufficientCapital {
                required: cost,
                available: self.capital,
            });
        }
        Ok(cost)
    }

    fn validate_position_size(
        &self,
        entry_price: Decimal,
        size: Decimal,
        stop_loss: Option<Decimal>,
    ) -> Result<Decimal, RiskError> {
        if entry_price <= Decimal::ZERO {
            return Err(RiskError::InvalidStopLoss {
                reason: "Entry price must be positive".to_string(),
            });
        }

        let trade_cost = entry_price * size;
        self.validate_trade_cost(trade_cost)?;

        if let Some(sl) = stop_loss {
            if sl <= Decimal::ZERO {
                return Err(RiskError::InvalidStopLoss {
                    reason: "Stop loss must be positive".to_string(),
                });
            }
        }

        Ok(size)
    }

    fn max_position_size(&self, entry_price: Decimal) -> Decimal {
        if entry_price <= Decimal::ZERO {
            return Decimal::ZERO;
        }
        self.capital / entry_price
    }
}

pub async fn verify_margin_and_open(
    pool: &SqlitePool,
    telemetry_tx: &mpsc::Sender<db::TelemetryMsg>,
    symbol: &str,
    direction: &str,
    current_price: f64,
) -> PaperTradeResult {
    verify_margin_and_open_with_alloc(pool, telemetry_tx, symbol, direction, current_price, None)
        .await
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
            message: format!(
                "{} already has an active {} position",
                symbol,
                position.unwrap().direction
            ),
            entry_price: None,
            size: None,
            allocated_usd: None,
        };
    }

    let price_dec = dec(current_price);
    if price_dec <= Decimal::ZERO {
        return PaperTradeResult {
            success: false,
            message: format!(
                "Invalid current price for {}: ${:.4}",
                symbol, current_price
            ),
            entry_price: None,
            size: None,
            allocated_usd: None,
        };
    }

    let validator = PositionRiskValidator::new(
        balance.current_cash,
        balance.max_risk_pct,
        balance.leverage,
    );

    let alloc_pct = allocation_pct_override.unwrap_or(balance.allocation_pct);
    let alloc_pct_dec = dec(alloc_pct);
    let capital_dec = dec(balance.current_cash);
    let trade_cost = capital_dec * alloc_pct_dec / dec(100.0);

    if let Err(e) = validator.validate_trade_cost(trade_cost) {
        return PaperTradeResult {
            success: false,
            message: format!(
                "Insufficient margin for {}. Cash: ${:.2}, Allocation: {:.1}% → Trade Cost: ${:.2} — {}",
                symbol, balance.current_cash, balance.allocation_pct, f64_from_dec(trade_cost), e
            ),
            entry_price: None,
            size: None,
            allocated_usd: None,
        };
    }

    let position_size = trade_cost / price_dec;
    let position_size_f64 = f64_from_dec(position_size);
    let trade_cost_f64 = f64_from_dec(trade_cost);

    if let Err(e) =
        sqlx::query("UPDATE paper_balances SET current_cash = current_cash - ?2 WHERE symbol = ?1")
            .bind(symbol)
            .bind(trade_cost_f64)
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

    let _ = telemetry_tx
        .send(db::TelemetryMsg::PaperOpenPosition {
            symbol: symbol.to_string(),
            direction: direction.to_string(),
            entry_price: current_price,
            size: position_size_f64,
            allocated_usd: trade_cost_f64,
        })
        .await;

    PaperTradeResult {
        success: true,
        message: format!(
            "OPEN {} {} | Entry: ${:.2} | Size: {:.4} units | Allocated: ${:.2} ({:.0}% of ${:.2})",
            symbol,
            direction,
            current_price,
            position_size_f64,
            trade_cost_f64,
            alloc_pct,
            balance.current_cash + trade_cost_f64
        ),
        entry_price: Some(current_price),
        size: Some(position_size_f64),
        allocated_usd: Some(trade_cost_f64),
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
            let entry_dec = dec(entry);
            let exit_dec = dec(exit_price);
            let size_dec = dec(pos.size);
            let alloc_dec = dec(pos.allocated_usd);

            let realized_pnl_dec = if pos.direction == "LONG" {
                (exit_dec - entry_dec) * size_dec
            } else {
                (entry_dec - exit_dec) * size_dec
            };
            let realized_pnl = f64_from_dec(realized_pnl_dec);

            let roi_pct = if alloc_dec > Decimal::ZERO {
                f64_from_dec((realized_pnl_dec / alloc_dec) * dec(100.0))
            } else {
                0.0
            };

            let _ = telemetry_tx
                .send(db::TelemetryMsg::PaperClosePosition {
                    symbol: symbol.to_string(),
                    exit_price,
                    exit_timestamp: now,
                    trigger: trigger.to_string(),
                })
                .await;

            let _ = telemetry_tx
                .send(db::TelemetryMsg::JournalTrade {
                    symbol: symbol.to_string(),
                    direction: pos.direction.clone(),
                    entry_price: entry,
                    exit_price,
                    entry_timestamp: pos.entry_timestamp,
                    exit_timestamp: now,
                    size: pos.size,
                    realized_pnl,
                    roi_pct,
                    allocated_usd: pos.allocated_usd,
                    trigger: trigger.to_string(),
                })
                .await;

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

    let validator = PositionRiskValidator::new(
        balance.current_cash,
        balance.max_risk_pct,
        balance.leverage,
    );

    let init_dec = dec(balance.initial_usd);
    let alloc_pct_dec = dec(balance.allocation_pct);
    let total_allocation = init_dec * alloc_pct_dec / dec(100.0);
    let portion_cost = total_allocation / dec(3.0);

    if let Err(e) = validator.validate_trade_cost(portion_cost) {
        return PaperScaleInResult {
            success: false,
            message: format!(
                "Insufficient capital for portion {}. Need ${:.2}, have ${:.2} — {}",
                portion_number,
                f64_from_dec(portion_cost),
                balance.current_cash,
                e
            ),
            new_average_entry_price: 0.0,
            total_size: 0.0,
            portion_number,
        };
    }

    let portion_cost_f64 = f64_from_dec(portion_cost);
    let price_dec = dec(entry_price);

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
            message: format!(
                "Cannot {} scale into existing {} position",
                direction, pos.direction
            ),
            new_average_entry_price: pos.average_entry_price.unwrap_or(pos.entry_price),
            total_size: pos.size,
            portion_number,
        },
        Some(pos) => {
            // Scale into existing position
            let new_size = portion_cost / price_dec;
            let old_avg_dec = dec(pos.average_entry_price.unwrap_or(pos.entry_price));
            let old_size_dec = dec(pos.size);
            let new_avg_dec = ((old_avg_dec * old_size_dec) + (price_dec * new_size))
                / (old_size_dec + new_size);
            let total_size_dec = old_size_dec + new_size;
            let total_size = f64_from_dec(total_size_dec);
            let new_avg = f64_from_dec(new_avg_dec);
            let new_size_f64 = f64_from_dec(new_size);

            // Deduct cash
            sqlx::query(
                "UPDATE paper_balances SET current_cash = current_cash - ?2 WHERE symbol = ?1",
            )
            .bind(symbol)
            .bind(portion_cost_f64)
            .execute(&*pool)
            .await
            .ok();

            let _ = telemetry_tx
                .send(db::TelemetryMsg::PaperScaleInPortion {
                    symbol: symbol.to_string(),
                    direction: direction.to_string(),
                    entry_price,
                    size: new_size_f64,
                    allocated_usd: portion_cost_f64,
                    portion_number,
                    new_average_entry_price: new_avg,
                    total_size,
                    final_invalidation_level,
                })
                .await;

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
            let new_size = portion_cost / price_dec;
            let new_size_f64 = f64_from_dec(new_size);

            sqlx::query(
                "UPDATE paper_balances SET current_cash = current_cash - ?2 WHERE symbol = ?1",
            )
            .bind(symbol)
            .bind(portion_cost_f64)
            .execute(&*pool)
            .await
            .ok();

            let _ = telemetry_tx
                .send(db::TelemetryMsg::PaperScaleInPortion {
                    symbol: symbol.to_string(),
                    direction: direction.to_string(),
                    entry_price,
                    size: new_size_f64,
                    allocated_usd: portion_cost_f64,
                    portion_number: 1,
                    new_average_entry_price: entry_price,
                    total_size: new_size_f64,
                    final_invalidation_level,
                })
                .await;

            PaperScaleInResult {
                success: true,
                message: format!(
                    "OPEN (Portion 1/3): {} {} @ ${:.2} | Size: {:.4} | Allocated: ${:.2}",
                    symbol, direction, entry_price, new_size_f64, portion_cost_f64
                ),
                new_average_entry_price: entry_price,
                total_size: new_size_f64,
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
            let entry_dec = dec(entry);
            let exit_dec = dec(exit_price);
            let size_dec = dec(pos.size);
            let frac_dec = dec(size_fraction);

            let close_size_dec = size_dec * frac_dec;
            let remaining_size_dec = size_dec - close_size_dec;
            let remaining_size = f64_from_dec(remaining_size_dec).max(0.0);

            let realized_pnl_dec = if pos.direction == "LONG" {
                (exit_dec - entry_dec) * close_size_dec
            } else {
                (entry_dec - exit_dec) * close_size_dec
            };
            let realized_pnl = f64_from_dec(realized_pnl_dec);

            if remaining_size_dec <= Decimal::ZERO {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as i64;
                let _ = telemetry_tx
                    .send(db::TelemetryMsg::PaperClosePosition {
                        symbol: symbol.to_string(),
                        exit_price,
                        exit_timestamp: now,
                        trigger: trigger.to_string(),
                    })
                    .await;
            } else {
                let _ = telemetry_tx
                    .send(db::TelemetryMsg::PaperScaleOutPortion {
                        symbol: symbol.to_string(),
                        exit_price,
                        size_fraction,
                        realized_pnl,
                        remaining_size,
                        target_id,
                    })
                    .await;
            }

            PaperScaleOutResult {
                success: true,
                message: format!(
                    "SCALE-OUT: {} @ ${:.2} | {:.0}% closed | PnL: ${:.2} | Remaining: {:.4}",
                    symbol,
                    exit_price,
                    f64_from_dec(frac_dec * dec(100.0)),
                    realized_pnl,
                    remaining_size
                ),
                realized_pnl,
                remaining_size,
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
            let entry_dec = dec(entry);
            let exit_dec = dec(exit_price);
            let size_dec = dec(pos.size);
            let alloc_dec = dec(pos.allocated_usd);

            let realized_loss_dec = if pos.direction == "LONG" {
                (exit_dec - entry_dec) * size_dec
            } else {
                (entry_dec - exit_dec) * size_dec
            };
            let realized_loss = f64_from_dec(realized_loss_dec);

            let roi_pct = if alloc_dec > Decimal::ZERO {
                f64_from_dec((realized_loss_dec / alloc_dec) * dec(100.0))
            } else {
                0.0
            };

            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64;

            let _ = telemetry_tx
                .send(db::TelemetryMsg::PaperInvalidatePosition {
                    symbol: symbol.to_string(),
                    exit_price,
                    exit_timestamp: now,
                    realized_loss,
                    reason: reason.to_string(),
                })
                .await;

            let _ = telemetry_tx
                .send(db::TelemetryMsg::JournalTrade {
                    symbol: symbol.to_string(),
                    direction: pos.direction.clone(),
                    entry_price: entry,
                    exit_price,
                    entry_timestamp: pos.entry_timestamp,
                    exit_timestamp: now,
                    size: pos.size,
                    realized_pnl: realized_loss,
                    roi_pct,
                    allocated_usd: pos.allocated_usd,
                    trigger: format!("INVALIDATION:{}", reason),
                })
                .await;

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
        bias,
        snap,
        support_levels,
        resistance_levels,
        macro_trend,
    );
    let signals_json = serde_json::to_string(&score.signals).unwrap_or_default();
    (
        score.allocated_capital_pct,
        score.total_score,
        score.max_score,
        signals_json,
    )
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
        position_direction,
        snap,
        support_levels,
        resistance_levels,
        macro_trend,
    );
    (opposite_score > max_opposite, opposite_score)
}

/// Check if a break-even trailing update is needed.
/// When the current price crosses beyond TP1 (first take-profit target),
/// the stop-loss for the remaining position should be moved to the entry price.
pub async fn check_break_even_trail(pool: &SqlitePool, symbol: &str, current_price: f64) -> bool {
    let position = db::paper_get_active_position(pool, symbol).await;
    let pos = match position {
        Some(ref p) => p,
        None => return false,
    };

    let entry = pos.average_entry_price.unwrap_or(pos.entry_price);
    let targets = sqlx::query_as::<_, (i64, f64, f64, i64)>(
        "SELECT id, target_price, size_fraction, is_hit FROM position_take_profit_targets
         WHERE symbol = ?1 AND is_hit = 0
         ORDER BY target_price ASC",
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
                "UPDATE active_positions SET final_invalidation_level = ?2 WHERE symbol = ?1",
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
