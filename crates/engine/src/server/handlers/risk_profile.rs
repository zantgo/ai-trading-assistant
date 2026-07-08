use crate::server::helpers::{default_pair_key, get_active_pair};
use crate::server::AppState;
use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct RiskProfileQuery {
    #[serde(default)]
    pub pair_key: Option<String>,
}

#[derive(Debug, Serialize)]
struct RrBlockDto {
    block_index: i64,
    wins: i64,
    losses: i64,
    win_rate_estimate: f64,
    breakeven_ratio: f64,
    recommended_ratio: f64,
    confidence: f64,
    net_block_pnl: f64,
}

#[derive(Debug, Serialize)]
struct RiskProfileResponse {
    pair_key: String,
    symbol: String,
    available: bool,
    message: Option<String>,
    profile: Option<shared::risk::RiskProfile>,
    rr_history: Vec<RrBlockDto>,
}

/// `GET /api/risk-profile?pair_key=BTC-USDT` — full deterministic IRML profile
/// for the pair plus the adaptive Reward/Risk block-calibration history.
pub async fn serve_risk_profile(
    State(state): State<Arc<AppState>>,
    Query(query): Query<RiskProfileQuery>,
) -> impl IntoResponse {
    let pair_key = match query.pair_key {
        Some(k) if !k.is_empty() => k,
        _ => {
            let cfg = state.config.read().await;
            let first = cfg.symbols.first().cloned().unwrap_or_default();
            default_pair_key(&first)
        }
    };
    let symbol = crate::server::helpers::extract_base_symbol(&pair_key);

    // Config-derived risk parameters.
    let (risk_cfg, suspend, drawdown_limit) = {
        let cfg = state.config.read().await;
        (
            cfg.risk.clone(),
            cfg.safety.consecutive_loss_suspend,
            cfg.safety.capital_drawdown_pct,
        )
    };
    let engine = crate::risk_engine::RiskEngine::new(risk_cfg, suspend, drawdown_limit);

    // Latest completed micro snapshot carries indicators + all context layers.
    let snapshot = match get_active_pair(&state.workspace, &pair_key).await {
        Some(pair) => pair.latest_snapshots_all_tf().await.0,
        None => None,
    };

    // Always surface the R:R history (backfilled from historical trades).
    engine
        .reconcile_blocks(&state.pool, &pair_key, &symbol)
        .await;
    let rr_history = load_rr_history(&state.pool, &pair_key).await;

    let response = match snapshot {
        Some(snap) => {
            let tf_secs = snap.timeframe_secs as i64;
            let profile = engine
                .evaluate(
                    &state.pool,
                    &pair_key,
                    &symbol,
                    tf_secs,
                    &snap.indicators,
                    snap.context.as_ref(),
                    snap.decision_context.as_ref(),
                    snap.statistical_context.as_ref(),
                    None,
                )
                .await;
            RiskProfileResponse {
                pair_key,
                symbol,
                available: true,
                message: None,
                profile: Some(profile),
                rr_history,
            }
        }
        None => RiskProfileResponse {
            pair_key,
            symbol,
            available: false,
            message: Some(
                "No live market snapshot yet for this pair; risk profile will populate once data warms up.".to_string(),
            ),
            profile: None,
            rr_history,
        },
    };

    Json(response)
}

async fn load_rr_history(pool: &sqlx::SqlitePool, pair_key: &str) -> Vec<RrBlockDto> {
    let rows: Vec<crate::db::RrCalibrationRow> = sqlx::query_as::<_, crate::db::RrCalibrationRow>(
        "SELECT id, pair_key, block_index, wins, losses, win_rate_estimate, breakeven_ratio,
                recommended_ratio, confidence, net_block_pnl, timestamp
         FROM rr_calibration WHERE pair_key = ?1 ORDER BY block_index ASC",
    )
    .bind(pair_key)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    rows.into_iter()
        .map(|r| RrBlockDto {
            block_index: r.block_index,
            wins: r.wins,
            losses: r.losses,
            win_rate_estimate: r.win_rate_estimate,
            breakeven_ratio: r.breakeven_ratio,
            recommended_ratio: r.recommended_ratio,
            confidence: r.confidence,
            net_block_pnl: r.net_block_pnl,
        })
        .collect()
}
