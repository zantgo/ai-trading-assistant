# Export Data Payload Schema

<!-- pascal-display-strings -->

<!-- This document's JSON examples carry UI display strings (e.g.
     "Momentum", "Bullish", "Trend Continuation") which are intentionally
     PascalCase — they document the screen-facing *display* fields, not
     wire enums. Exempted from the G6 enum-casing lint via the marker. -->

**Version:** 6.10 (2026-08-13) — see docs/CHANGELOG.md for the canonical version history.
**Status:** Approved
**Purpose:** This document specifies the JSON payload produced by every panel's `Export Data` button. Each panel's export mirrors **1:1 the data the panel renders** — the same numbers, the same prices, the same dynamic strings, the same words; only the presentation changes (the screen formats `63390` as `$63390`, the JSON carries the raw value). Consumers (AI agents, downstream services, debugging tools) can rely on the field shapes documented here.

> **Filter pills.** On the single-TF Metrics surface the filter pills
> (`Active only`, `Confirmed+`, `Hide gates`, `Hide overlays`, search) are
> presentation-only conveniences — that export always carries the full
> dataset. The **MTF export is the one exception**: it serializes the
> active pill state (`filter_state`) plus a per-row `visible` flag so the
> on-screen row set is reconstructible from the JSON (the payload rows
> themselves remain the unfiltered superset).

---

## 1. Architecture Overview

Each panel has a **scoped builder** that emits a JSON payload matching the panel's rendered surface. The legacy "kitchen-sink" (`buildMetricsExportJson` in `lib/metricsExport.ts`) is preserved for backward compatibility but is **not** called by any panel.

| Tab (UI label) | Panel source | Builder file | Source button |
|---|---|---|---|
| Charts → Positions | `BottomConsole.svelte` | `lib/exportBuilders/chartsTab.ts` | `buildPositionsTabExport` |
| Charts → Open Orders | `BottomConsole.svelte` | `lib/exportBuilders/chartsTab.ts` | `buildOrdersTabExport` |
| Charts → History | `BottomConsole.svelte` | `lib/exportBuilders/chartsTab.ts` | `buildHistoryTabExport` |
| Charts → Plan | `BottomConsole.svelte` | `lib/exportBuilders/chartsTab.ts` | `buildPlanTabExport` |
| Metrics → single-TF | `TerminalMonitor.svelte` | `lib/exportBuilders/metricsTab.ts` | `buildMetricsTabExport` |
| Metrics → MTF | `TerminalMonitor.svelte` | `lib/exportBuilders/mtfTab.ts` | `buildMtfExportJson` |
| Alignment | `AlignmentPanel.svelte` | `lib/exportBuilders/alignmentTab.ts` | `buildAlignmentTabExport` |
| Opportunities | `OpportunitiesPanel.svelte` | `lib/exportBuilders/opportunityTab.ts` | `buildOpportunityTabExport` |
| Risks | `RiskPanel.svelte` | `lib/exportBuilders/riskTab.ts` | `buildRiskTabExport` |
| Analysis | `AnalysisPanel.svelte` | `lib/exportBuilders/analysisTab.ts` | `buildAnalysisTabExport` |
| Recommendation | `RecommendationPanel.svelte` | `lib/exportBuilders/recommendationTab.ts` | `buildRecommendationTabExport` |

The shared envelope (`meta`, `header`, R:R helper) is in `lib/exportBuilders/shared.ts`. Canonical human-readable sentences shared between panels, facets and exports (Fibonacci GP status, VP position, …) live in `lib/structuralStrings.ts`; the phase prettifier is `lib/prettifyPhase.ts`.

---

## 2. Shared Envelope

### 2.1 MetaEnvelope — every payload

```json
{
  "source_tab": "metrics",
  "meta": {
    "datetime_utc": "2026-08-12T12:34:56.789Z",
    "exchange": "Hyperliquid",
    "pair": "BTC-USDT",
    "timeframe_secs": 60,
    "current_price": 63390.0,
    "prev_day_price": 62000.0,
    "price_change": 2.24,
    "price_change_direction": "up",
    "timestamp": 1753950000,
    "is_completed": true
  },
  "header": {
    "layer_name": "Metrics",
    "badge": { "label": "STRONG BULLISH", "sublabel": "TRENDING", "tone": "bull" },
    "chips": [ { "label": "Score", "value": 0.62 }, { "label": "Signals", "value": 7 } ],
    "status": "live"
  }
}
```

Rules:
- Numbers are raw (no `%`, `$`, `1 :` diminutives).
- Strings mixed with numbers are split into structured fields plus `*_display` verbatim copies of the screen sentence where the screen renders one (e.g. `rr_display`, `entry_danger_display`, `current_position_label`, `badge_text`).
- `header` mirrors the LayerHeader chrome (badge label/sublabel + meta chips + status). Chip values that are numeric strings are parsed to numbers.
- **No `filter_state` in `meta`.** The MTF and single-TF Metrics tabs carry a top-level `filter_state` block (see §3.2; metrics: §3.1) — never inside `meta`.

### 2.2 Canonical value sources (cross-tab consistency)

Every tab exports from the **same active instance**, so the meta block is
identical across all seven tabs for a given click-epoch. The rules below
are enforced by the shared `buildPriceBlock` in `lib/exportBuilders/shared.ts`
and by the panels' builder wiring:

| Field | Canonical source | Notes |
|---|---|---|
| `meta.pair` | The full `instancesMap` key (`BTC-USDT` / `BTC-USDC`) | NEVER the bare base (`BTC`). All panels pass the full `pairKey` prop; the Analysis panel resolves `app.activeTab` (not `activeSymbol`). |
| `meta.current_price` | `pickInstanceLivePrice(activeInstance terms)` — the freshest live price within 30 s, else last known good | **Live tick value.** Two exports clicked at the same instant carry the same price; sequential exports naturally drift as the market moves — that is expected and is not an inconsistency. |
| `meta.prev_day_price` / `price_change` / `price_change_direction` | `pickLatestCompletedSnapshot(terms)` — the newest **completed-candle** snapshot (shadow/live-tick frames drop `prev_day_px`) | Consistent across tabs: one canonical completed snapshot feeds all seven. |
| `meta.timestamp` | The snapshot's Unix-seconds timestamp (`null` for MTF — no single TF) | Single-TF exports carry their active TF's snapshot ts; the L3–L6 tabs carry the newest snapshot's ts. |
| `meta.timeframe_secs` | `0` for MTF (multi-TF sentinel); the active TF's `barDurationSec` otherwise | MTF also emits `meta.timesframes: ["Micro","Fast","Slow","Macro"]`. |
| `meta.datetime_utc` | `now` at click time | Each export is a fresh click-epoch; timestamps legitimately differ across sequential clicks. |
| `meta.is_completed` | The snapshot's `is_completed` flag | Consistent per snapshot. |

---

## 3. Per-Tab Payload Schemas

### 3.1 Metrics Tab (single-TF) — `source_tab: "metrics"`

Mirrors `TerminalMonitor.svelte` single-TF mode (Market Context strip, Group Confluence grid, Structural Anchors strip, Indicators/Signals/Divergences/Levels facets).

```json
{
  "source_tab": "metrics",
  "meta": { },
  "header": { },
  "filter_state": {
    "active_only": false,
    "confirmed_plus_only": false,
    "hide_gates": false,
    "hide_overlays": false,
    "query": ""
  },
  "market_context": {
    "regime": "TRENDING_BULL",
    "overall_score": 0.62,
    "overall_label": "STRONG_BULLISH",
    "trend":      { "score": 0.7, "confidence": 80, "label": "BULLISH" },
    "momentum":   { "score": 0.5, "confidence": 70, "label": "BULLISH" },
    "volatility": { "score": -0.2, "confidence": 60, "label": "EXPANDING" },
    "volume":     { "score": 0.3, "confidence": 65, "label": "STRONG" },
    "liquidity":  { "score": 0.4, "confidence": 60, "label": "HEALTHY" },
    "signal_count": 7,
    "age_bars_display": "5b"
  },
  "group_confluence": [
    {
      "group": "Momentum", "label": "Momentum",
      "total": 2, "gates": 0, "bullish": 2, "bearish": 0, "neutral": 0,
      "active": 2, "active_signals": 3, "dominant": "bull"
    }
  ],
  "structural_anchors": {
    "fibonacci": {
      "present": true,
      "gp_top": 64050, "gp_bottom": 62600,
      "swing_direction": "BULL SWING",
      "status": "INSIDE GP (-8.97% from center)",
      "price_vs_gp_pct": 0.10,
      "ext_1618": 65500, "ext_2618": 67200,
      "retracement_coefficients": {
        "fib_0236": 63100, "fib_0382": 63250, "fib_0500": 63330,
        "fib_0618": 63400, "fib_0660": 63440, "fib_0786": 63500
      }
    },
    "volume_profile": {
      "poc_price": 63300, "value_area_high": 63700, "value_area_low": 63000,
      "total_volume": 12500, "range_low": 62800, "range_high": 63900,
      "num_bins": 60, "timestamp_ms": 1753950000000,
      "top_hvn": [ { "price_low": 63280, "price_high": 63320, "volume": 400, "buy_volume": 250, "sell_volume": 150, "strength_x_mean": 1.6 } ],
      "buy_total": 450, "sell_total": 370,
      "buy_sell_bias": 0.0976,
      "current_position_label": "INSIDE VALUE AREA",
      "range_pos_pct": 48.18
    },
    "liquidity": {
      "oi_long_pct": 57, "oi_short_pct": 43,
      "cascade_state": "SUSTAINED", "cascade_intensity": 72.5,
      "cascade_intensity_display": "73",
      "cascade_state_label": "SUSTAINED",
      "cascade_asymmetry": 0.35, "cascade_asymmetry_sign": "+",
      "cascade_asymmetry_magnitude_pct": 35.0,
      "cascade_asymmetry_description": "long_squeeze_risk",
      "estimation_confidence": 0.85, "estimation_confidence_pct": 85,
      "total_short_clusters": 4, "total_long_clusters": 4,
      "top_short": [
        { "peak_price": 63900, "price_low": 63800, "price_high": 63950, "notional_usd": 3200000,
          "dominant_leverage": 10, "distance_from_mid_pct": 0.8, "magnet_strength": 82, "cluster_kind": "ABOVE_CURRENT_PRICE" }
      ],
      "top_long": [ ]
    },
    "cascade_alert": { "state": "SUSTAINED", "intensity": 72.5 },
    "micro_volume_profile": {
      "poc_price": 63300, "value_area_high": 63700, "value_area_low": 63000,
      "total_volume": 12500, "range_low": 62800, "range_high": 63900,
      "num_bins": 60, "timestamp_ms": 1753950000000,
      "top_hvn": [ ],
      "buy_total": 450, "sell_total": 370,
      "buy_sell_bias": 0.0976,
      "current_position_label": "INSIDE VALUE AREA",
      "range_pos_pct": 48.18
    },
    "micro_cascade_alert": { "state": "SUSTAINED", "intensity": 72.5 }
  },
  "indicators": [
    {
      "key": "rsi", "period": 14, "fast_period": null, "slow_period": null, "signal_period": null,
      "display_name": "RSI (14)",
      "group": "Momentum", "class": "Leading",
      "raw": 63.5, "raw_display": "63.50",
      "normalized_available": true, "normalized_value": 0.31, "normalized_reason": null,
      "state": "LIVE", "state_display": "LIVE",
      "confidence_pct": 70,
      "signals": [
        { "key": "rsi_14", "display_name": "RSI (14)", "kind": "CRO", "direction": "Bullish",
          "status": "Confirmed", "label": "RSI crossed above 60", "strength": 0.8,
          "age_bars": 2, "display_label": "CRO·2" }
      ],
      "sub_values": null,
      "indicator_lifecycle": {
        "state": "Live", "state_display": "LIVE", "bars_seen": 14, "bars_required": 14,
        "last_updated_at": 1753950000, "last_error": null, "feed_state": null, "not_active": false
      }
    }
  ],
  "signals_by_kind": {
    "Divergence": [], "Crossover": [ ],
    "Threshold": [], "Breakout": [], "BandTouch": [], "ZeroLineCross": [],
    "CompressionRelease": [], "LevelTest": [], "TrendFlip": [],
    "VolumeClimax": [], "StackChange": [], "PatternForming": []
  },
  "divergences": [
    {
      "key": "rsi", "period": 14, "display_name": "RSI (14)",
      "sub_kind": "Regular Bull", "direction": "Bullish", "status": "Active",
      "strength": 0.75, "confidence_pct": 70, "age_bars": 3,
      "label": "BULLISH_DIVERGENCE",
      "pivots": [ { "time": 1753950000, "value": 34.2 } ]
    }
  ],
  "levels": [
    {
      "key": "pivot_points", "coefficient": null, "display_name": "Pivot Points",
      "level_name": "R2", "kind": "Pivot", "role": "resistance",
      "value_key": "r2", "is_range": false,
      "price_text": "$64800",
      "direction": "Bearish", "status": "Potential", "strength": 0.7,
      "confidence_pct": 40, "age_bars": 2
    }
  ],
  "liquidity_panel": {
    "flow": {
      "available": true,
      "long_liquidations_usd": 50000, "short_liquidations_usd": 15000,
      "net_liquidation_usd": 35000, "event_count": 4,
      "largest_event_usd": 30000, "largest_event_price": 63400, "largest_event_side": "LONG",
      "cascade_state": "SUSTAINED", "cascade_intensity": 72.5
    },
    "cluster": {
      "available": true, "mid_price": 63390, "cascade_asymmetry": 0.35,
      "estimation_confidence": 0.85,
      "total_long_oi_usd": 40000000, "total_short_oi_usd": 30000000,
      "total_short_clusters": 4, "total_long_clusters": 4,
      "leverage_assumptions": {
        "source": "FUNDING_ADAPTIVE", "buckets": [1,3,5,10,20,50,100],
        "weights": [0.05,0.1,0.2,0.3,0.2,0.1,0.05],
        "funding_modulation_active": true, "funding_extreme_pct": 0.0005
      },
      "short_clusters": [ ], "long_clusters": [ ]
    },
    "context": {
      "available": true, "long_oi_usd": 40000000, "short_oi_usd": 30000000,
      "estimation_confidence_pct": 85, "signals": []
    }
  }
}
```

