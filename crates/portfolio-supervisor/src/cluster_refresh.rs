//! Liquidation cluster-matrix refresh task.
//!
//! Spawns a 5-minute timer per pair. On each tick, the task reads the
//! current OI / funding / mark / price history and calls
//! `core_domain::liquidity::estimate_clusters`. The resulting
//! `LiquidationClusterMatrix` is held in an `Arc<RwLock<Option<...>>>` so
//! the analyzer can attach the latest matrix to every completed
//! `MarketSnapshot`.
//!
//! Cancellation: the supplied `CancellationToken` cleanly stops the loop
//! on instance shutdown.

use rust_decimal::prelude::ToPrimitive;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

use core_domain::liquidity::{estimate_clusters, ClusterEstimateInput, LiquidationClusterMatrix};
use core_domain::models::MarketSnapshot;

use config_models::LiquidityConfig;
use crate::instance::{Instance, TimeframeBuffers};

/// Shared handle to the most recent cluster matrix. The analyzer reads
/// this on every candle close and attaches it to the snapshot.
pub type ClusterMatrixHandle = Arc<RwLock<Option<LiquidationClusterMatrix>>>;

/// Run the cluster refresh task for a single instance.
pub async fn run_cluster_refresh(
    instance: Arc<Instance>,
    config: LiquidityConfig,
    handle: ClusterMatrixHandle,
    cancel: CancellationToken,
) {
    let refresh_secs = config.cluster_refresh_secs.max(30);
    let buckets: Vec<u32> = vec![1, 3, 5, 10, 20, 50, 100];
    let weights: Vec<f64> = vec![0.05, 0.10, 0.20, 0.30, 0.20, 0.10, 0.05];
    println!(
        "🌀 Cluster Refresh: Started for {} ({}s cadence)",
        instance.pair_key(),
        refresh_secs
    );

    let mut interval = tokio::time::interval(Duration::from_secs(refresh_secs));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    // First tick fires immediately; we want a real first computation, so
    // skip it.
    interval.tick().await;

    loop {
        tokio::select! {
            biased;
            _ = cancel.cancelled() => {
                println!("🛑 Cluster Refresh: {} cancelled, shutting down.", instance.pair_key());
                break;
            }
            _ = interval.tick() => {}
        }

        let matrix = compute_cluster_matrix(&instance, &config, &buckets, &weights).await;
        if let Ok(m) = matrix {
            *handle.write().await = Some(m);
        }
    }
}

/// One-shot computation. Public so integration tests can drive it
/// without running the timer.
pub async fn compute_cluster_matrix(
    instance: &Instance,
    config: &LiquidityConfig,
    buckets: &[u32],
    weights: &[f64],
) -> Result<LiquidationClusterMatrix, String> {
    // 1. Pull latest snapshot from the micro timeframe.
    let micro = instance.micro.latest.read().await.clone();
    let micro = match micro {
        Some(s) => s,
        None => return Err("no micro snapshot yet".to_string()),
    };
    let mid = match micro.mid_price.to_f64() {
        Some(v) if v > 0.0 => v,
        _ => return Err("invalid mid price".to_string()),
    };
    let funding = micro.funding_rate.and_then(|d| d.to_f64()).unwrap_or(0.0);

    // 2. Get OI from the most recent completed snapshot.
    let oi = micro.open_interest.and_then(|d| d.to_f64()).unwrap_or(0.0);
    if oi <= 0.0 {
        return Err("no OI yet".to_string());
    }

    // 3. Build price history (last 200 candles, micro timeframe).
    let history_handle = instance.micro.history.read().await;
    let price_history: Vec<f64> = history_handle
        .iter()
        .rev()
        .take(200)
        .filter_map(|c| c.close.to_f64())
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    // 4. Compute.
    let symbol = micro.symbol.clone();
    let input = ClusterEstimateInput {
        symbol: &symbol,
        mid_price: mid,
        price_history: &price_history,
        total_oi_usd: oi,
        funding_rate: funding,
        long_oi_pct: None,
        maintenance_margin_rate: config.maintenance_margin_rate,
        funding_extreme_pct: config.funding_extreme_pct,
        funding_modulation_active: true,
        leverage_buckets: buckets,
        leverage_weights: weights,
        min_cluster_notional_usd: 50_000.0,
    };
    Ok(estimate_clusters(&input))
}

