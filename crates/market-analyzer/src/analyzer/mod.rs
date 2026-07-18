use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::{broadcast, RwLock};
use tokio_util::sync::CancellationToken;

use config_models::FibonacciConfig;
use config_models::OrderBookConfig;
use config_models::QualityConfig;
use config_models::TimeframeConfig;
use network_adapters::pipeline_reliability::ReliabilityTracker;

use crate::sr_engine::SrRoleTracker;
use crate::indicators::normalized::NormalizedIndicatorValue;
use crate::indicators::normalized::PreviousBarState;
use crate::indicators::{
    detect_pattern, Adx, AnchoredVwap, Aroon, Atr, AwesomeOscillator, Bbwp, BollingerBands,
    Candlestick, CandlestickConfig, Cci, ChandeMO, Choppiness, Cmf, DivergenceDetector, Donchian,
    Ema, FibonacciRange, ForceIndex, HistoricalVolatility, HullMA, Ichimoku, Keltner, LinRegSlope,
    Macd, Mfi, Obv, OrderBookAnalysis, ParabolicSar, PivotMethod, PivotPoints, Rsi,
    SeriesDivergence, SmartMoney, SqueezeMomentum, StdDevChannel, Stochastic, Supertrend,
    VolumeProfile, WilliamsR, ZScore,
};
use core_domain::liquidity::LiquidationClusterMatrix;
use core_domain::models::{CandleQualityEnvelope, MarketSnapshot, SequenceIntegrity};
use core_domain::normalized::{Exchange, NormalizedCandle, NormalizedEvent};
use crate::candle_generator::CandleGenerator;
use core_domain::statistics::{StatisticsConfig, StatisticsEngine};

pub mod normalize;
pub mod warm;
pub use warm::{warm_indicators_for_timeframe, WarmedPipelineState, HIST_BUFFER_MAX};

pub struct TimeframePipeline {
    pub history: Arc<RwLock<VecDeque<NormalizedCandle>>>,
    pub broadcast_tx: broadcast::Sender<MarketSnapshot>,
    pub latest_snapshot: Arc<RwLock<Option<MarketSnapshot>>>,
    pub snapshot_history: Arc<RwLock<VecDeque<MarketSnapshot>>>,
    pub timeframe_secs: u64,
    pub timeframe_label: &'static str,
    pub divergence_detector: Arc<tokio::sync::Mutex<DivergenceDetector>>,
    pub sr_tracker: Arc<tokio::sync::Mutex<SrRoleTracker>>,
    pub fibonacci: FibonacciConfig,
    /// Latest Open Interest (shared across timeframes, updated by WS OI events).
    pub latest_oi: Arc<RwLock<Option<Decimal>>>,
    /// Latest Funding Rate (shared across timeframes, updated by WS funding events).
    pub latest_funding: Arc<RwLock<Option<Decimal>>>,
    /// Latest Mark Price (shared across timeframes, updated by mark events).
    pub latest_mark_px: Arc<RwLock<Option<Decimal>>>,
    /// Latest Index Price (shared across timeframes).
    pub latest_index_px: Arc<RwLock<Option<Decimal>>>,
    /// Active indicator/signal activation set (from config).
    pub active_set: crate::active_set::ActiveSet,
}

pub struct ActivePair {
    pub symbol: String,
    pub micro: TimeframePipeline,
    pub fast: TimeframePipeline,
    pub slow: TimeframePipeline,
    pub r#macro: TimeframePipeline,
    pub snapshot_tx: tokio::sync::mpsc::Sender<NormalizedEvent>,
    pub cancel: CancellationToken,
    /// Latest Open Interest (shared across all timeframes, updated by WS events).
    pub latest_oi: Arc<RwLock<Option<Decimal>>>,
    /// Latest Funding Rate (shared across all timeframes, updated by WS events).
    pub latest_funding: Arc<RwLock<Option<Decimal>>>,
    /// Latest Mark Price (shared across all timeframes, updated by mark events).
    pub latest_mark_px: Arc<RwLock<Option<Decimal>>>,
    /// Latest Index Price (shared across all timeframes).
    pub latest_index_px: Arc<RwLock<Option<Decimal>>>,
    /// Latest LiquidationClusterMatrix (Phase 2). Updated by the
    /// `cluster_refresh` task every 5 minutes. The analyzer reads this
    /// when building each completed snapshot.
    pub cluster_matrix: Arc<RwLock<Option<core_domain::liquidity::LiquidationClusterMatrix>>>,
    /// Cross-cutting latency telemetry (ingest skew, observation loop,
    /// heartbeat) for the DIE observation path.
    pub latency_tracker: core_domain::SharedLatencyTracker,
}

impl ActivePair {
    fn pipeline_for(&self, timeframe_secs: u64) -> &TimeframePipeline {
        if self.fast.timeframe_secs == timeframe_secs {
            return &self.fast;
        }
        if self.slow.timeframe_secs == timeframe_secs {
            return &self.slow;
        }
        if self.r#macro.timeframe_secs == timeframe_secs {
            return &self.r#macro;
        }
        &self.micro
    }

    pub fn subscribe_broadcast(&self, timeframe_secs: u64) -> broadcast::Receiver<MarketSnapshot> {
        self.pipeline_for(timeframe_secs).broadcast_tx.subscribe()
    }

    pub async fn latest_close_str(&self) -> Option<String> {
        let hist = self.micro.history.read().await;
        hist.back().map(|c| c.close.to_string())
    }

    pub async fn latest_price(&self) -> Option<f64> {
        let snap = self.micro.latest_snapshot.read().await;
        snap.as_ref()
            .and_then(|s| s.mid_price.to_string().parse::<f64>().ok())
    }

    pub async fn snapshot_history_vec(&self, timeframe_secs: u64) -> Vec<MarketSnapshot> {
        let hist = self
            .pipeline_for(timeframe_secs)
            .snapshot_history
            .read()
            .await;
        hist.iter().cloned().collect()
    }

    /// Latest completed snapshot for each of the four timeframes
    /// (micro, fast, slow, macro), for cross-timeframe synthesis.
    pub async fn latest_snapshots_all_tf(
        &self,
    ) -> (
        Option<MarketSnapshot>,
        Option<MarketSnapshot>,
        Option<MarketSnapshot>,
        Option<MarketSnapshot>,
    ) {
        (
            self.micro.latest_snapshot.read().await.clone(),
            self.fast.latest_snapshot.read().await.clone(),
            self.slow.latest_snapshot.read().await.clone(),
            self.r#macro.latest_snapshot.read().await.clone(),
        )
    }
}

pub async fn run_event_router(
    mut rx: Receiver<NormalizedEvent>,
    micro_tx: Sender<NormalizedEvent>,
    fast_tx: Sender<NormalizedEvent>,
    slow_tx: Sender<NormalizedEvent>,
    macro_tx: Sender<NormalizedEvent>,
    symbol: String,
    cancel: CancellationToken,
) {
    println!(
        "🔄 Event Router: Started for {} (fanning out to 4 timeframes)...",
        symbol
    );

    loop {
        let event = tokio::select! {
            biased;
            _ = cancel.cancelled() => {
                println!("🛑 Event Router: {} cancelled, shutting down.", symbol);
                break;
            }
            result = rx.recv() => {
                match result {
                    Some(e) => e,
                    None => {
                        println!("🛑 Event Router: {} upstream channel closed.", symbol);
                        break;
                    }
                }
            }
        };

        let _ = micro_tx.send(event.clone()).await;
        let _ = fast_tx.send(event.clone()).await;
        let _ = slow_tx.send(event.clone()).await;
        let _ = macro_tx.send(event).await;
    }
}

// DIE L3 median price filter (03-01-04 §4.1), owned by `network-adapters`
// (the DIE crate). See `crates/network-adapters/src/median_filter.rs`.
use network_adapters::median_filter::{FilterVerdict, MedianPriceFilter};

/// Venue REST coordinates for DIE L3 quarantine-refetch and runtime
/// gap-filling (03-01-04 §2.1.2 / §4.2). Built once per pipeline by the
/// registry from the active exchange choice.
#[derive(Clone)]
pub struct RestRefetchSpec {
    pub is_bitget: bool,
    /// Exchange-native symbol (e.g. Hyperliquid "BTC", Bitget "BTCUSDT").
    pub exchange_raw: String,
    /// Bitget product type ("" on other venues).
    pub product_type: String,
    pub rest_url: String,
}

/// Timeout for the rare quarantine/gap REST refetch so a venue stall can
/// never wedge the analysis loop.
const REFETCH_TIMEOUT_SECS: u64 = 5;

/// Fetch `[start_ms, end_ms)` candles of `duration_secs` from the venue REST
/// history. Returns an empty vector on error/timeout (callers treat that as
/// "gap remains open").
async fn fetch_interval_candles(
    spec: &RestRefetchSpec,
    internal_symbol: &str,
    start_ms: u64,
    end_ms: u64,
    duration_secs: u64,
) -> Vec<NormalizedCandle> {
    let fut = async {
        if spec.is_bitget {
            let interval =
                network_adapters::adapters::bitget_rest::timeframe_secs_to_interval(duration_secs);
            network_adapters::adapters::bitget_rest::fetch_historical_candles(
                &spec.exchange_raw,
                internal_symbol,
                &spec.product_type,
                interval,
                start_ms,
                end_ms,
                &spec.rest_url,
            )
            .await
        } else {
            let interval = network_adapters::adapters::hyperliquid_rest::timeframe_secs_to_interval(
                duration_secs,
            );
            network_adapters::adapters::hyperliquid_rest::fetch_historical_candles(
                &spec.exchange_raw,
                internal_symbol,
                interval,
                start_ms,
                end_ms,
                &spec.rest_url,
            )
            .await
        }
    };
    match tokio::time::timeout(std::time::Duration::from_secs(REFETCH_TIMEOUT_SECS), fut).await {
        Ok(Ok(candles)) => candles,
        Ok(Err(e)) => {
            eprintln!("⚠️  DIE L3: REST refetch failed for {}: {}", internal_symbol, e);
            Vec::new()
        }
        Err(_) => {
            eprintln!(
                "⚠️  DIE L3: REST refetch timed out after {}s for {}",
                REFETCH_TIMEOUT_SECS, internal_symbol
            );
            Vec::new()
        }
    }
}

/// Build the minimal completed `MarketSnapshot` used to transport a
/// gap-filled candle (08-04 §Forwarding). Reconstructed candles carry no
/// indicator payload — they exist so charts, persistence, and rollups see a
/// continuous candle series; indicator state resumes on the next live candle.
fn build_gapfill_snapshot(candle: &NormalizedCandle, symbol: &str, timeframe_secs: u64) -> MarketSnapshot {
    MarketSnapshot {
        exchange: Some(candle.exchange),
        timeframe_secs,
        timestamp: candle.start_time_ms / 1000,
        symbol: symbol.to_string(),
        is_completed: Some(true),
        mid_price: candle.close,
        bid_price: candle.close,
        ask_price: candle.close,
        bid_size: None,
        ask_size: None,
        funding_rate: None,
        open_interest: None,
        oi_delta_1h: None,
        mark_price: None,
        index_price: None,
        mark_index_spread_pct: None,
        prev_day_px: None,
        open: Some(candle.open),
        high: Some(candle.high),
        low: Some(candle.low),
        close: Some(candle.close),
        volume: Some(candle.volume),
        average_volume: None,
        context: None,
        decision_context: None,
        statistical_context: None,
        indicators: HashMap::new(),
        alignment: None,
        risk: None,
        analysis: None,
        advisory: None,
        opportunity: None,
        liquidity_signals: vec![],
        metrics_config: None,
        risk_profile: None,
        liquidity: None,
        cluster: None,
        quality_envelope: Some(CandleQualityEnvelope {
            quality_score: 100.0,
            is_valid: true,
            is_gap_filled: true,
            had_outliers_rejected: false,
            spike_detected: false,
            is_stale: false,
            sequence_integrity: SequenceIntegrity::Valid,
            gap_since_last: candle.duration_ms / 1000,
            validated_at: candle.start_time_ms + candle.duration_ms,
        }),
    }
}

