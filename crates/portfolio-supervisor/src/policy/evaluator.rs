use config_models::{Condition, ConditionGroup, ConditionValue, Operator};
use core_domain::analysis::OpportunityType;
use core_domain::models::MarketSnapshot;

pub fn evaluate_condition_group(group: &ConditionGroup, snapshot: &MarketSnapshot) -> bool {
    match group {
        ConditionGroup::And(conditions) => {
            for condition in conditions {
                if !evaluate_condition(condition, snapshot) {
                    return false;
                }
            }
            true
        }
        ConditionGroup::Or(conditions) => {
            for condition in conditions {
                if evaluate_condition(condition, snapshot) {
                    return true;
                }
            }
            false
        }
    }
}

pub fn evaluate_condition(condition: &Condition, snapshot: &MarketSnapshot) -> bool {
    let str_val = resolve_field_string(&condition.field, snapshot);
    let num_val = resolve_field_numeric(&condition.field, snapshot);

    match &condition.value {
        ConditionValue::String(_) | ConditionValue::StringList(_) => {
            match str_val {
                Some(ref sv) => evaluate_operator_string(&condition.operator, sv, &condition.value),
                None => false,
            }
        }
        ConditionValue::Number(_) | ConditionValue::NumberList(_) => {
            match num_val {
                Some(v) => evaluate_operator_numeric(&condition.operator, v, &condition.value),
                None => false,
            }
        }
    }
}

fn resolve_field_string(field: &str, snapshot: &MarketSnapshot) -> Option<String> {
    let dc = snapshot.decision_context.as_ref();
    let ad = snapshot.advisory.as_ref();
    let analysis = snapshot.analysis.as_ref();
    let opp = snapshot.opportunity.as_ref();

    match field {
        "decision.bias" => dc.map(|d| d.bias.clone()),
        "decision.market_stance" => ad.map(|a| format!("{}", a.market_stance)),
        "decision.directional_guidance" => ad.map(|a| format!("{:?}", a.directional_guidance)),
        "decision.strategy_environment" => ad.map(|a| format!("{:?}", a.strategy_environment)),
        "decision.entry_guidance" => ad.map(|a| format!("{:?}", a.entry_guidance)),
        "decision.trade_readiness" => dc.map(|d| d.trade_readiness.clone()),
        "analysis.market_regime" => analysis.map(|a| a.market_regime.to_string()),
        "analysis.market_quality" => analysis.map(|a| format!("{:?}", a.market_quality)),
        "analysis.market_interpretation" => analysis.map(|a| a.market_interpretation.clone()),
        "opportunity.primary_opportunity" => opp.map(|o| format!("{:?}", o.primary_opportunity)),
        "opportunity.setup_quality" => opp.map(|o| format!("{:?}", o.setup_quality)),
        _ => None,
    }
}

