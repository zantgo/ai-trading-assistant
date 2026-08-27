//! COLD PATH — per-instance pipeline reliability metrics.
//!
//! `PipelineReliabilityMetrics` rolls up data-quality signals (coverage,
//! gaps, outliers, ordering) across a session so the `/api/data-quality`
//! endpoint can surface them. The struct is updated inline by the DIE L3
//! data-quality validation path and is read by the API gateway.
//!
//! This is the canonical home of the metrics described in
//! `docs/engines/data-infrastructure-engine/03-01-04-die-layer3-data-quality.md` §5.

use serde::Serialize;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Candle provenance counts backing the "Source mix" metric of
/// 03-01-04 §5: ratio of DB-warm vs REST-gap vs live candles.
#[derive(Debug, Clone, Default, Serialize)]
pub struct SourceMix {
    /// Candles seeded from the local telemetry store at bootstrap.
    pub db_warm: u64,
    /// Candles fetched from exchange REST history (bootstrap gap or runtime gap-fill).
    pub rest_gap: u64,
    /// Candles built live from the WebSocket trade stream.
    pub live: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PipelineReliabilityMetrics {
    pub coverage: f64,
    pub gap_count: u32,
    pub outliers_rejected: u32,
    /// Ticks that skipped the median filter because the rolling median was
    /// exactly zero (venue reset). Bypassed ticks are accepted, not rejected.
    pub outliers_bypassed: u32,
    pub out_of_order_dropped: u32,
    pub total_candles_processed: u64,
    pub reconstructed_candles: u32,
    pub source_mix: SourceMix,
}

impl Default for PipelineReliabilityMetrics {
    fn default() -> Self {
        Self {
            coverage: 0.0,
            gap_count: 0,
            outliers_rejected: 0,
            outliers_bypassed: 0,
            out_of_order_dropped: 0,
            total_candles_processed: 0,
            reconstructed_candles: 0,
            source_mix: SourceMix::default(),
        }
    }
}

#[derive(Clone)]
pub struct ReliabilityTracker {
    state: Arc<RwLock<PipelineReliabilityMetrics>>,
}

impl Default for ReliabilityTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl ReliabilityTracker {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(PipelineReliabilityMetrics::default())),
        }
    }

    pub async fn snapshot(&self) -> PipelineReliabilityMetrics {
        self.state.read().await.clone()
    }

    pub async fn has_data(&self) -> bool {
        let m = self.state.read().await;
        m.total_candles_processed > 0 || m.gap_count > 0
    }

    pub async fn increment_candles(&self, count: u64) {
        let mut m = self.state.write().await;
        m.total_candles_processed += count;
        m.source_mix.live += count;
        let expected = m.total_candles_processed + m.gap_count as u64;
        m.coverage = if expected > 0 {
            m.total_candles_processed as f64 / expected as f64
        } else {
            0.0
        };
    }

    pub async fn increment_gaps(&self, count: u32) {
        let mut m = self.state.write().await;
        m.gap_count += count;
        if m.total_candles_processed > 0 {
            let expected = m.total_candles_processed + m.gap_count as u64;
            m.coverage = if expected > 0 {
                m.total_candles_processed as f64 / expected as f64
            } else {
                0.0
            };
        }
    }

    pub async fn increment_outliers(&self, count: u32) {
        let mut m = self.state.write().await;
        m.outliers_rejected += count;
    }

    pub async fn increment_bypassed(&self, count: u32) {
        let mut m = self.state.write().await;
        m.outliers_bypassed += count;
    }

    /// Record bootstrap candle provenance (03-01-04 §2 / §5): how many warm
    /// candles came from the local DB vs the REST gap fetch.
    pub async fn record_bootstrap_sources(&self, db_warm: u64, rest_gap: u64) {
        let mut m = self.state.write().await;
        m.source_mix.db_warm += db_warm;
        m.source_mix.rest_gap += rest_gap;
    }

    pub async fn increment_out_of_order(&self, count: u32) {
        let mut m = self.state.write().await;
        m.out_of_order_dropped += count;
    }

    pub async fn increment_reconstructed(&self, count: u32) {
        let mut m = self.state.write().await;
        m.reconstructed_candles += count;
    }
}
