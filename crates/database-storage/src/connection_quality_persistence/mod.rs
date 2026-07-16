//! Connection-quality persistence loop.
//!
//! The tracker lives in `network_adapters::connection_quality_tracker`
//! (no DB dep). This module subscribes to its broadcast channel and writes
//! persistence rows to `connection_quality_events` (see migration
//! `20260715120000_connection_quality.sql`).

use sqlx::SqlitePool;
use tokio_util::sync::CancellationToken;

/// Spawn the persistence loop. Returns immediately; the loop exits when
/// the cancel token fires or the receiver channel closes.
///
/// `window_secs` is the bucket length for the rolling event aggregation.
/// Callers typically pass the same value they pass to the tracker's
/// `run_persistence_loop`.
pub fn spawn_persistence_loop(
    pool: SqlitePool,
    cancel: CancellationToken,
    mut receiver: tokio::sync::broadcast::Receiver<QualityPersistenceEvent>,
) {
    tokio::spawn(async move {
        loop {
            tokio::select! {
                biased;
                _ = cancel.cancelled() => break,
                evt = receiver.recv() => {
                    match evt {
                        Ok(evt) => {
                            if let Err(e) = persist_event(&pool, &evt).await {
                                eprintln!("connection-quality persistence error: {}", e);
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                            eprintln!("connection-quality persistence lagged {} events", n);
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    }
                }
            }
        }
    });
}

async fn persist_event(pool: &SqlitePool, evt: &QualityPersistenceEvent) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT OR REPLACE INTO connection_quality_events
            (window, window_start_ms, window_end_ms, uptime_pct, disconnect_count,
             avg_reconnect_ms, total_data_loss_secs, reconstructed_candles, score, measured_at_ms)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&evt.window)
    .bind(evt.window_start_ms as i64)
    .bind(evt.window_end_ms as i64)
    .bind(evt.uptime_pct)
    .bind(evt.disconnect_count as i64)
    .bind(evt.avg_reconnect_ms)
    .bind(evt.total_data_loss_secs as i64)
    .bind(evt.reconstructed_candles as i64)
    .bind(evt.score)
    .bind(evt.measured_at_ms as i64)
    .execute(pool)
    .await?;
    Ok(())
}

/// Event shape broadcast by the tracker. Kept in this crate so it lives
/// next to the schema it serializes into.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct QualityPersistenceEvent {
    pub window: String,
    pub window_start_ms: u64,
    pub window_end_ms: u64,
    pub uptime_pct: f64,
    pub disconnect_count: u32,
    pub avg_reconnect_ms: f64,
    pub total_data_loss_secs: u64,
    pub reconstructed_candles: u32,
    pub score: f64,
    pub measured_at_ms: u64,
}
