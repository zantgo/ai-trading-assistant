//! Shared context type for portfolio-supervisor's instance registry.
//!
//! Lives in `portfolio-supervisor` (not in `api-gateway`) so the registry
//! functions can take it without creating a circular `api-gateway` dep.
//! `api-gateway`'s HTTP handlers build this from their `AppState`.

use std::sync::Arc;
use tokio::sync::RwLock;

use config_models::PlatformConfig;
use core_domain::normalized::SymbolMapper;
use database_storage::TelemetryMsg;
use tokio::sync::mpsc;

use crate::session::SessionState;
use crate::workspace_state::WorkspaceState;

#[derive(Clone)]
pub struct RegistryContext {
    /// Live workspace state (workspace config + active `Arc<Instance>` map).
    pub workspace: WorkspaceState,
    pub session: Arc<SessionState>,
    /// Platform-level config (server, DB, exchanges, clock). Held by the
    /// daemon and passed through unchanged. The registry does not mutate it.
    pub platform: Arc<RwLock<PlatformConfig>>,
    pub pool: sqlx::SqlitePool,
    pub symbol_mapper: Arc<SymbolMapper>,
    pub telemetry_tx: mpsc::Sender<TelemetryMsg>,
    pub ws_url: String,
    pub bitget_ws_url: String,
}