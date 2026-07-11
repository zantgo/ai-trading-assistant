//! # Decision Matrix — Market Bias per Symbol
//!
//! The Decision Matrix consumes the Confluence Matrix to produce a
//! directional market bias (Bullish / Bearish / Neutral) for one symbol.
//! It uses MTF alignment from the Confluence Matrix plus signal activity
//! and regime context to derive confidence-adjusted market bias.
//!
//! Layer: L4.5 in the 9-layer architecture (between Confluence and State).

use crate::alignment::AlignmentMatrix;
use serde::{Deserialize, Serialize};

/// Directional market bias for a single symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarketBias {
    Bullish,
    Bearish,
    Neutral,
}

impl std::fmt::Display for MarketBias {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MarketBias::Bullish => write!(f, "BULLISH"),
            MarketBias::Bearish => write!(f, "BEARISH"),
            MarketBias::Neutral => write!(f, "NEUTRAL"),
        }
    }
}

/// Decision Matrix output for one symbol.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalysisMatrix {
    pub symbol: String,
    pub bias: MarketBias,
    pub confidence: f64,
    pub rationale: String,
    pub supporting_signals: Vec<String>,
    pub contradicting_signals: Vec<String>,
    pub timeframes_considered: u8,
}

impl AnalysisMatrix {
    /// Create an empty AnalysisMatrix for a symbol with no data.
    pub fn empty(symbol: &str) -> Self {
        Self {
            symbol: symbol.to_string(),
            bias: MarketBias::Neutral,
            confidence: 0.0,
            rationale: "No data available — no candles have been completed.".to_string(),
            supporting_signals: Vec::new(),
            contradicting_signals: Vec::new(),
            timeframes_considered: 0,
        }
    }
}

