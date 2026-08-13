"""Write the 7 fresh ETH-USDC export JSONs captured at 2026-08-13T13:51Z
into audits/2026-08-13-2/exports/."""

from pathlib import Path

EXPORTS = Path("audits/2026-08-13-2/exports")
EXPORTS.mkdir(parents=True, exist_ok=True)

RECOMMENDATION = r'''{
  "source_tab": "recommendation",
  "meta": {
    "datetime_utc": "2026-08-13T13:51:17.569Z",
    "exchange": "Hyperliquid",
    "pair": "ETH-USDC",
    "timeframe_secs": 60,
    "current_price": 1890.35,
    "prev_day_price": 1900.8,
    "price_change": -0.5497685185185209,
    "price_change_direction": "down",
    "timestamp": 1786629060,
    "is_completed": false
  },
  "header": {
    "layer_name": "Recommendation",
    "badge": {
      "label": "STAND ASIDE",
      "sublabel": "STAND ASIDE",
      "tone": "warn"
    },
    "chips": [
      {"label": "Confidence", "value": 19},
      {"label": "R:R", "value": "N/A"},
      {"label": "Stance", "value": "Cautious"}
    ],
    "status": "live"
  },
  "gauge": {
    "net_bias_pct": -22,
    "bias_direction": "SHORT",
    "long_pct": 2,
    "short_pct": 24,
    "hold_pct": 74,
    "net_bias_display": "-22%"
  },
  "environment": {
    "directional_guidance": "Neutral",
    "market_stance": "Cautious",
    "strategy_environment": "HighVolatility",
    "opportunity_classification": "Pullback",
    "confidence_pct": 19.308940848254576,
    "readiness": "STAND_ASIDE",
    "entry_danger_score": 45.08179545454546,
    "entry_danger_level": "MODERATE"
  },
  "verdict": {
    "top": "HOLD",
    "long_probability": 2,
    "short_probability": 24,
    "hold_probability": 74
  },
  "top_setup": {
    "opportunity_type": "Pullback",
    "viability": "DirectionalNeutral",
    "badge_text": "HOLD \u00b7 NO DIRECTIONAL EDGE",
    "score": 59.83640909090909,
    "preconditions_met": 2,
    "preconditions_total": 2,
    "direction_label": "NEUTRAL",
    "entry_zone": {"low": 1891.7, "high": 1892.5396304152107},
    "target_zone": {"low": 1887.5018479239466, "high": 1889.1811087543679},
    "invalidation": 1892.625,
    "entry_zone_display": "$1892\u2013$1893",
    "target_zone_display": "$1888\u2013$1889",
    "invalidation_display": "$1893",
    "rr_display": "R:R 1 : 5.8",
    "rr_available": true,
    "rr_value": 5.82,
    "rr_reason": null,
    "rationale": "Pullback: preconditions 2/2"
  },
  "no_clear_card": null,
  "safety_flags": {
    "readiness": "STAND_ASIDE",
    "rr_available": false,
    "rr_value": null,
    "rr_reason": "no_directional_bias",
    "stop_loss_pct": 5.338769933415519,
    "confidence_pct": 19.308940848254576,
    "entry_danger_score": 45.08179545454546,
    "entry_danger_level": "MODERATE",
    "rr_display": "N/A",
    "stop_loss_display": "5.34%",
    "confidence_display": "19%",
    "entry_danger_display": "45 (MODERATE)"
  },
  "why_note": "No directional edge \u2014 these bullets read the same across all three arms (LONG/SHORT/HOLD). They trace the data, not a trade call.",
  "why": [
    "Neutral bias, confluence score 0 (L2 tradability_dim + L3 quality + L4 opportunity)",
    "Setup: Pullback (L4 score 60, Moderate)",
    "Trade readiness = STAND_ASIDE because confidence_assessment 19 < 20"
  ],
  "price_levels": {
    "side": "hold",
    "entry_zone": null,
    "target_zone": null,
    "invalidation": null,
    "horizon": "SWING",
    "hold_placeholder": "No active setup \u2014 verdict is HOLD. Top Setup card above carries the Neutral primary bracket (entry = target = invalidation = close; R:R = 0.00)."
  },
  "strategy": {
    "entry": "Wait For Confirmation",
    "exit": "Trend Weakening",
    "protection": "ATR-Based",
    "target": "Trailing Method"
  },
  "final_verdict": "Neutral \u2014 no directional edge: NEUTRAL bias with 19% confidence, cautious stance in a high-volatility environment. Pullback opportunity. Entry: wait for confirmation. Stop: ATR-based."
}'''

