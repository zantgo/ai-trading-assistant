use shared::TriggerType;
use sqlx::SqlitePool;

pub async fn insert_master_placeholder(
    pool: &SqlitePool,
    position: &str,
    entry_price: &str,
    price_at_analysis: &str,
    symbol: &str,
    trigger_type: TriggerType,
) -> i64 {
    let trigger_str = trigger_type.to_string();
    match sqlx::query(
        "INSERT INTO master_assistant_records (
            position, entry_price, price_at_analysis, general_trend,
            support_levels, resistance_levels,
            indicator_synthesis_summary, indicator_synthesis_evaluation,
            recommended_action, recommendation_rationale, symbol, trigger_type
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
    )
    .bind(position)
    .bind(entry_price)
    .bind(price_at_analysis)
    .bind("PENDING")
    .bind("PENDING")
    .bind("PENDING")
    .bind("PENDING")
    .bind("PENDING")
    .bind("PENDING")
    .bind("PENDING")
    .bind(symbol)
    .bind(&trigger_str)
    .execute(&*pool)
    .await
    {
        Ok(result) => result.last_insert_rowid(),
        Err(e) => {
            eprintln!("Database Error: Failed to insert master placeholder: {}", e);
            0
        }
    }
}

pub async fn insert_individual_log_internal(
    pool: &SqlitePool,
    master_record_id: i64,
    indicator_name: &str,
    signal: &str,
    reason: &str,
    timeframe_secs: u64,
) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    if let Err(e) = sqlx::query(
        "INSERT INTO individual_indicator_logs (
            master_record_id, indicator_name, signal, reason, timeframe_secs, timestamp
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )
    .bind(master_record_id)
    .bind(indicator_name)
    .bind(signal)
    .bind(reason)
    .bind(timeframe_secs as i64)
    .bind(now)
    .execute(&*pool)
    .await
    {
        eprintln!(
            "Database Error: Failed to save individual indicator log for {}: {}",
            indicator_name, e
        );
    }
}

pub async fn update_master_record_internal(
    pool: &SqlitePool,
    master_id: i64,
    general_trend: &str,
    support_levels: &str,
    resistance_levels: &str,
    indicator_synthesis_summary: &str,
    indicator_synthesis_evaluation: &str,
    recommended_action: &str,
    recommendation_rationale: &str,
    score_points: Option<i32>,
    signals_json: Option<String>,
) {
    if let Err(e) = sqlx::query(
        "UPDATE master_assistant_records SET
            general_trend = ?2,
            support_levels = ?3,
            resistance_levels = ?4,
            indicator_synthesis_summary = ?5,
            indicator_synthesis_evaluation = ?6,
            recommended_action = ?7,
            recommendation_rationale = ?8
        WHERE id = ?1",
    )
    .bind(master_id)
    .bind(general_trend)
    .bind(support_levels)
    .bind(resistance_levels)
    .bind(indicator_synthesis_summary)
    .bind(indicator_synthesis_evaluation)
    .bind(recommended_action)
    .bind(recommendation_rationale)
    .execute(&*pool)
    .await
    {
        eprintln!(
            "Database Error: Failed to update master record {}: {}",
            master_id, e
        );
    }

    if let (Some(points), Some(signals)) = (score_points, signals_json) {
        sqlx::query(
            "UPDATE master_assistant_records SET score_points = ?2, signals_json = ?3 WHERE id = ?1"
        )
        .bind(master_id)
        .bind(points)
        .bind(&signals)
        .execute(&*pool)
        .await
        .ok();
    }
}

#[derive(sqlx::FromRow)]
pub struct MasterRecord {
    pub id: i64,
    pub created_at: String,
    pub position: String,
    pub entry_price: Option<String>,
    pub price_at_analysis: String,
    pub general_trend: String,
    pub support_levels: String,
    pub resistance_levels: String,
    pub indicator_synthesis_summary: String,
    #[allow(dead_code)]
    pub indicator_synthesis_evaluation: String,
    pub recommended_action: String,
    pub recommendation_rationale: String,
    pub symbol: String,
    pub trigger_type: String,
}

pub async fn query_master_records(pool: &SqlitePool, limit: u32) -> Vec<MasterRecord> {
    sqlx::query_as::<_, MasterRecord>(
        "SELECT id, created_at, position, entry_price, price_at_analysis,
                general_trend, support_levels, resistance_levels,
                indicator_synthesis_summary, indicator_synthesis_evaluation,
                recommended_action, recommendation_rationale, symbol, trigger_type
         FROM master_assistant_records
         WHERE general_trend != 'PENDING'
         ORDER BY id DESC
         LIMIT ?1",
    )
    .bind(limit as i64)
    .fetch_all(&*pool)
    .await
    .unwrap_or_else(|e| {
        eprintln!("Database Error: Failed to query master records: {}", e);
        vec![]
    })
}

pub async fn query_master_records_by_trigger(
    pool: &SqlitePool,
    trigger_type: &str,
    limit: u32,
) -> Vec<MasterRecord> {
    sqlx::query_as::<_, MasterRecord>(
        "SELECT id, created_at, position, entry_price, price_at_analysis,
                general_trend, support_levels, resistance_levels,
                indicator_synthesis_summary, indicator_synthesis_evaluation,
                recommended_action, recommendation_rationale, symbol, trigger_type
         FROM master_assistant_records
         WHERE general_trend != 'PENDING' AND trigger_type = ?1
         ORDER BY id DESC
         LIMIT ?2",
    )
    .bind(trigger_type)
    .bind(limit as i64)
    .fetch_all(&*pool)
    .await
    .unwrap_or_else(|e| {
        eprintln!(
            "Database Error: Failed to query master records by trigger: {}",
            e
        );
        vec![]
    })
}

pub async fn query_master_action_by_id(pool: &SqlitePool, master_id: i64) -> Option<String> {
    sqlx::query_as::<_, (String,)>(
        "SELECT recommended_action FROM master_assistant_records WHERE id = ?1",
    )
    .bind(master_id)
    .fetch_optional(&*pool)
    .await
    .ok()
    .flatten()
    .map(|(action,)| action)
}
