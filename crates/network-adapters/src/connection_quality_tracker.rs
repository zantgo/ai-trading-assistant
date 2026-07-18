use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

const MAX_EVENT_RETENTION_MS: u64 = 24 * 60 * 60 * 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum QualityWindow {
    OneHour,
    SixHour,
    TwentyFourHour,
}

impl QualityWindow {
    pub fn duration(&self) -> Duration {
        match self {
            Self::OneHour => Duration::from_secs(3600),
            Self::SixHour => Duration::from_secs(6 * 3600),
            Self::TwentyFourHour => Duration::from_secs(24 * 3600),
        }
    }

    fn database_name(&self) -> &'static str {
        match self {
            Self::OneHour => "ONE_HOUR",
            Self::SixHour => "SIX_HOUR",
            Self::TwentyFourHour => "TWENTY_FOUR_HOUR",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionQualityReport {
    pub window: QualityWindow,
    pub window_start_ms: u64,
    pub window_end_ms: u64,
    pub uptime_pct: f64,
    pub disconnect_count: u32,
    pub avg_reconnect_ms: f64,
    pub total_data_loss_secs: u64,
    pub reconstructed_candles: u32,
    pub score: f64,
}

#[derive(Clone)]
pub struct ConnectionQualityTracker {
    state: Arc<RwLock<TrackerState>>,
}

struct TrackerState {
    events: VecDeque<QualityEvent>,
    reconstructed_candle_count: u32,
    last_heartbeat_ms: u64,
    last_connected_ms: Option<u64>,
    cumulative_connected_secs: u64,
}

#[derive(Debug, Clone)]
enum QualityEvent {
    Connected { at_ms: u64 },
    Disconnected { at_ms: u64 },
    ReconnectCompleted { at_ms: u64, duration_ms: u64 },
}

impl ConnectionQualityTracker {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(TrackerState {
                events: VecDeque::new(),
                reconstructed_candle_count: 0,
                last_heartbeat_ms: 0,
                last_connected_ms: None,
                cumulative_connected_secs: 0,
            })),
        }
    }

    pub async fn record_connect(&self, at_ms: u64) {
        let mut state = self.state.write().await;
        if state.last_connected_ms.is_none() {
            state.last_connected_ms = Some(at_ms);
        }
        append_event(&mut state, QualityEvent::Connected { at_ms }, at_ms);
    }

    pub async fn record_disconnect(&self, at_ms: u64) {
        let mut state = self.state.write().await;
        if let Some(connected_at_ms) = state.last_connected_ms.take() {
            if at_ms >= connected_at_ms {
                state.cumulative_connected_secs = state
                    .cumulative_connected_secs
                    .saturating_add((at_ms - connected_at_ms) / 1000);
            }
        }
        append_event(&mut state, QualityEvent::Disconnected { at_ms }, at_ms);
    }

    pub async fn record_reconnect(&self, at_ms: u64, duration_ms: u64) {
        let mut state = self.state.write().await;
        if state.last_connected_ms.is_none() {
            state.last_connected_ms = Some(at_ms);
        }
        append_event(
            &mut state,
            QualityEvent::ReconnectCompleted { at_ms, duration_ms },
            at_ms,
        );
    }

    pub async fn record_heartbeat(&self, at_ms: u64) {
        let mut state = self.state.write().await;
        state.last_heartbeat_ms = at_ms;
    }

    pub async fn record_reconstructed_candle(&self) {
        let mut state = self.state.write().await;
        state.reconstructed_candle_count = state.reconstructed_candle_count.saturating_add(1);
    }

    pub async fn report(&self, window: QualityWindow, now_ms: u64) -> ConnectionQualityReport {
        let mut state = self.state.write().await;
        prune_events(&mut state, now_ms);

        let window_start_ms = now_ms.saturating_sub(window.duration().as_millis() as u64);
        let window_end_ms = now_ms;
        let events: Vec<QualityEvent> = state.events.iter().cloned().collect();
        let mut ordered_events = events;
        ordered_events.sort_by_key(event_timestamp);

        let mut disconnected_at_ms = None;
        for event in ordered_events.iter() {
            if event_timestamp(event) >= window_start_ms {
                break;
            }
            match event {
                QualityEvent::Disconnected { .. } => {
                    disconnected_at_ms = Some(window_start_ms);
                }
                QualityEvent::Connected { .. } | QualityEvent::ReconnectCompleted { .. } => {
                    disconnected_at_ms = None;
                }
            }
        }

        let mut data_loss_ms = 0_u64;
        let mut disconnect_count = 0_u32;
        let mut reconnect_sum_ms = 0_u64;
        let mut reconnect_count = 0_u32;

        for event in ordered_events.iter() {
            let at_ms = event_timestamp(event);
            if at_ms < window_start_ms || at_ms > window_end_ms {
                continue;
            }

            match event {
                QualityEvent::Connected { .. } => {
                    close_disconnect(&mut data_loss_ms, &mut disconnected_at_ms, at_ms);
                }
                QualityEvent::Disconnected { .. } => {
                    disconnect_count = disconnect_count.saturating_add(1);
                    if disconnected_at_ms.is_none() {
                        disconnected_at_ms = Some(at_ms);
                    }
                }
                QualityEvent::ReconnectCompleted { duration_ms, .. } => {
                    reconnect_sum_ms = reconnect_sum_ms.saturating_add(*duration_ms);
                    reconnect_count = reconnect_count.saturating_add(1);
                    close_disconnect(&mut data_loss_ms, &mut disconnected_at_ms, at_ms);
                }
            }
        }

        if let Some(disconnected_at_ms) = disconnected_at_ms {
            data_loss_ms =
                data_loss_ms.saturating_add(window_end_ms.saturating_sub(disconnected_at_ms));
        }

        let window_duration_secs = window.duration().as_secs();
        let total_data_loss_secs = (data_loss_ms / 1000).min(window_duration_secs);
        let uptime_pct = ((window_duration_secs.saturating_sub(total_data_loss_secs) as f64)
            / window_duration_secs as f64
            * 100.0)
            .clamp(0.0, 100.0);
        let avg_reconnect_ms = if reconnect_count == 0 {
            0.0
        } else {
            reconnect_sum_ms as f64 / reconnect_count as f64
        };
        let disconnect_factor = 1.0 - (disconnect_count as f64 / 10.0).min(1.0);
        let reconnect_factor = 1.0 - (avg_reconnect_ms / 5000.0).min(1.0);
        let data_loss_penalty = 5.0 * (total_data_loss_secs as f64 / 600.0).min(1.0);
        let reconstructed_penalty =
            5.0 * (state.reconstructed_candle_count as f64 / 100.0).min(1.0);
        let score = (0.5 * uptime_pct + 30.0 * disconnect_factor + 20.0 * reconnect_factor
            - data_loss_penalty
            - reconstructed_penalty)
            .clamp(0.0, 100.0);

        ConnectionQualityReport {
            window,
            window_start_ms,
            window_end_ms,
            uptime_pct,
            disconnect_count,
            avg_reconnect_ms,
            total_data_loss_secs,
            reconstructed_candles: state.reconstructed_candle_count,
            score,
        }
    }

    pub async fn run_persistence_loop(
        self: Arc<Self>,
        db_pool: sqlx::SqlitePool,
        cancel: tokio_util::sync::CancellationToken,
    ) {
        let registry = ConnectionQualityRegistry::default();
        registry
            .insert_existing("GLOBAL", 0, (*self).clone())
            .await;
        registry.run_persistence_loop(db_pool, cancel).await;
    }
}