Notes:
- `display_name` is the registry display name exactly as rendered (`RSI (14)`).
- Signal `display_label` mirrors the on-screen badge (`CRO·2`, `DIV·3` — `·` separator only when age > 0).
- WARMING entries render `raw_display: "--"` with `raw: null` — never a fabricated `0.00`. The one exception is the onoff family (squeeze): the screen's onoff branch runs before the warming check, so a warming squeeze renders `"OFF"` in both places. Normalized cells for WARMING rows and for non-Directional modes (`normalization_mode` ≠ `Directional`, e.g. the EventOnly Hull MA) emit `normalized_available: false` + a `normalized_reason` (`warming` / `context_only`) — the screen renders `--` / `N/A`.
- `state_display` mirrors the State column incl. `Warming (5/14)`, `SILENT`, `WAITING FEED ⏳`, `AWAITING DATA` and the pending-candle `⦿` suffix. When no lifecycle map is present the legacy heuristic mirrors the screen exactly (`AWAITING DATA` for WARMING, `SILENT` for Conditional/DataOnly rows without signals, `—` for empty labels).
- Liquidity ladder clusters (`top_short` / `top_long`) are the **top-4 by magnet strength** — the same selection as the anchors-strip ladder and the Levels facet "Liquidation Magnets".
- `fibonacci.status` and `volume_profile.current_position_label` are the canonical sentences also rendered by the strip and the Levels facet (`lib/structuralStrings.ts`).
- **`micro_volume_profile` / `micro_cascade_alert`** mirror the Structural Anchors strip's Volume Profile tile and the Tier-1 cascade banner, both of which are anchored to the **micro** timeframe regardless of the active TF. `volume_profile` / `cascade_alert` mirror the active-TF values (Levels facet / strip liquidity tile).
- Derived strings (`fibonacci.status`, `price_vs_gp_pct`, VP position label, `age_bars_display`) are computed from the **active TF's `priceText`** — the same price the screen uses. `meta.current_price` remains the freshest price across all four slots.

