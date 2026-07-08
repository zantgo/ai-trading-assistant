use axum::{
    response::Redirect,
    routing::{delete, get, post},
    Router,
};
use shared::normalized::SymbolMapper;
use sqlx::SqlitePool;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

use crate::config::AppConfig;
use crate::llm::LlmClient;
use crate::workspace::Workspace;

pub mod handlers;
pub mod helpers;
pub mod math;
pub mod pipeline;
pub mod telemetry;
pub mod types;
pub mod ws;

pub use math::compute_support_resistance;
pub use pipeline::run_multi_agent_pipeline;
pub use telemetry::compile_deterministic_telemetry;
pub use types::IndicatorSnapshot;

pub struct AppState {
    pub workspace: Arc<Workspace>,
    pub config: Arc<RwLock<AppConfig>>,
    pub pool: SqlitePool,
    pub llm_client: Arc<LlmClient>,
    pub api_key_configured: Arc<AtomicBool>,
    pub symbol_mapper: Arc<SymbolMapper>,
    pub telemetry_tx: mpsc::Sender<crate::db::TelemetryMsg>,
    pub ws_url: String,
    pub bitget_ws_url: String,
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
pub struct LlmState(pub Arc<LlmClient>);

impl axum::extract::FromRef<Arc<AppState>> for LlmState {
    fn from_ref(state: &Arc<AppState>) -> Self {
        Self(state.llm_client.clone())
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
        .route("/api/config/key", post(handlers::config::serve_set_key))
        .route("/api/config/scoring-weights", post(handlers::config::serve_set_scoring_weights))
        .route(
            "/api/rules",
            get(handlers::config::serve_get_rules).post(handlers::config::serve_set_rules),
        )
        .route("/api/history", get(handlers::history::serve_history))
        .route("/api/monitor", get(handlers::monitor::serve_monitor))
        .route("/api/analyze", post(handlers::analyze::serve_analyze))
        .route("/api/chat", post(handlers::chat::serve_chat))
        .route(
            "/api/trades",
            get(handlers::trades::serve_get_trades).post(handlers::trades::serve_add_trade),
        )
        .route(
            "/api/assistant-records",
            get(handlers::assistant::serve_assistant_records),
        )
        .route(
            "/api/automated-performance",
            get(handlers::assistant::serve_automated_performance),
        )
        .route(
            "/api/paper/status",
            get(handlers::paper::serve_paper_status),
        )
        .route(
            "/api/paper/config",
            post(handlers::paper::serve_paper_config),
        )
        .route("/api/paper/reset", post(handlers::paper::serve_paper_reset))
        .route("/api/paper/order", post(handlers::paper::serve_paper_order))
        .route(
            "/api/paper/position",
            post(handlers::paper::serve_paper_position_pct),
        )
        .route(
            "/api/paper/close",
            post(handlers::paper::serve_paper_close_pct),
        )
        .route(
            "/api/paper/tp",
            post(handlers::paper::serve_paper_set_tp),
        )
        .route(
            "/api/paper/sl",
            post(handlers::paper::serve_paper_set_sl),
        )
        .route(
            "/api/paper/unrealized",
            get(handlers::paper::serve_paper_unrealized),
        )
        .route(
            "/api/paper/performance",
            get(handlers::paper::serve_paper_performance),
        )
        .route(
            "/api/paper/open-orders",
            get(handlers::paper::serve_paper_open_orders),
        )
        .route(
            "/api/paper/order/place",
            post(handlers::paper::serve_paper_place_order),
        )
        .route(
            "/api/paper/order/cancel",
            post(handlers::paper::serve_paper_cancel_order),
        )
        .route(
            "/api/paper/portion/open",
            post(handlers::paper::serve_paper_portion_open),
        )
        .route(
            "/api/paper/portion/close",
            post(handlers::paper::serve_paper_portion_close),
        )
        .route(
            "/api/paper/equity-history",
            get(handlers::paper::serve_paper_equity_history),
        )
        .route(
            "/api/paper/slot-states",
            get(handlers::paper::serve_paper_slot_states),
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
            "/api/instances/:instance_id/api-key",
            post(handlers::instances::serve_set_instance_api_key),
        )
        .route(
            "/api/instances/:instance_id/api-key",
            delete(handlers::instances::serve_delete_instance_api_key),
        )
        .route(
            "/api/instances/:instance_id/usage",
            get(handlers::instances::serve_instance_usage),
        )
        .route(
            "/api/settings/backup-api-key",
            post(handlers::config::serve_set_backup_api_key),
        )
        .route(
            "/api/settings/profile",
            get(handlers::config::serve_get_profile)
                .post(handlers::config::serve_set_profile),
        )
        .route(
            "/api/settings/max-instances",
            post(handlers::config::serve_set_max_instances),
        )
        .route(
            "/api/historical-recommendations",
            get(handlers::assistant::serve_historical_recommendations),
        )
        .route(
            "/api/instances/:instance_id/chat",
            post(handlers::chat::serve_instance_chat),
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
            "/api/exchange-keys",
            get(handlers::exchange_keys::serve_exchange_keys_list)
                .post(handlers::exchange_keys::serve_exchange_keys_add),
        )
        .route(
            "/api/exchange-keys/:id",
            delete(handlers::exchange_keys::serve_exchange_keys_delete)
                .post(handlers::exchange_keys::serve_exchange_keys_sync),
        )
        .route(
            "/api/dashboard/stats",
            get(handlers::dashboard::serve_dashboard_stats),
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
            "/api/cost-estimate",
            get(handlers::assistant::serve_cost_estimate),
        )
        .route(
            "/api/edges",
            get(handlers::edges::serve_edges_list),
        )
        .route(
            "/api/edges/save",
            post(handlers::edges::serve_edges_save),
        )
        .route(
            "/api/edges/analyze",
            post(handlers::edges::serve_edges_analyze),
        )
        .route(
            "/api/edges/:id",
            delete(handlers::edges::serve_edges_delete),
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
        map.insert("rvol".into(), NormalizedIndicatorValue::scalar(0.8, -0.5, "CONSOLIDATION_VOLUME"));
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
        map.insert(
            "rsi_divergence".into(),
            NormalizedIndicatorValue::scalar(0.5, 0.5, "POTENTIAL_BULLISH_DIVERGENCE"),
        );
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

        // bbwp < 10.0 -> COMPRESSION regime
        assert_eq!(telemetry.market_regime, "COMPRESSION");
        // Bearish elements present (bearish stack, weak ADX, negative MACD) but
        // balanced by RSI-25 undervalued + bullish divergence potential + VWAP
        // extreme discount — the registry weighted mean should not be strongly
        // bullish.
        assert!(telemetry.total_confluence_score <= 5, "expected <= 5, got {}", telemetry.total_confluence_score);
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
