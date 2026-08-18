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
    /// Profile qualifies, geometry passes, AND the bracket's R:R ≥ 1.0
    /// (v6.10.18 I-5 — a sub-1 R:R bracket is never "act on this").
    /// Operator can act on this profile.
    Actionable,
    /// v6.10.18 (I-5): profile qualifies with valid geometry but the
    /// bracket's R:R < 1.0 — a real bracket, yet not worth acting on
    /// (a sub-1 R:R needs a >50% win rate just to break even). The
    /// frontend renders QUALIFYING; the readiness gate still applies.
    Qualifying,
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
    /// 0.0 means the side is not configured (no entry/target/invalidation
    /// triangle was produced). Negative values are NOT allowed — the
    /// producer emits 0.0 on geometric inversion rather than a signed
    /// negative that the UI would misread as "trade against me".
    #[serde(default)]
    pub long_expected_rr_internal: f64,
    #[serde(default)]
    pub short_expected_rr_internal: f64,
    /// Trade viability classification for this profile. The L4 producer
    /// sets this so the UI can color-code the card. `None` on legacy
    /// payloads — UI should treat `None` as `NoClear`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trade_viability: Option<TradeViability>,
    /// Server-side geometry-consistency flag for the LONG side. `true` when
    /// `long_invalidation_level < long_entry_zone.low` AND
    /// `long_target_zone.low > long_entry_zone.high` (L4 invariant).
    /// Computed from the same per-side R:R check the `trade_viability`
    /// badge uses. Defaults to `false` (no zones).
    #[serde(default)]
    pub long_geometry_consistent: bool,
    /// Server-side geometry-consistency flag for the SHORT side.
    #[serde(default)]
    pub short_geometry_consistent: bool,
    /// Internal-only scoring factors (raw blend, precondition ratio).
    /// NEVER serialised on the wire — operators don't see these. Kept in
    /// the Rust struct for telemetry consumers that read profiles
    /// directly from `OpportunityMatrix`. UI panels read `notes` for the
    /// user-facing rationale.
    #[serde(skip)]
    pub scoring_factors: Option<ScoringFactors>,
    /// v6.14: precondition-scaled operator-facing score —
    /// `round(score × min(1, preconditions_met / preconditions_total))`,
    /// computed by the L4 producer so the displayed setup score has a
    /// single source of truth. The raw `score` field above is untouched
    /// (data-science logging keeps the true viability blend). `None` on
    /// legacy payloads — the UI falls back to its local scaling rule.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_score: Option<f64>,
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
    /// v6.12 numeric companions: the exact 0-100 alignment dimension
    /// scores each qualitative assessment is bucketed from (the
    /// disaggregated siblings of `market_quality_score`). L3-owned,
    /// derived from L2 during `derive_analysis`; `Some` whenever at least
    /// one timeframe is present, `None` on the empty sentinel so the wire
    /// omits them (§6 convention). The label can never disagree with its
    /// score — the label IS the band the score falls into (see
    /// docs/matrices/02-02-analysis-matrix.md §4.2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trend_score: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub momentum_score: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub structure_score: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volatility_score: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volume_score: Option<f64>,
    /// v6.10.21 traceability: the L3 regime-input values (representative
    /// first-TF-wins `bbwp` / `adx` raw values) that the `rationale`
    /// quotes — carried on the matrix so UI exports can trace the regime
    /// derivation without re-deriving the representative map. The
    /// pair-level matrix mirror is per-slot last-writer-wins, so the
    /// exporting slot's own indicator map can differ from the matrix's
    /// provenance; these fields pin the exact inputs used.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub representative_bbwp: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub representative_adx: Option<f64>,
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
            trend_score: None,
            momentum_score: None,
            structure_score: None,
            volatility_score: None,
            volume_score: None,
            representative_bbwp: None,
            representative_adx: None,
            market_phase: MarketPhase::Unknown,
            market_interpretation: "No data available — no candles have been completed.".into(),
            rationale: String::new(),
            supporting_signals: Vec::new(),
            contradicting_signals: Vec::new(),
            timeframes_considered: 0,
        }
    }
}

/// v6.10.16 grace-band constants (L3 bias): a composite inside
/// `(BIAS_GRACE_BAND_MIN, BIAS_GRACE_BAND_MAX]` is rescued to a
/// directional bias only when the per-timeframe vote is coherent.
/// See `derive_analysis` for the full gate.
pub const BIAS_GRACE_BAND_MIN: f64 = 15.0;
pub const BIAS_GRACE_BAND_MAX: f64 = 20.0;
/// Minimum decisive TFs required on the dominant side (4:0 or 3:1).
/// Scaled to ≥3/4 of `timeframes_present` (never below 3) — a 2-TF
/// warmup window can never grace.
pub const BIAS_GRACE_VOTE_MIN: usize = 3;
/// A per-TF `overall_score` with `|score| <= BIAS_GRACE_FLAT_TF` does not
/// count as a directional vote (it is effectively flat).
pub const BIAS_GRACE_FLAT_TF: i32 = 10;
/// Intra-pair multi-TF agreement required on the grace path.
pub const BIAS_GRACE_AGREEMENT_MIN: f64 = 75.0;
/// Cross-TF signal breadth required on the grace path.
pub const BIAS_GRACE_SIGNALS_MIN: u32 = 3;
/// Confidence haircut applied to a graced read (the raw math did not
/// confirm the direction).
pub const BIAS_GRACE_CONFIDENCE_FACTOR: f64 = 0.9;
/// Hysteresis (FIX-H1): once graced, the bias HOLDS while `|score|` stays
/// above this lower band and the vote does not collapse — preventing
/// Bullish↔Neutral flip-flop on sub-point composite moves.
pub const BIAS_GRACE_HOLD_BAND_MIN: f64 = 12.0;
/// Hysteresis: minimum decisive votes to sustain a held grace (2:1+).
pub const BIAS_GRACE_HOLD_VOTE_MIN: usize = 2;
/// A per-TF window in `COMPRESSION` does not cast a grace vote — its
/// positive score is mean-reversion bait, not directional confirmation.
const BIAS_GRACE_SKIP_REGIME: &str = "COMPRESSION";
/// v6.10.17 LEAN tier (L3 bias): a composite inside the
/// `[-BIAS_LEAN_COMPOSITE_TOLERANCE, +BIAS_LEAN_COMPOSITE_TOLERANCE]`
/// window below the grace band is still rescued to a directional bias
/// when the per-timeframe vote is decisive (≥3:1) and agreement/signal
/// gates hold — so a MINIMAL bearish/bullish confirmation (e.g. composite
/// 2.6 with a 3:1 bearish vote) yields a LEAN directional read instead of
/// a flat NEUTRAL + 96% HOLD. The composite may oppose the vote only
/// within this tolerance; a stronger disagreement vetoes the lean.
pub const BIAS_LEAN_COMPOSITE_TOLERANCE: f64 = 10.0;
/// Confidence haircut applied to a LEAN-tier read (heavier than grace,
/// ×0.8) because the composite sits entirely below the grace band — only
/// the vote coherence carries the direction.
pub const BIAS_LEAN_CONFIDENCE_FACTOR: f64 = 0.8;

