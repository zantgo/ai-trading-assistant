# RECOMMENDATION — front end vs export JSON (checklist)

Source: audits/2026-08-13-2/exports/recommendation.json
Docs: docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-...

| # | FRONT-END VALUE | EXPORT JSON PATH | DOCS SECTION | STATUS |
|---|---|---|---|---|
| 1 | 'recommendation' | `source_tab` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | EXPORT_ONLY |
| 2 | '2026-08-13T13:51:17.569Z' | `meta.datetime_utc` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | EXPORT_ONLY |
| 3 | 'Hyperliquid' | `meta.exchange` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 4 | 'ETH-USDC' | `meta.pair` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 5 | 60 | `meta.timeframe_secs` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 6 | 1890.35 | `meta.current_price` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 7 | 1900.8 | `meta.prev_day_price` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 8 | -0.5497685185185209 | `meta.price_change` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 9 | 'down' | `meta.price_change_direction` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 10 | 1786629060 | `meta.timestamp` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | EXPORT_ONLY |
| 11 | False | `meta.is_completed` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | EXPORT_ONLY |
| 12 | 'Recommendation' | `header.layer_name` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 13 | 'STAND ASIDE' | `header.badge.label` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 14 | 'STAND ASIDE' | `header.badge.sublabel` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 15 | 'warn' | `header.badge.tone` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 16 | 'Confidence' | `header.chips[0].label` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 17 | 19 | `header.chips[0].value` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 18 | 'R:R' | `header.chips[1].label` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 19 | 'N/A' | `header.chips[1].value` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 20 | 'Stance' | `header.chips[2].label` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 21 | 'Cautious' | `header.chips[2].value` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 22 | 'live' | `header.status` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | EXPORT_ONLY |
| 23 | -22 | `gauge.net_bias_pct` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 24 | 'SHORT' | `gauge.bias_direction` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 25 | 2 | `gauge.long_pct` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 26 | 24 | `gauge.short_pct` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 27 | 74 | `gauge.hold_pct` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 28 | '-22%' | `gauge.net_bias_display` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 29 | 'Neutral' | `environment.directional_guidance` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 30 | 'Cautious' | `environment.market_stance` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 31 | 'HighVolatility' | `environment.strategy_environment` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 32 | 'Pullback' | `environment.opportunity_classification` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 33 | 19.308940848254576 | `environment.confidence_pct` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 34 | 'STAND_ASIDE' | `environment.readiness` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 35 | 45.08179545454546 | `environment.entry_danger_score` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 36 | 'MODERATE' | `environment.entry_danger_level` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 37 | 'HOLD' | `verdict.top` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 38 | 2 | `verdict.long_probability` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 39 | 24 | `verdict.short_probability` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 40 | 74 | `verdict.hold_probability` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 41 | 'Pullback' | `top_setup.opportunity_type` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 42 | 'DirectionalNeutral' | `top_setup.viability` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 43 | 'HOLD · NO DIRECTIONAL EDGE' | `top_setup.badge_text` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 44 | 59.83640909090909 | `top_setup.score` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 45 | 2 | `top_setup.preconditions_met` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 46 | 2 | `top_setup.preconditions_total` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 47 | 'NEUTRAL' | `top_setup.direction_label` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 48 | 1891.7 | `top_setup.entry_zone.low` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 49 | 1892.5396304152107 | `top_setup.entry_zone.high` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 50 | 1887.5018479239466 | `top_setup.target_zone.low` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 51 | 1889.1811087543679 | `top_setup.target_zone.high` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 52 | 1892.625 | `top_setup.invalidation` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 53 | '$1892–$1893' | `top_setup.entry_zone_display` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 54 | '$1888–$1889' | `top_setup.target_zone_display` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 55 | '$1893' | `top_setup.invalidation_display` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 56 | 'R:R 1 : 5.8' | `top_setup.rr_display` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 57 | True | `top_setup.rr_available` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 58 | 5.82 | `top_setup.rr_value` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 59 | None | `top_setup.rr_reason` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 60 | 'Pullback: preconditions 2/2' | `top_setup.rationale` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 61 | None | `no_clear_card` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 62 | 'STAND_ASIDE' | `safety_flags.readiness` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 63 | False | `safety_flags.rr_available` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 64 | None | `safety_flags.rr_value` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 65 | 'no_directional_bias' | `safety_flags.rr_reason` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 66 | 5.338769933415519 | `safety_flags.stop_loss_pct` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 67 | 19.308940848254576 | `safety_flags.confidence_pct` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 68 | 45.08179545454546 | `safety_flags.entry_danger_score` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 69 | 'MODERATE' | `safety_flags.entry_danger_level` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 70 | 'N/A' | `safety_flags.rr_display` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 71 | '5.34%' | `safety_flags.stop_loss_display` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 72 | '19%' | `safety_flags.confidence_display` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 73 | '45 (MODERATE)' | `safety_flags.entry_danger_display` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 74 | 'No directional edge — these bullets read the same across all th | `why_note` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 75 | 'Neutral bias, confluence score 0 (L2 tradability_dim + L3 quali | `why[0]` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 76 | 'Setup: Pullback (L4 score 60, Moderate)' | `why[1]` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 77 | 'Trade readiness = STAND_ASIDE because confidence_assessment 19  | `why[2]` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 78 | 'hold' | `price_levels.side` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 79 | None | `price_levels.entry_zone` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 80 | None | `price_levels.target_zone` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 81 | None | `price_levels.invalidation` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 82 | 'SWING' | `price_levels.horizon` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 83 | 'No active setup — verdict is HOLD. Top Setup card above carries | `price_levels.hold_placeholder` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 84 | 'Wait For Confirmation' | `strategy.entry` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 85 | 'Trend Weakening' | `strategy.exit` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 86 | 'ATR-Based' | `strategy.protection` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 87 | 'Trailing Method' | `strategy.target` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
| 88 | 'Neutral — no directional edge: NEUTRAL bias with 19% confidence | `final_verdict` | docs/matrices/02-04-decision-matrix.md §3 + docs/operations-and-compliance/06-01-... | RENDERED |