**EMA Ribbon block (`ema`, per-TF Metrics tab only).** Added in v6.11 — the four EMA lines (`fast` / `medium` / `slow` / `long`) read the SAME canonical record as the chart overlay (`MarketSnapshot.indicators["ema_stack"].values.*`), so every surface shows byte-identical numbers. `distance_from_price = (close − ema[role]) / close`; `spread_pct = (values.fast − values.long) / close` (positive = bull ribbon). Periods flow from the configured `ema_fast/medium/slow/long` — the same config the dashboard settings UI edits.

```json
{
  "source_tab": "metrics",
  "ema": {
    "fast":   { "value": 63420.5, "period": 10, "distance_from_price": 0.0005 },
    "medium": { "value": 63300.0, "period": 50, "distance_from_price": 0.0014 },
    "slow":   { "value": 63100.0, "period": 100, "distance_from_price": 0.0045 },
    "long":   { "value": 62800.0, "period": 200, "distance_from_price": 0.0093 },
    "spread_pct": 0.0098
  }
}
```

Notes:
- Each line's `value` is null until the EMA has warmed up (`bars_seen < bars_required`); `distance_from_price` is null when the line or close is unavailable.
- The `ema` block is additive — it does NOT replace `indicators[ema_stack].sub_values.*` and does not change the registry row. Other 6 MME tab exports do not carry it.

### 3.2 MTF Tab — `source_tab: "mtf"`

Mirrors `MtfView.svelte` (4 × N grid with per-row agreement). The meta block
carries `timesframes: ["Micro","Fast","Slow","Macro"]` and
`timeframe_secs: 0` (multi-TF sentinel).

```json
{
  "source_tab": "mtf",
  "meta": { "timesframes": ["Micro", "Fast", "Slow", "Macro"] },
  "header": { },
  "filter_state": {
    "active_only": false, "confirmed_plus_only": false,
    "hide_gates": false, "hide_overlays": false, "query": ""
  },
  "groups": [
    { "key": "Momentum", "label": "Momentum", "accent": "#a78bfa", "indicator_count": 2, "total_indicator_count": 2 }
  ],
  "indicators": [
    {
      "key": "rsi", "period": 14, "display_name": "RSI (14)",
      "group": "Momentum", "label": "Momentum", "class": "Leading",
      "directional": true, "visible": true,
      "normalized_available": true, "confidence_pct": 70,
      "values": [
        { "timeframe": "Micro", "normalized": 0.31, "normalized_display": "+0.31", "active": true },
        { "timeframe": "Fast",  "normalized": 0.31, "normalized_display": "+0.31", "active": true },
        { "timeframe": "Slow",  "normalized": 0.31, "normalized_display": "+0.31", "active": true },
        { "timeframe": "Macro", "normalized": 0.31, "normalized_display": "+0.31", "active": true }
      ],
      "agreement": 0.31, "agreement_display": "+0.31", "agreement_label": "BULL"
    }
  ],
  "group_confluence": [ ],
  "signals_by_kind": { "Divergence": [], "Crossover": [ ] },
  "divergences": [ ],
  "levels": [ ],
  "liquidity_panel": { "flow": null, "cluster": null, "context": { "available": true, "signals": [] } },
  "timeframes": [
    {
      "label": "Micro", "duration_seconds": 60, "mark_price": 63390,
      "timestamp": 1753950000, "pipeline_state": "LIVE", "is_completed": true,
      "context": null,
      "fibonacci_summary": { "present": true, "gp_top": 64050, "gp_bottom": 62600, "swing_direction": "BULL SWING", "status": "INSIDE GP (-8.97% from center)", "ext_1618": 65500, "ext_2618": 67200, "retracement_coefficients": { } },
      "volume_profile": null, "liquidity_cluster": null, "liquidity_flow": null,
      "indicators": [
        {
          "key": "rsi", "period": 14, "display_name": "RSI (14)",
          "group": "Momentum", "class": "Leading",
          "raw": 63.5, "raw_display": "63.50",
          "normalized_available": true, "normalized_value": 0.31,
          "state": "LIVE", "state_display": "LIVE",
          "confidence_pct": 70, "signals": [], "sub_values": null
        }
      ]
    }
  ]
}
```

Notes:
- Per-TF indicator rows carry the **same triple** as the single-TF Metrics
  export: `raw` / `raw_display` / `state_display` (state is humanized the
  same way — `OVERBOUGHT_DISTRIBUTION` → `OVERBOUGHT DISTRIBUTION`).
- The top-level `signals_by_kind` / `divergences` / `levels` /
  `group_confluence` / `liquidity_panel` blocks aggregate across all 4 TFs
  with the exact shapes the Metrics single-TF export uses:
  `signals_by_kind` uses the **canonical kind keys** (`LevelTest`,
  `Divergence`, `Threshold`, … — never the abbreviated `LV`/`DIV`/… tokens;
  the per-entry `kind` field stays abbreviated like the Metrics entries),
  and `divergences` / `levels` surface every Divergence / LevelTest signal
  present on any TF.
- **`filter_state` + `visible` (v7.0-verify).** The MTF export serializes
  the active filter pills and marks every row with the same
  `filterRegistry` visibility the on-screen grid applies (`Active only`
  counts signals across all four slots — same as `MtfView.svelte`).
  `groups[].indicator_count` counts **visible** rows (the screen's
  section count); `total_indicator_count` counts all registry rows in the
  group. Consumers reconstruct the on-screen set via
  `indicators[].visible` / `filter_state`.

### 3.3 Alignment Tab — `source_tab: "alignment"`