/// v6.10.17: `true` when a Bullish/Bearish bias was LIFTED by the margin
/// machinery (grace band, hysteresis hold, or LEAN tier) rather than
/// produced by a plain threshold — a directional bias with
/// `|market_bias_score| <= BIAS_GRACE_BAND_MAX` (in 0–100 composite
/// units) can only have come from those paths (plain Bullish/Bearish
/// requires `score > 20` strictly, Strong `> 40`). Downstream consumers
/// (DecisionContext probabilities, Advisory guidance, UI qualifier
/// labels) use this to treat a lifted read as directional even when the
/// risk gate would otherwise silence it.
///
/// **v6.10.18 (P0 unit fix):** `AnalysisMatrix.market_bias_score` is the
/// WIRE FRACTION `mtf_overall_score / 100` (range `[-1, 1]`, docs 02-02
/// §2.1) — the comparison therefore multiplies by 100 to land on the
/// 0–100 composite scale the band constants live on. Before this fix
/// every directional bias (e.g. plain Bullish at composite 21.77 →
/// fraction 0.2177) satisfied `|0.2177| <= 20` and was wrongly treated
/// as margin-lifted, silently disabling the §3.1 risk gate.
pub fn bias_lifted(bias: MarketBias, market_bias_score: f64) -> bool {
    matches!(bias, MarketBias::Bullish | MarketBias::Bearish)
        && (market_bias_score.abs() * 100.0) <= BIAS_GRACE_BAND_MAX
}

/// Per-timeframe directional vote: counts `timeframe_alignments` with a
/// decisive `overall_score` on each side (flat TFs and COMPRESSION
/// windows excluded). Returns the dominant side when it holds
/// `minimum_votes` decisive TFs and the opponent holds at most
/// `max_opponent` — a 4:0 or 3:1 window at the fire threshold.
fn vote_lean_with(
    alignment: &AlignmentMatrix,
    minimum_votes: usize,
    max_opponent: usize,
) -> Option<MarketBias> {
    let mut bull = 0usize;
    let mut bear = 0usize;
    for tf in &alignment.timeframe_alignments {
        if tf.regime.to_uppercase() == BIAS_GRACE_SKIP_REGIME {
            continue;
        }
        if tf.overall_score > BIAS_GRACE_FLAT_TF {
            bull += 1;
        } else if tf.overall_score < -BIAS_GRACE_FLAT_TF {
            bear += 1;
        }
    }
    if bull >= minimum_votes && bear <= max_opponent {
        Some(MarketBias::Bullish)
    } else if bear >= minimum_votes && bull <= max_opponent {
        Some(MarketBias::Bearish)
    } else {
        None
    }
}

/// Fire-side vote: ≥3/4 of `timeframes_present` (min 3) decisive, ≤1
/// opponent — requires real breadth, never a warmup window.
fn directional_vote_lean(alignment: &AlignmentMatrix) -> Option<MarketBias> {
    let required =
        ((alignment.timeframes_present as f64 * 0.75).ceil() as usize).max(BIAS_GRACE_VOTE_MIN);
    vote_lean_with(alignment, required, 1)
}

