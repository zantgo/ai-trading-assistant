use sqlx::SqlitePool;

pub(crate) async fn paper_open_position_internal(
    pool: &SqlitePool,
    symbol: &str,
    direction: &str,
    entry_price: f64,
    size: f64,
    allocated_usd: f64,
) -> Result<(), sqlx::Error> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    let mut tx = pool.begin().await?;
    sqlx::query(
        "INSERT OR REPLACE INTO active_positions (symbol, direction, entry_price, size, allocated_usd, entry_timestamp, average_entry_price, current_portions, final_invalidation_level, target_profit_ratio, initial_allocated_margin, realized_pnl_accumulator)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?3, 1, 0, 2.0, ?5, 0.0)"
    )
    .bind(symbol)
    .bind(direction)
    .bind(entry_price)
    .bind(size)
    .bind(allocated_usd)
    .bind(now)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    println!(
        "📄 Paper Position: OPEN {} {} @ ${:.2} (Size: {:.4}, Allocated: ${:.2})",
        symbol, direction, entry_price, size, allocated_usd
    );
    Ok(())
}

pub(crate) async fn paper_close_position_internal(
    pool: &SqlitePool,
    symbol: &str,
    exit_price: f64,
    exit_timestamp: i64,
    trigger: &str,
) -> Result<(), sqlx::Error> {
    let position = sqlx::query_as::<_, (i64, String, String, f64, f64, f64, i64)>(
        "SELECT id, symbol, direction, entry_price, size, allocated_usd, entry_timestamp
         FROM active_positions WHERE symbol = ?1",
    )
    .bind(symbol)
    .fetch_optional(pool)
    .await?;

    let (_id, sym, direction, entry_price, size, allocated_usd, entry_ts) = match position {
        Some(pos) => pos,
        None => {
            eprintln!("⚠️ Paper DB: No active position to close for {}", symbol);
            return Ok(());
        }
    };

    let realized_pnl = if direction == "LONG" {
        (exit_price - entry_price) * size
    } else {
        (entry_price - exit_price) * size
    };
    let roi_pct = if allocated_usd > 0.0 {
        (realized_pnl / allocated_usd) * 100.0
    } else {
        0.0
    };

    let mut tx = pool.begin().await?;
    sqlx::query(
        "INSERT INTO paper_trades (symbol, direction, entry_price, exit_price, size, realized_pnl, roi_pct, entry_timestamp, exit_timestamp, trigger)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"
    )
    .bind(&sym)
    .bind(&direction)
    .bind(entry_price)
    .bind(exit_price)
    .bind(size)
    .bind(realized_pnl)
    .bind(roi_pct)
    .bind(entry_ts)
    .bind(exit_timestamp)
    .bind(trigger)
    .execute(&mut *tx)
    .await?;

    sqlx::query("DELETE FROM active_positions WHERE symbol = ?1")
        .bind(symbol)
        .execute(&mut *tx)
        .await?;

    sqlx::query("DELETE FROM position_slots WHERE symbol = ?1")
        .bind(symbol)
        .execute(&mut *tx)
        .await?;

    sqlx::query("DELETE FROM open_orders WHERE associated_position_id IN (SELECT id FROM active_positions WHERE symbol = ?1)")
        .bind(symbol)
        .execute(&mut *tx)
        .await?;

    sqlx::query("UPDATE paper_balances SET current_cash = current_cash + ?2 WHERE symbol = ?1")
        .bind(symbol)
        .bind(allocated_usd + realized_pnl)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    println!(
        "📄 Paper Position: CLOSE {} {} @ ${:.2} → PnL: ${:.2} (ROI: {:.2}%) [{}]",
        symbol, direction, exit_price, realized_pnl, roi_pct, trigger
    );
    Ok(())
}