/// Registry of per-`(pair_key, timeframe_secs)` quality scopes
/// (08-05-connection-quality.md). Each scope is an independent
/// `ConnectionQualityTracker`; the persistence loop collapses every scope
/// into per-window rows every 60 s.
#[derive(Clone, Default)]
pub struct ConnectionQualityRegistry {
    scopes: Arc<RwLock<std::collections::HashMap<(String, u64), ConnectionQualityTracker>>>,
}

impl ConnectionQualityRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get-or-create the tracker scope for `(pair_key, timeframe_secs)`.
    pub async fn scope(&self, pair_key: &str, timeframe_secs: u64) -> ConnectionQualityTracker {
        let mut scopes = self.scopes.write().await;
        scopes
            .entry((pair_key.to_string(), timeframe_secs))
            .or_insert_with(ConnectionQualityTracker::new)
            .clone()
    }

    /// Register a pre-built tracker under a scope key (used for legacy
    /// process-wide tracking and in tests).
    pub async fn insert_existing(
        &self,
        pair_key: &str,
        timeframe_secs: u64,
        tracker: ConnectionQualityTracker,
    ) {
        let mut scopes = self.scopes.write().await;
        scopes.insert((pair_key.to_string(), timeframe_secs), tracker);
    }

    /// Report for one scope, or `None` when the scope has never been seen.
    pub async fn scoped_report(
        &self,
        pair_key: &str,
        timeframe_secs: u64,
        window: QualityWindow,
        now_ms: u64,
    ) -> Option<ConnectionQualityReport> {
        let tracker = {
            let scopes = self.scopes.read().await;
            scopes
                .get(&(pair_key.to_string(), timeframe_secs))
                .cloned()
        };
        match tracker {
            Some(t) => Some(t.report(window, now_ms).await),
            None => None,
        }
    }

    /// All per-scope reports, keyed by `(pair_key, timeframe_secs)`.
    pub async fn all_reports(
        &self,
        window: QualityWindow,
        now_ms: u64,
    ) -> Vec<(String, u64, ConnectionQualityReport)> {
        let snapshot: Vec<((String, u64), ConnectionQualityTracker)> = {
            let scopes = self.scopes.read().await;
            scopes
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        };
        let mut out = Vec::with_capacity(snapshot.len());
        for ((pair_key, tf), tracker) in snapshot {
            out.push((pair_key, tf, tracker.report(window, now_ms).await));
        }
        out
    }

    /// Cross-scope aggregate (the process-wide view served when the API
    /// caller does not filter by instance/timeframe). Recomputes the
    /// composite score from the aggregated components using the canonical
    /// formula so a perfect empty session still scores 100.
    pub async fn aggregate_report(
        &self,
        window: QualityWindow,
        now_ms: u64,
    ) -> ConnectionQualityReport {
        let reports = self.all_reports(window, now_ms).await;
        let window_start_ms = now_ms.saturating_sub(window.duration().as_millis() as u64);
        if reports.is_empty() {
            return ConnectionQualityReport {
                window,
                window_start_ms,
                window_end_ms: now_ms,
                uptime_pct: 100.0,
                disconnect_count: 0,
                avg_reconnect_ms: 0.0,
                total_data_loss_secs: 0,
                reconstructed_candles: 0,
                score: 100.0,
            };
        }
        let n = reports.len() as f64;
        let uptime_pct = reports.iter().map(|(_, _, r)| r.uptime_pct).sum::<f64>() / n;
        let disconnect_count: u32 = reports.iter().map(|(_, _, r)| r.disconnect_count).sum();
        let reconnects: Vec<f64> = reports
            .iter()
            .map(|(_, _, r)| r.avg_reconnect_ms)
            .filter(|v| *v > 0.0)
            .collect();
        let avg_reconnect_ms = if reconnects.is_empty() {
            0.0
        } else {
            reconnects.iter().sum::<f64>() / reconnects.len() as f64
        };
        let total_data_loss_secs = reports
            .iter()
            .map(|(_, _, r)| r.total_data_loss_secs)
            .max()
            .unwrap_or(0);
        let reconstructed_candles: u32 =
            reports.iter().map(|(_, _, r)| r.reconstructed_candles).sum();
        let disconnect_factor = 1.0 - (disconnect_count as f64 / 10.0).min(1.0);
        let reconnect_factor = 1.0 - (avg_reconnect_ms / 5000.0).min(1.0);
        let data_loss_penalty =
            5.0 * (total_data_loss_secs as f64 / 600.0).min(1.0);
        let reconstructed_penalty =
            5.0 * (reconstructed_candles as f64 / 100.0).min(1.0);
        let score = (0.5 * uptime_pct + 30.0 * disconnect_factor + 20.0 * reconnect_factor
            - data_loss_penalty
            - reconstructed_penalty)
            .clamp(0.0, 100.0);
        ConnectionQualityReport {
            window,
            window_start_ms,
            window_end_ms: now_ms,
            uptime_pct,
            disconnect_count,
            avg_reconnect_ms,
            total_data_loss_secs,
            reconstructed_candles,
            score,
        }
    }

    /// Persist every scope × window as one row in
    /// `connection_quality_samples` every 60 s (03-01-00 §3). Rows carry the
    /// `(pair_key, timeframe_secs)` scope columns per 08-05.
    pub async fn run_persistence_loop(
        self,
        db_pool: sqlx::SqlitePool,
        cancel: tokio_util::sync::CancellationToken,
    ) {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            tokio::select! {
                _ = cancel.cancelled() => break,
                _ = interval.tick() => {
                    let now: DateTime<Utc> = Utc::now();
                    let now_ms = now.timestamp_millis().max(0) as u64;
                    for window in [
                        QualityWindow::OneHour,
                        QualityWindow::SixHour,
                        QualityWindow::TwentyFourHour,
                    ] {
                        for (pair_key, timeframe_secs, report) in
                            self.all_reports(window, now_ms).await
                        {
                            if let Err(error) = sqlx::query(
                                "INSERT INTO connection_quality_samples (timestamp_ms, window, uptime_pct, disconnect_count, avg_reconnect_ms, total_data_loss_secs, reconstructed_candles, score, pair_key, timeframe_secs) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                            )
                            .bind(now_ms as i64)
                            .bind(window.database_name())
                            .bind(report.uptime_pct)
                            .bind(report.disconnect_count as i64)
                            .bind(report.avg_reconnect_ms)
                            .bind(report.total_data_loss_secs as i64)
                            .bind(report.reconstructed_candles as i64)
                            .bind(report.score)
                            .bind(&pair_key)
                            .bind(timeframe_secs as i64)
                            .execute(&db_pool)
                            .await
                            {
                                eprintln!("Connection quality persistence failed: {error}");
                            }
                        }
                    }
                }
            }
        }
    }
}