pub async fn run_single(
    mut rx: Receiver<NormalizedEvent>,
    telemetry_tx: tokio::sync::mpsc::Sender<database_storage::TelemetryMsg>,
    broadcast_tx: broadcast::Sender<MarketSnapshot>,
    tf_config: TimeframeConfig,
    fib_config: FibonacciConfig,
    statistics_config: StatisticsConfig,
    divergence_detector: Arc<tokio::sync::Mutex<DivergenceDetector>>,
    history: Arc<RwLock<VecDeque<NormalizedCandle>>>,
    latest_snapshot: Arc<RwLock<Option<MarketSnapshot>>>,
    snapshot_history: Arc<RwLock<VecDeque<MarketSnapshot>>>,
    symbol: String,
    pair_key: String,
    timeframe_secs: u64,
    timeframe_label: &'static str,
    cancel: CancellationToken,
    candle_forward: Option<tokio::sync::mpsc::Sender<NormalizedCandle>>,
    warmed: Option<WarmedPipelineState>,
    paper_pool: Option<sqlx::SqlitePool>,
    latest_oi: Arc<RwLock<Option<Decimal>>>,
    latest_funding: Arc<RwLock<Option<Decimal>>>,
    latest_mark_px: Arc<RwLock<Option<Decimal>>>,
    latest_index_px: Arc<RwLock<Option<Decimal>>>,
    cluster_matrix: Arc<RwLock<Option<LiquidationClusterMatrix>>>,
    ob_config: OrderBookConfig,
    cross_tf_snapshot_a: Arc<RwLock<Option<MarketSnapshot>>>,
    cross_tf_snapshot_b: Arc<RwLock<Option<MarketSnapshot>>>,
    cross_tf_snapshot_c: Arc<RwLock<Option<MarketSnapshot>>>,
    latency_tracker: core_domain::SharedLatencyTracker,
    active_set: crate::active_set::ActiveSet,
    quality_config: Option<QualityConfig>,
    reliability: Arc<ReliabilityTracker>,
    refetch: Option<RestRefetchSpec>,
    quality_scope: Option<network_adapters::connection_quality_tracker::ConnectionQualityTracker>,
) {
    println!(
        "📊 Analysis Task: Started {} ({}) — {} ({})s candles{}...",
        symbol,
        pair_key,
        timeframe_label,
        tf_config.candles.duration_seconds,
        if warmed.is_some() {
            " [pre-warmed]"
        } else {
            ""
        }
    );

    let active_indicators = tf_config.indicators.clone();

    let (
        mut ema_fast,
        mut ema_medium,
        mut ema_slow,
        mut ema_long,
        mut rsi_14,
        mut macd,
        mut adx_14,
        mut sqz_mom,
        mut bollinger,
        mut atr_standalone,
        mut bbwp_indicator,
        mut stochastic_indicator,
        mut chandemo_indicator,
        mut supertrend_indicator,
        mut keltner_indicator,
        mut donchian_indicator,
        mut obv_indicator,
        mut cmf_indicator,
        mut mfi_indicator,
        mut hv_indicator,
        mut aroon_indicator,
        mut choppiness_indicator,
        mut linreg_indicator,
        mut zscore_indicator,
        mut stoch_div,
        mut chandemo_div,
        mut mfi_div,
        mut cmf_div,
        mut obv_div,
        mut squeeze_div,
        mut vwap_sum_tp_vol,
        mut vwap_sum_vol,
        mut last_day_index,
        mut volume_history,
        mut pivot_points_indicator,
        mut candlestick_indicator,
        mut ichimoku_indicator,
        mut cci_indicator,
        mut psar_indicator,
        mut wr_indicator,
        mut hma_indicator,
        mut ao_indicator,
        mut fi_indicator,
        mut sdc_indicator,
        mut volume_profile_indicator,
        mut smc_indicator,
        mut anchored_vwap_indicator,
    );

    // Strict chronological handover boundary: the start time of the newest
    // historical (REST/DB) candle used for pre-warming. Live candles at or
    // before this timestamp are discarded so partially-filled live wicks cannot
    // overwrite complete historical data or corrupt stateful indicators.
    // Defaults to 0 (no gate) for cold / sub-minute / non-warmed pipelines.
    let t_last_hist: u64 = warmed
        .as_ref()
        .and_then(|w| w.history.last().map(|c| c.start_time_ms))
        .unwrap_or(0);

    // Support/Resistance role-reversal tracker (flip tolerance 0.3%). Persists
    // across live bars; inherits warmed flip-state from the pre-warm pass.
    let mut sr_tracker = SrRoleTracker::new(0.003);

    // Statistical Intelligence Layer — per-timeframe engine.
    let mut sil_engine = StatisticsEngine::new(statistics_config);
    let mut prev_sil_close: f64 = 0.0;
    let mut mc_counter: u64 = 0;

    if let Some(w) = warmed {
        ema_fast = w.ema_fast;
        ema_medium = w.ema_medium;
        ema_slow = w.ema_slow;
        ema_long = w.ema_long;
        rsi_14 = w.rsi_14;
        macd = w.macd;
        adx_14 = w.adx_14;
        sqz_mom = w.sqz_mom;
        bollinger = w.bollinger;
        atr_standalone = w.atr_standalone;
        bbwp_indicator = w.bbwp_indicator;
        stochastic_indicator = w.stochastic_indicator;
        chandemo_indicator = w.chandemo_indicator;
        supertrend_indicator = w.supertrend_indicator;
        keltner_indicator = w.keltner_indicator;
        donchian_indicator = w.donchian_indicator;
        obv_indicator = w.obv_indicator;
        cmf_indicator = w.cmf_indicator;
        mfi_indicator = w.mfi_indicator;
        hv_indicator = w.hv_indicator;
        aroon_indicator = w.aroon_indicator;
        choppiness_indicator = w.choppiness_indicator;
        linreg_indicator = w.linreg_indicator;
        zscore_indicator = w.zscore_indicator;
        stoch_div = w.stoch_div;
        chandemo_div = w.chandemo_div;
        mfi_div = w.mfi_div;
        cmf_div = w.cmf_div;
        obv_div = w.obv_div;
        squeeze_div = w.squeeze_div;
        vwap_sum_tp_vol = w.vwap_sum_tp_vol;
        vwap_sum_vol = w.vwap_sum_vol;
        last_day_index = w.last_day_index;
        volume_history = w.volume_history;
        sr_tracker = w.sr_tracker;
        pivot_points_indicator = w.pivot_points_indicator;
        candlestick_indicator = w.candlestick_indicator;
        ichimoku_indicator = w.ichimoku_indicator;
        cci_indicator = w.cci_indicator;
        psar_indicator = w.psar_indicator;
        wr_indicator = w.wr_indicator;
        hma_indicator = w.hma_indicator;
        ao_indicator = w.ao_indicator;
        fi_indicator = w.fi_indicator;
        sdc_indicator = w.sdc_indicator;
        volume_profile_indicator = w.volume_profile_indicator;
        smc_indicator = w.smc_indicator;
        anchored_vwap_indicator = w.anchored_vwap_indicator;

        // Pre-populate history from warmed state
        {
            let mut hist = history.write().await;
            for c in &w.history {
                hist.push_back(c.clone());
            }
        }
        // Pre-populate latest_snapshot from warmed state
        if let Some(ref snap) = w.latest_snapshot {
            *latest_snapshot.write().await = Some(snap.clone());
        }
        // Pre-populate snapshot_history from warmed state
        {
            let mut snap_hist = snapshot_history.write().await;
            for snap in &w.snapshot_history {
                snap_hist.push_back(snap.clone());
            }
        }
        // Pre-populate divergence detector state
        {
            let mut det = divergence_detector.lock().await;
            *det = w.divergence_detector.clone();
        }
    } else {
        ema_fast = Ema::new(active_indicators.ema_fast);
        ema_medium = Ema::new(active_indicators.ema_medium);
        ema_slow = Ema::new(active_indicators.ema_slow);
        ema_long = Ema::new(active_indicators.ema_long);
        rsi_14 = Rsi::new(active_indicators.rsi_period);
        macd = Macd::new();
        adx_14 = Adx::new(active_indicators.adx_period);
        adx_14.set_thresholds(
            Decimal::from(active_indicators.adx_trend_threshold),
            Decimal::from(active_indicators.adx_exhaustion_threshold),
            active_indicators.adx_slope_lookback,
        );
        sqz_mom = SqueezeMomentum::new(active_indicators.squeeze_period);
        sqz_mom.set_min_duration(active_indicators.squeeze_min_duration);
        bollinger = BollingerBands::new(20);
        atr_standalone = Atr::new(active_indicators.atr_period);
        bbwp_indicator = Bbwp::new(
            active_indicators.bbwp_lookback,
            active_indicators.bbwp_period,
        );
        stochastic_indicator = Stochastic::new(
            active_indicators.stoch_k_period,
            active_indicators.stoch_d_period,
            active_indicators.stoch_s_period,
        );
        chandemo_indicator = ChandeMO::new(active_indicators.chandemo_period);
        supertrend_indicator = Supertrend::new(
            active_indicators.supertrend_period,
            active_indicators.supertrend_multiplier,
        );
        keltner_indicator = Keltner::new(
            active_indicators.keltner_ema_period,
            active_indicators.keltner_atr_period,
            active_indicators.keltner_multiplier,
        );
        donchian_indicator = Donchian::new(active_indicators.donchian_period);
        obv_indicator = Obv::new(active_indicators.obv_smoothing);
        cmf_indicator = Cmf::new(active_indicators.cmf_period);
        mfi_indicator = Mfi::new(active_indicators.mfi_period);
        hv_indicator = HistoricalVolatility::new(active_indicators.hv_period);
        aroon_indicator = Aroon::new(active_indicators.aroon_period);
        choppiness_indicator = Choppiness::new(active_indicators.chop_period);
        linreg_indicator = LinRegSlope::new(active_indicators.linreg_period);
        zscore_indicator = ZScore::new(active_indicators.zscore_period);
        stoch_div = SeriesDivergence::new(20);
        chandemo_div = SeriesDivergence::new(20);
        mfi_div = SeriesDivergence::new(20);
        cmf_div = SeriesDivergence::new(20);
        obv_div = SeriesDivergence::new(20);
        squeeze_div = SeriesDivergence::new(20);
        vwap_sum_tp_vol = Decimal::ZERO;
        vwap_sum_vol = Decimal::ZERO;
        last_day_index = None;
        volume_history = VecDeque::with_capacity(20);
        pivot_points_indicator = PivotPoints::new(PivotMethod::Classic);
        candlestick_indicator = Candlestick::new(CandlestickConfig::default());
        ichimoku_indicator = Ichimoku::new(
            active_indicators.ichimoku_tenkan,
            active_indicators.ichimoku_kijun,
            active_indicators.ichimoku_senkou_b,
            active_indicators.ichimoku_displacement,
        );
        cci_indicator = Cci::new(active_indicators.cci_period);
        psar_indicator = ParabolicSar::new(
            active_indicators.psar_af_step,
            active_indicators.psar_af_max,
        );
        wr_indicator = WilliamsR::new(active_indicators.williams_r_period);
        hma_indicator = HullMA::new(active_indicators.hull_ma_period);
        ao_indicator = AwesomeOscillator::new();
        fi_indicator = ForceIndex::new(active_indicators.force_index_smoothing);
        sdc_indicator = StdDevChannel::new(active_indicators.stddev_channel_period);
        volume_profile_indicator = VolumeProfile::new(
            active_indicators.volume_profile_window,
            active_indicators.volume_profile_bins,
            active_indicators.volume_profile_value_area,
        );
        smc_indicator = SmartMoney::new(active_indicators.smc_lookback);
        anchored_vwap_indicator = AnchoredVwap::new();
    }

    // ADX slope history for the 2-bar consecutive-deceleration hook exit.
    let mut adx_slope_history: VecDeque<Decimal> = VecDeque::with_capacity(3);

    // Signal-age tracker: maps "<indicator>:<kind>" → (first-seen bar, direction).
    // Stamps `age_bars` on each completed snapshot's signals. Live-only (resets
    // on warm handover, which is acceptable — historical bars aren't decisions).
    let mut signal_age_tracker: std::collections::HashMap<
        String,
        (u32, crate::indicators::SignalDirection),
    > = std::collections::HashMap::new();
    let mut live_bar: u32 = 0;
    let mut prev_bar_state = PreviousBarState::default();
    let mut last_pivot_count: usize = 0;
    let mut last_cascade_state: core_domain::liquidity::CascadeState =
        core_domain::liquidity::CascadeState::None;
    let mut prev_mtf_score: Option<f64> = None;
    let mut prev_regime: Option<core_domain::analysis::MarketRegime> = None;

    // OI delta tracking: rolling 1-hour window of OI values (60 × 60s candles).
    let mut oi_history: VecDeque<f64> = VecDeque::with_capacity(60);

    // Phase 1: real liquidation event accumulator. Per-candle aggregation
    // produces a `LiquidityFlow` on every completed bar.
    let mut liquidity_acc = core_domain::liquidity::LiquidityEventAccumulator::new(&symbol);

    let mut candle_gen = CandleGenerator::new(&symbol, tf_config.candles.duration_seconds, Exchange::Hyperliquid);

    let mut median_filter = quality_config.as_ref().map(MedianPriceFilter::new);

    let staleness_threshold_ms = quality_config
        .as_ref()
        .map(|q| q.staleness_threshold_secs * 1000)
        .unwrap_or(600_000);

    #[allow(unused_assignments)]
    let mut last_trade_ts_ms: u64 = 0;

    // DIE L3 runtime sequence audit + gap-fill state (03-01-04 §3 / §2.1.2).
    let duration_ms = tf_config.candles.duration_seconds * 1000;
    let mut last_completed_start_ms: Option<u64> = (t_last_hist > 0).then_some(t_last_hist);
    let mut outliers_at_prev_candle: u32 = 0;
    let reconstructor = network_adapters::adapters::reconstruction::CandleReconstructor::new();
    // Cap the number of bars filled per detected hole so a multi-hour outage
    // cannot flood the pipeline (larger recoveries belong to the bootstrap path).
    const MAX_GAP_FILL_BARS: u64 = 60;

    let mut order_book_analysis =
        OrderBookAnalysis::new(ob_config.depth_levels, ob_config.wall_threshold);
    let spread_wide_threshold_pct = ob_config.spread_wide_threshold_pct;

    let mut shadow_bid = Decimal::ZERO;
    let mut shadow_ask = Decimal::ZERO;
    #[allow(unused_assignments)]
    let mut shadow_exchange: Option<Exchange> = None;
    let mut shadow_prev_day_px: Option<Decimal> = None;

    loop {
        let event = tokio::select! {
            biased;
            _ = cancel.cancelled() => {
                println!("🛑 Analysis Task: {} ({}) cancelled, shutting down.", symbol, timeframe_label);
                break;
            }
            result = rx.recv() => {
                match result {
                    Some(e) => e,
                    None => {
                        println!("🛑 Analysis Task: {} ({}) channel closed.", symbol, timeframe_label);
                        break;
                    }
                }
            }
        };

        {
            let mut hist = history.write().await;
            while hist.len() > HIST_BUFFER_MAX {
                hist.pop_front();
            }
        }

        match event {
            NormalizedEvent::Trade(ref trade) => {
                shadow_exchange = Some(trade.exchange);
                candle_gen.set_exchange(trade.exchange);

                latency_tracker.record_ingest_skew(
                    core_domain::LatencyTracker::now_ms(),
                    trade.timestamp_ms,
                );

                if candle_gen.is_late_tick(trade.timestamp_ms) {
                    reliability.increment_out_of_order(1).await;
                    continue;
                }

                last_trade_ts_ms = trade.timestamp_ms;

                let trade_price_f = trade.price.to_f64().unwrap_or(0.0);
                let verdict = if let Some(ref mut filter) = median_filter {
                    filter.evaluate(trade_price_f)
                } else {
                    FilterVerdict::Accepted
                };
                match verdict {
                    FilterVerdict::Rejected => {
                        reliability.increment_outliers(1).await;
                        continue;
                    }
                    FilterVerdict::Bypassed => {
                        reliability.increment_bypassed(1).await;
                        eprintln!(
                            "🔍 DIE L3 [{} {}]: median = 0 (venue reset) — filter bypassed for tick at price {}",
                            symbol, timeframe_label, trade_price_f
                        );
                    }
                    FilterVerdict::Accepted => {}
                }

                let (completed_opt, live_candle) = candle_gen.process_trade(trade);
                let mut completed_opt = completed_opt.filter(|c| c.start_time_ms > t_last_hist);

                // ── DIE L3 §4.2: quarantine + REST refetch on validity failure.
                // An invalid candle never reaches L4; a REST replacement is
                // attempted for its interval, and if none validates the slot
                // stays open and is counted as a gap.
                if let Some(ref candidate) = completed_opt {
                    if let Err(reason) = candidate.assert_validity() {
                        let _ = telemetry_tx
                            .send(database_storage::TelemetryMsg::ConsoleLog(format!(
                                "DIE L3: validity check failed for {}/{} candle at {} — quarantined ({})",
                                symbol, timeframe_label, candidate.start_time_ms, reason
                            )))
                            .await;
                        let mut replacement: Option<NormalizedCandle> = None;
                        if tf_config.candles.duration_seconds >= 60 {
                            if let Some(ref spec) = refetch {
                                let refetched = fetch_interval_candles(
                                    spec,
                                    &symbol,
                                    candidate.start_time_ms,
                                    candidate.start_time_ms + duration_ms,
                                    tf_config.candles.duration_seconds,
                                )
                                .await;
                                replacement = refetched
                                    .into_iter()
                                    .find(|c| {
                                        c.start_time_ms == candidate.start_time_ms
                                            && c.assert_validity().is_ok()
                                    })
                                    .map(|mut c| {
                                        c.reconstructed = Some(
                                            core_domain::normalized::ReconstructionMethod::ExchangeHistorical,
                                        );
                                        c
                                    });
                            }
                        }
                        match replacement {
                            Some(good) => {
                                reliability.increment_reconstructed(1).await;
                                if let Some(ref cq) = quality_scope {
                                    cq.record_reconstructed_candle().await;
                                }
                                completed_opt = Some(good);
                            }
                            None => {
                                reliability.increment_gaps(1).await;
                                completed_opt = None;
                            }
                        }
                    }
                }

                // ── DIE L3 §3: runtime missing-bar sequence audit. A hole
                // between consecutive completed candles flags a gap and
                // triggers recovery: REST for ≥1m tiers, EMA/linear synthesis
                // for sub-minute tiers (03-01-04 §2.1.2). Ticks arriving while
                // recovery runs queue in the pipeline channel, so indicator
                // state is not touched until reconstruction completes.
                if let Some(ref completed) = completed_opt {
                    if let Some(prev_start) = last_completed_start_ms {
                        let expected_start = prev_start + duration_ms;
                        if completed.start_time_ms > expected_start {
                            let missing =
                                (completed.start_time_ms - expected_start) / duration_ms;
                            let fill_n = missing.min(MAX_GAP_FILL_BARS);
                            reliability.increment_gaps(missing as u32).await;
                            eprintln!(
                                "🕳️  DIE L3 [{} {}]: {} missing bar(s) detected before {} — recovering {}",
                                symbol, timeframe_label, missing, completed.start_time_ms, fill_n
                            );

                            let mut filled: Vec<NormalizedCandle> = Vec::new();
                            if tf_config.candles.duration_seconds >= 60 {
                                if let Some(ref spec) = refetch {
                                    let fetched = fetch_interval_candles(
                                        spec,
                                        &symbol,
                                        completed.start_time_ms
                                            .saturating_sub(fill_n * duration_ms),
                                        completed.start_time_ms,
                                        tf_config.candles.duration_seconds,
                                    )
                                    .await;
                                    filled = fetched
                                        .into_iter()
                                        .filter(|c| {
                                            c.start_time_ms >= expected_start
                                                && c.start_time_ms < completed.start_time_ms
                                                && c.assert_validity().is_ok()
                                        })
                                        .map(|mut c| {
                                            c.reconstructed = Some(
                                                core_domain::normalized::ReconstructionMethod::ExchangeHistorical,
                                            );
                                            c
                                        })
                                        .collect();
                                }
                            } else {
                                let recent_closes: Vec<f64> = {
                                    let hist = history.read().await;
                                    hist.iter()
                                        .filter_map(|c| c.close.to_f64())
                                        .collect()
                                };
                                let fill_start =
                                    completed.start_time_ms - fill_n * duration_ms;
                                for i in 0..fill_n {
                                    let s = fill_start + i * duration_ms;
                                    if s < expected_start {
                                        continue;
                                    }
                                    if let Some(rc) = reconstructor.reconstruct(
                                        completed.exchange,
                                        s,
                                        s + duration_ms,
                                        duration_ms,
                                        &recent_closes,
                                    ) {
                                        let mut c = rc.candle;
                                        c.symbol = symbol.clone();
                                        filled.push(c);
                                    }
                                }
                            }

                            for gap_candle in filled {
                                reliability.increment_reconstructed(1).await;
                                if let Some(ref cq) = quality_scope {
                                    cq.record_reconstructed_candle().await;
                                }
                                {
                                    let mut hist = history.write().await;
                                    hist.push_back(gap_candle.clone());
                                }
                                if let Some(ref fwd) = candle_forward {
                                    let _ = fwd.send(gap_candle.clone()).await;
                                }
                                let gap_snapshot = build_gapfill_snapshot(
                                    &gap_candle,
                                    &symbol,
                                    timeframe_secs,
                                );
                                let _ = telemetry_tx
                                    .send(database_storage::TelemetryMsg::InsertSnapshot(
                                        gap_snapshot.clone(),
                                    ))
                                    .await;
                                let _ = broadcast_tx.send(gap_snapshot);
                            }
                        }
                    }
                }

                if let Some(completed) = completed_opt {
                    last_completed_start_ms = Some(completed.start_time_ms);
                    reliability.increment_candles(1).await;

                    let is_valid = completed.assert_validity().is_ok();

                    let is_reconstructed = completed.reconstructed.is_some();
                    let rejected_total = median_filter
                        .as_ref()
                        .map(|f| f.outliers_rejected())
                        .unwrap_or(0);
                    let had_outliers_this_candle = rejected_total > outliers_at_prev_candle;
                    outliers_at_prev_candle = rejected_total;

                    let now_ms = core_domain::LatencyTracker::now_ms();
                    let candle_close_ms = completed.start_time_ms + duration_ms;
                    let candle_stale = last_trade_ts_ms > 0
                        && candle_close_ms.saturating_sub(last_trade_ts_ms) > staleness_threshold_ms;

                    let gap_secs = match last_completed_start_ms {
                        Some(_prev) => {
                            let gap_ms = completed.start_time_ms.saturating_sub(
                                last_completed_start_ms.unwrap_or(completed.start_time_ms),
                            );
                            gap_ms / 1000
                        }
                        None => duration_ms / 1000,
                    };

                    let quality_score = if !is_valid {
                        0.0
                    } else {
                        let mut score = 100.0_f64;
                        if is_reconstructed {
                            score -= 20.0;
                        }
                        if had_outliers_this_candle {
                            score -= 10.0;
                        }
                        if candle_stale {
                            score -= 30.0;
                        }
                        score.clamp(0.0, 100.0)
                    };

                    let quality_envelope = CandleQualityEnvelope {
                        quality_score,
                        is_valid,
                        is_gap_filled: is_reconstructed,
                        had_outliers_rejected: had_outliers_this_candle,
                        spike_detected: had_outliers_this_candle,
                        is_stale: candle_stale,
                        sequence_integrity: SequenceIntegrity::Valid,
                        gap_since_last: gap_secs,
                        validated_at: now_ms,
                    };
                    let candle_close_sec = completed.start_time_ms / 1000;
                    let day_index = candle_close_sec / 86400;
                    if let Some(prev_day) = last_day_index {
                        if day_index > prev_day {
                            vwap_sum_tp_vol = Decimal::ZERO;
                            vwap_sum_vol = Decimal::ZERO;
                        }
                    }
                    last_day_index = Some(day_index);

                    // Session Pivot Points: accumulate this session's H/L/C and
                    // recompute levels on UTC-day rollover.
                    let pivot_levels = pivot_points_indicator.update(
                        completed.high,
                        completed.low,
                        completed.close,
                        day_index,
                    );

                    // Candlestick recognition (Stage 1 geometry + Stage 3 confirm).
                    let candlestick_reading = candlestick_indicator.update(
                        completed.open,
                        completed.high,
                        completed.low,
                        completed.close,
                    );

                    // Ichimoku Cloud (Tenkan/Kijun/Senkou A/B/Chikou).
                    let ichimoku_reading =
                        ichimoku_indicator.update(completed.high, completed.low, completed.close);

                    // CCI (Commodity Channel Index).
                    let cci_reading =
                        cci_indicator.update(completed.high, completed.low, completed.close);

                    // Parabolic SAR.
                    let psar_reading = psar_indicator.update(completed.high, completed.low);

                    let wr_reading =
                        wr_indicator.update(completed.high, completed.low, completed.close);
                    let hma_reading = hma_indicator.update(completed.close);
                    let ao_reading = ao_indicator.update(completed.high, completed.low);
                    let fi_reading = fi_indicator.update(completed.close, completed.volume);
                    let sdc_reading = sdc_indicator.update(completed.close);

                    let volume_profile_reading = volume_profile_indicator.update(
                        completed.high,
                        completed.low,
                        completed.close,
                        completed.volume,
                    );
                    let smc_reading = smc_indicator.update(
                        completed.open,
                        completed.high,
                        completed.low,
                        completed.close,
                    );

                    let typical_price =
                        (completed.high + completed.low + completed.close) / Decimal::from(3);
                    vwap_sum_tp_vol += typical_price * completed.volume;
                    vwap_sum_vol += completed.volume;

                    let final_vwap = if vwap_sum_vol > Decimal::ZERO {
                        Some(vwap_sum_tp_vol / vwap_sum_vol)
                    } else {
                        None
                    };

                    let avwap_reading = anchored_vwap_indicator.update(
                        completed.high,
                        completed.low,
                        completed.close,
                        completed.volume,
                        day_index,
                        final_vwap.unwrap_or(Decimal::ZERO),
                    );

                    let final_ema_fast = ema_fast.update(completed.close);
                    let final_ema_medium = ema_medium.update(completed.close);
                    let final_ema_slow = ema_slow.update(completed.close);
                    let final_ema_long = ema_long.update(completed.close);

                    let ema_stack_state = if final_ema_fast > final_ema_medium
                        && final_ema_medium > final_ema_slow
                        && final_ema_slow > final_ema_long
                        && completed.close > final_ema_fast
                    {
                        Some("bullish".to_string())
                    } else if final_ema_fast < final_ema_medium
                        && final_ema_medium < final_ema_slow
                        && final_ema_slow < final_ema_long
                        && completed.close < final_ema_fast
                    {
                        Some("bearish".to_string())
                    } else {
                        Some("tangled".to_string())
                    };

                    let final_rsi = rsi_14.update(completed.close);
                    let final_macd = macd.update(completed.close);
                    let final_adx = adx_14.update(completed.high, completed.low, completed.close);
                    let final_sqz = sqz_mom.update(completed.high, completed.low, completed.close);
                    let final_bb = bollinger.update(completed.close);
                    let final_atr =
                        atr_standalone.update(completed.high, completed.low, completed.close);
                    let final_bbwp = bbwp_indicator.update(completed.close);
                    let final_stoch =
                        stochastic_indicator.update(completed.high, completed.low, completed.close);
                    let final_cmo = chandemo_indicator.update(completed.close);
                    let final_supertrend =
                        supertrend_indicator.update(completed.high, completed.low, completed.close);
                    let final_keltner =
                        keltner_indicator.update(completed.high, completed.low, completed.close);
                    let final_donchian = donchian_indicator.update(completed.high, completed.low);
                    let final_obv = obv_indicator.update(completed.close, completed.volume);
                    let final_cmf = cmf_indicator.update(
                        completed.high,
                        completed.low,
                        completed.close,
                        completed.volume,
                    );
                    let final_mfi = mfi_indicator.update(
                        completed.high,
                        completed.low,
                        completed.close,
                        completed.volume,
                    );
                    let final_hv = hv_indicator.update(completed.close);
                    let final_aroon = aroon_indicator.update(completed.high, completed.low);
                    let final_chop =
                        choppiness_indicator.update(completed.high, completed.low, completed.close);
                    let final_linreg = linreg_indicator.update(completed.close);
                    let final_zscore = zscore_indicator.update(completed.close);

                    // ── Generalized divergence detection ──
                    // Each oscillator's SeriesDivergence is updated every bar
                    // for the RSI/MACD confirmation path below. The 6 extra
                    // divergence states are resolved inline inside NormalizeParams
                    // (below) so sr_supports/resistances are in scope for the
                    // S/R-confirmation gate.

                    // Divergence detection (live — potential status; confirmation
                    // applied after S/R levels are computed below).
                    let mut div_result = {
                        if let (Some(rsi), macd_hist) = (final_rsi, final_macd.histogram) {
                            divergence_detector.lock().await.update_full(
                                completed.close,
                                rsi,
                                macd_hist,
                            )
                        } else {
                            crate::indicators::DivergenceResult::default_div()
                        }
                    };

                    let log_line = format!(
                        "🕯️  [{}] {} Candle Closed | Start: {} | Close: ${:.4} | Vol: {:.4} | Trades: {}",
                        symbol, timeframe_label, completed.start_time_ms, completed.close,
                        completed.volume, completed.trades_count
                    );
                    let _ = telemetry_tx
                        .send(database_storage::TelemetryMsg::ConsoleLog(log_line))
                        .await;

                    volume_history.push_back(completed.volume);
                    if volume_history.len() > 20 {
                        volume_history.pop_front();
                    }
                    let avg_vol = if !volume_history.is_empty() {
                        let sum: Decimal = volume_history.iter().sum();
                        Some(sum / Decimal::from(volume_history.len()))
                    } else {
                        None
                    };

                    let rvol = match (completed.volume, avg_vol) {
                        (vol, Some(avg)) if avg > Decimal::ZERO => Some(vol / avg),
                        _ => None,
                    };

                    // Fibonacci retracement/extension computation
                    let fib = {
                        let hist = history.read().await;
                        let candles_high: Vec<Decimal> = hist.iter().map(|c| c.high).collect();
                        let candles_low: Vec<Decimal> = hist.iter().map(|c| c.low).collect();
                        FibonacciRange::compute_from_candles(
                            &candles_high,
                            &candles_low,
                            fib_config.swing_lookback,
                            fib_config.swing_scan_range,
                            &fib_config.retracement_coefficients,
                            &fib_config.extension_coefficients,
                        )
                    };

                    // Chart pattern detection from pivots (reused for S/R zones)
                    let pivots = {
                        let hist = history.read().await;
                        let candles_high: Vec<Decimal> = hist.iter().map(|c| c.high).collect();
                        let candles_low: Vec<Decimal> = hist.iter().map(|c| c.low).collect();
                        FibonacciRange::detect_pivots(
                            &candles_high,
                            &candles_low,
                            fib_config.swing_lookback,
                            fib_config.swing_scan_range,
                        )
                    };
                    if pivots.len() > last_pivot_count {
                        anchored_vwap_indicator.reset_swing();
                    }
                    last_pivot_count = pivots.len();
                    let pattern_result = detect_pattern(&pivots);

                    // Support/Resistance zones: derive role-adjusted levels from
                    // the swing pivots and update the flip tracker on this close.
                    let (sr_supports, sr_resistances) = update_sr_levels(
                        &mut sr_tracker,
                        &pivots,
                        completed.close,
                        candle_close_sec,
                    );

                    // Upgrade RSI/MACD potential divergences to Confirmed when
                    // the candle close decisively breaks the nearest S/R level.
                    // check_divergence_confirmation is a &self method on the
                    // DivergenceDetector — we lock it again briefly.
                    {
                        let near_sup = sr_supports
                            .iter()
                            .copied()
                            .filter(|s| *s > 0.0 && *s <= completed.close.to_f64().unwrap_or(0.0))
                            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                        let near_res = sr_resistances
                            .iter()
                            .copied()
                            .filter(|r| *r > 0.0 && *r >= completed.close.to_f64().unwrap_or(0.0))
                            .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                        if near_sup.is_some() || near_res.is_some() {
                            let det = divergence_detector.lock().await;
                            div_result = det.check_divergence_confirmation(
                                &div_result,
                                completed.close,
                                near_sup
                                    .map(|s| Decimal::from_f64_retain(s).unwrap_or(Decimal::ZERO)),
                                near_res
                                    .map(|r| Decimal::from_f64_retain(r).unwrap_or(Decimal::ZERO)),
                            );
                        }
                    }

                    // Track ADX slope history for the 2-bar hook-exit rule.
                    if let Some(a) = final_adx.as_ref() {
                        adx_slope_history.push_back(a.adx_slope);
                        while adx_slope_history.len() > 3 {
                            adx_slope_history.pop_front();
                        }
                    }
                    let adx_consecutive_deceleration = adx_slope_history.len() >= 2
                        && adx_slope_history
                            .iter()
                            .rev()
                            .take(2)
                            .all(|s| *s < Decimal::ZERO);

                    // Active position context for direction-aware normalization.
                    let active_position: Option<i8> = if let Some(ref pool) = paper_pool {
                        match database_storage::paper::queries::paper_get_active_position(pool, &symbol)
                            .await
                            .map(|p| p.direction)
                            .as_deref()
                        {
                            Some("LONG") => Some(1),
                            Some("SHORT") => Some(-1),
                            _ => Some(0),
                        }
                    } else {
                        Some(0)
                    };

                    let ema_stack_str = ema_stack_state.as_deref();
                    let indicators = normalize::build_indicator_map(normalize::NormalizeParams {
                        close: completed.close,
                        rsi: final_rsi,
                        rsi_divergence: normalize::rsi_divergence_state(&div_result),
                        macd_divergence: normalize::macd_divergence_state(&div_result),
                        stoch_k: final_stoch.as_ref().map(|s| s.k_value),
                        stoch_d: final_stoch.as_ref().map(|s| s.d_value),
                        chandemo: final_cmo,
                        supertrend_line: final_supertrend.as_ref().map(|s| s.line),
                        supertrend_dir: final_supertrend.as_ref().map(|s| s.direction),
                        keltner: final_keltner.as_ref().map(|k| (k.upper, k.middle, k.lower)),
                        donchian: final_donchian
                            .as_ref()
                            .map(|d| (d.upper, d.middle, d.lower)),
                        obv: final_obv.as_ref().map(|o| o.obv),
                        obv_sma: final_obv.as_ref().map(|o| o.obv_sma),
                        cmf: final_cmf,
                        mfi: final_mfi,
                        hv: final_hv,
                        aroon_up: final_aroon.as_ref().map(|a| a.up),
                        aroon_down: final_aroon.as_ref().map(|a| a.down),
                        choppiness: final_chop,
                        linreg_slope: final_linreg,
                        zscore: final_zscore,
                        extra_div: normalize::ExtraDivergence {
                            stochastic: final_stoch
                                .as_ref()
                                .map(|s| {
                                    normalize::series_divergence_confirmed(
                                        &stoch_div.update(completed.close, s.k_value),
                                        completed.close,
                                        &sr_supports,
                                        &sr_resistances,
                                    )
                                })
                                .unwrap_or_default(),
                            chandemo: final_cmo
                                .map(|v| {
                                    normalize::series_divergence_confirmed(
                                        &chandemo_div.update(completed.close, v),
                                        completed.close,
                                        &sr_supports,
                                        &sr_resistances,
                                    )
                                })
                                .unwrap_or_default(),
                            mfi: final_mfi
                                .map(|v| {
                                    normalize::series_divergence_confirmed(
                                        &mfi_div.update(completed.close, v),
                                        completed.close,
                                        &sr_supports,
                                        &sr_resistances,
                                    )
                                })
                                .unwrap_or_default(),
                            cmf: final_cmf
                                .map(|v| {
                                    normalize::series_divergence_confirmed(
                                        &cmf_div.update(completed.close, v),
                                        completed.close,
                                        &sr_supports,
                                        &sr_resistances,
                                    )
                                })
                                .unwrap_or_default(),
                            obv: final_obv
                                .as_ref()
                                .map(|o| {
                                    normalize::series_divergence_confirmed(
                                        &obv_div.update(completed.close, o.obv),
                                        completed.close,
                                        &sr_supports,
                                        &sr_resistances,
                                    )
                                })
                                .unwrap_or_default(),
                            squeeze: final_sqz
                                .as_ref()
                                .map(|s| {
                                    normalize::series_divergence_confirmed(
                                        &squeeze_div.update(completed.close, s.momentum_value),
                                        completed.close,
                                        &sr_supports,
                                        &sr_resistances,
                                    )
                                })
                                .unwrap_or_default(),
                        },
                        macd: &final_macd,
                        sqz: final_sqz.as_ref(),
                        adx: final_adx.as_ref(),
                        bb: final_bb,
                        atr: final_atr.as_ref(),
                        bbwp: final_bbwp,
                        vwap: final_vwap,
                        anchored_vwap: Some(avwap_reading),
                        ema_stack_state: ema_stack_str,
                        ema_fast: Some(final_ema_fast),
                        ema_medium: Some(final_ema_medium),
                        ema_slow: Some(final_ema_slow),
                        ema_long: Some(final_ema_long),
                        rvol,
                        volume: Some(completed.volume),
                        average_volume: avg_vol,
                        fib: Some(&fib),
                        pattern: Some(&pattern_result),
                        support_levels: &sr_supports,
                        resistance_levels: &sr_resistances,
                        active_position,
                        adx_consecutive_deceleration,
                        supertrend_flipped: final_supertrend
                            .as_ref()
                            .map(|s| s.flipped)
                            .unwrap_or(false),
                        adx_di_crossover: final_adx.as_ref().and_then(|a| {
                            a.di_crossover.map(|c| match c {
                                crate::indicators::DiCrossoverDir::Bullish => 1i8,
                                crate::indicators::DiCrossoverDir::Bearish => -1i8,
                            })
                        }),
                        pivot_levels,
                        pivot_proximity_pct: 0.0015,
                        candlestick: Some(candlestick_reading),
                        candlestick_min_confidence: 0.3,
                        ichimoku: ichimoku_reading,
                        cci: cci_reading,
                        psar: psar_reading,
                        williams_r: wr_reading,
                        awesome_oscillator: ao_reading,
                        force_index: fi_reading,
                        hull_ma: hma_reading,
                        stddev_channel: sdc_reading,
                        volume_profile: volume_profile_reading,
                        smc: smc_reading,
                        prev: prev_bar_state,
                    });

                    // Read derivative state for prev_bar_state snapshot.
                    let prev_fund_f = latest_funding.read().await.and_then(|f| f.to_f64());

                    // ── Save current bar's indicator values for next bar's cross-over detection ──
                    prev_bar_state = PreviousBarState {
                        rsi: final_rsi.map(|d| d.to_f64().unwrap_or(0.0)),
                        stoch_k: final_stoch
                            .as_ref()
                            .map(|s| s.k_value.to_f64().unwrap_or(0.0)),
                        stoch_d: final_stoch
                            .as_ref()
                            .map(|s| s.d_value.to_f64().unwrap_or(0.0)),
                        cmf: final_cmf.map(|d| d.to_f64().unwrap_or(0.0)),
                        chandemo: final_cmo.map(|d| d.to_f64().unwrap_or(0.0)),
                        aroon_up: final_aroon.as_ref().map(|a| a.up.to_f64().unwrap_or(0.0)),
                        aroon_down: final_aroon.as_ref().map(|a| a.down.to_f64().unwrap_or(0.0)),
                        macd_line: Some(final_macd.macd_line.to_f64().unwrap_or(0.0)),
                        linreg_slope: final_linreg.map(|d| d.to_f64().unwrap_or(0.0)),
                        zscore: final_zscore.map(|d| d.to_f64().unwrap_or(0.0)),
                        obv: final_obv.as_ref().map(|o| o.obv.to_f64().unwrap_or(0.0)),
                        obv_sma: final_obv
                            .as_ref()
                            .map(|o| o.obv_sma.to_f64().unwrap_or(0.0)),
                        mfi: final_mfi.map(|d| d.to_f64().unwrap_or(0.0)),
                        adx_plus_di: final_adx
                            .as_ref()
                            .map(|a| a.plus_di.to_f64().unwrap_or(0.0)),
                        adx_minus_di: final_adx
                            .as_ref()
                            .map(|a| a.minus_di.to_f64().unwrap_or(0.0)),
                        price: Some(completed.close.to_f64().unwrap_or(0.0)),
                        ema_fast: Some(final_ema_fast.to_f64().unwrap_or(0.0)),
                        ema_medium: Some(final_ema_medium.to_f64().unwrap_or(0.0)),
                        supertrend_line: final_supertrend
                            .as_ref()
                            .map(|s| s.line.to_f64().unwrap_or(0.0)),
                        // Populated in later phases (Pivots: P2, Ichimoku: P4).
                        pivot_active_level: pivot_levels.map(|lv| {
                            let p = lv.pivot.to_f64().unwrap_or(0.0);
                            let c = completed.close.to_f64().unwrap_or(0.0);
                            if c >= p {
                                1.0
                            } else {
                                -1.0
                            }
                        }),
                        ichimoku_tenkan: ichimoku_reading.map(|r| r.tenkan.to_f64().unwrap_or(0.0)),
                        ichimoku_kijun: ichimoku_reading.map(|r| r.kijun.to_f64().unwrap_or(0.0)),
                        price_vs_cloud: ichimoku_reading.map(|r| {
                            let top = r
                                .senkou_a_current
                                .to_f64()
                                .unwrap_or(0.0)
                                .max(r.senkou_b_current.to_f64().unwrap_or(0.0));
                            let bot = r
                                .senkou_a_current
                                .to_f64()
                                .unwrap_or(0.0)
                                .min(r.senkou_b_current.to_f64().unwrap_or(0.0));
                            let px = completed.close.to_f64().unwrap_or(0.0);
                            if px > top {
                                1.0
                            } else if px < bot {
                                -1.0
                            } else {
                                0.0
                            }
                        }),
                        ichimoku_future_bias: ichimoku_reading
                            .map(|r| (r.senkou_a - r.senkou_b).to_f64().unwrap_or(0.0).signum()),
                        hull_ma: hma_reading.map(|d| d.to_f64().unwrap_or(0.0)),
                        awesome_oscillator: ao_reading.map(|d| d.value.to_f64().unwrap_or(0.0)),
                        force_index: fi_reading.map(|d| d.to_f64().unwrap_or(0.0)),
                        williams_r: wr_reading.map(|d| d.to_f64().unwrap_or(0.0)),
                        cci: cci_reading.map(|d| d.to_f64().unwrap_or(0.0)),
                        psar_sar: psar_reading.map(|d| d.sar.to_f64().unwrap_or(0.0)),
                        funding_rate: prev_fund_f,
                        cascade_state: Some(last_cascade_state),
                    };

                    // Stamp signal freshness (age in completed bars).
                    let mut indicators = indicators;
                    live_bar = live_bar.wrapping_add(1);
                    stamp_signal_ages(&mut indicators, &mut signal_age_tracker, live_bar);

                    // Inject Derivatives Data indicators (OI & Funding Rate).
                    let oi_f = latest_oi.read().await.and_then(|o| o.to_f64());
                    let fund_f = latest_funding.read().await.and_then(|f| f.to_f64());
                    let mark_f = latest_mark_px.read().await.and_then(|m| m.to_f64());
                    let idx_f = latest_index_px.read().await.and_then(|i| i.to_f64());
                    let spread_pct = match (mark_f, idx_f) {
                        (Some(m), Some(i)) if i > 0.0 => Some((m - i) / i * 100.0),
                        _ => None,
                    };

                    let oi_delta_f = match oi_f {
                        Some(cur) => {
                            oi_history.push_back(cur);
                            if oi_history.len() > 60 {
                                oi_history.pop_front();
                            }
                            if oi_history.len() > 1 {
                                Some(cur - oi_history.front().unwrap())
                            } else {
                                None
                            }
                        }
                        None => None,
                    };
                    inject_derivatives_indicators(
                        &mut indicators,
                        oi_f,
                        fund_f,
                        oi_delta_f,
                        mark_f,
                        spread_pct,
                    );

                    // Inject order book depth analysis indicators
                    inject_orderbook_indicators(
                        &mut indicators,
                        &order_book_analysis,
                        spread_wide_threshold_pct,
                    );

                    // Compute quantitative decision-support context.
                    let atr_val = indicators.get("atr").map(|v| v.raw_value).unwrap_or(0.0);

                    // Compute Statistical Intelligence Layer enrichment.
                    let close_f = completed.close.to_f64().unwrap_or(0.0);
                    let rsi_val = indicators.get("rsi").map(|v| v.raw_value).unwrap_or(50.0);
                    let bbwp_val = indicators.get("bbwp").map(|v| v.raw_value).unwrap_or(50.0);
                    let sqz_mom = indicators
                        .get("squeeze")
                        .and_then(|v| v.values.as_ref())
                        .and_then(|vals| vals.get("momentum").copied())
                        .unwrap_or(0.0);
                    let sqz_on = indicators
                        .get("squeeze")
                        .map(|v| v.state_label.contains("ON"))
                        .unwrap_or(false);
                    let vol_f = completed.volume.to_f64().unwrap_or(0.0);
                    let rvol_f = rvol.and_then(|r| r.to_f64()).unwrap_or(1.0);
                    let adx_val = indicators.get("adx").map(|v| v.raw_value).unwrap_or(25.0);

                    let current_context =
                        crate::market_context_synth::synthesize_market_context(&indicators);

                    let this_snapshot_for_synth = MarketSnapshot {
                        exchange: shadow_exchange,
                        timeframe_secs,
                        timestamp: candle_close_sec,
                        symbol: symbol.clone(),
                        is_completed: Some(true),
                        mid_price: completed.close,
                        bid_price: shadow_bid,
                        ask_price: shadow_ask,
                        bid_size: Some(completed.volume),
                        ask_size: Some(completed.volume),
                        funding_rate: fund_f
                            .map(|f| Decimal::from_f64_retain(f))
                            .flatten(),
                        open_interest: oi_f
                            .map(|o| Decimal::from_f64_retain(o))
                            .flatten(),
                        oi_delta_1h: oi_delta_f
                            .map(|d| Decimal::from_f64_retain(d))
                            .flatten(),
                        mark_price: latest_mark_px.read().await.clone(),
                        index_price: latest_index_px.read().await.clone(),
                        mark_index_spread_pct: spread_pct,
                        prev_day_px: shadow_prev_day_px,
                        open: Some(completed.open),
                        high: Some(completed.high),
                        low: Some(completed.low),
                        close: Some(completed.close),
                        volume: Some(completed.volume),
                        average_volume: avg_vol,
                        context: Some(current_context.clone()),
                        decision_context: None,
                        statistical_context: None,
                        indicators: indicators.clone(),
                        alignment: None,
                        risk: None,
                        analysis: None,
                        advisory: None,
                        opportunity: None,
                        liquidity_signals: vec![],
                        metrics_config: None,
                        risk_profile: None,
                        liquidity: None,
                        cluster: None,
                        quality_envelope: Some(quality_envelope.clone()),
                    };

                    let mut cross_tf_snaps: Vec<(u64, MarketSnapshot)> =
                        Vec::with_capacity(4);
                    cross_tf_snaps.push((
                        timeframe_secs,
                        this_snapshot_for_synth,
                    ));
                    for arc in [
                        &cross_tf_snapshot_a,
                        &cross_tf_snapshot_b,
                        &cross_tf_snapshot_c,
                    ] {
                        if let Some(s) = arc.read().await.clone() {
                            if !cross_tf_snaps
                                .iter()
                                .any(|(_, existing)| {
                                    existing.timeframe_secs == s.timeframe_secs
                                })
                            {
                                cross_tf_snaps
                                    .push((s.timeframe_secs, s));
                            }
                        }
                    }

                    let cross_refs: Vec<(u64, &MarketSnapshot)> = cross_tf_snaps
                        .iter()
                        .map(|(secs, s)| (*secs, s))
                        .collect();

                    let cluster_guard = cluster_matrix.read().await.clone();
                    let liquidity_flow = liquidity_acc.flush_to_flow();
                    last_cascade_state = liquidity_flow.cascade_state;
                    let liquidity_signals = core_domain::liquidity::derive_liquidity_signals(
                        &core_domain::liquidity::SignalInput {
                            flow: Some(&liquidity_flow),
                            cluster: cluster_guard.as_ref(),
                            funding_rate: fund_f.unwrap_or(0.0),
                            oi_delta_1h_pct: oi_delta_f.map(|d| if oi_f.unwrap_or(1.0).max(1.0).abs() > 1e-9 { d / oi_f.unwrap_or(1.0).max(1.0) * 100.0 } else { 0.0 }).unwrap_or(0.0),
                            price_bias: indicators.get("ema_stack").map(|v| v.normalized).unwrap_or(0.0),
                            prev_funding_rate: prev_bar_state.funding_rate,
                            prev_cascade_state: prev_bar_state.cascade_state,
                            ..Default::default()
                        },
                    );

                    let synthesis = crate::synthesis::synthesize_cross_tf(
                        &symbol,
                        &cross_refs,
                        Some(&liquidity_flow),
                        cluster_guard.as_ref(),
                        prev_mtf_score,
                        prev_regime,
                    );

                    prev_mtf_score = Some(synthesis.alignment.mtf_overall_score);
                    prev_regime = Some(synthesis.analysis.market_regime);

                    let confluence_score = {
                        let tradability_dim =
                            synthesis.alignment.dimensions.get(9).map(|d| d.score).unwrap_or(0.0);
                        let market_quality_score = synthesis.analysis.market_quality_score;
                        let opp_score = synthesis
                            .opportunity
                            .as_ref()
                            .map(|o| o.opportunity_score)
                            .unwrap_or(0.0);
                        (0.50 * tradability_dim + 0.30 * market_quality_score + 0.20 * opp_score)
                            .clamp(0.0, 100.0)
                    };

                    let l4_opportunity = synthesis.opportunity.clone();

                    let dec_ctx = core_domain::decision_context::DecisionContext::compute(
                        &indicators,
                        completed.close.to_f64().unwrap_or(0.0),
                        atr_val,
                        confluence_score,
                        &synthesis.analysis,
                        l4_opportunity.as_ref(),
                        &synthesis.risk,
                    );
                    let sil_ctx = sil_engine.advance_ext(
                        close_f,
                        atr_val,
                        rsi_val,
                        bbwp_val,
                        sqz_mom,
                        vol_f,
                        rvol_f,
                        adx_val,
                        prev_sil_close,
                        sqz_on,
                        indicators.get("macd").map(|v| v.raw_value).unwrap_or(0.0),
                        indicators.get("obv").map(|v| v.raw_value).unwrap_or(0.0),
                        indicators
                            .get("stochastic")
                            .and_then(|v| v.values.as_ref())
                            .and_then(|vals| vals.get("k_line").copied())
                            .unwrap_or(50.0),
                        indicators
                            .get("choppiness")
                            .map(|v| v.raw_value)
                            .unwrap_or(50.0),
                        indicators
                            .get("ema_stack")
                            .and_then(|v| v.values.as_ref())
                            .and_then(|vals| vals.get("medium").copied())
                            .unwrap_or(close_f),
                    );
                    prev_sil_close = close_f;

                    mc_counter += 1;
                    if mc_counter % 10 == 0 {
                        sil_engine.run_monte_carlo(close_f, atr_val);
                    }

                    let completed_snapshot = MarketSnapshot {
                        exchange: shadow_exchange,
                        timeframe_secs,
                        timestamp: candle_close_sec,
                        symbol: symbol.clone(),
                        is_completed: Some(true),
                        mid_price: completed.close,
                        bid_price: shadow_bid,
                        ask_price: shadow_ask,
                        bid_size: Some(completed.volume),
                        ask_size: Some(completed.volume),
                        funding_rate: fund_f.map(|f| Decimal::from_f64_retain(f)).flatten(),
                        open_interest: oi_f.map(|o| Decimal::from_f64_retain(o)).flatten(),
                        oi_delta_1h: oi_delta_f.map(|d| Decimal::from_f64_retain(d)).flatten(),
                        mark_price: latest_mark_px.read().await.clone(),
                        index_price: latest_index_px.read().await.clone(),
                        mark_index_spread_pct: spread_pct,
                        prev_day_px: shadow_prev_day_px,
                        open: Some(completed.open),
                        high: Some(completed.high),
                        low: Some(completed.low),
                        close: Some(completed.close),
                        volume: Some(completed.volume),
                        average_volume: avg_vol,
                        context: Some(current_context),
                        decision_context: Some(dec_ctx),
                        statistical_context: Some(sil_ctx),
                        indicators,
                        alignment: Some(synthesis.alignment),
                        risk: Some(synthesis.risk),
                        analysis: Some(synthesis.analysis),
                        advisory: Some(synthesis.advisory),
                        opportunity: synthesis.opportunity,
                        risk_profile: None,
                        liquidity: Some(liquidity_flow),
                        cluster: cluster_matrix.read().await.clone(),
                        liquidity_signals,
                        metrics_config: active_set.to_metrics_config(),
                        quality_envelope: Some(quality_envelope),
                    };

                    let _ = telemetry_tx
                        .send(database_storage::TelemetryMsg::InsertSnapshot(completed_snapshot.clone()))
                        .await;

                    latency_tracker.record_observation_latency(
                        core_domain::LatencyTracker::now_ms().saturating_sub(completed.start_time_ms),
                    );

                    let _ = broadcast_tx.send(completed_snapshot.clone());

                    // Publish the completed snapshot as the latest for this TF.
                    {
                        let mut snap = latest_snapshot.write().await;
                        *snap = Some(completed_snapshot.clone());
                    }

                    // Decisive close invalidation: check at every 1-minute candle close
                    if timeframe_secs == 60 {
                        if let Some(ref pool) = paper_pool {
                            if let Some(pos) =
                                database_storage::paper::queries::paper_get_active_position(pool, &symbol).await
                            {
                                if let Some(inval_level) = pos.final_invalidation_level {
                                    let tolerance = 0.002;
                                    let close_f64 = completed.close.to_f64().unwrap_or(0.0);
                                    let invalidated = match pos.direction.as_str() {
                                        "LONG" => close_f64 < inval_level * (1.0 - tolerance),
                                        "SHORT" => close_f64 > inval_level * (1.0 + tolerance),
                                        _ => false,
                                    };
                                    if invalidated {
                                        // paper_trading::invalidate_position removed (stub; cycle broken)
                                    }
                                }
                            }
                        }
                    }

                    // The strict handover gate above guarantees this candle is
                    // strictly newer than any historical candle, so it is always
                    // a fresh append — no dedup/overwrite of historical data.
                    {
                        let mut hist = history.write().await;
                        hist.push_back(completed.clone());
                        while hist.len() > HIST_BUFFER_MAX {
                            hist.pop_front();
                        }
                        let mut snap_hist = snapshot_history.write().await;
                        snap_hist.push_back(completed_snapshot.clone());
                        while snap_hist.len() > HIST_BUFFER_MAX {
                            snap_hist.pop_front();
                        }
                    }
                    if let Some(ref tx) = candle_forward {
                        let _ = tx.send(completed.clone()).await;
                    }
                }

                // BROADCAST: Flickering snapshot from live candle
                broadcast_live_snapshot(
                    &broadcast_tx,
                    &symbol,
                    &live_candle,
                    shadow_exchange,
                    shadow_bid,
                    shadow_ask,
                    &ema_fast,
                    &ema_medium,
                    &ema_slow,
                    &ema_long,
                    &rsi_14,
                    &macd,
                    &adx_14,
                    &sqz_mom,
                    &bollinger,
                    &atr_standalone,
                    &bbwp_indicator,
                    &stochastic_indicator,
                    &chandemo_indicator,
                    &supertrend_indicator,
                    &keltner_indicator,
                    &donchian_indicator,
                    &obv_indicator,
                    &cmf_indicator,
                    &mfi_indicator,
                    &hv_indicator,
                    &aroon_indicator,
                    &choppiness_indicator,
                    &linreg_indicator,
                    &zscore_indicator,
                    &vwap_sum_tp_vol,
                    &vwap_sum_vol,
                    &volume_history,
                    timeframe_secs,
                    shadow_prev_day_px,
                );
            }

            NormalizedEvent::OrderBook(ref book) => {
                shadow_exchange = Some(book.exchange);
                if let (Some(best_bid), Some(best_ask)) = (book.bids.first(), book.asks.first()) {
                    shadow_bid = best_bid.0;
                    shadow_ask = best_ask.0;
                }

                // Update order book depth analysis
                {
                    let bids_f64: Vec<(f64, f64)> = book
                        .bids
                        .iter()
                        .map(|(p, s)| (p.to_f64().unwrap_or(0.0), s.to_f64().unwrap_or(0.0)))
                        .collect();
                    let asks_f64: Vec<(f64, f64)> = book
                        .asks
                        .iter()
                        .map(|(p, s)| (p.to_f64().unwrap_or(0.0), s.to_f64().unwrap_or(0.0)))
                        .collect();
                    order_book_analysis.update(&bids_f64, &asks_f64);
                }

                if candle_gen.current_candle.is_some() {
                    let mid = (shadow_bid + shadow_ask) / Decimal::from(2);
                    let shadow_candle = NormalizedCandle {
                        exchange: candle_gen.exchange,
                        symbol: symbol.clone(),
                        start_time_ms: candle_gen.current_start_ms,
                        duration_ms: candle_gen.duration_ms,
                        open: candle_gen.current_open,
                        high: candle_gen.current_high.max(mid),
                        low: candle_gen.current_low.min(mid),
                        close: mid,
                        volume: candle_gen.current_volume,
                        trades_count: candle_gen.current_trades,
                        reconstructed: None,
                    };

                    broadcast_live_snapshot(
                        &broadcast_tx,
                        &symbol,
                        &shadow_candle,
                        shadow_exchange,
                        shadow_bid,
                        shadow_ask,
                        &ema_fast,
                        &ema_medium,
                        &ema_slow,
                        &ema_long,
                        &rsi_14,
                        &macd,
                        &adx_14,
                        &sqz_mom,
                        &bollinger,
                        &atr_standalone,
                        &bbwp_indicator,
                        &stochastic_indicator,
                        &chandemo_indicator,
                        &supertrend_indicator,
                        &keltner_indicator,
                        &donchian_indicator,
                        &obv_indicator,
                        &cmf_indicator,
                        &mfi_indicator,
                        &hv_indicator,
                        &aroon_indicator,
                        &choppiness_indicator,
                        &linreg_indicator,
                        &zscore_indicator,
                        &vwap_sum_tp_vol,
                        &vwap_sum_vol,
                        &volume_history,
                        timeframe_secs,
                        shadow_prev_day_px,
                    );
                }
            }

            NormalizedEvent::AssetContext(ref ctx) => {
                shadow_prev_day_px = Some(ctx.prev_day_px);
            }

            NormalizedEvent::OpenInterest(ref oi) => {
                let mut guard = latest_oi.write().await;
                *guard = Some(oi.oi);
            }

            NormalizedEvent::FundingRate(ref fr) => {
                let mut guard = latest_funding.write().await;
                *guard = Some(fr.rate);
            }

            NormalizedEvent::MarkPrice(ref mp) => {
                let mut mark_guard = latest_mark_px.write().await;
                *mark_guard = Some(mp.mark_px);
                if let Some(idx) = mp.index_px {
                    let mut idx_guard = latest_index_px.write().await;
                    *idx_guard = Some(idx);
                }
            }

            // Phase 1 hook: Liquidation events are also persisted to DB via
            // the telemetry channel. The flow aggregation happens here in a
            // later phase (Phase 1 + accumulator) but persisting the raw events
            // now means we have data ready when the flow logic lands.
            NormalizedEvent::Liquidation(ref liq) => {
                let side_str = match liq.side {
                    core_domain::normalized::LiquidationSide::Long => "LONG",
                    core_domain::normalized::LiquidationSide::Short => "SHORT",
                };
                let size_usd = liq.price.to_f64().unwrap_or(0.0) * liq.size.to_f64().unwrap_or(0.0);
                let _ = telemetry_tx
                    .send(database_storage::TelemetryMsg::InsertLiquidationEvent {
                        exchange: liq.exchange,
                        symbol: liq.symbol.clone(),
                        side: side_str.to_string(),
                        price: liq.price.to_f64().unwrap_or(0.0),
                        size_usd,
                        timestamp_ms: liq.timestamp_ms,
                        venue_order_id: liq.venue_order_id.clone(),
                    })
                    .await;
                // Phase 1: feed the per-candle aggregator. This drives the
                // `LiquidityFlow` attached to the next completed snapshot.
                liquidity_acc.record_event(liq.clone());
            }

            NormalizedEvent::Status {
                exchange,
                status,
                message,
            } => {
                println!(
                    "[STATUS {}] {}: {:?} — {}",
                    timeframe_label, exchange, status, message
                );
            }
        }
    }
}

