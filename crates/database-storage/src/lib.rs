use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqlitePool;

pub mod analyzer_normalize_fallback;
pub mod crypto;
pub mod logger;
pub mod queries;
pub mod seed;

// ─── Logger re-exports ─────────────────────────────────────────────

pub use logger::{run_telemetry_logger, TelemetryMsg};

// ─── Query re-exports ──────────────────────────────────────────────

pub use queries::analytics::{
    insert_backtest_run, insert_optimization_report, insert_performance_matrix_snapshot,
    insert_performance_summary, insert_risk_analytics, insert_strategy_analytics,
    query_backtest_run, query_backtest_runs_list, query_optimization_reports,
    query_performance_matrix_latest, query_risk_analytics_latest,
    query_strategy_analytics_history, BacktestRunRow,
};
pub use queries::journals::{
    insert_trade_journal, query_recent_journal_for_context, query_trade_journal,
    update_journal_notes, TradeJournalRecord,
};
pub use queries::profiles::{
    decision_profile_delete, decision_profile_insert, decision_profile_update,
    decision_profiles_list, profile_indicator_delete, profile_indicator_insert,
    profile_indicator_update, risk_profile_by_id, risk_profile_delete, risk_profile_insert,
    risk_profile_update, risk_profiles_list, DecisionProfile, ProfileIndicator, RiskProfile,
};
pub use queries::snapshots::{
    insert_snapshot_internal, query_backtest_coverage, query_backtest_snapshots,
    query_closest_close_price, query_latest_snapshot, query_recent_candles,
    BacktestCoverageRow, RecordedSnapshot,
};
pub use queries::stats::{
    dash_trade_detail, dash_trade_timestamps, get_daily_pnl, query_all_closed_trades,
    ClosedTradeRow, TradeDetailRow,
};
pub use queries::trades::{
    insert_user_trade, query_user_trades, trade_telemetry_count, trade_telemetry_insert,
    trade_telemetry_query_all, TradeTelemetryRecord, UserTrade,
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
