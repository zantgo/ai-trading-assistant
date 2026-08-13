# ANALYSIS — front end vs export JSON (checklist)

Source: audits/2026-08-13-2/exports/analysis.json
Docs: docs/matrices/02-02-analysis-matrix.md §2 + §3

| # | FRONT-END VALUE | EXPORT JSON PATH | DOCS SECTION | STATUS |
|---|---|---|---|---|
| 1 | 'analysis' | `source_tab` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | EXPORT_ONLY |
| 2 | '2026-08-13T13:51:27.005Z' | `meta.datetime_utc` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | EXPORT_ONLY |
| 3 | 'Hyperliquid' | `meta.exchange` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 4 | 'ETH-USDC' | `meta.pair` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 5 | 60 | `meta.timeframe_secs` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 6 | 1890.3 | `meta.current_price` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 7 | 1900.8 | `meta.prev_day_price` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 8 | -0.55239898989899 | `meta.price_change` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 9 | 'down' | `meta.price_change_direction` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 10 | 1786629060 | `meta.timestamp` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | EXPORT_ONLY |
| 11 | False | `meta.is_completed` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | EXPORT_ONLY |
| 12 | 'Analysis' | `header.layer_name` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 13 | 'Neutral' | `header.badge.label` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 14 | '' | `header.badge.sublabel` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 15 | 'warn' | `header.badge.tone` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 16 | 'Quality' | `header.chips[0].label` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 17 | 'Average' | `header.chips[0].value` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 18 | 'Confidence' | `header.chips[1].label` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 19 | 42 | `header.chips[1].value` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 20 | 'Regime' | `header.chips[2].label` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 21 | 'Expansion' | `header.chips[2].value` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 22 | 'live' | `header.status` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | EXPORT_ONLY |
| 23 | 'Neutral' | `body.bias` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 24 | 42 | `body.confidence_pct` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 25 | 0.4186154703276399 | `body.state_confidence` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 26 | 'Expansion' | `body.market_regime` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 27 | 'Average' | `body.market_quality` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 28 | 'UNKNOWN' | `body.cycle_phase` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 29 | 'Net bullish (4↑ vs 0↓)' | `signal_lean_hero.label_html` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 30 | '4:1 signal ratio' | `signal_lean_hero.meta_html` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 31 | 100 | `signal_lean_hero.bullish_pct` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 32 | 0 | `signal_lean_hero.bearish_pct` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 33 | 'bull' | `signal_lean_hero.tone` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 34 | [] | `signals.supporting` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 35 | 'unknown' | `signals.contradicting[0].key` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 36 | None | `signals.contradicting[0].period` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 37 | 'UNKNOWN' | `signals.contradicting[0].display_name` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 38 | 'MICRO' | `signals.contradicting[0].timeframe` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 39 | 21 | `signals.contradicting[0].score` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 40 | '+21' | `signals.contradicting[0].score_display` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 41 | 'EXPANSION' | `signals.contradicting[0].regime` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 42 | 22 | `signals.contradicting[0].signals_count` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 43 | 'MICRO (bullish): score +21, EXPANSION regime, 22 signals' | `signals.contradicting[0].raw` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 44 | 'unknown' | `signals.contradicting[1].key` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 45 | None | `signals.contradicting[1].period` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 46 | 'UNKNOWN' | `signals.contradicting[1].display_name` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 47 | 'FAST' | `signals.contradicting[1].timeframe` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 48 | 18 | `signals.contradicting[1].score` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 49 | '+18' | `signals.contradicting[1].score_display` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 50 | 'TRENDING' | `signals.contradicting[1].regime` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 51 | 26 | `signals.contradicting[1].signals_count` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 52 | 'FAST (bullish): score +18, TRENDING regime, 26 signals' | `signals.contradicting[1].raw` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 53 | 'unknown' | `signals.contradicting[2].key` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 54 | None | `signals.contradicting[2].period` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 55 | 'UNKNOWN' | `signals.contradicting[2].display_name` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 56 | 'SLOW' | `signals.contradicting[2].timeframe` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 57 | 21 | `signals.contradicting[2].score` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 58 | '+21' | `signals.contradicting[2].score_display` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 59 | 'TRENDING' | `signals.contradicting[2].regime` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 60 | 25 | `signals.contradicting[2].signals_count` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 61 | 'SLOW (bullish): score +21, TRENDING regime, 25 signals' | `signals.contradicting[2].raw` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 62 | 'unknown' | `signals.contradicting[3].key` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 63 | None | `signals.contradicting[3].period` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 64 | 'UNKNOWN' | `signals.contradicting[3].display_name` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 65 | 'MACRO' | `signals.contradicting[3].timeframe` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 66 | 19 | `signals.contradicting[3].score` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 67 | '+19' | `signals.contradicting[3].score_display` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 68 | 'TRENDING' | `signals.contradicting[3].regime` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 69 | 30 | `signals.contradicting[3].signals_count` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 70 | 'MACRO (bullish): score +19, TRENDING regime, 30 signals' | `signals.contradicting[3].raw` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 71 | 'unknown' | `signals.list[0].key` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 72 | None | `signals.list[0].period` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 73 | 'UNKNOWN' | `signals.list[0].display_name` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 74 | 'MICRO' | `signals.list[0].timeframe` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 75 | 21 | `signals.list[0].score` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 76 | '+21' | `signals.list[0].score_display` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 77 | 'EXPANSION' | `signals.list[0].regime` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 78 | 22 | `signals.list[0].signals_count` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 79 | 'MICRO (bullish): score +21, EXPANSION regime, 22 signals' | `signals.list[0].raw` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 80 | 'contradicting' | `signals.list[0].bucket` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 81 | 'unknown' | `signals.list[1].key` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 82 | None | `signals.list[1].period` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 83 | 'UNKNOWN' | `signals.list[1].display_name` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 84 | 'FAST' | `signals.list[1].timeframe` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 85 | 18 | `signals.list[1].score` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 86 | '+18' | `signals.list[1].score_display` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 87 | 'TRENDING' | `signals.list[1].regime` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 88 | 26 | `signals.list[1].signals_count` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 89 | 'FAST (bullish): score +18, TRENDING regime, 26 signals' | `signals.list[1].raw` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 90 | 'contradicting' | `signals.list[1].bucket` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 91 | 'unknown' | `signals.list[2].key` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 92 | None | `signals.list[2].period` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 93 | 'UNKNOWN' | `signals.list[2].display_name` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 94 | 'SLOW' | `signals.list[2].timeframe` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 95 | 21 | `signals.list[2].score` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 96 | '+21' | `signals.list[2].score_display` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 97 | 'TRENDING' | `signals.list[2].regime` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 98 | 25 | `signals.list[2].signals_count` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 99 | 'SLOW (bullish): score +21, TRENDING regime, 25 signals' | `signals.list[2].raw` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 100 | 'contradicting' | `signals.list[2].bucket` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 101 | 'unknown' | `signals.list[3].key` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 102 | None | `signals.list[3].period` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 103 | 'UNKNOWN' | `signals.list[3].display_name` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 104 | 'MACRO' | `signals.list[3].timeframe` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 105 | 19 | `signals.list[3].score` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 106 | '+19' | `signals.list[3].score_display` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 107 | 'TRENDING' | `signals.list[3].regime` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 108 | 30 | `signals.list[3].signals_count` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 109 | 'MACRO (bullish): score +19, TRENDING regime, 30 signals' | `signals.list[3].raw` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 110 | 'contradicting' | `signals.list[3].bucket` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 111 | 'Net bullish · 4↑ vs 0↓' | `signals.lean.label` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 112 | 4 | `signals.lean.bullish` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 113 | 0 | `signals.lean.bearish` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 114 | 'bull' | `signals.lean.tone` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 115 | 'Developing' | `qualitative_assessment.trend` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 116 | 'Weakening' | `qualitative_assessment.momentum` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 117 | 'Weak' | `qualitative_assessment.structure` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 118 | 'Normal' | `qualitative_assessment.volatility` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 119 | 'Normal' | `qualitative_assessment.volume` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 120 | 'UNKNOWN' | `qualitative_assessment.cycle_phase` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 121 | 'MICRO' | `per_timeframe_alignment[0].name` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 122 | True | `per_timeframe_alignment[0].active` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 123 | 0.29019269463197006 | `per_timeframe_alignment[0].trend` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 124 | '+0.29' | `per_timeframe_alignment[0].trend_display` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 125 | 0.0819168375833447 | `per_timeframe_alignment[0].momentum` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 126 | '+0.08' | `per_timeframe_alignment[0].momentum_display` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 127 | 21 | `per_timeframe_alignment[0].overall` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 128 | '+21.0' | `per_timeframe_alignment[0].overall_display` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 129 | 'EXPANSION' | `per_timeframe_alignment[0].regime` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 130 | 'FAST' | `per_timeframe_alignment[1].name` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 131 | True | `per_timeframe_alignment[1].active` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 132 | 0.3057083352129776 | `per_timeframe_alignment[1].trend` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 133 | '+0.31' | `per_timeframe_alignment[1].trend_display` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 134 | -0.002457405643544202 | `per_timeframe_alignment[1].momentum` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 135 | '-0.00' | `per_timeframe_alignment[1].momentum_display` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 136 | 18 | `per_timeframe_alignment[1].overall` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 137 | '+18.0' | `per_timeframe_alignment[1].overall_display` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 138 | 'TRENDING' | `per_timeframe_alignment[1].regime` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 139 | 'SLOW' | `per_timeframe_alignment[2].name` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 140 | True | `per_timeframe_alignment[2].active` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 141 | 0.3048466009820931 | `per_timeframe_alignment[2].trend` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 142 | '+0.30' | `per_timeframe_alignment[2].trend_display` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 143 | 0.07294571535407912 | `per_timeframe_alignment[2].momentum` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 144 | '+0.07' | `per_timeframe_alignment[2].momentum_display` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 145 | 21 | `per_timeframe_alignment[2].overall` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 146 | '+21.0' | `per_timeframe_alignment[2].overall_display` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 147 | 'TRENDING' | `per_timeframe_alignment[2].regime` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 148 | 'MACRO' | `per_timeframe_alignment[3].name` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 149 | True | `per_timeframe_alignment[3].active` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 150 | 0.33097543903762594 | `per_timeframe_alignment[3].trend` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 151 | '+0.33' | `per_timeframe_alignment[3].trend_display` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 152 | -0.02349681107830989 | `per_timeframe_alignment[3].momentum` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 153 | '-0.02' | `per_timeframe_alignment[3].momentum_display` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 154 | 19 | `per_timeframe_alignment[3].overall` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 155 | '+19.0' | `per_timeframe_alignment[3].overall_display` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 156 | 'TRENDING' | `per_timeframe_alignment[3].regime` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 157 | 'Expanding market with developing trend, weakening momentum, wea | `interpretation` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 158 | '<strong>Expanding</strong> market with <strong>developing</stro | `interpretation_display` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
| 159 | 'MTF overall score 17/100 → NEUTRAL. Majority of 4 timeframes ag | `rationale` | docs/matrices/02-02-analysis-matrix.md §2 + §3 | RENDERED |