```json
{
  "source_tab": "alignment",
  "meta": { },
  "header": { },
  "hero": {
    "mtf_overall_score": 0.4, "mtf_overall_label": "STRONG_BULLISH",
    "mtf_overall_label_display": "STRONG BULL",
    "timeframes_present": 4, "signal_cross_tf_count": 2, "trend_agreement_pct": 82
  },
  "breakdown_meta": "T:0.45 M:0.30 Vt:0.10 Vm:-0.20",
  "dimensions": [
    { "name": "Trend", "score": 75, "state": "STRONG", "confidence": 78 }
  ],
  "consensus": {
    "trend_agreement_pct": 82,
    "label": "strong_consensus",
    "label_display": "Strong consensus — timeframes aligned",
    "polarization": [
      { "key": "T",  "label": "Trend",      "value": 0.45, "value_display": "+0.45" },
      { "key": "M",  "label": "Momentum",   "value": 0.30, "value_display": "+0.30" },
      { "key": "Vt", "label": "Volume",     "value": 0.10, "value_display": "+0.10" },
      { "key": "Vm", "label": "Volatility", "value": -0.20, "value_display": "-0.20" }
    ]
  },
  "per_timeframe": [
    { "timeframe": "MICRO", "trend_score": 0.45, "trend_score_display": "0.45",
      "momentum_score": 0.30, "momentum_score_display": "0.30",
      "overall_score": 1.0, "overall_score_display": "1.0",
      "regime": "TRENDING_BULL", "active_signals": 5 }
  ],
  "score_calculation": {
    "weights": [
      { "key": "T", "label": "Trend", "pct": 50, "color": "#22c55e",
        "value": 0.45, "value_display": "+0.45", "contribution": 0.225, "contribution_display": "+0.23" }
    ],
    "formula": "0.5 * (0.45) + 0.3 * (0.30) + 0.1 * (0.10) + 0.1 * (-0.20) = 0.4"
  },
  "interpretation": "Multi-timeframe alignment shows <strong>strong directional consensus</strong> (82% agreement across 4/4 timeframes). The composite score of 0.4 is classified as <strong>STRONG BULL</strong>. 2 cross-timeframe signals reinforce the current bias.",
  "consensus_conflict_banner": ""
}
```

Notes:
- Dimension `confidence` is already 0..100 on the wire — it mirrors the screen's `confidence.toFixed(0)%` (no ×100 inflation).
- `interpretation` carries the real `mtf_overall_label` (never a hardcoded token) and matches the panel sentence verbatim (HTML `<strong>` markers included).
- `consensus.trend_agreement_pct` keeps the raw float (screen text `toFixed(0)`, bar width `toFixed(1)`).
- The `NO_DATA` dimension state renders `"NO DATA"` on both surfaces (panel and export).
- **Null state (`alignment: null`).** The consensus block mirrors the screen placeholders exactly: `trend_agreement_pct: null`, `label: null`, `label_display: "—"`, polarization `value_display: "+0.00"`, and score-calculation `value_display` / `contribution_display` / `formula` all `"—"` — never fabricated zeros or a fabricated "Conflict" verdict.

### 3.4 Opportunity Tab — `source_tab: "opportunity"`

```json
{
  "source_tab": "opportunity",
  "meta": { },
  "header": { },
  "directional_bars": { "bullish_pct": 60, "bearish_pct": 10, "hold_pct": 30, "sort": "desc" },
  "header_block": {
    "opportunity_class": "Trend Continuation",
    "lean": "Bullish setups dominate",
    "setup_score": 78, "setup_quality": "STRONG"
  },
  "trade_setups": [
    {
      "opportunity_type": "Trend Continuation", "viability": "Actionable",
      "badge_text": "TOP · ACTIONABLE", "side": "LONG", "rank_idx": 0, "is_top": true,
      "geometry_consistent": true,
      "entry_mid": 63300, "entry_zone": { "low": 63200, "high": 63400 },
      "tp1": 66000, "tp2": 66500, "invalidation": 62800,
      "rr_available": true, "rr_value": 2.5, "rr_reason": null,
      "score": 78, "preconditions_met": 3, "preconditions_total": 3,
      "notes": "Trend + bias + momentum aligned"
    },
    {
      "opportunity_type": "Mean Reversion", "viability": "DirectionalNeutral",
      "badge_text": "NEUTRAL · HOLD", "side": "NEUTRAL", "rank_idx": 1, "is_top": false,
      "geometry_consistent": false, "entry_mid": null, "entry_zone": null,
      "tp1": 0, "tp2": 0, "invalidation": null,
      "rr_available": false, "rr_value": null, "rr_reason": "no_actionable_geometry",
      "score": 42, "preconditions_met": 2, "preconditions_total": 3, "notes": "Reversion candidate"
    }
  ],
  "no_clear_strip": null,
  "hold_scenario_note": null,
  "rr_internal": {
    "expected_rr_available": true, "expected_rr_value": 2.5, "expected_rr_reason": null,
    "time_horizon": "SWING"
  },
  "invalidation_note": "Close below 62800 invalidates the continuation thesis.",
  "evaluated_setups": [
    { "opportunity_type": "Trend Continuation", "viability": "Actionable", "score": 78,
      "preconditions_met": 3, "preconditions_total": 3, "trade_viability": "Actionable",
      "notes": "Trend + bias + momentum aligned" }
  ],
  "confluent_entry_levels": [
    { "price": 63330, "sources": ["FIB", "VP", "PP"], "strength": 78 }
  ],
  "confluent_target_levels": [ ],
  "market_position": { "bias": "Bullish", "regime": "TRENDING_BULL", "trend": "Healthy", "quality": "Good" },
  "environment": { "timeframes_considered": 4, "timeframes_considered_display": "4/4 TFs considered", "confidence_pct": 72, "confidence_display": "72%" }
}
```

