use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct KeyQuery {
    pub exchange: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AddKeyRequest {
    pub exchange: String,
    pub account_name: String,
    pub api_key: String,
    pub api_secret: String,
    #[serde(default)]
    pub passphrase: String,
    #[serde(default)]
    pub referred_uid: String,
    #[serde(default = "default_true")]
    pub is_active: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize)]
pub struct KeyMetadata {
    pub id: i64,
    pub exchange: String,
    pub account_name: String,
    pub is_active: bool,
    pub referred_uid: String,
    pub last_sync_timestamp: Option<i64>,
}

pub async fn list_keys(
    State(state): State<Arc<AppState>>,
    Query(query): Query<KeyQuery>,
) -> impl IntoResponse {
    let exchange_filter = query.exchange.unwrap_or_default();

    let rows = if exchange_filter.is_empty() {
        sqlx::query_as::<_, (i64, String, String, String, i32, String, Option<i64>)>(
            "SELECT id, exchange, account_name, api_key, is_active, referred_uid, last_sync_timestamp FROM exchange_keys ORDER BY id DESC",
        )
        .fetch_all(&state.pool)
        .await
    } else {
        sqlx::query_as::<_, (i64, String, String, String, i32, String, Option<i64>)>(
            "SELECT id, exchange, account_name, api_key, is_active, referred_uid, last_sync_timestamp FROM exchange_keys WHERE exchange = ? ORDER BY id DESC",
        )
        .bind(&exchange_filter)
        .fetch_all(&state.pool)
        .await
    };

    let rows = match rows {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": format!("Failed to query keys: {}", e)
                })),
            )
                .into_response();
        }
    };

    let keys: Vec<KeyMetadata> = rows
        .into_iter()
        .map(
            |(id, exchange, account_name, _api_key, is_active, referred_uid, last_sync)| {
                KeyMetadata {
                    id,
                    exchange,
                    account_name,
                    is_active: is_active != 0,
                    referred_uid,
                    last_sync_timestamp: last_sync,
                }
            },
        )
        .collect();

    Json(serde_json::json!({
        "keys": keys,
        "count": keys.len(),
    }))
    .into_response()
}

pub async fn add_key(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AddKeyRequest>,
) -> impl IntoResponse {
    if req.exchange.is_empty()
        || req.account_name.is_empty()
        || req.api_key.is_empty()
        || req.api_secret.is_empty()
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "exchange, account_name, api_key, and api_secret are required"
            })),
        )
            .into_response();
    }

    let valid_exchanges = ["Hyperliquid", "Bitget"];
    if !valid_exchanges.contains(&req.exchange.as_str()) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Invalid exchange. Must be Hyperliquid or Bitget"
            })),
        )
            .into_response();
    }

    let is_active_int = if req.is_active { 1 } else { 0 };

    // AUDIT-H10: the documented contract (§2.10 / 06-02 §3.5) requires
    // AES-256-GCM at rest. The old code INSERTed the raw secret —
    // contradicting `verify_encryption_or_panic` at daemon boot, which
    // asserts every existing row is encrypted. Refuse to store plaintext
    // when no master key is provisioned.
    let master_key_ok = database_storage::crypto::master_key_available();
    let encrypted_secret = if master_key_ok {
        database_storage::crypto::encrypt_field(&req.api_secret)
    } else {
        Err("EXCHANGE_SECRET_KEY is not set".to_string())
    };
    let encrypted_passphrase = if req.passphrase.is_empty() {
        Ok(String::new())
    } else if master_key_ok {
        database_storage::crypto::encrypt_field(&req.passphrase)
    } else {
        Err("EXCHANGE_SECRET_KEY is not set".to_string())
    };
    let (secret, passphrase) = match (encrypted_secret, encrypted_passphrase) {
        (Ok(s), Ok(p)) => (s, p),
        (Err(e), _) | (_, Err(e)) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({
                    "error": format!(
                        "Cannot store exchange credentials: {} — set EXCHANGE_SECRET_KEY in the environment to enable encrypted storage",
                        e
                    )
                })),
            )
                .into_response();
        }
    };

    let result = sqlx::query(
        "INSERT INTO exchange_keys (exchange, account_name, api_key, api_secret, passphrase, referred_uid, is_active, last_sync_timestamp) VALUES (?, ?, ?, ?, ?, ?, ?, NULL)",
    )
    .bind(&req.exchange)
    .bind(&req.account_name)
    .bind(&req.api_key)
    .bind(&secret)
    .bind(&passphrase)
    .bind(&req.referred_uid)
    .bind(is_active_int)
    .execute(&state.pool)
    .await;

    match result {
        Ok(r) => {
            let id = r.last_insert_rowid();
            (
                StatusCode::CREATED,
                Json(serde_json::json!({
                    "id": id,
                    "exchange": req.exchange,
                    "account_name": req.account_name,
                    "message": "Key stored successfully"
                })),
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": format!("Failed to store key: {}", e)
            })),
        )
            .into_response(),
    }
}

pub async fn delete_key(
    State(state): State<Arc<AppState>>,
    Path(key_id): Path<String>,
) -> impl IntoResponse {
    let id: i64 = match key_id.parse() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "Invalid key ID" })),
            )
                .into_response();
        }
    };

    let result = sqlx::query("DELETE FROM exchange_keys WHERE id = ?")
        .bind(id)
        .execute(&state.pool)
        .await;

    match result {
        Ok(r) if r.rows_affected() > 0 => Json(serde_json::json!({
            "id": id,
            "message": "Key deleted"
        }))
        .into_response(),
        Ok(_) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Key not found" })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": format!("Failed to delete key: {}", e)
            })),
        )
            .into_response(),
    }
}
