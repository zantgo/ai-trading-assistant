use sqlx::SqlitePool;

fn spawn_logger(pool: SqlitePool) -> tokio::sync::mpsc::Sender<engine::db::TelemetryMsg> {
    let (tx, rx) = tokio::sync::mpsc::channel::<engine::db::TelemetryMsg>(100);
    let logger_pool = pool.clone();
    let llm_client = std::sync::Arc::new(engine::llm::LlmClient::from_env().0);
    tokio::spawn(async move {
        engine::db::run_telemetry_logger(logger_pool, rx, llm_client).await;
    });
    tx
}

async fn setup_paper_db() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    pool
}

async fn seed_balance(pool: &SqlitePool, symbol: &str, cash: f64, alloc_pct: f64) {
    sqlx::query(
        "INSERT OR REPLACE INTO paper_balances (symbol, initial_usd, current_cash, allocation_pct, auto_execute, max_risk_pct, leverage, auto_execute_intervals, lookback_trades, break_even_trail_enabled, leverage_mode, leverage_cap, atr_leverage_multiplier)
         VALUES (?1, ?2, ?2, ?3, 0, 2.0, 20, 15, 10, 0, 'Fixed', 20, 0.0)",
    )
    .bind(symbol)
    .bind(cash)
    .bind(alloc_pct)
    .execute(pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn test_over_100_pct_rejected() {
    let pool = setup_paper_db().await;
    seed_balance(&pool, "BTC", 10000.0, 25.0).await;

    let tx = spawn_logger(pool.clone());

    let p1 =
        engine::paper_trading::open_position_pct(&pool, &tx, "BTC", "LONG", 75.0, 50000.0).await;
    assert!(p1.success, "75% open should succeed: {}", p1.message);
    assert_eq!(p1.position_pct as i32, 75);

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let p2 =
        engine::paper_trading::open_position_pct(&pool, &tx, "BTC", "LONG", 50.0, 51000.0).await;
    assert!(!p2.success, "50% scale-in on 75% should exceed 100%: {}", p2.message);
    assert!(p2.message.contains("exceeds 100%"));
}

#[tokio::test]
async fn test_netting_closes_existing_with_opposite() {
    let pool = setup_paper_db().await;
    seed_balance(&pool, "ETH", 10000.0, 25.0).await;

    let tx = spawn_logger(pool.clone());

    let open =
        engine::paper_trading::open_position_pct(&pool, &tx, "ETH", "LONG", 50.0, 3000.0).await;
    assert!(open.success, "LONG open should succeed");

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Open SHORT with 75% — expected to net: close 50% LONG, open 25% SHORT
    let net =
        engine::paper_trading::open_position_pct(&pool, &tx, "ETH", "SHORT", 75.0, 3100.0).await;
    assert!(net.success, "Netting should succeed: {}", net.message);
    assert_eq!(net.direction, "SHORT");
    // Should be 25% SHORT (75 - 50 = 25)
    assert!((net.position_pct - 25.0).abs() < 0.01, "Expected 25% SHORT, got {:.0}%", net.position_pct);
}