fn resolve_field_numeric(field: &str, snapshot: &MarketSnapshot) -> Option<f64> {
    let dc = snapshot.decision_context.as_ref();
    let ad = snapshot.advisory.as_ref();
    let risk = snapshot.risk.as_ref();
    let opp = snapshot.opportunity.as_ref();
    let analysis = snapshot.analysis.as_ref();

    match field {
        "decision.bias" => dc.and_then(|d| bias_numeric(&d.bias)),
        "decision.confidence_assessment" => dc.map(|d| d.confidence * 100.0),
        "decision.score" => dc.map(|d| d.score),
        "decision.expected_reward_risk_ratio" => dc.map(|d| d.expected_reward_risk_ratio),
        "decision.entry_danger.score" => dc.map(|d| d.entry_danger),
        "decision.market_stance" => ad.and_then(|a| stance_numeric(a.market_stance)),
        "decision.directional_guidance" => ad.and_then(|a| dir_guidance_numeric(a.directional_guidance)),
        "decision.strategy_environment" => ad.and_then(|a| strategy_env_numeric(a.strategy_environment)),
        "decision.entry_guidance" => ad.and_then(|a| entry_guidance_numeric(a.entry_guidance)),
        "analysis.market_regime" => ad.and_then(|_| regime_numeric(snapshot)),
        "analysis.market_quality" => ad.and_then(|_| quality_numeric(snapshot)),
        "analysis.market_bias_score" => analysis.map(|a| a.market_bias_score),
        "analysis.state_confidence" => analysis.map(|a| a.state_confidence * 100.0),
        "opportunity.primary_opportunity" => opp.map(|o| opportunity_type_numeric(o.primary_opportunity)),
        "opportunity.opportunity_score" => opp.map(|o| o.opportunity_score),
        "risk.market_risk.score" => risk.map(|r| r.market_risk.score),
        "risk.market_risk.level" => risk.map(|r| risk_level_numeric(&r.market_risk.level)),
        "risk.volatility_risk.score" => risk.map(|r| r.volatility_risk.score),
        "risk.execution_liquidity_risk.score" => risk.map(|r| r.execution_liquidity_risk.score),
        "risk.structure_risk.score" => risk.map(|r| r.structure_risk.score),
        "risk.momentum_risk.score" => risk.map(|r| r.momentum_risk.score),
        "risk.signal_risk.score" => risk.map(|r| r.signal_risk.score),
        "risk.execution_risk.score" => risk.map(|r| r.execution_risk.score),
        "risk.cascade_risk.score" => risk.map(|r| r.cascade_risk.score),
        "risk.overall_risk.score" => risk.map(|r| r.overall_risk.score),
        "risk.overall_risk.confidence" => risk.map(|r| r.overall_risk.confidence),
        _ => None,
    }
}

fn risk_level_numeric(level: &core_domain::risk::RiskLevel) -> f64 {
    match level {
        core_domain::risk::RiskLevel::Extreme => 100.0,
        core_domain::risk::RiskLevel::High => 75.0,
        core_domain::risk::RiskLevel::Moderate => 50.0,
        core_domain::risk::RiskLevel::Low => 25.0,
        core_domain::risk::RiskLevel::VeryLow => 0.0,
    }
}

fn bias_numeric(bias: &str) -> Option<f64> {
    Some(match bias {
        "STRONG_BULLISH" => 5.0,
        "BULLISH" => 3.0,
        "BEARISH" => 1.0,
        "STRONG_BEARISH" => 0.0,
        _ => 2.0,
    })
}

fn stance_numeric(s: core_domain::advisory::MarketStance) -> Option<f64> {
    Some(match s {
        core_domain::advisory::MarketStance::Aggressive => 5.0,
        core_domain::advisory::MarketStance::Constructive => 4.0,
        core_domain::advisory::MarketStance::Neutral => 3.0,
        core_domain::advisory::MarketStance::Cautious => 2.0,
        core_domain::advisory::MarketStance::Avoid => 1.0,
    })
}

fn dir_guidance_numeric(d: core_domain::advisory::DirectionalGuidance) -> Option<f64> {
    Some(match d {
        core_domain::advisory::DirectionalGuidance::StrongLong => 5.0,
        core_domain::advisory::DirectionalGuidance::Long => 4.0,
        core_domain::advisory::DirectionalGuidance::Neutral => 3.0,
        core_domain::advisory::DirectionalGuidance::Short => 2.0,
        core_domain::advisory::DirectionalGuidance::StrongShort => 1.0,
        core_domain::advisory::DirectionalGuidance::AvoidDirectionalExposure => 0.0,
    })
}

fn strategy_env_numeric(e: core_domain::advisory::StrategyEnvironment) -> Option<f64> {
    Some(match e {
        core_domain::advisory::StrategyEnvironment::TrendFollowing => 5.0,
        core_domain::advisory::StrategyEnvironment::Breakout => 4.0,
        core_domain::advisory::StrategyEnvironment::MeanReversion => 3.0,
        core_domain::advisory::StrategyEnvironment::HighVolatility => 2.0,
        core_domain::advisory::StrategyEnvironment::LowActivity => 1.0,
        core_domain::advisory::StrategyEnvironment::Unfavorable => 0.0,
    })
}

