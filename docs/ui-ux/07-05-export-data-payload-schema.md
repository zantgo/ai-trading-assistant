# Export Data Payload Schema

**Version:** 6.9 (2026-08-04) — see docs/CHANGELOG.md for the canonical version history.
**Status:** Approved
**Purpose:** This document specifies the JSON payload produced by every panel's `Export Data` button. Each panel's export mirrors 1:1 the data the panel renders — no more, no less. Consumers (AI agents, downstream services, debugging tools) can rely on the field shapes documented here.

---

## 1. Architecture Overview

Each panel has a **scoped builder** that emits a JSON payload matching the panel's rendered surface. The legacy "kitchen-sink" (`buildMetricsExportJson`) is preserved in `lib/metricsExport.ts` for backward compatibility but is no longer called by any panel.

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

The shared envelope (meta, filter state, account block, counts) is in `lib/exportBuilders/shared.ts`.

---

## 2. Shared Envelope

### 2.1 MetaEnvelope

Every payload has a `meta` (or top-level) block with:

```json
{
  "source_tab": "metrics",
  "exported_at": "2026-07-31T12:34:56.789Z",
  "symbol": "BTC-USDT",
  "tf_secs": 60,
  "timestamp": 1753950000,
  "mark_price": 65000.00,
  "is_completed": true,
  "pipeline_state": "LIVE",
  "filter_state": {
    "active_only": false,
    "confirmed_plus_only": false,
    "hide_gates": false,
    "hide_overlays": false
  }
}
```

### 2.2 AccountBlock (Charts sub-tabs only)

```json
{
  "balance": 10000.00,
  "available": 9500.00,
  "margin_used": 320.00,
  "leverage": 10
}
```

### 2.3 CountsBlock (Charts sub-tabs only)

```json
{
  "positions": 1,
  "open_orders": 3,
  "history": 12
}
```

---

## 3. Per-Tab Payload Schemas

### 3.1 Metrics Tab (single-TF) — `source_tab: "metrics"`

Mirrors `TerminalMonitor.svelte` single-TF mode (rows 1–4 + 4 facet tabs).

```json
{
  "source_tab": "metrics",
  "meta": { ... },
  "market_context": {
    "regime": "TRENDING_BULL",
    "overall_score": 0.5,
    "overall_label": "BULLISH",
    "trend":        { "score": 0.6, "confidence": 0.8, "label": "BULLISH" },
    "momentum":     { "score": 0.5, "confidence": 0.7, "label": "BULLISH" },
    "volatility":   { "score": 0.2, "confidence": 0.6, "label": "NEUTRAL" },
    "volume":       { "score": 0.4, "confidence": 0.7, "label": "BULLISH" },
    "liquidity":    { "score": 0.3, "confidence": 0.6, "label": "NEUTRAL" },
    "signal_count": 4,
    "age_bars": null
  },
  "group_confluence": [
    {
      "group": "MOMENTUM",
      "total": 5, "gates": 1, "bullish": 3, "bearish": 1, "neutral": 0,
      "active": 2, "active_signals": 4,
      "dominant": "bull",
      "dots": ["bull", "bull", "neutral", "neutral", "neutral"]
    }
  ],
  "structural_anchors": {
    "fibonacci": {
      "present": true,
      "gp_top": 66000, "gp_bottom": 64000,
      "ext_1618": 68000, "ext_2618": 70000,
      "retracement_coefficients": {
        "fib_0236": 65100, "fib_0382": 64800, "fib_0500": 64500,
        "fib_0618": 64500, "fib_0660": 64400, "fib_0786": 64200
      }
    },
    "volume_profile": { /* VolumeProfileExport — see §3.9 */ },
    "liquidation_clusters": {
      "top_short": [ { "peak_price": 55000, "distance_from_mid_pct": 0.1, "notional_usd": 1000000, "magnet_strength": 80, "cluster_kind": "short" } ],
      "top_long":  [ { "peak_price": 45000, "distance_from_mid_pct": 0.1, "notional_usd": 1000000, "magnet_strength": 70, "cluster_kind": "long" } ]
    },
    "cascade_alert": { "state": "DETECTED", "intensity": 65 }
  },
  "indicators": [
    {
      "key": "rsi",
      "display_name": "RSI",
      "group": "MOMENTUM",
      "class": "LEADING",
      "raw": 65,
      "normalized": 0.3,
      "state": "BULLISH",
      "pending_candle": false,
      "confidence_pct": 75,
      "signals": [
        { "kind": "CRO", "direction": "BULLISH", "status": "ACTIVE", "label": "RSI cross up", "strength": 0.8, "age_bars": 2 }
      ],
      "sub_values": null,
      "indicator_lifecycle": { "state": "LIVE", "bars_seen": 100, "bars_required": 14 }
    }
  ],
  "signals_total": 4,
  "signals_by_kind": {
    "DIVERGENCE": [], "CROSSOVER": [ /* ... */ ], "THRESHOLD": [], "BREAKOUT": [],
    "BAND_TOUCH": [], "ZERO_LINE_CROSS": [], "COMPRESSION_RELEASE": [], "LEVEL_TEST": [],
    "TREND_FLIP": [], "VOLUME_CLIMAX": [], "STACK_CHANGE": [], "PATTERN_FORMING": []
  },
  "divergences": [
    {
      "indicator_key": "rsi",
      "display_name": "RSI",
      "sub_kind": "Regular Bull",
      "direction": "BULLISH",
      "status": "CONFIRMED",
      "strength": 0.85,
      "confidence_pct": 80,
      "age_bars": 3,
      "label": "RSI Bull Divergence",
      "pivots": [
        { "time": 1753950000, "value": 30.5 },
        { "time": 1753950600, "value": 25.2 }
      ]
    }
  ],
  "levels": [
    {
      "indicator_key": "pivot_points",
      "display_name": "Pivot Points",
      "level_name": "R1",
      "kind": "PIVOT",
      "role": "resistance",
      "price_text": "65100",
      "direction": "BULLISH",
      "status": "ACTIVE",
      "strength": 0.7,
      "confidence_pct": 80,
      "age_bars": 0
    }
  ],
  "liquidity_signals": [
    { "kind": "CASCADE", "direction": "BULLISH", "strength": 0.8, "confidence": 0.7, "evidence": ["Liq surge"] }
  ],
  "liquidity_flow": { /* LiquidityFlowExport — see §3.10 */ },
  "cluster_matrix": { /* ClusterMatrixExport — see §3.11 */ }
}
```