ANALYSIS = r'''{
  "source_tab": "analysis",
  "meta": {
    "datetime_utc": "2026-08-13T13:51:27.005Z",
    "exchange": "Hyperliquid",
    "pair": "ETH-USDC",
    "timeframe_secs": 60,
    "current_price": 1890.3,
    "prev_day_price": 1900.8,
    "price_change": -0.55239898989899,
    "price_change_direction": "down",
    "timestamp": 1786629060,
    "is_completed": false
  },
  "header": {
    "layer_name": "Analysis",
    "badge": {"label": "Neutral", "sublabel": "", "tone": "warn"},
    "chips": [
      {"label": "Quality", "value": "Average"},
      {"label": "Confidence", "value": 42},
      {"label": "Regime", "value": "Expansion"}
    ],
    "status": "live"
  },
  "body": {
    "bias": "Neutral",
    "confidence_pct": 42,
    "state_confidence": 0.4186154703276399,
    "market_regime": "Expansion",
    "market_quality": "Average",
    "cycle_phase": "UNKNOWN"
  },
  "signal_lean_hero": {
    "label_html": "Net bullish (4\u2191 vs 0\u2193)",
    "meta_html": "4:1 signal ratio",
    "bullish_pct": 100,
    "bearish_pct": 0,
    "tone": "bull"
  },
  "signals": {
    "supporting": [],
    "contradicting": [
      {"key": "unknown", "period": null, "display_name": "UNKNOWN", "timeframe": "MICRO", "score": 21, "score_display": "+21", "regime": "EXPANSION", "signals_count": 22, "raw": "MICRO (bullish): score +21, EXPANSION regime, 22 signals"},
      {"key": "unknown", "period": null, "display_name": "UNKNOWN", "timeframe": "FAST", "score": 18, "score_display": "+18", "regime": "TRENDING", "signals_count": 26, "raw": "FAST (bullish): score +18, TRENDING regime, 26 signals"},
      {"key": "unknown", "period": null, "display_name": "UNKNOWN", "timeframe": "SLOW", "score": 21, "score_display": "+21", "regime": "TRENDING", "signals_count": 25, "raw": "SLOW (bullish): score +21, TRENDING regime, 25 signals"},
      {"key": "unknown", "period": null, "display_name": "UNKNOWN", "timeframe": "MACRO", "score": 19, "score_display": "+19", "regime": "TRENDING", "signals_count": 30, "raw": "MACRO (bullish): score +19, TRENDING regime, 30 signals"}
    ],
    "list": [
      {"key": "unknown", "period": null, "display_name": "UNKNOWN", "timeframe": "MICRO", "score": 21, "score_display": "+21", "regime": "EXPANSION", "signals_count": 22, "raw": "MICRO (bullish): score +21, EXPANSION regime, 22 signals", "bucket": "contradicting"},
      {"key": "unknown", "period": null, "display_name": "UNKNOWN", "timeframe": "FAST", "score": 18, "score_display": "+18", "regime": "TRENDING", "signals_count": 26, "raw": "FAST (bullish): score +18, TRENDING regime, 26 signals", "bucket": "contradicting"},
      {"key": "unknown", "period": null, "display_name": "UNKNOWN", "timeframe": "SLOW", "score": 21, "score_display": "+21", "regime": "TRENDING", "signals_count": 25, "raw": "SLOW (bullish): score +21, TRENDING regime, 25 signals", "bucket": "contradicting"},
      {"key": "unknown", "period": null, "display_name": "UNKNOWN", "timeframe": "MACRO", "score": 19, "score_display": "+19", "regime": "TRENDING", "signals_count": 30, "raw": "MACRO (bullish): score +19, TRENDING regime, 30 signals", "bucket": "contradicting"}
    ],
    "lean": {"label": "Net bullish \u00b7 4\u2191 vs 0\u2193", "bullish": 4, "bearish": 0, "tone": "bull"}
  },
  "qualitative_assessment": {
    "trend": "Developing",
    "momentum": "Weakening",
    "structure": "Weak",
    "volatility": "Normal",
    "volume": "Normal",
    "cycle_phase": "UNKNOWN"
  },
  "per_timeframe_alignment": [
    {"name": "MICRO", "active": true, "trend": 0.29019269463197006, "trend_display": "+0.29", "momentum": 0.0819168375833447, "momentum_display": "+0.08", "overall": 21, "overall_display": "+21.0", "regime": "EXPANSION"},
    {"name": "FAST", "active": true, "trend": 0.3057083352129776, "trend_display": "+0.31", "momentum": -0.002457405643544202, "momentum_display": "-0.00", "overall": 18, "overall_display": "+18.0", "regime": "TRENDING"},
    {"name": "SLOW", "active": true, "trend": 0.3048466009820931, "trend_display": "+0.30", "momentum": 0.07294571535407912, "momentum_display": "+0.07", "overall": 21, "overall_display": "+21.0", "regime": "TRENDING"},
    {"name": "MACRO", "active": true, "trend": 0.33097543903762594, "trend_display": "+0.33", "momentum": -0.02349681107830989, "momentum_display": "-0.02", "overall": 19, "overall_display": "+19.0", "regime": "TRENDING"}
  ],
  "interpretation": "Expanding market with developing trend, weakening momentum, weak structure, normal volatility, and normal volume participation. Pullback opportunity forming.",
  "interpretation_display": "<strong>Expanding</strong> market with <strong>developing</strong> trend, <strong>weakening</strong> momentum, <strong>weak</strong> structure, <strong>normal</strong> volatility, and <strong>normal</strong> volume participation. Pullback opportunity forming.",
  "rationale": "MTF overall score 17/100 \u2192 NEUTRAL. Majority of 4 timeframes agree (100%). BBWP=87 ADX=31. Regime: EXPANSION 31 signals across multiple timeframes."
}'''

