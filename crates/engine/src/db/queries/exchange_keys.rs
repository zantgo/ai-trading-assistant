use shared::normalized::Exchange;
use sqlx::SqlitePool;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ExchangeKey {
    pub id: i64,
    pub exchange: Exchange,
    pub account_name: String,
    pub api_key: String,
    pub api_secret: String,
    pub passphrase: String,
    pub referred_uid: String,
    pub is_active: bool,
    pub last_sync_timestamp: Option<i64>,
}

pub async fn exchange_keys_list(pool: &SqlitePool) -> Result<Vec<ExchangeKey>, String> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT id, exchange, account_name, api_key, api_secret, passphrase, referred_uid, is_active, last_sync_timestamp
         FROM exchange_keys ORDER BY id DESC"
    )
    .fetch_all(&*pool).await.unwrap_or_default();

    rows.iter()
        .map(|r| {
            let raw_key: String = r.get(3);
            let raw_secret: String = r.get(4);
            let raw_passphrase: String = r.get(5);
            let row_id = r.get::<i64, _>(0);
            Ok(ExchangeKey {
                id: row_id,
                exchange: Exchange::Hyperliquid,
                account_name: r.get(2),
                api_key: crate::db::crypto::decrypt_field(&raw_key).map_err(|e| {
                    format!("Failed to decrypt api_key for id={}: {}", row_id, e)
                })?,
                api_secret: crate::db::crypto::decrypt_field(&raw_secret).map_err(|e| {
                    format!("Failed to decrypt api_secret for id={}: {}", row_id, e)
                })?,
                passphrase: crate::db::crypto::decrypt_field(&raw_passphrase).map_err(|e| {
                    format!("Failed to decrypt passphrase for id={}: {}", row_id, e)
                })?,
                referred_uid: r.get(6),
                is_active: r.get::<i32, _>(7) != 0,
                last_sync_timestamp: r.get(8),
            })
        })
        .collect()
}

pub async fn exchange_keys_insert(
    pool: &SqlitePool,
    exchange: &str,
    account_name: &str,
    api_key: &str,
    api_secret: &str,
    passphrase: &str,
    referred_uid: &str,
    is_active: bool,
) -> Result<i64, String> {
    if !crate::db::crypto::master_key_available() {
        return Err("API registration rejected: EXCHANGE_SECRET_KEY is not set. Plaintext credentials storage is prohibited.".to_string());
    }
    let active_val: i32 = if is_active { 1 } else { 0 };
    let encrypted_key = crate::db::crypto::encrypt_field(api_key)?;
    let encrypted_secret = crate::db::crypto::encrypt_field(api_secret)?;
    let encrypted_passphrase = crate::db::crypto::encrypt_field(passphrase)?;

    let result = sqlx::query(
        "INSERT INTO exchange_keys (exchange, account_name, api_key, api_secret, passphrase, referred_uid, is_active) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"
    )
    .bind(exchange).bind(account_name).bind(&encrypted_key).bind(&encrypted_secret)
    .bind(&encrypted_passphrase).bind(referred_uid).bind(active_val)
    .execute(pool).await
    .map_err(|e| format!("Database error: {}", e))?;
    Ok(result.last_insert_rowid())
}

pub async fn exchange_keys_delete(pool: &SqlitePool, id: i64) -> bool {
    sqlx::query("DELETE FROM exchange_keys WHERE id = ?1")
        .bind(id)
        .execute(&*pool)
        .await
        .map(|r| r.rows_affected() > 0)
        .unwrap_or(false)
}

pub async fn exchange_keys_update_sync(pool: &SqlitePool, id: i64, timestamp: i64) {
    sqlx::query("UPDATE exchange_keys SET last_sync_timestamp = ?2 WHERE id = ?1")
        .bind(id)
        .bind(timestamp)
        .execute(&*pool)
        .await
        .ok();
}

pub async fn exchange_keys_active_count(pool: &SqlitePool) -> i64 {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM exchange_keys WHERE is_active = 1")
        .fetch_one(&*pool)
        .await
        .unwrap_or((0,));
    row.0
}
