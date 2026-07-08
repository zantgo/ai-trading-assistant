//! Statistical Intelligence Layer — top-level module.
//!
//! The SIL transforms historical OHLC information into statistical knowledge.
//! It enriches `MarketSnapshot` with a `StatisticalContext` without modifying
//! any existing indicators, `DecisionContext`, or `MarketContext`.
//!
//! Six independent modules (A–F) plus an ML layer and derived decision-support
//! features.  All computations are incremental, streaming, deterministic, and
//! constant-memory where possible.

pub mod types;
pub mod rolling_window;
pub mod distribution;
pub mod statistical_object;
pub mod statistical_context;

pub mod market_shape;
pub mod probability;
pub mod bayesian;
pub mod confidence;
pub mod relationship;
pub mod monte_carlo;
pub mod kalman;
pub mod online_learning;
pub mod feature_importance;
pub mod clustering;
pub mod anomaly;
pub mod regime_classifier;
pub mod derived_features;

use distribution::{DistributionTracker, METRIC_COUNT};
use probability::ProbabilityEngine;
use bayesian::{BayesianEngine, ObservationKind};
use confidence::ConfidenceEngine;
use market_shape::MarketShape;
use relationship::RelationshipSnapshot;
use monte_carlo::MonteCarloPriceOutcome;
use kalman::KalmanFilter;
use online_learning::OnlineLearner;
use feature_importance::FeatureImportanceTracker;
use clustering::RegimeClusterer;
use anomaly::AnomalyDetector;
use regime_classifier::StatisticalRegime;
use derived_features::DerivedFeatures;
use statistical_context::StatisticalContext;

pub use types::WINDOW_SIZES;
pub use types::StatisticsConfig;

// ── StatisticsEngine ───────────────────────────────────────────
//
// The single entry-point for the analyzer pipeline.  Instantiated once
// per timeframe, `advance()` is called on every completed candle to
// produce a fresh `StatisticalContext`.

/// Central engine that coordinates all SIL sub-modules.
#[derive(Debug, Clone)]
pub struct StatisticsEngine {
    config: StatisticsConfig,
    distribution: DistributionTracker,
    probability: ProbabilityEngine,
    bayesian: BayesianEngine,
    confidence: ConfidenceEngine,
    mc_cache: Option<MonteCarloPriceOutcome>,
    kalman: Option<KalmanFilter>,
    online: OnlineLearner,
    feature_imp: FeatureImportanceTracker,
    regime_cluster: RegimeClusterer,
    anomaly: AnomalyDetector,
    bar_count: u64,
    /// Pending feature snapshots queued for forward-return evaluation.
    feature_queue: std::collections::VecDeque<Vec<f64>>,
    /// Cached top predictors (recomputed every 20 bars).
    cached_predictors: Vec<(String, f64)>,
    bars_since_importance: usize,
}

impl StatisticsEngine {
    /// Create a new engine using the provided configuration.
    pub fn new(config: StatisticsConfig) -> Self {
        let windows = if config.windows.is_empty() {
            WINDOW_SIZES.to_vec()
        } else {
            config.windows.clone()
        };
        Self {
            distribution: DistributionTracker::new(&windows),
            probability: ProbabilityEngine::new(
                config.probability_min_observations,
                config.probability_forward_bars,
            ),
            bayesian: BayesianEngine::new(
                config.bayesian_prior_alpha,
                config.bayesian_prior_beta,
                config.probability_forward_bars,
            ),
            confidence: ConfidenceEngine::new(),
            mc_cache: None,
            kalman: if config.kalman_enabled {
                Some(KalmanFilter::new(
                    config.kalman_process_noise,
                    config.kalman_measurement_noise,
                    config.kalman_residual_window,
                ))
            } else {
                None
            },
            online: OnlineLearner::new(500),
            feature_imp: FeatureImportanceTracker::new(config.feature_importance_top_n),
            regime_cluster: RegimeClusterer::new(),
            anomaly: AnomalyDetector::default(),
            bar_count: 0,
            feature_queue: std::collections::VecDeque::new(),
            cached_predictors: Vec::new(),
            bars_since_importance: 0,
            config,
        }
    }

