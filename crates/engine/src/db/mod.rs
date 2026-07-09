use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqlitePool;

pub mod crypto;
pub mod logger;
pub mod paper;
pub mod queries;
pub mod seed;

// ─── Paper re-exports ──────────────────────────────────────────────

pub use paper::{
    paper_count_brackets_by_type, paper_ensure_balance, paper_fetch_equity_history,
    paper_find_vacant_slot, paper_get_account_metrics, paper_get_active_position,
    paper_get_active_slot_count, paper_get_active_slots, paper_get_balance,
    paper_get_brackets_for_position, paper_get_oldest_active_slot, paper_get_open_orders,
    paper_insert_equity_snapshot, paper_query_trades, paper_reset_account,
    paper_set_advanced_config, paper_set_balance_config, ActivePaperPosition, OpenOrder,
    PaperAccountMetrics, PaperBalance, PaperTradeRecord, PositionSlotRecord, ScaleInPortionRecord,
};

// ─── Logger re-exports ─────────────────────────────────────────────

pub use logger::{run_telemetry_logger, TelemetryMsg};

// ─── Edge re-exports ───────────────────────────────────────────────

pub use queries::edges::{
    edge_analytics_cache_get, edge_analytics_cache_upsert, edges_delete, edges_get, edges_insert,
    edges_list,
};

// ─── Query re-exports ──────────────────────────────────────────────

pub use queries::exchange_keys::{
    exchange_keys_active_count, exchange_keys_delete, exchange_keys_insert, exchange_keys_list,
    exchange_keys_update_sync, ExchangeKey,
};
pub use queries::journals::{
    insert_trade_journal, query_recent_journal_for_context, query_trade_journal,
    update_journal_notes, TradeJournalRecord,
};
pub use queries::master::{
    insert_individual_log_internal, insert_master_placeholder, query_master_action_by_id,
    query_master_records, query_master_records_by_trigger, update_master_record_internal,
    MasterRecord,
};
pub use queries::memory::{
    insert_agent_thought_log, insert_decision_memory_buffer, query_completed_trades_buffer,
    query_decision_memory_buffer, CompletedTradesBufferRow, DecisionMemoryBufferRow,
};
pub use queries::performance::{
    insert_automated_performance_baseline, query_automated_performance,
    query_pending_performance_entries, update_performance_tracker_prices, AutomatedPerformanceRow,
};
pub use queries::profiles::{
    decision_profile_delete, decision_profile_insert, decision_profile_update,
    decision_profiles_list, profile_indicator_delete, profile_indicator_insert,
    profile_indicator_update, risk_profile_by_id, risk_profile_delete, risk_profile_insert,
    risk_profile_update, risk_profiles_list, DecisionProfile, ProfileIndicator, RiskProfile,
};
pub use queries::snapshots::{
    insert_snapshot_internal, query_atr_snapshots, query_closest_close_price,
    query_indicator_snapshots, query_latest_snapshot, query_recent_candles, IndicatorSnapshotRow,
};
pub use queries::stats::{
    dash_trade_detail, dash_trade_timestamps, get_daily_pnl, insert_optimization_report,
    query_all_closed_trades, ClosedTradeRow, TradeDetailRow,
};
pub use queries::trades::{
    insert_user_trade, query_user_trades, trade_telemetry_count, trade_telemetry_insert,
    trade_telemetry_query_all, TradeTelemetryRecord, UserTrade,
};
pub use queries::risk::{
    insert_risk_event, insert_rr_calibration, latest_risk_event, latest_rr_block_index,
    latest_rr_calibration, pair_initial_capital, pair_realized_pnls, pair_recent_ohlc,
    pair_trade_count, RiskEventRow, RrCalibrationRow,
};

// ─── Init ──────────────────────────────────────────────────────────

/// Run all embedded schema migrations against the given pool. Exposed so
/// integration tests can build the real schema on an in-memory database.
pub async fn run_migrations(pool: &SqlitePool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}

pub async fn verify_encryption_or_panic(pool: &SqlitePool) {
    if crypto::master_key_available() {
        return;
    }
    let row: Result<(i64,), _> = sqlx::query_as("SELECT COUNT(*) FROM exchange_keys")
        .fetch_one(pool)
        .await;
    if let Ok((count,)) = row {
        if count > 0 {
            panic!(
                "SECURITY: EXCHANGE_SECRET_KEY is not set but {} exchange key(s) exist in the database. \
                 Plaintext credential storage is prohibited. \
                 Set EXCHANGE_SECRET_KEY in your environment or .env file.",
                count
            );
        }
    }
}

pub async fn init_db() -> SqlitePool {
    let db_options = SqliteConnectOptions::new()
        .filename("telemetry.db")
        .create_if_missing(true)
        .busy_timeout(std::time::Duration::from_secs(5));

    let pool = SqlitePool::connect_with(db_options)
        .await
        .expect("Database Setup: Failed to initialize SQLite database pool");

    if let Err(e) = sqlx::query("PRAGMA journal_mode = WAL;")
        .execute(&pool)
        .await
    {
        eprintln!("Database: Failed to set PRAGMA journal_mode=WAL: {}", e);
    }
    if let Err(e) = sqlx::query("PRAGMA synchronous = NORMAL;")
        .execute(&pool)
        .await
    {
        eprintln!("Database: Failed to set PRAGMA synchronous=NORMAL: {}", e);
    }

    run_migrations(&pool)
        .await
        .expect("Database Setup: Failed to run schema migrations");

    seed::seed_default_profiles(&pool).await;

    pool
}