pub(crate) async fn paper_update_balance_internal(
    pool: &SqlitePool,
    symbol: &str,
    current_cash: f64,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query(
        "INSERT OR REPLACE INTO paper_balances (symbol, initial_usd, current_cash, allocation_pct, auto_execute, max_risk_pct, leverage, auto_execute_intervals, lookback_trades)
         VALUES (?1, ?2, ?2, 25.0, 0, 2.0, 20, 15, 10)"
    )
    .bind(symbol)
    .bind(current_cash)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

pub(crate) async fn paper_scale_in_portion_internal(
    pool: &SqlitePool,
    symbol: &str,
    direction: &str,
    entry_price: f64,
    size: f64,
    allocated_usd: f64,
    portion_number: i32,
    new_average_entry_price: f64,
    total_size: f64,
    final_invalidation_level: f64,
) -> Result<(), sqlx::Error> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    let position = sqlx::query_as::<_, (i64,)>("SELECT id FROM active_positions WHERE symbol = ?1")
        .bind(symbol)
        .fetch_optional(pool)
        .await?;

    let mut tx = pool.begin().await?;
    if let Some((pos_id,)) = position {
        sqlx::query(
            "INSERT OR IGNORE INTO position_slots (position_id, symbol, direction, slot_index, is_active, entry_price, size, allocated_usd, realized_pnl, timestamp)
             VALUES (?1, ?2, ?3, ?4, 1, ?5, ?6, ?7, 0.0, ?8)"
        )
        .bind(pos_id)
        .bind(symbol)
        .bind(direction)
        .bind(portion_number)
        .bind(entry_price)
        .bind(size)
        .bind(allocated_usd)
        .bind(now)
        .execute(&mut *tx)
        .await?;
    }

    sqlx::query(
        "INSERT INTO active_positions (symbol, direction, entry_price, size, allocated_usd, entry_timestamp, average_entry_price, current_portions, final_invalidation_level, target_profit_ratio, initial_allocated_margin, realized_pnl_accumulator)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 2.0, ?5, 0.0)
         ON CONFLICT(symbol) DO UPDATE SET
            entry_price = excluded.entry_price,
            size = excluded.size,
            average_entry_price = excluded.average_entry_price,
            current_portions = excluded.current_portions,
            final_invalidation_level = excluded.final_invalidation_level"
    )
    .bind(symbol)
    .bind(direction)
    .bind(new_average_entry_price)
    .bind(total_size)
    .bind(allocated_usd)
    .bind(now)
    .bind(new_average_entry_price)
    .bind(portion_number)
    .bind(final_invalidation_level)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    println!(
        "📄 Paper Scale-In: Slot {} for {} {} @ ${:.2} | New Avg: ${:.2} | Total Size: {:.4}",
        portion_number, symbol, direction, entry_price, new_average_entry_price, total_size
    );
    Ok(())
}

pub(crate) async fn paper_scale_out_portion_internal(
    pool: &SqlitePool,
    symbol: &str,
    exit_price: f64,
    size_fraction: f64,
    realized_pnl: f64,
    remaining_size: f64,
    target_id: i64,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    if target_id > 0 {
        sqlx::query("DELETE FROM open_orders WHERE id = ?1")
            .bind(target_id)
            .execute(&mut *tx)
            .await?;
    }

    if remaining_size > 0.0 {
        sqlx::query("UPDATE active_positions SET size = ?2 WHERE symbol = ?1")
            .bind(symbol)
            .bind(remaining_size)
            .execute(&mut *tx)
            .await?;
    } else {
    sqlx::query("DELETE FROM active_positions WHERE symbol = ?1")
        .bind(symbol)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM position_slots WHERE symbol = ?1")
        .bind(symbol)
        .execute(&mut *tx)
        .await?;
        sqlx::query("DELETE FROM open_orders WHERE associated_position_id IS NULL AND symbol = ?1")
            .bind(symbol)
            .execute(&mut *tx)
            .await?;
    }

    sqlx::query("UPDATE paper_balances SET current_cash = current_cash + ?2 WHERE symbol = ?1")
        .bind(symbol)
        .bind(realized_pnl)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    println!(
        "📄 Paper Scale-Out: {} @ ${:.2} | Fraction: {:.0}% | Realized PnL: ${:.2} | Remaining: {:.4}",
        symbol, exit_price, size_fraction * 100.0, realized_pnl, remaining_size
    );
    Ok(())
}