    /// Advance the engine by one candle.  Returns the current
    /// `StatisticalContext`.  This is the **only** method the analyzer
    /// pipeline needs to call.
    ///
    /// # Arguments
    ///
    /// * `close`    — completed candle close price
    /// * `atr`      — current ATR value (raw, from indicator map)
    /// * `rsi`      — current RSI value (raw)
    /// * `bbwp`     — current BBWP value (raw)
    /// * `squeeze`  — current squeeze momentum value (0 = neutral/off)
    /// * `volume`   — candle volume
    /// * `rvol`     — current relative volume
    /// * `adx`      — current ADX value (raw)
    /// * `prev_close` — previous candle close (for log-return calc)
    /// * `squeeze_on` — whether squeeze is currently active (red dots)
    pub fn advance(
        &mut self,
        close: f64,
        atr: f64,
        rsi: f64,
        bbwp: f64,
        squeeze: f64,
        volume: f64,
        rvol: f64,
        adx: f64,
        prev_close: f64,
        squeeze_on: bool,
    ) -> StatisticalContext {
        self.advance_ext(close, atr, rsi, bbwp, squeeze, volume, rvol, adx,
            prev_close, squeeze_on, 0.0, 0.0, 0.0, 50.0, 0.0)
    }

    pub fn advance_ext(
        &mut self,
        close: f64,
        atr: f64,
        rsi: f64,
        bbwp: f64,
        squeeze: f64,
        volume: f64,
        rvol: f64,
        adx: f64,
        prev_close: f64,
        squeeze_on: bool,
        macd: f64,
        obv: f64,
        stochk: f64,
        choppiness: f64,
        ema_50: f64,
    ) -> StatisticalContext {
        if !self.config.enabled {
            return StatisticalContext::default();
        }

        self.bar_count = self.bar_count.wrapping_add(1);

        let log_return = if prev_close > 1e-12 {
            (close / prev_close).ln() * 100.0
        } else {
            0.0
        };

        // ── Kalman drift update (per-candle, O(1)) ──
        if let Some(ref mut kf) = self.kalman {
            kf.update(close, prev_close);
        }

        let metrics: [f64; METRIC_COUNT] = [
            close, log_return, atr, rsi, bbwp, squeeze, volume, rvol, adx,
            macd, obv, stochk, choppiness, ema_50,
        ];
        self.distribution.advance(&metrics);

        // ── Distribution (Module A) ──
        let ks = self.distribution.key_statistics();

        // ── Probability (Module B) ──
        let prob_snap = self.probability.compute_all(&self.distribution);

        // ── Bayesian (Module B) ──
        // Queue triggers for this bar so they resolve after forward_bars.
        self.queue_bayesian_triggers(close, atr, rsi, bbwp, squeeze_on);
        // Advance bar counter and resolve pending observations.
        self.bayesian.advance_bar(close, atr);
        let posteriors = self.bayesian.posterior_map();

        let mut obs_counts = prob_snap.observation_counts.clone();
        for name in probability::EVENT_NAMES {
            if let Some(t) = self.bayesian.tracker(name) {
                obs_counts.insert(name.to_string(), t.trials);
            }
        }

        // ── Confidence (Module C) ──
        let total_trials = self.bayesian.total_trials();
        // Record reliability: use trend_continuation_prob as the "prediction"
        // and the current bar's direction as the "actual outcome".
        let bar_direction_up = close > prev_close;
        self.confidence.record_outcome(
            prob_snap.trend_continuation_prob,
            bar_direction_up,
        );
        let conf_snap = self.confidence.compute_all(&self.distribution, total_trials);

        // ── Market Shape (Module D) ──
        let shape = MarketShape::compute(&self.distribution);

        // ── Relationship (Module E) ──
        let rel = RelationshipSnapshot::compute(&self.distribution);

        // ── ML Layer (Phase 7) ────────────────────────────
        // Online learning: observe outcomes for resolved Bayesian events.
        for name in probability::EVENT_NAMES {
            if let Some(t) = self.bayesian.tracker(name) {
                if t.trials > 0 {
                    let mean = t.posterior_mean();
                    let success = mean > 0.5;
                    self.online.observe(name, success);
                }
            }
        }

        // Feature-outcome queue: push current 9-metric vector, record
        // forward returns when lookahead elapses.
        let current_features: Vec<f64> = (0..METRIC_COUNT)
            .map(|mi| self.distribution.metric_values(
                self.distribution.best_window_idx(), mi,
            ).last().copied().unwrap_or(0.0))
            .collect();
        self.feature_queue.push_back(current_features);
        while self.feature_queue.len() > self.config.probability_forward_bars * 2 {
            let _ = self.feature_queue.pop_front();
        }

        // Recompute feature importance every 20 bars.
        self.bars_since_importance += 1;
        if self.bars_since_importance >= 20 && self.online.feature_count() >= 10 {
            let history: Vec<(Vec<f64>, f64)> = self.online.outcome_history
                .iter()
                .map(|fo| (fo.features.clone(), fo.forward_return))
                .collect();
            self.cached_predictors = self.feature_imp.compute(&history);
            self.bars_since_importance = 0;
        }

        // Record feature-outcome pairs when lookahead is available.
        if let Some(front_features) = self.feature_queue.front().cloned() {
            // Use current log-return as the "forward return" for the oldest
            // queued feature snapshot.
            self.online.observe_features(front_features, log_return);
        }

        // Anomaly detection.
        let (anomaly_score, top_reason) = self.anomaly.detect(&self.distribution);

        // Statistical regime from distribution shape + clustering.
        let stat_regime = StatisticalRegime::classify(
            shape.skewness,
            shape.kurtosis,
            shape.entropy,
            shape.volatility_percentile,
            shape.compression_percentile,
            rel.trend_consistency,
        );

        // Clustering: normalized feature vector for online k-means.
        let cluster_features = normalize_features(
            close, atr, rsi, bbwp, squeeze, rvol, adx, log_return,
        );
        let (cluster_label, cluster_stability) =
            self.regime_cluster.classify_and_update(&cluster_features);

        let mut ctx = StatisticalContext {
            price_stats: ks.price,
            return_stats: ks.returns,
            atr_stats: ks.atr,
            rsi_stats: ks.rsi,
            bbwp_stats: ks.bbwp,

            trend_continuation_prob: prob_snap.trend_continuation_prob,
            mean_reversion_prob: prob_snap.mean_reversion_prob,
            breakout_success_prob: prob_snap.breakout_success_prob,
            reversal_prob: prob_snap.reversal_prob,
            atr_expansion_prob: prob_snap.atr_expansion_prob,
            squeeze_release_prob: prob_snap.squeeze_release_prob,
            volatility_expansion_prob: prob_snap.volatility_expansion_prob,
            stop_before_target_prob: prob_snap.stop_before_target_prob,
            observation_counts: obs_counts,
            bayesian_posteriors: posteriors,

            prediction_interval_68: conf_snap.prediction_interval_68,
            prediction_interval_95: conf_snap.prediction_interval_95,
            prediction_interval_99: conf_snap.prediction_interval_99,
            bootstrap_confidence_95: conf_snap.bootstrap_confidence_95,
            historical_reliability: conf_snap.historical_reliability,
            confidence_score: conf_snap.confidence_score,

            skewness: shape.skewness,
            kurtosis: shape.kurtosis,
            entropy: shape.entropy,
            tail_risk: shape.tail_risk,
            distribution_symmetry: shape.distribution_symmetry,
            market_shape_label: shape.shape_label,
            volatility_percentile: shape.volatility_percentile,
            compression_percentile: shape.compression_percentile,

            feature_agreement: rel.feature_agreement,
            indicator_redundancy: rel.indicator_redundancy,
            consensus_stability: rel.consensus_stability,
            trend_consistency: rel.trend_consistency,
            momentum_consistency: rel.momentum_consistency,

            mc_target_hit_prob: self.mc_cache.as_ref().map(|m| m.target_hit_prob).unwrap_or(0.0),
            mc_stop_hit_prob: self.mc_cache.as_ref().map(|m| m.stop_hit_prob).unwrap_or(0.0),
            mc_max_drawdown_95: self.mc_cache.as_ref().map(|m| m.max_drawdown_95).unwrap_or(0.0),
            mc_max_favorable_excursion_95: self.mc_cache.as_ref().map(|m| m.max_favorable_excursion_95).unwrap_or(0.0),
            mc_expected_movement: self.mc_cache.as_ref().map(|m| m.expected_movement).unwrap_or(0.0),
            mc_best_case: self.mc_cache.as_ref().map(|m| m.best_case).unwrap_or(0.0),
            mc_worst_case: self.mc_cache.as_ref().map(|m| m.worst_case).unwrap_or(0.0),
            mc_median_outcome: self.mc_cache.as_ref().map(|m| m.median_outcome).unwrap_or(0.0),
            mc_confidence_95_range: self.mc_cache.as_ref().map(|m| m.confidence_95_range).unwrap_or((0.0, 0.0)),

            kalman_drift: self.kalman.as_ref().map(|k| k.drift).unwrap_or(0.0),
            kalman_noise_vol: self.kalman.as_ref().map(|k| k.noise_vol).unwrap_or(0.0),
            kalman_trend_strength: self.kalman.as_ref().map(|k| k.trend_strength).unwrap_or(0.0),

            regime_label: stat_regime.label,
            regime_stability: cluster_stability,
            anomaly_score,
            top_anomaly_reason: top_reason.to_string(),
            top_predictive_indicators: self.cached_predictors.clone(),

            // Derived features populated post-construction.
            ..StatisticalContext::default()
        };

        let derived = DerivedFeatures::from_context(&ctx);
        ctx.market_stretch_score = derived.market_stretch_score;
        ctx.trend_reliability = derived.trend_reliability;
        ctx.momentum_stability = derived.momentum_stability;
        ctx.volatility_shock_prob = derived.volatility_shock_prob;
        ctx.compression_probability = derived.compression_probability;
        ctx.expansion_probability = derived.expansion_probability;
        ctx.breakout_confidence = derived.breakout_confidence;
        ctx.trend_confidence = derived.trend_confidence;
        ctx.risk_confidence = derived.risk_confidence;
        ctx.expected_opportunity = derived.expected_opportunity;
        ctx.market_predictability = derived.market_predictability;

        ctx
    }

