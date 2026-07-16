use axum::{
    response::Redirect,
    routing::{delete, get, post},
    Router,
};
use shared::normalized::SymbolMapper;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

use crate::config::AppConfig;
use crate::workspace::Workspace;
use crate::session::{Currency, ExchangeChoice, SessionState};

pub mod handlers;
pub mod helpers;
pub mod math;
pub mod telemetry;
pub mod types;
pub mod ws;

pub use math::compute_support_resistance;
pub use telemetry::compile_deterministic_telemetry;
pub use types::IndicatorSnapshot;

pub struct AppState {
    pub workspaces: Arc<RwLock<HashMap<String, Arc<Workspace>>>>,
    pub session: SessionState,
    pub config: Arc<RwLock<AppConfig>>,
    pub pool: SqlitePool,
    pub symbol_mapper: Arc<SymbolMapper>,
    pub telemetry_tx: mpsc::Sender<crate::db::TelemetryMsg>,
    pub ws_url: String,
    pub bitget_ws_url: String,
}

impl AppState {
    pub async fn workspace_count(&self) -> usize {
        self.workspaces.read().await.len()
    }

    pub async fn get_all_instances(&self) -> Vec<Arc<Workspace>> {
        self.workspaces.read().await.values().cloned().collect()
    }

    pub async fn get_active_pair(
        &self,
        pair_key: &str,
    ) -> Option<Arc<crate::analyzer::ActivePair>> {
        self.workspaces
            .read()
            .await
            .get(pair_key)
            .map(|inst| inst.active_pair.clone())
    }

    pub async fn get_workspace_by_id(&self, id: &str) -> Option<Arc<Workspace>> {
        self.workspaces
            .read()
            .await
            .values()
            .find(|i| i.id == id)
            .cloned()
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
                    ExchangeChoice::Hyperliquid =>
                        "Hyperliquid perpetuals settle in USDC only.",
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

        let workspace_ids: Vec<String> = {
            let instances = self.workspaces.read().await;
            instances.values().map(|i| i.id.clone()).collect()
        };

        for workspace_id in &workspace_ids {
            let instances = self.workspaces.read().await;
            if let Some(instance) = instances.values().find(|i| &i.id == workspace_id) {
                instance.cancel.cancel();
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        self.workspaces.write().await.clear();

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
pub struct WsState(pub mpsc::Sender<crate::db::TelemetryMsg>);

impl axum::extract::FromRef<Arc<AppState>> for WsState {
    fn from_ref(state: &Arc<AppState>) -> Self {
        Self(state.telemetry_tx.clone())
    }
}

#[derive(Clone)]
pub struct ConfigState(pub Arc<RwLock<AppConfig>>);

impl axum::extract::FromRef<Arc<AppState>> for ConfigState {
    fn from_ref(state: &Arc<AppState>) -> Self {
        Self(state.config.clone())
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
            "/api/rules",
            get(handlers::config::serve_get_rules).post(handlers::config::serve_set_rules),
        )
        .route("/api/history", get(handlers::history::serve_history))
        .route("/api/monitor", get(handlers::monitor::serve_monitor))
        .route(
            "/api/trades",
            get(handlers::trades::serve_get_trades),
        )
        .route(
            "/api/workspaces",
            get(handlers::workspaces::serve_list_workspaces)
                .post(handlers::workspaces::serve_add_workspace),
        )
        .route(
            "/api/workspaces/:workspace_id",
            get(handlers::workspaces::serve_get_workspace_detail)
                .delete(handlers::workspaces::serve_delete_workspace),
        )
        .route(
            "/api/workspaces/by-pair/:pair_key",
            delete(handlers::workspaces::serve_delete_workspace_by_pair),
        )
        .route(
            "/api/workspaces/:workspace_id/config",
            post(handlers::workspaces::serve_update_workspace_config),
        )
        .route(
            "/api/workspaces/:workspace_id/pause",
            post(handlers::workspaces::serve_pause_workspace),
        )
        .route(
            "/api/workspaces/:workspace_id/stop",
            post(handlers::workspaces::serve_stop_workspace),
        )
        .route(
            "/api/workspaces/:workspace_id/safety/reset",
            post(handlers::workspaces::serve_reset_safety),
        )
        .route(
            "/api/workspaces/:workspace_id/intervals",
            post(handlers::workspaces::serve_workspace_intervals),
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
            "/api/risk/fee-table",
            get(handlers::profiles::serve_fee_table),
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
        .fallback_service(ServeDir::new("crates/frontend/dist"))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::types::IndicatorSnapshot;

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
        use shared::indicators::normalized::NormalizedIndicatorValue;
        use std::collections::HashMap;

        let mut map: HashMap<String, NormalizedIndicatorValue> = HashMap::new();
        map.insert("rsi".into(), NormalizedIndicatorValue::scalar(25.0, 0.8, "OVERSOLD_ACCUMULATION"));
        map.insert("bbwp".into(), NormalizedIndicatorValue::scalar(5.0, 0.0, "MAX_VOLATILITY_COMPRESSION"));
        map.insert(
            "rvol".into(),
            NormalizedIndicatorValue::with_values(
                0.8,
                0.0,
                "LOW_PARTICIPATION_VOLUME",
                [("rvol_band".to_string(), -0.5)].into_iter().collect(),
            ),
        );
        map.insert("squeeze".into(), NormalizedIndicatorValue::scalar(-0.05, -0.2, "BEARISH_MOMENTUM_EXHAUSTING"));
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
            rsi_entry.signals.push(shared::indicators::normalized::IndicatorSignal::new(
                shared::indicators::normalized::SignalKind::Divergence,
                shared::indicators::normalized::SignalDirection::Bullish,
                shared::indicators::normalized::SignalStatus::Potential,
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
