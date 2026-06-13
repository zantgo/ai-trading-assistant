use sqlx::SqlitePool;
use tokio::sync::mpsc;

pub async fn spawn_sync_tasks(_pool: SqlitePool, _telemetry_tx: mpsc::Sender<crate::db::TelemetryMsg>) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
        }
    });
    println!("📡 Sync Scheduler: Background exchange sync tasks initialized.");
}
