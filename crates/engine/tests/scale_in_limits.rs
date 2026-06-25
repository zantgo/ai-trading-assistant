use engine::db;
use engine::llm::LlmClient;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::mpsc;

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
        )",
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
        )",
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
        )",
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
        )",
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
        )",
    )
    .execute(&pool)
    .await
    .unwrap();

    pool
}

fn spawn_logger(pool: SqlitePool) -> mpsc::Sender<db::TelemetryMsg> {
    let (tx, rx) = mpsc::channel::<db::TelemetryMsg>(100);
    let (llm_client, _) = LlmClient::from_env();
    let llm = Arc::new(llm_client);
    tokio::spawn(async move {
        db::run_telemetry_logger(pool, rx, llm).await;
    });
    tx
}

async fn seed_balance(pool: &SqlitePool, symbol: &str, cash: f64, alloc_pct: f64) {
    sqlx::query(
        "INSERT OR REPLACE INTO paper_balances (symbol, initial_usd, current_cash, allocation_pct)
         VALUES (?1, ?2, ?3, ?4)",
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
async fn test_fourth_scale_in_rejected() {
    let pool = setup_paper_db().await;
    seed_balance(&pool, "BTC", 10000.0, 30.0).await;

    let tx = spawn_logger(pool.clone());

    let p1 =
        engine::paper_trading::scale_in_portion(&pool, &tx, "BTC", "LONG", 50000.0, 1, 49000.0)
            .await;
    assert!(p1.success, "Portion 1 should succeed: {}", p1.message);
    assert_eq!(p1.portion_number, 1);

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let p2 =
        engine::paper_trading::scale_in_portion(&pool, &tx, "BTC", "LONG", 51000.0, 2, 49500.0)
            .await;
    assert!(p2.success, "Portion 2 should succeed: {}", p2.message);
    assert_eq!(p2.portion_number, 2);

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let p3 =
        engine::paper_trading::scale_in_portion(&pool, &tx, "BTC", "LONG", 52000.0, 3, 50000.0)
            .await;
    assert!(p3.success, "Portion 3 should succeed: {}", p3.message);
    assert_eq!(p3.portion_number, 3);

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let p4 =
        engine::paper_trading::scale_in_portion(&pool, &tx, "BTC", "LONG", 53000.0, 4, 50500.0)
            .await;
    assert!(!p4.success, "Portion 4 should be rejected: {}", p4.message);
    assert!(p4.message.contains("All 3 portions already filled"));
}

#[tokio::test]
async fn test_wrong_direction_scale_in_rejected() {
    let pool = setup_paper_db().await;
    seed_balance(&pool, "ETH", 10000.0, 30.0).await;

    let tx = spawn_logger(pool.clone());

    let open =
        engine::paper_trading::scale_in_portion(&pool, &tx, "ETH", "LONG", 3000.0, 1, 2900.0).await;
    assert!(open.success, "LONG open should succeed");

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let wrong =
        engine::paper_trading::scale_in_portion(&pool, &tx, "ETH", "SHORT", 3100.0, 2, 3200.0)
            .await;
    assert!(
        !wrong.success,
        "SHORT scale-in on LONG position should be rejected"
    );
    assert!(
        wrong.message.contains("Cannot"),
        "Rejection should mention direction mismatch, got: {}",
        wrong.message
    );
}
