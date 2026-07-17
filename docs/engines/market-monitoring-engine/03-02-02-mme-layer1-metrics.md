# MME Layer 1 — Metrics Layer

**Version:** 6.2 (2026-07-17) — see docs/CHANGELOG.md for the canonical version history.
**Status:** Approved
**Engine:** Market Monitoring Engine (MME)
**Layer:** 1 of 7
**Output Contract:** [Metrics Matrix](../../matrices/02-07-metrics-matrix.md)
**Purpose:** This document specifies the Metrics Layer — the process by which the MME computes technical indicators and signals for a single timeframe and projects each onto its standardized Evaluation Axes.

---

## 1. Purpose

The Metrics Layer is the first analytical transformation: it converts a completed OHLCV candle plus indicator buffers into a fully contextualized [Metrics Matrix](../../matrices/02-07-metrics-matrix.md). It projects each indicator across its **8 Indicator Evaluation Axes** and each detected signal across its **10 Signal Evaluation Axes**.

```
[completed candle + buffers]
   │
   ▼
indicator calculators ──► NormalizationEngine ──► signal detectors ──► MarketContext
   │                                                                        │
   └──────────────────► [Metrics Matrix] ◄──────────────────────────────────┘
```

Implementation: `analyzer/mod.rs::run_single()`, `analyzer/normalize.rs::build_indicator_map()`.

---

## 2. Stage 1 — Indicator Computation

Every registry-enabled indicator calculator runs against the current candle buffers, producing a native `raw_value` (and auxiliary component lines for multi-line indicators such as MACD, Bollinger, ADX). The 50 indicators span eight functional groups:

| Group | Examples |
|-------|----------|
| Trend | EMA ribbon, Supertrend, Donchian, Keltner, ADX, VWAP, Ichimoku, PSAR, Hull MA |
| Momentum | RSI, Stochastic, ChandeMO, Williams %R, Awesome Oscillator, CCI, MACD |
| Volume | Volume, RVOL, Volume Profile, OBV, CMF, MFI, Force Index |
| Volatility | ATR, Bollinger, BBWP, TTM Squeeze, HV, StdDev Channel |
| Structure | Fibonacci, Support/Resistance, Pivot Points, Chart Patterns, Candlestick |
| Regime | Aroon, Choppiness, LinReg Slope, Z-Score |
| Institutional | SMC Structure, Liquidity, FVG, Order Blocks |
| DerivativesData | Open Interest, OI Delta, Funding Rate, OI-Price Divergence, Order Flow Imbalance, Spread, Depth Bias |

See [indicators/index.md](indicators/04-02-00-indicator-index.md) for the authoritative manifest.

---

## 3. Stage 2 — Normalization

The `NormalizationEngine` (`crates/market-analyzer/src/indicators/normalized/`) maps each raw value to a continuous `normalized ∈ [-1.0, 1.0]` score and a context-aware `state_label`. Normalization is **regime-aware**: thresholds shift with market context (e.g. RSI overbought tightens to 80 in a strong trend).

Each indicator becomes an `IndicatorEvaluation` (`NormalizedIndicatorValue`) carrying `raw_value`, `normalized`, `state_label`, optional `values`, `signals`, and `confidence`. See [Metrics Matrix §3](../../matrices/02-07-metrics-matrix.md).

> **Target Architecture (Not Yet Implemented).** To eliminate slow string-hashing lookups (e.g. `map.get("rsi")`), the hot-path Metrics Matrix is intended to be a flat, cache-aligned struct indexed by a compiled `Enum` offset rather than a `HashMap<String, …>`:
>
> ```rust
> #[repr(C)]
> pub struct MetricsMatrix {
>     pub indicators: [IndicatorEvaluation; 50], // indexable by an Enum offset
>     pub timestamp: u64,
>     pub close: f64,
> }
> ```
>
> All 50 technical calculators (RSI, ATR, MACD, …) would execute their smoothing and crossovers using raw `f64` primitives, enabling CPU SIMD auto-vectorization. *Current implementation:* indicators are stored in `MarketSnapshot.indicators: HashMap<String, NormalizedIndicatorValue>` and most calculators compute in `rust_decimal::Decimal`.