/// Inject Derivatives Data (OI & Funding) normalized indicator entries into
/// the snapshot indicator map. Called after the main indicator map is built.
fn inject_derivatives_indicators(
    indicators: &mut HashMap<String, NormalizedIndicatorValue>,
    oi: Option<f64>,
    funding: Option<f64>,
    oi_delta: Option<f64>,
    mark_px: Option<f64>,
    spread_pct: Option<f64>,
) {
    use crate::indicators::normalized::derivatives;

    // Open Interest
    if let Some(o) = oi {
        indicators.insert("open_interest".into(), derivatives::normalize_open_interest(o));
    }

    // OI Delta (1h change)
    if let Some(delta) = oi_delta {
        indicators.insert("oi_delta".into(), derivatives::normalize_oi_delta(delta));
    }

    // Funding Rate (non-directional gate)
    if let Some(f) = funding {
        indicators.insert("funding_rate".into(), derivatives::normalize_funding_rate(f));
    }

    // OI-Price Divergence
    if let (Some(_o), Some(delta)) = (oi, oi_delta) {
        let ema_bias = indicators
            .get("ema_stack")
            .map(|v| v.normalized)
            .unwrap_or(0.0);
        indicators.insert(
            "oi_price_divergence".into(),
            derivatives::normalize_oi_price_divergence(delta, ema_bias),
        );
    }

    // Mark-Index Spread (Phase 0: derivatives telemetry activation).
    // Positive spread = mark premium (perp trades above index, bullish bias).
    // Negative spread = perp discount (bearish bias). Wide spread signals
    // market stress and is a leading indicator of forced liquidations.
    if let Some(spread) = spread_pct {
        indicators.insert(
            "mark_index_spread".into(),
            derivatives::normalize_mark_index_spread(spread, mark_px),
        );
    }
}