    /// Scan current indicators and queue any active trigger conditions in
    /// the Bayesian engine.  These will be resolved after forward_bars.
    fn queue_bayesian_triggers(
        &mut self,
        close: f64,
        atr: f64,
        rsi: f64,
        bbwp: f64,
        squeeze_on: bool,
    ) {
        let wi = self.distribution.best_window_idx();
        let prices = self.distribution.metric_values(wi, 0);

        // Trend continuation trigger: price on same side of SMA(20) for 3 bars.
        if prices.len() >= 3 {
            let sma20: f64 = prices.iter().rev().take(20).sum::<f64>()
                / 20.0_f64.min(prices.len() as f64);
            if sma20 > 1e-12 {
                let above = close > sma20;
                let p1 = prices[prices.len() - 2];
                let p2 = prices[prices.len() - 3];
                let above1 = p1 > sma20;
                let above2 = p2 > sma20;
                if above == above1 && above == above2 {
                    self.bayesian.queue_trigger(
                        ObservationKind::TrendContinuation, close, atr, rsi, bbwp,
                    );
                }
            }
        }

        // Mean reversion trigger: price deviated > 1.5σ from SMA.
        if prices.len() >= 20 {
            let sma20: f64 = prices.iter().rev().take(20).sum::<f64>()
                / 20.0_f64.min(prices.len() as f64);
            let var: f64 = prices.iter().rev().take(20)
                .map(|p| (p - sma20).powi(2)).sum::<f64>() / 19.0_f64;
            let std = var.sqrt().max(1e-12);
            if (close - sma20).abs() > 1.5 * std {
                self.bayesian.queue_trigger(
                    ObservationKind::MeanReversion, close, atr, rsi, bbwp,
                );
            }
        }

        // Reversal trigger: RSI extreme.
        if rsi > 70.0 || rsi < 30.0 {
            self.bayesian.queue_trigger(
                ObservationKind::Reversal, close, atr, rsi, bbwp,
            );
        }

        // ATR expansion trigger: BBWP high.
        if bbwp > 80.0 {
            self.bayesian.queue_trigger(
                ObservationKind::AtrExpansion, close, atr, rsi, bbwp,
            );
        }

        // Squeeze release trigger.
        if squeeze_on {
            // Don't queue during squeeze — queue when it releases.
            // We detect release by looking at squeeze momentum in the tracker.
            let squeeze_vals = self.distribution.metric_values(wi, 5);
            if squeeze_vals.len() >= 4 {
                let prev3 = &squeeze_vals[squeeze_vals.len() - 4..];
                let was_coiling = prev3.iter().all(|&v| v.abs() < 0.2);
                let curr_mom = squeeze_vals[squeeze_vals.len() - 1];
                if was_coiling && curr_mom.abs() > 0.3 {
                    self.bayesian.queue_trigger(
                        ObservationKind::SqueezeReleaseBullish,
                        close, atr, rsi, bbwp,
                    );
                }
            }
        }

        // Volatility expansion trigger.
        if bbwp > 60.0 {
            self.bayesian.queue_trigger(
                ObservationKind::VolatilityExpansion, close, atr, rsi, bbwp,
            );
        }

        // Stop before target trigger: every bar is a candidate for
        // hypothetical trade simulation.
        self.bayesian.queue_trigger(
            ObservationKind::StopBeforeTarget, close, atr, rsi, bbwp,
        );
    }

