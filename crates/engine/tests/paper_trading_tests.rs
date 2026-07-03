use engine::db;
use engine::llm::LlmClient;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::mpsc;

async fn setup_paper_db() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to connect to in-memory SQLite database");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations in test setup");

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
        "INSERT OR REPLACE INTO paper_balances (symbol, initial_usd, current_cash, allocation_pct, auto_execute, max_risk_pct, leverage, auto_execute_intervals, lookback_trades, break_even_trail_enabled, leverage_mode, leverage_cap, atr_leverage_multiplier)
         VALUES (?1, ?2, ?3, ?4, 0, 2.0, 20, 15, 10, 0, 'Fixed', 20, 0.0)",
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
        &pool,
        &tx,
        "BTC",
        "LONG",
        50000.0,
        Some(20.0),
    )
    .await;

    assert!(result.success, "Position should open: {}", result.message);
    assert_eq!(result.entry_price.unwrap(), 50000.0);
    let expected_size = 2000.0 / 50000.0;
    assert!(
        (result.size.unwrap() - expected_size).abs() < 0.0001,
        "Size should be {} but got {}",
        expected_size,
        result.size.unwrap()
    );
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
        &pool,
        &tx,
        "ETH",
        "LONG",
        3000.0,
        Some(10.0),
    )
    .await;
    assert!(r1.success);

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let r2 = engine::paper_trading::verify_margin_and_open_with_alloc(
        &pool,
        &tx,
        "ETH",
        "SHORT",
        3100.0,
        Some(10.0),
    )
    .await;
    assert!(
        !r2.success,
        "Should reject duplicate position: {}",
        r2.message
    );
    assert!(r2.message.contains("already has an active"));
}

#[tokio::test]
async fn test_close_position_pnl() {
    let pool = setup_paper_db().await;
    seed_balance(&pool, "SOL", 10000.0, 10.0).await;

    let tx = spawn_logger(pool.clone());

    let open = engine::paper_trading::verify_margin_and_open_with_alloc(
        &pool,
        &tx,
        "SOL",
        "LONG",
        100.0,
        Some(10.0),
    )
    .await;
    assert!(open.success);
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let close = engine::paper_trading::close_paper_position(&pool, &tx, "SOL", 110.0, "TEST").await;
    assert!(close.success, "Close failed: {}", close.message);
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let balance = db::paper_get_balance(&pool, "SOL").await;
    assert!(
        (balance.current_cash - 10100.0).abs() < 0.01,
        "Expected ~10100 but got {}",
        balance.current_cash
    );

    let pos = db::paper_get_active_position(&pool, "SOL").await;
    assert!(pos.is_none());
}

#[tokio::test]
async fn test_close_position_loss() {
    let pool = setup_paper_db().await;
    seed_balance(&pool, "AVAX", 10000.0, 10.0).await;

    let tx = spawn_logger(pool.clone());

    engine::paper_trading::verify_margin_and_open_with_alloc(
        &pool,
        &tx,
        "AVAX",
        "LONG",
        50.0,
        Some(10.0),
    )
    .await;
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    engine::paper_trading::close_paper_position(&pool, &tx, "AVAX", 45.0, "STOP_LOSS").await;
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let balance = db::paper_get_balance(&pool, "AVAX").await;
    assert!(
        (balance.current_cash - 9900.0).abs() < 0.01,
        "Expected ~9900 but got {}",
        balance.current_cash
    );
}

#[tokio::test]
async fn test_position_invalidation() {
    let pool = setup_paper_db().await;
    seed_balance(&pool, "DOT", 10000.0, 10.0).await;

    let tx = spawn_logger(pool.clone());

    let open =
        engine::paper_trading::verify_margin_and_open_with_alloc(&pool, &tx, "DOT", "SHORT", 10.0, Some(10.0)).await;
    assert!(open.success, "Open failed: {}", open.message);

    // Wait for logger to persist the position
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    let inval =
        engine::paper_trading::invalidate_position(&pool, &tx, "DOT", 12.0, "DECISIVE_CLOSE").await;
    assert!(inval.success, "Invalidate failed: {}", inval.message);

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    let pos = db::paper_get_active_position(&pool, "DOT").await;
    assert!(pos.is_none());
}