/// Inject Order Book Depth Analysis normalized indicator entries into the
/// snapshot indicator map. Called after the main indicator map is built.
fn inject_orderbook_indicators(
    indicators: &mut HashMap<String, NormalizedIndicatorValue>,
    ob: &OrderBookAnalysis,
    spread_wide_threshold_pct: f64,
) {
    use crate::indicators::normalized::derivatives;

    // Order Flow Imbalance
    if let Some(ofi) = ob.order_flow_imbalance() {
        indicators.insert(
            "order_flow_imbalance".into(),
            derivatives::normalize_order_flow_imbalance(ofi),
        );
    }

    // Spread (non-directional gate)
    if let Some(spread) = ob.spread_pct() {
        indicators.insert(
            "spread".into(),
            derivatives::normalize_spread(spread * 100.0, spread_wide_threshold_pct),
        );
    }

    // Depth Bias (bid depth / ask depth ratio)
    if let Some(ratio) = ob.depth_imbalance_ratio(1.0) {
        if ratio.is_finite() {
            indicators.insert(
                "depth_bias".into(),
                derivatives::normalize_depth_bias(ratio),
            );
        }
    }

    // Wall signals: attach to order_flow_imbalance entry if it exists
    if let Some(ref wall) = ob.wall_detected() {
        use crate::indicators::normalized::{IndicatorSignal, SignalDirection, SignalKind, SignalStatus};
        match wall.as_str() {
            "BID_WALL" => {
                if let Some(ofier) = indicators.get_mut("order_flow_imbalance") {
                    ofier.signals.push(IndicatorSignal {
                        kind: SignalKind::Threshold,
                        direction: SignalDirection::Bullish,
                        status: SignalStatus::Active,
                        label: "BID_WALL".to_string(),
                        strength: 0.8,
                        age_bars: 0,
                        points: None,
                    });
                }
            }
            "ASK_WALL" => {
                if let Some(ofier) = indicators.get_mut("order_flow_imbalance") {
                    ofier.signals.push(IndicatorSignal {
                        kind: SignalKind::Threshold,
                        direction: SignalDirection::Bearish,
                        status: SignalStatus::Active,
                        label: "ASK_WALL".to_string(),
                        strength: 0.8,
                        age_bars: 0,
                        points: None,
                    });
                }
            }
            _ => {}
        }
    }
}