    /// Expose the distribution tracker for modules that need raw window
    /// access (market shape, probability, etc.).
    pub(crate) fn distribution(&self) -> &DistributionTracker {
        &self.distribution
    }

    /// Reference to the active configuration.
    pub(crate) fn config(&self) -> &StatisticsConfig {
        &self.config
    }

    /// Number of completed candles processed so far.
    pub fn bar_count(&self) -> u64 {
        self.bar_count
    }

    /// Run a Monte Carlo price-path simulation using the current price,
    /// ATR, and historical returns from the distribution tracker.  Results
    /// are cached for subsequent `advance()` calls until the next
    /// simulation run.
    ///
    /// This is called from a background task (engine Phase 9) on a
    /// configurable interval.  It is NOT called automatically in
    /// `advance()` to avoid blocking the main analysis loop.
    pub fn run_monte_carlo(&mut self, price: f64, atr: f64) {
        let wi = self.distribution.best_window_idx();
        let outcome = if let Some(ref kf) = self.kalman {
            if kf.is_ready() {
                let residuals = kf.residuals_slice();
                MonteCarloPriceOutcome::compute_with_kalman(
                    price,
                    atr,
                    kf.drift,
                    &residuals,
                    self.config.monte_carlo_target_atr_mult,
                    self.config.monte_carlo_stop_atr_mult,
                    self.config.monte_carlo_paths,
                    self.config.monte_carlo_steps,
                    Some(self.bar_count),
                )
            } else {
                let returns = self.distribution.metric_values(wi, 1);
                MonteCarloPriceOutcome::compute(
                    price,
                    atr,
                    &returns,
                    self.config.monte_carlo_target_atr_mult,
                    self.config.monte_carlo_stop_atr_mult,
                    self.config.monte_carlo_paths,
                    self.config.monte_carlo_steps,
                    Some(self.bar_count),
                )
            }
        } else {
            let returns = self.distribution.metric_values(wi, 1);
            MonteCarloPriceOutcome::compute(
                price,
                atr,
                &returns,
                self.config.monte_carlo_target_atr_mult,
                self.config.monte_carlo_stop_atr_mult,
                self.config.monte_carlo_paths,
                self.config.monte_carlo_steps,
                Some(self.bar_count),
            )
        };
        self.mc_cache = Some(outcome);
    }
}

