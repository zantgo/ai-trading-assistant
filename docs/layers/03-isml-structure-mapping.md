# ISML — Institutional Structure Mapping Layer

> **Layer 3 of 10 in the Institutional Trading Strategy Decision Pipeline.**
> **Implementation Status: COMPLETE** — S/R engine, Fibonacci, Volume Profile, SMC sub-engines all implemented.

---

## Purpose

The Institutional Structure Mapping Layer (ISML) answers the battlefield intelligence question:

> **Where are the institutional zones, levels, and structural boundaries?**

ISML constructs a complete map of the trading terrain. Before any position can be entered, the system must know: where is support? Where is resistance? Where are the institutional order blocks? Where is the Golden Pocket? Where has volume clustered? What is the market structure (trending or reversing)? What patterns are forming?

This layer provides the **contextual geography** that every downstream layer relies on — the ICSL scores proximity to levels, the IDCL computes recommended stops from the level hierarchy, the IEPL places entries and targets at specific levels, and the IASL Analyst Agent describes structure in natural language.

**ISML is purely descriptive. It maps the battlefield but never fires a shot.**

---

## Inputs

| Input | Source | Format |
|-------|--------|--------|
| OHLCV candles (5 TFs) | L0 — Candle Aggregator | `NormalizedCandle` histories |
| Pivot highs/lows | ITIL indicator `pivot_points` | Level markers |
| S/R indicator values | ITIL indicators `support_resistance`, `fibonacci` | NormalizedIndicatorValue with LevelTest signals |
| Volume Profile data | ITIL indicator `volume_profile` | POC/VAH/VAL values |
| SMC indicator values | ITIL indicators `smc_structure`, `smc_liquidity`, `smc_fvg`, `smc_order_blocks` | NormalizedIndicatorValue with Breakout/TrendFlip/LevelTest/PatternForming signals |
| Pattern detection | ITIL indicators `patterns`, `candlestick` | PatternForming signals |
| Price history | L0 | 100-bar close price buffer |

---

## Outputs

| Output | Format | Consumer |
|--------|--------|----------|
| Level map (ranked) | Ordered list of price levels with type, role, significance | IDCL (`recommended_stop`), IEPL (entry/TP/SL placement), ICSL (proximity scoring) |
| Structural integrity score | f64 ∈ [0, 1] | IDCL, IASL (structure section) |
| S/R role tracking | Per-level role (Support/Resistance), flip history, proximity | ICSL (directional context), IEPL (invalidation) |
| Active patterns | Pattern type, boundaries, breakout status | ICSL (pattern confirmation scoring), IASL (structure section) |
| SMC zone inventory | Active OB, unmitigated FVG, BOS/CHoCH status, sweep events | ICSL (institutional confluence), IEPL (stop placement priority) |

---

## Sub-Components

---

### A. Support & Resistance Engine

The S/R engine detects, tracks, and manages horizontal price levels using pivot-based detection and a role-reversal state machine.

**Detection:** Pivot highs and lows are identified from the OHLCV history using a configurable lookback. Levels where price has reversed at least twice are marked as S/R zones. The engine maintains both major (multi-touch) and minor (single-touch) levels.

**Role Tracking (SrRoleTracker):** Each detected level carries:
- `price`: The exact price of the level
- `role`: Current role — `Support` (price above, level below) or `Resistance` (price below, level above)
- `original_role`: Role at first detection (for context)
- `last_flip_timestamp`: When the level last changed roles
- `flip_count`: Number of times this level has flipped (high flip count = weaker level)

**Flip Detection:** A level flips roles when price closes decisively beyond it:
```
Long position context:
  Resistance → Support when: close > level × (1 + flip_tolerance)
  Support → Resistance when: close < level × (1 − flip_tolerance)
flip_tolerance = 0.3% (configurable)
```

A flip generates a `FlipEvent { level_price, from_role, to_role, candle_close, candle_timestamp }`.

**Merge Protocol:** When a new pivot is detected within `merge_tolerance` (default 0.5%) of an existing level, the new pivot is merged into the existing level rather than creating a duplicate. The existing level's flip memory is preserved, and its significance score increases.

