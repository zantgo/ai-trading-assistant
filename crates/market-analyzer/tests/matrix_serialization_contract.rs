//! Wire-contract key parity (Phase 2 of the final verification).
//!
//! Every matrix the MME dashboard consumes is serialized to JSON with
//! snake_case keys — the frontend store (`ui/src/types.ts`) parses these
//! verbatim. This test locks the exact key sets so a struct refactor or a
//! new serde rename can never silently drift the wire shape the
//! dashboards and export builders depend on.
//!
//! Expected key lists are derived from `ui/src/types.ts` interfaces and
//! the `docs/ui-ux/07-05-export-data-payload-schema.md` payload schema.

use std::collections::HashMap;

use core_domain::advisory::AdvisoryMatrix;
use core_domain::alignment::AlignmentMatrix;
use core_domain::analysis::AnalysisMatrix;
use core_domain::decision_context::DecisionContext;
use core_domain::indicator_dtos::NormalizedIndicatorValue;
use core_domain::market_context::{ContextDimension, MarketContext};
use core_domain::models::{
    CandleQualityEnvelope, MarketSnapshot, SequenceIntegrity, TimeframeSlot,
};
use core_domain::normalized::Exchange;
use core_domain::risk::RiskMatrix;
use rust_decimal_macros::dec;
use serde_json::{json, Value};

fn neutral_dimension() -> ContextDimension {
    ContextDimension {
        score: 0.0,
        confidence: 0.0,
        label: "NEUTRAL".into(),
    }
}

fn build_realistic_snapshot() -> MarketSnapshot {
    let mut indicators: HashMap<String, NormalizedIndicatorValue> = HashMap::new();
    indicators.insert(
        "rsi".to_string(),
        NormalizedIndicatorValue::scalar(62.4, 0.24, "BULLISH"),
    );
    indicators.insert(
        "adx".to_string(),
        NormalizedIndicatorValue::scalar(34.99, 0.7, "STRONG_BULL_TREND"),
    );

    let dim = neutral_dimension();
    let context = MarketContext {
        trend: dim.clone(),
        momentum: dim.clone(),
        volatility: dim.clone(),
        volume: dim.clone(),
        liquidity: dim.clone(),
        regime: "TRENDING".into(),
        overall_score: 50,
        overall_label: "BULLISH".into(),
    };

    let decision = DecisionContext {
        score: 45.0,
        bias: "Bullish".into(),
        score_confidence: 0.68,
        entry_danger: core_domain::risk::RiskDimension::from_score(35.0),
        expected_reward_risk_ratio: 2.1,
        trade_readiness: "WATCH".into(),
        contributing_indicators: vec!["rsi".into(), "adx".into()],
        long_probability: 0.0,
        short_probability: 0.0,
        hold_probability: 0.0,
        net_bias_pct: 0.0,
    };

    MarketSnapshot {
        timeframe_slot: Some(TimeframeSlot::Micro),
        exchange: Some(Exchange::Hyperliquid),
        timeframe_secs: 60,
        timestamp: 1700000000,
        symbol: "BTC-USDT".into(),
        is_completed: Some(true),
        mid_price: dec!(50000.0),
        bid_price: dec!(49995.0),
        ask_price: dec!(50005.0),
        bid_size: Some(dec!(1.5)),
        ask_size: Some(dec!(2.0)),
        funding_rate: Some(dec!(0.0001)),
        open: Some(dec!(49800.0)),
        high: Some(dec!(50200.0)),
        low: Some(dec!(49750.0)),
        close: Some(dec!(50000.0)),
        volume: Some(dec!(100.0)),
        average_volume: Some(dec!(85.0)),
        indicators,
        context: Some(context),
        alignment: Some(AlignmentMatrix::empty("BTC-USDT")),
        analysis: Some(AnalysisMatrix::empty("BTC-USDT")),
        risk: Some(RiskMatrix::empty("BTC-USDT")),
        advisory: Some(AdvisoryMatrix::empty("BTC-USDT")),
        open_interest: Some(dec!(15000000.0)),
        oi_delta_1h: Some(dec!(500000.0)),
        mark_price: Some(dec!(50002.0)),
        index_price: Some(dec!(50000.0)),
        mark_index_spread_pct: Some(0.004),
        prev_day_px: Some(dec!(49500.0)),
        statistical_context: None,
        decision_context: Some(decision),
        opportunity: None,
        liquidity_signals: vec![],
        metrics_config: None,
        risk_profile: Some(1),
        liquidity: None,
        cluster: None,
        volume_profile: None,
        quality_envelope: Some(CandleQualityEnvelope {
            quality_score: 95.0,
            is_valid: true,
            is_gap_filled: false,
            had_outliers_rejected: false,
            spike_detected: false,
            is_stale: false,
            sequence_integrity: SequenceIntegrity::Valid,
            gap_since_last: 60,
            validated_at: 1700000000000,
        }),
        pipeline_state: core_domain::models::CandlePipelineState::default(),
        indicator_lifecycle: std::collections::HashMap::new(),
    }
}