/// Spawn the cluster refresh task. Returns the shared handle.
pub fn spawn_cluster_refresh(
    instance: Arc<Instance>,
    config: LiquidityConfig,
    cancel: CancellationToken,
) -> ClusterMatrixHandle {
    let handle: ClusterMatrixHandle = Arc::new(RwLock::new(None));
    let h = handle.clone();
    let cfg = config.clone();
    tokio::spawn(async move {
        run_cluster_refresh(instance, cfg, h, cancel).await;
    });
    handle
}

#[cfg(test)]
mod tests {
    use super::*;
    use config_models::LiquidityConfig;
    use crate::instance::Instance;
    use crate::session::{Currency, ExchangeChoice, SessionState};
    use rust_decimal::Decimal;
    use core_domain::models::MarketSnapshot;
    use std::collections::VecDeque;

    fn empty_snapshot() -> MarketSnapshot {
        MarketSnapshot {
            exchange: Some(core_domain::normalized::Exchange::Hyperliquid),
            timeframe_secs: 60,
            timestamp: 0,
            symbol: "BTC-USDT".to_string(),
            is_completed: Some(true),
            mid_price: Decimal::from(50_000),
            bid_price: Decimal::ZERO,
            ask_price: Decimal::ZERO,
            bid_size: None,
            ask_size: None,
            funding_rate: Some(Decimal::from_f64_retain(0.0001).unwrap()),
            open_interest: Some(Decimal::from(1_000_000)),
            oi_delta_1h: None,
            mark_price: None,
            index_price: None,
            mark_index_spread_pct: None,
            prev_day_px: None,
            open: Some(Decimal::from(50_000)),
            high: Some(Decimal::from(50_100)),
            low: Some(Decimal::from(49_900)),
            close: Some(Decimal::from(50_000)),
            volume: Some(Decimal::from(100)),
            average_volume: Some(Decimal::from(100)),
            indicators: std::collections::HashMap::new(),
            context: None,
            decision_context: None,
            statistical_context: None,
            alignment: None,
            risk: None,
            analysis: None,
            advisory: None,
            risk_profile: None,
            liquidity: None,
            cluster: None,
        }
    }

    #[tokio::test]
    async fn compute_cluster_matrix_returns_err_with_no_data() {
        let session = SessionState::new();
        let _ = session
            .exchange
            .write()
            .await
            .insert(ExchangeChoice::Hyperliquid);
        let _ = session.base_currency.write().await.insert(Currency::USDC);
        let micro = TimeframeBuffers::new();
        let instance = Instance::new_test(
            "inst".to_string(),
            ("BTC".to_string(), "USDC".to_string()),
            micro,
        );
        let res = compute_cluster_matrix(
            &instance,
            &LiquidityConfig::default(),
            &[1, 3, 5, 10, 20, 50, 100],
            &[0.05, 0.10, 0.20, 0.30, 0.20, 0.10, 0.05],
        )
        .await;
        assert!(res.is_err(), "expected err with empty data");
    }

    #[tokio::test]
    async fn compute_cluster_matrix_returns_ok_with_data() {
        let session = SessionState::new();
        let _ = session
            .exchange
            .write()
            .await
            .insert(ExchangeChoice::Hyperliquid);
        let _ = session.base_currency.write().await.insert(Currency::USDC);
        let micro = TimeframeBuffers::new();
        let instance = Instance::new_test(
            "inst".to_string(),
            ("BTC".to_string(), "USDC".to_string()),
            micro,
        );
        // Pre-populate latest snapshot and price history.
        {
            let mut latest = instance.micro.latest.write().await;
            *latest = Some(empty_snapshot());
        }
        {
            let mut hist = instance.micro.history.write().await;
            for i in 0..50 {
                let price = Decimal::from(50_000) + Decimal::from(i * 10);
                hist.push_back(core_domain::normalized::NormalizedCandle {
                    symbol: "BTC-USDT".to_string(),
                    start_time_ms: i * 60_000,
                    duration_ms: 60_000,
                    open: price,
                    high: price,
                    low: price,
                    close: price,
                    volume: Decimal::from(1),
                    trades_count: 1,
                    reconstructed: None,
                });
            }
        }
        let res = compute_cluster_matrix(
            &instance,
            &LiquidityConfig::default(),
            &[1, 3, 5, 10, 20, 50, 100],
            &[0.05, 0.10, 0.20, 0.30, 0.20, 0.10, 0.05],
        )
        .await;
        assert!(res.is_ok(), "expected ok with data: {:?}", res.err());
        let m = res.unwrap();
        assert!(!m.short_clusters.is_empty(), "should have short clusters");
        assert!(!m.long_clusters.is_empty(), "should have long clusters");
    }
}