**Proximity Scoring:** For each active level, compute proximity to current price:
```
proximity_pct = |level_price − current_price| / current_price
```
Levels within 0.5% of price → active confluence (strong signal). Levels within 1.0% → nearby (moderate). Levels > 2.0% → distant (weak).

**Role-Reversal Rules:**
- Resistance decisively broken above → flips to Support (prior resistance becomes floor)
- Support decisively broken below → flips to Resistance (prior support becomes ceiling)
- Flip tolerance of 0.3% prevents false flips from wicks and noise
- Flip events are tracked with timestamps; levels with frequent flips are demoted in the hierarchy

**Code location:** `crates/engine/src/sr_engine.rs` (SrRoleTracker), `crates/engine/src/server/math.rs` (compute_support_resistance).

---

### B. Fibonacci Framework

**Swing Detection:** Major swing highs and lows are automatically identified from pivot points. The most recent completed impulse leg (swing low → swing high for uptrend, swing high → swing low for downtrend) defines the Fibonacci anchor and terminal.

**Retracement Levels:**
| Level | Ratio | Role |
|-------|-------|------|
| 23.6% | 0.236 | Shallow pullback — weak trend |
| 38.2% | 0.382 | Moderate pullback — healthy trend |
| 50.0% | 0.500 | Equilibrium — decision zone |
| 61.8% | 0.618 | Golden Pocket entry — institutional accumulation/distribution |
| 66.0% | 0.660 | Golden Pocket lower boundary |
| 78.6% | 0.786 | Deep retracement — trend under threat |

**Golden Pocket (61.8% – 66.0%):** The zone between the 61.8% and 66.0% retracement is the highest-probability institutional entry zone. When price pulls back into this zone:
- **Bullish GP (Long):** Price pulls back from swing high into 61.8%-66.0%. Enter Long when RSI/Squeeze confirm reversal.
- **Bearish GP (Short):** Price rallies from swing low into 61.8%-66.0%. Enter Short when indicators confirm.
```
GP_Top = Anchor ± (D × 0.618)
GP_Bottom = Anchor ± (D × 0.660)
```

**Extension Targets:**
| Extension | Ratio | Role |
|-----------|-------|------|
| 1.272 | 127.2% | Primary algorithmic target |
| 1.618 | 161.8% | TP2 — sharp pauses/reversals cluster here |
| 2.618 | 261.8% | TP3 — ultimate parabolic climax target |

Extensions carry more weight when ATR volatility is EXPANDING (confirmed momentum).

---

### C. Volume Profile

100-bar rolling window analysis of volume distribution at price. 30-bin OHLCV-based histogram.

**Key Levels:**
- **POC (Point of Control):** Price level with the highest traded volume. Acts as a magnet level — price is drawn to it. When price is away from POC, expect return.
- **VAH (Value Area High):** Upper boundary of the 70% value area. Price above VAH = bullish breakout, market has moved outside accepted value.
- **VAL (Value Area Low):** Lower boundary of the 70% value area. Price below VAL = bearish breakdown.
- **HVN (High Volume Node):** Price cluster with significantly elevated volume — acts as support/resistance.
- **LVN (Low Volume Node):** Price cluster with minimal volume — price moves quickly through these zones (potential breakout acceleration).

**Classification Signals:**
- **Breakout:** Price closes above VAH (bullish) or below VAL (bearish)
- **LevelTest:** Price tests POC from either direction (potential support/resistance reaction)
- **Value Acceptance:** Price inside value area — equilibrium, trending less likely
- **Value Rejection:** Price closes outside after being inside — directional commitment

---

### D. Smart Money Concepts (SMC)

Four independent sub-engines analyze institutional order flow from OHLCV data.

**SMC Structure:**
- **BOS (Break of Structure):** Higher high after lower highs = bullish BOS. Lower low after higher lows = bearish BOS. BOS confirms the current trend structure. Signal: Breakout.
- **CHoCH (Change of Character):** Lower high after higher high in a bull trend = bearish CHoCH. Higher low after lower low in a bear trend = bullish CHoCH. CHoCH warns of potential trend reversal. Signal: TrendFlip.
- Structural context: is the market in a trending structure (series of BOS events) or a reversing structure (CHoCH event detected)?