Notes:
- `trade_setups` mirrors the panel's full leaderboard **one card per qualifying profile** — NEUTRAL-side cards (`side: "NEUTRAL"`, `NEUTRAL · HOLD`) and aggregate-bracket fallbacks included.
- `viability` is the **PascalCase-normalized** token (`Actionable` / `DirectionalNeutral` / `GeometryInverted` / `NoClear`). The wire serializes `TradeViability` as SCREAMING_SNAKE_CASE (`ACTIONABLE`); the panel conditionals and the export's `badge_text` both compare on the normalized form, so `badge_text` (`TOP · ACTIONABLE`, `NEUTRAL · HOLD`, `GEOMETRY INVERTED`) matches the screen exactly.
- `rr_value` prefers the wire per-side `expected_rr_internal` exactly like the screen card.
- `hold_scenario_note` (badge `HOLD / NO CLEAR` + body) appears only when the decision rank is HOLD.
- Confluent levels are capped at the screen's first 4 per group; sources are the on-screen abbreviations (FIB/VP/PP/SR/LIQ, with the screen's `ATR` fallback for unknown tokens).
- The evaluated list excludes the NoClearOpportunity profile (it has its own strip).
- `directional_bars` is **always** emitted — when the matrix is absent it mirrors the screen's always-rendered bars as `{0, 0, 100}`.
- `expected_rr` mirrors the screen cell exactly: `N/A` only when the verdict is HOLD **and** the active-side R:R is 0; any other state emits `available: true` with the raw value (including `0` → screen `"0.00"`).
- `evaluated_setups[].notes` / `trade_setups[].notes` are the **raw wire strings** the panel renders verbatim (never prettified).
- Empty states render the screen's `"—"` placeholder in `header_block.opportunity_class`, `rr_internal.time_horizon` and all four `market_position` fields.

### 3.5 Risk Tab — `source_tab: "risk"`

```json
{
  "source_tab": "risk",
  "meta": { },
  "header": { },
  "hero": {
    "overall_score": 48, "overall_level": "Moderate", "overall_state": "Elevated",
    "overall_confidence": 74, "top_severity": "High",
    "hint": "Lower is safer. State modifiers adjust each dimension's contribution but not the headline score."
  },
  "summary_counts": {
    "very_low": { "label": "Very Low", "count": 0 },
    "low": { "label": "Low", "count": 3 },
    "moderate": { "label": "Moderate", "count": 2 },
    "high": { "label": "High", "count": 3 },
    "extreme": { "label": "Extreme", "count": 0 }
  },
  "dimensions": [
    {
      "name": "Cascade Risk", "key": "cascade_risk",
      "weight": 0.14, "weight_pct": 14,
      "score": 70, "level": "High", "state": "Critical",
      "state_display": "⚠ CRITICAL", "confidence": 85,
      "evidence": ["SUSTAINED cascade above price"],
      "no_evidence_text": null, "not_active_text": null,
      "awaiting": false, "awaiting_badge": null,
      "bar_pct": 70, "weight_mark_pct": 14,
      "is_cascade_dim": true, "not_active": false,
      "cascade_extras": {
        "state_label": "SUSTAINED", "intensity_display": "72.5",
        "asymmetry_sign": "+", "asymmetry_magnitude_pct": 35.0,
        "asymmetry_description": "short squeeze",
        "asymmetry_display": "↑35.0% (short squeeze)"
      }
    }
  ],
  "headline_parts": {
    "very_low_count": 0, "low_count": 3, "moderate_count": 2,
    "high_count": 3, "extreme_count": 0, "overall_level": "Moderate"
  },
  "interpretation_headline": "3 high · 2 moderate · overall moderate",
  "interpretation_full": "<strong>Elevated risk environment.</strong> 3 dimensions at high levels. Consider reduced position sizing and wider stops. Monitor the highest-severity dimensions for evidence of improvement before committing capital. Overall composite score is <strong>moderate</strong> at 74% confidence.",
  "disclosure": {
    "weights": [
      { "label": "Market", "pct": 14 }, { "label": "Volatility", "pct": 14 },
      { "label": "ExecLiq", "pct": 14 }, { "label": "Structure", "pct": 10 },
      { "label": "Momentum", "pct": 14 }, { "label": "Signal", "pct": 10 },
      { "label": "Execution", "pct": 10 }, { "label": "Cascade", "pct": 14 }
    ],
    "note": "Overall risk is a weighted sum of the 8 dimension scores. State and confidence modify each dimension's contribution, but do not alter the headline score directly."
  },
  "awaiting_dimensions_text": "Awaiting risk assessment — this dimension will populate once market data stabilizes."
}
```

Notes:
- `top_severity` is `null` when it equals the overall level (the screen hides the "peak" chip in that case).
- `asymmetry_magnitude_pct` is the screen's percentage (`|asym| × 100`); `asymmetry_display` is the exact badge sentence.
- Zero-count sentences are omitted from `interpretation_full` exactly like the screen paragraph.
- Dimension names are **byte-identical to the screen cards** — including the abbreviated `"Exec Liquidity Risk"` (not `"Execution Liquidity Risk"`).
- **Null state (`risk: null`).** `dimensions` carries the 8 placeholder rows the screen's "AWAITING" cards render (name + `weight_pct`, `awaiting: true`, `awaiting_badge: "AWAITING"`), and `interpretation_full` carries the "Risk synthesis is initializing — …" paragraph verbatim.

### 3.6 Analysis Tab — `source_tab: "analysis"`

