//! Shared context type for portfolio-supervisor's instance registry.
//!
//! Lives in `portfolio-supervisor` (not in `api-gateway`) so the registry
//! functions can take it without creating a circular `api-gateway` dep.
//! `api-gateway`'s HTTP handlers build this from their `AppState`.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use config_models::AppConfig;
use core_domain::normalized::SymbolMapper;
use database_storage::TelemetryMsg;
use tokio::sync::mpsc;

use crate::instance::Instance;
use crate::session::SessionState;

#[derive(Clone)]
pub struct RegistryContext {
    pub instances: Arc<RwLock<HashMap<String, Arc<Instance>>>>,
    pub session: Arc<SessionState>,
    pub config: Arc<RwLock<AppConfig>>,
    pub pool: sqlx::SqlitePool,
    pub symbol_mapper: Arc<SymbolMapper>,
    pub telemetry_tx: mpsc::Sender<TelemetryMsg>,
    pub ws_url: String,
    pub bitget_ws_url: String,
}