pub(crate) async fn paper_invalidate_position_internal(
    pool: &SqlitePool,
    symbol: &str,
    exit_price: f64,
    exit_timestamp: i64,
    realized_loss: f64,
    reason: &str,
) -> Result<(), sqlx::Error> {
    let position = sqlx::query_as::<_, (String, f64, f64, f64, i64)>(
        "SELECT direction, entry_price, size, allocated_usd, entry_timestamp
         FROM active_positions WHERE symbol = ?1",
    )
    .bind(symbol)
    .fetch_optional(pool)
    .await?;

    let (direction, entry_price_avg, size, allocated_usd, entry_ts) = match position {
        Some(pos) => pos,
        None => return Ok(()),
    };

    let roi_pct = if allocated_usd > 0.0 {
        (realized_loss / allocated_usd) * 100.0
    } else {
        0.0
    };

    let mut tx = pool.begin().await?;

    sqlx::query(
        "INSERT INTO paper_trades (symbol, direction, entry_price, exit_price, size, realized_pnl, roi_pct, entry_timestamp, exit_timestamp, trigger)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"
    )
    .bind(symbol)
    .bind(&direction)
    .bind(entry_price_avg)
    .bind(exit_price)
    .bind(size)
    .bind(realized_loss)
    .bind(roi_pct)
    .bind(entry_ts)
    .bind(exit_timestamp)
    .bind(format!("INVALIDATION:{}", reason))
    .execute(&mut *tx)
    .await?;
    sqlx::query("DELETE FROM active_positions WHERE symbol = ?1")
        .bind(symbol)
        .execute(&mut *tx)
        .await?;

    sqlx::query("DELETE FROM position_slots WHERE symbol = ?1")
        .bind(symbol)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM open_orders WHERE associated_position_id IN (SELECT id FROM active_positions WHERE symbol = ?1)")
        .bind(symbol)
        .execute(&mut *tx)
        .await?;

    let refund = allocated_usd + realized_loss;
    if refund > 0.0 {
        sqlx::query("UPDATE paper_balances SET current_cash = current_cash + ?2 WHERE symbol = ?1")
            .bind(symbol)
            .bind(refund)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;

    println!(
        "📄 Paper INVALIDATION: {} @ ${:.2} | Loss: ${:.2} ({:.2}%) | Reason: {}",
        symbol, exit_price, realized_loss, roi_pct, reason
    );
    Ok(())
}

pub async fn paper_set_balance_config(
    pool: &SqlitePool,
    symbol: &str,
    initial_usd: f64,
    allocation_pct: f64,
    auto_execute: bool,
) -> Result<(), sqlx::Error> {
    let auto_val: i32 = if auto_execute { 1 } else { 0 };
    sqlx::query(
        "INSERT INTO paper_balances (symbol, initial_usd, current_cash, allocation_pct, auto_execute, max_risk_pct, leverage, auto_execute_intervals, lookback_trades)
         VALUES (?1, ?2, ?2, ?3, ?4, 2.0, 20, 15, 10)
         ON CONFLICT(symbol) DO UPDATE SET
            initial_usd = excluded.initial_usd,
            current_cash = excluded.initial_usd,
            allocation_pct = excluded.allocation_pct,
            auto_execute = excluded.auto_execute"
    )
    .bind(symbol)
    .bind(initial_usd)
    .bind(allocation_pct)
    .bind(auto_val)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn paper_set_advanced_config(
    pool: &SqlitePool,
    symbol: &str,
    initial_usd: f64,
    allocation_pct: f64,
    auto_execute: bool,
    max_risk_pct: f64,
    leverage: i32,
    auto_execute_intervals: i32,
    lookback_trades: i32,
    break_even_trail_enabled: bool,
) -> Result<(), sqlx::Error> {
    let auto_val: i32 = if auto_execute { 1 } else { 0 };
    let bet_val: i32 = if break_even_trail_enabled { 1 } else { 0 };
    sqlx::query(
        "INSERT INTO paper_balances (symbol, initial_usd, current_cash, allocation_pct, auto_execute, max_risk_pct, leverage, auto_execute_intervals, lookback_trades, break_even_trail_enabled)
         VALUES (?1, ?2, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
         ON CONFLICT(symbol) DO UPDATE SET
            initial_usd = excluded.initial_usd,
            current_cash = excluded.initial_usd,
            allocation_pct = excluded.allocation_pct,
            auto_execute = excluded.auto_execute,
            max_risk_pct = excluded.max_risk_pct,
            leverage = excluded.leverage,
            auto_execute_intervals = excluded.auto_execute_intervals,
            lookback_trades = excluded.lookback_trades,
            break_even_trail_enabled = excluded.break_even_trail_enabled"
    )
    .bind(symbol)
    .bind(initial_usd)
    .bind(allocation_pct)
    .bind(auto_val)
    .bind(max_risk_pct)
    .bind(leverage)
    .bind(auto_execute_intervals)
    .bind(lookback_trades)
    .bind(bet_val)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn paper_reset_account(pool: &SqlitePool, symbol: &str) -> Result<(), sqlx::Error> {
    let balance = super::queries::paper_get_balance(pool, symbol).await;

    let mut tx = pool.begin().await?;
    sqlx::query("UPDATE paper_balances SET current_cash = ?2 WHERE symbol = ?1")
        .bind(symbol)
        .bind(balance.initial_usd)
        .execute(&mut *tx)
        .await?;

    sqlx::query("DELETE FROM active_positions WHERE symbol = ?1")
        .bind(symbol)
        .execute(&mut *tx)
        .await?;

    sqlx::query("DELETE FROM position_slots WHERE symbol = ?1")
        .bind(symbol)
        .execute(&mut *tx)
        .await?;

    sqlx::query("DELETE FROM open_orders WHERE symbol = ?1")
        .bind(symbol)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    println!(
        "📄 Paper Account: {} reset to initial balance ${:.2}",
        symbol, balance.initial_usd
    );
    Ok(())
}

pub(crate) async fn paper_insert_open_order(
    pool: &SqlitePool,
    symbol: &str,
    order_type: &str,
    direction: &str,
    price: Option<f64>,
    trigger_price: Option<f64>,
    size: f64,
    is_reduce_only: bool,
    associated_position_id: Option<i64>,
) -> Result<i64, sqlx::Error> {
    // Enforce entry order capacity constraint: E + 1 ≤ 4 - A
    if associated_position_id.is_none() && !is_reduce_only {
        let active_count: i64 = sqlx::query_scalar(
            "SELECT COALESCE(COUNT(*), 0) FROM position_slots WHERE symbol = ?1 AND is_active = 1"
        )
        .bind(symbol)
        .fetch_one(pool)
        .await
        .unwrap_or(0);
        let pending_count: i64 = sqlx::query_scalar(
            "SELECT COALESCE(COUNT(*), 0) FROM open_orders WHERE symbol = ?1 AND associated_position_id IS NULL AND is_reduce_only = 0"
        )
        .bind(symbol)
        .fetch_one(pool)
        .await
        .unwrap_or(0);
        if pending_count + 1 > 4 - active_count {
            return Err(sqlx::Error::Protocol(format!(
                "Entry order cap exceeded: {} pending + 1 new > {} vacant slots (4 - {} active)",
                pending_count, 4 - active_count, active_count
            )));
        }
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    let reduce_val: i32 = if is_reduce_only { 1 } else { 0 };

    let mut tx = pool.begin().await?;
    sqlx::query(
        "INSERT INTO open_orders (symbol, order_type, direction, price, trigger_price, size, is_reduce_only, associated_position_id, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"
    )
    .bind(symbol)
    .bind(order_type)
    .bind(direction)
    .bind(price)
    .bind(trigger_price)
    .bind(size)
    .bind(reduce_val)
    .bind(associated_position_id)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    let id = sqlx::query_scalar::<_, i64>("SELECT last_insert_rowid()")
        .fetch_one(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(id)
}

pub(crate) async fn paper_delete_open_order(pool: &SqlitePool, order_id: i64) -> Result<bool, sqlx::Error> {
    let rows = sqlx::query("DELETE FROM open_orders WHERE id = ?1")
        .bind(order_id)
        .execute(pool)
        .await?
        .rows_affected();
    Ok(rows > 0)
}