fn sorted_keys(v: &Value) -> Vec<String> {
    let mut keys: Vec<String> = v.as_object().expect("object").keys().cloned().collect();
    keys.sort();
    keys
}

fn assert_keys_eq(v: &Value, expected: &[&str], what: &str) {
    let mut exp: Vec<String> = expected.iter().map(|s| s.to_string()).collect();
    exp.sort();
    assert_eq!(
        sorted_keys(v),
        exp,
        "{what} JSON keys must match the wire contract"
    );
}

#[test]
fn alignment_matrix_keys_match_frontend_contract() {
    let snap = build_realistic_snapshot();
    let v = serde_json::to_value(snap.alignment.as_ref().unwrap()).unwrap();
    // `ui/src/types.ts` `AlignmentMatrix` (12 fields).
    assert_keys_eq(
        &v,
        &[
            "symbol",
            "timeframes_present",
            "dimensions",
            "mtf_trend_alignment",
            "mtf_momentum_alignment",
            "mtf_volume_alignment",
            "mtf_volatility_alignment",
            "mtf_overall_score",
            "mtf_overall_label",
            "timeframe_alignments",
            "signal_cross_tf_count",
            "trend_agreement_pct",
        ],
        "AlignmentMatrix",
    );
    let row = &v["timeframe_alignments"];
    assert!(row.is_array() && row.as_array().map(|a| a.is_empty()).unwrap_or(false));
}

#[test]
fn analysis_matrix_keys_match_frontend_contract() {
    let snap = build_realistic_snapshot();
    let v = serde_json::to_value(snap.analysis.as_ref().unwrap()).unwrap();
    // `ui/src/types.ts` `AnalysisMatrix` — the full 20-field wire shape.
    assert_keys_eq(
        &v,
        &[
            "symbol",
            "bias",
            "market_bias_score",
            "state_confidence",
            "confidence",
            "market_regime",
            "trend_assessment",
            "momentum_assessment",
            "structure_assessment",
            "volatility_assessment",
            "volume_assessment",
            "opportunity_analysis",
            "market_quality",
            "market_quality_score",
            "market_phase",
            "market_interpretation",
            "rationale",
            "supporting_signals",
            "contradicting_signals",
            "timeframes_considered",
        ],
        "AnalysisMatrix",
    );
}

#[test]
fn risk_matrix_keys_match_frontend_contract() {
    let snap = build_realistic_snapshot();
    let v = serde_json::to_value(snap.risk.as_ref().unwrap()).unwrap();
    // `ui/src/types.ts` `RiskMatrix` — includes the Phase-3 rename
    // `execution_liquidity_risk` and `cascade_risk`.
    assert_keys_eq(
        &v,
        &[
            "symbol",
            "market_risk",
            "volatility_risk",
            "execution_liquidity_risk",
            "structure_risk",
            "momentum_risk",
            "signal_risk",
            "execution_risk",
            "cascade_risk",
            "overall_risk",
        ],
        "RiskMatrix",
    );
}

