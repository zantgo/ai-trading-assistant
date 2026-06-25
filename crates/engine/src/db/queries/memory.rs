use sqlx::SqlitePool;

pub async fn insert_agent_thought_log(
    pool: &SqlitePool,
    master_record_id: i64,
    agent_name: &str,
    thought_process: &str,
    json_rpc_payload: &str,
    confidence_score: i32,
) {
    if let Err(e) = sqlx::query(
        "INSERT INTO agent_thought_logs (master_record_id, agent_name, thought_process, json_rpc_payload, confidence_score) \
         VALUES (?1, ?2, ?3, ?4, ?5)"
    )
    .bind(master_record_id).bind(agent_name).bind(thought_process)
    .bind(json_rpc_payload).bind(confidence_score)
    .execute(pool).await
    {
        eprintln!("Database Error: Failed to save agent thought log for {}: {}", agent_name, e);
    }
}

pub async fn insert_decision_memory_buffer(
    pool: &SqlitePool,
    symbol: &str,
    timestamp: i64,
    regime: &str,
    decision: &str,
    confidence: i32,
    score: i32,
    risk: f64,
) {
    if let Err(e) = sqlx::query(
        "INSERT INTO decision_memory_buffer (symbol, timestamp, regime_classification, orchestrator_decision, confidence_score, eight_factor_score, portfolio_risk_pct) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"
    )
    .bind(symbol).bind(timestamp).bind(regime).bind(decision)
    .bind(confidence).bind(score).bind(risk)
    .execute(pool).await
    {
        eprintln!("Database Error: Failed to write decision memory buffer: {}", e);
    }
}

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct DecisionMemoryBufferRow {
    pub id: i64,
    pub symbol: String,
    pub timestamp: i64,
    pub regime_classification: String,
    pub orchestrator_decision: String,
    pub confidence_score: i32,
    pub eight_factor_score: i32,
    pub portfolio_risk_pct: f64,
}

pub async fn query_decision_memory_buffer(
    pool: &SqlitePool,
    symbol: &str,
    limit: i64,
) -> Vec<DecisionMemoryBufferRow> {
    sqlx::query_as(
        "SELECT id, symbol, timestamp, regime_classification, orchestrator_decision, confidence_score, eight_factor_score, portfolio_risk_pct \
         FROM decision_memory_buffer WHERE symbol = ?1 ORDER BY id DESC LIMIT ?2"
    )
    .bind(symbol).bind(limit).fetch_all(pool).await.unwrap_or_default()
}

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct CompletedTradesBufferRow {
    pub id: i64,
    pub symbol: String,
    pub direction: String,
    pub entry_price: f64,
    pub exit_price: f64,
    pub realized_pnl: f64,
    pub roi_pct: f64,
    pub execution_score: f64,
    pub primary_mistake: Option<String>,
    pub closed_at: i64,
}

pub async fn query_completed_trades_buffer(
    pool: &SqlitePool,
    symbol: &str,
    limit: i64,
) -> Vec<CompletedTradesBufferRow> {
    sqlx::query_as(
        "SELECT tth.id, tth.symbol, tth.direction, tth.entry_price, tth.exit_price, tth.realized_pnl, \
                COALESCE(tth.roi_percentage, 0.0) as roi_pct, \
                COALESCE(tlj.execution_score, 5.0) as execution_score, \
                tlj.final_analysis as primary_mistake, \
                tth.exit_timestamp as closed_at \
          FROM trade_telemetry_history tth \
         LEFT JOIN trade_learning_journal tlj ON tlj.trade_id = tth.id \
         WHERE tth.symbol = ?1 \
         ORDER BY tth.id DESC LIMIT ?2"
    )
    .bind(symbol).bind(limit).fetch_all(pool).await.unwrap_or_default()
}