### 3.2 MTF Tab — `source_tab: "mtf"`

Mirrors `MtfView.svelte` (4 × N grid with per-row agreement).

```json
{
  "source_tab": "mtf",
  "meta": { ... },
  "groups": [
    { "key": "MOMENTUM", "label": "MOMENTUM", "accent": "#a78bfa", "indicator_count": 4 }
  ],
  "indicators": [
    {
      "key": "rsi",
      "display_name": "RSI",
      "group": "MOMENTUM",
      "directional": true,
      "values": [
        { "timeframe": "MICRO", "normalized": 0.5, "active": true },
        { "timeframe": "FAST",  "normalized": 0.3, "active": true },
        { "timeframe": "SLOW",  "normalized": -0.1, "active": true },
        { "timeframe": "MACRO", "normalized": -0.5, "active": true }
      ],
      "agreement": 0.05,
      "agreement_label": "MIXED"
    }
  ],
  "timeframes": [
    {
      "label": "MICRO",
      "duration_seconds": 60,
      "mark_price": 65000,
      "timestamp": 1753950000,
      "pipeline_state": "LIVE",
      "is_completed": true,
      "context": null,
      "fibonacci_summary": { /* same shape as Metrics.indicators[].sub_values for fib */ },
      "indicators": [ /* same shape as Metrics.indicators[] */ ],
      "liquidity_signals": [],
      "volume_profile": null,
      "liquidity_flow": null,
      "cluster_matrix": null
    }
  ],
  "signals_total": 4
}
```

### 3.3 Alignment Tab — `source_tab: "alignment"`

