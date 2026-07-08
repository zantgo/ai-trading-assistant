//! Decision Context — read-only quantitative decision-support layer.
//!
//! Reads the existing 51-indicator normalized map and produces structured
//! probability, consensus, expected-range, forward-looking volatility, risk,
//! quality, and trade-readiness metrics without introducing any new indicators
//! or state.
//!
//! All computations are deterministic, reproducible, and explainable — designed
//! for AI consumption and institutional decision support.

use std::collections::HashMap;

/// A single snapshot's quantitative decision-support metrics.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DecisionContext {
    // ── Directional ──
    pub bullish_probability: f64,
    pub bearish_probability: f64,
    pub directional_bias: f64,
    pub consensus: f64,

    // ── Expected Range ──
    pub expected_range_1bar: f64,
    pub expected_range_5bar: f64,
    pub expected_range_20bar: f64,
    pub expected_volatility: f64,

    // ── Confluence ──
    pub confluence: f64,

    // ── Risk & Reward ──
    pub risk_level: f64,
    pub reward_risk_ratio: f64,
    pub recommended_stop: f64,

    // ── Quality ──
    pub trade_quality: f64,
    pub market_quality: f64,

    // ── Regime & Trend ──
    pub regime_confidence: f64,
    pub trend_persistence: f64,

    // ── Synthesis ──
    pub trade_readiness: f64,
}

