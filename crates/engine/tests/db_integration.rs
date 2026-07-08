use engine::db::{run_telemetry_logger, TelemetryMsg};
use shared::TriggerType;
use sqlx::SqlitePool;
use std::sync::Arc;

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
        )",
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
            symbol TEXT NOT NULL,
            trigger_type TEXT NOT NULL DEFAULT 'Manual'
        )",
    )
    .execute(&pool)
    .await
    .unwrap();

    pool
}

#[tokio::test]
async fn test_orchestrator_database_pipeline() {
    let pool = setup_test_db().await;

    let (tx, rx) = tokio::sync::mpsc::channel::<TelemetryMsg>(100);

    // Spawn background logger to process channel messages
    let logger_pool = pool.clone();
    let (llm_client, _) = engine::llm::LlmClient::from_env();
    let llm = Arc::new(llm_client);
    tokio::spawn(async move {
        run_telemetry_logger(logger_pool, rx, llm).await;
    });

    let master_id = engine::db::insert_master_placeholder(
        &pool,
        "Long",
        "3100.00",
        "3125.50",
        "ETH",
        TriggerType::Manual,
    )
    .await;
    assert!(
        master_id > 0,
        "Master ID should be a valid incrementing integer"
    );

    tx.send(TelemetryMsg::InsertIndividualLog {
        master_record_id: master_id,
        indicator_name: "RSI".to_string(),
        signal: "BULLISH".to_string(),
        reason: "RSI is above 50 and rising".to_string(),
        timeframe_secs: 60,
    })
    .await
    .expect("Failed to send InsertIndividualLog");

    tx.send(TelemetryMsg::UpdateMasterRecord {
        master_id,
        general_trend: "UPWARD".to_string(),
        support_levels: "[\"3100.00\"]".to_string(),
        resistance_levels: "[\"3150.00\"]".to_string(),
        indicator_synthesis_summary: "1 Bullish, 0 Bearish".to_string(),
        indicator_synthesis_evaluation: "Supported by technical indicators".to_string(),
        recommended_action: "Hold".to_string(),
        recommendation_rationale: "Trend is upward and indicators are strong".to_string(),
        score_points: None,
        signals_json: None,
    })
    .await
    .expect("Failed to send UpdateMasterRecord");

    // Give the logger a moment to process
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let records = engine::db::query_master_records(&pool, 1).await;
    assert_eq!(records.len(), 1);
    let record = &records[0];
    assert_eq!(record.id, master_id);
    assert_eq!(record.position, "Long");
    assert_eq!(record.general_trend, "UPWARD");
    assert_eq!(record.recommended_action, "Hold");
}

#[tokio::test]
async fn test_normalized_snapshot_persistence_roundtrip() {
    use rust_decimal_macros::dec;
    use shared::indicators::normalized::{DivergenceState, NormalizationEngine};
    use shared::models::MarketSnapshot;
    use std::collections::HashMap;

    // Real schema (including the Phase 3 normalized columns) via migrations.
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    engine::db::run_migrations(&pool)
        .await
        .expect("migrations should apply cleanly");

    // Build a snapshot with a populated normalized indicator map.
    let mut indicators = HashMap::new();
    indicators.insert(
        "rsi".to_string(),
        NormalizationEngine::normalize_rsi(25.0, DivergenceState::None),
    );
    indicators.insert(
        "macd".to_string(),
        NormalizationEngine::normalize_macd(-12.4, -17.6, 5.2, 8.0, Some(1)),
    );
    indicators.insert("rvol".to_string(), NormalizationEngine::normalize_rvol(2.0));
    indicators.insert(
        "adx".to_string(),
        NormalizationEngine::normalize_adx(30.0, 30.0, 10.0, 1.0, false),
    );

    let snap = MarketSnapshot {
        exchange: Some(shared::normalized::Exchange::Hyperliquid),
        timeframe_secs: 60,
        timestamp: 1_718_000_000,
        symbol: "BTC".to_string(),
        is_completed: Some(true),
        mid_price: dec!(50000.00),
        bid_price: dec!(49999.5),
        ask_price: dec!(50000.5),
        bid_size: None,
        ask_size: None,
        funding_rate: None,
        prev_day_px: None,
        open: Some(dec!(49800.0)),
        high: Some(dec!(50200.0)),
        low: Some(dec!(49750.0)),
        close: Some(dec!(50000.0)),
        volume: Some(dec!(150.0)),
        average_volume: Some(dec!(120.0)),
        indicators,
        context: None,
        decision_context: None,
        
    };

    engine::db::insert_snapshot_internal(&pool, &snap).await;

    // Dedicated normalized columns received numeric data.
    let (rsi_norm, rsi_label): (Option<f64>, Option<String>) =
        sqlx::query_as("SELECT rsi_normalized, rsi_state_label FROM market_snapshots LIMIT 1")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(rsi_norm.unwrap() >= 0.70, "oversold rsi should map high");
    assert_eq!(rsi_label.as_deref(), Some("OVERSOLD_ACCUMULATION"));

    let aux: Option<String> =
        sqlx::query_scalar("SELECT auxiliary_normalized_data FROM market_snapshots LIMIT 1")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(aux.is_some(), "auxiliary JSON blob should be persisted");

    // Full map round-trips through query_latest_snapshot (via auxiliary JSON).
    let loaded = engine::db::query_latest_snapshot(&pool, "BTC", 60)
        .await
        .expect("snapshot should be retrievable");
    let loaded_rsi = loaded.indicators.get("rsi").expect("rsi present");
    assert_eq!(loaded_rsi.state_label, "OVERSOLD_ACCUMULATION");
    assert!(loaded_rsi.normalized >= 0.70);
    let loaded_macd = loaded.indicators.get("macd").expect("macd present");
    assert_eq!(loaded_macd.state_label, "BULLISH_CROSSOVER_ACCELERATING");
    assert!(loaded_macd.values.is_some(), "macd multi-line values preserved");
    assert_eq!(loaded.indicators.get("rvol").unwrap().normalized, 0.8);
}