/// Derive a AnalysisMatrix from a AlignmentMatrix.
///
/// The Decision bias is based on the MTF overall score with confidence
/// modifiers for trend agreement, cross-TF signals, and timeframe count.
pub fn derive_analysis(confluence: &AlignmentMatrix) -> AnalysisMatrix {
    if confluence.timeframes_present == 0 {
        return AnalysisMatrix::empty(&confluence.symbol);
    }

    let score = confluence.mtf_overall_score;
    let bias = if score > 20.0 {
        MarketBias::Bullish
    } else if score < -20.0 {
        MarketBias::Bearish
    } else {
        MarketBias::Neutral
    };

    // Base confidence = |score| / 100 clamped to 0-1
    let base_confidence = (score.abs() / 100.0).clamp(0.0, 1.0);

    // Modifier: Trend agreement across timeframes
    let trend_agreement = confluence.trend_agreement_pct;
    let mut confidence = if trend_agreement >= 75.0 {
        (base_confidence + 0.15).min(1.0)
    } else if trend_agreement < 50.0 {
        base_confidence.min(0.5)
    } else {
        base_confidence
    };

    // Modifier: Cross-TF signal count
    let cross_tf = confluence.signal_cross_tf_count;
    if cross_tf >= 3 {
        confidence = (confidence + 0.1 * (cross_tf as f64).min(5.0) / 5.0).min(1.0);
    }

    // Modifier: Low TF count caps confidence
    let tf_count = confluence.timeframes_present;
    if tf_count <= 1 {
        confidence = confidence.min(0.5);
    } else if tf_count == 2 {
        confidence = confidence.min(0.75);
    }

    // Build rationale
    let mut rationale_parts: Vec<String> = Vec::new();

    let bias_str = match bias {
        MarketBias::Bullish => "bullish",
        MarketBias::Bearish => "bearish",
        MarketBias::Neutral => "neutral",
    };

    rationale_parts.push(format!(
        "MTF overall score {:.0}/100 → {} bias. {} of {} timeframes agree on direction ({:.0}%).",
        score, bias_str, if trend_agreement >= 50.0 { "Majority" } else { "Minority" },
        tf_count, trend_agreement
    ));

    if cross_tf > 0 {
        rationale_parts.push(format!(
            "{} signals appear across multiple timeframes.", cross_tf
        ));
    }

    if tf_count <= 2 {
        rationale_parts.push(
            "Limited timeframe data — confidence capped.".to_string()
        );
    }

    // Collect regime info from alignments
    let regimes: Vec<&str> = confluence.timeframe_alignments
        .iter()
        .map(|a| a.regime.as_str())
        .collect();

    if !regimes.is_empty() {
        let unique_regimes: std::collections::HashSet<&str> =
            regimes.iter().copied().collect();
        if unique_regimes.len() == 1 {
            rationale_parts.push(format!(
                "All timeframes in {} regime.", unique_regimes.iter().next().unwrap()
            ));
        }
    }

    let mut supporting: Vec<String> = Vec::new();
    let mut contradicting: Vec<String> = Vec::new();

    for tf in &confluence.timeframe_alignments {
        let direction = if tf.overall_score > 0 {
            "bullish"
        } else if tf.overall_score < 0 {
            "bearish"
        } else {
            "neutral"
        };

        let label = format!(
            "{} ({}): score {:+}, {} regime, {} active signals",
            tf.timeframe, direction, tf.overall_score, tf.regime, tf.active_signals
        );

        // Classify as supporting or contradicting based on alignment with the bias
        let aligns = match (bias, tf.overall_score) {
            (MarketBias::Bullish, s) if s > 0 => true,
            (MarketBias::Bearish, s) if s < 0 => true,
            (MarketBias::Neutral, s) if s.abs() < 10 => true,
            _ => false,
        };

        if aligns {
            supporting.push(label);
        } else {
            contradicting.push(label);
        }
    }

    AnalysisMatrix {
        symbol: confluence.symbol.clone(),
        bias,
        confidence,
        rationale: rationale_parts.join(" "),
        supporting_signals: supporting,
        contradicting_signals: contradicting,
        timeframes_considered: tf_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alignment::TfAlignmentInfo;

    fn simple_confluence(symbol: &str, tfs: u8, score: f64, agreement: f64, cross_tf: u32) -> AlignmentMatrix {
        let mut alignments = Vec::new();
        for i in 0..tfs {
            let labels = ["micro60", "fast180", "slow300", "macro900"];
            let secs = [60, 180, 300, 900];
            alignments.push(TfAlignmentInfo {
                timeframe: labels[i as usize].to_string(),
                timeframe_secs: secs[i as usize],
                trend_score: score / 100.0,
                momentum_score: score / 120.0,
                overall_score: score as i32,
                regime: if score.abs() > 30.0 { "TRENDING".into() } else { "RANGE".into() },
                active_signals: if cross_tf > 0 { 2 } else { 0 },
                price: 64000.0,
            });
        }

        AlignmentMatrix {
            symbol: symbol.to_string(),
            timeframes_present: tfs,
            mtf_trend_alignment: score / 100.0,
            mtf_momentum_alignment: score / 120.0,
            mtf_volume_alignment: 0.0,
            mtf_volatility_alignment: 0.0,
            mtf_overall_score: score,
            mtf_overall_label: if score > 20.0 { "WEAK_BULL_MTF".into() } else { "NEUTRAL_MTF".into() },
            timeframe_alignments: alignments,
            signal_cross_tf_count: cross_tf,
            trend_agreement_pct: agreement,
        }
    }

    #[test]
    fn strong_bullish_mtf_produces_bullish_decision() {
        let c = simple_confluence("BTC-USD", 4, 75.0, 100.0, 4);
        let d = derive_analysis(&c);
        assert_eq!(d.bias, MarketBias::Bullish);
        assert!(d.confidence > 0.7);
        assert_eq!(d.timeframes_considered, 4);
    }

    #[test]
    fn neutral_score_produces_neutral_decision() {
        let c = simple_confluence("ETH-USD", 4, 10.0, 40.0, 0);
        let d = derive_analysis(&c);
        assert_eq!(d.bias, MarketBias::Neutral);
        assert!(d.confidence <= 0.5);
    }

    #[test]
    fn single_tf_caps_confidence() {
        let c = simple_confluence("BTC-USD", 1, 80.0, 100.0, 0);
        let d = derive_analysis(&c);
        assert_eq!(d.bias, MarketBias::Bullish);
        assert!(d.confidence <= 0.5, "single TF should cap confidence at 0.5, got {}", d.confidence);
    }

    #[test]
    fn empty_confluence_returns_neutral() {
        let c = AlignmentMatrix::empty("BTC-USD");
        let d = derive_analysis(&c);
        assert_eq!(d.bias, MarketBias::Neutral);
        assert_eq!(d.timeframes_considered, 0);
    }
}
