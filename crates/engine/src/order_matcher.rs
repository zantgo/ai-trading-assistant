use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::db;
use crate::paper_trading;
use crate::workspace::Workspace;

pub async fn run_order_matcher(
    workspace: Arc<Workspace>,
    pool: SqlitePool,
    telemetry_tx: mpsc::Sender<db::TelemetryMsg>,
    cancel: CancellationToken,
) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(2));
    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,
            _ = interval.tick() => {
                process_tick(&workspace, &pool, &telemetry_tx).await;
            }
        }
    }
}

async fn process_tick(
    workspace: &Arc<Workspace>,
    pool: &SqlitePool,
    telemetry_tx: &mpsc::Sender<db::TelemetryMsg>,
) {
    let instances = workspace.instances.read().await;
    for (pair_key, instance) in instances.iter() {
        let symbol = pair_key.clone();

        let orders = db::paper_get_open_orders(pool, &symbol).await;
        let all_bracket_orders: Vec<db::OpenOrder> =
            if let Some(pos) = db::paper_get_active_position(pool, &symbol).await {
                db::paper_get_brackets_for_position(pool, pos.id).await
            } else {
                vec![]
            };

        let all_orders: Vec<db::OpenOrder> =
            orders.into_iter().chain(all_bracket_orders).collect();
        if all_orders.is_empty() {
            continue;
        }

        let current_price = match instance.latest_price().await {
            Some(p) if p > 0.0 => p,
            _ => continue,
        };

        for order in &all_orders {
            let triggered = match (order.order_type.as_str(), order.direction.as_str()) {
                ("LIMIT", "BUY") => current_price <= order.price.unwrap_or(f64::INFINITY),
                ("LIMIT", "SELL") => current_price >= order.price.unwrap_or(0.0),
                ("STOP", "BUY") => current_price >= order.trigger_price.unwrap_or(0.0),
                ("STOP", "SELL") => current_price <= order.trigger_price.unwrap_or(f64::INFINITY),
                _ => false,
            };

            if !triggered {
                continue;
            }

            let _ = db::paper::operations::paper_delete_open_order(pool, order.id).await;

            if order.is_reduce_only {
                handle_reduce_only_fill(pool, telemetry_tx, &symbol, order, current_price)
                    .await;
            } else {
                handle_standard_entry_fill(pool, telemetry_tx, &symbol, order, current_price)
                    .await;
            }
        }
    }
}

