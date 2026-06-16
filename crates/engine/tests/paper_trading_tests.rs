use engine::db;
use engine::llm::LlmClient;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

async fn setup_paper_db() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to connect to in-memory SQLite database");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS paper_balances (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            symbol TEXT NOT NULL UNIQUE,
            initial_usd REAL NOT NULL DEFAULT 10000.0,
            current_cash REAL NOT NULL DEFAULT 10000.0,
            allocation_pct REAL NOT NULL DEFAULT 10.0,
            auto_execute INTEGER NOT NULL DEFAULT 0,
            max_risk_pct REAL NOT NULL DEFAULT 2.0,
            leverage INTEGER NOT NULL DEFAULT 20,
            auto_execute_intervals INTEGER NOT NULL DEFAULT 15,
            lookback_trades INTEGER NOT NULL DEFAULT 10
        )"
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS active_positions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            symbol TEXT NOT NULL UNIQUE,
            direction TEXT NOT NULL,
            entry_price REAL NOT NULL,
            size REAL NOT NULL,
            allocated_usd REAL NOT NULL,
            entry_timestamp INTEGER NOT NULL,
            final_invalidation_level REAL,
            target_profit_ratio REAL DEFAULT 2.0,
            current_portions INTEGER DEFAULT 1,
            average_entry_price REAL
        )"
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS active_position_portions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            position_id INTEGER,
            symbol TEXT NOT NULL,
            direction TEXT NOT NULL,
            entry_price REAL NOT NULL,
            size REAL NOT NULL,
            allocated_usd REAL NOT NULL,
            portion_number INTEGER NOT NULL,
            timestamp INTEGER NOT NULL
        )"
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS position_take_profit_targets (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            position_id INTEGER,
            symbol TEXT NOT NULL,
            target_price REAL NOT NULL,
            size_fraction REAL NOT NULL,
            is_hit INTEGER NOT NULL DEFAULT 0,
            timestamp INTEGER NOT NULL
        )"
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS paper_trades (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            symbol TEXT NOT NULL,
            direction TEXT NOT NULL,
            entry_price REAL NOT NULL,
            exit_price REAL NOT NULL,
            size REAL NOT NULL,
            realized_pnl REAL NOT NULL,
            roi_pct REAL NOT NULL,
            entry_timestamp INTEGER NOT NULL,
            exit_timestamp INTEGER NOT NULL,
            trigger TEXT NOT NULL DEFAULT 'MANUAL'
        )"
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS trade_telemetry_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            exchange TEXT NOT NULL DEFAULT 'Hyperliquid',
            symbol TEXT NOT NULL,
            direction TEXT NOT NULL,
            entry_timestamp INTEGER NOT NULL,
            exit_timestamp INTEGER NOT NULL,
            entry_price REAL NOT NULL,
            exit_price REAL NOT NULL,
            size REAL NOT NULL,
            commission_fees REAL NOT NULL DEFAULT 0.0,
            funding_fees REAL NOT NULL DEFAULT 0.0,
            realized_pnl REAL NOT NULL,
            roi_percentage REAL NOT NULL DEFAULT 0.0,
            trigger_source TEXT NOT NULL DEFAULT 'MANUAL'
        )"
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS trade_learning_journal (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            trade_id INTEGER NOT NULL,
            entry_date TEXT NOT NULL,
            exit_date TEXT NOT NULL,
            asset TEXT NOT NULL,
            direction TEXT NOT NULL,
            entry_reason TEXT NOT NULL,
            roe_percentage REAL NOT NULL DEFAULT 0.0,
            final_analysis TEXT NOT NULL DEFAULT '',
            execution_score REAL NOT NULL DEFAULT 5.0,
            human_notes TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )"
    )
    .execute(&pool)
    .await
    .unwrap();

    pool
}

fn spawn_logger(pool: SqlitePool) -> mpsc::Sender<db::TelemetryMsg> {
    let (tx, rx) = mpsc::channel::<db::TelemetryMsg>(100);
    let (llm_client, _) = LlmClient::from_env();
    let llm = Arc::new(RwLock::new(llm_client));
    tokio::spawn(async move {
        db::run_telemetry_logger(pool, rx, llm).await;
    });
    tx
}

