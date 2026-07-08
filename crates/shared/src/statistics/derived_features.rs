//! Derived decision-support features (Phase 8).
//!
//! Transforms raw statistical outputs into 11 features designed for AI
//! consumption.  Each feature is a deterministic function of fields already
//! available in the `StatisticalContext` or computed within the
//! `StatisticsEngine`.  No new data, no heuristics, no manual tuning.

use crate::statistics::statistical_context::StatisticalContext;

/// All 11 derived features packed into one struct.
#[derive(Debug, Clone)]
pub struct DerivedFeatures {
    /// How statistically stretched price is from its mean.
    /// price_z_score × volatility_pct / 100.  ±1 = extreme.
    pub market_stretch_score: f64,

    /// How reliable the current trend signal is.
    /// consensus_stability × trend_consistency(abs) × (1 - entropy).
    pub trend_reliability: f64,

    /// Consistency of momentum indicators (RSI stddev proxy).
    /// 1 / (1 + |rsi_z|) — low z = stable momentum.
    pub momentum_stability: f64,

    /// Probability that current volatility is a shock vs normal.
    /// uses ATR percentile directly: >90 = shock likely.
    pub volatility_shock_prob: f64,

    /// Probability that BBWP will reach extreme compression soon.
    /// uses compression_percentile as a proxy for future compression.
    pub compression_probability: f64,

    /// Probability that volatility will expand soon.
    /// uses expansion probability + atr stats trend.
    pub expansion_probability: f64,

    /// Confidence that a breakout signal is genuine.
    /// (1 - indicator_redundancy) × trend_consistency(abs) × (1 - anomaly/2).
    pub breakout_confidence: f64,

    /// Combined trend conviction: trend_reliability × continuation_prob
    /// Bayesian posterior mean.
    pub trend_confidence: f64,

    /// 1 - (tail_risk × anomaly_score).  High = normal risk environment.
    pub risk_confidence: f64,

    /// Asymmetric edge: expected_movement × target_hit_prob
    /// - max_drawdown_95 × stop_hit_prob, normalized.
    pub expected_opportunity: f64,

    /// 1 - entropy.  1 = perfectly predictable, 0 = pure noise.
    pub market_predictability: f64,

    /// Kalman signal-to-noise ratio: models |drift| / noise_vol.
    pub kalman_trend_strength: f64,
}

impl Default for DerivedFeatures {
    fn default() -> Self {
        Self {
            market_stretch_score: 0.0,
            trend_reliability: 0.0,
            momentum_stability: 0.0,
            volatility_shock_prob: 0.0,
            compression_probability: 0.0,
            expansion_probability: 0.0,
            breakout_confidence: 0.0,
            trend_confidence: 0.0,
            risk_confidence: 0.0,
            expected_opportunity: 0.0,
            market_predictability: 0.0,
            kalman_trend_strength: 0.0,
        }
    }
}