```json
{
  "source_tab": "alignment",
  "meta": { ... },
  "hero": {
    "mtf_overall_score": 40.0,
    "mtf_overall_label": "WEAK_BULL_MTF",
    "timeframes_present": 4,
    "signal_cross_tf_count": 5,
    "trend_agreement_pct": 75
  },
  "dimensions": [
    { "name": "TREND",       "score": 75, "state": "BULLISH", "confidence": 80 },
    { "name": "MOMENTUM",    "score": 60, "state": "BULLISH", "confidence": 70 },
    { "name": "VOLUME",      "score": 50, "state": "NEUTRAL", "confidence": 65 },
    { "name": "VOLATILITY",  "score": 40, "state": "NEUTRAL", "confidence": 60 },
    { "name": "STRUCTURE",   "score": 55, "state": "BULLISH", "confidence": 70 },
    { "name": "SIGNAL",      "score": 65, "state": "BULLISH", "confidence": 75 },
    { "name": "REGIME",      "score": 70, "state": "BULLISH", "confidence": 80 },
    { "name": "CONFIDENCE",  "score": 80, "state": "BULLISH", "confidence": 85 },
    { "name": "LIQUIDITY",   "score": 45, "state": "NEUTRAL", "confidence": 60 },
    { "name": "TRADABILITY", "score": 60, "state": "BULLISH", "confidence": 70 }
  ],
  "consensus": {
    "trend_agreement_pct": 75,
    "label": "strong_consensus",
    "polarization": [
      { "key": "T",  "label": "TREND",      "value": 0.5 },
      { "key": "M",  "label": "MOMENTUM",   "value": 0.3 },
      { "key": "VT", "label": "VOLUME",     "value": 0.1 },
      { "key": "VM", "label": "VOLATILITY", "value": 0.2 }
    ]
  },
  "per_timeframe": [
    { "timeframe": "MICRO", "trend_score": 0.5, "momentum_score": 0.3, "overall_score": 30, "regime": "TRENDING_BULL", "active_signals": 3 }
  ],
  "score_calculation": {
    "weights": [
      { "key": "T",  "label": "TREND",      "pct": 50, "color": "#22c55e", "value": 0.5, "contribution": 0.25 },
      { "key": "M",  "label": "MOMENTUM",   "pct": 30, "color": "#3b82f6", "value": 0.3, "contribution": 0.09 },
      { "key": "VT", "label": "Vol.trend",  "pct": 10, "color": "#a78bfa", "value": 0.1, "contribution": 0.01 },
      { "key": "VM", "label": "Vol.market", "pct": 10, "color": "#f59e0b", "value": 0.2, "contribution": 0.02 }
    ],
    "formula": "0.5 * (0.50) + 0.3 * (0.30) + 0.1 * (0.10) + 0.1 * (0.20) = 40.0"
  },
  "interpretation": "Multi-timeframe alignment shows strong directional consensus..."
}
```

### 3.4 Opportunity Tab — `source_tab: "opportunity"`

```json
{
  "source_tab": "opportunity",
  "meta": { ... },
  "header": {
    "opportunity_class": "TREND_CONTINUATION",
    "lean": "bullish_setups_dominate",
    "setup_score": 78,
    "setup_quality": "STRONG"
  },
  "trade_setups": [
    {
      "opportunity_type": "TREND_CONTINUATION",
      "side": "LONG",
      "rank_idx": 0,
      "is_top": true,
      "geometry_consistent": true,
      "entry_mid": 64250,
      "entry_zone": { "low": 64000, "high": 64500 },
      "tp1": 66000, "tp2": 67000,
      "invalidation": 63000,
      "rr": 2.5,
      "score": 78,
      "preconditions_met": 4,
      "preconditions_total": 5,
      "notes": "Trend alignment strong"
    }
  ],
  "rr_internal": { "expected_rr": 2.5, "time_horizon": "INTRADAY" },
  "invalidation_note": "Below 63000 invalidates the setup",
  "evaluated_setups": [
    { "opportunity_type": "TREND_CONTINUATION", "score": 78, "preconditions_met": 4, "preconditions_total": 5, "notes": "Trend alignment strong" },
    { "opportunity_type": "BREAKOUT",          "score": 65, "preconditions_met": 3, "preconditions_total": 5, "notes": "Watching for breakout above 65k" },
    { "opportunity_type": "NO_CLEAR_OPPORTUNITY", "score": 20, "preconditions_met": 0, "preconditions_total": 5, "notes": "" }
  ],
  "confluent_entry_levels": [
    { "price": 64000, "sources": ["FIBONACCI", "VOLUME_PROFILE"], "strength": 85 }
  ],
  "confluent_target_levels": [
    { "price": 67000, "sources": ["FIBONACCI"], "strength": 70 }
  ],
  "market_position": {
    "bias": "BULLISH",
    "regime": "TRENDING_BULL",
    "trend": "HEALTHY",
    "quality": "GOOD"
  },
  "environment": {
    "timeframes_considered": 4,
    "confidence_pct": 70
  }
}
```

