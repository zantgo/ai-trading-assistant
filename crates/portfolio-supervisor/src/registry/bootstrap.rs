use sqlx::SqlitePool;
use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;

use market_analyzer::analyzer;
use config_models::{FibonacciConfig, TimeframeConfig};
use database_storage;
use crate::session::{Currency, ExchangeChoice};
use core_domain::models::MarketSnapshot;
use core_domain::normalized::NormalizedCandle;

pub struct BootstrapInput {
    pub base: String,
    /// Unified internal symbol (e.g. "BTC-USDT") assigned to all candles.
    pub internal_symbol: String,
    /// Settlement/quote currency for this session (drives raw symbol + product type).
    pub quote: Currency,
    pub rest_url: String,
    pub exchange_choice: ExchangeChoice,
    pub pool: SqlitePool,
    pub micro_cfg: TimeframeConfig,
    pub fast_cfg: TimeframeConfig,
    pub slow_cfg: TimeframeConfig,
    pub macro_cfg: TimeframeConfig,
    pub fib_config: FibonacciConfig,
    pub micro_secs: u64,
    pub fast_secs: u64,
    pub slow_secs: u64,
    pub macro_secs: u64,
    /// Canonical candle buffer size from `[candle_buffer] size` (CB-01).
    /// Single source of truth for the rolling window. Replaces the previous
    /// per-tier `analysis_limit` field.
    pub buffer_size: usize,
    /// Per-TF stale-threshold (CB-04 / DCP-05 / ILS-07).
    #[allow(dead_code)]
    pub stale_threshold_secs: u64,
    /// Per-TF fetch timeout (HFP-10).
    pub fetch_timeout_ms: u64,
    /// Sub-minute bypass flag (CB-05 / HFP-03).
    #[allow(dead_code)]
    pub sub_minute_skip_historical: bool,
    /// When present, bootstrap candle provenance (DB-warm vs REST-gap) is
    /// recorded into the pipeline reliability source mix (03-01-04 §5).
    pub reliability: Option<Arc<network_adapters::pipeline_reliability::ReliabilityTracker>>,
}