async fn handle_reduce_only_fill(
    pool: &SqlitePool,
    telemetry_tx: &mpsc::Sender<db::TelemetryMsg>,
    symbol: &str,
    order: &db::OpenOrder,
    current_price: f64,
) {
    let pos = match db::paper_get_active_position(pool, symbol).await {
        Some(p) => p,
        None => return,
    };

    // Use FIFO slot-based closure
    let oldest = match db::paper_get_oldest_active_slot(pool, symbol).await {
        Some(s) => s,
        None => {
            // Legacy fallback: no position_slots entries — close via old percentage method
            let size_fraction_pct = order.size;
            let close_fraction = (size_fraction_pct / 100.0).min(1.0);
            let exit_size = pos.size * close_fraction;
            let remaining_size = pos.size - exit_size;
            let entry_avg = pos.average_entry_price.unwrap_or(pos.entry_price);
            let realized_pnl = if pos.direction == "LONG" {
                (current_price - entry_avg) * exit_size
            } else {
                (entry_avg - current_price) * exit_size
            };
            let allocated_released = pos.allocated_usd * close_fraction;
            let total_credit = allocated_released + realized_pnl;
            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as i64;

            if let Ok(mut tx) = pool.begin().await {
                if remaining_size > 0.0 {
                    let new_allocated = pos.allocated_usd - allocated_released;
                    sqlx::query("UPDATE active_positions SET size = ?2, allocated_usd = ?3 WHERE symbol = ?1")
                        .bind(symbol).bind(remaining_size).bind(new_allocated)
                        .execute(&mut *tx).await.ok();
                } else {
                    sqlx::query("DELETE FROM active_positions WHERE symbol = ?1").bind(symbol).execute(&mut *tx).await.ok();
                    sqlx::query("DELETE FROM position_slots WHERE symbol = ?1").bind(symbol).execute(&mut *tx).await.ok();
                    sqlx::query("DELETE FROM open_orders WHERE associated_position_id = ?1").bind(pos.id).execute(&mut *tx).await.ok();
                }
                sqlx::query("UPDATE paper_balances SET current_cash = current_cash + ?2 WHERE symbol = ?1")
                    .bind(symbol).bind(total_credit).execute(&mut *tx).await.ok();
                let roi_pct = if allocated_released > 0.0 { (realized_pnl / allocated_released) * 100.0 } else { 0.0 };
                sqlx::query("INSERT INTO paper_trades (symbol, direction, entry_price, exit_price, size, realized_pnl, roi_pct, entry_timestamp, exit_timestamp, trigger) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)")
                    .bind(symbol).bind(&pos.direction).bind(entry_avg).bind(current_price).bind(exit_size).bind(realized_pnl).bind(roi_pct).bind(pos.entry_timestamp).bind(now).bind(if order.order_type == "LIMIT" { "TP" } else { "SL" })
                    .execute(&mut *tx).await.ok();
                let _ = tx.commit().await;
            }
            let _ = telemetry_tx.send(db::TelemetryMsg::JournalTrade {
                symbol: symbol.to_string(), direction: pos.direction.clone(),
                entry_price: entry_avg, exit_price: current_price,
                entry_timestamp: pos.entry_timestamp, exit_timestamp: now,
                size: exit_size, realized_pnl, roi_pct: if allocated_released > 0.0 { (realized_pnl / allocated_released) * 100.0 } else { 0.0 },
                allocated_usd: allocated_released,
                trigger: if order.order_type == "LIMIT" { "TP".to_string() } else { "SL".to_string() },
            }).await;
            return;
        }
    };

    // FIFO slot-based closure: close the oldest active slot
    let pnl = if oldest.direction == "LONG" {
        (current_price - oldest.entry_price) * oldest.size
    } else {
        (oldest.entry_price - current_price) * oldest.size
    };
    let refund = oldest.allocated_usd + pnl;
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as i64;

    if let Ok(mut tx) = pool.begin().await {
        // Mark slot inactive with realized PnL
        sqlx::query("UPDATE position_slots SET is_active = 0, realized_pnl = ?2 WHERE id = ?1")
            .bind(oldest.id).bind(pnl).execute(&mut *tx).await.ok();

        // Update realized_pnl_accumulator
        let current_accum = pos.realized_pnl_accumulator.unwrap_or(0.0);
        sqlx::query("UPDATE active_positions SET realized_pnl_accumulator = ?2 WHERE symbol = ?1")
            .bind(symbol).bind(current_accum + pnl).execute(&mut *tx).await.ok();

        // Refund margin + PnL to balance
        sqlx::query("UPDATE paper_balances SET current_cash = current_cash + ?2 WHERE symbol = ?1")
            .bind(symbol).bind(refund).execute(&mut *tx).await.ok();

        // Record trade
        let roi_pct = if oldest.allocated_usd > 0.0 { (pnl / oldest.allocated_usd) * 100.0 } else { 0.0 };
        sqlx::query("INSERT INTO paper_trades (symbol, direction, entry_price, exit_price, size, realized_pnl, roi_pct, entry_timestamp, exit_timestamp, trigger) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)")
            .bind(symbol).bind(&oldest.direction).bind(oldest.entry_price).bind(current_price).bind(oldest.size).bind(pnl).bind(roi_pct).bind(oldest.timestamp).bind(now).bind(if order.order_type == "LIMIT" { "TP" } else { "SL" })
            .execute(&mut *tx).await.ok();

        // Recalculate aggregates; clean up if no active slots remain
        let remaining = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM position_slots WHERE symbol = ?1 AND is_active = 1")
            .bind(symbol).fetch_one(&mut *tx).await.unwrap_or(0);
        if remaining == 0 {
            sqlx::query("DELETE FROM active_positions WHERE symbol = ?1").bind(symbol).execute(&mut *tx).await.ok();
            sqlx::query("DELETE FROM position_slots WHERE symbol = ?1").bind(symbol).execute(&mut *tx).await.ok();
            sqlx::query("DELETE FROM open_orders WHERE associated_position_id = ?1").bind(pos.id).execute(&mut *tx).await.ok();
        } else {
            // Recalculate aggregate position fields
            let total_size: f64 = sqlx::query_scalar("SELECT COALESCE(SUM(size), 0.0) FROM position_slots WHERE symbol = ?1 AND is_active = 1")
                .bind(symbol).fetch_one(&mut *tx).await.unwrap_or(0.0);
            let total_alloc: f64 = sqlx::query_scalar("SELECT COALESCE(SUM(allocated_usd), 0.0) FROM position_slots WHERE symbol = ?1 AND is_active = 1")
                .bind(symbol).fetch_one(&mut *tx).await.unwrap_or(0.0);
            let weighted_price = if total_size > 0.0 {
                sqlx::query_scalar::<_, f64>("SELECT COALESCE(SUM(entry_price * size) / SUM(size), 0.0) FROM position_slots WHERE symbol = ?1 AND is_active = 1")
                    .bind(symbol).fetch_one(&mut *tx).await.unwrap_or(0.0)
            } else { 0.0 };
            let count = remaining as i32;
            sqlx::query("UPDATE active_positions SET size = ?2, allocated_usd = ?3, average_entry_price = ?4, current_portions = ?5 WHERE symbol = ?1")
                .bind(symbol).bind(total_size).bind(total_alloc).bind(weighted_price).bind(count)
                .execute(&mut *tx).await.ok();
        }
        let _ = tx.commit().await;
    }

    let _ = telemetry_tx.send(db::TelemetryMsg::JournalTrade {
        symbol: symbol.to_string(), direction: oldest.direction.clone(),
        entry_price: oldest.entry_price, exit_price: current_price,
        entry_timestamp: oldest.timestamp, exit_timestamp: now,
        size: oldest.size, realized_pnl: pnl,
        roi_pct: if oldest.allocated_usd > 0.0 { (pnl / oldest.allocated_usd) * 100.0 } else { 0.0 },
        allocated_usd: oldest.allocated_usd,
        trigger: if order.order_type == "LIMIT" { "TP".to_string() } else { "SL".to_string() },
    }).await;
}

async fn handle_standard_entry_fill(
    pool: &SqlitePool,
    telemetry_tx: &mpsc::Sender<db::TelemetryMsg>,
    symbol: &str,
    order: &db::OpenOrder,
    current_price: f64,
) {
    let dir = if order.direction == "BUY" { "LONG" } else { "SHORT" };
    let result =
        paper_trading::open_slot_internal(pool, telemetry_tx, symbol, dir, current_price).await;
    if result.success {
        println!(
            "📄 Order Matcher: Filled {} entry for {} at ${:.2} (slot {})",
            order.order_type, symbol, current_price, result.slot_index
        );
    } else {
        eprintln!(
            "⚠️ Order Matcher: Failed to fill {} entry for {}: {}",
            order.order_type, symbol, result.message
        );
    }
}