/// Hold-side vote (hysteresis): a looser 2:1+ requirement so a graced
/// bias survives minor vote erosion without surviving a collapse.
fn directional_vote_hold(alignment: &AlignmentMatrix) -> Option<MarketBias> {
    vote_lean_with(alignment, BIAS_GRACE_HOLD_VOTE_MIN, 1)
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
/// - `previous_bias`: the prior bar's categorical bias — used by the grace-band
///   hysteresis (FIX-H1): a graced direction HOLDS while the score stays above
///   `BIAS_GRACE_HOLD_BAND_MIN` and the vote does not collapse, so a 19.5→13.8
///   composite move does not flip the bias Bullish→Neutral mid-consensus.
pub fn derive_analysis(
    alignment: &AlignmentMatrix,
    bbwp: Option<f64>,
    adx: Option<f64>,
    previous_score: Option<f64>,
    previous_regime: Option<MarketRegime>,
    previous_volume_dim: Option<f64>,
    previous_bias: Option<MarketBias>,
) -> AnalysisMatrix {
    if alignment.timeframes_present == 0 {
        return AnalysisMatrix::empty(&alignment.symbol);
    }

    let score = alignment.mtf_overall_score;
    let mut graced = false;
    let mut leaned = false;
    // FIX-H1 (hysteresis): a previously-graced direction holds while the
    // score stays inside the hold band and the vote survives (2:1+). The
    // `previous_score` guard separates grace-held states from plain
    // threshold states (a plain Bullish has prev_score > 20).
    let held = previous_bias
        .filter(|pb| matches!(pb, MarketBias::Bullish | MarketBias::Bearish))
        .filter(|_| {
            previous_score.is_some_and(|ps| {
                (ps > BIAS_GRACE_HOLD_BAND_MIN && ps <= BIAS_GRACE_BAND_MAX)
                    || (ps < -BIAS_GRACE_HOLD_BAND_MIN && ps >= -BIAS_GRACE_BAND_MAX)
            })
        })
        .and_then(|pb| {
            let in_band = (score > BIAS_GRACE_HOLD_BAND_MIN && score <= BIAS_GRACE_BAND_MAX)
                || (score < -BIAS_GRACE_HOLD_BAND_MIN && score >= -BIAS_GRACE_BAND_MAX);
            if !in_band || alignment.trend_agreement_pct < BIAS_GRACE_AGREEMENT_MIN {
                return None;
            }
            match pb {
                MarketBias::Bullish
                    if matches!(directional_vote_hold(alignment), Some(MarketBias::Bullish)) =>
                {
                    Some(MarketBias::Bullish)
                }
                MarketBias::Bearish
                    if matches!(directional_vote_hold(alignment), Some(MarketBias::Bearish)) =>
                {
                    Some(MarketBias::Bearish)
                }
                _ => None,
            }
        });

    let held_active = held.is_some();

    let bias = if score > 40.0 {
        MarketBias::StrongBullish
    } else if score > 20.0 {
        MarketBias::Bullish
    } else if score < -40.0 {
        MarketBias::StrongBearish
    } else if score < -20.0 {
        MarketBias::Bearish
    } else if let Some(held_bias) = held {
        // Hysteresis hold (FIX-H1) — the grace state persists; the
        // haircut keeps applying because the read is still grace-sustained.
        graced = true;
        held_bias
    } else if score > BIAS_GRACE_BAND_MIN
        && score <= BIAS_GRACE_BAND_MAX
        && alignment.trend_agreement_pct >= BIAS_GRACE_AGREEMENT_MIN
        && alignment.signal_cross_tf_count >= BIAS_GRACE_SIGNALS_MIN
        && matches!(directional_vote_lean(alignment), Some(MarketBias::Bullish))
    {
        // v6.10.16 grace band: a composite just below the ±20 line is
        // rescued ONLY when the per-timeframe vote is directionally
        // coherent (≥3/4 TFs decisive on the same side, high intra-pair
        // agreement, cross-TF signal breadth; COMPRESSION windows do not
        // vote). The cap is Bullish/Bearish — grace never fabricates
        // Strong conviction — and the confidence is haircut (×0.9)
        // because the raw math did not confirm the read.
        graced = true;
        MarketBias::Bullish
    } else if score < -BIAS_GRACE_BAND_MIN
        && score >= -BIAS_GRACE_BAND_MAX
        && alignment.trend_agreement_pct >= BIAS_GRACE_AGREEMENT_MIN
        && alignment.signal_cross_tf_count >= BIAS_GRACE_SIGNALS_MIN
        && matches!(directional_vote_lean(alignment), Some(MarketBias::Bearish))
    {
        graced = true;
        MarketBias::Bearish
    } else if score >= -BIAS_LEAN_COMPOSITE_TOLERANCE
        && score <= BIAS_GRACE_BAND_MIN
        && alignment.trend_agreement_pct >= BIAS_GRACE_AGREEMENT_MIN
        && alignment.signal_cross_tf_count >= BIAS_GRACE_SIGNALS_MIN
        && matches!(directional_vote_lean(alignment), Some(MarketBias::Bullish))
    {
        // v6.10.17 LEAN tier: the composite sits below the grace band
        // (|score| ≤ 15) yet the per-timeframe vote is decisively bullish
        // (≥3:1, agreement ≥ 75, cross-TF signal breadth; COMPRESSION
        // windows do not vote) and does not oppose the lean by more than
        // the tolerance. A minimal bullish confirmation therefore yields a
        // directional read — capped at Bullish (never Strong) with the
        // heavier ×0.8 confidence haircut.
        leaned = true;
        MarketBias::Bullish
    } else if score <= BIAS_LEAN_COMPOSITE_TOLERANCE
        && score >= -BIAS_GRACE_BAND_MIN
        && alignment.trend_agreement_pct >= BIAS_GRACE_AGREEMENT_MIN
        && alignment.signal_cross_tf_count >= BIAS_GRACE_SIGNALS_MIN
        && matches!(directional_vote_lean(alignment), Some(MarketBias::Bearish))
    {
        // v6.10.17 LEAN tier — bearish mirror (composite may oppose the
        // bearish vote by at most +BIAS_LEAN_COMPOSITE_TOLERANCE).
        leaned = true;
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
    if graced {
        state_confidence *= BIAS_GRACE_CONFIDENCE_FACTOR;
    }
    if leaned {
        state_confidence *= BIAS_LEAN_CONFIDENCE_FACTOR;
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

    // Trend assessment from `mtf_trend_alignment` (signed mean in
    // `[-1, +1]`). Bug-fix #20: the legacy code read
    // `alignment.dimensions[0].score`, which is the 0-100 mapped
    // version produced by `AlignmentDimension::from_signed` and is
    // not directly comparable to the `mtf_trend_alignment` raw
    // signed mean that downstream consumers (DecisionContext,
    // confluence-score sign) use. We now read the raw
    // `mtf_trend_alignment` field directly and scale to 0-100 via
    // `((mtf_trend_alignment + 1) / 2) * 100` so the rest of the L3
    // derivation operates on the canonical 0-100 scale.
    let trend_dim = ((alignment.mtf_trend_alignment + 1.0) / 2.0 * 100.0)
        .max(0.0)
        .min(100.0);
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
    //
    // Bug-fix #17: the legacy `is_low_volatility` band was
    // `vol_dim >= 20.0 && vol_dim < 40.0` (the "Compressed" band only).
    // A volatility score of 5-19 falls into the "Unstable" band and
    // was silently excluded from the Wyckoff low-volatility path,
    // which is the exact opposite of intent — the canonical Wyckoff
    // "Accumulation" / "Distribution" setups prefer ANY form of
    // suppressed activity (compressed OR unstable OR low-vol). We now
    // use the full low-vol band (`vol_dim < 40.0`) which subsumes
    // both Compressed and Unstable. The "Unstable" name is a label
    // artifact; the L2 score is monotonically "less volatility than
    // baseline" in that band, so it's a valid low-volatility
    // detection input.
    let is_low_volatility = vol_dim < 40.0;
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

    // Opportunity (deprecated — L4 owns the canonical tree; kept for
    // backward compat on `analysis.opportunity_analysis`). v6.10.8: the
    // chain is synced with the L4 §4 tree (02-08-opportunity-matrix.md)
    // so the Analysis interpretation prose and the Metrics export label
    // can never contradict the L4 verdict:
    //   - TrendContinuation requires a directional bias AND non-reversing
    //     momentum (the legacy default fell through to TrendContinuation
    //     for EVERY market, including collapsed/neutral ones).
    //   - MeanReversion requires the range regime (`is_range`) — the same
    //     B2 gate the L4 tree enforces.
    //   - LiquiditySqueeze is NOT derivable here (it needs L1.5 cascade
    //     data); the legacy `opp_dim` heuristic is dropped.
    //   - The default is NoClearOpportunity.
    // Reversal (confirmed divergence) is likewise not derivable here —
    // it needs the indicator signal map.
    let momentum_not_exhausted = !matches!(momentum_assessment, MomentumAssessment::Reversing);
    let is_range = matches!(regime, MarketRegime::Range | MarketRegime::Contraction);
    let opportunity = if trend_dim >= 75.0
        && (matches!(
            bias,
            MarketBias::Bullish
                | MarketBias::StrongBullish
                | MarketBias::Bearish
                | MarketBias::StrongBearish
        ))
        && momentum_not_exhausted
    {
        OpportunityType::TrendContinuation
    } else if vol_dim >= 70.0 && struct_dim >= 60.0 {
        OpportunityType::Breakout
    } else if trend_dim >= 60.0 && momentum_assessment == MomentumAssessment::Weakening {
        OpportunityType::Pullback
    } else if vol_dim <= 30.0 && is_range {
        OpportunityType::MeanReversion
    } else {
        OpportunityType::NoClearOpportunity
    };

    // Market quality aggregate. Bug-fix #12: the legacy aggregate
    // averaged 4 dimensions (trend + momentum + structure + volume) and
    // omitted `vol_dim` (volatility). For a high-TF BTC symbol with
    // strong trend + momentum + structure + volume but compressed
    // volatility (the typical "quiet before expansion" pattern), the
    // quality score was inflated to 75+ (Good) even though the
    // volatility dimension was clearly 25-35. (SUPERSEDED by the F3
    // decision below — the current implementation again excludes
    // volatility per the canonical spec.)
    // v6.10 (Phase 6 / F3): `market_quality` is the mean of the trend,
    // momentum, structure, and volume dimension scores (4 dims, NOT 5 —
    // volatility is excluded per the canonical spec at
    // `docs/matrices/02-02-analysis-matrix.md §3.6`). The previous v6.9
    // implementation included volatility, which inflated the score for
    // "compression" regimes. Bands are the canonical half-open
    // thresholds: POOR <30, WEAK [30,50), AVERAGE [50,70), GOOD [70,85),
    // EXCELLENT ≥85.
    let quality_score = (trend_dim + mom_dim + struct_dim + volu_dim) / 4.0;
    let market_quality = if quality_score >= 85.0 {
        QualityLevel::Excellent
    } else if quality_score >= 70.0 {
        QualityLevel::Good
    } else if quality_score >= 50.0 {
        QualityLevel::Average
    } else if quality_score >= 30.0 {
        QualityLevel::Weak
    } else {
        QualityLevel::Poor
    };

    let mut rationale_parts: Vec<String> = Vec::new();
    // v6.10.17 (F22): BBWP/ADX render with one decimal so the rationale
    // matches the indicator-history values (73.7, not "75"). The bias
    // qualifier phrase explains any lifted read so the rationale can
    // never claim a plain directional bias for a grace/lean/hold state.
    rationale_parts.push(format!(
        "MTF overall score {:.0}/100 → {:?}. {} of {} timeframes agree ({:.0}%). BBWP={:.1} ADX={:.1}.{}",
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
        match (held_active, graced, leaned) {
            (true, _, _) => " Bias lifted by TF-vote margin (held from a previous grace state).",
            (false, true, _) => " Bias lifted by TF-vote margin (grace band).",
            (false, false, true) => " Bias lifted by TF-vote margin (LEAN tier — minimal confirmation).",
            (false, false, false) => "",
        }
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
        // v6.12: the exact 0-100 alignment dimension scores each
        // assessment is bucketed from — the numeric companions rendered
        // as badges on the Analysis panel (the disaggregated siblings of
        // `market_quality_score`). Present whenever timeframes_present >= 1.
        trend_score: Some(trend_dim),
        momentum_score: Some(mom_dim),
        structure_score: Some(struct_dim),
        volatility_score: Some(vol_dim),
        volume_score: Some(volu_dim),
        // v6.10.21: pin the exact L3 regime inputs the rationale quotes so
        // export surfaces can trace the derivation without re-deriving the
        // representative map (which can differ from this matrix's slot).
        representative_bbwp: bbwp,
        representative_adx: adx,
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
            blend_weights: vec![
                ("Trend".into(), 0.5),
                ("Momentum".into(), 0.3),
                ("Volume".into(), 0.1),
                ("Volatility".into(), 0.1),
            ],
            timeframe_alignments: alignments,
            signal_cross_tf_count: cross_tf,
            trend_agreement_pct: agreement,
        }
    }

    /// Realistic multi-TF shape (v6.10.17): explicit per-TF `overall_score`
    /// values with a composite that may differ from their mean — exactly
    /// how the live pipeline produces e.g. −58 / −51 / −11 / +42 at a
    /// composite of 2.6 (the user's 03:40 capture).
    fn capture_alignment(
        score: f64,
        agreement: f64,
        cross_tf: u32,
        per_tf_scores: &[i32],
    ) -> AlignmentMatrix {
        let labels = ["micro60", "fast180", "slow300", "macro900"];
        let secs = [60, 180, 300, 900];
        let mut c = simple_alignment(per_tf_scores.len() as u8, score, agreement, cross_tf);
        for (i, tf) in c.timeframe_alignments.iter_mut().enumerate() {
            let s = per_tf_scores[i] as f64;
            tf.timeframe = labels[i].to_string();
            tf.timeframe_secs = secs[i];
            tf.overall_score = per_tf_scores[i];
            tf.trend_score = s / 100.0;
            tf.momentum_score = s / 120.0;
            tf.regime = if s.abs() > 30.0 {
                "TRENDING".into()
            } else {
                "RANGE".into()
            };
        }
        c
    }

    #[test]
    fn strong_bullish_mtf_produces_bullish() {
        let c = simple_alignment(4, 75.0, 100.0, 4);
        let d = derive_analysis(&c, Some(60.0), Some(28.0), None, None, None, None);
        assert!(matches!(
            d.bias,
            MarketBias::Bullish | MarketBias::StrongBullish
        ));
        assert!(d.state_confidence > 0.7);
    }

    #[test]
    fn neutral_score_neutral() {
        let c = simple_alignment(4, 10.0, 40.0, 0);
        let d = derive_analysis(&c, Some(50.0), Some(20.0), None, None, None, None);
        assert_eq!(d.bias, MarketBias::Neutral);
    }

    // ── v6.10.16 grace band (sensitivity lever) ──

    #[test]
    fn grace_band_rescues_coherent_vote_just_below_threshold() {
        // The user's live capture shape: composite 19.1, 4/4 TFs positive,
        // 100% agreement, 33 cross-TF signals → Bullish, not HOLD.
        let c = simple_alignment(4, 19.0, 100.0, 33);
        let d = derive_analysis(&c, Some(50.0), Some(20.0), None, None, None, None);
        assert_eq!(d.bias, MarketBias::Bullish);
        // Never Strong from the grace path.
        assert!(!matches!(d.bias, MarketBias::StrongBullish));
        // Confidence haircut ×0.9 applied (base 0.19 +0.15 +0.1 = 0.44 → 0.396).
        assert!((d.state_confidence - 0.396).abs() < 1e-9);
    }

    #[test]
    fn grace_band_rescues_bearish_vote() {
        let c = simple_alignment(4, -19.0, 100.0, 33);
        let d = derive_analysis(&c, Some(50.0), Some(20.0), None, None, None, None);
        assert_eq!(d.bias, MarketBias::Bearish);
    }

    #[test]
    fn grace_band_requires_three_quarter_vote() {
        // 2:2 split at 19 → no grace.
        let mut c = simple_alignment(4, 19.0, 100.0, 33);
        for (i, tf) in c.timeframe_alignments.iter_mut().enumerate() {
            if i >= 2 {
                tf.overall_score = -tf.overall_score;
            }
        }
        let d = derive_analysis(&c, Some(50.0), Some(20.0), None, None, None, None);
        assert_eq!(d.bias, MarketBias::Neutral);
    }

    #[test]
    fn grace_band_requires_agreement() {
        let c = simple_alignment(4, 19.0, 60.0, 33);
        let d = derive_analysis(&c, Some(50.0), Some(20.0), None, None, None, None);
        assert_eq!(d.bias, MarketBias::Neutral);
    }

    #[test]
    fn grace_band_requires_signal_breadth() {
        let c = simple_alignment(4, 19.0, 100.0, 1);
        let d = derive_analysis(&c, Some(50.0), Some(20.0), None, None, None, None);
        assert_eq!(d.bias, MarketBias::Neutral);
    }

    #[test]
    fn lean_tier_fires_below_grace_band_with_coherent_vote() {
        // v6.10.17: 14.9 with a perfect 4:0 vote is now a LEAN read (the
        // grace band was (15,20]; the LEAN tier rescues (0,15] with the
        // same vote/agreement/signal gates). The user's 03:40 capture
        // (composite 2.6, 3:1 bearish vote) is the canonical case.
        let c = simple_alignment(4, 14.9, 100.0, 33);
        let d = derive_analysis(&c, Some(50.0), Some(20.0), None, None, None, None);
        assert_eq!(d.bias, MarketBias::Bullish);
        assert!(bias_lifted(d.bias, d.market_bias_score));
        // Heavier ×0.8 haircut: base 0.149 + 0.15 + 0.1 = 0.399 → 0.3192.
        assert!((d.state_confidence - 0.3192).abs() < 1e-9);
    }

    #[test]
    fn lean_tier_rescues_minimal_bearish_confirmation() {
        // The 03:40-style state: composite 2.6 but the per-TF scores are
        // −58 / −51 / −11 / +42 → a 3:1 bearish vote (agreement 75,
        // 37 cross-TF signals) → LEAN Bearish, never a flat HOLD.
        let c = capture_alignment(2.6, 75.0, 37, &[-58, -51, -11, 42]);
        let d = derive_analysis(&c, Some(50.0), Some(20.0), None, None, None, None);
        assert_eq!(d.bias, MarketBias::Bearish);
        assert!(bias_lifted(d.bias, d.market_bias_score));
        // base 0.026 + 0.15 + 0.1 = 0.276 → ×0.8 = 0.2208
        assert!((d.state_confidence - 0.2208).abs() < 1e-9);
    }

    #[test]
    fn lean_tier_does_not_fire_on_balanced_vote() {
        // 2:2 vote at composite 2.6 (−58 / +51 / −11 / +42) → genuinely
        // flat → Neutral (the flat state keeps its 96% HOLD — HOLD is
        // reserved for real no-direction).
        let c = capture_alignment(2.6, 75.0, 37, &[-58, 51, -11, 42]);
        let d = derive_analysis(&c, Some(50.0), Some(20.0), None, None, None, None);
        assert_eq!(d.bias, MarketBias::Neutral);
    }

    #[test]
    fn lean_tier_is_vetoed_by_opposing_composite() {
        // 3:1 bearish vote but composite +12 (opposition beyond the ±10
        // tolerance) → the vote and the composite conflict → Neutral.
        let c = capture_alignment(12.0, 75.0, 37, &[-58, -51, -11, 42]);
        let d = derive_analysis(&c, Some(50.0), Some(20.0), None, None, None, None);
        assert_eq!(d.bias, MarketBias::Neutral);
    }

    #[test]
    fn lean_tier_requires_agreement() {
        // 3:1 bullish vote but agreement 60 < 75 → no lean → Neutral.
        let c = capture_alignment(6.0, 60.0, 33, &[58, 51, 11, -42]);
        let d = derive_analysis(&c, Some(50.0), Some(20.0), None, None, None, None);
        assert_eq!(d.bias, MarketBias::Neutral);
    }

    #[test]
    fn lean_tier_requires_signal_breadth() {
        let c = capture_alignment(6.0, 100.0, 2, &[58, 51, 11, -42]);
        let d = derive_analysis(&c, Some(50.0), Some(20.0), None, None, None, None);
        assert_eq!(d.bias, MarketBias::Neutral);
    }

    #[test]
    fn bias_lifted_uses_the_composite_scale() {
        // v6.10.18 (I-1) unit fix: `market_bias_score` is the wire
        // FRACTION (`mtf_overall_score / 100`) — the lift predicate
        // compares on the 0–100 composite scale.
        // Plain Bullish at composite 21.77 → fraction 0.2177 → NOT lifted.
        assert!(!bias_lifted(MarketBias::Bullish, 0.2177));
        assert!(!bias_lifted(MarketBias::Bearish, -0.2177));
        // Graced at 19.9 → 0.199 → lifted.
        assert!(bias_lifted(MarketBias::Bullish, 0.199));
        assert!(bias_lifted(MarketBias::Bearish, -0.199));
        // LEAN at 2.6 → 0.026 → lifted.
        assert!(bias_lifted(MarketBias::Bullish, 0.026));
        assert!(bias_lifted(MarketBias::Bearish, -0.026));
        // Strong biases are never lifted.
        assert!(!bias_lifted(MarketBias::StrongBullish, 0.5));
        assert!(!bias_lifted(MarketBias::StrongBearish, -0.5));
        // Neutral is never lifted.
        assert!(!bias_lifted(MarketBias::Neutral, 0.0));
    }

    #[test]
    fn lean_tier_fires_for_bullish_mirror() {
        // The exact mirror of the 03:40 capture (+58 / +51 / +11 / −42 at
        // composite −2.6) → LEAN Bullish — longs and shorts are generated
        // with equal possibility (sign-symmetric).
        let c = capture_alignment(-2.6, 75.0, 37, &[58, 51, 11, -42]);
        let d = derive_analysis(&c, Some(50.0), Some(20.0), None, None, None, None);
        assert_eq!(d.bias, MarketBias::Bullish);
        assert!(bias_lifted(d.bias, d.market_bias_score));
    }

    #[test]
    fn grace_band_flat_tfs_do_not_vote() {
        // Per-TF scores of 8 are below the flat threshold (|score| <= 10) —
        // 4 flat TFs at 19 → no directional vote → Neutral.
        let mut c = simple_alignment(4, 19.0, 100.0, 33);
        for tf in c.timeframe_alignments.iter_mut() {
            tf.overall_score = 8;
        }
        let d = derive_analysis(&c, Some(50.0), Some(20.0), None, None, None, None);
        assert_eq!(d.bias, MarketBias::Neutral);
    }

    #[test]
    fn grace_band_does_not_affect_scores_above_band() {
        // 25 → normal Bullish path, no haircut.
        let c = simple_alignment(4, 25.0, 100.0, 33);
        let d = derive_analysis(&c, Some(50.0), Some(20.0), None, None, None, None);
        assert_eq!(d.bias, MarketBias::Bullish);
        assert!((d.state_confidence - 0.5).abs() < 1e-9); // 0.25 + 0.15 + 0.1 = 0.5, no ×0.9
    }

    // ── FIX-H1: grace-band hysteresis ──

    #[test]
    fn grace_band_holds_bias_across_band_reentry() {
        // Frame 1: 19.0 with a 4:0 vote fires Bullish.
        let c1 = simple_alignment(4, 19.0, 100.0, 33);
        let d1 = derive_analysis(&c1, Some(50.0), Some(20.0), None, None, None, None);
        assert_eq!(d1.bias, MarketBias::Bullish);
        // Frame 2: composite drifts to 13.8 — still a 4:0 vote. Without
        // hysteresis this flipped Bullish→Neutral on a sub-point move; the
        // hold path keeps the bias while the score stays > 12.
        let c2 = simple_alignment(4, 13.8, 100.0, 33);
        let d2 = derive_analysis(
            &c2,
            Some(50.0),
            Some(20.0),
            Some(19.0),
            None,
            None,
            Some(MarketBias::Bullish),
        );
        assert_eq!(d2.bias, MarketBias::Bullish);
        // The haircut keeps applying to held states.
        assert!(d2.state_confidence < d1.state_confidence);
    }

    #[test]
    fn grace_band_exits_below_hold_band() {
        // Held Bullish at 11.0: the hysteresis hold band is (12,20] so the
        // hold path exits — but the LEAN tier (v6.10.17) still rescues the
        // read because the 4:0 vote remains coherent (|11| ≤ 15, no
        // opposition). The bias persists via the LEAN path with the
        // heavier ×0.8 haircut instead of freezing stale state.
        let c = simple_alignment(4, 11.0, 100.0, 33);
        let d = derive_analysis(
            &c,
            Some(50.0),
            Some(20.0),
            Some(19.0),
            None,
            None,
            Some(MarketBias::Bullish),
        );
        assert_eq!(d.bias, MarketBias::Bullish);
        assert!(bias_lifted(d.bias, d.market_bias_score));
        // base 0.11 + 0.15 + 0.1 = 0.36 → ×0.8 = 0.288
        assert!((d.state_confidence - 0.288).abs() < 1e-9);
    }

    #[test]
    fn grace_band_exits_on_vote_collapse() {
        // Held Bullish at 13.8, but the vote collapses to 2:2 → Neutral.
        let mut c = simple_alignment(4, 13.8, 100.0, 33);
        for (i, tf) in c.timeframe_alignments.iter_mut().enumerate() {
            if i >= 2 {
                tf.overall_score = -tf.overall_score;
            }
        }
        let d = derive_analysis(
            &c,
            Some(50.0),
            Some(20.0),
            Some(19.0),
            None,
            None,
            Some(MarketBias::Bullish),
        );
        assert_eq!(d.bias, MarketBias::Neutral);
    }

    #[test]
    fn grace_band_is_not_held_from_plain_threshold_state() {
        // Previous frame was plain Bullish (score 25, no grace) and the
        // current frame drops to 13.8 with a 4:0 vote — the hold path must
        // NOT engage (prev_score 25 is outside the grace band), so the
        // state cannot freeze a stale bias. The LEAN tier (v6.10.17)
        // legitimately re-fires on the coherent vote with the ×0.8 haircut.
        let c = simple_alignment(4, 13.8, 100.0, 33);
        let d = derive_analysis(
            &c,
            Some(50.0),
            Some(20.0),
            Some(25.0),
            None,
            None,
            Some(MarketBias::Bullish),
        );
        assert_eq!(d.bias, MarketBias::Bullish);
        assert!(bias_lifted(d.bias, d.market_bias_score));
        // base 0.138 + 0.15 + 0.1 = 0.388 → ×0.8 = 0.3104
        assert!((d.state_confidence - 0.3104).abs() < 1e-9);
    }

    #[test]
    fn grace_band_bearish_hold_is_symmetric() {
        // simple_alignment(-13.8) gives per-TF scores of -13 → a 4:0
        // bearish vote; the held Bearish bias must persist.
        let c = simple_alignment(4, -13.8, 100.0, 33);
        let d = derive_analysis(
            &c,
            Some(50.0),
            Some(20.0),
            Some(-19.0),
            None,
            None,
            Some(MarketBias::Bearish),
        );
        assert_eq!(d.bias, MarketBias::Bearish);
    }

    #[test]
    fn grace_band_compression_tfs_do_not_vote() {
        // 2 TRENDING + 2 COMPRESSION windows, composite 19: the vote is
        // only 2:0 — below the ≥3 requirement → Neutral. COMPRESSION
        // positive scores are mean-reversion bait, not confirmation.
        let mut c = simple_alignment(4, 19.0, 100.0, 33);
        for (i, tf) in c.timeframe_alignments.iter_mut().enumerate() {
            if i >= 2 {
                tf.regime = "COMPRESSION".into();
            }
        }
        let d = derive_analysis(&c, Some(50.0), Some(20.0), None, None, None, None);
        assert_eq!(d.bias, MarketBias::Neutral);

        // 3 TRENDING + 1 COMPRESSION → 3:0 vote → still fires.
        let mut c2 = simple_alignment(4, 19.0, 100.0, 33);
        c2.timeframe_alignments[3].regime = "COMPRESSION".into();
        let d2 = derive_analysis(&c2, Some(50.0), Some(20.0), None, None, None, None);
        assert_eq!(d2.bias, MarketBias::Bullish);
    }

    #[test]
    fn grace_band_vote_is_pinned_to_timeframes_present() {
        // 3 decisive TFs of 3 present → required = max(3, ceil(2.25)) = 3
        // → 3:0 fires. 2 decisive TFs of 2 present → required 3 → a 2-TF
        // warmup window can never grace.
        let c3 = simple_alignment(3, 19.0, 100.0, 33);
        let d3 = derive_analysis(&c3, Some(50.0), Some(20.0), None, None, None, None);
        assert_eq!(d3.bias, MarketBias::Bullish);

        let c2 = simple_alignment(2, 19.0, 100.0, 33);
        let d2 = derive_analysis(&c2, Some(50.0), Some(20.0), None, None, None, None);
        assert_eq!(d2.bias, MarketBias::Neutral);
    }

    #[test]
    fn empty_returns_empty() {
        let c = AlignmentMatrix::empty("BTC-USD");
        let d = derive_analysis(&c, None, None, None, None, None, None);
        assert_eq!(d.bias, MarketBias::Neutral);
        assert_eq!(d.timeframes_considered, 0);
    }

    #[test]
    fn expansion_regime_from_high_bbwp() {
        let c = simple_alignment(4, 50.0, 60.0, 2);
        let d = derive_analysis(&c, Some(90.0), Some(22.0), None, None, None, None);
        assert_eq!(d.market_regime, MarketRegime::Expansion);
    }

    #[test]
    fn contraction_regime_from_low_bbwp() {
        let c = simple_alignment(4, 0.0, 50.0, 1);
        let d = derive_analysis(&c, Some(5.0), Some(20.0), None, None, None, None);
        assert_eq!(d.market_regime, MarketRegime::Contraction);
    }

    #[test]
    fn trending_bull_from_adx_and_score() {
        let c = simple_alignment(4, 55.0, 70.0, 3);
        let d = derive_analysis(&c, Some(40.0), Some(30.0), None, None, None, None);
        assert_eq!(d.market_regime, MarketRegime::TrendingBull);
    }

    #[test]
    fn trending_bear_from_adx_and_negative_score() {
        let c = simple_alignment(4, -55.0, 70.0, 3);
        let d = derive_analysis(&c, Some(40.0), Some(30.0), None, None, None, None);
        assert_eq!(d.market_regime, MarketRegime::TrendingBear);
    }

    #[test]
    fn accumulation_from_rising_score() {
        let c = simple_alignment(4, 15.0, 55.0, 2);
        let d = derive_analysis(&c, Some(50.0), Some(20.0), Some(5.0), None, None, None);
        assert_eq!(d.market_regime, MarketRegime::Accumulation);
    }

    #[test]
    fn distribution_from_falling_score() {
        let c = simple_alignment(4, -15.0, 55.0, 2);
        let d = derive_analysis(&c, Some(50.0), Some(20.0), Some(-5.0), None, None, None);
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
            None,
        );
        assert_eq!(d.market_regime, MarketRegime::Transition);
    }

    #[test]
    fn range_fallback_when_nothing_matches() {
        let c = simple_alignment(4, 5.0, 55.0, 2);
        let d = derive_analysis(&c, Some(50.0), Some(30.0), Some(5.0), None, None, None);
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
        let mut families: Vec<DirectionFamily> =
            all.iter().map(|ot| direction_family_for(*ot)).collect();
        families.sort_by_key(|f| *f as u8);
        families.dedup();
        assert_eq!(
            families.len(),
            3,
            "expected exactly TrendRiding, CounterTrend, Neutral families (got {:?})",
            families
        );
        // TrendRiding is the majority family.
        assert!(
            all.iter()
                .filter(|ot| matches!(direction_family_for(**ot), DirectionFamily::TrendRiding))
                .count()
                >= 4
        );
        // CounterTrend covers MeanReversion + Reversal.
        assert!(
            all.iter()
                .filter(|ot| matches!(direction_family_for(**ot), DirectionFamily::CounterTrend))
                .count()
                == 2
        );
        // Neutral covers NoClearOpportunity.
        assert!(
            all.iter()
                .filter(|ot| matches!(direction_family_for(**ot), DirectionFamily::Neutral))
                .count()
                == 1
        );
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

    // ── AN-4: the deprecated L3 opportunity chain mirrors the fixed L4 tree ──

    fn custom_alignment(
        trend_signed: f64,
        mom_dim: f64,
        vol_dim: f64,
        struct_dim: f64,
        score: f64,
    ) -> AlignmentMatrix {
        let mut a = AlignmentMatrix::empty("BTC-USD");
        a.timeframes_present = 4;
        a.mtf_trend_alignment = trend_signed;
        a.mtf_overall_score = score;
        if let Some(d) = a.dimensions.get_mut(1) {
            d.score = mom_dim;
        }
        if let Some(d) = a.dimensions.get_mut(2) {
            d.score = 50.0; // volume — neutral in these tests
        }
        if let Some(d) = a.dimensions.get_mut(3) {
            d.score = vol_dim;
        }
        if let Some(d) = a.dimensions.get_mut(4) {
            d.score = struct_dim;
        }
        a
    }

    #[test]
    fn opportunity_chain_defaults_to_no_clear() {
        // AN-4: a compressed market in an EXPANSION regime (vol ≤ 30 but
        // NOT range) must NOT classify as MeanReversion, and the legacy
        // default-TrendContinuation fallthrough is gone — the verdict
        // mirrors the fixed L4 tree (B2 parity).
        let a = custom_alignment(0.0, 50.0, 25.0, 50.0, 10.0);
        // bbwp ≥ 85 → Expansion regime.
        let d = derive_analysis(&a, Some(90.0), Some(20.0), None, None, None, None);
        assert_eq!(d.market_regime, MarketRegime::Expansion);
        assert_eq!(d.opportunity_analysis, OpportunityType::NoClearOpportunity);
    }

    #[test]
    fn opportunity_mean_reversion_requires_range_regime() {
        // The same compressed-vol profile in a RANGE regime classifies as
        // MeanReversion (is_range satisfied).
        let a = custom_alignment(0.0, 50.0, 25.0, 50.0, 10.0);
        let d = derive_analysis(&a, Some(50.0), Some(20.0), None, None, None, None);
        assert_eq!(d.market_regime, MarketRegime::Range);
        assert_eq!(d.opportunity_analysis, OpportunityType::MeanReversion);
    }

    #[test]
    fn opportunity_trend_continuation_requires_bias_and_momentum() {
        // trend ≥ 75 + directional bias + stable momentum → TrendContinuation.
        let a = custom_alignment(0.75, 70.0, 45.0, 60.0, 30.0);
        let d = derive_analysis(&a, Some(50.0), Some(28.0), None, None, None, None);
        assert_eq!(d.opportunity_analysis, OpportunityType::TrendContinuation);

        // Reversing momentum must NOT classify as TrendContinuation.
        let a2 = custom_alignment(0.75, 25.0, 45.0, 60.0, 30.0);
        let d2 = derive_analysis(&a2, Some(50.0), Some(28.0), None, None, None, None);
        assert_ne!(d2.opportunity_analysis, OpportunityType::TrendContinuation);
    }

    #[test]
    fn opportunity_pullback_requires_weakening_momentum() {
        // trend 65 (≥ 60) + Weakening momentum + vol above the 30 gate
        // → Pullback, not MeanReversion.
        let a = custom_alignment(0.3, 45.0, 45.0, 50.0, 10.0);
        let d = derive_analysis(&a, Some(50.0), Some(20.0), None, None, None, None);
        assert_eq!(d.opportunity_analysis, OpportunityType::Pullback);
    }
}