**SMC Liquidity:**
- **Buy-Side Sweep:** Price wicks below a recent swing low, then closes ABOVE it → long stops hunted, bullish reversal expected. Signal: PatternForming (Bullish).
- **Sell-Side Sweep:** Price wicks above a recent swing high, then closes BELOW it → short stops hunted, bearish reversal expected. Signal: PatternForming (Bearish).
- Sweeps indicate where institutional stop-hunting has occurred, revealing potential reversal zones.

**SMC FVG (Fair Value Gap):**
- **Bullish FVG:** 3-candle gap where Low[3] > High[1] → bullish imbalance. The gap zone acts as a magnet — price expected to return and fill. Signal: LevelTest.
- **Bearish FVG:** 3-candle gap where High[3] < Low[1] → bearish imbalance.
- **Mitigated FVG:** Price has traded through the gap — imbalance resolved. A mitigated bullish FVG that later holds as support = inverse FVG (structural support formed).
- FVG lifecycle: Formation → Active (unmitigated) → Mitigated (price filled gap) → Inverse (role reversal).

**SMC Order Blocks:**
- **Bullish OB:** Last bearish candle before a bullish BOS = demand zone. Price returning here is a potential long entry. Signal: LevelTest (Bullish).
- **Bearish OB:** Last bullish candle before a bearish BOS = supply zone. Price returning here is a potential short entry. Signal: LevelTest (Bearish).
- **Mitigated OB:** Price closes beyond the OB zone → block broken. Mitigated bullish OB → potential resistance; mitigated bearish OB → potential support. Signal: TrendFlip (role inversion).
- OB lifecycle: Formation → Active → Tested → Mitigated → Role-Inverted.

**Signal summary:**

| SMC Key | Signals | SignalKinds |
|---------|---------|-------------|
| `smc_structure` | 2 | Breakout (BOS), TrendFlip (CHoCH) |
| `smc_liquidity` | 1 | PatternForming (sweeps) |
| `smc_fvg` | 1 | LevelTest (gap test) |
| `smc_order_blocks` | 1 | LevelTest (OB test, also TrendFlip on mitigation) |

---

### E. Chart Pattern Classification

Five chart patterns are detected mathematically using linear regression through pivot highs and lows.

| Pattern | Characteristics | Bias | Breakout Validation |
|---------|----------------|------|---------------------|
| **Symmetrical Triangle** | Highs descending, lows ascending — converging | Neutral (breakout direction = confirmation) | Close beyond regression line + RVOL ≥ 1.5 |
| **Rising Wedge** | Both slopes positive, lower boundary steeper | Bearish | Close below lower line + RVOL |
| **Falling Wedge** | Both slopes negative, upper boundary steeper | Bullish | Close above upper line + RVOL |
| **Ascending Channel** | Parallel positive slopes | Bullish | Buy near lower boundary, break below = invalidation |
| **Descending Channel** | Parallel negative slopes | Bearish | Short near upper boundary, break above = invalidation |

**Breakout Validation Rule:** A breakout without volume confirmation (RVOL < 1.5) is classified as a potential fakeout and is NOT treated as a confirmed pattern completion. This prevents false signals in low-participation environments.

---

### F. Structural Integrity Score

The structural integrity score quantifies how clean and well-defined the market structure is. A high score means clear levels, clean patterns, and unambiguous SMC zones.

```
integrity = 0.25 × level_clarity + 0.25 × pattern_confidence + 0.20 × smc_activity + 0.15 × trend_cleanliness + 0.15 × level_proximity
```

**Sub-factors:**