async fn seed_balance(pool: &SqlitePool, symbol: &str, cash: f64, alloc_pct: f64) {
    sqlx::query(
        "INSERT OR REPLACE INTO paper_balances (symbol, initial_usd, current_cash, allocation_pct)
         VALUES (?1, ?2, ?3, ?4)"
    )
    .bind(symbol)
    .bind(cash)
    .bind(cash)
    .bind(alloc_pct)
    .execute(pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn test_open_position_sizing() {
    let pool = setup_paper_db().await;
    seed_balance(&pool, "BTC", 10000.0, 20.0).await;

    let tx = spawn_logger(pool.clone());

    let result = engine::paper_trading::verify_margin_and_open_with_alloc(
        &pool, &tx, "BTC", "LONG", 50000.0, Some(20.0),
    ).await;

    assert!(result.success, "Position should open: {}", result.message);
    assert_eq!(result.entry_price.unwrap(), 50000.0);
    let expected_size = 2000.0 / 50000.0;
    assert!((result.size.unwrap() - expected_size).abs() < 0.0001,
        "Size should be {} but got {}", expected_size, result.size.unwrap());
    assert_eq!(result.allocated_usd.unwrap(), 2000.0);

    let balance = db::paper_get_balance(&pool, "BTC").await;
    assert_eq!(balance.current_cash, 8000.0);

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    let pos = db::paper_get_active_position(&pool, "BTC").await;
    assert!(pos.is_some(), "Active position should exist after open");
    let p = pos.unwrap();
    assert_eq!(p.direction, "LONG");
    assert_eq!(p.entry_price, 50000.0);
}

#[tokio::test]
async fn test_cannot_open_duplicate_position() {
    let pool = setup_paper_db().await;
    seed_balance(&pool, "ETH", 5000.0, 10.0).await;

    let tx = spawn_logger(pool.clone());

    let r1 = engine::paper_trading::verify_margin_and_open_with_alloc(
        &pool, &tx, "ETH", "LONG", 3000.0, Some(10.0),
    ).await;
    assert!(r1.success);

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let r2 = engine::paper_trading::verify_margin_and_open_with_alloc(
        &pool, &tx, "ETH", "SHORT", 3100.0, Some(10.0),
    ).await;
    assert!(!r2.success, "Should reject duplicate position: {}", r2.message);
    assert!(r2.message.contains("already has an active"));
}

#[tokio::test]
async fn test_close_position_pnl() {
    let pool = setup_paper_db().await;
    seed_balance(&pool, "SOL", 10000.0, 10.0).await;

    let tx = spawn_logger(pool.clone());

    let open = engine::paper_trading::verify_margin_and_open_with_alloc(
        &pool, &tx, "SOL", "LONG", 100.0, Some(10.0),
    ).await;
    assert!(open.success);
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let close = engine::paper_trading::close_paper_position(
        &pool, &tx, "SOL", 110.0, "TEST",
    ).await;
    assert!(close.success, "Close failed: {}", close.message);
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let balance = db::paper_get_balance(&pool, "SOL").await;
    assert!((balance.current_cash - 10100.0).abs() < 0.01,
        "Expected ~10100 but got {}", balance.current_cash);

    let pos = db::paper_get_active_position(&pool, "SOL").await;
    assert!(pos.is_none());
}

#[tokio::test]
async fn test_close_position_loss() {
    let pool = setup_paper_db().await;
    seed_balance(&pool, "AVAX", 10000.0, 10.0).await;

    let tx = spawn_logger(pool.clone());

    engine::paper_trading::verify_margin_and_open_with_alloc(
        &pool, &tx, "AVAX", "LONG", 50.0, Some(10.0),
    ).await;
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    engine::paper_trading::close_paper_position(
        &pool, &tx, "AVAX", 45.0, "STOP_LOSS",
    ).await;
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let balance = db::paper_get_balance(&pool, "AVAX").await;
    assert!((balance.current_cash - 9900.0).abs() < 0.01,
        "Expected ~9900 but got {}", balance.current_cash);
}

#[tokio::test]
async fn test_position_invalidation() {
    let pool = setup_paper_db().await;
    seed_balance(&pool, "DOT", 10000.0, 10.0).await;

    let tx = spawn_logger(pool.clone());

    let open = engine::paper_trading::scale_in_portion(
        &pool, &tx, "DOT", "SHORT", 10.0, 1, 12.0,
    ).await;
    assert!(open.success, "Open failed: {}", open.message);

    // Wait for logger to persist the position
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    let inval = engine::paper_trading::invalidate_position(
        &pool, &tx, "DOT", 12.0, "DECISIVE_CLOSE",
    ).await;
    assert!(inval.success, "Invalidate failed: {}", inval.message);
    assert!(inval.realized_pnl < 0.0);

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    let pos = db::paper_get_active_position(&pool, "DOT").await;
    assert!(pos.is_none());
}

#[tokio::test]
async fn test_scale_in_portion_1_opens_position() {
    let pool = setup_paper_db().await;
    seed_balance(&pool, "MATIC", 10000.0, 30.0).await;

    let tx = spawn_logger(pool.clone());

    let p1 = engine::paper_trading::scale_in_portion(
        &pool, &tx, "MATIC", "LONG", 1.00, 1, 0.90,
    ).await;
    assert!(p1.success, "Failed portion 1: {}", p1.message);
    assert_eq!(p1.portion_number, 1);
    assert!((p1.new_average_entry_price - 1.00).abs() < 0.001);

    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    let pos = db::paper_get_active_position(&pool, "MATIC").await;
    assert!(pos.is_some());
    let p = pos.unwrap();
    assert_eq!(p.current_portions.unwrap_or(0), 1);
}

#[tokio::test]
async fn test_close_refunds_correct_amount() {
    let pool = setup_paper_db().await;
    seed_balance(&pool, "LINK", 10000.0, 10.0).await;

    let tx = spawn_logger(pool.clone());

    let open = engine::paper_trading::verify_margin_and_open_with_alloc(
        &pool, &tx, "LINK", "LONG", 100.0, Some(10.0),
    ).await;
    assert!(open.success);
    let allocated = open.allocated_usd.unwrap();
    assert!((allocated - 1000.0).abs() < 0.01);
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Close at same price — no profit/loss
    let close = engine::paper_trading::close_paper_position(
        &pool, &tx, "LINK", 100.0, "TEST",
    ).await;
    assert!(close.success);
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Cash should be back to 10000 (9000 remaining + 1000 refunded = 10000)
    let balance = db::paper_get_balance(&pool, "LINK").await;
    assert!((balance.current_cash - 10000.0).abs() < 0.01,
        "Expected ~10000 but got {}", balance.current_cash);
}