RISK = r'''{
  "source_tab": "risk",
  "meta": {
    "datetime_utc": "2026-08-13T13:51:44.488Z",
    "exchange": "Hyperliquid",
    "pair": "ETH-USDC",
    "timeframe_secs": 60,
    "current_price": 1890.9,
    "prev_day_price": 1900.8,
    "price_change": -0.5208333333333262,
    "price_change_direction": "down",
    "timestamp": 1786629060,
    "is_completed": false
  },
  "header": {
    "layer_name": "Risk",
    "badge": {"label": "Moderate", "sublabel": "Stable", "tone": "warn"},
    "chips": [
      {"label": "Score", "value": 53.87},
      {"label": "Dimensions", "value": "8/8"}
    ],
    "status": "live"
  },
  "hero": {
    "overall_score": 54,
    "overall_level": "Moderate",
    "overall_state": "Stable",
    "overall_confidence": 42,
    "top_severity": "Extreme",
    "hint": "Lower is safer. State modifiers adjust each dimension's contribution but not the headline score."
  },
  "summary_counts": {
    "very_low": {"label": "Very Low", "count": 0},
    "low": {"label": "Low", "count": 2},
    "moderate": {"label": "Moderate", "count": 2},
    "high": {"label": "High", "count": 2},
    "extreme": {"label": "Extreme", "count": 2}
  },
  "dimensions": [
    {"name": "Signal Risk", "key": "signal_risk", "weight": 0.1, "weight_pct": 10, "score": 85, "not_active": false, "level": "Extreme", "state": "Stable", "state_display": "\u2192 STABLE", "confidence": 42, "evidence": ["4 contradicting signals", "Low analysis confidence"], "no_evidence_text": null, "not_active_text": null, "bar_pct": 85, "weight_mark_pct": 10, "is_cascade_dim": false, "cascade_extras": null},
    {"name": "Execution Liquidity Risk", "key": "execution_liquidity_risk", "weight": 0.14, "weight_pct": 14, "score": 80, "not_active": false, "level": "Extreme", "state": "Stable", "state_display": "\u2192 STABLE", "confidence": 42, "evidence": ["Very low relative volume", "Wide spread"], "no_evidence_text": null, "not_active_text": null, "bar_pct": 80, "weight_mark_pct": 14, "is_cascade_dim": false, "cascade_extras": null},
    {"name": "Execution Risk", "key": "execution_risk", "weight": 0.1, "weight_pct": 10, "score": 65, "not_active": false, "level": "High", "state": "Stable", "state_display": "\u2192 STABLE", "confidence": 42, "evidence": ["Wide spread", "Low participation"], "no_evidence_text": null, "not_active_text": null, "bar_pct": 65, "weight_mark_pct": 10, "is_cascade_dim": false, "cascade_extras": null},
    {"name": "Market Risk", "key": "market_risk", "weight": 0.14, "weight_pct": 14, "score": 60, "not_active": false, "level": "High", "state": "Stable", "state_display": "\u2192 STABLE", "confidence": 42, "evidence": ["Conflicting signals"], "no_evidence_text": null, "not_active_text": null, "bar_pct": 60, "weight_mark_pct": 14, "is_cascade_dim": false, "cascade_extras": null},
    {"name": "Structure Risk", "key": "structure_risk", "weight": 0.1, "weight_pct": 10, "score": 55, "not_active": false, "level": "Moderate", "state": "Stable", "state_display": "\u2192 STABLE", "confidence": 42, "evidence": ["Weak structure"], "no_evidence_text": null, "not_active_text": null, "bar_pct": 55, "weight_mark_pct": 10, "is_cascade_dim": false, "cascade_extras": null},
    {"name": "Momentum Risk", "key": "momentum_risk", "weight": 0.14, "weight_pct": 14, "score": 45, "not_active": false, "level": "Moderate", "state": "Stable", "state_display": "\u2192 STABLE", "confidence": 42, "evidence": ["Momentum weakening"], "no_evidence_text": null, "not_active_text": null, "bar_pct": 45, "weight_mark_pct": 14, "is_cascade_dim": false, "cascade_extras": null},
    {"name": "Cascade Risk", "key": "cascade_risk", "weight": 0.14, "weight_pct": 14, "score": 30, "not_active": false, "level": "Low", "state": "Stable", "state_display": "\u2192 STABLE", "confidence": 42, "evidence": [], "no_evidence_text": null, "not_active_text": null, "bar_pct": 30, "weight_mark_pct": 14, "is_cascade_dim": true, "cascade_extras": {"state_label": "\u2014", "intensity_display": "0.0", "asymmetry_sign": "+", "asymmetry_magnitude_pct": 26.615976331360635, "asymmetry_description": "short squeeze", "asymmetry_display": "\u219126.6% (short squeeze)"}},
    {"name": "Volatility Risk", "key": "volatility_risk", "weight": 0.14, "weight_pct": 14, "score": 23, "not_active": false, "level": "Low", "state": "Stable", "state_display": "\u2192 STABLE", "confidence": 42, "evidence": ["BBWP elevated"], "no_evidence_text": null, "not_active_text": null, "bar_pct": 23.387699334155194, "weight_mark_pct": 14, "is_cascade_dim": false, "cascade_extras": null}
  ],
  "headline_parts": {"very_low_count": 0, "low_count": 2, "moderate_count": 2, "high_count": 2, "extreme_count": 2, "overall_level": "Moderate"},
  "interpretation_headline": "2 extreme \u00b7 2 high \u00b7 2 moderate \u00b7 overall moderate",
  "interpretation_full": "<strong>Elevated risk environment.</strong> 2 dimensions at extreme levels. 2 dimensions at high levels. Consider reduced position sizing and wider stops. Monitor the highest-severity dimensions for evidence of improvement before committing capital. Overall composite score is <strong>moderate</strong> at 42% confidence.",
  "disclosure": {
    "weights": [
      {"label": "Market", "pct": 14},
      {"label": "Volatility", "pct": 14},
      {"label": "ExecLiq", "pct": 14},
      {"label": "Structure", "pct": 10},
      {"label": "Momentum", "pct": 14},
      {"label": "Signal", "pct": 10},
      {"label": "Execution", "pct": 10},
      {"label": "Cascade", "pct": 14}
    ],
    "note": "Overall risk is a weighted sum of the 8 dimension scores. State and confidence modify each dimension's contribution, but do not alter the headline score directly."
  },
  "awaiting_dimensions_text": "Awaiting risk assessment \u2014 this dimension will populate once market data stabilizes."
}'''