| Factor | Range | Formula |
|--------|-------|---------|
| `level_clarity` | [0, 1] | Fraction of detected levels with confidence above threshold; penalized by frequent flips |
| `pattern_confidence` | [0, 1] | Mean confidence of active chart patterns; 0 if no patterns detected |
| `smc_activity` | [0, 1] | Count of active (unmitigated) order blocks + FVGs; decayed if too many conflicting zones |
| `trend_cleanliness` | [0, 1] | 1 − choppiness_norm; clean trends have high score, choppy markets have low |
| `level_proximity` | [0, 1] | How close price is to the nearest significant level; too far = structure irrelevant |

**Interpretation:**
| Score | Label | Meaning |
|-------|-------|---------|
| > 0.75 | Clean Structure | Levels well-defined, patterns clear, SMC zones active. High confidence in structural analysis. |
| 0.50 – 0.75 | Adequate Structure | Some ambiguity but usable. Standard structural confidence. |
| 0.25 – 0.50 | Degraded Structure | Conflicting levels, unclear patterns, mitigated zones. Reduced confidence. |
| < 0.25 | Poor Structure | No clear levels, high noise. Avoid relying on structural analysis. |

---

### G. Level Hierarchy Engine

Not all levels are equal. The hierarchy engine ranks detected levels by institutional significance to determine which ones matter most for stop placement, entry timing, and target setting.

**Priority ranking (highest to lowest):**

| Rank | Level Type | Source | Use Case |
|------|-----------|--------|----------|
| 1 | **Bullish/Bearish Order Block** | SMC Order Blocks | Primary stop placement — institutional demand/supply zones |
| 2 | **Volume Profile POC** | Volume Profile | Entry confirmation, mean-reversion target |
| 3 | **Fibonacci Golden Pocket** | Fibonacci | Entry zone (61.8%-66.0%) |
| 4 | **Pivot S/R (multi-touch)** | Support/Resistance Engine | Secondary stop placement, TP1 target |
| 5 | **VWAP** | VWAP indicator | Intraday fair value, pullback entry |
| 6 | **Anchored VWAP** | Anchored VWAP | Multi-session institutional cost basis |
| 7 | **ATR-based level** | ATR × 2.0 (fallback) | Last-resort stop — always produces a value |

**Stop placement priority chain (used by IDCL `recommended_stop`):**
```
For Long:  Bullish OB low → Swing Low → VWAP → Volume VAL → Pivot S1 → ATR × 2.0
For Short: Bearish OB high → Swing High → VWAP → Volume VAH → Pivot R1 → ATR × 2.0
```

Each level in the chain is checked: is it below price (for Long) / above price (for Short)? If yes, use it. If no, move to the next level. The ATR × 2.0 fallback guarantees `recommended_stop` always returns a value.

**Proximity bonus:** When multiple levels cluster within 1.0% of each other, their combined significance is amplified (confluence zone). A cluster of 3+ levels within 1.0% = strong structural magnet.

---

## Integration

### Feeds Into
- **ICSL (Layer 4)** — Proximity-to-level scoring, S/R breakout/flip confirmation, SMC directional context
- **IDCL (Layer 5)** — `recommended_stop` via level hierarchy, `reward_risk_ratio` from level distances
- **IEPL (Layer 9)** — Entry placement at levels (S/R retest, Golden Pocket), TP placement at extensions, SL placement via hierarchy
- **IASL (Layer 8)** — Structure section of Analyst Document (key levels, patterns, SMC status)

### Receives From
- **ITIL (Layer 1)** — All structure-group indicators, SMC indicators, Volume Profile, Fibonacci, Pivot Points
- **IRCL (Layer 2)** — Regime context (Trending favors S/R breakouts, Range favors level bounces)

### Cross-References
- [ITIL: §C Structure Group](../layers/01-itil-technical-indicator.md) — Fibonacci, S/R, Pivot Points, Patterns, Candlestick
- [ITIL: §C Institutional Group](../layers/01-itil-technical-indicator.md) — SMC Structure, Liquidity, FVG, Order Blocks
- [IDCL: §Risk & Reward](../layers/05-idcl-decision-context.md) — How `recommended_stop` uses the level hierarchy
- [IEPL: §C Stop-Loss Placement](../layers/09-iepl-execution-protocol.md) — How the hierarchy translates to actual stop levels
