//! # Analysis Matrix — Market Intelligence Layer
//!
//! The Analysis Matrix transforms structured observations and multi-timeframe
//! relationships into a complete interpretation of current market conditions.
//! It represents the transition from market observation to market understanding.
//!
//! Layer: L4.5 in the architecture (Market Intelligence).

use crate::alignment::AlignmentMatrix;
use serde::{Deserialize, Serialize};

/// Generic `[low, high]` price range used for per-profile zones, the
/// aggregated per-direction zones, and the legacy `entry_zone`/`target_zone`
/// fields. Lives here (rather than in `opportunity.rs`) so per-profile
/// zone fields on `OpportunityProfile` can reference it without a
/// cross-crate import cycle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PriceRange {
    pub low: f64,
    pub high: f64,
}

/// Direction family an `OpportunityType` belongs to. Used by
/// `derive_profile_zones` to resolve a profile's actual trade direction
/// from `Analysis.bias`. `TrendRiding` follows the macro bias;
/// `CounterTrend` flips it; `Neutral` carries no actionable setup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DirectionFamily {
    /// Profile rides the prevailing trend (TrendContinuation, Breakout,
    /// Pullback, Scalp, LiquiditySqueeze). Resolves to LONG when
    /// `Analysis.bias` is Bullish/StrongBullish, SHORT when Bearish/
    /// StrongBearish, NEUTRAL otherwise.
    TrendRiding,
    /// Profile counters the prevailing trend (MeanReversion, Reversal).
    /// Resolves to the OPPOSITE of the macro bias.
    CounterTrend,
    /// Profile is direction-neutral (NoClearOpportunity). Carries no zones.
    Neutral,
}

/// Trade viability of an `OpportunityProfile`. Tells the operator whether
/// the profile carries an actionable bracket (zones pass per-side
/// invariants on a resolvable direction) or whether the profile is
/// informational only. Set by the L4 producer and surfaced in the UI as
/// a coloured badge next to each qualifying profile's preconditions bar.
///
/// Wire format is SCREAMING_SNAKE_CASE. Optional on the wire so older
/// snapshots deserialize cleanly (`trade_viability = None` ⇒
/// `TradeViability::NoClear`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TradeViability {
    /// Profile qualifies AND its per-side zones pass the LONG/SHORT
    /// geometry invariants. Operator can act on this profile.
    Actionable,
    /// Profile qualifies (preconditions met) but `selectProfileSide`
    /// resolves to NEUTRAL — typically Neutral family + Neutral bias.
    /// The aggregated Neutral sentinel (close-pinned) is the only
    /// available bracket; R:R is 0.
    DirectionalNeutral,
    /// Profile qualifies and direction resolves, but per-side invariants
    /// failed (entry/target/SL geometrically inconsistent). Stale zones.
    GeometryInverted,
    /// `NoClearOpportunity` fallback. Never actionable regardless of
    /// preconditions count.
    #[default]
    NoClear,
}

/// Direction family assigned to each `OpportunityType`. Total over all
/// eight variants — see `compute_candidate_score` test
/// `direction_family_table_is_total`.
pub fn direction_family_for(ot: OpportunityType) -> DirectionFamily {
    match ot {
        OpportunityType::TrendContinuation
        | OpportunityType::Breakout
        | OpportunityType::Pullback
        | OpportunityType::Scalp
        | OpportunityType::LiquiditySqueeze => DirectionFamily::TrendRiding,
        OpportunityType::MeanReversion | OpportunityType::Reversal => DirectionFamily::CounterTrend,
        OpportunityType::NoClearOpportunity => DirectionFamily::Neutral,
    }
}

/// Directional market bias.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarketBias {
    StrongBullish,
    Bullish,
    Neutral,
    Bearish,
    StrongBearish,
}