OPPORTUNITY = r'''{
  "source_tab": "opportunity",
  "meta": {
    "datetime_utc": "2026-08-13T13:52:10.005Z",
    "exchange": "Hyperliquid",
    "pair": "ETH-USDC",
    "timeframe_secs": 60,
    "current_price": 1890.3,
    "prev_day_price": 1900.8,
    "price_change": -0.55239898989899,
    "price_change_direction": "down",
    "timestamp": 1786629120,
    "is_completed": false
  },
  "header": {
    "layer_name": "Opportunity",
    "badge": {"label": "Pullback", "sublabel": "Moderate", "tone": "neutral"},
    "chips": [
      {"label": "Score", "value": 58.9},
      {"label": "R:R", "value": "1:4.48"},
      {"label": "Horizon", "value": "SWING"}
    ],
    "status": "live"
  },
  "directional_bars": {"bullish_pct": 1, "bearish_pct": 58, "hold_pct": 41, "sort": "desc"},
  "header_block": {"opportunity_class": "Pullback", "lean": "Lean: neutral", "setup_score": 58.90257142857143, "setup_quality": "MODERATE"},
  "trade_setups": [
    {
      "opportunity_type": "Pullback",
      "viability": "DirectionalNeutral",
      "badge_text": "NEUTRAL \u00b7 HOLD",
      "side": "NEUTRAL",
      "rank_idx": 0,
      "is_top": false,
      "geometry_consistent": true,
      "entry_mid": 1891.3171731799248,
      "entry_zone": {"low": 1890.9, "high": 1891.7343463598493},
      "tp1": 1888.3969609204523,
      "tp2": 1886.7282682007537,
      "invalidation": 1892.155,
      "rr_available": true,
      "rr_value": 3.49,
      "rr_reason": null,
      "score": 58.90257142857143,
      "preconditions_met": 2,
      "preconditions_total": 2,
      "notes": "Pullback"
    }
  ],
  "no_clear_strip": {"badge": "NO CLEAR OPPORTUNITY", "preconditions_met": 0, "preconditions_total": 1, "meta": "0/1 preconditions met \u00b7 informational only"},
  "hold_scenario_note": "HOLD / NO CLEAR \u2014 No directional call. The cards below show each qualifying profile's aggregated bracket \u2014 when geometry is inverted (entry/target/SL on the wrong side of close, or zero-bound contamination), R:R reads N/A and the bracket is non-actionable. None are active.",
  "rr_internal": {"expected_rr_available": false, "expected_rr_value": null, "expected_rr_reason": "no_directional_bias", "time_horizon": "SWING"},
  "invalidation_note": "Close below 1876.8 invalidates the Pullback thesis.",
  "evaluated_setups": [
    {"opportunity_type": "Liquidity Squeeze", "viability": "NoClear", "score": 58.90257142857143, "preconditions_met": 0, "preconditions_total": 3, "trade_viability": null, "notes": "Liquidity Squeeze"},
    {"opportunity_type": "Scalp", "viability": "NoClear", "score": 58.90257142857143, "preconditions_met": 0, "preconditions_total": 3, "trade_viability": null, "notes": "Scalp"},
    {"opportunity_type": "Trend Continuation", "viability": "NoClear", "score": 58.90257142857143, "preconditions_met": 0, "preconditions_total": 3, "trade_viability": null, "notes": "Trend Continuation"},
    {"opportunity_type": "Breakout", "viability": "NoClear", "score": 58.90257142857143, "preconditions_met": 0, "preconditions_total": 2, "trade_viability": null, "notes": "Breakout"},
    {"opportunity_type": "Reversal", "viability": "NoClear", "score": 58.90257142857143, "preconditions_met": 0, "preconditions_total": 3, "trade_viability": null, "notes": "Reversal"},
    {"opportunity_type": "Pullback", "viability": "DIRECTIONAL_NEUTRAL", "score": 58.90257142857143, "preconditions_met": 2, "preconditions_total": 2, "trade_viability": "DIRECTIONAL_NEUTRAL", "notes": "Pullback"},
    {"opportunity_type": "Mean Reversion", "viability": "NoClear", "score": 58.90257142857143, "preconditions_met": 0, "preconditions_total": 2, "trade_viability": null, "notes": "Mean Reversion"}
  ],
  "confluent_entry_levels": [{"price": 1892.155, "sources": ["VP"], "strength": 30}],
  "confluent_target_levels": [{"price": 1877.585, "sources": ["VP"], "strength": 30}],
  "market_position": {"bias": "Neutral", "regime": "Accumulation", "trend": "Developing", "quality": "Average"},
  "environment": {"timeframes_considered": 4, "timeframes_considered_display": "4/4 TFs considered", "confidence_pct": 42, "confidence_display": "42%"}
}'''

