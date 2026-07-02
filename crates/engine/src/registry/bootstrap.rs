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
    pub fast_cfg: TimeframeConfig,
    pub slow_cfg: TimeframeConfig,
    pub macro_cfg: TimeframeConfig,
    pub fib_config: FibonacciConfig,
    pub micro_secs: u64,
    pub fast_secs: u64,
    pub slow_secs: u64,
    pub macro_secs: u64,
    pub micro_limit: u64,
    pub fast_limit: u64,
    pub slow_limit: u64,
    pub macro_limit: u64,
}

pub async fn fetch_and_warm_bootstrap(
    input: &BootstrapInput,
) -> Result<
    (
        analyzer::WarmedPipelineState,
        analyzer::WarmedPipelineState,
        analyzer::WarmedPipelineState,
        analyzer::WarmedPipelineState,
    ),
    String,
> {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    let micro_start = now_ms.saturating_sub(input.micro_secs * input.micro_limit * 1000);
    let fast_start = now_ms.saturating_sub(input.fast_secs * input.fast_limit * 1000);
    let slow_start = now_ms.saturating_sub(input.slow_secs * input.slow_limit * 1000);
    let macro_start = now_ms.saturating_sub(input.macro_secs * input.macro_limit * 1000);

    let fetch_base = input.base.clone();
    let fetch_rest = input.rest_url.clone();

    let micro_fut = if input.micro_secs < 60 {
        None
    } else {
        Some(crate::adapters::hyperliquid_rest::fetch_historical_candles(
            &fetch_base,
            crate::adapters::hyperliquid_rest::timeframe_secs_to_interval(input.micro_secs),
            micro_start,
            now_ms,
            &fetch_rest,
        ))
    };
    let fast_fut = if input.fast_secs < 60 {
        None
    } else {
        Some(crate::adapters::hyperliquid_rest::fetch_historical_candles(
            &fetch_base,
            crate::adapters::hyperliquid_rest::timeframe_secs_to_interval(input.fast_secs),
            fast_start,
            now_ms,
            &fetch_rest,
        ))
    };
    let slow_fut = if input.slow_secs < 60 {
        None
    } else {
        Some(crate::adapters::hyperliquid_rest::fetch_historical_candles(
            &fetch_base,
            crate::adapters::hyperliquid_rest::timeframe_secs_to_interval(input.slow_secs),
            slow_start,
            now_ms,
            &fetch_rest,
        ))
    };
    let macro_fut = if input.macro_secs < 60 {
        None
    } else {
        Some(crate::adapters::hyperliquid_rest::fetch_historical_candles(
            &fetch_base,
            crate::adapters::hyperliquid_rest::timeframe_secs_to_interval(input.macro_secs),
            macro_start,
            now_ms,
            &fetch_rest,
        ))
    };

    let micro_candles: Result<Vec<NormalizedCandle>, String> = match micro_fut {
        Some(f) => f.await,
        None => Ok(Vec::new()),
    };
    let fast_candles: Result<Vec<NormalizedCandle>, String> = match fast_fut {
        Some(f) => f.await,
        None => Ok(Vec::new()),
    };
    let slow_candles: Result<Vec<NormalizedCandle>, String> = match slow_fut {
        Some(f) => f.await,
        None => Ok(Vec::new()),
    };
    let macro_candles: Result<Vec<NormalizedCandle>, String> = match macro_fut {
        Some(f) => f.await,
        None => Ok(Vec::new()),
    };

    #[allow(clippy::type_complexity)]
    let bootstrap_result: Result<
        (
            Vec<NormalizedCandle>,
            Vec<NormalizedCandle>,
            Vec<NormalizedCandle>,
            Vec<NormalizedCandle>,
        ),
        String,
    > = Ok((micro_candles?, fast_candles?, slow_candles?, macro_candles?));

    match bootstrap_result {
        Ok((micro_candles, fast_candles, slow_candles, macro_candles)) => {
            let label = |s: u64| -> String {
                if s >= 86400 { format!("{}d", s / 86400) }
                else if s >= 3600 { format!("{}h", s / 3600) }
                else if s >= 60 { format!("{}m", s / 60) }
                else { format!("{}s", s) }
            };
            println!(
                "📡 Historical Bootstrap [{}]: Fetched {}/{}/{}/{} candles ({}/{}/{}/{})",
                input.base,
                micro_candles.len(),
                fast_candles.len(),
                slow_candles.len(),
                macro_candles.len(),
                label(input.micro_secs),
                label(input.fast_secs),
                label(input.slow_secs),
                label(input.macro_secs),
            );

            if micro_candles.is_empty() {
                if input.micro_secs < 60 {
                    eprintln!("⚡ Historical Bootstrap [{}]: {}s sub-minute — starting from live data only.", input.base, input.micro_secs);
                } else {
                    eprintln!("⚠️  Historical Bootstrap [{}]: {}s REST returned 0 candles — chart will populate from live data only.", input.base, label(input.micro_secs));
                }
            }
            if fast_candles.is_empty() {
                if input.fast_secs < 60 {
                    eprintln!("⚡ Historical Bootstrap [{}]: {}s sub-minute — starting from live data only.", input.base, input.fast_secs);
                } else {
                    eprintln!("⚠️  Historical Bootstrap [{}]: {}s REST returned 0 candles.", input.base, label(input.fast_secs));
                }
            }
            if slow_candles.is_empty() {
                if input.slow_secs < 60 {
                    eprintln!("⚡ Historical Bootstrap [{}]: {}s sub-minute — starting from live data only.", input.base, input.slow_secs);
                } else {
                    eprintln!("⚠️  Historical Bootstrap [{}]: {}s REST returned 0 candles.", input.base, label(input.slow_secs));
                }
            }
            if macro_candles.is_empty() {
                if input.macro_secs < 60 {
                    eprintln!("⚡ Historical Bootstrap [{}]: {}s sub-minute — starting from live data only.", input.base, input.macro_secs);
                } else {
                    eprintln!("⚠️  Historical Bootstrap [{}]: {}s REST returned 0 candles.", input.base, label(input.macro_secs));
                }
            }

            let w_micro = analyzer::warm_indicators_for_timeframe(
                micro_candles.clone(),
                &input.micro_cfg,
                &input.fib_config,
                &input.base,
                input.micro_secs,
            );
            let w_fast = analyzer::warm_indicators_for_timeframe(
                fast_candles,
                &input.fast_cfg,
                &input.fib_config,
                &input.base,
                input.fast_secs,
            );
            let w_slow = analyzer::warm_indicators_for_timeframe(
                slow_candles.clone(),
                &input.slow_cfg,
                &input.fib_config,
                &input.base,
                input.slow_secs,
            );
            let w_macro = analyzer::warm_indicators_for_timeframe(
                macro_candles,
                &input.macro_cfg,
                &input.fib_config,
                &input.base,
                input.macro_secs,
            );

            Ok((w_micro, w_fast, w_slow, w_macro))
        }
        Err(e) => {
            eprintln!(
                "⚠️  Historical Bootstrap [{}]: REST fetch failed — {}. Falling back to live-only data.",
                input.base, e
            );
            Err(e)
        }
    }
}