/// Fetch candles for a single timeframe via the
/// [`HistoricalFetchPolicy`](network_adapters::adapters::historical_fetch::HistoricalFetchPolicy)
/// trait. The trait hides per-exchange divergence (HFP-01 … HFP-10) and
/// handles sub-minute short-circuit (HFP-03) internally. Returns a
/// chronologically ordered (oldest-first) candle vector plus provenance
/// counts `(candles, db_warm, rest_gap)` for the source-mix metric
/// (03-01-04 §5).
async fn collect_candles(
    is_bitget: bool,
    exchange_raw: String,
    internal_symbol: String,
    product_type: String,
    rest_url: String,
    pool: SqlitePool,
    secs: u64,
    limit: u64,
    now_ms: u64,
    fetch_timeout_ms: u64,
) -> Result<(Vec<NormalizedCandle>, u64, u64), String> {
    use network_adapters::adapters::bitget_historical_fetch::BitgetHistoricalFetch;
    use network_adapters::adapters::historical_fetch::{
        HistoricalFetchPolicy, HistoricalFetchRequest,
    };
    use network_adapters::adapters::hyperliquid_historical_fetch::HyperliquidHistoricalFetch;

    // 1. Local DB warm base (most recent completed candles for this symbol/tf).
    let db_candles = database_storage::query_recent_candles(&pool, &internal_symbol, secs, limit as u32).await;

    // 2. Build the HistoricalFetchPolicy implementation for this exchange.
    //    The policy handles HFP-03 sub-minute bypass, HFP-04..HFP-06
    //    pagination, HFP-07 open-candle filter, HFP-08 provenance tagging,
    //    and HFP-10 timeout enforcement.
    let request = HistoricalFetchRequest {
        exchange_symbol: exchange_raw.clone(),
        internal_symbol: internal_symbol.clone(),
        timeframe_secs: secs,
        target_count: limit as usize,
        end_ts: now_ms,
        product_type: if is_bitget {
            Some(product_type.clone())
        } else {
            None
        },
        fetch_timeout_ms,
    };

    let policy: Box<dyn HistoricalFetchPolicy> = if is_bitget {
        Box::new(BitgetHistoricalFetch::new(rest_url.clone(), product_type.clone()))
    } else {
        Box::new(HyperliquidHistoricalFetch::new(rest_url.clone()))
    };

    // 3. Compute the historical-fetch range. The policy paginates from
    //    `end_ts` backward/forward as needed; for ≥ 1 minute TFs we anchor
    //    on the DB's last candle + 1 interval (gap fill), and for cold DBs
    //    we anchor on the full lookback window.
    let rest_candles = if secs < 60 {
        // HFP-03 sub-minute: short-circuit at the trait-caller level. We
        // still attempt the DB read above because callers may want the
        // persisted history to seed the live buffer if any exists.
        Vec::new()
    } else {
        match policy.fetch(request).await {
            Ok(c) => c,
            Err(network_adapters::adapters::historical_fetch::HistoricalFetchError::SubMinuteBypassed(_)) => {
                Vec::new()
            }
            Err(e) => {
                if db_candles.is_empty() {
                    return Err(format!("Historical fetch failed: {}", e));
                }
                eprintln!(
                    "⚠️  Historical fetch failed for {} ({}s): {} — using local DB candles only.",
                    internal_symbol, secs, e
                );
                Vec::new()
            }
        }
    };

    // 4. Merge DB + REST deduped by start_time_ms. Per 03-01-04 §3 + HFP-09,
    //    the local store is authoritative for already-seen candles: REST is
    //    inserted first, then DB overwrites on overlap.
    let db_keys: std::collections::HashSet<u64> =
        db_candles.iter().map(|c| c.start_time_ms).collect();
    let mut map: BTreeMap<u64, NormalizedCandle> = BTreeMap::new();
    for c in rest_candles {
        map.insert(c.start_time_ms, c);
    }
    for c in db_candles {
        map.insert(c.start_time_ms, c);
    }
    let mut out: Vec<NormalizedCandle> = map.into_values().collect();
    if out.len() > limit as usize {
        out = out.split_off(out.len() - limit as usize);
    }
    let db_warm = out
        .iter()
        .filter(|c| db_keys.contains(&c.start_time_ms))
        .count() as u64;
    let rest_gap = out.len() as u64 - db_warm;
    Ok((out, db_warm, rest_gap))
}

