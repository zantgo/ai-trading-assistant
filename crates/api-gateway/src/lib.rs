use axum::{
    response::Redirect,
    routing::{delete, get, post},
    Router,
};
use core_domain::normalized::SymbolMapper;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

use config_models::PlatformConfig;
use network_adapters::connection_quality_tracker::ConnectionQualityTracker;
use portfolio_supervisor::session::{Currency, ExchangeChoice, SessionState};
use portfolio_supervisor::instance::Instance;
use portfolio_supervisor::workspace_state::WorkspaceState;

pub mod handlers;
pub mod helpers;
pub mod math;
pub mod telemetry;
pub mod types;
pub mod ws;

pub use math::compute_support_resistance;
pub use telemetry::compile_deterministic_telemetry;
pub use types::IndicatorSnapshot;

use portfolio_supervisor::registry_context::RegistryContext;

pub struct AppState {
    /// The single workspace state (workspace config + live `Arc<Instance>`
    /// map). The binary supports one workspace per deployment; the field is
    /// named `workspace` not `instances` to make the hierarchy explicit
    /// (PLATFORM > WORKSPACE > INSTANCE — see
    /// `docs/conceptual-foundations/01-07-data-model-hierarchy.md`).
    pub workspace: WorkspaceState,
    pub session: Arc<SessionState>,
    /// Platform-level config (exchange endpoints, clock monitor). Separate
    /// from the workspace.
    pub platform: Arc<RwLock<PlatformConfig>>,
    pub pool: SqlitePool,
    pub symbol_mapper: Arc<SymbolMapper>,
    pub telemetry_tx: mpsc::Sender<database_storage::TelemetryMsg>,
    pub connection_quality: Arc<ConnectionQualityTracker>,
    pub ws_url: String,
    pub bitget_ws_url: String,
}

impl AppState {
    /// Build the `RegistryContext` view that portfolio-supervisor's registry
    /// functions take. Extracted here to keep the registry decoupled from
    /// `api-gateway`'s `AppState` type.
    pub fn registry_context(&self) -> RegistryContext {
        RegistryContext {
            workspace: self.workspace.clone(),
            session: self.session.clone(),
            platform: self.platform.clone(),
            pool: self.pool.clone(),
            symbol_mapper: self.symbol_mapper.clone(),
            telemetry_tx: self.telemetry_tx.clone(),
            ws_url: self.ws_url.clone(),
            bitget_ws_url: self.bitget_ws_url.clone(),
        }
    }

    pub async fn instance_count(&self) -> usize {
        self.workspace.len().await
    }

    pub async fn get_all_instances(&self) -> Vec<Arc<Instance>> {
        self.workspace.list().await
    }

    pub async fn get_active_pair(
        &self,
        pair_key: &str,
    ) -> Option<Arc<market_analyzer::analyzer::ActivePair>> {
        self.workspace
            .get(pair_key)
            .await
            .map(|inst| inst.active_pair.clone())
    }

    pub async fn get_instance_by_id(&self, id: &str) -> Option<Arc<Instance>> {
        for inst in self.workspace.list().await {
            if inst.id == id {
                return Some(inst);
            }
        }
        None
    }

    pub async fn init_session(
        &self,
        currency: Currency,
        exchange: ExchangeChoice,
    ) -> Result<(), String> {
        if exchange != ExchangeChoice::Hyperliquid && exchange != ExchangeChoice::Bitget {
            return Err("Unsupported exchange selected.".to_string());
        }
        if !exchange.supports_currency(&currency) {
            return Err(format!(
                "{} does not support {} settlement. {}",
                exchange.as_str(),
                currency.as_str(),
                match exchange {
                    ExchangeChoice::Hyperliquid => "Hyperliquid perpetuals settle in USDC only.",
                    ExchangeChoice::Bitget => "Select USDT or USDC.",
                }
            ));
        }

        *self.session.base_currency.write().await = Some(currency.clone());
        *self.session.exchange.write().await = Some(exchange.clone());
        self.session
            .active
            .store(true, std::sync::atomic::Ordering::Relaxed);

        println!(
            "✅ Session initialized: {} on {}",
            currency.as_str(),
            exchange.as_str(),
        );
        Ok(())
    }