pub(crate) async fn populate_buffers(
    warmed_micro: &Option<analyzer::WarmedPipelineState>,
    warmed_fast: &Option<analyzer::WarmedPipelineState>,
    warmed_slow: &Option<analyzer::WarmedPipelineState>,
    warmed_macro: &Option<analyzer::WarmedPipelineState>,
    micro_history: &Arc<RwLock<VecDeque<NormalizedCandle>>>,
    fast_history: &Arc<RwLock<VecDeque<NormalizedCandle>>>,
    slow_history: &Arc<RwLock<VecDeque<NormalizedCandle>>>,
    macro_history: &Arc<RwLock<VecDeque<NormalizedCandle>>>,
    micro_latest: &Arc<RwLock<Option<MarketSnapshot>>>,
    fast_latest: &Arc<RwLock<Option<MarketSnapshot>>>,
    slow_latest: &Arc<RwLock<Option<MarketSnapshot>>>,
    macro_latest: &Arc<RwLock<Option<MarketSnapshot>>>,
    micro_snapshot_history: &Arc<RwLock<VecDeque<MarketSnapshot>>>,
    fast_snapshot_history: &Arc<RwLock<VecDeque<MarketSnapshot>>>,
    slow_snapshot_history: &Arc<RwLock<VecDeque<MarketSnapshot>>>,
    macro_snapshot_history: &Arc<RwLock<VecDeque<MarketSnapshot>>>,
) {
    populate_single(
        warmed_micro,
        micro_history,
        micro_latest,
        micro_snapshot_history,
    )
    .await;
    populate_single(
        warmed_fast,
        fast_history,
        fast_latest,
        fast_snapshot_history,
    )
    .await;
    populate_single(
        warmed_slow,
        slow_history,
        slow_latest,
        slow_snapshot_history,
    )
    .await;
    populate_single(
        warmed_macro,
        macro_history,
        macro_latest,
        macro_snapshot_history,
    )
    .await;
}

pub(crate) async fn populate_single(
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
