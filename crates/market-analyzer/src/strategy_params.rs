//! # Strategy → runtime-params conversions (v9)
//!
//! `config-models::StrategyConfig` is the persisted single source of
//! truth; `core-domain::DecisionParams` and `synthesis::OpportunityParams`
//! are the runtime structs the hot path reads. This module converts the
//! strategy's `l6` section into the shared `DecisionParams` (the seed the
//! F-05 unification created). Defaults reproduce the pre-v9 constants.

/// Build the shared L6 `DecisionParams` from the strategy's `l6` section.
pub fn decision_params_from_strategy(
    l6: &config_models::L6Params,
) -> core_domain::decision_params::DecisionParams {
    use core_domain::decision_params::DecisionParams;
    let mut p = DecisionParams::default();
    p.confluence_weights = l6.synthesis.confluence_weights;
    p.risk_discount_k = l6.synthesis.risk_discount_k;
    p.opportunity_fallback = l6.synthesis.opportunity_fallback;
    p.stance_risk_avoid = l6.stance.risk.avoid;
    p.stance_risk_cautious = l6.stance.risk.cautious;
    p.stance_risk_neutral = l6.stance.risk.neutral;
    p.stance_risk_constructive = l6.stance.risk.constructive;
    p.stance_risk_aggressive = l6.stance.risk.aggressive;
    p.direction_risk_strong = l6.direction.risk_strong;
    p.direction_risk_plain = l6.direction.risk_plain;
    p.entry_vol_no_entry = l6.entry.vol_risk_no_entry;
    p.entry_vol_immediate = l6.entry.vol_risk_immediate;
    p.entry_vol_breakout = l6.entry.vol_risk_breakout;
    p.exit_risk_increasing = l6.exit.risk_increasing;
    p.exit_trend_weakening = l6.exit.trend_weakening;
    p.protection_vol_risk = l6.protection.vol_risk;
    p.sr_proximity_atr_mult = l6.protection.sr_proximity_atr_mult;
    p.target_rr_based = l6.target.rr_based;
    p.target_trailing = l6.target.trailing;
    p.stop_base_mult_strong = l6.stop.base_multiplier.strong;
    p.stop_base_mult_weak = l6.stop.base_multiplier.weak;
    p.stop_base_pct = l6.stop.base_pct;
    p.stop_base_clamp_min = l6.stop.base_clamp[0];
    p.stop_base_clamp_max = l6.stop.base_clamp[1];
    p.stop_vol_bump_scale = l6.stop.vol_bump_scale;
    p.stop_final_clamp_min = l6.stop.final_clamp[0];
    p.stop_final_clamp_max = l6.stop.final_clamp[1];
    if let Some(q) = l6.entry_danger.quality_penalties.get("Excellent") {
        p.quality_penalties[0] = *q;
    }
    if let Some(q) = l6.entry_danger.quality_penalties.get("Good") {
        p.quality_penalties[1] = *q;
    }
    if let Some(q) = l6.entry_danger.quality_penalties.get("Average") {
        p.quality_penalties[2] = *q;
    }
    if let Some(q) = l6.entry_danger.quality_penalties.get("Weak") {
        p.quality_penalties[3] = *q;
    }
    if let Some(q) = l6.entry_danger.quality_penalties.get("Poor") {
        p.quality_penalties[4] = *q;
    }
    p.entry_danger_quality_weight = l6.entry_danger.blend[0];
    p.readiness_aside_max = l6.readiness.aside_max;
    p.readiness_ready_min = l6.readiness.ready_min;
    let prob = &l6.probability;
    p.prob_guidance_amp = prob.guidance_amp;
    p.prob_guidance_atten = prob.guidance_atten;
    p.prob_stance_amp = prob.stance_amp;
    p.prob_avoid_atten = prob.avoid_atten;
    p.prob_avoid_hold_amp = prob.avoid_hold_amp;
    p.prob_rr_penalty = prob.rr_penalty;
    p.prob_min_pct = prob.min_pct;
    p.prob_hold_cap = prob.hold_cap;
    p.prob_arm_floor = prob.arm_floor;
    p.prob_geometric_offset = prob.geometric_offset;
    p.prob_eff_conf_floor = prob.eff_conf_floor;
    p.prob_hold_scale = prob.hold_scale;
    p.prob_contributing_conf_min = prob.contributing_conf_min;
    p.risk_ceiling_max_overall_risk = l6.risk_ceiling.max_overall_risk;
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_strategy_l6_round_trips_to_default_decision_params() {
        let l6 = config_models::L6Params::default();
        let p = decision_params_from_strategy(&l6);
        assert_eq!(p, core_domain::decision_params::DecisionParams::default());
    }

    #[test]
    fn custom_stop_and_ceiling_flow_through() {
        let mut l6 = config_models::L6Params::default();
        l6.stop.base_pct = 3.5;
        l6.stop.final_clamp = [1.0, 10.0];
        l6.risk_ceiling.max_overall_risk = Some(60.0);
        l6.stance.risk.avoid = 75.0;
        let p = decision_params_from_strategy(&l6);
        assert_eq!(p.stop_base_pct, 3.5);
        assert_eq!(p.stop_final_clamp_max, 10.0);
        assert_eq!(p.risk_ceiling_max_overall_risk, Some(60.0));
        assert_eq!(p.stance_risk_avoid, 75.0);
    }

    #[test]
    fn quality_penalties_map_in_order() {
        let mut l6 = config_models::L6Params::default();
        l6.entry_danger
            .quality_penalties
            .insert("Poor".into(), 90.0);
        let p = decision_params_from_strategy(&l6);
        assert_eq!(p.quality_penalty(4), 90.0);
        assert_eq!(p.quality_penalty(0), 10.0);
    }
}