### 3.5 Risk Tab — `source_tab: "risk"`

```json
{
  "source_tab": "risk",
  "meta": { ... },
  "hero": {
    "overall_score": 65,
    "overall_level": "HIGH",
    "overall_state": "STABLE",
    "overall_confidence": 80,
    "top_severity": "EXTREME",
    "ring_pct": 65
  },
  "summary_counts": {
    "very_low": 1, "low": 2, "moderate": 3, "high": 1, "extreme": 1
  },
  "dimensions": [
    {
      "name": "Cascade Risk",
      "key": "cascade_risk",
      "weight": 0.14, "weight_pct": 14,
      "score": 90, "level": "EXTREME", "state": "CRITICAL",
      "confidence": 90,
      "evidence": ["$50M long liquidation event"],
      "bar_pct": 90, "weight_mark_pct": 14,
      "is_cascade_dim": true
    }
  ],
  "cascade_telemetry": {
    "cascade_state": "DETECTED",
    "cascade_intensity": 65,
    "cascade_asymmetry": 0.3
  },
  "interpretation": "1 extreme · 1 high · 3 moderate · overall high"
}
```

### 3.6 Analysis Tab — `source_tab: "analysis"`

```json
{
  "source_tab": "analysis",
  "meta": { ... },
  "header": {
    "bias": "BULLISH",
    "confidence": 0.72,
    "state_confidence": 0.72,
    "market_regime": "TRENDING_BULL",
    "market_quality": "GOOD"
  },
  "signals": {
    "supporting": [
      { "raw": "[MICRO] BULLISH regime score +60 — 4 signals", "timeframe": "MICRO", "score": 60, "regime": "BULLISH", "signals_count": 4 }
    ],
    "contradicting": [
      { "raw": "[MACRO] BEARISH regime score -30 — 2 signals", "timeframe": "MACRO", "score": -30, "regime": "BEARISH", "signals_count": 2 }
    ],
    "lean": { "label": "Net bullish · 2↑ vs 1↓", "bullish": 2, "bearish": 1, "tone": "bull" }
  },
  "qualitative_assessment": {
    "trend": "HEALTHY",
    "momentum": "INCREASING",
    "structure": "STRONG",
    "volatility": "NORMAL",
    "volume": "STRONG",
    "cycle_phase": "MARKUP"
  },
  "per_timeframe_alignment": [
    { "name": "MICRO", "active": true, "trend": 0.5, "momentum": 0.3, "overall": 30, "regime": "TRENDING_BULL" },
    { "name": "FAST",  "active": true, "trend": 0.4, "momentum": 0.2, "overall": 20, "regime": "TRENDING_BULL" },
    { "name": "SLOW",  "active": true, "trend": 0.1, "momentum": 0.0, "overall": 10, "regime": "RANGE" },
    { "name": "MACRO", "active": true, "trend": -0.1, "momentum": -0.2, "overall": -10, "regime": "TRENDING_BEAR" }
  ],
  "interpretation": "Trend is healthy with momentum increasing",
  "rationale": "Multi-timeframe alignment supports the bullish bias"
}
```

### 3.7 Recommendation Tab — `source_tab: "recommendation"`

