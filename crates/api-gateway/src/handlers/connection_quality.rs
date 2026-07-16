use network_adapters::connection_quality_tracker::{ConnectionQualityReport, QualityWindow};
use crate::AppState;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct QualityParams {
    pub window: Option<String>,
}

pub async fn get_connection_quality(
    Query(params): Query<QualityParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<ConnectionQualityReport>, StatusCode> {
    let window = match params.window.as_deref() {
        Some("six_hour") => QualityWindow::SixHour,
        Some("twenty_four_hour") => QualityWindow::TwentyFourHour,
        _ => QualityWindow::OneHour,
    };
    let now_ms = chrono::Utc::now().timestamp_millis().max(0) as u64;
    let report = state.connection_quality.report(window, now_ms).await;
    Ok(Json(report))
}