impl DecisionContext {
    pub fn compute(
        map: &HashMap<String, crate::indicators::NormalizedIndicatorValue>,
        price: f64,
        atr_value: f64,
        confluence: f64,
    ) -> Self {
        use crate::indicators::registry::INDICATORS;

        // ── helper: read raw from map ──
        let raw = |k: &str| map.get(k).map(|v| v.raw_value).unwrap_or(0.0);
        let norm = |k: &str| map.get(k).map(|v| v.normalized).unwrap_or(0.0);
        let label = |k: &str| -> &str {
            map.get(k).map(|v| v.state_label.as_str()).unwrap_or("")
        };
        let val = |k: &str, sub: &str| -> Option<f64> {
            map.get(k).and_then(|v| v.values.as_ref()).and_then(|vals| vals.get(sub).copied())
        };
        let _dir = (norm("ema_stack") + norm("supertrend")).signum();

        // ── Bullish / Bearish probability (weighted vote) ──
        let mut bull_votes = 0.0f64;
        let mut bear_votes = 0.0f64;
        let mut directional_count = 0u32;
        let mut agree_bull = 0u32;
        let mut agree_bear = 0u32;
        let mut neut_count = 0u32;

        for meta in INDICATORS {
            if !meta.directional { continue; }
            directional_count += 1;
            if let Some(v) = map.get(meta.key) {
                let w = meta.default_weight;
                let n = v.normalized;
                let c = v.confidence;
                if n > 0.1 { bull_votes += w * n * c; agree_bull += 1; }
                else if n < -0.1 { bear_votes += w * n.abs() * c; agree_bear += 1; }
                else { neut_count += 1; }
            } else { neut_count += 1; }
        }
        let total = bull_votes + bear_votes;
        let bullish_probability = if total > f64::EPSILON { bull_votes / total } else { 0.5 };
        let bearish_probability = 1.0 - bullish_probability;
        let directional_bias = if total > f64::EPSILON {
            ((bull_votes - bear_votes) / total).clamp(-1.0_f64, 1.0_f64)
        } else { 0.0 };
        let max_agree = agree_bull.max(agree_bear) as f64;
        let consensus = if directional_count > 0 { max_agree / directional_count as f64 } else { 0.5 };
        let bd = directional_bias.signum(); // 1=bull, -1=bear, 0=neutral
        let _ = neut_count;

        // ── Expected range (ATR × regime × √N) ──
        let atr_pct = if price > f64::EPSILON { atr_value / price } else { 0.0 };
        let chop_raw = raw("choppiness");
        let regime_factor = if chop_raw <= 38.2 { 1.3 } else if chop_raw >= 61.8 { 0.6 } else { 1.0 };
        let base_range = atr_pct * regime_factor;

        // ── Expected volatility (forward-looking, coil-aware) ──
        let hv_raw = raw("hv");
        let squeeze_label = label("squeeze");
        let bbwp_raw = raw("bbwp");
        let atr_label = label("atr");
        let coil_factor = if squeeze_label.contains("COILING") { 1.5 }
            else if bbwp_raw > 95.0 { 1.3 } else if bbwp_raw < 10.0 { 0.9 } else { 1.0 };
        let atr_factor = if atr_label.contains("EXPANDING") { 1.2 }
            else if atr_label.contains("CONTRACTING") { 0.8 } else { 1.0 };
        let expected_volatility = hv_raw * coil_factor * atr_factor;

        // ── 1. Risk Level (weighted composite, 6 sub-factors) ──
        let vol_risk = ((hv_raw / 50.0).min(1.0) + ((atr_pct / 0.05).min(1.0))).min(1.0) / 2.0;
        let dis_risk = 1.0 - consensus;
        let rvol_raw = raw("rvol");
        let ex_risk = if rvol_raw >= 3.0 { 1.0 } else if bbwp_raw > 95.0 { 0.8 } else { 0.3 };
        let unc_risk = if chop_raw >= 61.8 { 1.0 } else if chop_raw <= 38.2 { 0.1 } else { 0.5 };
        let adx_raw = raw("adx");
        let trend_instability = if adx_raw < 20.0 { 1.0 } else if adx_raw < 25.0 { 0.7 } else { 0.2 };
        let liq_risk = if squeeze_label.contains("COILING") { 0.9 } else { 0.4 };
        let risk_level = (0.25 * vol_risk + 0.20 * dis_risk + 0.15 * ex_risk + 0.15 * unc_risk
            + 0.15 * trend_instability + 0.10 * liq_risk).clamp(0.0_f64, 1.0_f64);

        // ── 2. Trade Quality (direction-aware, 7 factors + contradiction penalty) ──
        let conf_score = (confluence.abs() / 100.0).min(1.0);
        let prob_score = bullish_probability.max(bearish_probability);
        let trend_score = if adx_raw >= 30.0 { 1.0 } else if adx_raw >= 20.0 { 0.7 } else { 0.3 };
        let vol_score = if rvol_raw >= 1.5 { 1.0 } else if rvol_raw >= 1.0 { 0.7 } else { 0.4 };
        let clean_score = if chop_raw <= 38.2 { 1.0 } else if chop_raw >= 61.8 { 0.2 } else { 0.6 };
        let consensus_score = consensus;
        let obv_dir = norm("obv").signum();
        let fi_dir = norm("force_index").signum();
        let confirm_score = if bd != 0.0 && (obv_dir == bd || fi_dir == bd) { 1.0 }
            else if bd != 0.0 { 0.5 } else { 0.0 };
        let mut trade_quality: f64 = (0.20 * conf_score + 0.20 * prob_score + 0.15 * trend_score
            + 0.10 * vol_score + 0.10 * clean_score + 0.15 * consensus_score + 0.10 * confirm_score)
            .clamp(0.0_f64, 1.0_f64);

        // Contradiction penalty: structural or divergence signals opposing the bias.
        let sweep_bull = val_bool(map, "smc_liquidity", "sweep_buy");
        let sweep_bear = val_bool(map, "smc_liquidity", "sweep_sell");
        let choch_bull = val_bool(map, "smc_structure", "choch_bullish");
        let choch_bear = val_bool(map, "smc_structure", "choch_bearish");
        let macd_div = norm("macd_divergence");
        if (bd > 0.0 && (sweep_bear || choch_bear || macd_div < -0.3))
            || (bd < 0.0 && (sweep_bull || choch_bull || macd_div > 0.3))
        {
            trade_quality *= 0.5;
        }

        // ── 3. Market Quality (regime-agnostic cleanliness, 6 factors) ──
        let trend_q = if adx_raw >= 25.0 { 1.0 } else if adx_raw >= 20.0 { 0.6 } else { 0.2 };
        let clean_q = if chop_raw <= 38.2 { 1.0 } else if chop_raw >= 61.8 { 0.1 } else { 0.5 };
        let vol_q = if bbwp_raw >= 10.0 && bbwp_raw <= 90.0 { 1.0 } else { 0.5 };
        let ema_abs = norm("ema_stack").abs();
        let structure_q = if ema_abs > 0.7 { 1.0 } else if ema_abs > 0.3 { 0.6 } else { 0.3 };
        let aroon_abs = norm("aroon").abs();
        let regime_q = if aroon_abs > 0.5 { 1.0 } else { 0.5 };
        let pat_q = if map.contains_key("candlestick") { 0.3 } else { 0.0 };
        let market_quality: f64 = {
            let raw: f64 = 0.25 * trend_q + 0.25 * clean_q + 0.15 * vol_q
                + 0.15 * structure_q + 0.10 * regime_q + 0.10 * pat_q;
            raw.clamp(0.0, 1.0)
        };

        // ── 4. Regime Confidence (weighted agreement of 6 regime indicators) ──
        let adx_bull = adx_raw > 25.0;
        let chop_trend = chop_raw <= 38.2;
        let ichi_norm = norm("ichimoku");
        let ichi_bull = ichi_norm > 0.3;
        let _ichi_bear = ichi_norm < -0.3;
        let aroon_n = norm("aroon");
        let aroon_bull = aroon_n > 0.5;
        let _aroon_bear = aroon_n < -0.5;
        let st_n = norm("supertrend");
        let st_bull = st_n > 0.6;
        let _st_bear = st_n < -0.6;
        let ema_n = norm("ema_stack");
        let ema_bull = ema_n > 0.7;
        let _ema_bear = ema_n < -0.7;

        let regime_bull_w: f64 = (if adx_bull { 0.25 } else { 0.0 })
            + (if chop_trend && !adx_bull { 0.25 } else if chop_trend { 0.0 } else { 0.25 })
            + (if ichi_bull { 0.20 } else { 0.0 })
            + (if aroon_bull { 0.15 } else { 0.0 })
            + (if st_bull { 0.10 } else { 0.0 })
            + (if ema_bull { 0.05 } else { 0.0 });
        // For bearish, same weights opposite direction. ADX alone doesn't give
        // direction, so we weight the EMA/Ichimoku/Aroon bearish side instead.
        let adx_trend = adx_raw > 25.0;
        let ichi_bear = ichi_norm < -0.3;
        let aroon_bear = aroon_n < -0.5;
        let st_bear = st_n < -0.6;
        let ema_bear = ema_n < -0.7;
        let regime_bear_w: f64 = (if adx_trend && (ema_bear || ichi_bear) { 0.25 } else if adx_trend { 0.0 } else { 0.25 })
            + (if !chop_trend { 0.25 } else { 0.0 })
            + (if ichi_bear { 0.20 } else { 0.0 })
            + (if aroon_bear { 0.15 } else { 0.0 })
            + (if st_bear { 0.10 } else { 0.0 })
            + (if ema_bear { 0.05 } else { 0.0 });
        let regime_confidence: f64 = regime_bull_w.max(regime_bear_w).clamp(0.0_f64, 1.0_f64);
        let _ = (adx_bull, aroon_bull, st_bull, ema_bull, ichi_bull, chop_trend, adx_trend);

        // ── 5. Trend Persistence (9 confirmations) ──
        let adx_rising = val("adx", "adx_slope").unwrap_or(0.0) > 0.0;
        let ema_ok = ema_abs > 0.5;
        let macd_n = norm("macd");
        let _macd_expanding = label("macd").contains("EXPANDING") && macd_n.signum() == bd;
        let macd_ok = macd_n.abs() > 0.3;
        let obv_ok = norm("obv").abs() > 0.2;
        let aroon_ok = aroon_n.abs() > 0.5;
        let st_ok = st_n.abs() > 0.6;
        let vp_label = label("volume_profile");
        let volprof_ok = vp_label.contains("BREAKOUT");
        let rsi_div_present = norm("rsi_divergence").abs() > 0.4;
        let macd_div_present = norm("macd_divergence").abs() > 0.4;
        let no_div = !rsi_div_present && !macd_div_present;
        let no_choch = !choch_bull && !choch_bear;
        let confirmations = [adx_rising, ema_ok, macd_ok, obv_ok, aroon_ok, st_ok, volprof_ok, no_div, no_choch]
            .iter().filter(|x| **x).count();
        let trend_persistence = (confirmations as f64 / 9.0).clamp(0.0, 1.0);

        // ── 6. Reward / Risk (priority-ranked target ÷ stop) ──
        let pivot_r1 = val("pivot_points", "r1");
        let _pivot_s1 = val("pivot_points", "s1");
        let ob_target_up = val("smc_order_blocks", "ob_bearish_high");
        let target_candidates: Vec<f64> = [ob_target_up, pivot_r1]
            .iter().filter_map(|x| *x).filter(|&x| x > price).collect();
        let target_dist = target_candidates.first().map(|t| (t - price) / price).unwrap_or(atr_pct * 3.0);

        // Stop levels: priority-ordered support.
        let ob_stop = val("smc_order_blocks", "ob_bullish_low");
        let swing_stop = val("pivot_points", if bd > 0.0 { "s1" } else { "r1" });
        let vwap_stop = val("vwap", "vwap");
        let vp_stop = if bd > 0.0 { val("volume_profile", "val") } else { val("volume_profile", "vah") };
        let pivot_stop = if bd > 0.0 { val("pivot_points", "s1") } else { val("pivot_points", "r1") };
        let atr_stop = if bd > 0.0 { price - 2.0 * atr_value } else { price + 2.0 * atr_value };
        let stop_candidates: Vec<f64> = [ob_stop, swing_stop, vwap_stop, vp_stop, pivot_stop]
            .iter().filter_map(|x| *x)
            .filter(|&x| if bd > 0.0 { x < price && (price - x) / price > 0.005 }
                         else { x > price && (x - price) / price > 0.005 })
            .collect();
        let _ = (vwap_stop, vp_stop, pivot_stop);

        // Compute best stop: closest-but-safe institutional level, or ATR fallback.
        let best_stop = stop_candidates.iter()
            .reduce(|a, b| if (a - price).abs() < (b - price).abs() { a } else { b })
            .copied();
        let stop_dist = best_stop.map(|s| (price - s).abs() / price).unwrap_or(atr_pct * 2.0);
        let recommended_stop = best_stop.unwrap_or(atr_stop);

        let reward_risk_ratio = if target_dist > 0.0 && stop_dist > 0.0 {
            target_dist / stop_dist
        } else { 0.0 };

        // ── 7. Trade Readiness (synthesis) ──
        let trade_readiness: f64 = (0.30 * trade_quality + 0.25 * (1.0 - risk_level)
            + 0.20 * market_quality + 0.10 * regime_confidence + 0.15 * trend_persistence)
            .clamp(0.0_f64, 1.0_f64);

        DecisionContext {
            bullish_probability, bearish_probability, directional_bias, consensus,
            expected_range_1bar: base_range,
            expected_range_5bar: base_range * (5.0_f64).sqrt(),
            expected_range_20bar: base_range * (20.0_f64).sqrt(),
            expected_volatility, confluence,
            risk_level, reward_risk_ratio, recommended_stop,
            trade_quality, market_quality,
            regime_confidence, trend_persistence,
            trade_readiness,
        }
    }
}