    pub async fn quit_session(&self) -> Result<(), String> {
        println!("🛑 Initiating graceful shutdown of all instances...");

        let instance_ids: Vec<String> = {
            self.workspace
                .list()
                .await
                .iter()
                .map(|i| i.id.clone())
                .collect()
        };

        for instance_id in &instance_ids {
            if let Some(instance) = self.workspace.get(instance_id).await {
                instance.cancel.cancel();
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        let mut ws = self.workspace.config().await;
        ws.instances.clear();
        self.workspace.set_config(ws).await;

        self.session
            .active
            .store(false, std::sync::atomic::Ordering::Relaxed);
        *self.session.base_currency.write().await = None;
        *self.session.exchange.write().await = None;

        println!("✅ Session terminated. All instances stopped.");
        Ok(())
    }
}

// ── Stratified state types for Axum FromRef ──────────────────────

#[derive(Clone)]
pub struct DbState(pub SqlitePool);

impl axum::extract::FromRef<Arc<AppState>> for DbState {
    fn from_ref(state: &Arc<AppState>) -> Self {
        Self(state.pool.clone())
    }
}

#[derive(Clone)]
pub struct WsState(pub mpsc::Sender<database_storage::TelemetryMsg>);

impl axum::extract::FromRef<Arc<AppState>> for WsState {
    fn from_ref(state: &Arc<AppState>) -> Self {
        Self(state.telemetry_tx.clone())
    }
}

#[derive(Clone)]
pub struct ConfigState(pub Arc<RwLock<PlatformConfig>>);

impl axum::extract::FromRef<Arc<AppState>> for ConfigState {
    fn from_ref(state: &Arc<AppState>) -> Self {
        Self(state.platform.clone())
    }
}

pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route(
            "/api/session/status",
            get(handlers::session::serve_session_status),
        )
        .route(
            "/api/session/init",
            post(handlers::session::serve_session_init),
        )
        .route(
            "/api/session/quit",
            post(handlers::session::serve_session_quit),
        )
        .route(
            "/api/config",
            get(handlers::config::serve_config).post(handlers::config::update_config),
        )
        .route(
            "/api/workspace/toml",
            get(handlers::config::serve_workspace_toml).post(handlers::config::serve_workspace_toml_import),
        )
        .route(
            "/api/rules",
            get(handlers::config::serve_get_rules).post(handlers::config::serve_set_rules),
        )
        .route("/api/history", get(handlers::history::serve_history))
        .route(
            "/api/connection-quality",
            get(handlers::connection_quality::get_connection_quality),
        )
        .route("/api/monitor", get(handlers::monitor::serve_monitor))
        .route(
            "/api/trades",
            get(handlers::trades::serve_get_trades).post(handlers::trades::serve_add_trade),
        )
        .route(
            "/api/trade-ledger",
            get(handlers::trades::serve_trade_ledger),
        )
        .route(
            "/api/trade-journal",
            get(handlers::trades::serve_trade_journal),
        )
        .route(
            "/api/trade-journal/:id/notes",
            post(handlers::trades::serve_update_journal_notes),
        )
        .route(
            "/api/trade-journal/export/csv",
            get(handlers::trades::serve_export_journal_csv),
        )
        .route(
            "/api/trade-journal/export/json",
            get(handlers::trades::serve_export_journal_json),
        )
        .route(
            "/api/trades/telemetry",
            post(handlers::trades::serve_trade_telemetry_add),
        )
        .route(
            "/api/instances",
            get(handlers::instances::serve_list_instances)
                .post(handlers::instances::serve_add_instance),
        )
        .route(
            "/api/instances/:instance_id",
            get(handlers::instances::serve_get_instance_detail)
                .delete(handlers::instances::serve_delete_instance),
        )
        .route(
            "/api/instances/by-pair/:pair_key",
            delete(handlers::instances::serve_delete_instance_by_pair),
        )
        .route(
            "/api/instances/:instance_id/config",
            post(handlers::instances::serve_update_instance_config),
        )
        .route(
            "/api/instances/:instance_id/pause",
            post(handlers::instances::serve_pause_instance),
        )
        .route(
            "/api/instances/:instance_id/stop",
            post(handlers::instances::serve_stop_instance),
        )
        .route(
            "/api/instances/:instance_id/safety/reset",
            post(handlers::instances::serve_reset_safety),
        )
        .route(
            "/api/instances/:instance_id/manual/open",
            post(handlers::instances::serve_instance_manual_open),
        )
        .route(
            "/api/instances/:instance_id/manual/close",
            post(handlers::instances::serve_instance_manual_close),
        )
        .route(
            "/api/instances/:instance_id/intervals",
            post(handlers::instances::serve_instance_intervals),
        )
        .route(
            "/api/decision-profiles",
            get(handlers::profiles::serve_decision_profiles_list)
                .post(handlers::profiles::serve_decision_profile_create),
        )
        .route(
            "/api/decision-profiles/:id",
            delete(handlers::profiles::serve_decision_profile_delete)
                .post(handlers::profiles::serve_decision_profile_update),
        )
        .route(
            "/api/decision-profiles/:id/evaluate",
            post(handlers::profiles::serve_decision_evaluate),
        )
        .route(
            "/api/decision-profiles/:id/indicators",
            post(handlers::profiles::serve_profile_indicator_add),
        )
        .route(
            "/api/decision-profiles/:id/indicators/:iid",
            post(handlers::profiles::serve_profile_indicator_update)
                .delete(handlers::profiles::serve_profile_indicator_delete),
        )
        .route(
            "/api/risk-profiles",
            get(handlers::profiles::serve_risk_profiles_list)
                .post(handlers::profiles::serve_risk_profile_create),
        )
        .route(
            "/api/risk-profiles/:id",
            delete(handlers::profiles::serve_risk_profile_delete)
                .post(handlers::profiles::serve_risk_profile_update),
        )
        .route(
            "/api/risk/calculate",
            post(handlers::profiles::serve_risk_calculate),
        )
        .route(
            "/api/risk/fee-table",
            get(handlers::profiles::serve_fee_table),
        )
        .route(
            "/api/risk/commission-projection",
            post(handlers::profiles::serve_commission_projection),
        )
        .route(
            "/api/dashboard/stats",
            get(handlers::dashboard::serve_dashboard_stats),
        )
        .route(
            "/api/system/status",
            get(handlers::system::serve_system_status),
        )
        .route(
            "/api/system/observability",
            get(handlers::system::serve_observability_buffers),
        )
        .route("/ws", get(ws::ws_handler))
        .route(
            "/favicon.ico",
            get(|| async { Redirect::to("/favicon.svg") }),
        )
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .fallback_service(ServeDir::new("ui/dist"))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::IndicatorSnapshot;

    #[test]
    fn test_support_resistance_calculations() {
        let prices = vec![
            3110.0, 3135.0, 3105.0, 3140.0, 3100.0, 3150.0, 3115.0, 3145.0, 3120.0, 3130.0,
        ];
        let current_price = 3125.0;

        let (support, resistance) = compute_support_resistance(&prices, current_price);

        for s in &support {
            let s_val: f64 = s.parse().unwrap();
            assert!(
                s_val < current_price,
                "Support {} should be below current price",
                s_val
            );
        }

        for r in &resistance {
            let r_val: f64 = r.parse().unwrap();
            assert!(
                r_val > current_price,
                "Resistance {} should be above current price",
                r_val
            );
        }

        assert!(support.len() <= 3);
        assert!(resistance.len() <= 3);
    }

    #[test]
    fn test_compile_deterministic_telemetry() {
        use market_analyzer::indicators::normalized::NormalizedIndicatorValue;
        use std::collections::HashMap;

        let mut map: HashMap<String, NormalizedIndicatorValue> = HashMap::new();
        map.insert(
            "rsi".into(),
            NormalizedIndicatorValue::scalar(25.0, 0.8, "OVERSOLD_ACCUMULATION"),
        );
        map.insert(
            "bbwp".into(),
            NormalizedIndicatorValue::scalar(5.0, 0.0, "MAX_VOLATILITY_COMPRESSION"),
        );
        // RVOL fallback: non-directional gate (normalized=0.0, band in values.rvol_band).
        map.insert(
            "rvol".into(),
            NormalizedIndicatorValue::with_values(
                0.8,
                0.0,
                "LOW_PARTICIPATION_VOLUME",
                [("rvol_band".to_string(), -0.5)].into_iter().collect(),
            ),
        );
        map.insert(
            "squeeze".into(),
            NormalizedIndicatorValue::scalar(-0.05, -0.2, "BEARISH_MOMENTUM_EXHAUSTING"),
        );
        map.insert("macd".into(), {
            let mut v = HashMap::new();
            v.insert("line".to_string(), -0.5);
            v.insert("signal".to_string(), -0.3);
            v.insert("histogram".to_string(), -0.2);
            NormalizedIndicatorValue::with_values(-0.2, -0.5, "BEARISH_MOMENTUM_EXPANDING", v)
        });
        map.insert("adx".into(), {
            let mut v = HashMap::new();
            v.insert("adx".to_string(), 15.0);
            v.insert("plus_di".to_string(), 12.0);
            v.insert("minus_di".to_string(), 18.0);
            NormalizedIndicatorValue::with_values(15.0, 0.0, "TRENDLESS_CONGESTION", v)
        });
        map.insert("ema_stack".into(), {
            let mut v = HashMap::new();
            v.insert("long".to_string(), 3200.0);
            NormalizedIndicatorValue::with_values(3125.0, -1.0, "ESTABLISHED_BEARISH_STACK", v)
        });
        map.insert("vwap".into(), {
            let mut v = HashMap::new();
            v.insert("vwap".to_string(), 3130.0);
            NormalizedIndicatorValue::with_values(3130.0, 0.8, "EXTREME_DISCOUNT_REVERSION_ZONE", v)
        });
        // Push a Divergence signal onto the RSI entry (divergence lives on parent).
        if let Some(rsi_entry) = map.get_mut("rsi") {
            rsi_entry
                .signals
                .push(market_analyzer::indicators::normalized::IndicatorSignal::new(
                    market_analyzer::indicators::normalized::SignalKind::Divergence,
                    market_analyzer::indicators::normalized::SignalDirection::Bullish,
                    market_analyzer::indicators::normalized::SignalStatus::Potential,
                    "POTENTIAL_BULLISH_DIVERGENCE",
                ));
        }
        map.insert("atr".into(), {
            let mut v = HashMap::new();
            v.insert("atr_14".to_string(), 1.5);
            NormalizedIndicatorValue::with_values(1.5, 0.0, "ATR_RAW", v)
        });
        let indicators = IndicatorSnapshot::new(map, Some(3125.0));

        let support_levels: Vec<String> = vec!["3100.00".to_string(), "3050.00".to_string()];
        let resistance_levels: Vec<String> = vec!["3150.00".to_string(), "3200.00".to_string()];

        let telemetry = telemetry::compile_deterministic_telemetry(
            &indicators,
            &support_levels,
            &resistance_levels,
        );

        assert_eq!(telemetry.market_regime, "COMPRESSION");
        assert!(telemetry.total_confluence_score < 0);
        assert_eq!(telemetry.rvol, 0.8);
        assert_eq!(telemetry.adx_value, 15.0);
        assert_eq!(telemetry.adx_regime, "congestion");
        assert!((telemetry.bbwp_percentile - 5.0).abs() < 0.001);
        assert!(!telemetry.squeeze_on);
        assert_eq!(telemetry.vwap_bias, "discount");
        assert_eq!(telemetry.rsi_divergence_state, "potential_bullish");
        assert_eq!(telemetry.macd_divergence_state, "none");
        assert_eq!(telemetry.macd_crossover_state, "none");
        assert_eq!(telemetry.squeeze_release_state, "none");
        assert_eq!(telemetry.support_levels.len(), 2);
        assert_eq!(telemetry.resistance_levels.len(), 2);
    }
}
