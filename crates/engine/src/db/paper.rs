//! # Paper Trading Database (Placeholder)
//!
//! Database operations for paper trading positions.

use serde::{Deserialize, Serialize};

/// Active paper trading position.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperPosition {
    pub id: i64,
    pub symbol: String,
    pub direction: String,
    pub entry_price: f64,
    pub size: f64,
    pub final_invalidation_level: Option<f64>,
    pub status: String,
    pub created_at: String,
}

/// Query the currently active paper trading position for a symbol.
pub async fn paper_get_active_position(_symbol: &str) -> Option<PaperPosition> {
    None
}

/// Queries sub-module for paper trading database operations.
pub mod queries {
    use super::PaperPosition;
    use sqlx::SqlitePool;

    /// Query the active paper position from the database.
    pub async fn paper_get_active_position(
        _pool: &SqlitePool,
        _symbol: &str,
    ) -> Option<PaperPosition> {
        None
    }
}
