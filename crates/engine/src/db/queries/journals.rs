use sqlx::SqlitePool;

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct TradeJournalRecord {
    pub id: i64,
    pub trade_id: i64,
    pub entry_date: String,
    pub exit_date: String,
    pub asset: String,
    pub direction: String,
    pub entry_reason: String,
    pub roe_percentage: f64,
    pub final_analysis: String,
    pub execution_score: f64,
    pub human_notes: String,
    pub created_at: String,
    pub symbol: String,
    pub realized_pnl: f64,
    pub roi_percentage: f64,
}

pub async fn insert_trade_journal(
    pool: &SqlitePool,
    trade_id: i64,
    entry_date: &str,
    exit_date: &str,
    asset: &str,
    direction: &str,
    entry_reason: &str,
    roe_percentage: f64,
    final_analysis: &str,
    execution_score: f64,
) -> i64 {
    match sqlx::query(
        "INSERT INTO trade_learning_journal (trade_id, entry_date, exit_date, asset, direction, entry_reason, roe_percentage, final_analysis, execution_score)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"
    )
    .bind(trade_id).bind(entry_date).bind(exit_date)
    .bind(asset).bind(direction).bind(entry_reason)
    .bind(roe_percentage).bind(final_analysis).bind(execution_score)
    .execute(pool)
    .await
    {
        Ok(r) => r.last_insert_rowid(),
        Err(e) => {
            eprintln!("DB: Failed to insert trade journal: {}", e);
            0
        }
    }
}

pub async fn query_trade_journal(pool: &SqlitePool, limit: u32) -> Vec<TradeJournalRecord> {
    sqlx::query_as::<_, TradeJournalRecord>(
        "SELECT j.id, j.trade_id, j.entry_date, j.exit_date, j.asset, j.direction,
                j.entry_reason, j.roe_percentage, j.final_analysis, j.execution_score,
                j.human_notes, j.created_at,
                COALESCE(t.symbol, '') AS symbol,
                COALESCE(t.realized_pnl, 0.0) AS realized_pnl,
                COALESCE(t.roi_percentage, 0.0) AS roi_percentage
         FROM trade_learning_journal j
         LEFT JOIN trade_telemetry_history t ON j.trade_id = t.id
         ORDER BY j.id DESC
         LIMIT ?1",
    )
    .bind(limit as i64)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
}

pub async fn update_journal_notes(
    pool: &SqlitePool,
    id: i64,
    human_notes: &str,
    execution_score: f64,
) -> bool {
    sqlx::query(
        "UPDATE trade_learning_journal SET human_notes = ?2, execution_score = ?3 WHERE id = ?1",
    )
    .bind(id)
    .bind(human_notes)
    .bind(execution_score)
    .execute(pool)
    .await
    .map(|r| r.rows_affected() > 0)
    .unwrap_or(false)
}

pub async fn query_recent_journal_for_context(
    pool: &SqlitePool,
    symbol: &str,
    limit: u32,
) -> String {
    let rows = query_trade_journal(pool, limit).await;
    if rows.is_empty() {
        return String::new();
    }

    let filtered: Vec<&TradeJournalRecord> = if symbol.is_empty() {
        rows.iter().collect()
    } else {
        rows.iter()
            .filter(|r| r.symbol.contains(symbol) || r.symbol.is_empty())
            .collect()
    };

    if filtered.is_empty() {
        return String::new();
    }

    let mut table = String::from(
        "RECENT TRADE EXECUTION HISTORY & LEARNING LESSONS (Last trades):\n\
         | Asset | Direction | ROI% | Execution Score | Mistakes / Success Factors Identified |\n\
         |---|---|---|---|---|\n",
    );

    for r in filtered.iter().take(limit as usize) {
        let mistakes = if r.final_analysis.len() > 120 {
            format!("{}...", &r.final_analysis[..120])
        } else {
            r.final_analysis.clone()
        };
        table.push_str(&format!(
            "| {} | {} | {:.1}% | {:.1} | {} |\n",
            r.asset, r.direction, r.roe_percentage, r.execution_score, mistakes
        ));
    }

    table
}