// ── Helpers ──

fn val_bool(map: &HashMap<String, crate::indicators::NormalizedIndicatorValue>, key: &str, sub: &str) -> bool {
    map.get(key)
        .and_then(|v| v.values.as_ref())
        .and_then(|vals| vals.get(sub))
        .map(|&x| x > 0.5)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indicators::NormalizedIndicatorValue;

    fn entry(norm: f64, conf: f64) -> NormalizedIndicatorValue {
        let mut v = NormalizedIndicatorValue::scalar(0.0, norm, "TEST");
        v.confidence = conf;
        v
    }

    fn seed_map(directional_norm: f64, conf: f64) -> HashMap<String, NormalizedIndicatorValue> {
        let mut map = HashMap::new();
        for meta in crate::indicators::registry::INDICATORS {
            if meta.directional {
                map.insert(meta.key.to_string(), entry(directional_norm, conf));
            }
        }
        map
    }

    fn inject(map: &mut HashMap<String, NormalizedIndicatorValue>, key: &str, norm: f64, raw: f64, label: &str) {
        map.insert(key.to_string(), NormalizedIndicatorValue::scalar(raw, norm, label));
    }

    #[test]
    fn test_unanimous_bullish() {
        let map = seed_map(0.85, 0.9);
        let dc = DecisionContext::compute(&map, 50000.0, 500.0, 65.0);
        assert!(dc.bullish_probability > 0.95);
        assert!(dc.bearish_probability < 0.05);
        assert!(dc.directional_bias > 0.9);
        assert!(dc.consensus > 0.9);
    }

    #[test]
    fn test_unanimous_bearish() {
        let map = seed_map(-0.85, 0.9);
        let dc = DecisionContext::compute(&map, 50000.0, 500.0, -65.0);
        assert!(dc.bearish_probability > 0.95);
        assert!(dc.bullish_probability < 0.05);
        assert!(dc.directional_bias < -0.9);
    }

    #[test]
    fn test_split_vote_near_50() {
        let mut map = HashMap::new();
        let mut count = 0;
        for meta in crate::indicators::registry::INDICATORS {
            if meta.directional {
                count += 1;
                let n = if count % 2 == 0 { 0.85 } else { -0.85 };
                map.insert(meta.key.to_string(), entry(n, 0.9));
            }
        }
        let dc = DecisionContext::compute(&map, 50000.0, 500.0, 0.0);
        assert!(dc.bullish_probability > 0.45 && dc.bullish_probability < 0.55);
        assert!(dc.consensus < 0.6);
    }

    #[test]
    fn test_range_scales_with_sqrt_n() {
        let map = seed_map(0.5, 0.7);
        let dc = DecisionContext::compute(&map, 50000.0, 500.0, 0.0);
        assert!(dc.expected_range_1bar > 0.0);
        let r5 = dc.expected_range_5bar / dc.expected_range_1bar;
        let r20 = dc.expected_range_20bar / dc.expected_range_1bar;
        assert!((r5 - (5.0_f64).sqrt()).abs() < 0.01);
        assert!((r20 - (20.0_f64).sqrt()).abs() < 0.01);
    }

    #[test]
    fn test_coil_boosts_expected_volatility() {
        let mut map = seed_map(0.0, 0.5);
        inject(&mut map, "squeeze", 0.0, 0.0, "COMPRESSION_COILING");
        inject(&mut map, "hv", 0.0, 20.0, "HV_NORMAL");
        inject(&mut map, "atr", 0.0, 500.0, "ATR_STABLE");
        inject(&mut map, "bbwp", 0.0, 50.0, "BBWP_NORMAL");
        inject(&mut map, "choppiness", 0.0, 50.0, "CHOP_NORMAL");
        let dc = DecisionContext::compute(&map, 50000.0, 500.0, 0.0);
        assert!(dc.expected_volatility >= 20.0 * 1.5);
    }

    #[test]
    fn test_risk_low_in_calm_trend() {
        let mut map = seed_map(0.5, 0.8);
        inject(&mut map, "adx", 0.8, 35.0, "STRONG_BULL_TREND");
        inject(&mut map, "choppiness", 0.0, 25.0, "CHOP_STRONG_TREND");
        inject(&mut map, "hv", 0.0, 15.0, "HV_NORMAL");
        inject(&mut map, "bbwp", 0.0, 50.0, "BBWP_NORMAL");
        inject(&mut map, "rvol", 0.0, 1.2, "NORMAL_PARTICIPATION");
        inject(&mut map, "atr", 0.0, 500.0, "ATR_STABLE");
        inject(&mut map, "squeeze", 0.0, 0.0, "SQUEEZE_OFF");
        let dc = DecisionContext::compute(&map, 50000.0, 500.0, 50.0);
        assert!(dc.risk_level < 0.3, "calm trend should have low risk, got {}", dc.risk_level);
    }

    #[test]
    fn test_risk_high_in_chaos() {
        let mut map = seed_map(0.1, 0.3);
        inject(&mut map, "adx", 0.1, 12.0, "TRENDLESS_CONGESTION");
        inject(&mut map, "choppiness", 0.0, 75.0, "CHOP_CONSOLIDATION_RANGE");
        inject(&mut map, "hv", 0.0, 60.0, "HV_EXTREME");
        inject(&mut map, "bbwp", 0.0, 97.0, "MAX_VOLATILITY_COMPRESSION");
        inject(&mut map, "rvol", 0.0, 3.5, "EXHAUSTION_CLIMAX");
        inject(&mut map, "atr", 0.0, 1200.0, "ATR_EXPANDING");
        inject(&mut map, "squeeze", 0.0, 0.0, "COMPRESSION_COILING");
        let dc = DecisionContext::compute(&map, 50000.0, 1200.0, 0.0);
        assert!(dc.risk_level > 0.65, "chaotic market should have high risk, got {}", dc.risk_level);
    }

    #[test]
    fn test_contradiction_penalty_reduces_trade_quality() {
        let mut map = seed_map(0.85, 0.9);
        inject(&mut map, "adx", 0.9, 40.0, "STRONG_BULL_TREND");
        inject(&mut map, "choppiness", 0.0, 20.0, "CHOP_STRONG_TREND");
        inject(&mut map, "rvol", 0.0, 2.0, "INSTITUTIONAL_BREAKOUT");
        inject(&mut map, "macd", 0.7, 0.0, "BULLISH_CROSSOVER_ACCELERATING");
        inject(&mut map, "obv", 0.6, 0.0, "OBV_ACCUMULATION");
        inject(&mut map, "force_index", 0.5, 0.0, "FI_BULLISH");
        // inject contradiction: bearish CHoCH
        let mut smc_vals = std::collections::HashMap::new();
        smc_vals.insert("choch_bearish".to_string(), 1.0);
        let mut smc = NormalizedIndicatorValue::scalar(0.0, 0.0, "SMC_STRUCTURE_BEARISH_CHOCH");
        smc.values = Some(smc_vals);
        map.insert("smc_structure".to_string(), smc);
        // No contradiction: let it compute normally first
        let dc_clean = DecisionContext::compute(&map, 50000.0, 500.0, 60.0);
        // Now add sweep sell
        let mut liq_vals = std::collections::HashMap::new();
        liq_vals.insert("sweep_sell".to_string(), 1.0);
        let mut liq = NormalizedIndicatorValue::scalar(0.0, 0.0, "SMC_LIQUIDITY_SELL_SWEEP");
        liq.values = Some(liq_vals);
        map.insert("smc_liquidity".to_string(), liq);
        let dc_contra = DecisionContext::compute(&map, 50000.0, 500.0, 60.0);
        // With structural contradiction, trade quality should be significantly lower
        assert!(dc_contra.trade_quality < dc_clean.trade_quality,
            "contradiction should reduce trade quality (clean={}, contra={})",
            dc_clean.trade_quality, dc_contra.trade_quality);
    }

    #[test]
    fn test_stop_always_returns_value() {
        let map = seed_map(0.5, 0.7);
        let dc = DecisionContext::compute(&map, 50000.0, 500.0, 0.0);
        assert!(dc.recommended_stop > 0.0, "recommended_stop must always return a value");
        assert!(dc.recommended_stop < 50000.0, "stop should be below price");
    }

    #[test]
    fn test_trade_readiness_synthesis() {
        let map = seed_map(0.5, 0.8);
        let dc = DecisionContext::compute(&map, 50000.0, 500.0, 50.0);
        assert!(dc.trade_readiness >= 0.0 && dc.trade_readiness <= 1.0);
        // In a neutral-ish market with some confluence, readiness should be moderate
        assert!(dc.trade_readiness > 0.2);
    }
}