/// Derive current support/resistance levels from swing pivots, updating the
/// role-reversal tracker with the latest levels and candle close. Swing highs
/// act as resistance, swing lows as support; the tracker flips a level's role
/// when a candle closes decisively beyond it. Returns the current role-adjusted
/// `(support_levels, resistance_levels)` for normalization.
pub(crate) fn update_sr_levels(
    tracker: &mut SrRoleTracker,
    pivots: &[crate::indicators::PivotPoint],
    close: Decimal,
    timestamp_sec: u64,
) -> (Vec<f64>, Vec<f64>) {
    let mut raw_sup: Vec<f64> = Vec::new();
    let mut raw_res: Vec<f64> = Vec::new();
    for p in pivots {
        let price = p.price.to_f64().unwrap_or(0.0);
        if price <= 0.0 {
            continue;
        }
        match p.pivot_type {
            crate::indicators::PivotType::High => raw_res.push(price),
            crate::indicators::PivotType::Low => raw_sup.push(price),
        }
    }
    tracker.register_levels(&raw_sup, &raw_res);
    let _ = tracker.process_candle_close(close.to_f64().unwrap_or(0.0), timestamp_sec);
    (tracker.get_supports(), tracker.get_resistances())
}

/// Stamp `age_bars` on every signal using a persistent tracker keyed by
/// `<indicator>:<kind>`. A signal resets to age 0 when it first appears or flips
/// direction; otherwise its age is the number of completed bars since first seen.
fn stamp_signal_ages(
    map: &mut std::collections::HashMap<String, crate::indicators::NormalizedIndicatorValue>,
    tracker: &mut std::collections::HashMap<String, (u32, crate::indicators::SignalDirection)>,
    bar: u32,
) {
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (key, entry) in map.iter_mut() {
        for sig in entry.signals.iter_mut() {
            let tk = format!("{}:{:?}", key, sig.kind);
            seen.insert(tk.clone());
            match tracker.get(&tk) {
                Some((first, dir)) if *dir == sig.direction => {
                    sig.age_bars = bar.saturating_sub(*first);
                }
                _ => {
                    tracker.insert(tk, (bar, sig.direction));
                    sig.age_bars = 0;
                }
            }
        }
    }
    // Evict trackers whose signal no longer fires so a re-appearance is "fresh".
    tracker.retain(|k, _| seen.contains(k));
}

