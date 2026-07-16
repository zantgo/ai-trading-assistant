use sqlx::SqlitePool;
use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::analyzer;
use crate::config::{FibonacciConfig, TimeframeConfig};
use crate::db;
use crate::session::{Currency, ExchangeChoice};
use shared::models::MarketSnapshot;
use shared::normalized::NormalizedCandle;

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
    pub micro_limit: u64,
    pub fast_limit: u64,
    pub slow_limit: u64,
    pub macro_limit: u64,
}

/// Fetch candles for a single timeframe via the local-DB-first, REST-gap
/// strategy, returning a chronologically ordered (oldest-first) candle vector.
///
/// - Sub-minute intervals (`secs < 60`) bypass both DB and REST entirely,
///   returning an empty vector so the pipeline starts cleanly from live ticks.
/// - Otherwise, the most recent completed candles are loaded from
///   `market_snapshots`; the exchange REST API is queried only for the missing
///   "gap" between the last local candle and now. If no local data exists, the
///   full lookback window is fetched from REST.
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
) -> Result<Vec<NormalizedCandle>, String> {
    // Sub-minute timeframes still consult the local DB first — sub-minute REST
    // history is rarely available from venue APIs, so the local warm base is
    // often the only usable seed. The cascade is:
    //   1. Local DB (PRIMARY for both sub-minute and ≥1m timeframes)
    //   2. REST gap window (best-effort; sub-minute REST is generally unavailable)
    //   3. Empty (caller falls through to live ticks)
    // A previous version short-circuited `secs < 60` to `Vec::new()`, bypassing
    // the DB and forcing the engine to bootstrap from live ticks only — see
    // `03-01-04-die-layer3-data-quality.md` §2 and the consolidated architecture
    // audit register (issue ARCH‑02).

    // 1. Local DB warm base (most recent completed candles for this symbol/tf).
    let db_candles = db::query_recent_candles(&pool, &internal_symbol, secs, limit as u32).await;

    // 2. REST gap window: only fetch what the DB is missing up to now.
    let rest_start = match db_candles.last() {
        Some(last) => last.start_time_ms.saturating_add(secs * 1000),
        None => now_ms.saturating_sub(secs * limit * 1000),
    };

    let rest_candles = if rest_start < now_ms {
        let interval = if is_bitget {
            crate::adapters::bitget_rest::timeframe_secs_to_interval(secs)
        } else {
            crate::adapters::hyperliquid_rest::timeframe_secs_to_interval(secs)
        };
        let fetched = if is_bitget {
            crate::adapters::bitget_rest::fetch_historical_candles(
                &exchange_raw,
                &internal_symbol,
                &product_type,
                interval,
                rest_start,
                now_ms,
                &rest_url,
            )
            .await
        } else {
            crate::adapters::hyperliquid_rest::fetch_historical_candles(
                &exchange_raw,
                &internal_symbol,
                interval,
                rest_start,
                now_ms,
                &rest_url,
            )
            .await
        };
        match fetched {
            Ok(c) => c,
            Err(e) => {
                if db_candles.is_empty() {
                    return Err(e);
                }
                eprintln!(
                    "⚠️  REST gap fetch failed for {} ({}s): {} — using local DB candles only.",
                    internal_symbol, secs, e
                );
                Vec::new()
            }
        }
    } else {
        Vec::new()
    };

    // 3. Merge DB + REST deduped by start_time_ms (REST wins on overlap).
    let mut map: BTreeMap<u64, NormalizedCandle> = BTreeMap::new();
    for c in db_candles {
        map.insert(c.start_time_ms, c);
    }
    for c in rest_candles {
        map.insert(c.start_time_ms, c);
    }
    let mut out: Vec<NormalizedCandle> = map.into_values().collect();
    if out.len() > limit as usize {
        out = out.split_off(out.len() - limit as usize);
    }
    Ok(out)
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

    let is_bitget = input.exchange_choice == ExchangeChoice::Bitget;
    let exchange_raw = input.exchange_choice.raw_symbol(&input.base, &input.quote);
    let product_type = input
        .exchange_choice
        .bitget_product_type(&input.quote)
        .unwrap_or("")
        .to_string();

    let (micro_res, fast_res, slow_res, macro_res) = tokio::join!(
        collect_candles(
            is_bitget,
            exchange_raw.clone(),
            input.internal_symbol.clone(),
            product_type.clone(),
            input.rest_url.clone(),
            input.pool.clone(),
            input.micro_secs,
            input.micro_limit,
            now_ms,
        ),
        collect_candles(
            is_bitget,
            exchange_raw.clone(),
            input.internal_symbol.clone(),
            product_type.clone(),
            input.rest_url.clone(),
            input.pool.clone(),
            input.fast_secs,
            input.fast_limit,
            now_ms,
        ),
        collect_candles(
            is_bitget,
            exchange_raw.clone(),
            input.internal_symbol.clone(),
            product_type.clone(),
            input.rest_url.clone(),
            input.pool.clone(),
            input.slow_secs,
            input.slow_limit,
            now_ms,
        ),
        collect_candles(
            is_bitget,
            exchange_raw.clone(),
            input.internal_symbol.clone(),
            product_type.clone(),
            input.rest_url.clone(),
            input.pool.clone(),
            input.macro_secs,
            input.macro_limit,
            now_ms,
        ),
    );

    let micro_candles = micro_res?;
    let fast_candles = fast_res?;
    let slow_candles = slow_res?;
    let macro_candles = macro_res?;

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

    let w_micro = analyzer::warm_indicators_for_timeframe(
        micro_candles,
        &input.micro_cfg,
        &input.fib_config,
        &input.internal_symbol,
        input.micro_secs,
    );
    let w_fast = analyzer::warm_indicators_for_timeframe(
        fast_candles,
        &input.fast_cfg,
        &input.fib_config,
        &input.internal_symbol,
        input.fast_secs,
    );
    let w_slow = analyzer::warm_indicators_for_timeframe(
        slow_candles,
        &input.slow_cfg,
        &input.fib_config,
        &input.internal_symbol,
        input.slow_secs,
    );
    let w_macro = analyzer::warm_indicators_for_timeframe(
        macro_candles,
        &input.macro_cfg,
        &input.fib_config,
        &input.internal_symbol,
        input.macro_secs,
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