#[tokio::test]
async fn test_open_position_pct_opens_position() {
    let pool = setup_paper_db().await;
    seed_balance(&pool, "MATIC", 10000.0, 25.0).await;

    let tx = spawn_logger(pool.clone());

    let result =
        engine::paper_trading::open_position_pct(&pool, &tx, "MATIC", "LONG", 25.0, 1.00).await;
    assert!(result.success, "Failed open: {}", result.message);
    assert_eq!(result.position_pct as i32, 25);

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
        &pool,
        &tx,
        "LINK",
        "LONG",
        100.0,
        Some(10.0),
    )
    .await;
    assert!(open.success);
    let allocated = open.allocated_usd.unwrap();
    assert!((allocated - 1000.0).abs() < 0.01);
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Close at same price — no profit/loss
    let close =
        engine::paper_trading::close_paper_position(&pool, &tx, "LINK", 100.0, "TEST").await;
    assert!(close.success);
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Cash should be back to 10000 (9000 remaining + 1000 refunded = 10000)
    let balance = db::paper_get_balance(&pool, "LINK").await;
    assert!(
        (balance.current_cash - 10000.0).abs() < 0.01,
        "Expected ~10000 but got {}",
        balance.current_cash
    );
}

#[tokio::test]
async fn test_trailing_stop_to_break_even() {
    let pool = setup_paper_db().await;
    seed_balance(&pool, "BTC", 10000.0, 30.0).await;
    // Break-even trailing is gated OFF by default; enable it for this pair.
    sqlx::query("UPDATE paper_balances SET break_even_trail_enabled = 1 WHERE symbol = 'BTC'")
        .execute(&pool)
        .await
        .unwrap();

    let tx = spawn_logger(pool.clone());

    let open = engine::paper_trading::verify_margin_and_open_with_alloc(
        &pool,
        &tx,
        "BTC",
        "LONG",
        50000.0,
        Some(30.0),
    )
    .await;
    assert!(open.success);
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let pos = db::paper_get_active_position(&pool, "BTC").await.unwrap();
    let pos_id = pos.id;

    // Set a TP bracket via public API (writes to open_orders)
    engine::paper_trading::set_take_profit_targets(&pool, "BTC", &[(50.0, 55000.0)]).await.unwrap();

    let triggered = engine::paper_trading::check_break_even_trail(&pool, "BTC", 55000.0).await;
    assert!(
        triggered,
        "TP1 should trigger break-even trail when price crosses 55000"
    );

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let pos_after = db::paper_get_active_position(&pool, "BTC").await.unwrap();
    assert!(
        (pos_after.final_invalidation_level.unwrap_or(0.0) - 50000.0).abs() < 0.01,
        "SL should be moved to entry price (50000.0) for break-even, got {:?}",
        pos_after.final_invalidation_level
    );

    // Verify the TP order was deleted (marked as filled)
    let remaining: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM open_orders WHERE associated_position_id = ?1 AND order_type = 'LIMIT'")
            .bind(pos_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(remaining.0, 0, "TP1 should be deleted from open_orders after trigger");
}

#[tokio::test]
async fn test_close_emits_journal_trade() {
    let pool = setup_paper_db().await;

    seed_balance(&pool, "ADA", 10000.0, 10.0).await;

    let tx = spawn_logger(pool.clone());

    let open = engine::paper_trading::verify_margin_and_open_with_alloc(
        &pool,
        &tx,
        "ADA",
        "LONG",
        1.00,
        Some(10.0),
    )
    .await;
    assert!(open.success);
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let close =
        engine::paper_trading::close_paper_position(&pool, &tx, "ADA", 1.20, "MANUAL").await;
    assert!(close.success);
    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    let trade_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM paper_trades WHERE symbol = 'ADA'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(
        trade_count.0 >= 1,
        "Paper trades should record the closed trade for ADA"
    );

    let balance = db::paper_get_balance(&pool, "ADA").await;
    assert!(
        balance.current_cash > 10000.0,
        "Cash should be > 10000 after profitable close (entry 1.00, exit 1.20), got {}",
        balance.current_cash
    );
}
