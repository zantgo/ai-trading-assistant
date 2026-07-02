//! Phase 6 verification: a mock orchestrator response matching the prompt's
//! OUTPUT SCHEMA still deserializes cleanly into `MasterOrchestratorResult`
//! after the continuous-scale prompt/ingestion changes.

use engine::llm::MasterOrchestratorResult;

#[test]
fn mock_orchestrator_json_parses_into_dto() {
    // Mirrors the OUTPUT SCHEMA declared in the (updated) orchestrator prompts.
    let mock = r#"{
        "general_trend": "UPWARD",
        "support_and_resistance": {
            "structural_analysis": "Price holding above S1 with a confirmed resistance flip."
        },
        "indicator_synthesis": {
            "summary_count": "5 Bullish, 1 Bearish, 2 Sideways",
            "evaluation": "Continuous vectors converge bullishly; confluence score is strongly positive."
        },
        "position_recommendation": {
            "action": "Open Long",
            "rationale": "High-magnitude bullish normalized floats with volume confirmation.",
            "next_interval": "fast"
        },
        "eight_factor_score": 72,
        "allocation_pct": 3.0
    }"#;

    let result: MasterOrchestratorResult =
        serde_json::from_str(mock).expect("mock orchestrator JSON must parse into the DTO");

    assert_eq!(result.general_trend, "UPWARD");
    assert_eq!(result.position_recommendation.action, "Open Long");
    assert_eq!(
        result.position_recommendation.next_interval.as_deref(),
        Some("fast")
    );
    assert_eq!(result.eight_factor_score, 72);
    assert_eq!(result.allocation_pct, 3.0);
    assert_eq!(
        result.support_and_resistance.structural_analysis,
        "Price holding above S1 with a confirmed resistance flip."
    );

    // Round-trips back to JSON (Serialize contract intact).
    let round = serde_json::to_string(&result).expect("DTO must serialize");
    assert!(round.contains("\"general_trend\""));
}

#[test]
fn orchestrator_dto_tolerates_missing_optional_score_fields() {
    // eight_factor_score / allocation_pct are #[serde(default)] — older/minimal
    // model outputs that omit them must still parse.
    let minimal = r#"{
        "general_trend": "SIDEWAYS",
        "indicator_synthesis": { "summary_count": "3/3", "evaluation": "Flat." },
        "position_recommendation": { "action": "Wait", "rationale": "Equilibrium." }
    }"#;
    let result: MasterOrchestratorResult =
        serde_json::from_str(minimal).expect("minimal JSON must parse with defaults");
    assert_eq!(result.eight_factor_score, 0);
    assert_eq!(result.allocation_pct, 0.0);
    assert_eq!(result.position_recommendation.action, "Wait");
}