```json
{
  "source_tab": "recommendation",
  "meta": { ... },
  "environment": {
    "directional_guidance": "LONG",
    "market_stance": "CONSTRUCTIVE",
    "strategy_environment": "TREND_FOLLOWING",
    "opportunity_type": "TREND_CONTINUATION",
    "confidence_pct": 78,
    "readiness": "READY",
    "entry_danger": { "score": 30, "level": "LOW", "state": "STABLE", "confidence": 0.7 }
  },
  "verdict": {
    "top": "LONG",
    "top_prob_pct": 60,
    "headline": { "action": "LONG", "label": "LONG — READY", "state": "READY", "confidence_pct": 75 },
    "long_probability": 60,
    "short_probability": 25,
    "hold_probability": 15
  },
  "runner_ups": [
    { "action": "SHORT", "prob_pct": 25 },
    { "action": "HOLD",  "prob_pct": 15 }
  ],
  "top_setup": {
    "opportunity_type": "TREND_CONTINUATION",
    "score": 78,
    "preconditions_met": 4,
    "preconditions_total": 5,
    "direction": "long",
    "direction_label": "LONG",
    "entry_zone": { "low": 64000, "high": 64500 },
    "target_zone": { "low": 66000, "high": 67000 },
    "invalidation": 63000,
    "rr": 2.5,
    "notes": "Trend alignment strong"
  },
  "safety_flags": {
    "readiness": "READY",
    "internal_rr": 2.5,
    "risk_adj_rr": 2.5,
    "stop_loss_pct": 0.015,
    "confidence_pct": 78
  },
  "why": [
    "BULLISH bias, confluence score 50 ...",
    "Setup: TrendContinuation (L4 score 78, STRONG)",
    "Trade readiness = READY (entry_danger 30)"
  ],
  "price_levels": {
    "side": "long",
    "entry_zone": { "low": 64000, "high": 64500 },
    "target_zone": { "low": 66000, "high": 67000 },
    "invalidation": 63000,
    "horizon": "INTRADAY",
    "scenarios": null
  },
  "strategy": {
    "entry": "PULLBACK",
    "exit": "TREND_WEAKENING",
    "protection": "STRUCTURE_BASED",
    "target": "RRBased"
  },
  "final_verdict": "Long bias — structure-based entry with R:R 2.5"
}
```

### 3.8 Charts Sub-Tabs — `source_tab: "positions" | "orders" | "history" | "plan"`

#### Positions

```json
{
  "source_tab": "positions",
  "exported_at": "...",
  "symbol": "BTC-USDT",
  "mark_price": 65000.00,
  "active_view": "positions",
  "counts": { "positions": 1, "open_orders": 3, "history": 12 },
  "position": {
    "symbol": "BTC-USDT",
    "direction": "LONG",
    "size": 0.05,
    "average_entry_price": 64000,
    "liq_price": 57600,
    "mark_price": 65000,
    "margin_used": 320,
    "unrealized_pnl": 50,
    "unrealized_pnl_display": "+$50.00",
    "unrealized_roi_pct": 15.63,
    "opened_at": 1753950000,
    "leverage": 10
  },
  "slots": [
    {
      "slot_index": 1,
      "entry_price": 63500,
      "size": 0.025,
      "allocated_usd": 1587.50,
      "is_active": true,
      "mark_price": 65000,
      "pnl": 37.50,
      "status": "ACTIVE"
    }
  ],
  "brackets": {
    "take_profit": [
      { "id": 101, "order_type": "LIMIT", "price": 66000, "trigger_price": null, "size_pct": 50 }
    ],
    "stop_loss": [
      { "id": 103, "order_type": "STOP", "price": null, "trigger_price": 62500, "size_pct": 100 }
    ]
  },
  "account": { "balance": 10000, "available": 9500, "margin_used": 320, "leverage": 10 }
}
```

#### Orders

```json
{
  "source_tab": "orders",
  "exported_at": "...",
  "symbol": "BTC-USDT",
  "mark_price": 65000.00,
  "active_view": "orders",
  "counts": { "positions": 1, "open_orders": 1, "history": 12 },
  "open_orders": [
    {
      "id": 201, "order_type": "LIMIT", "direction": "BUY",
      "price": 63000, "trigger_price": null, "size_pct": 25,
      "created_at": 1753950120000, "created_at_display": "14:22"
    }
  ],
  "account": { /* AccountBlock */ }
}
```

#### History

```json
{
  "source_tab": "history",
  "exported_at": "...",
  "symbol": "BTC-USDT",
  "mark_price": 65000.00,
  "active_view": "history",
  "counts": { "positions": 1, "open_orders": 0, "history": 12 },
  "history": [
    {
      "exit_timestamp": 1753940000,
      "exit_timestamp_display": "11:33",
      "symbol": "BTC-USDT",
      "direction": "LONG",
      "entry_price": 63500, "exit_price": 66000,
      "realized_pnl": 125, "realized_pnl_display": "+$125.00",
      "roi_pct": 3.94,
      "trigger": "TP1"
    }
  ],
  "account": { /* AccountBlock */ }
}
```

#### Plan