---

## 4. Stage 3 — Signal Detection

Signal detectors project discrete events onto the 12 `SignalKind`s. Detection is stateful — crossovers and zero-line crosses require the `PreviousBarState` for reference.

| Detector family | SignalKinds produced |
|-----------------|----------------------|
| Divergence engine (8 indicators) | `Divergence` |
| Crossover detectors | `Crossover`, `ZeroLineCross`, `StackChange` |
| Threshold detectors | `Threshold` |
| Structural detectors | `Breakout`, `BandTouch`, `LevelTest`, `TrendFlip` |
| Volatility / volume | `CompressionRelease`, `VolumeClimax` |
| Pattern detectors | `PatternForming` |

Each `IndicatorSignal` carries `kind`, `direction`, `status`, `label`, `strength`, and `age_bars` (stamped by the stateful ager). See [Signals Guide](03-02-10-mme-signals-guide.md).

---

## 5. Stage 4 — Evaluation-Axis Projection

The layer projects flat telemetry onto the standardized axes defined in the [Ontology](../../conceptual-foundations/01-01-ontology.md):

### 5.1 Indicator Axes (8)
Value · State · Direction · Strength · Market Regime · Confidence · Freshness · Quality.

### 5.2 Signal Axes (10)
Signal Type · Direction · Strength · Confidence · Freshness · Confirmation · Market Regime · Multi-Timeframe Agreement · Risk · Priority.

The mapping from struct fields to axes is defined in [Metrics Matrix §3.2 / §4](../../matrices/02-07-metrics-matrix.md).

---

## 6. Stage 5 — Local Confluence (MarketContext)

`MarketContext::synthesize()` aggregates the indicator map into per-timeframe dimensions (trend, momentum, volatility, volume, liquidity), classifies the regime, and computes an `overall_score ∈ [-100, 100]`. This is **local confluence** — single-timeframe consensus — distinct from cross-timeframe alignment (Layer 2).

Regime rule (local 4-state `MarketContext.regime` vocabulary; see [Metrics Matrix §5.0](../../matrices/02-07-metrics-matrix.md) for the cross-layer mapping to the canonical 8-state L3 `MarketRegime`):
```
bbwp ≤ 15 OR chop ≥ 61.8 → COMPRESSION
bbwp ≥ 85                → EXPANSION
adx ≥ 25 OR chop ≤ 38.2  → TRENDING
else                     → RANGE
```

> **Cross-layer vocabulary.** Note that `MarketContext.regime` is the **local 4-state** vocabulary (`COMPRESSION` / `EXPANSION` / `TRENDING` / `RANGE`). The cross-TF canonical `MarketRegime` at L3 uses the **8-state** vocabulary (`TRENDING_BULL` / `TRENDING_BEAR` / `RANGE` / `ACCUMULATION` / `DISTRIBUTION` / `EXPANSION` / `CONTRACTION` / `TRANSITION`). The two are linked by the mapping table in [02-07-metrics-matrix.md §5.0](../../matrices/02-07-metrics-matrix.md); comparisons across layers must go through the L3 Analysis Matrix.
>
> **Layer-specific BBWP thresholds (intentional divergence).** The Layer 1 local 4-state regime uses a slightly looser compression threshold (`bbwp ≤ 15`) and expansion threshold (`bbwp ≥ 85`) than the L3 Analysis Matrix (`bbwp ≤ 10` for `CONTRACTION`, `bbwp ≥ 85` for `EXPANSION` — see [02-02-analysis-matrix.md §3.2](../../matrices/02-02-analysis-matrix.md)). The two thresholds serve different purposes: Layer 1's local `MarketContext.regime` is a coarse 4-state approximation for the chart-side composite that should flag borderline conditions early, while the L3 regime is the cross-symbol classifier that feeds the Decision Layer. A value in `[10, 15]` is therefore classified as `COMPRESSION` at Layer 1 but as `RANGE` (not `CONTRACTION`) at Layer 3 — both states are valid for their respective layers. Operators who rely on the local 4-state should treat `[10, 15]` as 'borderline compression' rather than canonical `CONTRACTION`.