```json
{
  "source_tab": "analysis",
  "meta": { },
  "header": { },
  "body": {
    "bias": "Bullish", "confidence_pct": 72, "state_confidence": 0.72,
    "market_regime": "TRENDING_BULL", "market_quality": "Good", "cycle_phase": "MARKUP"
  },
  "signal_lean_hero": {
    "label_html": "Net bullish (2↑ vs 1↓)",
    "meta_html": "2.0:1 signal ratio",
    "bullish_pct": 67, "bearish_pct": 33, "tone": "bull"
  },
  "signals": {
    "supporting": [
      { "key": "rsi", "period": 14, "display_name": "RSI 14", "timeframe": "MICRO",
        "score": 62, "score_display": "+62", "regime": "TRENDING_BULL",
        "signals_count": 3, "signals_count_display": "3",
        "raw": "MICRO (bullish): rsi_14 score +62, TRENDING_BULL regime, 3 signals" }
    ],
    "contradicting": [ ],
    "list": [
      { "key": "rsi", "period": 14, "display_name": "RSI 14", "timeframe": "MICRO",
        "score": 62, "score_display": "+62", "regime": "TRENDING_BULL",
        "signals_count": 3, "signals_count_display": "3",
        "raw": "…", "bucket": "supporting" }
    ],
    "lean": { "label": "Net bullish · 2↑ vs 1↓", "bullish": 2, "bearish": 1, "tone": "bull" }
  },
  "qualitative_assessment": {
    "trend": "Healthy", "momentum": "Increasing", "structure": "Strong",
    "volatility": "Normal", "volume": "Strong", "cycle_phase": "MARKUP"
  },
  "per_timeframe_alignment": [
    { "name": "MICRO", "active": true, "trend": 0.45, "trend_display": "+0.45",
      "momentum": 0.3, "momentum_display": "+0.30",
      "overall": 1.0, "overall_display": "+1.0", "regime": "TRENDING_BULL" },
    { "name": "MACRO", "active": true, "trend": -0.1, "trend_display": "-0.10",
      "momentum": -0.05, "momentum_display": "-0.05",
      "overall": -0.2, "overall_display": "-0.2", "regime": "RANGE" }
  ],
  "interpretation": "Price is making higher highs and higher lows on strong volume. Momentum is increasing and structure remains intact.",
  "interpretation_display": "Price is making <strong>higher</strong> highs and higher lows on <strong>strong</strong> volume. Momentum is <strong>increasing</strong> and structure remains intact.",
  "rationale": "The market is in a healthy uptrend with broad participation across timeframes."
}
```

Notes:
- `signals.list` is the exact merged, timeframe-sorted list the screen grid squares render (bucket-annotated).
- With zero directional signals the hero still carries the screen sentences (`label_html: "No signals"`, `meta_html: "Waiting for cross-TF consensus"`) — including when `analysis` is null; the hero block is never absent.
- The split-tone hero label is exactly the screen's `"Split signals"` (the bull/bear counts live in `meta_html` — no parenthetical in `label_html`).
- Each signal row carries `signals_count_display` (`"—"` when the count is absent) alongside the raw `signals_count`.
- Empty states render the screen's `"—"` placeholder in the five qualitative cards, `cycle_phase`, the inactive per-TF score displays and `rationale`.
- `interpretation` is the raw text; `interpretation_display` carries the keyword-`<strong>` markup the screen renders (shared `highlightKeywords` helper).
- `cycle_phase` uses the shared `prettifyPhase` helper — identical string on screen and in the JSON.

### 3.7 Recommendation Tab — `source_tab: "recommendation"`

```json
{
  "source_tab": "recommendation",
  "meta": { },
  "header": { },
  "gauge": {
    "net_bias_pct": 45, "bias_direction": "LONG",
    "long_pct": 60, "short_pct": 15, "hold_pct": 25,
    "net_bias_display": "+45%"
  },
  "environment": {
    "directional_guidance": "Long", "market_stance": "Constructive",
    "strategy_environment": "TrendFollowing", "opportunity_classification": "TrendContinuation",
    "confidence_pct": 72, "readiness": "READY",
    "entry_danger_score": 35, "entry_danger_level": "LOW"
  },
  "verdict": { "top": "LONG", "long_probability": 60, "short_probability": 15, "hold_probability": 25 },
  "top_setup_empty_text": null,
  "top_setup": {    "opportunity_type": "Trend Continuation", "viability": "Actionable",
    "badge_text": "ACTIONABLE", "score": 78,
    "preconditions_met": 3, "preconditions_total": 3, "direction_label": "LONG",
    "entry_zone": { "low": 63200, "high": 63400 }, "target_zone": { "low": 66000, "high": 66500 },
    "invalidation": 62800,
    "entry_zone_display": "$63200–$63400", "target_zone_display": "$66000–$66500",
    "invalidation_display": "$62800",
    "rr_display": "R:R 1 : 2.50",
    "rr_available": true, "rr_value": 2.5, "rr_reason": null,
    "rationale": "TrendContinuation: preconditions 3/3"
  },
  "no_clear_card": null,
  "safety_flags": {
    "readiness": "READY",
    "rr_available": true, "rr_value": 2.5, "rr_reason": null,
    "stop_loss_pct": 0.01, "confidence_pct": 72,
    "entry_danger_score": 35, "entry_danger_level": "LOW",
    "rr_display": "R:R 1 : 2.50",
    "stop_loss_display": "1.00%", "confidence_display": "72%",
    "entry_danger_display": "35 (LOW)"
  },
  "why_note": null,
  "why": [
    "Bullish bias, confluence score 62 (L2 tradability_dim + L3 quality + L4 opportunity)",
    "Setup: TrendContinuation (L4 score 78, Strong)",
    "Trade readiness = READY (entry_danger 35)"
  ],
  "price_levels": {
    "side": "long",
    "entry_zone": { "low": 63200, "high": 63400 }, "target_zone": { "low": 66000, "high": 66500 },
    "invalidation": 62800, "horizon": "SWING", "hold_placeholder": null
  },
  "strategy": {
    "entry": "Pullback", "exit": "Trend Weakening",
    "protection": "ATR-Based", "target": "Resistance-Based"
  },
  "final_verdict": "Long on pullback toward the 63200-63400 entry zone with invalidation below 62800."
}
```

