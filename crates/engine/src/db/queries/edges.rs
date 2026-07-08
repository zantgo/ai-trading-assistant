use sqlx::SqlitePool;

use crate::edges::types::{CachedAnalyticsRow, SavedEdge, SavedEdgeRow};

pub async fn edges_list(pool: &SqlitePool, pair_key: &str) -> Vec<SavedEdge> {
    let rows: Vec<SavedEdgeRow> = sqlx::query_as(
        "SELECT id, name, pair_key, description, config_payload, created_at, creator_name
         FROM saved_edges
         WHERE pair_key = ?1
         ORDER BY created_at DESC",
    )
    .bind(pair_key)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    rows.into_iter()
        .map(|row| {
            let config = serde_json::from_str(&row.config_payload).unwrap_or_default();
            SavedEdge {
                id: row.id,
                name: row.name,
                pair_key: row.pair_key,
                description: row.description,
                config,
                created_at: row.created_at,
                creator_name: row.creator_name,
            }
        })
        .collect()
}

pub async fn edges_insert(
    pool: &SqlitePool,
    name: &str,
    pair_key: &str,
    description: &str,
    config_json: &str,
    creator_name: Option<&str>,
) -> i64 {
    let result = sqlx::query(
        "INSERT INTO saved_edges (name, pair_key, description, config_payload, creator_name)
         VALUES (?1, ?2, ?3, ?4, ?5)",
    )
    .bind(name)
    .bind(pair_key)
    .bind(if description.is_empty() { None } else { Some(description) })
    .bind(config_json)
    .bind(creator_name.filter(|n| !n.is_empty()))
    .execute(pool)
    .await;

    match result {
        Ok(r) => r.last_insert_rowid(),
        Err(e) => {
            eprintln!("Database Error: Failed to insert edge: {}", e);
            -1
        }
    }
}

pub async fn edges_get(pool: &SqlitePool, id: i64) -> Result<SavedEdgeRow, String> {
    let row: Option<SavedEdgeRow> = sqlx::query_as(
        "SELECT id, name, pair_key, description, config_payload, created_at, creator_name
         FROM saved_edges WHERE id = ?1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Failed to query edge: {}", e))?;

    row.ok_or_else(|| format!("Edge with id {} not found", id))
}

pub async fn edges_delete(pool: &SqlitePool, id: i64) -> bool {
    let result = sqlx::query("DELETE FROM saved_edges WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await;

    match result {
        Ok(r) => r.rows_affected() > 0,
        Err(e) => {
            eprintln!("Database Error: Failed to delete edge: {}", e);
            false
        }
    }
}

pub async fn edge_analytics_cache_get(
    pool: &SqlitePool,
    edge_id: i64,
) -> Option<CachedAnalyticsRow> {
    sqlx::query_as(
        "SELECT edge_id, historical_metrics, monte_carlo_paths, bootstrap_results, generated_at
         FROM edge_analytics_cache
         WHERE edge_id = ?1",
    )
    .bind(edge_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
}

pub async fn edge_analytics_cache_upsert(
    pool: &SqlitePool,
    edge_id: i64,
    historical_metrics: &str,
    monte_carlo_paths: &str,
    bootstrap_results: &str,
) -> Result<(), String> {
    sqlx::query(
        "INSERT INTO edge_analytics_cache (edge_id, historical_metrics, monte_carlo_paths, bootstrap_results, generated_at)
         VALUES (?1, ?2, ?3, ?4, datetime('now'))
         ON CONFLICT(edge_id) DO UPDATE SET
             historical_metrics = excluded.historical_metrics,
             monte_carlo_paths = excluded.monte_carlo_paths,
             bootstrap_results = excluded.bootstrap_results,
             generated_at = excluded.generated_at",
    )
    .bind(edge_id)
    .bind(historical_metrics)
    .bind(monte_carlo_paths)
    .bind(bootstrap_results)
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to upsert analytics cache: {}", e))?;

    Ok(())
}
