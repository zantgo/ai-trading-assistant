use core_domain::models::MarketSnapshot;
use core_domain::normalized::Exchange;
use sqlx::SqlitePool;

#[derive(Debug)]
pub enum TelemetryMsg {
    InsertSnapshot(MarketSnapshot),
    InsertIndividualLog {
        master_record_id: i64,
        indicator_name: String,
        signal: String,
        reason: String,
        timeframe_secs: u64,
    },
    UpdateMasterRecord {
        master_id: i64,
        general_trend: String,
        support_levels: String,
        resistance_levels: String,
        indicator_synthesis_summary: String,
        indicator_synthesis_evaluation: String,
        recommended_action: String,
        recommendation_rationale: String,
        score_points: Option<i32>,
        signals_json: Option<String>,
    },
    ConsoleLog(String),
    JournalTrade {
        symbol: String,
        direction: String,
        entry_price: f64,
        exit_price: f64,
        entry_timestamp: i64,
        exit_timestamp: i64,
        size: f64,
        realized_pnl: f64,
        roi_pct: f64,
        allocated_usd: f64,
        trigger: String,
    },
    /// Real liquidation event captured from exchange WS (Phase 1+).
    InsertLiquidationEvent {
        exchange: Exchange,
        symbol: String,
        side: String,
        price: f64,
        size_usd: f64,
        timestamp_ms: u64,
        venue_order_id: Option<String>,
    },
}

pub async fn run_telemetry_logger(
    pool: SqlitePool,
    mut rx: tokio::sync::mpsc::Receiver<TelemetryMsg>,
) {
    println!("Telemetry & Logging Worker: Background log thread running.");

    // Initial cleanup on startup — delete snapshots older than 7 days.
    let cutoff = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .saturating_sub(7 * 86400) as i64;
    if let Err(e) = sqlx::query("DELETE FROM market_snapshots WHERE timestamp < ?1")
        .bind(cutoff)
        .execute(&pool)
        .await
    {
        eprintln!("DB cleanup error on startup: {}", e);
    }

    let mut last_cleanup = tokio::time::Instant::now();

    while let Some(msg) = rx.recv().await {
        // Periodic cleanup every hour — delete snapshots older than 7 days.
        if last_cleanup.elapsed() >= tokio::time::Duration::from_secs(3600) {
            let cutoff = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                .saturating_sub(7 * 86400) as i64;
            if let Err(e) = sqlx::query("DELETE FROM market_snapshots WHERE timestamp < ?1")
                .bind(cutoff)
                .execute(&pool)
                .await
            {
                eprintln!("DB cleanup error: {}", e);
            }
            last_cleanup = tokio::time::Instant::now();
        }
        match msg {
            TelemetryMsg::InsertSnapshot(snapshot) => {
                crate::queries::snapshots::insert_snapshot_internal(&pool, &snapshot).await;
            }
            TelemetryMsg::InsertIndividualLog { .. } => {
                // No-op: master records migrated out
            }
            TelemetryMsg::UpdateMasterRecord { .. } => {
                // No-op: master records migrated out
            }
            TelemetryMsg::ConsoleLog(log_text) => {
                println!("{}", log_text);
            }
            TelemetryMsg::JournalTrade {
                symbol,
                direction,
                entry_price,
                exit_price,
                entry_timestamp,
                exit_timestamp,
                size,
                realized_pnl,
                roi_pct,
                allocated_usd,
                trigger,
            } => {
                let pool_clone = pool.clone();
                tokio::spawn(async move {
                    run_journaling_task(
                        &pool_clone,
                        &symbol,
                        &direction,
                        entry_price,
                        exit_price,
                        entry_timestamp,
                        exit_timestamp,
                        size,
                        realized_pnl,
                        roi_pct,
                        allocated_usd,
                        &trigger,
                    )
                    .await;
                });
            }
            TelemetryMsg::InsertLiquidationEvent {
                exchange,
                symbol,
                side,
                price,
                size_usd,
                timestamp_ms,
                venue_order_id,
            } => {
                let exchange_str = match exchange {
                    Exchange::Hyperliquid => "Hyperliquid",
                    Exchange::Bitget => "Bitget",
                };
                if let Err(e) = sqlx::query(
                    "INSERT INTO liquidation_events
                        (exchange, symbol, side, price, size_usd, timestamp, venue_order_id)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                )
                .bind(exchange_str)
                .bind(&symbol)
                .bind(&side)
                .bind(price)
                .bind(size_usd)
                .bind(timestamp_ms as i64)
                .bind(&venue_order_id)
                .execute(&pool)
                .await
                {
                    eprintln!("DB error inserting liquidation event: {}", e);
                }
            }
        }
    }
}

async fn run_journaling_task(
    pool: &SqlitePool,
    symbol: &str,
    direction: &str,
    _entry_price: f64,
    _exit_price: f64,
    _entry_timestamp: i64,
    _exit_timestamp: i64,
    _size: f64,
    realized_pnl: f64,
    roi_pct: f64,
    allocated_usd: f64,
    trigger: &str,
) {
    let roe_pct = if allocated_usd > 0.0 {
        (realized_pnl / allocated_usd) * 100.0
    } else {
        roi_pct
    };

    let entry_date = format_ts(_entry_timestamp);
    let exit_date = format_ts(_exit_timestamp);
    let asset = symbol.to_string();

    let entry_reason = String::new();

    let notes = format!("[Market Monitor trade journal. Trigger: {}]", trigger);

    crate::queries::journals::insert_trade_journal(
        pool,
        0,
        &entry_date,
        &exit_date,
        &asset,
        direction,
        &entry_reason,
        roe_pct,
        &notes,
        5.0,
    )
    .await;

    println!("Trade Journal: {} {} recorded", symbol, direction);
}

fn format_ts(ms: i64) -> String {
    use chrono::TimeZone;
    let secs = ms / 1000;
    match chrono::Utc.timestamp_opt(secs, 0) {
        chrono::LocalResult::Single(dt) => dt.format("%Y-%m-%d %H:%M:%S").to_string(),
        _ => secs.to_string(),
    }
}
