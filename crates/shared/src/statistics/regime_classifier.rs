//! Statistical regime classification using distribution shape (Phase 7).
//!
//! Classifies the market regime using distribution statistics rather than
//! indicator values.  This is complementary to `MarketContext.regime` which
//! uses the 51-indicator normalized map.
//!
//! Regime labels: trending_up, trending_down, ranging, volatile, compressed,
//! transition.

/// Result of statistical regime classification.
#[derive(Debug, Clone)]
pub struct StatisticalRegime {
    pub label: String,
    pub confidence: f64, // [0, 1]
}

impl Default for StatisticalRegime {
    fn default() -> Self {
        Self { label: "unknown".into(), confidence: 0.0 }
    }
}

impl StatisticalRegime {
    /// Classify the market regime from distribution-shape statistics.
    ///
    /// # Arguments
    ///
    /// * `skewness`         — third moment of returns (signed)
    /// * `kurtosis`         — excess kurtosis (0 = normal)
    /// * `entropy`          — normalized entropy [0, 1]
    /// * `atr_percentile`   — current ATR rank [0, 100]
    /// * `bbwp_percentile`  — current BBWP rank [0, 100]
    /// * `trend_consistency`— lag-1 autocorrelation of returns [-1, 1]
    /// * `trend_persistence`— from DecisionContext (proxy: |trend_consistency|)
    pub fn classify(
        skewness: f64,
        kurtosis: f64,
        entropy: f64,
        atr_percentile: f64,
        bbwp_percentile: f64,
        trend_consistency: f64,
    ) -> Self {
        // Strength of directional bias.
        let has_direction = skewness.abs() > 0.3;
        let trend_strong = trend_consistency.abs() > 0.3;

        // ── Trending ──
        if has_direction && trend_strong && entropy < 0.6 && bbwp_percentile < 80.0 {
            let conf = (skewness.abs().min(1.0) * 0.4
                + trend_consistency.abs() * 0.4
                + (1.0 - entropy) * 0.2)
                .clamp(0.0, 1.0);
            let label = if skewness > 0.0 { "trending_up" } else { "trending_down" };
            return StatisticalRegime { label: label.into(), confidence: conf };
        }

        // ── Volatile / Explosive ──
        if atr_percentile > 85.0 && kurtosis > 2.0 {
            let conf = ((atr_percentile / 100.0) * 0.5
                + (kurtosis / 5.0).min(1.0) * 0.3
                + entropy.min(1.0) * 0.2)
                .clamp(0.0, 1.0);
            return StatisticalRegime { label: "volatile".into(), confidence: conf };
        }

        // ── Compressed ──
        if bbwp_percentile > 85.0 || atr_percentile < 20.0 {
            let conf = ((bbwp_percentile / 100.0) * 0.5
                + (1.0 - atr_percentile / 100.0) * 0.5)
                .clamp(0.0, 1.0);
            return StatisticalRegime { label: "compressed".into(), confidence: conf };
        }

        // ── Ranging ──
        if !has_direction && entropy > 0.5 {
            let conf = (entropy * 0.6 + (1.0 - trend_consistency.abs()) * 0.4).clamp(0.0, 1.0);
            return StatisticalRegime { label: "ranging".into(), confidence: conf };
        }

        // ── Transition ──
        StatisticalRegime { label: "transition".into(), confidence: 0.3 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trending_up() {
        let r = StatisticalRegime::classify(0.7, 1.0, 0.3, 50.0, 50.0, 0.6);
        assert_eq!(r.label, "trending_up");
        assert!(r.confidence > 0.5);
    }

    #[test]
    fn test_trending_down() {
        let r = StatisticalRegime::classify(-0.7, 1.0, 0.3, 50.0, 50.0, 0.6);
        assert_eq!(r.label, "trending_down");
        assert!(r.confidence > 0.5);
    }

    #[test]
    fn test_volatile() {
        let r = StatisticalRegime::classify(0.0, 3.0, 0.7, 90.0, 50.0, 0.1);
        assert_eq!(r.label, "volatile");
    }

    #[test]
    fn test_compressed() {
        let r = StatisticalRegime::classify(0.0, 0.5, 0.4, 50.0, 92.0, 0.1);
        assert_eq!(r.label, "compressed");
    }

    #[test]
    fn test_ranging() {
        let r = StatisticalRegime::classify(0.1, 0.5, 0.7, 50.0, 40.0, 0.1);
        assert_eq!(r.label, "ranging");
    }

    #[test]
    fn test_transition() {
        let r = StatisticalRegime::classify(0.1, 1.0, 0.5, 50.0, 50.0, 0.2);
        assert_eq!(r.label, "transition");
    }
}
