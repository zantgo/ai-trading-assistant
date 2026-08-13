# RISK — front end vs export JSON (checklist)

Source: audits/2026-08-13-2/exports/risk.json
Docs: docs/matrices/02-11-risk-matrix.md §2 + §3

| # | FRONT-END VALUE | EXPORT JSON PATH | DOCS SECTION | STATUS |
|---|---|---|---|---|
| 1 | 'risk' | `source_tab` | docs/matrices/02-11-risk-matrix.md §2 + §3 | EXPORT_ONLY |
| 2 | '2026-08-13T13:51:44.488Z' | `meta.datetime_utc` | docs/matrices/02-11-risk-matrix.md §2 + §3 | EXPORT_ONLY |
| 3 | 'Hyperliquid' | `meta.exchange` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 4 | 'ETH-USDC' | `meta.pair` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 5 | 60 | `meta.timeframe_secs` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 6 | 1890.9 | `meta.current_price` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 7 | 1900.8 | `meta.prev_day_price` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 8 | -0.5208333333333262 | `meta.price_change` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 9 | 'down' | `meta.price_change_direction` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 10 | 1786629060 | `meta.timestamp` | docs/matrices/02-11-risk-matrix.md §2 + §3 | EXPORT_ONLY |
| 11 | False | `meta.is_completed` | docs/matrices/02-11-risk-matrix.md §2 + §3 | EXPORT_ONLY |
| 12 | 'Risk' | `header.layer_name` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 13 | 'Moderate' | `header.badge.label` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 14 | 'Stable' | `header.badge.sublabel` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 15 | 'warn' | `header.badge.tone` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 16 | 'Score' | `header.chips[0].label` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 17 | 53.87 | `header.chips[0].value` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 18 | 'Dimensions' | `header.chips[1].label` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 19 | '8/8' | `header.chips[1].value` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 20 | 'live' | `header.status` | docs/matrices/02-11-risk-matrix.md §2 + §3 | EXPORT_ONLY |
| 21 | 54 | `hero.overall_score` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 22 | 'Moderate' | `hero.overall_level` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 23 | 'Stable' | `hero.overall_state` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 24 | 42 | `hero.overall_confidence` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 25 | 'Extreme' | `hero.top_severity` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 26 | "Lower is safer. State modifiers adjust each dimension's contrib | `hero.hint` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 27 | 'Very Low' | `summary_counts.very_low.label` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 28 | 0 | `summary_counts.very_low.count` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 29 | 'Low' | `summary_counts.low.label` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 30 | 2 | `summary_counts.low.count` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 31 | 'Moderate' | `summary_counts.moderate.label` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 32 | 2 | `summary_counts.moderate.count` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 33 | 'High' | `summary_counts.high.label` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 34 | 2 | `summary_counts.high.count` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 35 | 'Extreme' | `summary_counts.extreme.label` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 36 | 2 | `summary_counts.extreme.count` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 37 | 'Signal Risk' | `dimensions[0].name` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 38 | 'signal_risk' | `dimensions[0].key` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 39 | 0.1 | `dimensions[0].weight` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 40 | 10 | `dimensions[0].weight_pct` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 41 | 85 | `dimensions[0].score` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 42 | False | `dimensions[0].not_active` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 43 | 'Extreme' | `dimensions[0].level` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 44 | 'Stable' | `dimensions[0].state` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 45 | '→ STABLE' | `dimensions[0].state_display` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 46 | 42 | `dimensions[0].confidence` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 47 | '4 contradicting signals' | `dimensions[0].evidence[0]` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 48 | 'Low analysis confidence' | `dimensions[0].evidence[1]` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 49 | None | `dimensions[0].no_evidence_text` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 50 | None | `dimensions[0].not_active_text` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 51 | 85 | `dimensions[0].bar_pct` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 52 | 10 | `dimensions[0].weight_mark_pct` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 53 | False | `dimensions[0].is_cascade_dim` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 54 | None | `dimensions[0].cascade_extras` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 55 | 'Execution Liquidity Risk' | `dimensions[1].name` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 56 | 'execution_liquidity_risk' | `dimensions[1].key` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 57 | 0.14 | `dimensions[1].weight` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 58 | 14 | `dimensions[1].weight_pct` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 59 | 80 | `dimensions[1].score` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 60 | False | `dimensions[1].not_active` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 61 | 'Extreme' | `dimensions[1].level` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 62 | 'Stable' | `dimensions[1].state` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 63 | '→ STABLE' | `dimensions[1].state_display` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 64 | 42 | `dimensions[1].confidence` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 65 | 'Very low relative volume' | `dimensions[1].evidence[0]` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 66 | 'Wide spread' | `dimensions[1].evidence[1]` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 67 | None | `dimensions[1].no_evidence_text` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 68 | None | `dimensions[1].not_active_text` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 69 | 80 | `dimensions[1].bar_pct` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 70 | 14 | `dimensions[1].weight_mark_pct` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 71 | False | `dimensions[1].is_cascade_dim` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 72 | None | `dimensions[1].cascade_extras` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 73 | 'Execution Risk' | `dimensions[2].name` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 74 | 'execution_risk' | `dimensions[2].key` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 75 | 0.1 | `dimensions[2].weight` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 76 | 10 | `dimensions[2].weight_pct` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 77 | 65 | `dimensions[2].score` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 78 | False | `dimensions[2].not_active` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 79 | 'High' | `dimensions[2].level` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 80 | 'Stable' | `dimensions[2].state` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 81 | '→ STABLE' | `dimensions[2].state_display` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 82 | 42 | `dimensions[2].confidence` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 83 | 'Wide spread' | `dimensions[2].evidence[0]` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 84 | 'Low participation' | `dimensions[2].evidence[1]` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 85 | None | `dimensions[2].no_evidence_text` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 86 | None | `dimensions[2].not_active_text` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 87 | 65 | `dimensions[2].bar_pct` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 88 | 10 | `dimensions[2].weight_mark_pct` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 89 | False | `dimensions[2].is_cascade_dim` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 90 | None | `dimensions[2].cascade_extras` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 91 | 'Market Risk' | `dimensions[3].name` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 92 | 'market_risk' | `dimensions[3].key` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 93 | 0.14 | `dimensions[3].weight` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 94 | 14 | `dimensions[3].weight_pct` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 95 | 60 | `dimensions[3].score` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 96 | False | `dimensions[3].not_active` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 97 | 'High' | `dimensions[3].level` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 98 | 'Stable' | `dimensions[3].state` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 99 | '→ STABLE' | `dimensions[3].state_display` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 100 | 42 | `dimensions[3].confidence` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 101 | 'Conflicting signals' | `dimensions[3].evidence[0]` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 102 | None | `dimensions[3].no_evidence_text` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 103 | None | `dimensions[3].not_active_text` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 104 | 60 | `dimensions[3].bar_pct` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 105 | 14 | `dimensions[3].weight_mark_pct` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 106 | False | `dimensions[3].is_cascade_dim` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 107 | None | `dimensions[3].cascade_extras` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 108 | 'Structure Risk' | `dimensions[4].name` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 109 | 'structure_risk' | `dimensions[4].key` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 110 | 0.1 | `dimensions[4].weight` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 111 | 10 | `dimensions[4].weight_pct` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 112 | 55 | `dimensions[4].score` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 113 | False | `dimensions[4].not_active` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 114 | 'Moderate' | `dimensions[4].level` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 115 | 'Stable' | `dimensions[4].state` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 116 | '→ STABLE' | `dimensions[4].state_display` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 117 | 42 | `dimensions[4].confidence` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 118 | 'Weak structure' | `dimensions[4].evidence[0]` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 119 | None | `dimensions[4].no_evidence_text` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 120 | None | `dimensions[4].not_active_text` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 121 | 55 | `dimensions[4].bar_pct` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 122 | 10 | `dimensions[4].weight_mark_pct` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 123 | False | `dimensions[4].is_cascade_dim` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 124 | None | `dimensions[4].cascade_extras` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 125 | 'Momentum Risk' | `dimensions[5].name` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 126 | 'momentum_risk' | `dimensions[5].key` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 127 | 0.14 | `dimensions[5].weight` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 128 | 14 | `dimensions[5].weight_pct` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 129 | 45 | `dimensions[5].score` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 130 | False | `dimensions[5].not_active` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 131 | 'Moderate' | `dimensions[5].level` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 132 | 'Stable' | `dimensions[5].state` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 133 | '→ STABLE' | `dimensions[5].state_display` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 134 | 42 | `dimensions[5].confidence` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 135 | 'Momentum weakening' | `dimensions[5].evidence[0]` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 136 | None | `dimensions[5].no_evidence_text` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 137 | None | `dimensions[5].not_active_text` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 138 | 45 | `dimensions[5].bar_pct` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 139 | 14 | `dimensions[5].weight_mark_pct` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 140 | False | `dimensions[5].is_cascade_dim` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 141 | None | `dimensions[5].cascade_extras` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 142 | 'Cascade Risk' | `dimensions[6].name` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 143 | 'cascade_risk' | `dimensions[6].key` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 144 | 0.14 | `dimensions[6].weight` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 145 | 14 | `dimensions[6].weight_pct` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 146 | 30 | `dimensions[6].score` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 147 | False | `dimensions[6].not_active` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 148 | 'Low' | `dimensions[6].level` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 149 | 'Stable' | `dimensions[6].state` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 150 | '→ STABLE' | `dimensions[6].state_display` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 151 | 42 | `dimensions[6].confidence` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 152 | [] | `dimensions[6].evidence` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 153 | None | `dimensions[6].no_evidence_text` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 154 | None | `dimensions[6].not_active_text` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 155 | 30 | `dimensions[6].bar_pct` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 156 | 14 | `dimensions[6].weight_mark_pct` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 157 | True | `dimensions[6].is_cascade_dim` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 158 | '—' | `dimensions[6].cascade_extras.state_label` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 159 | '0.0' | `dimensions[6].cascade_extras.intensity_display` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 160 | '+' | `dimensions[6].cascade_extras.asymmetry_sign` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 161 | 26.615976331360635 | `dimensions[6].cascade_extras.asymmetry_magnitude_pct` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 162 | 'short squeeze' | `dimensions[6].cascade_extras.asymmetry_description` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 163 | '↑26.6% (short squeeze)' | `dimensions[6].cascade_extras.asymmetry_display` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 164 | 'Volatility Risk' | `dimensions[7].name` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 165 | 'volatility_risk' | `dimensions[7].key` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 166 | 0.14 | `dimensions[7].weight` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 167 | 14 | `dimensions[7].weight_pct` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 168 | 23 | `dimensions[7].score` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 169 | False | `dimensions[7].not_active` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 170 | 'Low' | `dimensions[7].level` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 171 | 'Stable' | `dimensions[7].state` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 172 | '→ STABLE' | `dimensions[7].state_display` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 173 | 42 | `dimensions[7].confidence` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 174 | 'BBWP elevated' | `dimensions[7].evidence[0]` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 175 | None | `dimensions[7].no_evidence_text` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 176 | None | `dimensions[7].not_active_text` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 177 | 23.387699334155194 | `dimensions[7].bar_pct` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 178 | 14 | `dimensions[7].weight_mark_pct` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 179 | False | `dimensions[7].is_cascade_dim` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 180 | None | `dimensions[7].cascade_extras` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 181 | 0 | `headline_parts.very_low_count` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 182 | 2 | `headline_parts.low_count` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 183 | 2 | `headline_parts.moderate_count` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 184 | 2 | `headline_parts.high_count` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 185 | 2 | `headline_parts.extreme_count` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 186 | 'Moderate' | `headline_parts.overall_level` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 187 | '2 extreme · 2 high · 2 moderate · overall moderate' | `interpretation_headline` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 188 | '<strong>Elevated risk environment.</strong> 2 dimensions at ext | `interpretation_full` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 189 | 'Market' | `disclosure.weights[0].label` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 190 | 14 | `disclosure.weights[0].pct` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 191 | 'Volatility' | `disclosure.weights[1].label` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 192 | 14 | `disclosure.weights[1].pct` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 193 | 'ExecLiq' | `disclosure.weights[2].label` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 194 | 14 | `disclosure.weights[2].pct` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 195 | 'Structure' | `disclosure.weights[3].label` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 196 | 10 | `disclosure.weights[3].pct` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 197 | 'Momentum' | `disclosure.weights[4].label` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 198 | 14 | `disclosure.weights[4].pct` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 199 | 'Signal' | `disclosure.weights[5].label` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 200 | 10 | `disclosure.weights[5].pct` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 201 | 'Execution' | `disclosure.weights[6].label` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 202 | 10 | `disclosure.weights[6].pct` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 203 | 'Cascade' | `disclosure.weights[7].label` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 204 | 14 | `disclosure.weights[7].pct` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 205 | "Overall risk is a weighted sum of the 8 dimension scores. State | `disclosure.note` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
| 206 | 'Awaiting risk assessment — this dimension will populate once ma | `awaiting_dimensions_text` | docs/matrices/02-11-risk-matrix.md §2 + §3 | RENDERED |