/// Normalize raw indicator values to a [-1, 1] scale for clustering input.
/// Feature order: ADX, RSI, BBWP, Choppiness-proxy, Returns.
fn normalize_features(
    _close: f64,
    _atr: f64,
    rsi: f64,
    bbwp: f64,
    squeeze: f64,
    rvol: f64,
    adx: f64,
    log_return: f64,
) -> [f64; 5] {
    // ADX: map [0, 50] → [-1, 1] via tanh-style normalization.
    let adx_norm = ((adx / 25.0) - 1.0).clamp(-1.0, 1.0);
    // RSI: map [0, 100] → [-1, 1].
    let rsi_norm = ((rsi / 50.0) - 1.0).clamp(-1.0, 1.0);
    // BBWP: map [0, 100] → [-1, 1] — high = compressed.
    let bbwp_norm = ((bbwp / 50.0) - 1.0).clamp(-1.0, 1.0);
    // Choppiness proxy: inverse of ADX + squeeze state.
    let chop_norm = ((25.0 / adx.max(1.0)) - 1.0).clamp(-1.0, 1.0);
    // Returns: clamp to [-1, 1] via tanh.
    let ret_norm = (log_return / 5.0).clamp(-1.0, 1.0);

    let _ = (squeeze, rvol);
    [adx_norm, rsi_norm, bbwp_norm, chop_norm, ret_norm]
}