ALIGNMENT = r'''{
  "source_tab": "alignment",
  "meta": {
    "datetime_utc": "2026-08-13T13:52:20.823Z",
    "exchange": "Hyperliquid",
    "pair": "ETH-USDC",
    "timeframe_secs": 60,
    "current_price": 1890.55,
    "prev_day_price": 1900.8,
    "price_change": -0.539246632996633,
    "price_change_direction": "down",
    "timestamp": 1786629120,
    "is_completed": false
  },
  "header": {
    "layer_name": "Alignment",
    "badge": {"label": "NEUTRAL MTF", "sublabel": "", "tone": "warn"},
    "chips": [
      {"label": "Score", "value": 17.33},
      {"label": "Agreement", "value": 100},
      {"label": "TFs", "value": "4/4"}
    ],
    "status": "live"
  },
  "hero": {
    "mtf_overall_score": 17.32600403148872,
    "mtf_overall_label": "NEUTRAL_MTF",
    "mtf_overall_label_display": "NEUTRAL",
    "timeframes_present": 4,
    "signal_cross_tf_count": 33,
    "trend_agreement_pct": 100
  },
  "breakdown_meta": "T:0.30 M:0.01 Vt:0.05 Vm:0.15",
  "dimensions": [
    {"name": "Trend", "score": 65, "state": "BULLISH", "confidence": 30},
    {"name": "Momentum", "score": 50, "state": "NEUTRAL", "confidence": 1},
    {"name": "Volume", "score": 53, "state": "NEUTRAL", "confidence": 5},
    {"name": "Volatility", "score": 58, "state": "NEUTRAL", "confidence": 15},
    {"name": "Structure", "score": 50, "state": "NEUTRAL", "confidence": 50},
    {"name": "Signal", "score": 100, "state": "STRONGBULLISH", "confidence": 100},
    {"name": "Regime", "score": 100, "state": "STRONGBULLISH", "confidence": 100},
    {"name": "Confidence", "score": 100, "state": "STRONGBULLISH", "confidence": 100},
    {"name": "Liquidity", "score": 11, "state": "STRONGBEARISH", "confidence": 11},
    {"name": "Tradability", "score": 100, "state": "STRONGBULLISH", "confidence": 100}
  ],
  "consensus": {
    "trend_agreement_pct": 100,
    "label": "strong_consensus",
    "label_display": "Strong consensus \u2014 timeframes aligned",
    "polarization": [
      {"key": "T", "label": "Trend", "value": 0.3002183083468573, "value_display": "+0.30"},
      {"key": "M", "label": "Momentum", "value": 0.007587513247807828, "value_display": "+0.01"},
      {"key": "Vt", "label": "Volume", "value": 0.05490016782500791, "value_display": "+0.05"},
      {"key": "Vm", "label": "Volatility", "value": 0.15384615384615383, "value_display": "+0.15"}
    ]
  },
  "per_timeframe": [
    {"timeframe": "MICRO", "trend_score": 0.13322880696816772, "trend_score_display": "0.13", "momentum_score": 0.06412371692596296, "momentum_score_display": "0.06", "overall_score": 11, "overall_score_display": "11.0", "regime": "TRENDING", "active_signals": 28},
    {"timeframe": "FAST", "trend_score": 0.3057083352129776, "trend_score_display": "0.31", "momentum_score": -0.002457405643544202, "momentum_score_display": "-0.00", "overall_score": 18, "overall_score_display": "18.0", "regime": "TRENDING", "active_signals": 26},
    {"timeframe": "SLOW", "trend_score": 0.3048466009820931, "trend_score_display": "0.30", "momentum_score": 0.07294571535407912, "momentum_score_display": "+0.07", "overall_score": 21, "overall_score_display": "21.0", "regime": "TRENDING", "active_signals": 25},
    {"timeframe": "MACRO", "trend_score": 0.33097543903762594, "trend_score_display": "0.33", "momentum_score": -0.02349681107830989, "momentum_score_display": "-0.02", "overall_score": 19, "overall_score_display": "19.0", "regime": "TRENDING", "active_signals": 30}
  ],
  "score_calculation": {
    "weights": [
      {"key": "T", "label": "Trend", "pct": 50, "color": "#22c55e", "value": 0.3002183083468573, "value_display": "+0.30", "contribution": 0.15010915417342865, "contribution_display": "+0.15"},
      {"key": "M", "label": "Momentum", "pct": 30, "color": "#3b82f6", "value": 0.007587513247807828, "value_display": "+0.01", "contribution": 0.0022762539743423483, "contribution_display": "+0.00"},
      {"key": "Vt", "label": "Vol.trend", "pct": 10, "color": "#a78bfa", "value": 0.05490016782500791, "value_display": "+0.05", "contribution": 0.005490016782500791, "contribution_display": "+0.01"},
      {"key": "Vm", "label": "Vol.market", "pct": 10, "color": "#f59e0b", "value": 0.15384615384615383, "value_display": "+0.15", "contribution": 0.015384615384615384, "contribution_display": "+0.02"}
    ],
    "formula": "0.5 * (0.30) + 0.3 * (0.01) + 0.1 * (0.05) + 0.1 * (0.15) = 17.3"
  },
  "interpretation": "Multi-timeframe alignment shows <strong>strong directional consensus</strong> (100% agreement across 4/4 timeframes). The composite score of 17.3 is classified as <strong>NEUTRAL</strong>. 33 cross-timeframe signals reinforce the current bias.",
  "consensus_conflict_banner": ""
}'''

PAIRS = [
    ("recommendation.json", RECOMMENDATION),
    ("analysis.json", ANALYSIS),
    ("risk.json", RISK),
    ("opportunity.json", OPPORTUNITY),
    ("alignment.json", ALIGNMENT),
]

for name, body in PAIRS:
    (EXPORTS / name).write_text(body, encoding="utf-8")
    print(f"wrote {EXPORTS/name}  ({len(body)} bytes)")

print("done")
