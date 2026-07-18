use config_models::{ExecutionPolicy, Direction, Stance};
use core_domain::models::MarketSnapshot;
use std::collections::HashMap;

use crate::policy::evaluator::evaluate_condition_group;

#[derive(Debug, Clone)]
pub struct PolicyTrigger {
    pub policy_id: String,
    pub symbol: String,
    pub direction: Direction,
    pub trigger_timestamp: u64,
    pub decision_context_snapshot: serde_json::Value,
    pub stance: Stance,
    pub risk_parameters: config_models::RiskParams,
}

pub struct PolicyEngine {
    policies: Vec<ExecutionPolicy>,
    stances: HashMap<String, Stance>,
    cooldowns: HashMap<String, u64>,
    policy_paused: HashMap<String, bool>,
}

impl PolicyEngine {
    pub fn new(policies: Vec<ExecutionPolicy>) -> Self {
        let mut stances = HashMap::new();
        for p in &policies {
            stances.entry(p.symbol.clone()).or_insert(Stance::Active);
        }
        Self {
            policies,
            stances,
            cooldowns: HashMap::new(),
            policy_paused: HashMap::new(),
        }
    }

    pub fn set_stance(&mut self, symbol: &str, stance: Stance) {
        self.stances.insert(symbol.to_string(), stance);
    }

    pub fn get_stance(&self, symbol: &str) -> Stance {
        self.stances.get(symbol).copied().unwrap_or(Stance::Active)
    }

    pub fn set_policy_auto_paused(&mut self, policy_id: &str, paused: bool) {
        self.policy_paused.insert(policy_id.to_string(), paused);
    }

    pub fn evaluate_policies(
        &mut self,
        snapshot: &MarketSnapshot,
        current_time: u64,
    ) -> Vec<PolicyTrigger> {
        let symbol = &snapshot.symbol;
        let symbol_stance = self.get_stance(symbol);

        if symbol_stance == Stance::Avoid {
            return vec![];
        }

        let mut triggers: Vec<PolicyTrigger> = Vec::new();

        for policy in &self.policies {
            if !policy.enabled {
                continue;
            }
            if policy.symbol != *symbol {
                continue;
            }
            if self.policy_paused.get(&policy.policy_id).copied().unwrap_or(false) {
                continue;
            }

            if let Some(cooldown_until) = self.cooldowns.get(&policy.policy_id) {
                if current_time < *cooldown_until {
                    continue;
                }
            }

            let conditions_met = evaluate_condition_group(&policy.conditions, snapshot);
            if !conditions_met {
                continue;
            }

            if symbol_stance == Stance::CloseOnly {
                if triggers.iter().any(|t| t.direction != policy.direction) {
                    eprintln!(
                        "TAE: Conflict blocked under CLOSE_ONLY — {:?} vs {:?}",
                        triggers[0].direction, policy.direction
                    );
                    triggers.clear();
                    break;
                }
            } else {
                if let Some(existing) = triggers.iter().find(|t| t.direction != policy.direction) {
                    eprintln!(
                        "TAE: Conflict blocked — {} ({:?}) vs {} ({:?})",
                        existing.policy_id, existing.direction, policy.policy_id, policy.direction
                    );
                    triggers.clear();
                    break;
                }

                let existing_same_dir = triggers.iter()
                    .find(|t| t.direction == policy.direction)
                    .map(|t| (t.policy_id.clone(), t.decision_context_snapshot
                        .get("confidence").and_then(|c| c.as_f64()).unwrap_or(0.0)));
                if let Some((existing_id, existing_conf)) = existing_same_dir {
                    let new_conf = snapshot.decision_context.as_ref()
                        .map(|d| d.confidence).unwrap_or(0.0);
                    if new_conf <= existing_conf {
                        continue;
                    }
                    triggers.retain(|t| t.policy_id != existing_id);
                }
            }

            if policy.cooldown_seconds > 0 {
                self.cooldowns.insert(
                    policy.policy_id.clone(),
                    current_time + policy.cooldown_seconds,
                );
            }

            let dc_snapshot = snapshot.decision_context.as_ref().map(|dc| {
                serde_json::json!({
                    "score": dc.score,
                    "bias": dc.bias,
                    "confidence": dc.confidence,
                    "trade_readiness": dc.trade_readiness,
                })
            }).unwrap_or(serde_json::Value::Null);

            triggers.push(PolicyTrigger {
                policy_id: policy.policy_id.clone(),
                symbol: symbol.clone(),
                direction: policy.direction,
                trigger_timestamp: current_time,
                decision_context_snapshot: dc_snapshot,
                stance: symbol_stance,
                risk_parameters: policy.risk.clone(),
            });
        }

        triggers
    }

