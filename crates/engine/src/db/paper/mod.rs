use sqlx::SqlitePool;

pub mod operations;
pub mod queries;

pub use operations::*;
pub use queries::*;

pub async fn run_in_transaction<F, Fut, T>(pool: &sqlx::SqlitePool, f: F) -> Result<T, sqlx::Error>
where
    F: FnOnce(&mut sqlx::Transaction<'_, sqlx::Sqlite>) -> Fut,
    Fut: std::future::Future<Output = Result<T, sqlx::Error>>,
{
    let mut tx = pool.begin().await?;
    match f(&mut tx).await {
        Ok(val) => { tx.commit().await?; Ok(val) }
        Err(err) => { tx.rollback().await?; Err(err) }
    }
}