---

## 7. Output & Freshness

- **Completed candle** → full Metrics Matrix (feeds Layers 2–7).
- **Live tick** → shadow snapshot (`is_completed = false`) for real-time display only.
- Signal `age_bars` increments per completed bar, driving the Freshness axis.

---

## 8. Guarantees

| Property | Guarantee |
|----------|-----------|
| **Completeness** | Every registry-enabled indicator is present (neutral if data-starved). |
| **Determinism** | Identical buffers + prior-bar state → identical Metrics Matrix. |
| **Regime awareness** | Normalization thresholds adapt to the active regime. |
| **Explainability** | Every signal carries its label, status, and strength. |

### 8.1 Configurable activation (Active Set)

The layer's indicator/signal computation is driven by a config-derived **Active Set**: registry defaults minus the union of the global `[activation]` denylist and the per-instance `[instances.*.activation]` denylist. Disabled indicators/signals are **absent** from the produced `MarketSnapshot` — never null, never tombstoned — and downstream layers reuse the existing NO_DATA/empty-state machinery with no new special cases. The active set is *recorded* on the snapshot via the optional `metrics_config` block (omitted when the active set equals the registry default, so default-path frames remain byte-identical to pre-feature frames). The 50-indicator / 12-SignalKind / 100-declaration **registry** describes capability and never changes with config; activation is a runtime config concern. Canonical spec, wire contract, downstream degradation rules, and the registry-invariance requirement are in [03-02-12-mme-configurable-activation.md](../../engines/market-monitoring-engine/03-02-12-mme-configurable-activation.md).

---

## 9. Phase 0-3 Extensions: Derivatives Telemetry

The Phase 0-3 Liquidity Intelligence extension adds a parallel
**derivatives telemetry** stream that runs alongside the price
indicators:

| New indicator group | Source | Field on `MarketSnapshot` |
|---|---|---|
| `mark_index_spread` | mark + index prices | `mark_index_spread_pct` |
| Real OI (Hyperliquid via activeAssetCtx) | `MetaAndAssetCtxs` polling | `open_interest`, `oi_delta_1h` |
| Real funding rate | WS push | `funding_rate` |
| Real liquidation events (Phase 1) | `userFills` / `fill` channel | `liquidity: LiquidityFlow` |
| Estimated heatmap (Phase 2) | Cluster estimator | `cluster: LiquidationClusterMatrix` |
| Liquidity signals (Phase 3) | Signal derivation | `liquidity_signals: Vec<LiquiditySignal>` |

> **Liquidity fields are part of Metrics Layer L1, not an independent matrix or layer.** The `liquidity`, `cluster`, and `liquidity_signals` fields on `MarketSnapshot` are computed by the same indicator-and-signal pipeline that produces all 50 indicators. The Liquidity Intelligence extension (Phases 0-4) adds computation to Layer 1 — it does not introduce a new engine layer, a separate tab, or a standalone UI component. In the frontend, liquidation cluster data renders inline on the Charts tab alongside price candlesticks and indicator panes, never as a standalone view.

These are not "indicators" in the strict sense (they are not
normalised f64 signals in the indicator map) — they are
*telemetry matrices* that ride the MarketSnapshot as optional
fields and are consumed by L1.5, L2.5, L5, and the UI.

## 10. Cross-References

- [Metrics Matrix](../../matrices/02-07-metrics-matrix.md) — Output contract.
- [Indicators Guide](03-02-09-mme-indicators-guide.md) · [Signals Guide](03-02-10-mme-signals-guide.md)
- [MME Layer 2 — Alignment](03-02-03-mme-layer2-alignment.md) — Direct consumer.
- [Liquidity Extension](03-02-11-mme-liquidity-extension.md) — Derivatives telemetry.
- [LiquidityMatrix](../../matrices/02-12-liquidity-matrix.md) · [ClusterMatrix](../../matrices/02-13-liquidation-cluster-matrix.md)
