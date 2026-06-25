use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::analyzer;
use crate::config::{FibonacciConfig, TimeframeConfig};
use shared::models::MarketSnapshot;
use shared::normalized::NormalizedCandle;

pub struct BootstrapInput {
    pub base: String,
    pub rest_url: String,
    pub micro_cfg: TimeframeConfig,
    pub short_cfg: TimeframeConfig,
    pub medium_cfg: TimeframeConfig,
    pub large_cfg: TimeframeConfig,
    pub fib_config: FibonacciConfig,
    pub micro_secs: u64,
    pub short_secs: u64,
    pub medium_secs: u64,
    pub large_secs: u64,
    pub micro_limit: u64,
    pub short_limit: u64,
    pub medium_limit: u64,
    pub large_limit: u64,
}

pub async fn run_bootstrap(
    input: &BootstrapInput,
    micro_history: &Arc<RwLock<VecDeque<NormalizedCandle>>>,
    short_history: &Arc<RwLock<VecDeque<NormalizedCandle>>>,
    medium_history: &Arc<RwLock<VecDeque<NormalizedCandle>>>,
    large_history: &Arc<RwLock<VecDeque<NormalizedCandle>>>,
    micro_latest: &Arc<RwLock<Option<MarketSnapshot>>>,
    short_latest: &Arc<RwLock<Option<MarketSnapshot>>>,
    medium_latest: &Arc<RwLock<Option<MarketSnapshot>>>,
    large_latest: &Arc<RwLock<Option<MarketSnapshot>>>,
    micro_snapshot_history: &Arc<RwLock<VecDeque<MarketSnapshot>>>,
    short_snapshot_history: &Arc<RwLock<VecDeque<MarketSnapshot>>>,
    medium_snapshot_history: &Arc<RwLock<VecDeque<MarketSnapshot>>>,
    large_snapshot_history: &Arc<RwLock<VecDeque<MarketSnapshot>>>,
) {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    let micro_start = now_ms.saturating_sub(input.micro_secs * input.micro_limit * 1000);
    let short_start = now_ms.saturating_sub(input.short_secs * input.short_limit * 1000);
    let medium_start = now_ms.saturating_sub(input.medium_secs * input.medium_limit * 1000);
    let large_start = now_ms.saturating_sub(input.large_secs * input.large_limit * 1000);

    let fetch_base = input.base.clone();
    let fetch_rest = input.rest_url.clone();

    let bootstrap_result: Result<
        (
            Vec<NormalizedCandle>,
            Vec<NormalizedCandle>,
            Vec<NormalizedCandle>,
            Vec<NormalizedCandle>,
        ),
        String,
    > = tokio::try_join!(
        crate::adapters::hyperliquid_rest::fetch_historical_candles(
            &fetch_base,
            crate::adapters::hyperliquid_rest::timeframe_secs_to_interval(input.micro_secs),
            micro_start,
            now_ms,
            &fetch_rest,
        ),
        crate::adapters::hyperliquid_rest::fetch_historical_candles(
            &fetch_base,
            crate::adapters::hyperliquid_rest::timeframe_secs_to_interval(input.short_secs),
            short_start,
            now_ms,
            &fetch_rest,
        ),
        crate::adapters::hyperliquid_rest::fetch_historical_candles(
            &fetch_base,
            crate::adapters::hyperliquid_rest::timeframe_secs_to_interval(input.medium_secs),
            medium_start,
            now_ms,
            &fetch_rest,
        ),
        crate::adapters::hyperliquid_rest::fetch_historical_candles(
            &fetch_base,
            crate::adapters::hyperliquid_rest::timeframe_secs_to_interval(input.large_secs),
            large_start,
            now_ms,
            &fetch_rest,
        ),
    );

    let (warmed_micro, warmed_short, warmed_medium, warmed_large, _historical_micro) =
        match bootstrap_result {
            Ok((micro_candles, short_candles, medium_candles, large_candles)) => {
                println!(
                    "📡 Historical Bootstrap [{}]: Fetched {}/{}/{}/{} candles (1m/5m/15m/1h)",
                    input.base,
                    micro_candles.len(),
                    short_candles.len(),
                    medium_candles.len(),
                    large_candles.len()
                );

                if micro_candles.is_empty() {
                    eprintln!("⚠️  Historical Bootstrap [{}]: 1m REST returned 0 candles — micro chart will populate from live data only.", input.base);
                }
                if short_candles.is_empty() {
                    eprintln!("⚠️  Historical Bootstrap [{}]: 5m REST returned 0 candles.", input.base);
                }
                if medium_candles.is_empty() {
                    eprintln!("⚠️  Historical Bootstrap [{}]: 15m REST returned 0 candles.", input.base);
                }
                if large_candles.is_empty() {
                    eprintln!("⚠️  Historical Bootstrap [{}]: 1h REST returned 0 candles.", input.base);
                }

                let w_micro = analyzer::warm_indicators_for_timeframe(
                    micro_candles.clone(),
                    &input.micro_cfg,
                    &input.fib_config,
                    &input.base,
                    input.micro_secs,
                );
                let w_short = analyzer::warm_indicators_for_timeframe(
                    short_candles,
                    &input.short_cfg,
                    &input.fib_config,
                    &input.base,
                    input.short_secs,
                );
                let w_medium = analyzer::warm_indicators_for_timeframe(
                    medium_candles.clone(),
                    &input.medium_cfg,
                    &input.fib_config,
                    &input.base,
                    input.medium_secs,
                );
                let w_large = analyzer::warm_indicators_for_timeframe(
                    large_candles,
                    &input.large_cfg,
                    &input.fib_config,
                    &input.base,
                    input.large_secs,
                );

                (
                    Some(w_micro),
                    Some(w_short),
                    Some(w_medium),
                    Some(w_large),
                    Some(micro_candles),
                )
            }
            Err(e) => {
                eprintln!(
                    "⚠️  Historical Bootstrap [{}]: REST fetch failed — {}. Falling back to live-only data.",
                    input.base, e
                );
                (None, None, None, None, None)
            }
        };

    populate_buffers(
        &warmed_micro,
        &warmed_short,
        &warmed_medium,
        &warmed_large,
        micro_history,
        short_history,
        medium_history,
        large_history,
        micro_latest,
        short_latest,
        medium_latest,
        large_latest,
        micro_snapshot_history,
        short_snapshot_history,
        medium_snapshot_history,
        large_snapshot_history,
    )
    .await;
}

