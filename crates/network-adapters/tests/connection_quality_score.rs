//! AC-DIE-10: Composite connection-quality score formula verification.
//!
//! Formula: 50×(uptime_pct/100) + 30×(1 − min(disconnects/10, 1)) +
//!           20×(1 − min(avg_reconnect_ms/5000, 1))
//!           − 5×min(total_data_loss_s/600, 1)
//!           − 5×min(reconstructed_candles/100, 1)
//!           clamped to [0, 100].

use network_adapters::connection_quality_tracker::{ConnectionQualityTracker, QualityWindow};

const ONE_HOUR_MS: u64 = 3_600_000;

#[tokio::test]
async fn perfect_session_scores_100() {
    let tracker = ConnectionQualityTracker::new();
    let now_ms = 10_000_000;

    let report = tracker.report(QualityWindow::OneHour, now_ms).await;

    assert!((report.score - 100.0).abs() < 1e-9);
    assert_eq!(report.uptime_pct, 100.0);
    assert_eq!(report.disconnect_count, 0);
    assert_eq!(report.avg_reconnect_ms, 0.0);
    assert_eq!(report.total_data_loss_secs, 0);
}

#[tokio::test]
async fn full_downtime_scores_45() {
    let tracker = ConnectionQualityTracker::new();
    let now_ms = 10_000_000;
    let window_start = now_ms - ONE_HOUR_MS;

    tracker.record_disconnect(window_start - 1).await;

    let report = tracker.report(QualityWindow::OneHour, now_ms).await;

    assert!((report.uptime_pct - 0.0).abs() < 1e-9);
    assert_eq!(report.total_data_loss_secs, 3600);
    assert!((report.score - 45.0).abs() < 1e-9);
}

#[tokio::test]
async fn disconnect_penalty_maxes_out_at_10() {
    let tracker = ConnectionQualityTracker::new();
    let now_ms = 10_000_000;
    let t0 = now_ms - ONE_HOUR_MS + 1;

    for i in 0..10_u64 {
        tracker.record_disconnect(t0 + i * 2).await;
        tracker.record_reconnect(t0 + i * 2 + 1, 1).await;
    }

    let report = tracker.report(QualityWindow::OneHour, now_ms).await;

    assert_eq!(report.disconnect_count, 10);
    assert!((report.score - 70.0).abs() < 0.1,
        "expected ~70: 50×(~100%) + 30×0 + 20×(~1), got {:.6}", report.score);
}

#[tokio::test]
async fn reconnect_penalty_maxes_out_at_5000ms() {
    let tracker = ConnectionQualityTracker::new();
    let now_ms = 10_000_000;
    let t0 = now_ms - ONE_HOUR_MS + 1;

    tracker.record_disconnect(t0).await;
    tracker.record_reconnect(t0 + 50, 5000).await;

    let report = tracker.report(QualityWindow::OneHour, now_ms).await;

    assert!(report.avg_reconnect_ms >= 5000.0);
    assert!((report.score - 77.0).abs() < 0.1,
        "expected ~77: 50×(~100%) + 30×0.9 + 20×0, got {:.6}", report.score);
}

#[tokio::test]
async fn data_loss_degrades_score_via_uptime() {
    let now_ms = 20_000_000;
    let t0 = now_ms - ONE_HOUR_MS;

    let tracker_light = ConnectionQualityTracker::new();
    tracker_light.record_disconnect(t0).await;
    tracker_light.record_reconnect(t0 + 300_000, 50).await;

    let tracker_heavy = ConnectionQualityTracker::new();
    tracker_heavy.record_disconnect(t0).await;
    tracker_heavy.record_reconnect(t0 + 1_800_000, 50).await;

    let light = tracker_light.report(QualityWindow::OneHour, now_ms).await;
    let heavy = tracker_heavy.report(QualityWindow::OneHour, now_ms).await;

    assert!(light.uptime_pct > heavy.uptime_pct,
        "lighter loss should preserve more uptime");
    assert!(light.score > heavy.score,
        "more data-loss must yield a lower score");
}

#[tokio::test]
async fn reconstructed_candles_tracked_in_report() {
    let tracker = ConnectionQualityTracker::new();

    tracker.record_reconstructed_candle(10_000_000).await;
    tracker.record_reconstructed_candle(10_000_000).await;
    tracker.record_reconstructed_candle(10_000_000).await;

    let report = tracker.report(QualityWindow::OneHour, 10_000_000).await;

    assert_eq!(report.reconstructed_candles, 3);
}

#[tokio::test]
async fn score_never_negative_and_never_exceeds_100() {
    let tracker = ConnectionQualityTracker::new();
    let now_ms = 30_000_000;
    let ws = now_ms - ONE_HOUR_MS;

    tracker.record_disconnect(ws - 1).await;

    for &t in &[ws, ws + 1, ws + 2, ws + 3, ws + 4, ws + 5, ws + 6, ws + 7, ws + 8, ws + 9] {
        tracker.record_disconnect(t).await;
    }

    for &t in &[ws + 10, ws + 11, ws + 12, ws + 13, ws + 14, ws + 15, ws + 16, ws + 17, ws + 18, ws + 19] {
        tracker.record_reconnect(t, 5000).await;
    }

    tracker.record_disconnect(ws + 20).await;

    let report = tracker.report(QualityWindow::OneHour, now_ms).await;

    assert!((report.score - 0.0).abs() <= 0.02,
        "worst-case expected ≈0, got {:.6}", report.score);
    assert!(report.score >= 0.0);
    assert!(report.score <= 100.0);
}