```json
{
  "source_tab": "plan",
  "exported_at": "...",
  "symbol": "BTC-USDT",
  "mark_price": 65000.00,
  "active_view": "plan",
  "plan_source": "L4_opportunity_matrix",
  "plan_visible": true,
  "counts": { "positions": 1, "open_orders": 0, "history": 12 },
  "targets": [
    { "label": "TP1", "price": 66000, "size_pct": 40 },
    { "label": "TP2", "price": 68000, "size_pct": 35 },
    { "label": "TP3", "price": 70000, "size_pct": 25 }
  ],
  "stop": { "label": "SL", "price": 62800, "distance_pct": 1.0 },
  "account": { /* AccountBlock */ }
}
```

When no plan is loaded, `plan_visible: false`, `targets: []`, `stop: null`.

### 3.9 VolumeProfileExport

```json
{
  "symbol": "BTC-USDT",
  "timeframe_slot": "micro",
  "timeframe_secs": 60,
  "poc_price": 65000,
  "value_area_high": 66000,
  "value_area_low": 64000,
  "total_volume": 1000000,
  "range_low": 63000,
  "range_high": 67000,
  "num_bins": 30,
  "timestamp_ms": 1753950000,
  "top_hvn": [
    { "price_low": 64900, "price_high": 65000, "volume": 50000, "buy_volume": 30000, "sell_volume": 20000, "strength_x_mean": 1.5 }
  ],
  "buy_total": 250000,
  "sell_total": 200000,
  "buy_sell_bias": 0.1111,
  "current_position": { "in_va": true, "range_pos_pct": 50.0 }
}
```

### 3.10 LiquidityFlowExport

```json
{
  "long_liquidations_usd": 50000,
  "short_liquidations_usd": 10000,
  "net_liquidation_usd": 40000,
  "event_count": 3,
  "largest_event_usd": 30000,
  "largest_event_price": 49500,
  "largest_event_side": "LONG",
  "cascade_state": "DETECTED",
  "cascade_intensity": 65
}
```

### 3.11 ClusterMatrixExport

```json
{
  "mid_price": 50000,
  "cascade_asymmetry": 0.3,
  "total_long_oi_usd": 100000000,
  "total_short_oi_usd": 90000000,
  "estimation_confidence": 0.8,
  "leverage_assumptions": {
    "source": "default",
    "buckets": [1, 3, 5, 10, 20, 50, 100],
    "weights": [0.05, 0.1, 0.2, 0.3, 0.2, 0.1, 0.05],
    "funding_modulation_active": true
  },
  "top_above": [
    { "peak_price": 55000, "distance_from_mid_pct": 0.1, "notional_usd": 1000000, "magnet_strength": 80, "cluster_kind": "short" }
  ],
  "top_below": [
    { "peak_price": 45000, "distance_from_mid_pct": 0.1, "notional_usd": 1000000, "magnet_strength": 70, "cluster_kind": "long" }
  ]
}
```

---

## 4. Migration Notes

The legacy `buildMetricsExportJson` / `buildPanelExportJson` functions in `lib/metricsExport.ts` continue to produce the "kitchen-sink" payload (every matrix in one JSON). External consumers depending on that shape can keep using those functions. The new panels route to the per-tab builders via `buildXxxTabExport(args)` directly.

The new payload shapes are **not** backwards compatible with the legacy kitchen-sink. Any downstream service that reads `analysis.bias`, `risk.overall.score`, etc. from the panel's export must adapt to the new per-tab scoped fields (e.g. `header.bias`, `hero.overall_score`).

---

## 5. Test Coverage

| Builder | Test file | Tests |
|---|---|---|
| `shared.ts` | `shared.test.ts` | 16 |
| `chartsTab.ts` | `chartsTab.test.ts` | 19 |
| `riskTab.ts` | `riskTab.test.ts` | 9 |
| `opportunityTab.ts` | `opportunityTab.test.ts` | 11 |
| `alignmentTab.ts` | `alignmentTab.test.ts` | 9 |
| `analysisTab.ts` | `analysisTab.test.ts` | 9 |
| `recommendationTab.ts` | `recommendationTab.test.ts` | 13 |
| `metricsTab.ts` | `metricsTab.test.ts` | 13 |
| `mtfTab.ts` | `mtfTab.test.ts` | 7 |
| `BottomConsole.test.ts` | (component) | 5 |
| **Total new** | | **111** |

Each builder is also exercised through the panel's component test (where present) to verify the rendered DOM and the exported JSON are in sync.