#[allow(clippy::too_many_arguments)]
fn broadcast_live_snapshot(
    broadcast_tx: &broadcast::Sender<MarketSnapshot>,
    symbol: &str,
    candle: &NormalizedCandle,
    exchange: Option<Exchange>,
    bid_price: Decimal,
    ask_price: Decimal,
    ema_fast: &Ema,
    ema_medium: &Ema,
    ema_slow: &Ema,
    ema_long: &Ema,
    rsi_14: &Rsi,
    macd: &Macd,
    adx_14: &Adx,
    sqz_mom: &SqueezeMomentum,
    bollinger: &BollingerBands,
    atr_standalone: &Atr,
    bbwp_indicator: &Bbwp,
    stochastic_indicator: &Stochastic,
    chandemo_indicator: &ChandeMO,
    supertrend_indicator: &Supertrend,
    keltner_indicator: &Keltner,
    donchian_indicator: &Donchian,
    obv_indicator: &Obv,
    cmf_indicator: &Cmf,
    mfi_indicator: &Mfi,
    hv_indicator: &HistoricalVolatility,
    aroon_indicator: &Aroon,
    choppiness_indicator: &Choppiness,
    linreg_indicator: &LinRegSlope,
    zscore_indicator: &ZScore,
    vwap_sum_tp_vol: &Decimal,
    vwap_sum_vol: &Decimal,
    volume_history: &VecDeque<Decimal>,
    timeframe_secs: u64,
    prev_day_px: Option<Decimal>,
) {
    let val_ema_fast = ema_fast.clone().update(candle.close);
    let val_ema_medium = ema_medium.clone().update(candle.close);
    let val_ema_slow = ema_slow.clone().update(candle.close);
    let val_ema_long = ema_long.clone().update(candle.close);
    let val_rsi = rsi_14.clone().update(candle.close);
    let val_macd = macd.clone().update(candle.close);
    let val_adx = adx_14.clone().update(candle.high, candle.low, candle.close);
    let val_sqz = sqz_mom
        .clone()
        .update(candle.high, candle.low, candle.close);
    let val_bb = bollinger.clone().update(candle.close);
    let val_atr = atr_standalone
        .clone()
        .update(candle.high, candle.low, candle.close);
    let val_bbwp = bbwp_indicator.clone().update(candle.close);
    let val_stoch = stochastic_indicator
        .clone()
        .update(candle.high, candle.low, candle.close);
    let val_cmo = chandemo_indicator.clone().update(candle.close);
    let val_supertrend = supertrend_indicator
        .clone()
        .update(candle.high, candle.low, candle.close);
    let val_keltner = keltner_indicator
        .clone()
        .update(candle.high, candle.low, candle.close);
    let val_donchian = donchian_indicator.clone().update(candle.high, candle.low);
    let val_obv = obv_indicator.clone().update(candle.close, candle.volume);
    let val_cmf =
        cmf_indicator
            .clone()
            .update(candle.high, candle.low, candle.close, candle.volume);
    let val_mfi =
        mfi_indicator
            .clone()
            .update(candle.high, candle.low, candle.close, candle.volume);
    let val_hv = hv_indicator.clone().update(candle.close);
    let val_aroon = aroon_indicator.clone().update(candle.high, candle.low);
    let val_chop = choppiness_indicator
        .clone()
        .update(candle.high, candle.low, candle.close);
    let val_linreg = linreg_indicator.clone().update(candle.close);
    let val_zscore = zscore_indicator.clone().update(candle.close);

    let typical_price = (candle.high + candle.low + candle.close) / Decimal::from(3);
    let temp_sum_tp_vol = *vwap_sum_tp_vol + typical_price * candle.volume;
    let temp_sum_vol = *vwap_sum_vol + candle.volume;
    let val_vwap = if temp_sum_vol > Decimal::ZERO {
        Some(temp_sum_tp_vol / temp_sum_vol)
    } else {
        None
    };

    let avg_vol = if !volume_history.is_empty() {
        let sum: Decimal = volume_history.iter().sum();
        Some(sum / Decimal::from(volume_history.len()))
    } else {
        None
    };

    let rvol = match (candle.volume, avg_vol) {
        (vol, Some(avg)) if avg > Decimal::ZERO => Some(vol / avg),
        _ => None,
    };

    let ema_stack_state = if val_ema_fast > val_ema_medium
        && val_ema_medium > val_ema_slow
        && val_ema_slow > val_ema_long
        && candle.close > val_ema_fast
    {
        Some("bullish")
    } else if val_ema_fast < val_ema_medium
        && val_ema_medium < val_ema_slow
        && val_ema_slow < val_ema_long
        && candle.close < val_ema_fast
    {
        Some("bearish")
    } else {
        Some("tangled")
    };

    let indicators = normalize::build_indicator_map(normalize::NormalizeParams {
        close: candle.close,
        rsi: val_rsi,
        rsi_divergence: crate::indicators::DivergenceState::None,
        macd_divergence: crate::indicators::DivergenceState::None,
        stoch_k: val_stoch.as_ref().map(|s| s.k_value),
        stoch_d: val_stoch.as_ref().map(|s| s.d_value),
        chandemo: val_cmo,
        supertrend_line: val_supertrend.as_ref().map(|s| s.line),
        supertrend_dir: val_supertrend.as_ref().map(|s| s.direction),
        keltner: val_keltner.as_ref().map(|k| (k.upper, k.middle, k.lower)),
        donchian: val_donchian.as_ref().map(|d| (d.upper, d.middle, d.lower)),
        obv: val_obv.as_ref().map(|o| o.obv),
        obv_sma: val_obv.as_ref().map(|o| o.obv_sma),
        cmf: val_cmf,
        mfi: val_mfi,
        hv: val_hv,
        aroon_up: val_aroon.as_ref().map(|a| a.up),
        aroon_down: val_aroon.as_ref().map(|a| a.down),
        choppiness: val_chop,
        linreg_slope: val_linreg,
        zscore: val_zscore,
        extra_div: normalize::ExtraDivergence::default(),
        macd: &val_macd,
        sqz: val_sqz.as_ref(),
        adx: val_adx.as_ref(),
        bb: val_bb,
        atr: val_atr.as_ref(),
        bbwp: val_bbwp,
        vwap: val_vwap,
        anchored_vwap: None,
        ema_stack_state,
        ema_fast: Some(val_ema_fast),
        ema_medium: Some(val_ema_medium),
        ema_slow: Some(val_ema_slow),
        ema_long: Some(val_ema_long),
        rvol,
        volume: Some(candle.volume),
        average_volume: avg_vol,
        fib: None,
        pattern: None,
        support_levels: &[],
        resistance_levels: &[],
        active_position: None,
        adx_consecutive_deceleration: false,
        supertrend_flipped: false,
        adx_di_crossover: None,
        pivot_levels: None,
        pivot_proximity_pct: 0.0015,
        candlestick: None,
        candlestick_min_confidence: 0.3,
        ichimoku: None,
        cci: None,
        psar: None,
        williams_r: None,
        awesome_oscillator: None,
        force_index: None,
        hull_ma: None,
        stddev_channel: None,
        volume_profile: None,
        smc: None,
        prev: PreviousBarState::default(),
    });

    let snapshot = MarketSnapshot {
        exchange,
        timeframe_secs,
        timestamp: candle.start_time_ms / 1000,
        symbol: symbol.to_string(),
        is_completed: Some(false),
        mid_price: candle.close,
        bid_price,
        ask_price,
        bid_size: Some(candle.volume),
        ask_size: Some(candle.volume),
        funding_rate: None,
        open_interest: None,
        oi_delta_1h: None,
        mark_price: None,
        index_price: None,
        mark_index_spread_pct: None,
        prev_day_px,
        open: Some(candle.open),
        high: Some(candle.high),
        low: Some(candle.low),
        close: Some(candle.close),
        volume: Some(candle.volume),
        average_volume: avg_vol,
        context: None,
        decision_context: None,
        statistical_context: None,
        indicators,
        alignment: None,
        risk: None,
        analysis: None,
        advisory: None,
        opportunity: None,
        liquidity_signals: vec![],
        metrics_config: None,
        risk_profile: None,
        liquidity: None,
        cluster: None,
        quality_envelope: None,
    };

    let _ = broadcast_tx.send(snapshot);
}