impl std::fmt::Display for MarketBias {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MarketBias::StrongBullish => write!(f, "STRONG_BULLISH"),
            MarketBias::Bullish => write!(f, "BULLISH"),
            MarketBias::Neutral => write!(f, "NEUTRAL"),
            MarketBias::Bearish => write!(f, "BEARISH"),
            MarketBias::StrongBearish => write!(f, "STRONG_BEARISH"),
        }
    }
}

/// Market regime classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarketRegime {
    TrendingBull,
    TrendingBear,
    Range,
    Accumulation,
    Distribution,
    Expansion,
    Contraction,
    Transition,
}

impl std::fmt::Display for MarketRegime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MarketRegime::TrendingBull => write!(f, "TRENDING_BULL"),
            MarketRegime::TrendingBear => write!(f, "TRENDING_BEAR"),
            MarketRegime::Range => write!(f, "RANGE"),
            MarketRegime::Accumulation => write!(f, "ACCUMULATION"),
            MarketRegime::Distribution => write!(f, "DISTRIBUTION"),
            MarketRegime::Expansion => write!(f, "EXPANSION"),
            MarketRegime::Contraction => write!(f, "CONTRACTION"),
            MarketRegime::Transition => write!(f, "TRANSITION"),
        }
    }
}

/// Trend quality assessment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrendAssessment {
    Weak,
    Developing,
    Healthy,
    Strong,
    Exhausted,
}

/// Momentum state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MomentumAssessment {
    Increasing,
    Stable,
    Weakening,
    Exhausted,
    Reversing,
}

/// Structure state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StructureAssessment {
    Strong,
    Healthy,
    Weak,
    Broken,
    Unknown,
}

/// Volatility state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VolatilityAssessment {
    Compressed,
    Normal,
    Expanding,
    Extreme,
    Unstable,
}

/// Volume participation state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VolumeAssessment {
    Weak,
    Normal,
    Strong,
    Exceptional,
}

/// Opportunity type classification — canonical 8-variant enum.
/// This is the authoritative home of the setup selector in the institutional
/// redesign; the Opportunity Matrix (L4) is its sole producer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum OpportunityType {
    TrendContinuation,
    Breakout,
    Pullback,
    MeanReversion,
    Reversal,
    LiquiditySqueeze,
    Scalp,
    #[default]
    NoClearOpportunity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SetupQuality {
    Prime,
    Strong,
    Moderate,
    Marginal,
    #[default]
    None,
}

/// Per-setup-type scored profile.
///
/// `OpportunityMatrix.profiles` carries one entry per `OpportunityType`
/// (eight max). When the profile's preconditions are satisfied AND
/// `OpportunityType != NoClearOpportunity`, the matching per-side zones
/// are populated:
///   - `direction_family` = `TrendRiding`: zones are emitted on the same
///     side as `Analysis.bias` (LONG-profile has `long_*` set, SHORT-profile
///     has `short_*` set).
///   - `direction_family` = `CounterTrend`: zones are emitted on the
///     opposite side of `Analysis.bias` (MeanReversion against a bullish
///     bias populates `short_*`).
///   - `direction_family` = `Neutral`: every zone field is `None`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpportunityProfile {
    pub opportunity_type: OpportunityType,
    pub score: f64,
    pub preconditions_met: u32,
    pub preconditions_total: u32,
    pub notes: String,
    /// Direction family this profile implies. Populated by the L4 producer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction_family: Option<DirectionFamily>,
    /// LONG-side entry zone (entry below close). Populated only when
    /// the profile resolves to LONG (TrendRiding + bullish bias, or
    /// CounterTrend + bearish bias).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub long_entry_zone: Option<PriceRange>,
    /// LONG-side target zone (target above close).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub long_target_zone: Option<PriceRange>,
    /// LONG-side invalidation level (price below which the long thesis
    /// is invalidated).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub long_invalidation_level: Option<f64>,
    /// SHORT-side entry zone (entry above close).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub short_entry_zone: Option<PriceRange>,
    /// SHORT-side target zone (target below close).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub short_target_zone: Option<PriceRange>,
    /// SHORT-side invalidation level (price above which the short thesis
    /// is invalidated).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub short_invalidation_level: Option<f64>,
    /// Per-side expected reward/risk ratio derived from the per-profile
    /// zones (faithful sign-aware R:R; never the legacy `2.5` mask).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub long_expected_rr_internal: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub short_expected_rr_internal: Option<f64>,
    /// Trade viability classification for this profile. The L4 producer
    /// sets this so the UI can color-code the card. `None` on legacy
    /// payloads — UI should treat `None` as `NoClear`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trade_viability: Option<TradeViability>,
    /// Internal-only scoring factors (raw blend, precondition ratio).
    /// NEVER serialised on the wire — operators don't see these. Kept in
    /// the Rust struct for telemetry consumers that read profiles
    /// directly from `OpportunityMatrix`. UI panels read `notes` for the
    /// user-facing rationale.
    #[serde(skip)]
    pub scoring_factors: Option<ScoringFactors>,
}