/// Minimum completed-bar count for a tier's warm-up to be considered
/// sufficient (03-01-04 §2.1.1).  200 bars guarantees every structural
/// indicator (Ichimoku ~78, Volume Profile ~100, SMC ~50, Fibonacci pivots)
/// completes its warm-up buffer before the first live snapshot is emitted.
/// Below this gate the tier still warms best-effort with whatever history
/// exists, but the shortfall is logged and indicators emit `WARMING`
/// labels / `confidence = 0.0` until their per-indicator minimum buffers fill.
pub const MIN_WARMUP_BARS: usize = 200;

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

    let is_bitget = input.exchange_choice == ExchangeChoice::Bitget;
    let exchange_raw = input.exchange_choice.raw_symbol(&input.base, &input.quote);
    let product_type = input
        .exchange_choice
        .bitget_product_type(&input.quote)
        .unwrap_or("")
        .to_string();

    let buffer_size_u64 = input.buffer_size as u64;
    let fetch_timeout_ms = input.fetch_timeout_ms;
    let (micro_res, fast_res, slow_res, macro_res) = tokio::join!(
        collect_candles(
            is_bitget,
            exchange_raw.clone(),
            input.internal_symbol.clone(),
            product_type.clone(),
            input.rest_url.clone(),
            input.pool.clone(),
            input.micro_secs,
            buffer_size_u64,
            now_ms,
            fetch_timeout_ms,
        ),
        collect_candles(
            is_bitget,
            exchange_raw.clone(),
            input.internal_symbol.clone(),
            product_type.clone(),
            input.rest_url.clone(),
            input.pool.clone(),
            input.fast_secs,
            buffer_size_u64,
            now_ms,
            fetch_timeout_ms,
        ),
        collect_candles(
            is_bitget,
            exchange_raw.clone(),
            input.internal_symbol.clone(),
            product_type.clone(),
            input.rest_url.clone(),
            input.pool.clone(),
            input.slow_secs,
            buffer_size_u64,
            now_ms,
            fetch_timeout_ms,
        ),
        collect_candles(
            is_bitget,
            exchange_raw.clone(),
            input.internal_symbol.clone(),
            product_type.clone(),
            input.rest_url.clone(),
            input.pool.clone(),
            input.macro_secs,
            buffer_size_u64,
            now_ms,
            fetch_timeout_ms,
        ),
    );

    let (micro_candles, micro_db, micro_rest) = micro_res?;
    let (fast_candles, fast_db, fast_rest) = fast_res?;
    let (slow_candles, slow_db, slow_rest) = slow_res?;
    let (macro_candles, macro_db, macro_rest) = macro_res?;

    if let Some(ref reliability) = input.reliability {
        reliability
            .record_bootstrap_sources(
                micro_db + fast_db + slow_db + macro_db,
                micro_rest + fast_rest + slow_rest + macro_rest,
            )
            .await;
    }

    let label = |s: u64| -> String {
        if s >= 86400 {
            format!("{}d", s / 86400)
        } else if s >= 3600 {
            format!("{}h", s / 3600)
        } else if s >= 60 {
            format!("{}m", s / 60)
        } else {
            format!("{}s", s)
        }
    };
    println!(
        "📡 Historical Bootstrap [{}]: Warmed {}/{}/{}/{} candles ({}/{}/{}/{})",
        input.internal_symbol,
        micro_candles.len(),
        fast_candles.len(),
        slow_candles.len(),
        macro_candles.len(),
        label(input.micro_secs),
        label(input.fast_secs),
        label(input.slow_secs),
        label(input.macro_secs),
    );

    let warn_empty = |candles: &[NormalizedCandle], secs: u64| {
        if candles.is_empty() {
            if secs < 60 {
                eprintln!(
                    "⚡ Historical Bootstrap [{}]: {}s sub-minute — starting from live data only.",
                    input.internal_symbol, secs
                );
            } else {
                eprintln!(
                    "⚠️  Historical Bootstrap [{}]: {} returned 0 candles — chart will populate from live data only.",
                    input.internal_symbol,
                    label(secs)
                );
            }
        }
    };
    warn_empty(&micro_candles, input.micro_secs);
    warn_empty(&fast_candles, input.fast_secs);
    warn_empty(&slow_candles, input.slow_secs);
    warn_empty(&macro_candles, input.macro_secs);

    // min_warmup_bars gate (03-01-04 §2.1.1): warm-up proceeds best-effort,
    // but a tier seeded below the gate is flagged as partially warmed.
    let gate_warn = |candles: &[NormalizedCandle], secs: u64| {
        if !candles.is_empty() && candles.len() < MIN_WARMUP_BARS {
            eprintln!(
                "⚠️  Historical Bootstrap [{}]: {} seeded with {} bars (< min_warmup_bars = {}) — indicators start partially warmed (INSUFFICIENT_DATA until buffers fill).",
                input.internal_symbol,
                label(secs),
                candles.len(),
                MIN_WARMUP_BARS
            );
        }
    };
    gate_warn(&micro_candles, input.micro_secs);
    gate_warn(&fast_candles, input.fast_secs);
    gate_warn(&slow_candles, input.slow_secs);
    gate_warn(&macro_candles, input.macro_secs);

    // v6.10 (Phase 5 / E1): bootstrap warm-up runs with all indicators enabled
    // by default. Per-instance activation sets are constructed later in
    // `build_pipelines`; warm-up only needs to seed all 50 indicators so the
    // production pipelines can apply active_set filtering to the warmed state.
    let warm_active_set = market_analyzer::active_set::ActiveSet::all_enabled();

    let w_micro = analyzer::warm_indicators_for_timeframe(
        micro_candles,
        &input.micro_cfg,
        &input.fib_config,
        &input.internal_symbol,
        input.micro_secs,
        core_domain::models::TimeframeSlot::Micro,
        input.buffer_size,
        &warm_active_set,
    );
    let w_fast = analyzer::warm_indicators_for_timeframe(
        fast_candles,
        &input.fast_cfg,
        &input.fib_config,
        &input.internal_symbol,
        input.fast_secs,
        core_domain::models::TimeframeSlot::Fast,
        input.buffer_size,
        &warm_active_set,
    );
    let w_slow = analyzer::warm_indicators_for_timeframe(
        slow_candles,
        &input.slow_cfg,
        &input.fib_config,
        &input.internal_symbol,
        input.slow_secs,
        core_domain::models::TimeframeSlot::Slow,
        input.buffer_size,
        &warm_active_set,
    );
    let w_macro = analyzer::warm_indicators_for_timeframe(
        macro_candles,
        &input.macro_cfg,
        &input.fib_config,
        &input.internal_symbol,
        input.macro_secs,
        core_domain::models::TimeframeSlot::Macro,
        input.buffer_size,
        &warm_active_set,
    );

    Ok((w_micro, w_fast, w_slow, w_macro))
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
    latest_oi: &Arc<RwLock<Option<rust_decimal::Decimal>>>,
    latest_funding: &Arc<RwLock<Option<rust_decimal::Decimal>>>,
    latest_mark_px: &Arc<RwLock<Option<rust_decimal::Decimal>>>,
    latest_index_px: &Arc<RwLock<Option<rust_decimal::Decimal>>>,
    oi_history: &Arc<RwLock<VecDeque<f64>>>,
    funding_history: &Arc<RwLock<VecDeque<f64>>>,
) {
    // All four timeframes share the same per-pair derivatives state
    // (latest_* locks and rolling history), so any warmed TF carries
    // the right restored values for them.
    if let Some(ref w) = warmed_micro {
        populate_derivatives(w, latest_oi, latest_funding, latest_mark_px, latest_index_px, oi_history, funding_history).await;
    } else if let Some(ref w) = warmed_fast {
        populate_derivatives(w, latest_oi, latest_funding, latest_mark_px, latest_index_px, oi_history, funding_history).await;
    } else if let Some(ref w) = warmed_slow {
        populate_derivatives(w, latest_oi, latest_funding, latest_mark_px, latest_index_px, oi_history, funding_history).await;
    } else if let Some(ref w) = warmed_macro {
        populate_derivatives(w, latest_oi, latest_funding, latest_mark_px, latest_index_px, oi_history, funding_history).await;
    }

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

/// Restore derivatives locks from a warmed state. Mirrors
/// `warm.rs::warm_derivatives_from_snapshots` on the write side — every
/// field that the warm helper put into `derivatives_state` is reapplied
/// to the live Arc locks here so the first WS event after boot sees
/// non-None priors.
async fn populate_derivatives(
    w: &analyzer::WarmedPipelineState,
    latest_oi: &Arc<RwLock<Option<rust_decimal::Decimal>>>,
    latest_funding: &Arc<RwLock<Option<rust_decimal::Decimal>>>,
    latest_mark_px: &Arc<RwLock<Option<rust_decimal::Decimal>>>,
    latest_index_px: &Arc<RwLock<Option<rust_decimal::Decimal>>>,
    oi_history: &Arc<RwLock<VecDeque<f64>>>,
    funding_history: &Arc<RwLock<VecDeque<f64>>>,
) {
    use market_analyzer::analyzer::warm::DerivativesWarmedState;
    let d = &w.derivatives_state;
    if d.latest_oi.is_some() {
        *latest_oi.write().await = d.latest_oi;
    }
    if d.latest_funding.is_some() {
        *latest_funding.write().await = d.latest_funding;
    }
    if d.latest_mark_px.is_some() {
        *latest_mark_px.write().await = d.latest_mark_px;
    }
    if d.latest_index_px.is_some() {
        *latest_index_px.write().await = d.latest_index_px;
    }
    if !d.oi_history.is_empty() {
        let mut hist = oi_history.write().await;
        hist.clear();
        for v in &d.oi_history {
            hist.push_back(*v);
        }
    }
    if !d.funding_history.is_empty() {
        let mut hist = funding_history.write().await;
        hist.clear();
        for v in &d.funding_history {
            hist.push_back(*v);
        }
    }
    let _ = std::marker::PhantomData::<DerivativesWarmedState>;
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
