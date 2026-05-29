use sqlx::SqlitePool;

async fn setup_test_db() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to connect to in-memory SQLite database");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS individual_indicator_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            master_record_id INTEGER NOT NULL,
            indicator_name TEXT NOT NULL,
            signal TEXT NOT NULL,
            reason TEXT NOT NULL,
            timestamp INTEGER NOT NULL
        )"
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS master_assistant_records (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            created_at TEXT DEFAULT (datetime('now')),
            position TEXT NOT NULL,
            entry_price TEXT,
            price_at_analysis TEXT NOT NULL,
            general_trend TEXT NOT NULL,
            support_levels TEXT NOT NULL,
            resistance_levels TEXT NOT NULL,
            indicator_synthesis_summary TEXT NOT NULL,
            indicator_synthesis_evaluation TEXT NOT NULL,
            recommended_action TEXT NOT NULL,
            recommendation_rationale TEXT NOT NULL,
            symbol TEXT NOT NULL
        )"
    )
    .execute(&pool)
    .await
    .unwrap();

    pool
}

#[tokio::test]
async fn test_orchestrator_database_pipeline() {
    let pool = setup_test_db().await;

    let master_id = engine::db::insert_master_placeholder(
        &pool,
        "Long",
        "3100.00",
        "3125.50",
        "ETH"
    ).await;
    assert!(master_id > 0, "Master ID should be a valid incrementing integer");

    engine::db::insert_individual_log(
        &pool,
        master_id,
        "RSI",
        "BULLISH",
        "RSI is above 50 and rising"
    ).await;

    engine::db::update_master_record(
        &pool,
        master_id,
        "UPWARD",
        "[\"3100.00\"]",
        "[\"3150.00\"]",
        "1 Bullish, 0 Bearish",
        "Supported by technical indicators",
        "Hold",
        "Trend is upward and indicators are strong"
    ).await;

    let records = engine::db::query_master_records(&pool, 1).await;
    assert_eq!(records.len(), 1);
    let record = &records[0];
    assert_eq!(record.id, master_id);
    assert_eq!(record.position, "Long");
    assert_eq!(record.general_trend, "UPWARD");
    assert_eq!(record.recommended_action, "Hold");
}