impl Default for ConnectionQualityTracker {
    fn default() -> Self {
        Self::new()
    }
}

fn append_event(state: &mut TrackerState, event: QualityEvent, at_ms: u64) {
    state.events.push_back(event);
    prune_events(state, at_ms);
}

fn prune_events(state: &mut TrackerState, reference_ms: u64) {
    let cutoff_ms = reference_ms.saturating_sub(MAX_EVENT_RETENTION_MS);
    while state.events.len() > 1 {
        let should_remove = state
            .events
            .get(1)
            .map(|event| event_timestamp(event) < cutoff_ms)
            .unwrap_or(false);
        if should_remove {
            state.events.pop_front();
        } else {
            break;
        }
    }
}

fn event_timestamp(event: &QualityEvent) -> u64 {
    match event {
        QualityEvent::Connected { at_ms }
        | QualityEvent::Disconnected { at_ms }
        | QualityEvent::ReconnectCompleted { at_ms, .. } => *at_ms,
    }
}

fn close_disconnect(
    data_loss_ms: &mut u64,
    disconnected_at_ms: &mut Option<u64>,
    connected_at_ms: u64,
) {
    if let Some(disconnected_at_ms_value) = disconnected_at_ms.take() {
        *data_loss_ms =
            data_loss_ms.saturating_add(connected_at_ms.saturating_sub(disconnected_at_ms_value));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn tracker_records_connect_disconnect_correctly() {
        let tracker = ConnectionQualityTracker::new();
        let base_ms = 10_000_000;
        tracker.record_connect(base_ms).await;
        tracker.record_disconnect(base_ms + 10_000).await;
        tracker.record_connect(base_ms + 20_000).await;

        let report = tracker
            .report(QualityWindow::OneHour, base_ms + 60_000)
            .await;

        assert_eq!(report.disconnect_count, 1);
        assert_eq!(report.total_data_loss_secs, 10);
    }

    #[tokio::test]
    async fn uptime_pct_computed_correctly_for_known_events() {
        let tracker = ConnectionQualityTracker::new();
        let now_ms = 10_000_000;
        let window_start_ms = now_ms - 3_600_000;
        tracker.record_connect(window_start_ms - 1_000).await;
        tracker.record_disconnect(window_start_ms + 600_000).await;
        tracker.record_connect(window_start_ms + 1_200_000).await;

        let report = tracker.report(QualityWindow::OneHour, now_ms).await;

        assert_eq!(report.total_data_loss_secs, 600);
        assert!((report.uptime_pct - (3000.0 / 3600.0 * 100.0)).abs() < 0.0001);
    }

    #[tokio::test]
    async fn disconnect_count_in_window() {
        let tracker = ConnectionQualityTracker::new();
        let now_ms = 10_000_000;
        tracker.record_connect(now_ms - 7_200_000).await;
        tracker.record_disconnect(now_ms - 7_000_000).await;
        tracker.record_connect(now_ms - 6_900_000).await;
        tracker.record_disconnect(now_ms - 100).await;
        tracker.record_connect(now_ms - 50).await;

        let report = tracker.report(QualityWindow::OneHour, now_ms).await;

        assert_eq!(report.disconnect_count, 1);
    }

    #[tokio::test]
    async fn avg_reconnect_ms_correct() {
        let tracker = ConnectionQualityTracker::new();
        let now_ms = 10_000_000;
        tracker.record_reconnect(now_ms - 2_000, 1_000).await;
        tracker.record_reconnect(now_ms - 1_000, 3_000).await;

        let report = tracker.report(QualityWindow::OneHour, now_ms).await;

        assert!((report.avg_reconnect_ms - 2_000.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn score_decreases_with_more_disconnects() {
        let one_disconnect = ConnectionQualityTracker::new();
        let many_disconnects = ConnectionQualityTracker::new();
        let base_ms = 10_000_000;
        one_disconnect.record_connect(base_ms).await;
        one_disconnect.record_disconnect(base_ms + 1_000).await;
        one_disconnect.record_reconnect(base_ms + 2_000, 100).await;

        many_disconnects.record_connect(base_ms).await;
        for index in 0..5 {
            let disconnected_at_ms = base_ms + 1_000 + index * 10_000;
            many_disconnects.record_disconnect(disconnected_at_ms).await;
            many_disconnects
                .record_reconnect(disconnected_at_ms + 1_000, 100)
                .await;
        }

        let one_report = one_disconnect
            .report(QualityWindow::OneHour, base_ms + 60_000)
            .await;
        let many_report = many_disconnects
            .report(QualityWindow::OneHour, base_ms + 60_000)
            .await;

        assert!(many_report.score < one_report.score);
    }

    #[tokio::test]
    async fn reconstructed_candles_counted() {
        let tracker = ConnectionQualityTracker::new();
        tracker.record_reconstructed_candle().await;
        tracker.record_reconstructed_candle().await;
        tracker.record_reconstructed_candle().await;

        let report = tracker.report(QualityWindow::OneHour, 10_000_000).await;

        assert_eq!(report.reconstructed_candles, 3);
    }

    #[tokio::test]
    async fn window_1h_filters_out_old_events() {
        let tracker = ConnectionQualityTracker::new();
        let now_ms = 10_000_000;
        tracker.record_connect(now_ms - 7_200_000).await;
        tracker.record_disconnect(now_ms - 7_000_000).await;
        tracker.record_reconnect(now_ms - 6_999_000, 9_000).await;
        tracker.record_disconnect(now_ms - 1_000).await;
        tracker.record_reconnect(now_ms - 500, 1_000).await;

        let report = tracker.report(QualityWindow::OneHour, now_ms).await;

        assert_eq!(report.disconnect_count, 1);
        assert!((report.avg_reconnect_ms - 1_000.0).abs() < f64::EPSILON);
    }
}
