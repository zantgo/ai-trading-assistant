# IEPL-E — Institutional Execution Protocol: Algorithmic Execution

> **Layer 9 Extension of the Institutional Trading Strategy Decision Pipeline.**
> **Implementation Status: PLANNED** — TWAP, VWAP, Implementation Shortfall execution algorithms.
>
> **Parent:** [09-iepl-execution-protocol.md](09-iepl-execution-protocol.md) — New sub-component H.

---

## Purpose

The existing IEPL defines a 3-layer entry scaling model that places orders at specific structural levels (L1 at signal, L2 at S/R pullback, L3 at Golden Pocket). This is effective for swing and position trading where entries are placed at predetermined price levels.

Algorithmic execution extends this model for:

> **Time-sensitive execution — entering or exiting positions over time rather than at specific price levels.**

This is particularly critical for **scalping** (sub-minute to few-minute trades), where:
- Speed of execution matters more than hitting a specific level
- Market impact must be minimized (even in crypto, large orders move the book)
- Timing risk (price moving against you while entering) must be managed
- VWAP benchmarking provides execution quality measurement

---

## Inputs

| Input | Source | Format |
|-------|--------|--------|
| TraderDecision | IASL | action + confidence + rationale |
| Target position size | IRML + IEPL allocation | f64 (base currency units) |
| Current price (mid) | L0 WebSocket | f64 streaming |
| Volume profile | ISML `volume_profile` indicator | POC/VAH/VAL + historical volume distribution |
| Order book depth | L2 OrderBookAnalysis | Bid/ask depth at levels |
| IRML permission | IRML | trade permission level (for abort gate) |
| ATR | ITIL `atr` indicator | f64 (for abort threshold) |

---

## Outputs

| Output | Format | Consumer |
|--------|--------|----------|
| Scheduled slice orders | `Vec<AlgoSlice>` with timing and size | `OrderManager` (order placement) |
| Algo progress | `AlgoProgress { filled_pct, slices_completed, vwap_performance }` | `MonitoringPanel` (frontend) |
| Algo completion | `AlgoCompletion { avg_price, total_filled, vwap_benchmark, slippage_bps }` | `PaperTrading` (slot machine entry), IPEL (journal) |
| Abort signal | `Option<AbortReason>` | `PaperTrading` (cancel remaining slices) |

---

## Sub-Components

---

### H. Algorithmic Execution Engine

#### H.1 Core Architecture

```
ExecutionAlgo
├── AlgoType: TWAP | VWAP | ImplementationShortfall
├── AlgoConfig: duration, slices, aggression, abort parameters
├── Slice Scheduler: determines when and how much to place
├── Progress Tracker: cumulative fill vs. target
├── VWAP Benchmark: running reference VWAP for performance measurement
└── Abort Monitor: IRML permission + price deviation checks
```

**Lifecycle:**
```
CREATED → SCHEDULING → EXECUTING → COMPLETED
                     ↘ ABORTED (IRML/permission/price deviation)
```

---

#### H.2 TWAP (Time-Weighted Average Price)

The simplest and most robust algo. Divides the total order into N equal slices, placed at fixed time intervals.

**Slice schedule:**
```
N = duration_secs / interval_secs
size_per_slice = total_size / N
scheduled_at[i] = t_start + i × interval_secs

For scalping (typical config):
  duration_secs  = 30     (half a candle)
  interval_secs   = 5     (every 5 seconds)
  N               = 6 slices
  aggression      = 0.01% (place at mid − 0.01% for buy)
```

**Pricing:**
- Buy: place limit order at `mid × (1 − aggression_pct)` — passive, maker-side
- Sell: place limit order at `mid × (1 + aggression_pct)` — passive, maker-side
- Unfilled slices after `interval_secs`: optionally cross spread to fill at market

**Advantages:**
- Predictable execution schedule
- Minimal market impact (small slices)
- Ideal for scalping fixed-duration entries

**Disadvantages:**
- No intelligence about volume patterns
- May underperform VWAP during high-volume periods

---

#### H.3 VWAP (Volume-Weighted Average Price)

Schedules slices according to historical volume distribution. More slices during typically high-volume periods, fewer during low-volume periods.

**Historical volume profile:**
```
Build a 24-hour volume distribution from the Volume Profile indicator.
Normalize: weight[t] = historical_volume[t] / total_historical_volume
Allocate slice count per bin: slices[i] = N × weight[bin(i)]
```

**Adaptive pricing:**
- Track cumulative VWAP of fills so far
- Price next slice to beat VWAP: `limit_price = min(mid − aggression, running_vwap − 1 tick)`
- Goal: `avg_fill_price < VWAP_reference` for buys, `> VWAP_reference` for sells

**VWAP benchmark:**
```
vwap_reference = Σ(price_i × volume_i) / Σ(volume_i)   over duration
vwap_performance = (avg_fill_price − vwap_reference) / vwap_reference × 10000  → basis points
```
Negative bps for buys = favorable execution (below VWAP).

**Advantages:**
- Adapts to market liquidity patterns
- Professional execution benchmark
- Better average price in liquid markets

**Disadvantages:**
- Requires historical volume profile data
- More complex scheduling

---

#### H.4 Implementation Shortfall (Arrival Price)

Minimizes the total cost of execution, balancing three competing factors:

