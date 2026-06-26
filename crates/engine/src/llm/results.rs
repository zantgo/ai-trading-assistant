use super::types::{
    IndividualIndicatorResult, MultiAgentResults,
};

// ─── Multi-Agent Pipeline Results ─────────────────────────────────

impl MultiAgentResults {
    pub fn to_legacy_signals(&self) -> Vec<IndividualIndicatorResult> {
        let trend_bias = &self.trend.data.directional_bias;
        let trend_thought = &self.trend.thought;
        let vol_thought = &self.volatility.thought;
        let vol_regime = &self.volatility.data.regime_classification;

        let squeeze_signal = match vol_regime.as_str() {
            "COMPRESSION" => "SIDEWAYS".to_string(),
            _ => trend_bias.clone(),
        };

        let adx_signal = match vol_regime.as_str() {
            "RANGE" => "SIDEWAYS".to_string(),
            _ => trend_bias.clone(),
        };

        vec![
            IndividualIndicatorResult {
                indicator_name: "micro-RSI".to_string(),
                signal: trend_bias.clone(),
                reason: trend_thought.clone(),
                confidence_score: 0,
                divergence_status: None,
                divergence_type: None,
                is_confirmed: None,
            },
            IndividualIndicatorResult {
                indicator_name: "micro-MACD".to_string(),
                signal: trend_bias.clone(),
                reason: trend_thought.clone(),
                confidence_score: 0,
                divergence_status: None,
                divergence_type: None,
                is_confirmed: None,
            },
            IndividualIndicatorResult {
                indicator_name: "small-SQUEEZE".to_string(),
                signal: squeeze_signal,
                reason: vol_thought.clone(),
                confidence_score: 0,
                divergence_status: None,
                divergence_type: None,
                is_confirmed: None,
            },
            IndividualIndicatorResult {
                indicator_name: "medium-ADX".to_string(),
                signal: adx_signal,
                reason: vol_thought.clone(),
                confidence_score: 0,
                divergence_status: None,
                divergence_type: None,
                is_confirmed: None,
            },
            IndividualIndicatorResult {
                indicator_name: "large-VWAP".to_string(),
                signal: trend_bias.clone(),
                reason: trend_thought.clone(),
                confidence_score: 0,
                divergence_status: None,
                divergence_type: None,
                is_confirmed: None,
            },
        ]
    }
}