/// Internal scoring factors for an `OpportunityProfile`. Kept on the
/// struct but skipped by serde so the wire stays clean.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoringFactors {
    pub raw_score: f64,
    pub precondition_ratio: f64,
}

/// Wyckoff-style market-cycle phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarketPhase {
    Accumulation,
    Markup,
    Distribution,
    Markdown,
    Unknown,
}

impl std::fmt::Display for MarketPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Market quality level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QualityLevel {
    Poor,
    Weak,
    Average,
    Good,
    Excellent,
}

impl std::fmt::Display for QualityLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Analysis Matrix — complete market interpretation per symbol.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalysisMatrix {
    pub symbol: String,
    pub bias: MarketBias,
    pub market_bias_score: f64,
    pub state_confidence: f64,
    /// UI-facing companion to `state_confidence`. Mirrors `state_confidence`
    /// so downstream panels (Analysis / Opportunities / Market Overview) can
    /// read `analysis.confidence` directly. `state_confidence` retains its
    /// canonical wire-format name `STATE_CONFIDENCE` per docs/matrices
    /// `02-00b-confidence-hierarchy.md`.
    pub confidence: f64,
    pub market_regime: MarketRegime,
    pub trend_assessment: TrendAssessment,
    pub momentum_assessment: MomentumAssessment,
    pub structure_assessment: StructureAssessment,
    pub volatility_assessment: VolatilityAssessment,
    pub volume_assessment: VolumeAssessment,
    pub opportunity_analysis: OpportunityType,
    pub market_quality: QualityLevel,
    pub market_quality_score: f64,
    pub market_phase: MarketPhase,
    pub market_interpretation: String,
    pub rationale: String,
    pub supporting_signals: Vec<String>,
    pub contradicting_signals: Vec<String>,
    pub timeframes_considered: u8,
}

impl AnalysisMatrix {
    pub fn empty(symbol: &str) -> Self {
        Self {
            symbol: symbol.to_string(),
            bias: MarketBias::Neutral,
            market_bias_score: 0.0,
            state_confidence: 0.0,
            confidence: 0.0,
            market_regime: MarketRegime::Transition,
            trend_assessment: TrendAssessment::Weak,
            momentum_assessment: MomentumAssessment::Stable,
            structure_assessment: StructureAssessment::Unknown,
            volatility_assessment: VolatilityAssessment::Normal,
            volume_assessment: VolumeAssessment::Normal,
            opportunity_analysis: OpportunityType::NoClearOpportunity,
            market_quality: QualityLevel::Poor,
            market_quality_score: 0.0,
            market_phase: MarketPhase::Unknown,
            market_interpretation: "No data available — no candles have been completed.".into(),
            rationale: String::new(),
            supporting_signals: Vec::new(),
            contradicting_signals: Vec::new(),
            timeframes_considered: 0,
        }
    }
}

