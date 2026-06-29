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
    pub take_profit_targets: Vec<TakeProfitTargetRecord>,
    pub max_risk_pct: f64,
    pub leverage: i32,
    pub auto_execute_intervals: i32,
    pub lookback_trades: i32,
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
pub struct TakeProfitTargetRow {
    pub id: i64,
    pub target_price: f64,
    pub size_fraction: f64,
    pub is_hit: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TakeProfitTargetRecord {
    pub id: i64,
    pub target_price: f64,
    pub size_fraction: f64,
    pub is_hit: bool,
}

// ─── Read Functions ────────────────────────────────────────────────

pub async fn paper_ensure_balance(pool: &SqlitePool, symbol: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT OR IGNORE INTO paper_balances (symbol, initial_usd, current_cash, allocation_pct, auto_execute, max_risk_pct, leverage, auto_execute_intervals, lookback_trades)
         VALUES (?1, 0.0, 0.0, 10.0, 0, 2.0, 20, 15, 10)"
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
            allocation_pct: 10.0,
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
                average_entry_price, current_portions, final_invalidation_level, target_profit_ratio
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

    // Fetch scale-in portions
    let portions: Vec<ScaleInPortionRecord> = sqlx::query_as(
        "SELECT id, entry_price, size, allocated_usd, portion_number FROM active_position_portions WHERE symbol = ?1 ORDER BY portion_number ASC"
    )
    .bind(symbol)
    .fetch_all(&*pool)
    .await
    .unwrap_or_default();

    // Fetch take-profit targets
    let targets: Vec<TakeProfitTargetRow> = sqlx::query_as(
        "SELECT id, target_price, size_fraction, is_hit FROM position_take_profit_targets WHERE symbol = ?1 ORDER BY target_price ASC"
    )
    .bind(symbol)
    .fetch_all(&*pool)
    .await
    .unwrap_or_default();

    let targets_bool: Vec<TakeProfitTargetRecord> = targets
        .into_iter()
        .map(|t| TakeProfitTargetRecord {
            id: t.id,
            target_price: t.target_price,
            size_fraction: t.size_fraction,
            is_hit: t.is_hit != 0,
        })
        .collect();

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
        take_profit_targets: targets_bool,
        max_risk_pct: balance.max_risk_pct,
        leverage: balance.leverage,
        auto_execute_intervals: balance.auto_execute_intervals,
        lookback_trades: balance.lookback_trades,
    }
}