async fn populate_buffers(
    warmed_micro: &Option<analyzer::WarmedPipelineState>,
    warmed_short: &Option<analyzer::WarmedPipelineState>,
    warmed_medium: &Option<analyzer::WarmedPipelineState>,
    warmed_large: &Option<analyzer::WarmedPipelineState>,
    micro_history: &Arc<RwLock<VecDeque<NormalizedCandle>>>,
    short_history: &Arc<RwLock<VecDeque<NormalizedCandle>>>,
    medium_history: &Arc<RwLock<VecDeque<NormalizedCandle>>>,
    large_history: &Arc<RwLock<VecDeque<NormalizedCandle>>>,
    micro_latest: &Arc<RwLock<Option<MarketSnapshot>>>,
    short_latest: &Arc<RwLock<Option<MarketSnapshot>>>,
    medium_latest: &Arc<RwLock<Option<MarketSnapshot>>>,
    large_latest: &Arc<RwLock<Option<MarketSnapshot>>>,
    micro_snapshot_history: &Arc<RwLock<VecDeque<MarketSnapshot>>>,
    short_snapshot_history: &Arc<RwLock<VecDeque<MarketSnapshot>>>,
    medium_snapshot_history: &Arc<RwLock<VecDeque<MarketSnapshot>>>,
    large_snapshot_history: &Arc<RwLock<VecDeque<MarketSnapshot>>>,
) {
    populate_single(
        warmed_micro,
        micro_history,
        micro_latest,
        micro_snapshot_history,
    )
    .await;
    populate_single(
        warmed_short,
        short_history,
        short_latest,
        short_snapshot_history,
    )
    .await;
    populate_single(
        warmed_medium,
        medium_history,
        medium_latest,
        medium_snapshot_history,
    )
    .await;
    populate_single(
        warmed_large,
        large_history,
        large_latest,
        large_snapshot_history,
    )
    .await;
}

async fn populate_single(
    warmed: &Option<analyzer::WarmedPipelineState>,
    history: &Arc<RwLock<VecDeque<NormalizedCandle>>>,
    latest: &Arc<RwLock<Option<MarketSnapshot>>>,
    snapshot_history: &Arc<RwLock<VecDeque<MarketSnapshot>>>,
) {
    if let Some(ref w) = warmed {
        {
            let mut hist = history.write().await;
            for c in &w.history {
                hist.push_back(c.clone());
            }
        }
        if let Some(ref snap) = w.latest_snapshot {
            *latest.write().await = Some(snap.clone());
        }
        {
            let mut sh = snapshot_history.write().await;
            for snap in &w.snapshot_history {
                sh.push_back(snap.clone());
            }
        }
    }
}