/// Derive an Analysis Matrix from the Alignment Matrix, optionally enriched with
/// per-timeframe indicator data (BBWP, ADX) and prior-bar state for the full
/// 8-state regime decision tree.
///
/// - `bbwp`: Bollinger Band Width Percentile from the representative indicator map.
/// - `adx`: ADX raw value from the representative indicator map.
/// - `previous_score`: the prior bar's `mtf_overall_score` for slope calculation.
/// - `previous_regime`: the regime from the previous bar for transition/stickiness detection.
/// - `previous_volume_dim`: the prior bar's volume dimension score (dim 2) for Wyckoff MarketPhase trend.
pub fn derive_analysis(
    alignment: &AlignmentMatrix,
    bbwp: Option<f64>,
    adx: Option<f64>,
    previous_score: Option<f64>,
    previous_regime: Option<MarketRegime>,
    previous_volume_dim: Option<f64>,
) -> AnalysisMatrix {
    if alignment.timeframes_present == 0 {
        return AnalysisMatrix::empty(&alignment.symbol);
    }

    let score = alignment.mtf_overall_score;
    let bias = if score > 40.0 {
        MarketBias::StrongBullish
    } else if score > 20.0 {
        MarketBias::Bullish
    } else if score < -40.0 {
        MarketBias::StrongBearish
    } else if score < -20.0 {
        MarketBias::Bearish
    } else {
        MarketBias::Neutral
    };

    let base_state_confidence = (score.abs() / 100.0).max(0.0).min(1.0);
    let mut state_confidence = base_state_confidence;
    if alignment.trend_agreement_pct >= 75.0 {
        state_confidence = (state_confidence + 0.15).min(1.0);
    } else if alignment.trend_agreement_pct < 50.0 {
        state_confidence = state_confidence.min(0.5);
    }
    if alignment.signal_cross_tf_count >= 3 {
        state_confidence = (state_confidence + 0.1).min(1.0);
    }
    if alignment.timeframes_present <= 1 {
        state_confidence = state_confidence.min(0.5);
    }

    let bbwp_val = bbwp.unwrap_or(50.0);
    let adx_val = adx.unwrap_or(25.0);
    let score_slope = previous_score.map(|prev| score - prev).unwrap_or(0.0);
    let regime_shifted = previous_regime.is_some_and(|prev| prev != MarketRegime::Range);

    // ── 8-state regime decision tree (6 priority levels) ──
    let is_expansion = bbwp_val >= 85.0;

    let regime = if bbwp_val >= 85.0 {
        MarketRegime::Expansion
    } else if bbwp_val <= 10.0 {
        MarketRegime::Contraction
    } else if adx_val >= 25.0 && score > 20.0 {
        MarketRegime::TrendingBull
    } else if adx_val >= 25.0 && score < -20.0 {
        MarketRegime::TrendingBear
    } else if score_slope > 0.0 && score >= 0.0 && !is_expansion {
        MarketRegime::Accumulation
    } else if score_slope < 0.0 && score <= 0.0 && !is_expansion {
        MarketRegime::Distribution
    } else if adx_val < 25.0 && bbwp_val > 10.0 && bbwp_val < 85.0 && regime_shifted {
        MarketRegime::Transition
    } else {
        MarketRegime::Range
    };

    // Trend assessment from alignment trend dimension
    let trend_dim = alignment.dimensions.get(0).map(|d| d.score).unwrap_or(50.0);
    let trend_assessment = if trend_dim >= 90.0 {
        TrendAssessment::Strong
    } else if trend_dim >= 75.0 {
        TrendAssessment::Healthy
    } else if trend_dim >= 50.0 {
        TrendAssessment::Developing
    } else if trend_dim >= 25.0 {
        TrendAssessment::Weak
    } else {
        TrendAssessment::Exhausted
    };

    // Momentum from alignment momentum dimension
    let mom_dim = alignment.dimensions.get(1).map(|d| d.score).unwrap_or(50.0);
    let momentum_assessment = if mom_dim >= 80.0 {
        MomentumAssessment::Increasing
    } else if mom_dim >= 60.0 {
        MomentumAssessment::Stable
    } else if mom_dim >= 40.0 {
        MomentumAssessment::Weakening
    } else {
        MomentumAssessment::Reversing
    };

    // Structure from alignment structure dimension
    let struct_dim = alignment.dimensions.get(4).map(|d| d.score).unwrap_or(50.0);
    let structure_assessment = if struct_dim >= 80.0 {
        StructureAssessment::Strong
    } else if struct_dim >= 60.0 {
        StructureAssessment::Healthy
    } else if struct_dim >= 40.0 {
        StructureAssessment::Weak
    } else if struct_dim >= 20.0 {
        StructureAssessment::Broken
    } else {
        StructureAssessment::Unknown
    };

    // Volatility from alignment volatility dimension
    let vol_dim = alignment.dimensions.get(3).map(|d| d.score).unwrap_or(50.0);
    let volatility_assessment = if vol_dim >= 90.0 {
        VolatilityAssessment::Extreme
    } else if vol_dim >= 70.0 {
        VolatilityAssessment::Expanding
    } else if vol_dim >= 40.0 {
        VolatilityAssessment::Normal
    } else if vol_dim >= 20.0 {
        VolatilityAssessment::Compressed
    } else {
        VolatilityAssessment::Unstable
    };

    // Volume from alignment volume dimension
    let volu_dim = alignment.dimensions.get(2).map(|d| d.score).unwrap_or(50.0);
    let volume_assessment = if volu_dim >= 90.0 {
        VolumeAssessment::Exceptional
    } else if volu_dim >= 70.0 {
        VolumeAssessment::Strong
    } else if volu_dim >= 40.0 {
        VolumeAssessment::Normal
    } else {
        VolumeAssessment::Weak
    };

    // ── MarketPhase: Wyckoff-style market-cycle phase (§3.9) ──
    let is_low_volatility = vol_dim >= 20.0 && vol_dim < 40.0;
    let is_price_trending_up = score > 20.0;
    let is_price_trending_down = score < -20.0;
    let is_volume_strong = volu_dim >= 70.0;
    let is_structure_healthy = struct_dim >= 60.0;
    let is_structure_weak = struct_dim < 60.0;
    let is_bullish = matches!(bias, MarketBias::Bullish | MarketBias::StrongBullish);
    let is_bearish = matches!(bias, MarketBias::Bearish | MarketBias::StrongBearish);

    let volume_rising = previous_volume_dim
        .map(|prev| volu_dim > prev + 5.0)
        .unwrap_or(false);
    let volume_falling = previous_volume_dim
        .map(|prev| volu_dim < prev - 5.0)
        .unwrap_or(false);

    let market_phase = if is_low_volatility && volume_rising && is_structure_healthy {
        MarketPhase::Accumulation
    } else if is_price_trending_up && is_volume_strong && is_bullish {
        MarketPhase::Markup
    } else if is_low_volatility && volume_falling && is_structure_weak {
        MarketPhase::Distribution
    } else if is_price_trending_down && is_volume_strong && is_bearish {
        MarketPhase::Markdown
    } else {
        MarketPhase::Unknown
    };

    // Opportunity from alignment dimensions (deprecated — L4 owns the canonical tree).
    // Kept for backward compat on `analysis.opportunity_analysis` field.
    let opp_dim = alignment.dimensions.get(9).map(|d| d.score).unwrap_or(50.0);
    let opportunity = if trend_dim >= 75.0
        && (matches!(
            bias,
            MarketBias::Bullish
                | MarketBias::StrongBullish
                | MarketBias::Bearish
                | MarketBias::StrongBearish
        )) {
        OpportunityType::TrendContinuation
    } else if vol_dim >= 70.0 && struct_dim >= 60.0 {
        OpportunityType::Breakout
    } else if trend_dim >= 60.0 && momentum_assessment == MomentumAssessment::Weakening {
        OpportunityType::Pullback
    } else if vol_dim <= 30.0 {
        OpportunityType::MeanReversion
    } else if opp_dim < 30.0 {
        OpportunityType::NoClearOpportunity
    } else if opp_dim >= 90.0 && vol_dim >= 60.0 {
        OpportunityType::LiquiditySqueeze
    } else {
        OpportunityType::TrendContinuation
    };

    // Market quality aggregate
    let quality_score = (trend_dim + mom_dim + struct_dim + volu_dim) / 4.0;
    let market_quality = if quality_score >= 80.0 {
        QualityLevel::Excellent
    } else if quality_score >= 65.0 {
        QualityLevel::Good
    } else if quality_score >= 50.0 {
        QualityLevel::Average
    } else if quality_score >= 35.0 {
        QualityLevel::Weak
    } else {
        QualityLevel::Poor
    };

    let mut rationale_parts: Vec<String> = Vec::new();
    rationale_parts.push(format!(
        "MTF overall score {:.0}/100 → {}. {} of {} timeframes agree ({:.0}%). BBWP={:.0} ADX={:.0}.",
        score,
        bias,
        if alignment.trend_agreement_pct >= 50.0 {
            "Majority"
        } else {
            "Minority"
        },
        alignment.timeframes_present,
        alignment.trend_agreement_pct,
        bbwp_val,
        adx_val,
    ));
    rationale_parts.push(format!("Regime: {}", regime));
    if alignment.signal_cross_tf_count > 0 {
        rationale_parts.push(format!(
            "{} signals across multiple timeframes.",
            alignment.signal_cross_tf_count
        ));
    }

    let mut supporting: Vec<String> = Vec::new();
    let mut contradicting: Vec<String> = Vec::new();
    for tf in &alignment.timeframe_alignments {
        let dir = if tf.overall_score > 0 {
            "bullish"
        } else if tf.overall_score < 0 {
            "bearish"
        } else {
            "neutral"
        };
        let label = format!(
            "{} ({}): score {:+}, {} regime, {} signals",
            tf.timeframe, dir, tf.overall_score, tf.regime, tf.active_signals
        );
        if (bias == MarketBias::Bullish || bias == MarketBias::StrongBullish)
            && tf.overall_score > 0
        {
            supporting.push(label);
        } else if (bias == MarketBias::Bearish || bias == MarketBias::StrongBearish)
            && tf.overall_score < 0
        {
            supporting.push(label);
        } else if bias == MarketBias::Neutral && tf.overall_score.abs() < 10 {
            supporting.push(label);
        } else {
            contradicting.push(label);
        }
    }

    let interpretation = format!(
        "{} market with {} trend, {} momentum, {} structure, {} volatility, and {} volume participation. {}",
        match regime {
            MarketRegime::TrendingBull => "Bullish trending",
            MarketRegime::TrendingBear => "Bearish trending",
            MarketRegime::Range => "Ranging",
            MarketRegime::Accumulation => "Accumulating",
            MarketRegime::Distribution => "Distributing",
            MarketRegime::Expansion => "Expanding",
            MarketRegime::Contraction => "Contracting",
            MarketRegime::Transition => "Transitional",
        },
        format!("{:?}", trend_assessment).to_lowercase(),
        format!("{:?}", momentum_assessment).to_lowercase(),
        format!("{:?}", structure_assessment).to_lowercase(),
        format!("{:?}", volatility_assessment).to_lowercase(),
        format!("{:?}", volume_assessment).to_lowercase(),
        match opportunity {
            OpportunityType::TrendContinuation => "Favors trend continuation.",
            OpportunityType::Breakout => "Breakout conditions present.",
            OpportunityType::Pullback => "Pullback opportunity forming.",
            OpportunityType::MeanReversion => "Mean reversion conditions detected.",
            OpportunityType::Reversal => "Reversal signals emerging.",
            OpportunityType::LiquiditySqueeze => "Liquidity squeeze setup (Phase 3).",
            OpportunityType::Scalp => "High-frequency scalp setup active.",
            OpportunityType::NoClearOpportunity => "No clear opportunity identified.",
        }
    );

    AnalysisMatrix {
        symbol: alignment.symbol.clone(),
        bias,
        market_bias_score: alignment.mtf_overall_score / 100.0,
        state_confidence,
        confidence: state_confidence,
        market_regime: regime,
        trend_assessment,
        momentum_assessment,
        structure_assessment,
        volatility_assessment,
        volume_assessment,
        opportunity_analysis: opportunity,
        market_quality,
        market_quality_score: quality_score,
        market_phase,
        market_interpretation: interpretation,
        rationale: rationale_parts.join(" "),
        supporting_signals: supporting,
        contradicting_signals: contradicting,
        timeframes_considered: alignment.timeframes_present,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alignment::{AlignmentMatrix, TfAlignmentInfo};

    fn simple_alignment(tfs: u8, score: f64, agreement: f64, cross_tf: u32) -> AlignmentMatrix {
        let mut alignments = Vec::new();
        let labels = ["micro60", "fast180", "slow300", "macro900"];
        let secs = [60, 180, 300, 900];
        for i in 0..tfs as usize {
            alignments.push(TfAlignmentInfo {
                timeframe: labels[i].to_string(),
                timeframe_secs: secs[i],
                trend_score: score / 100.0,
                momentum_score: score / 120.0,
                overall_score: score as i32,
                regime: if score.abs() > 30.0 {
                    "TRENDING".into()
                } else {
                    "RANGE".into()
                },
                active_signals: if cross_tf > 0 { 2 } else { 0 },
                price: 64000.0,
            });
        }
        AlignmentMatrix {
            symbol: "BTC-USD".into(),
            timeframes_present: tfs,
            dimensions: vec![AlignmentMatrix::empty("").dimensions[0].clone(); 10],
            mtf_trend_alignment: score / 100.0,
            mtf_momentum_alignment: score / 120.0,
            mtf_volume_alignment: 0.0,
            mtf_volatility_alignment: 0.0,
            mtf_overall_score: score,
            mtf_overall_label: if score > 20.0 {
                "WEAK_BULL_MTF".into()
            } else {
                "NEUTRAL_MTF".into()
            },
            timeframe_alignments: alignments,
            signal_cross_tf_count: cross_tf,
            trend_agreement_pct: agreement,
        }
    }

    #[test]
    fn strong_bullish_mtf_produces_bullish() {
        let c = simple_alignment(4, 75.0, 100.0, 4);
        let d = derive_analysis(&c, Some(60.0), Some(28.0), None, None, None);
        assert!(matches!(
            d.bias,
            MarketBias::Bullish | MarketBias::StrongBullish
        ));
        assert!(d.state_confidence > 0.7);
    }

    #[test]
    fn neutral_score_neutral() {
        let c = simple_alignment(4, 10.0, 40.0, 0);
        let d = derive_analysis(&c, Some(50.0), Some(20.0), None, None, None);
        assert_eq!(d.bias, MarketBias::Neutral);
    }

    #[test]
    fn empty_returns_empty() {
        let c = AlignmentMatrix::empty("BTC-USD");
        let d = derive_analysis(&c, None, None, None, None, None);
        assert_eq!(d.bias, MarketBias::Neutral);
        assert_eq!(d.timeframes_considered, 0);
    }

    #[test]
    fn expansion_regime_from_high_bbwp() {
        let c = simple_alignment(4, 50.0, 60.0, 2);
        let d = derive_analysis(&c, Some(90.0), Some(22.0), None, None, None);
        assert_eq!(d.market_regime, MarketRegime::Expansion);
    }

    #[test]
    fn contraction_regime_from_low_bbwp() {
        let c = simple_alignment(4, 0.0, 50.0, 1);
        let d = derive_analysis(&c, Some(5.0), Some(20.0), None, None, None);
        assert_eq!(d.market_regime, MarketRegime::Contraction);
    }

    #[test]
    fn trending_bull_from_adx_and_score() {
        let c = simple_alignment(4, 55.0, 70.0, 3);
        let d = derive_analysis(&c, Some(40.0), Some(30.0), None, None, None);
        assert_eq!(d.market_regime, MarketRegime::TrendingBull);
    }

    #[test]
    fn trending_bear_from_adx_and_negative_score() {
        let c = simple_alignment(4, -55.0, 70.0, 3);
        let d = derive_analysis(&c, Some(40.0), Some(30.0), None, None, None);
        assert_eq!(d.market_regime, MarketRegime::TrendingBear);
    }

    #[test]
    fn accumulation_from_rising_score() {
        let c = simple_alignment(4, 15.0, 55.0, 2);
        let d = derive_analysis(&c, Some(50.0), Some(20.0), Some(5.0), None, None);
        assert_eq!(d.market_regime, MarketRegime::Accumulation);
    }

    #[test]
    fn distribution_from_falling_score() {
        let c = simple_alignment(4, -15.0, 55.0, 2);
        let d = derive_analysis(&c, Some(50.0), Some(20.0), Some(-5.0), None, None);
        assert_eq!(d.market_regime, MarketRegime::Distribution);
    }

    #[test]
    fn transition_from_regime_shift_with_low_adx() {
        let c = simple_alignment(4, 5.0, 45.0, 1);
        let d = derive_analysis(
            &c,
            Some(50.0),
            Some(20.0),
            Some(5.0),
            Some(MarketRegime::TrendingBull),
            None,
        );
        assert_eq!(d.market_regime, MarketRegime::Transition);
    }

    #[test]
    fn range_fallback_when_nothing_matches() {
        let c = simple_alignment(4, 5.0, 55.0, 2);
        let d = derive_analysis(&c, Some(50.0), Some(30.0), Some(5.0), None, None);
        assert_eq!(d.market_regime, MarketRegime::Range);
    }

    #[test]
    fn direction_family_table_is_total() {
        // Every OpportunityType variant must map to exactly one
        // DirectionFamily variant. This guards against accidentally
        // adding a new setup type without registering its family.
        let all = [
            OpportunityType::LiquiditySqueeze,
            OpportunityType::Scalp,
            OpportunityType::TrendContinuation,
            OpportunityType::Breakout,
            OpportunityType::Reversal,
            OpportunityType::Pullback,
            OpportunityType::MeanReversion,
            OpportunityType::NoClearOpportunity,
        ];
        let mut families: Vec<DirectionFamily> = all
            .iter()
            .map(|ot| direction_family_for(*ot))
            .collect();
        families.sort_by_key(|f| *f as u8);
        families.dedup();
        assert_eq!(
            families.len(),
            3,
            "expected exactly TrendRiding, CounterTrend, Neutral families (got {:?})",
            families
        );
        // TrendRiding is the majority family.
        assert!(all
            .iter()
            .filter(|ot| matches!(direction_family_for(**ot), DirectionFamily::TrendRiding))
            .count()
            >= 4);
        // CounterTrend covers MeanReversion + Reversal.
        assert!(all
            .iter()
            .filter(|ot| matches!(direction_family_for(**ot), DirectionFamily::CounterTrend))
            .count()
            == 2);
        // Neutral covers NoClearOpportunity.
        assert!(all
            .iter()
            .filter(|ot| matches!(direction_family_for(**ot), DirectionFamily::Neutral))
            .count()
            == 1);
    }

    #[test]
    fn direction_family_serializes_as_screaming_snake_case() {
        // Wire format must be SCREAMING_SNAKE_CASE per the matrix spec.
        let json = serde_json::to_string(&DirectionFamily::TrendRiding).unwrap();
        assert_eq!(json, "\"TREND_RIDING\"");
        let json = serde_json::to_string(&DirectionFamily::CounterTrend).unwrap();
        assert_eq!(json, "\"COUNTER_TREND\"");
        let json = serde_json::to_string(&DirectionFamily::Neutral).unwrap();
        assert_eq!(json, "\"NEUTRAL\"");
    }
}