#[test]
fn advisory_matrix_keys_match_frontend_contract() {
    let snap = build_realistic_snapshot();
    let v = serde_json::to_value(snap.advisory.as_ref().unwrap()).unwrap();
    // `ui/src/types.ts` `AdvisoryMatrix` — 14 required fields (incl.
    // `cascade_risk_score` and the `environment_favorability` band).
    assert_keys_eq(
        &v,
        &[
            "symbol",
            "directional_guidance",
            "market_stance",
            "opportunity_classification",
            "strategy_environment",
            "entry_guidance",
            "exit_guidance",
            "protection_strategy",
            "target_strategy",
            "confidence_assessment",
            "stop_loss_distance_pct",
            "cascade_risk_score",
            "environment_favorability",
            "final_recommendation",
        ],
        "AdvisoryMatrix",
    );
}

#[test]
fn decision_context_keys_match_frontend_contract() {
    let snap = build_realistic_snapshot();
    let v = serde_json::to_value(snap.decision_context.as_ref().unwrap()).unwrap();
    // `ui/src/types.ts` `DecisionContext`.
    assert_keys_eq(
        &v,
        &[
            "score",
            "bias",
            "score_confidence",
            "entry_danger",
            "expected_reward_risk_ratio",
            "trade_readiness",
            "contributing_indicators",
            "long_probability",
            "short_probability",
            "hold_probability",
            "net_bias_pct",
        ],
        "DecisionContext",
    );
}

#[test]
fn market_context_keys_match_frontend_contract() {
    let snap = build_realistic_snapshot();
    let v = serde_json::to_value(snap.context.as_ref().unwrap()).unwrap();
    // `ui/src/types.ts` `MarketContext` — 5 dimensions + regime + score + label.
    assert_keys_eq(
        &v,
        &[
            "trend",
            "momentum",
            "volatility",
            "volume",
            "liquidity",
            "regime",
            "overall_score",
            "overall_label",
        ],
        "MarketContext",
    );
}

#[test]
fn snapshot_top_level_keys_match_frontend_contract() {
    let snap = build_realistic_snapshot();
    let v = serde_json::to_value(&snap).unwrap();
    // Top-level `MarketSnapshot` keys consumed by the store
    // (`ui/src/lib/websocket.svelte.ts`) and the metrics/MTF builders.
    let keys = sorted_keys(&v);
    for required in [
        "timeframe_slot",
        "exchange",
        "timeframe_secs",
        "timestamp",
        "symbol",
        "is_completed",
        "mid_price",
        "bid_price",
        "ask_price",
        "open",
        "high",
        "low",
        "close",
        "volume",
        "indicators",
        "context",
        "alignment",
        "analysis",
        "risk",
        "advisory",
        "decision_context",
        "open_interest",
        "oi_delta_1h",
        "mark_price",
        "index_price",
        "mark_index_spread_pct",
        "prev_day_px",
        "quality_envelope",
        "pipeline_state",
        "indicator_lifecycle",
    ] {
        assert!(
            keys.contains(&required.to_string()),
            "MarketSnapshot must carry `{required}` on the wire"
        );
    }
    // `liquidity_signals` is skipped when empty
    // (`#[serde(skip_serializing_if = "Vec::is_empty")]` in models.rs) —
    // when present it must be an array.
    if keys.contains(&"liquidity_signals".to_string()) {
        assert!(v["liquidity_signals"].is_array());
    }
    // The export builders read `snapshot.context` blocks verbatim — the
    // JSON here must be object-shaped, not a string.
    assert!(
        v["context"].is_object(),
        "context must serialize as an object"
    );
    assert!(
        v["indicators"].is_object(),
        "indicators must serialize as an object"
    );
    assert_eq!(v["symbol"], json!("BTC-USDT"));
}
