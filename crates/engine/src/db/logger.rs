use crate::llm::LlmClient;
use shared::models::MarketSnapshot;
use sqlx::SqlitePool;
use std::sync::Arc;

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
    PaperOpenPosition {
        symbol: String,
        direction: String,
        entry_price: f64,
        size: f64,
        allocated_usd: f64,
        pct: f64,
    },
    PaperClosePosition {
        symbol: String,
        exit_price: f64,
        exit_timestamp: i64,
        trigger: String,
    },
    PaperUpdateBalance {
        symbol: String,
        current_cash: f64,
    },
    PaperScaleInPortion {
        symbol: String,
        direction: String,
        entry_price: f64,
        size: f64,
        allocated_usd: f64,
        portion_number: i32,
        new_average_entry_price: f64,
        total_size: f64,
        final_invalidation_level: f64,
    },
    PaperScaleOutPortion {
        symbol: String,
        exit_price: f64,
        size_fraction: f64,
        realized_pnl: f64,
        remaining_size: f64,
        target_id: i64,
    },
    PaperInvalidatePosition {
        symbol: String,
        exit_price: f64,
        exit_timestamp: i64,
        realized_loss: f64,
        reason: String,
    },
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
}

pub async fn run_telemetry_logger(
    pool: SqlitePool,
    mut rx: tokio::sync::mpsc::Receiver<TelemetryMsg>,
    llm_client: Arc<LlmClient>,
) {
    println!("Telemetry & Logging Worker: Background log thread running.");
    while let Some(msg) = rx.recv().await {
        match msg {
            TelemetryMsg::InsertSnapshot(snapshot) => {
                crate::db::queries::snapshots::insert_snapshot_internal(&pool, &snapshot).await;
            }
            TelemetryMsg::InsertIndividualLog {
                master_record_id,
                indicator_name,
                signal,
                reason,
                timeframe_secs,
            } => {
                crate::db::queries::master::insert_individual_log_internal(
                    &pool,
                    master_record_id,
                    &indicator_name,
                    &signal,
                    &reason,
                    timeframe_secs,
                )
                .await;
            }
            TelemetryMsg::UpdateMasterRecord {
                master_id,
                general_trend,
                support_levels,
                resistance_levels,
                indicator_synthesis_summary,
                indicator_synthesis_evaluation,
                recommended_action,
                recommendation_rationale,
                score_points,
                signals_json,
            } => {
                crate::db::queries::master::update_master_record_internal(
                    &pool,
                    master_id,
                    &general_trend,
                    &support_levels,
                    &resistance_levels,
                    &indicator_synthesis_summary,
                    &indicator_synthesis_evaluation,
                    &recommended_action,
                    &recommendation_rationale,
                    score_points,
                    signals_json,
                )
                .await;
            }
            TelemetryMsg::ConsoleLog(log_text) => {
                println!("{}", log_text);
            }
            TelemetryMsg::PaperOpenPosition {
                symbol,
                direction,
                entry_price,
                size,
                allocated_usd,
                ..
            } => {
                if let Err(e) = crate::db::paper::paper_open_position_internal(
                    &pool,
                    &symbol,
                    &direction,
                    entry_price,
                    size,
                    allocated_usd,
                )
                .await
                {
                    eprintln!("⚠️ Paper DB: Failed to open position for {}: {}", symbol, e);
                }
            }
            TelemetryMsg::PaperClosePosition {
                symbol,
                exit_price,
                exit_timestamp,
                trigger,
            } => {
                if let Err(e) = crate::db::paper::paper_close_position_internal(
                    &pool,
                    &symbol,
                    exit_price,
                    exit_timestamp,
                    &trigger,
                )
                .await
                {
                    eprintln!(
                        "⚠️ Paper DB: Failed to close position for {}: {}",
                        symbol, e
                    );
                }
            }
            TelemetryMsg::PaperUpdateBalance {
                symbol,
                current_cash,
            } => {
                if let Err(e) =
                    crate::db::paper::paper_update_balance_internal(&pool, &symbol, current_cash)
                        .await
                {
                    eprintln!(
                        "⚠️ Paper DB: Failed to update balance for {}: {}",
                        symbol, e
                    );
                }
            }
            TelemetryMsg::PaperScaleInPortion {
                symbol,
                direction,
                entry_price,
                size,
                allocated_usd,
                portion_number,
                new_average_entry_price,
                total_size,
                final_invalidation_level,
            } => {
                if let Err(e) = crate::db::paper::paper_scale_in_portion_internal(
                    &pool,
                    &symbol,
                    &direction,
                    entry_price,
                    size,
                    allocated_usd,
                    portion_number,
                    new_average_entry_price,
                    total_size,
                    final_invalidation_level,
                )
                .await
                {
                    eprintln!("⚠️ Paper DB: Failed scale-in for {}: {}", symbol, e);
                }
            }
            TelemetryMsg::PaperScaleOutPortion {
                symbol,
                exit_price,
                size_fraction,
                realized_pnl,
                remaining_size,
                target_id,
            } => {
                if let Err(e) = crate::db::paper::paper_scale_out_portion_internal(
                    &pool,
                    &symbol,
                    exit_price,
                    size_fraction,
                    realized_pnl,
                    remaining_size,
                    target_id,
                )
                .await
                {
                    eprintln!("⚠️ Paper DB: Failed scale-out for {}: {}", symbol, e);
                }
            }
            TelemetryMsg::PaperInvalidatePosition {
                symbol,
                exit_price,
                exit_timestamp,
                realized_loss,
                reason,
            } => {
                if let Err(e) = crate::db::paper::paper_invalidate_position_internal(
                    &pool,
                    &symbol,
                    exit_price,
                    exit_timestamp,
                    realized_loss,
                    &reason,
                )
                .await
                {
                    eprintln!("⚠️ Paper DB: Failed invalidation for {}: {}", symbol, e);
                }
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
                let llm = llm_client.clone();
                tokio::spawn(async move {
                    run_journaling_task(
                        &pool_clone,
                        &llm,
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
        }
    }
}

async fn run_journaling_task(
    pool: &SqlitePool,
    llm_client: &Arc<LlmClient>,
    symbol: &str,
    direction: &str,
    entry_price: f64,
    exit_price: f64,
    entry_timestamp: i64,
    exit_timestamp: i64,
    size: f64,
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

    let entry_date = format_ts(entry_timestamp);
    let exit_date = format_ts(exit_timestamp);
    let asset = symbol.to_string();

    let entry_reason = lookup_entry_reason(pool, symbol, entry_timestamp).await;

    let indicator_context =
        build_entry_exit_indicator_context(pool, symbol, entry_timestamp, exit_timestamp).await;

    let trade_context = format!(
        "TRADE AUDIT REQUEST\n\
         Asset: {}\n\
         Direction: {}\n\
         Entry Price: ${:.4}\n\
         Exit Price: ${:.4}\n\
         Size: {:.6}\n\
         Entry Time: {}\n\
         Exit Time: {}\n\
         Realized PnL: ${:.2}\n\
         ROI: {:.2}%\n\
         ROE (Return on Equity): {:.2}%\n\
         Allocated Capital: ${:.2}\n\
         Exit Trigger: {}\n\
         Technical Entry Reason: {}\n\
         \n\
         Indicator Context at Entry & Exit:\n{}\n\
         \n\
         Critically analyze this trade. Identify execution mistakes or successes.\n\
         Score the execution quality from 0.0 to 10.0 independent of whether the trade won or lost.",
        asset, direction, entry_price, exit_price, size,
        entry_date, exit_date, realized_pnl, roi_pct, roe_pct, allocated_usd, trigger,
        entry_reason, indicator_context,
    );

    if llm_client.api_key.read().await.is_empty() {
        let notes = format!(
            "[API key not configured at time of close. Trigger: {}]",
            trigger
        );
        crate::db::queries::journals::insert_trade_journal(
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
        return;
    }

    match llm_client
        .run_journal_agent(&trade_context, Some(symbol))
        .await
    {
        Ok(result) => {
            let trade_id =
                find_trade_telemetry_id(pool, symbol, entry_timestamp, exit_timestamp).await;
            crate::db::queries::journals::insert_trade_journal(
                pool,
                trade_id,
                &entry_date,
                &exit_date,
                &asset,
                direction,
                &entry_reason,
                roe_pct,
                &result.final_analysis,
                result.execution_score,
            )
            .await;
            println!(
                "Trade Journal: {} {} recorded — Score: {:.1}/10",
                symbol, direction, result.execution_score
            );
        }
        Err(e) => {
            eprintln!("Journal agent failed for {}: {}", symbol, e);
            let notes = format!("[Journal agent error: {}]", e);
            crate::db::queries::journals::insert_trade_journal(
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
        }
    }
}

fn format_ts(ms: i64) -> String {
    use chrono::TimeZone;
    let secs = ms / 1000;
    match chrono::Utc.timestamp_opt(secs, 0) {
        chrono::LocalResult::Single(dt) => dt.format("%Y-%m-%d %H:%M:%S").to_string(),
        _ => secs.to_string(),
    }
}

async fn lookup_entry_reason(pool: &SqlitePool, symbol: &str, entry_timestamp: i64) -> String {
    use sqlx::Row;
    let row = sqlx::query(
        "SELECT recommended_action, recommendation_rationale
         FROM master_assistant_records
         WHERE symbol = ?1 AND general_trend != 'PENDING'
         ORDER BY ABS(id - (SELECT COALESCE(MAX(id),0) FROM master_assistant_records WHERE symbol = ?1 AND created_at <= datetime(?2 / 1000, 'unixepoch')))
         LIMIT 1"
    )
    .bind(symbol)
    .bind(entry_timestamp)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    match row {
        Some(r) => {
            let action: String = r.get(0);
            let rationale: String = r.get(1);
            format!("{} — {}", action, rationale)
        }
        None => "No prior analysis record found".to_string(),
    }
}

async fn build_entry_exit_indicator_context(
    pool: &SqlitePool,
    symbol: &str,
    entry_timestamp: i64,
    exit_timestamp: i64,
) -> String {
    use sqlx::Row;

    async fn get_snapshot_at(pool: &SqlitePool, symbol: &str, ts: i64) -> String {
        let row = sqlx::query(
            "SELECT close, rsi_14, squeeze_on, squeeze_momentum, macd_line, macd_signal, macd_hist,
                    adx_14, atr_14, ema_fast, ema_slow, bb_upper, bb_lower
             FROM market_snapshots
             WHERE symbol = ?1 AND close IS NOT NULL AND timeframe_secs = 60
             ORDER BY ABS(timestamp - ?2 / 1000) ASC
             LIMIT 1",
        )
        .bind(symbol)
        .bind(ts)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();

        row.map(|r| {
            let close_val: String = r.get::<String, _>(0);
            let rsi_val: Option<String> = r.get(1);
            let sqz_on_val: Option<i32> = r.get(2);
            let sqz_mom_val: Option<String> = r.get(3);
            let macd_l_val: Option<String> = r.get(4);
            let macd_s_val: Option<String> = r.get(5);
            let macd_h_val: Option<String> = r.get(6);
            let adx_val: Option<String> = r.get(7);
            let atr_val: Option<String> = r.get(8);
            let ema_f_val: Option<String> = r.get(9);
            let ema_s_val: Option<String> = r.get(10);
            let bb_u_val: Option<String> = r.get(11);
            let bb_l_val: Option<String> = r.get(12);

            let squeeze_status = sqz_on_val.map(|v| if v != 0 { "ON" } else { "OFF" }).unwrap_or("N/A");
            format!(
                "  Close: ${} | RSI: {} | Squeeze: {} (Mom: {}) | MACD: L={} S={} H={} | ADX: {} | ATR: {} | EMA Fast: {} Slow: {} | BB Upper: {} Lower: {}",
                close_val,
                rsi_val.as_deref().unwrap_or("N/A"),
                squeeze_status,
                sqz_mom_val.as_deref().unwrap_or("N/A"),
                macd_l_val.as_deref().unwrap_or("N/A"),
                macd_s_val.as_deref().unwrap_or("N/A"),
                macd_h_val.as_deref().unwrap_or("N/A"),
                adx_val.as_deref().unwrap_or("N/A"),
                atr_val.as_deref().unwrap_or("N/A"),
                ema_f_val.as_deref().unwrap_or("N/A"),
                ema_s_val.as_deref().unwrap_or("N/A"),
                bb_u_val.as_deref().unwrap_or("N/A"),
                bb_l_val.as_deref().unwrap_or("N/A"),
            )
        }).unwrap_or_else(|| "  No snapshot available".to_string())
    }

    let entry_snap = get_snapshot_at(pool, symbol, entry_timestamp).await;
    let exit_snap = get_snapshot_at(pool, symbol, exit_timestamp).await;

    format!(
        "Entry indicators (approx):\n{}\nExit indicators (approx):\n{}",
        entry_snap, exit_snap
    )
}

async fn find_trade_telemetry_id(
    pool: &SqlitePool,
    symbol: &str,
    entry_timestamp: i64,
    exit_timestamp: i64,
) -> i64 {
    use sqlx::Row;
    let row = sqlx::query(
        "SELECT id FROM trade_telemetry_history
         WHERE symbol = ?1 AND entry_timestamp = ?2 AND exit_timestamp = ?3
         ORDER BY id DESC LIMIT 1",
    )
    .bind(symbol)
    .bind(entry_timestamp)
    .bind(exit_timestamp)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    row.map(|r| r.get::<i64, _>(0)).unwrap_or(0)
}