    pub fn get_active_stance_policies(&self) -> Vec<&ExecutionPolicy> {
        self.policies.iter().filter(|p| p.enabled).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use config_models::{Condition, ConditionGroup, ConditionValue, Operator, RiskParams, TriggerMode};

    fn make_policy(id: &str, symbol: &str, dir: Direction) -> ExecutionPolicy {
        ExecutionPolicy {
            policy_id: id.into(),
            policy_name: format!("Policy {}", id),
            description: String::new(),
            symbol: symbol.into(),
            direction: dir,
            conditions: ConditionGroup::And(vec![Condition {
                field: "risk.overall_risk.score".into(),
                operator: Operator::Lt,
                value: ConditionValue::Number(50.0),
            }]),
            trigger_mode: TriggerMode::Interval { seconds: 300 },
            risk: RiskParams::default(),
            enabled: true,
            cooldown_seconds: 0,
            reduce_only_on_close_only: true,
        }
    }

    fn make_snapshot(symbol: &str, risk_score: f64) -> MarketSnapshot {
        use core_domain::analysis::{
            AnalysisMatrix, MarketBias, MarketRegime, TrendAssessment, MomentumAssessment,
            StructureAssessment, VolatilityAssessment, VolumeAssessment, OpportunityType, QualityLevel,
        };
        use core_domain::decision_context::DecisionContext;
        use core_domain::risk::{RiskDimension, RiskLevel, RiskMatrix, RiskState};
        use rust_decimal::Decimal;
        use std::collections::HashMap;

        let risk_dim = RiskDimension {
            score: risk_score,
            level: RiskLevel::Low,
            state: RiskState::Stable,
            confidence: 80.0,
            evidence: vec![],
        };

        MarketSnapshot {
            exchange: None,
            timeframe_secs: 60,
            timestamp: 1000,
            symbol: symbol.into(),
            is_completed: Some(true),
            mid_price: Decimal::from(50000),
            bid_price: Decimal::from(49999),
            ask_price: Decimal::from(50001),
            bid_size: None,
            ask_size: None,
            funding_rate: None,
            open: None, high: None, low: None, close: None,
            volume: None, average_volume: None,
            indicators: HashMap::new(),
            context: None,
            alignment: None,
            analysis: Some(AnalysisMatrix {
                symbol: symbol.into(),
                bias: MarketBias::StrongBullish,
                state_confidence: 0.85,
                market_regime: MarketRegime::TrendingBull,
                trend_assessment: TrendAssessment::Healthy,
                momentum_assessment: MomentumAssessment::Stable,
                structure_assessment: StructureAssessment::Healthy,
                volatility_assessment: VolatilityAssessment::Normal,
                volume_assessment: VolumeAssessment::Normal,
                opportunity_analysis: OpportunityType::Breakout,
                market_quality: QualityLevel::Good,
                market_quality_score: 70.0,
                market_interpretation: "Test".into(),
                rationale: String::new(),
                supporting_signals: vec![],
                contradicting_signals: vec![],
                timeframes_considered: 4,
            }),
            risk: Some(RiskMatrix {
                symbol: symbol.into(),
                market_risk: risk_dim.clone(),
                volatility_risk: risk_dim.clone(),
                execution_liquidity_risk: risk_dim.clone(),
                structure_risk: risk_dim.clone(),
                momentum_risk: risk_dim.clone(),
                signal_risk: risk_dim.clone(),
                execution_risk: risk_dim.clone(),
                cascade_risk: risk_dim.clone(),
                overall_risk: risk_dim,
            }),
            advisory: None,
            open_interest: None, oi_delta_1h: None,
            mark_price: None, index_price: None, mark_index_spread_pct: None,
            prev_day_px: None,
            statistical_context: None,
            decision_context: Some(DecisionContext {
                score: 97.0,
                bias: "STRONG_BULLISH".into(),
                confidence: 0.97,
                score_confidence: 0.97,
                entry_danger: 12.5,
                expected_reward_risk_ratio: 1.79,
                trade_readiness: "READY".into(),
                contributing_indicators: vec!["ema_stack".into()],
            }),
            risk_profile: None,
            liquidity: None,
            cluster: None,
            liquidity_signals: vec![],
            metrics_config: None,
            opportunity: None,
            quality_envelope: None,
        }
    }

    #[test]
    fn test_policy_triggers_on_low_risk() {
        let mut engine = PolicyEngine::new(vec![make_policy("test1", "BTC-USDT", Direction::Long)]);
        let snap = make_snapshot("BTC-USDT", 30.0);
        let triggers = engine.evaluate_policies(&snap, 1000);
        assert_eq!(triggers.len(), 1);
        assert_eq!(triggers[0].policy_id, "test1");
    }

    #[test]
    fn test_policy_skipped_on_high_risk() {
        let mut engine = PolicyEngine::new(vec![make_policy("test1", "BTC-USDT", Direction::Long)]);
        let snap = make_snapshot("BTC-USDT", 60.0);
        let triggers = engine.evaluate_policies(&snap, 1000);
        assert!(triggers.is_empty());
    }

    #[test]
    fn test_avoid_stance_blocks_all() {
        let mut engine = PolicyEngine::new(vec![make_policy("test1", "BTC-USDT", Direction::Long)]);
        engine.set_stance("BTC-USDT", Stance::Avoid);
        let snap = make_snapshot("BTC-USDT", 30.0);
        let triggers = engine.evaluate_policies(&snap, 1000);
        assert!(triggers.is_empty());
    }

    #[test]
    fn test_opposite_direction_conflict_blocked() {
        let mut engine = PolicyEngine::new(vec![
            make_policy("long1", "BTC-USDT", Direction::Long),
            make_policy("short1", "BTC-USDT", Direction::Short),
        ]);
        let snap = make_snapshot("BTC-USDT", 30.0);
        let triggers = engine.evaluate_policies(&snap, 1000);
        assert!(triggers.is_empty());
    }
}