Notes:
- `rr_display` in `top_setup` derives from the canonical wire `rr_value`
  (`long_/short_expected_rr_internal`, target-mid geometry) with the same
  `R:R 1 : X.Y` formatting as the header chip and the safety-flags KPI —
  the payload never recomputes an independent geometry (a legacy
  `computeRiskReward` recompute here disagreed with the chip and the
  cards; see the export-consistency regression tests).
- `safety_flags.*_display` are verbatim KPI-chip strings (`R:R 1 : 2.50`, `1.00%`, `72%`, `35 (LOW)`).
- Missing `entry_danger` reads 50 (MODERATE) exactly like the panel.
- `top_setup_empty_text` carries the section-meta caption the panel renders when no qualifying setup exists (`"no qualifying setup yet"`); it is `null` whenever a setup renders.
- The four `strategy` fields render the screen's `"—"` placeholder when the advisory guidance is absent.
- The `no_clear_card` renders only when the top setup is absent AND the primary opportunity is NoClearOpportunity. When present it carries the `title` / `body` strings:

```json
{
  "no_clear_card": {
    "title": "No Clear Setup",
    "body": "Neutral — no directional edge: NEUTRAL bias with 11% confidence, cautious stance in a mean-reversion environment. No clear opportunity. Entry: on breakout. Stop: structure-based."
  }
}
```

### 3.8 Charts Sub-Tabs — `source_tab: "positions" | "orders" | "history" | "plan"`

These use the legacy envelope (`exported_at`, `symbol`, `mark_price`, `tf_secs`) plus `account` / `counts` blocks and per-tab arrays (`position`+`slots`+`brackets`, `open_orders`, `history`, `targets`+`stop`+`plan_visible`). Shapes unchanged since 6.x — see the previous revision of this document for the full examples.

Notes (v7.0-verify):
- Orders/brackets/counts read `AppStore.openOrders` — the same array the console table renders (`paper.openOrders` is legacy and never written).
- `position.liq_price` uses the shared `calcLiqPrice` from `lib/telemetry.ts` — identical to the console cell, including the `SHORT` + leverage-1 `2× entry` case.
- `history[].symbol` is the raw wire string and `null` when absent (the screen renders `"—"`; there is no activeTab fallback).
- `buildPlanTabExport(app, planOverride?)` accepts the console's currently-edited plan rows so the exported targets/stop mirror what the user sees in the inputs (edits are local state, not written back to `app.activePlan`).
- The console's `fmtTs` renders 24-hour time (`hour12: false`), byte-identical to the `*_display` strings in the payload.

---

## 4. Migration Notes

The legacy `buildMetricsExportJson` / `buildPanelExportJson` functions in `lib/metricsExport.ts` continue to produce the "kitchen-sink" payload (every matrix in one JSON). External consumers depending on that shape can keep using those functions. The new panels route to the per-tab builders via `buildXxxTabExport(args)` directly.

The v7.0 payload shapes are **not** backwards compatible with the legacy kitchen-sink:
- The **MTF** export added a top-level `filter_state` block + per-row `visible` flags (v7.0-verify); the single-TF **Metrics** export later gained the same top-level `filter_state` block (payload rows stay the unfiltered superset) so each tab's on-screen row set is reconstructible.
- A single `meta.current_price` replaced the multiple price mirrors.
- Screen sentences are exposed as `*_display` fields alongside raw numerics.
- Ladder clusters are canonicalized to top-4 by magnet strength everywhere (strip, Levels facet, export).
- Empty/null states mirror the screen placeholders exactly (`"—"`, `null`, `AWAITING` / `NO DATA` tokens) — the export never fabricates values the screen does not show.

---

## 5. Test Coverage

| Builder | Test file | Tests |
|---|---|---|
| `shared.ts` | `shared.test.ts` | 10 |
| `chartsTab.ts` | `chartsTab.test.ts` | 22 |
| `riskTab.ts` | `riskTab.test.ts` | 6 |
| `opportunityTab.ts` | `opportunityTab.test.ts` | 11 |
| `alignmentTab.ts` | `alignmentTab.test.ts` | 9 |
| `analysisTab.ts` | `analysisTab.test.ts` | 9 |
| `recommendationTab.ts` | `recommendationTab.test.ts` | 9 |
| `metricsTab.ts` | `metricsTab.test.ts` | 14 |
| `mtfTab.ts` | `mtfTab.test.ts` | 4 |
| `BottomConsole.test.ts` | (component) | 5 |
| **Export-consistency harness** | `tests/exportConsistency/exportConsistency.test.ts` (renders each MME panel, presses EXPORT DATA, cross-checks DOM vs JSON both directions) | 12 |
| **Total** | | **111** |

The harness (`ui/src/tests/exportConsistency/`) is the enforcement mechanism for the "export == screen" contract: it renders every Market Monitoring panel with a rich synthetic store state, captures the clipboard JSON, and asserts both directions — every displayed number/string/word is present in the JSON, and every exported display string maps back to the screen.
