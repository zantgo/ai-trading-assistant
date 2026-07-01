use sqlx::SqlitePool;

// ─── Structs ───────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize)]
pub struct PaperBalance {
    pub id: i64,
    pub symbol: String,
    pub initial_usd: f64,
    pub current_cash: f64,
    pub allocation_pct: f64,
    pub auto_execute: bool,
    pub max_risk_pct: f64,
    pub leverage: i32,
    pub auto_execute_intervals: i32,
    pub lookback_trades: i32,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ActivePaperPosition {
    pub id: i64,
    pub symbol: String,
    pub direction: String,
    pub entry_price: f64,
    pub size: f64,
    pub allocated_usd: f64,
    pub entry_timestamp: i64,
    pub average_entry_price: Option<f64>,
    pub current_portions: Option<i32>,
    pub final_invalidation_level: Option<f64>,
    pub target_profit_ratio: Option<f64>,
    pub initial_allocated_margin: Option<f64>,
    pub realized_pnl_accumulator: Option<f64>,
}

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct PositionSlotRecord {
    pub id: i64,
    pub position_id: i64,
    pub symbol: String,
    pub direction: String,
    pub slot_index: i32,
    pub is_active: bool,
    pub entry_price: f64,
    pub size: f64,
    pub allocated_usd: f64,
    pub realized_pnl: f64,
    pub timestamp: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PaperTradeRecord {
    pub id: i64,
    pub symbol: String,
    pub direction: String,
    pub entry_price: f64,
    pub exit_price: f64,
    pub size: f64,
    pub realized_pnl: f64,
    pub roi_pct: f64,
    pub entry_timestamp: i64,
    pub exit_timestamp: i64,
    pub trigger: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PaperAccountMetrics {
    pub symbol: String,
    pub initial_usd: f64,
    pub current_cash: f64,
    pub allocation_pct: f64,
    pub auto_execute: bool,
    pub unrealized_pnl: f64,
    pub unrealized_roi_pct: f64,
    pub total_account_value: f64,
    pub margin_used: f64,
    pub max_trades: u32,
    pub active_trades: u32,
    pub available_trades: u32,
    pub active_position: Option<ActivePaperPosition>,
    pub scale_in_portions: Vec<ScaleInPortionRecord>,
    pub position_slots: Vec<PositionSlotRecord>,
    pub take_profit_targets: Vec<OpenOrder>,
    pub max_risk_pct: f64,
    pub leverage: i32,
    pub auto_execute_intervals: i32,
    pub lookback_trades: i32,
    pub initial_allocated_margin: f64,
    pub realized_pnl_accumulator: f64,
}

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct ScaleInPortionRecord {
    pub id: i64,
    pub entry_price: f64,
    pub size: f64,
    pub allocated_usd: f64,
    pub portion_number: i32,
}

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct OpenOrder {
    pub id: i64,
    pub symbol: String,
    pub order_type: String,
    pub direction: String,
    pub price: Option<f64>,
    pub trigger_price: Option<f64>,
    pub size: f64,
    pub is_reduce_only: bool,
    pub associated_position_id: Option<i64>,
    pub created_at: i64,
}

// ─── Read Functions ────────────────────────────────────────────────

pub async fn paper_ensure_balance(pool: &SqlitePool, symbol: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT OR IGNORE INTO paper_balances (symbol, initial_usd, current_cash, allocation_pct, auto_execute, max_risk_pct, leverage, auto_execute_intervals, lookback_trades)
         VALUES (?1, 0.0, 0.0, 25.0, 0, 2.0, 20, 15, 10)"
    )
    .bind(symbol)
    .execute(&*pool)
    .await?;
    Ok(())
}

pub async fn paper_get_balance(pool: &SqlitePool, symbol: &str) -> PaperBalance {
    use sqlx::Row;
    let _ = paper_ensure_balance(pool, symbol).await;
    let row = sqlx::query(
        "SELECT id, symbol, initial_usd, current_cash, allocation_pct, auto_execute,
                max_risk_pct, leverage, auto_execute_intervals, lookback_trades
         FROM paper_balances WHERE symbol = ?1",
    )
    .bind(symbol)
    .fetch_optional(&*pool)
    .await
    .ok()
    .flatten();

    match row {
        Some(r) => PaperBalance {
            id: r.get(0),
            symbol: r.get(1),
            initial_usd: r.get(2),
            current_cash: r.get(3),
            allocation_pct: r.get(4),
            auto_execute: r.get::<i32, _>(5) != 0,
            max_risk_pct: r.get::<f64, _>(6),
            leverage: r.get::<i32, _>(7),
            auto_execute_intervals: r.get::<i32, _>(8),
            lookback_trades: r.get::<i32, _>(9),
        },
        None => PaperBalance {
            id: 0,
            symbol: symbol.to_string(),
            initial_usd: 0.0,
            current_cash: 0.0,
            allocation_pct: 25.0,
            auto_execute: false,
            max_risk_pct: 2.0,
            leverage: 20,
            auto_execute_intervals: 15,
            lookback_trades: 10,
        },
    }
}

pub async fn paper_get_active_position(
    pool: &SqlitePool,
    symbol: &str,
) -> Option<ActivePaperPosition> {
    use sqlx::Row;
    let row = sqlx::query(
        "SELECT id, symbol, direction, entry_price, size, allocated_usd, entry_timestamp,
                average_entry_price, current_portions, final_invalidation_level, target_profit_ratio,
                initial_allocated_margin, realized_pnl_accumulator
         FROM active_positions WHERE symbol = ?1",
    )
    .bind(symbol)
    .fetch_optional(&*pool)
    .await
    .ok()
    .flatten();

    row.map(|r| ActivePaperPosition {
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
        initial_allocated_margin: r.get(11),
        realized_pnl_accumulator: r.get(12),
    })
}

pub async fn paper_query_trades(
    pool: &SqlitePool,
    symbol: Option<&str>,
    limit: u32,
) -> Vec<PaperTradeRecord> {
    use sqlx::Row;
    let rows = if let Some(sym) = symbol {
        sqlx::query(
            "SELECT id, symbol, direction, entry_price, exit_price, size, realized_pnl, roi_pct, entry_timestamp, exit_timestamp, trigger
             FROM paper_trades WHERE symbol = ?1 ORDER BY id DESC LIMIT ?2"
        )
        .bind(sym)
        .bind(limit as i64)
        .fetch_all(&*pool)
        .await
    } else {
        sqlx::query(
            "SELECT id, symbol, direction, entry_price, exit_price, size, realized_pnl, roi_pct, entry_timestamp, exit_timestamp, trigger
             FROM paper_trades ORDER BY id DESC LIMIT ?1"
        )
        .bind(limit as i64)
        .fetch_all(&*pool)
        .await
    };

    match rows {
        Ok(rows) => rows
            .iter()
            .map(|r| PaperTradeRecord {
                id: r.get(0),
                symbol: r.get(1),
                direction: r.get(2),
                entry_price: r.get(3),
                exit_price: r.get(4),
                size: r.get(5),
                realized_pnl: r.get(6),
                roi_pct: r.get(7),
                entry_timestamp: r.get(8),
                exit_timestamp: r.get(9),
                trigger: r.get(10),
            })
            .collect(),
        Err(e) => {
            eprintln!("⚠️ Database Error: Failed to query paper trades: {}", e);
            vec![]
        }
    }
}

pub async fn paper_get_account_metrics(
    pool: &SqlitePool,
    symbol: &str,
    current_price: f64,
) -> PaperAccountMetrics {
    let balance = paper_get_balance(pool, symbol).await;
    let position = paper_get_active_position(pool, symbol).await;

    let (unrealized_pnl, unrealized_roi, margin_used) = match &position {
        Some(pos) => {
            let pnl = if pos.direction == "LONG" {
                (current_price - pos.entry_price) * pos.size
            } else {
                (pos.entry_price - current_price) * pos.size
            };
            let roi = if pos.allocated_usd > 0.0 {
                (pnl / pos.allocated_usd) * 100.0
            } else {
                0.0
            };
            (pnl, roi, pos.allocated_usd)
        }
        None => (0.0, 0.0, 0.0),
    };

    let total_account_value = balance.current_cash + margin_used + unrealized_pnl;
    let max_trades = if balance.allocation_pct > 0.0 {
        (100.0 / balance.allocation_pct).floor() as u32
    } else {
        0
    };
    let active_trades = if position.is_some() { 1u32 } else { 0u32 };
    let available_trades = max_trades.saturating_sub(active_trades);

    // Fetch position slots
    let position_id = position.as_ref().map(|p| p.id);
    let slots: Vec<PositionSlotRecord> = if let Some(pid) = position_id {
        sqlx::query_as(
            "SELECT id, position_id, symbol, direction, slot_index, is_active, entry_price, size, allocated_usd, realized_pnl, timestamp
             FROM position_slots WHERE position_id = ?1 ORDER BY slot_index ASC"
        )
        .bind(pid)
        .fetch_all(&*pool)
        .await
        .unwrap_or_default()
    } else {
        vec![]
    };

    // Backward-compat: derive scale_in_portions from active slots
    let portions: Vec<ScaleInPortionRecord> = slots
        .iter()
        .filter(|s| s.is_active)
        .map(|s| ScaleInPortionRecord {
            id: s.id,
            entry_price: s.entry_price,
            size: s.size,
            allocated_usd: s.allocated_usd,
            portion_number: s.slot_index,
        })
        .collect();

    let initial_margin = position.as_ref().and_then(|p| p.initial_allocated_margin).unwrap_or(0.0);
    let realized_accum = position.as_ref().and_then(|p| p.realized_pnl_accumulator).unwrap_or(0.0);

    // Fetch take-profit targets (now from open_orders where is_reduce_only = 1)
    let take_profit_targets: Vec<OpenOrder> = if let Some(ref pos) = position {
        sqlx::query_as(
            "SELECT id, symbol, order_type, direction, price, trigger_price, size, is_reduce_only, associated_position_id, created_at
             FROM open_orders WHERE associated_position_id = ?1 AND is_reduce_only = 1 ORDER BY price ASC"
        )
        .bind(pos.id)
        .fetch_all(&*pool)
        .await
        .unwrap_or_default()
    } else {
        vec![]
    };

    PaperAccountMetrics {
        symbol: symbol.to_string(),
        initial_usd: balance.initial_usd,
        current_cash: balance.current_cash,
        allocation_pct: balance.allocation_pct,
        auto_execute: balance.auto_execute,
        unrealized_pnl,
        unrealized_roi_pct: unrealized_roi,
        total_account_value,
        margin_used,
        max_trades,
        active_trades,
        available_trades,
        active_position: position,
        scale_in_portions: portions,
        position_slots: slots,
        take_profit_targets,
        max_risk_pct: balance.max_risk_pct,
        leverage: balance.leverage,
        auto_execute_intervals: balance.auto_execute_intervals,
        lookback_trades: balance.lookback_trades,
        initial_allocated_margin: initial_margin,
        realized_pnl_accumulator: realized_accum,
    }
}

pub async fn paper_get_open_orders(pool: &SqlitePool, symbol: &str) -> Vec<OpenOrder> {
    sqlx::query_as(
        "SELECT id, symbol, order_type, direction, price, trigger_price, size, is_reduce_only, associated_position_id, created_at
         FROM open_orders WHERE symbol = ?1 AND associated_position_id IS NULL ORDER BY created_at DESC"
    )
    .bind(symbol)
    .fetch_all(&*pool)
    .await
    .unwrap_or_default()
}

pub async fn paper_get_brackets_for_position(pool: &SqlitePool, position_id: i64) -> Vec<OpenOrder> {
    sqlx::query_as(
        "SELECT id, symbol, order_type, direction, price, trigger_price, size, is_reduce_only, associated_position_id, created_at
         FROM open_orders WHERE associated_position_id = ?1 AND is_reduce_only = 1 ORDER BY order_type, price ASC"
    )
    .bind(position_id)
    .fetch_all(&*pool)
    .await
    .unwrap_or_default()
}

pub async fn paper_count_brackets_by_type(pool: &SqlitePool, position_id: i64) -> (i32, i32) {
    use sqlx::Row;
    let row = sqlx::query(
        "SELECT
            COALESCE(SUM(CASE WHEN order_type = 'LIMIT' THEN 1 ELSE 0 END), 0) as tp_count,
            COALESCE(SUM(CASE WHEN order_type = 'STOP' THEN 1 ELSE 0 END), 0) as sl_count
         FROM open_orders WHERE associated_position_id = ?1 AND is_reduce_only = 1"
    )
    .bind(position_id)
    .fetch_one(&*pool)
    .await;
    match row {
        Ok(r) => (r.get(0), r.get(1)),
        Err(_) => (0, 0),
    }
}

// ─── Slot Operations ──────────────────────────────────────────────

pub async fn paper_get_active_slots(pool: &SqlitePool, symbol: &str) -> Vec<PositionSlotRecord> {
    sqlx::query_as(
        "SELECT id, position_id, symbol, direction, slot_index, is_active, entry_price, size, allocated_usd, realized_pnl, timestamp
         FROM position_slots WHERE symbol = ?1 AND is_active = 1 ORDER BY slot_index ASC"
    )
    .bind(symbol)
    .fetch_all(&*pool)
    .await
    .unwrap_or_default()
}

pub async fn paper_get_active_slot_count(pool: &SqlitePool, symbol: &str) -> i32 {
    use sqlx::Row;
    let row = sqlx::query(
        "SELECT COUNT(*) FROM position_slots WHERE symbol = ?1 AND is_active = 1"
    )
    .bind(symbol)
    .fetch_one(&*pool)
    .await;
    match row {
        Ok(r) => r.get::<i64, _>(0) as i32,
        Err(_) => 0,
    }
}

pub async fn paper_find_vacant_slot(pool: &SqlitePool, symbol: &str) -> Option<i32> {
    use sqlx::Row;
    for idx in 0..4i32 {
        let row = sqlx::query(
            "SELECT COUNT(*) FROM position_slots WHERE symbol = ?1 AND slot_index = ?2 AND is_active = 1"
        )
        .bind(symbol)
        .bind(idx)
        .fetch_one(&*pool)
        .await;
        if let Ok(r) = row {
            if r.get::<i64, _>(0) == 0 {
                return Some(idx);
            }
        }
    }
    None
}

pub async fn paper_get_oldest_active_slot(pool: &SqlitePool, symbol: &str) -> Option<PositionSlotRecord> {
    sqlx::query_as(
        "SELECT id, position_id, symbol, direction, slot_index, is_active, entry_price, size, allocated_usd, realized_pnl, timestamp
         FROM position_slots WHERE symbol = ?1 AND is_active = 1 ORDER BY timestamp ASC LIMIT 1"
    )
    .bind(symbol)
    .fetch_optional(&*pool)
    .await
    .unwrap_or(None)
}

// ─── Position Equity Snapshots ────────────────────────────────────

pub async fn paper_insert_equity_snapshot(
    pool: &SqlitePool,
    symbol: &str,
    timestamp_ms: i64,
    equity_value: f64,
    cash_balance: f64,
    unrealized_pnl: f64,
) {
    let _ = sqlx::query(
        "INSERT INTO position_equity_snapshots (symbol, timestamp, equity_value, cash_balance, unrealized_pnl)
         VALUES (?1, ?2, ?3, ?4, ?5)",
    )
    .bind(symbol)
    .bind(timestamp_ms)
    .bind(equity_value)
    .bind(cash_balance)
    .bind(unrealized_pnl)
    .execute(pool)
    .await;
}

pub async fn paper_fetch_equity_history(
    pool: &SqlitePool,
    symbol: &str,
    limit: i64,
) -> Vec<(i64, f64, f64, f64)> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT timestamp, equity_value, cash_balance, unrealized_pnl
         FROM position_equity_snapshots WHERE symbol = ?1
         ORDER BY timestamp DESC LIMIT ?2"
    )
    .bind(symbol)
    .bind(limit)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    rows.iter()
        .map(|r| {
            let ts: i64 = r.get(0);
            let ev: f64 = r.get(1);
            let cb: f64 = r.get(2);
            let up: f64 = r.get(3);
            (ts, ev, cb, up)
        })
        .collect()
}