**Cost decomposition:**
```
Total Cost = Market Impact + Timing Risk + Opportunity Cost

Market Impact  ∝ σ × √(Q/V)        (larger orders move the market more)
Timing Risk    ∝ σ × √(T − t)      (longer execution = more drift risk)
Opportunity    ∝ α × unfilled_size  (not completing the order misses alpha)
```

**Optimal strategy (Almgren-Chriss framework):**
The optimal trading trajectory is exponential:
```
q(t) = Q × (1 − e^{−κ(T−t)}) / (1 − e^{−κT})

Where:
  Q  = total order size
  T  = total duration
  κ  = urgency parameter (higher = front-load, lower = spread evenly)
  κ  = √(λ·σ² / η) where λ = risk aversion, η = temporary impact
```

**Simplified for this implementation:**
- Use TWAP as base schedule
- Front-load: more weight on earlier slices (urgency > 0)
- Back-load: more weight on later slices (urgency < 0, passive)
- Adaptive: increase urgency when IRML permission is "Restricted" or "High Caution" (get out faster)

---

#### H.5 Integration with Slot Machine

Each filled slice feeds into the existing Fractional Dynamic Position Slot Machine:

```
Algo Slice N fills → order_matcher processes → slot created/updated
  → progress_tracker increments filled
  → if all slices filled: position entry complete → slot machine takes over
  → if abort: cancel remaining → slot machine manages partial fill
```

**Slot allocation:**
- TWAP/VWAP execution treats the entire order as a single entry (fills across all slices aggregate into one slot)
- Alternatively: each slice = independent slot entry for granular position building
- Default: aggregate mode (cleaner accounting)

---

#### H.6 Abort Conditions

The algo aborts if any of the following triggers:

| Trigger | Condition | Severity |
|---------|-----------|----------|
| **IRML permission downgrade** | Permission → "Suspended" or "Emergency Stop" | Hard abort — cancel all |
| **Price deviation** | |price − entry_trigger_price| > 2.0 × ATR | Hard abort — market moved too far |
| **Consecutive unfilled slices** | 3+ consecutive slices expire unfilled | Soft abort — pause, alert user |
| **Manual override** | User clicks "Cancel" in MonitoringPanel | User abort |
| **Opposite confluence** | `opposite_score > REGISTRY_OPPOSITE_EXIT_THRESHOLD` | Hard abort (signal changed) |

---

#### H.7 Scalping-Optimized Configuration

Short-duration execution tuned for scalping:

```toml
[execution.algo]
default_mode = "TWAP"              # simplest, most predictable
scalping_duration_secs = 15        # ultra-short (quarter of a 1m candle)
scalping_interval_secs = 3         # rapid slicing
scalping_aggression_pct = 0.005   # 0.5 bps (minimal spread for crypto)
abort_atr_multiplier = 2.0        # abort if price deviates 2× ATR
abort_consecutive_unfilled = 3     # abort after 3 consecutive unfilled slices
```

**Why TWAP for scalping:**
- In 15-30 second windows, volume profile doesn't matter — every slice sees similar conditions
- Equal slices = predictable execution, important when every tenth of a percent matters
- No model risk (volume profile may miss recent shifts)
- Simpler = fewer failure modes

---

## Integration

### Feeds Into
- **IEPL (Slot Machine)** — Filled algo slices create/update position slots
- **IPEL (Layer 10)** — Algo completion metrics journaled for performance review
- **IMOL (Layer 11)** — Algo progress displayed in MonitoringPanel

### Receives From
- **IASL (Layer 8)** — TraderDecision triggers algo creation
- **IRML (Layer 7)** — Permission gate for abort, allocation size
- **ISML (Layer 3)** — Volume Profile for VWAP slice scheduling
- **ITIL (Layer 1)** — ATR for abort threshold

### Cross-References
- [IEPL: §B Fractional Slot Machine](09-iepl-execution-protocol.md) — Consumer of filled algo slices
- [IEPL: §A Entry Protocol](09-iepl-execution-protocol.md) — Algo as alternative to 3-layer price-level entry
- [IRML: §13 AI Integration](../layers/07-irmL-risk-management.md) — Permission gate for abort
- [IMOL: §B Exit Signal Monitor](../layers/11-imol-monitoring.md) — MonitoringPanel algorithm status display
- [Commission & Fees](../commission.md) — Fee impact of multi-slice execution

---

## Configuration

```toml
[execution.algo]
default_mode = "TWAP"
scalping_duration_secs = 15
scalping_interval_secs = 3
scalping_aggression_pct = 0.005
abort_atr_multiplier = 2.0
abort_consecutive_unfilled = 3
vwap_profile_window = 24          # hours for volume profile
```

---

## Verification

| Test | Verifies |
|------|----------|
| `test_twap_slice_count` | N = duration / interval, all slices equal size |
| `test_twap_total_fills_match_target` | Sum of slice sizes = target size |
| `test_twap_abort_on_price_deviation` | Algo aborts when price moves > abort_threshold × ATR |
| `test_vwap_slice_distribution` | High-volume bins get proportionally more slices |
| `test_vwap_performance_positive` | Buy algo beats VWAP (avg_fill < vwap_ref) |
| `test_abort_on_permission_downgrade` | Emergency Stop triggers immediate full cancel |
| `test_slice_lifecycle` | Scheduled → Placed → Filled transitions correctly |
| `test_consecutive_unfilled_abort` | 3 consecutive unfilled slices triggers abort |