fn entry_guidance_numeric(e: core_domain::advisory::EntryGuidance) -> Option<f64> {
    Some(match e {
        core_domain::advisory::EntryGuidance::Immediate => 5.0,
        core_domain::advisory::EntryGuidance::WaitForConfirmation => 3.0,
        core_domain::advisory::EntryGuidance::Pullback => 4.0,
        core_domain::advisory::EntryGuidance::Breakout => 4.0,
        core_domain::advisory::EntryGuidance::NoEntryContext => 0.0,
    })
}

fn regime_numeric(snapshot: &MarketSnapshot) -> Option<f64> {
    snapshot.analysis.as_ref().map(|a| match a.market_regime {
        core_domain::analysis::MarketRegime::TrendingBull => 5.0,
        core_domain::analysis::MarketRegime::TrendingBear => 1.0,
        core_domain::analysis::MarketRegime::Range => 3.0,
        core_domain::analysis::MarketRegime::Accumulation => 4.0,
        core_domain::analysis::MarketRegime::Distribution => 2.0,
        core_domain::analysis::MarketRegime::Expansion => 3.0,
        core_domain::analysis::MarketRegime::Contraction => 2.0,
        core_domain::analysis::MarketRegime::Transition => 2.0,
    })
}

fn quality_numeric(snapshot: &MarketSnapshot) -> Option<f64> {
    snapshot.analysis.as_ref().map(|a| match a.market_quality {
        core_domain::analysis::QualityLevel::Excellent => 5.0,
        core_domain::analysis::QualityLevel::Good => 4.0,
        core_domain::analysis::QualityLevel::Average => 3.0,
        core_domain::analysis::QualityLevel::Weak => 2.0,
        core_domain::analysis::QualityLevel::Poor => 1.0,
    })
}

fn opportunity_type_numeric(ot: OpportunityType) -> f64 {
    match ot {
        OpportunityType::Breakout | OpportunityType::TrendContinuation => 5.0,
        OpportunityType::LiquiditySqueeze | OpportunityType::Reversal => 4.0,
        OpportunityType::Pullback | OpportunityType::MeanReversion => 3.0,
        _ => 1.0,
    }
}

fn evaluate_operator_numeric(op: &Operator, field_val: f64, cond_val: &ConditionValue) -> bool {
    match op {
        Operator::Eq => match cond_val {
            ConditionValue::Number(n) => (field_val - n).abs() < 1e-9,
            _ => false,
        },
        Operator::Gt => match cond_val {
            ConditionValue::Number(n) => field_val > *n,
            _ => false,
        },
        Operator::Lt => match cond_val {
            ConditionValue::Number(n) => field_val < *n,
            _ => false,
        },
        Operator::Gte => match cond_val {
            ConditionValue::Number(n) => field_val >= *n,
            _ => false,
        },
        Operator::Lte => match cond_val {
            ConditionValue::Number(n) => field_val <= *n,
            _ => false,
        },
        Operator::NotEq => match cond_val {
            ConditionValue::Number(n) => (field_val - n).abs() >= 1e-9,
            _ => true,
        },
        Operator::In => match cond_val {
            ConditionValue::NumberList(nums) => nums.iter().any(|n| (field_val - n).abs() < 1e-9),
            _ => false,
        },
        Operator::Between => match cond_val {
            ConditionValue::NumberList(nums) if nums.len() == 2 => {
                let (lo, hi) = (nums[0].min(nums[1]), nums[0].max(nums[1]));
                field_val >= lo && field_val <= hi
            }
            _ => false,
        },
    }
}