#[cfg(test)]
mod age_tests {
    use super::stamp_signal_ages;
    use crate::indicators::{
        IndicatorSignal, NormalizedIndicatorValue, SignalDirection, SignalKind, SignalStatus,
    };
    use std::collections::HashMap;

    fn entry_with_signal(dir: SignalDirection) -> NormalizedIndicatorValue {
        NormalizedIndicatorValue::scalar(0.0, 0.5, "X").push_signal(IndicatorSignal::new(
            SignalKind::Divergence,
            dir,
            SignalStatus::Potential,
            "DIV",
        ))
    }

    #[test]
    fn age_increments_while_signal_persists() {
        let mut tracker = HashMap::new();
        let mut m = HashMap::new();
        m.insert(
            "rsi".to_string(),
            entry_with_signal(SignalDirection::Bullish),
        );
        stamp_signal_ages(&mut m, &mut tracker, 1);
        assert_eq!(m["rsi"].signals[0].age_bars, 0, "fresh signal age 0");

        let mut m2 = HashMap::new();
        m2.insert(
            "rsi".to_string(),
            entry_with_signal(SignalDirection::Bullish),
        );
        stamp_signal_ages(&mut m2, &mut tracker, 4);
        assert_eq!(m2["rsi"].signals[0].age_bars, 3, "3 bars since first seen");
    }

    #[test]
    fn age_resets_on_direction_flip() {
        let mut tracker = HashMap::new();
        let mut m = HashMap::new();
        m.insert(
            "rsi".to_string(),
            entry_with_signal(SignalDirection::Bullish),
        );
        stamp_signal_ages(&mut m, &mut tracker, 1);

        let mut m2 = HashMap::new();
        m2.insert(
            "rsi".to_string(),
            entry_with_signal(SignalDirection::Bearish),
        );
        stamp_signal_ages(&mut m2, &mut tracker, 5);
        assert_eq!(m2["rsi"].signals[0].age_bars, 0, "flip resets age");
    }
}