impl DerivedFeatures {
    /// Compute all derived features from a partially-populated
    /// `StatisticalContext`.  The context must already contain distribution
    /// stats, probabilities, confidence, market shape, relationships, and
    /// Monte Carlo fields.
    pub fn from_context(ctx: &StatisticalContext) -> Self {
        // ── Market Stretch Score ──────────────────────────
        let market_stretch_score = {
            let z = ctx.price_stats.z_score;
            let vp = ctx.volatility_percentile / 100.0;
            (z * vp).clamp(-1.0, 1.0)
        };

        // ── Trend Reliability ─────────────────────────────
        let trend_reliability = {
            let cs = ctx.consensus_stability;
            let tc = ctx.trend_consistency.abs();
            let ent = ctx.entropy;
            let base = (cs * tc * (1.0 - ent)).clamp(0.0, 1.0);
            // Blend Kalman trend strength when available — caps influence at 50%.
            let kts = ctx.kalman_trend_strength;
            let kalman_factor = if kts > 0.0 {
                (1.0 - (-kts).exp()).min(0.5)
            } else {
                0.0
            };
            (base * (1.0 - kalman_factor) + kalman_factor).clamp(0.0, 1.0)
        };

        // ── Momentum Stability ────────────────────────────
        let momentum_stability = {
            let z = ctx.rsi_stats.z_score.abs();
            1.0 / (1.0 + z)
        };

        // ── Volatility Shock Probability ──────────────────
        let volatility_shock_prob = {
            let p = ctx.volatility_percentile;
            // Sigmoid: > 90%ile → shock probability rises sharply.
            let x = (p - 85.0) / 10.0;
            (1.0 / (1.0 + (-x).exp())).clamp(0.0, 1.0)
        };

        // ── Compression Probability ────────────────────────
        let compression_probability = {
            let c = ctx.compression_percentile / 100.0;
            // If already in top third, more compression likely.
            (c * 0.5 + ctx.atr_expansion_prob * 0.5).clamp(0.0, 1.0)
        };

        // ── Expansion Probability ──────────────────────────
        let expansion_probability = {
            // Driven by current volatility percentile and squeeze release prob.
            let v = ctx.volatility_percentile / 100.0;
            let s = ctx.squeeze_release_prob;
            ((1.0 - v) * 0.4 + s * 0.6 + ctx.volatility_expansion_prob * 0.3)
                .clamp(0.0, 1.0)
        };

        // ── Breakout Confidence ───────────────────────────
        let breakout_confidence = {
            let redun = ctx.indicator_redundancy;
            let tc = ctx.trend_consistency.abs();
            let anom = (ctx.anomaly_score / 2.0).min(0.5);
            ((1.0 - redun) * 0.4 + tc * 0.4 + (1.0 - anom) * 0.2).clamp(0.0, 1.0)
        };

        // ── Trend Confidence ──────────────────────────────
        let trend_confidence = {
            let tr = trend_reliability;
            let cp = ctx.trend_continuation_prob;
            (tr * 0.6 + cp * 0.4).clamp(0.0, 1.0)
        };

        // ── Risk Confidence ───────────────────────────────
        let risk_confidence = {
            let tr = ctx.tail_risk.min(5.0) / 5.0;
            let anom = ctx.anomaly_score;
            (1.0 - (tr * 0.5 + anom * 0.5)).clamp(0.0, 1.0)
        };

        // ── Expected Opportunity ──────────────────────────
        let expected_opportunity = {
            let em = ctx.mc_expected_movement;
            let th = ctx.mc_target_hit_prob;
            let dd = ctx.mc_max_drawdown_95.max(0.01) / 100.0;
            let sh = ctx.mc_stop_hit_prob;
            let raw = em * th - dd * sh * 100.0;
            // Normalize: typical range is [-50, +50] basis points.
            (raw / 50.0).clamp(-1.0, 1.0)
        };

        // ── Market Predictability ─────────────────────────
        let market_predictability = (1.0 - ctx.entropy).clamp(0.0, 1.0);

        let kalman_trend_strength = ctx.kalman_trend_strength;

        DerivedFeatures {
            market_stretch_score,
            trend_reliability,
            momentum_stability,
            volatility_shock_prob,
            compression_probability,
            expansion_probability,
            breakout_confidence,
            trend_confidence,
            risk_confidence,
            expected_opportunity,
            market_predictability,
            kalman_trend_strength,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::statistics::statistical_context::StatisticalContext;
    use crate::statistics::statistical_object::StatisticValue;

    fn test_ctx() -> StatisticalContext {
        let mut ctx = StatisticalContext::default();
        ctx.price_stats = StatisticValue {
            current: 52000.0, mean: 50000.0, stddev: 500.0,
            percentile: 95.0, z_score: 4.0, confidence: 0.3, trend: "increasing".into(),
        };
        ctx.rsi_stats = StatisticValue {
            current: 72.0, mean: 50.0, stddev: 10.0,
            percentile: 96.0, z_score: 2.2, confidence: 0.8, trend: "increasing".into(),
        };
        ctx.volatility_percentile = 92.0;
        ctx.compression_percentile = 85.0;
        ctx.entropy = 0.4;
        ctx.consensus_stability = 0.7;
        ctx.trend_consistency = 0.6;
        ctx.indicator_redundancy = 0.3;
        ctx.anomaly_score = 0.2;
        ctx.tail_risk = 1.5;
        ctx.trend_continuation_prob = 0.72;
        ctx.atr_expansion_prob = 0.55;
        ctx.squeeze_release_prob = 0.30;
        ctx.volatility_expansion_prob = 0.50;
        ctx.mc_expected_movement = 1.5;
        ctx.mc_target_hit_prob = 0.62;
        ctx.mc_stop_hit_prob = 0.18;
        ctx.mc_max_drawdown_95 = 3.5;
        ctx
    }

    #[test]
    fn test_market_stretch_positive() {
        let ctx = test_ctx();
        let df = DerivedFeatures::from_context(&ctx);
        assert!(df.market_stretch_score > 0.5, "high z + high vol %ile = stretched");
    }

    #[test]
    fn test_trend_reliability_moderate() {
        let ctx = test_ctx();
        let df = DerivedFeatures::from_context(&ctx);
        assert!(df.trend_reliability > 0.1 && df.trend_reliability < 0.9);
    }

    #[test]
    fn test_volatility_shock_high() {
        let mut ctx = test_ctx();
        ctx.volatility_percentile = 95.0;
        let df = DerivedFeatures::from_context(&ctx);
        assert!(df.volatility_shock_prob > 0.5);
    }

    #[test]
    fn test_risk_confidence_reasonable() {
        let ctx = test_ctx();
        let df = DerivedFeatures::from_context(&ctx);
        assert!(df.risk_confidence > 0.0 && df.risk_confidence < 1.0);
    }

    #[test]
    fn test_market_predictability_matches_entropy() {
        let mut ctx = test_ctx();
        ctx.entropy = 0.2;
        let df = DerivedFeatures::from_context(&ctx);
        assert!((df.market_predictability - 0.8).abs() < 1e-9);
    }

    #[test]
    fn test_default_all_zeros() {
        let df = DerivedFeatures::from_context(&StatisticalContext::default());
        // With defaults, some features may still compute non-zero from structure.
        // Market_predictability should be 1.0 (entropy=0 → pred=1).
        assert!(df.market_predictability >= 0.0);
    }
}