fn evaluate_operator_string(op: &Operator, field_val: &str, cond_val: &ConditionValue) -> bool {
    match op {
        Operator::Eq => match cond_val {
            ConditionValue::String(s) => field_val == s.as_str(),
            _ => false,
        },
        Operator::In => match cond_val {
            ConditionValue::StringList(list) => list.iter().any(|s| s == field_val),
            _ => false,
        },
        Operator::NotEq => match cond_val {
            ConditionValue::String(s) => field_val != s.as_str(),
            _ => true,
        },
        Operator::Gt | Operator::Lt | Operator::Gte | Operator::Lte | Operator::Between => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_and_all_true() {
        let group = ConditionGroup::And(vec![
            Condition {
                field: "risk.overall_risk.score".into(),
                operator: Operator::Lt,
                value: ConditionValue::Number(50.0),
            },
        ]);
        let mut snap = stub_snapshot();
        snap.risk.as_mut().unwrap().overall_risk.score = 30.0;
        assert!(evaluate_condition_group(&group, &snap));
    }

    #[test]
    fn test_and_one_false_short_circuits() {
        let group = ConditionGroup::And(vec![
            Condition {
                field: "risk.overall_risk.score".into(),
                operator: Operator::Lt,
                value: ConditionValue::Number(50.0),
            },
            Condition {
                field: "decision.confidence_assessment".into(),
                operator: Operator::Gte,
                value: ConditionValue::Number(60.0),
            },
        ]);
        let mut snap = stub_snapshot();
        snap.risk.as_mut().unwrap().overall_risk.score = 60.0;
        assert!(!evaluate_condition_group(&group, &snap));
    }

    #[test]
    fn test_or_first_true() {
        let group = ConditionGroup::Or(vec![
            Condition {
                field: "risk.overall_risk.score".into(),
                operator: Operator::Gt,
                value: ConditionValue::Number(50.0),
            },
            Condition {
                field: "decision.confidence_assessment".into(),
                operator: Operator::Gte,
                value: ConditionValue::Number(60.0),
            },
        ]);
        let snap = stub_snapshot();
        assert!(evaluate_condition_group(&group, &snap));
    }

    #[test]
    fn test_operators() {
        assert!(evaluate_operator_numeric(&Operator::Eq, 3.0, &ConditionValue::Number(3.0)));
        assert!(!evaluate_operator_numeric(&Operator::Eq, 3.0, &ConditionValue::Number(3.1)));
        assert!(evaluate_operator_numeric(&Operator::Gt, 5.0, &ConditionValue::Number(3.0)));
        assert!(evaluate_operator_numeric(&Operator::Lt, 2.0, &ConditionValue::Number(5.0)));
        assert!(evaluate_operator_numeric(&Operator::Gte, 3.0, &ConditionValue::Number(3.0)));
        assert!(evaluate_operator_numeric(&Operator::Lte, 3.0, &ConditionValue::Number(3.0)));
        assert!(evaluate_operator_numeric(&Operator::NotEq, 3.0, &ConditionValue::Number(4.0)));
        assert!(evaluate_operator_numeric(&Operator::In, 3.0, &ConditionValue::NumberList(vec![1.0, 2.0, 3.0])));
        assert!(evaluate_operator_numeric(&Operator::Between, 5.0, &ConditionValue::NumberList(vec![1.0, 10.0])));
        assert!(!evaluate_operator_numeric(&Operator::Between, 15.0, &ConditionValue::NumberList(vec![1.0, 10.0])));
    }

    #[test]
    fn test_string_in_operator() {
        assert!(evaluate_operator_string(&Operator::In, "BULLISH",
            &ConditionValue::StringList(vec!["BULLISH".into(), "STRONG_BULLISH".into()])));
        assert!(!evaluate_operator_string(&Operator::In, "NEUTRAL",
            &ConditionValue::StringList(vec!["BULLISH".into(), "STRONG_BULLISH".into()])));
    }

    #[test]
    fn test_string_eq_operator() {
        assert!(evaluate_operator_string(&Operator::Eq, "BREAKOUT",
            &ConditionValue::String("BREAKOUT".into())));
        assert!(!evaluate_operator_string(&Operator::Eq, "BREAKOUT",
            &ConditionValue::String("TREND_CONTINUATION".into())));
    }

    #[test]
    fn test_entry_guidance_maps_correctly() {
        let snap = stub_snapshot();
        let val = resolve_field_string("decision.entry_guidance", &snap);
        assert!(val.is_some());
        let val_str = val.unwrap();
        assert!(val_str.contains("Immediate") || val_str.contains("IMMEDIATE"));
    }

    #[test]
    fn test_opportunity_fields_resolved() {
        let mut snap = stub_snapshot();
        snap.opportunity = Some(core_domain::opportunity::OpportunityMatrix {
            symbol: "BTC-USDT".into(),
            primary_opportunity: OpportunityType::Breakout,
            opportunity_score: 85.0,
            setup_quality: core_domain::analysis::SetupQuality::Prime,
            profiles: vec![],
            forecast_confidence: 0.85,
            entry_zone: core_domain::opportunity::PriceRange { low: 0.0, high: 0.0 },
            target_zone: core_domain::opportunity::PriceRange { low: 0.0, high: 0.0 },
            invalidation_level: 0.0,
            expected_rr_internal: 2.5,
            time_horizon: "SWING".into(),
            ..Default::default()
        });
        let type_val = resolve_field_string("opportunity.primary_opportunity", &snap);
        assert_eq!(type_val, Some("Breakout".into()));
        let score_val = resolve_field_numeric("opportunity.opportunity_score", &snap);
        assert_eq!(score_val, Some(85.0));
    }

    fn stub_snapshot() -> MarketSnapshot {
        use core_domain::advisory::{
            AdvisoryMatrix, DirectionalGuidance, MarketStance, OpportunityClass,
            StrategyEnvironment, EntryGuidance, ExitGuidance, ProtectionStrategy, TargetStrategy,
        };
        use core_domain::analysis::{
            AnalysisMatrix, MarketBias, MarketRegime, TrendAssessment, MomentumAssessment,
            StructureAssessment, VolatilityAssessment, VolumeAssessment, OpportunityType, QualityLevel,
        };
        use core_domain::decision_context::DecisionContext;
        use core_domain::risk::{
            RiskDimension, RiskLevel, RiskMatrix, RiskState,
        };
        use rust_decimal::Decimal;
        use std::collections::HashMap;

        let risk_dim = RiskDimension {
            score: 28.3,
            level: RiskLevel::Low,
            state: RiskState::Stable,
            confidence: 80.0,
            evidence: vec![],
        };

        MarketSnapshot {
            timeframe_slot: Some(core_domain::models::TimeframeSlot::Micro),
            exchange: None,
            timeframe_secs: 60,
            timestamp: 1000,
            symbol: "BTC-USDT".into(),
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
            context: None, alignment: None,
            analysis: Some(AnalysisMatrix {
                symbol: "BTC-USDT".into(),
                bias: MarketBias::StrongBullish,
                market_bias_score: 85.0,
                state_confidence: 0.85,
                confidence: 0.85,
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
                symbol: "BTC-USDT".into(),
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
            advisory: Some(AdvisoryMatrix {
                symbol: "BTC-USDT".into(),
                directional_guidance: DirectionalGuidance::StrongLong,
                market_stance: MarketStance::Constructive,
                opportunity_classification: OpportunityClass::Breakout,
                strategy_environment: StrategyEnvironment::TrendFollowing,
                entry_guidance: EntryGuidance::Immediate,
                exit_guidance: ExitGuidance::NoWarning,
                protection_strategy: ProtectionStrategy::ATRBased,
                target_strategy: TargetStrategy::ResistanceBased,
                confidence_assessment: 71.7,
                stop_loss_distance_pct: 2.0,
                cascade_risk_score: 30.0,
                environment_favorability: RiskDimension::default(),
                final_recommendation: "Test reco".into(),
            }),
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
                contributing_indicators: vec!["ema_stack".into(), "macd".into()],
            }),
            risk_profile: None,
            liquidity: None,
            cluster: None,
            volume_profile: None,
            liquidity_signals: vec![],
            metrics_config: None,
            opportunity: None,
            quality_envelope: None,
        }
    }
}
