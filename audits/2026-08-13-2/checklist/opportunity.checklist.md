# OPPORTUNITY — front end vs export JSON (checklist)

Source: audits/2026-08-13-2/exports/opportunity.json
Docs: docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4

| # | FRONT-END VALUE | EXPORT JSON PATH | DOCS SECTION | STATUS |
|---|---|---|---|---|
| 1 | 'opportunity' | `source_tab` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | EXPORT_ONLY |
| 2 | '2026-08-13T13:52:10.005Z' | `meta.datetime_utc` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | EXPORT_ONLY |
| 3 | 'Hyperliquid' | `meta.exchange` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 4 | 'ETH-USDC' | `meta.pair` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 5 | 60 | `meta.timeframe_secs` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 6 | 1890.3 | `meta.current_price` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 7 | 1900.8 | `meta.prev_day_price` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 8 | -0.55239898989899 | `meta.price_change` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 9 | 'down' | `meta.price_change_direction` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 10 | 1786629120 | `meta.timestamp` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | EXPORT_ONLY |
| 11 | False | `meta.is_completed` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | EXPORT_ONLY |
| 12 | 'Opportunity' | `header.layer_name` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 13 | 'Pullback' | `header.badge.label` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 14 | 'Moderate' | `header.badge.sublabel` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 15 | 'neutral' | `header.badge.tone` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 16 | 'Score' | `header.chips[0].label` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 17 | 58.9 | `header.chips[0].value` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 18 | 'R:R' | `header.chips[1].label` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 19 | '1:4.48' | `header.chips[1].value` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 20 | 'Horizon' | `header.chips[2].label` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 21 | 'SWING' | `header.chips[2].value` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 22 | 'live' | `header.status` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | EXPORT_ONLY |
| 23 | 1 | `directional_bars.bullish_pct` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 24 | 58 | `directional_bars.bearish_pct` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 25 | 41 | `directional_bars.hold_pct` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 26 | 'desc' | `directional_bars.sort` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 27 | 'Pullback' | `header_block.opportunity_class` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 28 | 'Lean: neutral' | `header_block.lean` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 29 | 58.90257142857143 | `header_block.setup_score` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 30 | 'MODERATE' | `header_block.setup_quality` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 31 | 'Pullback' | `trade_setups[0].opportunity_type` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 32 | 'DirectionalNeutral' | `trade_setups[0].viability` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 33 | 'NEUTRAL · HOLD' | `trade_setups[0].badge_text` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 34 | 'NEUTRAL' | `trade_setups[0].side` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 35 | 0 | `trade_setups[0].rank_idx` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 36 | False | `trade_setups[0].is_top` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 37 | True | `trade_setups[0].geometry_consistent` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 38 | 1891.3171731799248 | `trade_setups[0].entry_mid` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 39 | 1890.9 | `trade_setups[0].entry_zone.low` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 40 | 1891.7343463598493 | `trade_setups[0].entry_zone.high` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 41 | 1888.3969609204523 | `trade_setups[0].tp1` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 42 | 1886.7282682007537 | `trade_setups[0].tp2` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 43 | 1892.155 | `trade_setups[0].invalidation` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 44 | True | `trade_setups[0].rr_available` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 45 | 3.49 | `trade_setups[0].rr_value` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 46 | None | `trade_setups[0].rr_reason` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 47 | 58.90257142857143 | `trade_setups[0].score` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 48 | 2 | `trade_setups[0].preconditions_met` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 49 | 2 | `trade_setups[0].preconditions_total` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 50 | 'Pullback' | `trade_setups[0].notes` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 51 | 'NO CLEAR OPPORTUNITY' | `no_clear_strip.badge` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 52 | 0 | `no_clear_strip.preconditions_met` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 53 | 1 | `no_clear_strip.preconditions_total` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 54 | '0/1 preconditions met · informational only' | `no_clear_strip.meta` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 55 | "HOLD / NO CLEAR — No directional call. The cards below show eac | `hold_scenario_note` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 56 | False | `rr_internal.expected_rr_available` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 57 | None | `rr_internal.expected_rr_value` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 58 | 'no_directional_bias' | `rr_internal.expected_rr_reason` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 59 | 'SWING' | `rr_internal.time_horizon` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 60 | 'Close below 1876.8 invalidates the Pullback thesis.' | `invalidation_note` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 61 | 'Liquidity Squeeze' | `evaluated_setups[0].opportunity_type` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 62 | 'NoClear' | `evaluated_setups[0].viability` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 63 | 58.90257142857143 | `evaluated_setups[0].score` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 64 | 0 | `evaluated_setups[0].preconditions_met` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 65 | 3 | `evaluated_setups[0].preconditions_total` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 66 | None | `evaluated_setups[0].trade_viability` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 67 | 'Liquidity Squeeze' | `evaluated_setups[0].notes` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 68 | 'Scalp' | `evaluated_setups[1].opportunity_type` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 69 | 'NoClear' | `evaluated_setups[1].viability` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 70 | 58.90257142857143 | `evaluated_setups[1].score` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 71 | 0 | `evaluated_setups[1].preconditions_met` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 72 | 3 | `evaluated_setups[1].preconditions_total` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 73 | None | `evaluated_setups[1].trade_viability` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 74 | 'Scalp' | `evaluated_setups[1].notes` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 75 | 'Trend Continuation' | `evaluated_setups[2].opportunity_type` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 76 | 'NoClear' | `evaluated_setups[2].viability` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 77 | 58.90257142857143 | `evaluated_setups[2].score` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 78 | 0 | `evaluated_setups[2].preconditions_met` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 79 | 3 | `evaluated_setups[2].preconditions_total` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 80 | None | `evaluated_setups[2].trade_viability` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 81 | 'Trend Continuation' | `evaluated_setups[2].notes` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 82 | 'Breakout' | `evaluated_setups[3].opportunity_type` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 83 | 'NoClear' | `evaluated_setups[3].viability` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 84 | 58.90257142857143 | `evaluated_setups[3].score` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 85 | 0 | `evaluated_setups[3].preconditions_met` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 86 | 2 | `evaluated_setups[3].preconditions_total` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 87 | None | `evaluated_setups[3].trade_viability` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 88 | 'Breakout' | `evaluated_setups[3].notes` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 89 | 'Reversal' | `evaluated_setups[4].opportunity_type` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 90 | 'NoClear' | `evaluated_setups[4].viability` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 91 | 58.90257142857143 | `evaluated_setups[4].score` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 92 | 0 | `evaluated_setups[4].preconditions_met` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 93 | 3 | `evaluated_setups[4].preconditions_total` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 94 | None | `evaluated_setups[4].trade_viability` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 95 | 'Reversal' | `evaluated_setups[4].notes` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 96 | 'Pullback' | `evaluated_setups[5].opportunity_type` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 97 | 'DIRECTIONAL_NEUTRAL' | `evaluated_setups[5].viability` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 98 | 58.90257142857143 | `evaluated_setups[5].score` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 99 | 2 | `evaluated_setups[5].preconditions_met` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 100 | 2 | `evaluated_setups[5].preconditions_total` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 101 | 'DIRECTIONAL_NEUTRAL' | `evaluated_setups[5].trade_viability` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 102 | 'Pullback' | `evaluated_setups[5].notes` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 103 | 'Mean Reversion' | `evaluated_setups[6].opportunity_type` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 104 | 'NoClear' | `evaluated_setups[6].viability` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 105 | 58.90257142857143 | `evaluated_setups[6].score` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 106 | 0 | `evaluated_setups[6].preconditions_met` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 107 | 2 | `evaluated_setups[6].preconditions_total` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 108 | None | `evaluated_setups[6].trade_viability` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 109 | 'Mean Reversion' | `evaluated_setups[6].notes` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 110 | 1892.155 | `confluent_entry_levels[0].price` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 111 | 'VP' | `confluent_entry_levels[0].sources[0]` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 112 | 30 | `confluent_entry_levels[0].strength` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 113 | 1877.585 | `confluent_target_levels[0].price` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 114 | 'VP' | `confluent_target_levels[0].sources[0]` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 115 | 30 | `confluent_target_levels[0].strength` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 116 | 'Neutral' | `market_position.bias` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 117 | 'Accumulation' | `market_position.regime` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 118 | 'Developing' | `market_position.trend` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 119 | 'Average' | `market_position.quality` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 120 | 4 | `environment.timeframes_considered` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 121 | '4/4 TFs considered' | `environment.timeframes_considered_display` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 122 | 42 | `environment.confidence_pct` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
| 123 | '42%' | `environment.confidence_display` | docs/matrices/02-08-opportunity-matrix.md §2 + §3 + §4 | RENDERED |
