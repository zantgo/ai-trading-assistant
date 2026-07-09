# IEPL — Institutional Execution Protocol Layer

> **Layer 9 of 10 in the Institutional Trading Strategy Decision Pipeline.**
> **Implementation Status: COMPLETE** — 3-layer entry, fractional slot machine (4-slot FIFO), 7-level stop hierarchy, bracket order constraints, paper trading.
>
> **Consumed by: IMOL (Layer 11)** — Active position lifecycle management, scale monitoring, trailing stop status, exit signal tracking. See `docs/layers/11-imol-monitoring.md`.

---

## Purpose

The Institutional Execution Protocol Layer (IEPL) answers the mechanical execution question:

> **Given the opportunity and risk assessment, how exactly should positions be entered, managed, and exited?**

IEPL defines the precise rules for position lifecycle management. It translates the strategic decisions from IASL and the risk boundaries from IRML into concrete execution instructions: where to enter, at what size, where to place stops, how to tier take-profits, what invalidates the position, and how bracket orders must be constrained. IEPL is the layer between "should we trade?" and the actual order placement.

**IEPL is a deterministic protocol. It defines HOW, never IF.**

**Code location:** `crates/engine/src/paper_trading.rs` (slot machine + order matching + break-even trail), `crates/engine/src/automation.rs` (decisive close invalidation + trigger engine), `crates/engine/src/commission.rs` (fee modeling), `crates/engine/src/trigger_engine.rs` (trigger dispatch).

---

## Inputs

| Input | Source | Format |
|-------|--------|--------|
| TraderDecision | IASL | action + confidence + rationale + risk_notes |
| IRML risk profile | IRML | overall_risk, permission, exposure tier, recommended_allocation_pct, reward_risk |
| Regime classification | IRCL | Regime label + strategy gates (allowed trade types, max allocation, stop multiplier) |
| Level map (ranked) | ISML | S/R levels, Fibonacci GP, Volume Profile POC/VAH/VAL, SMC OB zones |
| Confluence score | ICSL | Directional score + opposite score for invalidation |
| Current position state | paper_trading DB | active_slots count, vacant_slots, cycle capital, realized PnL accumulator, weighted avg entry |
| Paper balance | paper_balances table | Current cash balance for margin checks |
| Candle data (1m TF) | L0 | Close prices for decisive close invalidation |

---

## Outputs

| Output | Format | Consumer |
|--------|--------|----------|
| Entry execution plan | Price levels + allocation percentages per entry layer | paper_trading (order placement) |
| Stop-loss levels | Price per active slot (via 7-level hierarchy) | paper_trading (bracket order creation) |
| Take-profit levels | TP1/TP2/TP3 prices per slot | paper_trading (limit order placement) |
| Invalidation triggers | Boolean flags + trigger conditions | automation (monitoring loop) |
| Position sizing | Slot margin + unit size per new entry | paper_trading (slot creation) |
| Bracket order instructions | TP/SL order creation/deletion commands | paper_trading (order matching) |

---

## Sub-Components

---

### A. Entry Protocol — 3-Layer Scaling Model

IEPL uses a three-layer entry strategy that scales into a position across progressively higher-conviction levels. This prevents overcommitting capital on a single price point and enables averaging into favorable zones.

| Entry Layer | Allocation | Trigger Condition | Gate |
|-------------|-----------|-------------------|------|
| **Entry 1 (L1)** | 33% of cycle capital | Initial signal confirmation: confluence score passes regime-specific threshold, volume gate passes, no contravening IRML restrictions | Standard entry rules |
| **Entry 2 (L2)** | 33% of cycle capital | Price pulls back to nearest S/R level (support for long, resistance for short). Level test confirmed (LevelTest signal active). Trade thesis remains intact. | Confluence score must remain ≥ entry score × 0.8 (no significant deterioration) |
| **Entry 3 (L3)** | 33% of cycle capital | Price reaches Fibonacci Golden Pocket (61.8%-66.0% retracement). GP test confirmed. Highest conviction scale-in. | Confluence score must remain ≥ 75. This prevents averaging into deteriorating trades. |

**Counter-Trend Prohibition:** Entry direction MUST align with the macro structural trend (1h EMA stack direction + 15m IRCL regime). Counter-trend entries are prohibited regardless of confluence score. This is a hard gate — if structural trend is bearish, Open Long is rejected even if confluence is +80.

**Capital allocation per entry:**
```
slot_margin_L1 = C_cycle × 0.33
slot_margin_L2 = U_cycle / N_vacant     (dynamic, see §B)
slot_margin_L3 = U_cycle / N_vacant     (dynamic, see §B)
```

**L3 Gate Detail:** Entry 3 is the highest-conviction scale-in. It is only permitted when:
1. Confluence score ≥ 75 (high-conviction setup)
2. Price has pulled back into the Golden Pocket (61.8%-66.0%)
3. IRML permission is Allowed or better
4. The original trade thesis is intact (no CHoCH, no confirmed opposing divergence)

If any condition fails, L3 is skipped and the position remains at 66% allocation.

---

### B. Fractional Dynamic Position Slot Machine

The position is managed as 4 discrete FIFO slots. Each slot has its own entry price, margin, size, and P&L. This decomposition enables granular risk scaling and precise exit tracking.

**Reference:** Full mathematical specification in `docs/fractional-dynamic-position-slot-machine.md`.

**Core formulas:**

**Cycle Capital:**
```
C_cycle = I_margin + R_accum

Where:
  I_margin = initial allocated margin at cycle inception
  R_accum = realized_pnl_accumulator (Σ of closed slot PnLs, net of fees)
```

**Unallocated Capital:**
```
U_cycle = C_cycle − Σ(K_i)    for all active slots i
```

**Weighted Slot Margin (new slot):**
```
K_new = (U_cycle / N_vacant) × W

Where:
  N_vacant = number of remaining inactive slots (1-4)
  W = dynamic slot scaling weight (default 1.0, configurable)
```

**Slot Unit Size:**
```
S_i = (K_new × L) / P_entry_i

Where:
  L = leverage (fixed at 20× cross for paper trading)
  P_entry_i = entry price for slot i
```

**Portions Inequality:** Because slots are opened at different prices, their nominal sizes and margins will not be equal during an active cycle. This is intentional — it reflects the compounding model. Portions are equalized (reset to equal split of C_cycle / 4) only when the entire position is fully closed and a new cycle begins.

**Consolidated Metrics (recalculated on every slot state change):**
```
S_total = Σ(S_i)                                    for all active i
K_total = Σ(K_i)                                    for all active i
P_avg = Σ(P_entry_i × S_i) / S_total               weighted average entry
```

**FIFO Scale-Out:** When closing a portion, the oldest slot (minimum timestamp) is deallocated first.

**Exit sequence per slot:**
1. Identify oldest active slot j
2. Calculate PnL_j = (P_exit − P_entry_j) × S_j (long) or (P_entry_j − P_exit) × S_j (short)
3. Update R_accum ← R_accum + PnL_j − fees
4. Refund = K_j + PnL_j − fees → credited to paper_balances
5. Mark slot inactive, persist realized PnL
6. Mirror to trade_telemetry_history (for journal agent audit)
7. Recalculate consolidated metrics
8. If active slots = 0 → terminate cycle, reset R_accum = 0

**Constraints:**
- Maximum 4 active slots per symbol
- Dual-hedged states (simultaneous long AND short on same symbol) strictly prohibited
- Pending entry orders ≤ N_vacant
- Entry rejected if cash balance < K_new

---

### C. Stop-Loss Placement Rules

Stop-loss levels are determined by a 7-level hierarchical fallback chain. Each level is checked in priority order; the first valid level (on the correct side of price) is used.

**Hierarchy (Long positions — stop below entry):**

| Priority | Level Type | Source | Condition |
|----------|-----------|--------|-----------|
| 1 | Bullish Order Block low | ISML SMC Order Blocks | Active bullish OB exists AND OB low < current price |
| 2 | Recent Swing Low | ISML Pivot S/R | Nearest support pivot below price |
| 3 | VWAP | ITIL VWAP indicator | VWAP < current price |
| 4 | Volume Profile VAL | ISML Volume Profile | VAL < current price |
| 5 | Pivot S1 | ISML Pivot Points | S1 < current price |
| 6 | Anchored VWAP (weekly) | ITIL Anchored VWAP | Weekly AVWAP < current price |
| 7 | ATR × 2.0 | ITIL ATR | **FALLBACK** — always returns a value: `SL = price − 2.0 × ATR` |

**Hierarchy (Short positions — stop above entry):**

| Priority | Level Type | Source | Condition |
|----------|-----------|--------|-----------|
| 1 | Bearish Order Block high | ISML SMC Order Blocks | Active bearish OB exists AND OB high > current price |
| 2 | Recent Swing High | ISML Pivot S/R | Nearest resistance pivot above price |
| 3 | VWAP | ITIL VWAP indicator | VWAP > current price |
| 4 | Volume Profile VAH | ISML Volume Profile | VAH > current price |
| 5 | Pivot R1 | ISML Pivot Points | R1 > current price |
| 6 | Anchored VWAP (weekly) | ITIL Anchored VWAP | Weekly AVWAP > current price |
| 7 | ATR × 2.0 | ITIL ATR | **FALLBACK** — always returns: `SL = price + 2.0 × ATR` |

**Dynamic ATR multiplier by regime:**

| Regime | Multiplier | Rationale |
|--------|-----------|-----------|
| Trending | ×2.0 | Normal volatility, standard stop width |
| Compression | ×1.5 | Tight stops, low volatility (coiling) |
| Expansion | ×2.5 – 3.0 | Wide stops, breakout volatility |
| Range | ×1.0 | Very tight stops, mean-reversion precision |
| Transitional | ×2.0 | Default conservative |

---

### D. Take-Profit Tiering

Profit extraction follows a structured tiering model. Each TP level triggers a specific allocation closure and stop adjustment.

| TP Level | Target | Close % | Stop Adjustment |
|----------|--------|---------|-----------------|
| **TP1** | Nearest structural resistance (Long) or support (Short). Prioritizes ISML level hierarchy: S/R → Pivot R1/S1 → Fibonacci 1.272. | 50% of position (close oldest FIFO slots) | **Move remaining slots' SL to weighted average entry price (breakeven).** |
| **TP2** | Fibonacci 1.618 extension. | 50% of remaining position. | No further stop adjustment. |
| **TP3** | Fibonacci 2.618 extension. Used only in high-volatility (expanding ATR) regimes. | Close all remaining position. | N/A — position terminated. |

**TP validation rules:**
- TP targets must be on the correct side of entry (above for long, below for short)
- TP1 must be at least 1.5× ATR from entry to be viable (prevents noise-level targets)
- If the nearest structural level is closer than 1.5× ATR, TP1 is set at 1.5× ATR
- TP2 is skipped if the 1.618 extension is within 0.5% of TP1 (avoids redundant targets)
- TP3 is only active if IRCL regime is Expansion (confirmed breakout) — in other regimes, close 100% at TP2

---

### E. Position Invalidation Rules

Three independent invalidation triggers can terminate a position before TP levels are reached.

**Trigger 1 — Opposite Confluence Score (Primary):**
```
If opposite_score > REGISTRY_OPPOSITE_EXIT_THRESHOLD (default 60%):
    → Immediate full position close (all active slots)
    → Cancel all pending entry orders
    → Cancel all TP/SL brackets
    → Terminate trade cycle
```
This replaces the old "5 opposite signals" rule. It uses the ICSL weighted scoring model — the opposite direction must have stronger conviction than 60% of the active indicator weight. This prevents weak signals from triggering exits while catching genuine structural shifts.

**Trigger 2 — Decisive Close Invalidation:**
```
For Long:  if 1m_candle_close < final_invalidation_level × 0.998    (0.2% tolerance buffer)
For Short: if 1m_candle_close > final_invalidation_level × 1.002    (0.2% tolerance buffer)
    → Full liquidation of all active slots
    → Cancel all pending + bracket orders
    → Terminate cycle
```
- Only 1-minute completed candle closes count — intra-candle wicks are ignored
- The 0.2% tolerance buffer prevents premature invalidation from noise
- `final_invalidation_level` starts as the initial stop-loss, updates to breakeven after TP1 hit

**Trigger 3 — Structural Breakdown:**
```
If CHoCH detected (ISML SMC Structure) AND volume confirms (RVOL ≥ 1.5):
    → Structural trend has reversed
    → Full position close
    → Do NOT re-enter in same direction until new BOS confirms trend resumption
```
Structural breakdown is the most serious invalidation — it means the market structure itself has changed. The CHoCH must be confirmed by institutional volume to prevent false signals.

**Emergency Exit (any trigger):**
```
If structural trend changes on 1h timeframe (EMA 200 cross + ADX regime shift):
    → Close all regardless of confluence or pending TP levels
    → Terminate all active cycles for this symbol
    → IRML permission downgraded to Restricted
```

---

### F. Break-Even Trailing

Break-even trailing protects realized profits by moving the stop-loss to the entry price after TP1 is hit.

**Trigger:** TP1 limit order fills, deallocating the corresponding FIFO slot.

**Action:**
```
final_invalidation_level = P_avg  (weighted average entry price of remaining slots)
```
The `final_invalidation_level` is updated in the master `active_positions` record. All remaining active slots now have their invalidation point at breakeven.

**Guarantee:** After TP1 hit, the trade cycle cannot result in a net capital loss. The worst outcome is breakeven (excluding fees already deducted at TP1 fill).

**Interaction with slot-level stops:** Individual slot bracket stop-losses remain at their original levels (from the hierarchy fallback). The `final_invalidation_level` is the position-level safety net that triggers the decisive close check — it does not replace individual bracket stops.

---

### G. Bracket Order Constraints

Bracket orders (TP limit + SL stop) are managed with strict 1:1 resource mapping to prevent logical conflicts.

**Constraint 1 — Entry Order Capacity:**
```
pending_entry_orders ≤ N_vacant    (= 4 − active_slots)
```
The user cannot place more pending entry orders than available slot capacity. Placing a 3rd entry order when only 2 slots are vacant is blocked by the API handler.

**Constraint 2 — Take-Profit Brackets:**
```
active_tp_orders ≤ active_slots
```
At most one TP limit order per active slot. Creating a new TP order is permitted only when active_tp < active_slots.

**Constraint 3 — Stop-Loss Brackets:**
```
active_sl_orders ≤ active_slots
```
At most one SL stop order per active slot. Creating a new SL order is permitted only when active_sl < active_slots.

**Constraint 4 — Automatic Bracket Creation:**
When a pending entry order fills and a new slot is created, the engine automatically creates corresponding TP and SL brackets for that slot, maintaining `TP = active_slots` and `SL = active_slots`.

**Constraint 5 — FIFO Cleanup on Exit:**
When a slot is closed (manual, TP hit, SL hit, or invalidation):
1. Identify the slot being closed
2. Delete the oldest TP bracket order associated with this position
3. Delete the oldest SL bracket order associated with this position
4. Delete the slot from `position_slots` (mark inactive)
5. Recalculate consolidated metrics

This prevents orphaned bracket orders from executing after their slot is closed. Without FIFO cleanup, a TP order could fill against a position that no longer exists, creating unintended exposure.

**Constraint 6 — Reduce-Only Enforcement:**
All TP and SL bracket orders are **reduce_only** — they can only decrease position size, never increase it. This prevents bracket execution from creating oversize positions during partial fills.

---

## Integration

### Feeds Into
- **IPEL (Layer 10)** — Realized trade outcomes for journaling and performance evaluation
- **Paper Trading Engine** (`paper_trading.rs`) — Order placement, slot creation, bracket management

### Receives From
- **IASL (Layer 8)** — TraderDecision action trigger
- **IRML (Layer 7)** — Allocation tier, permission gate, R:R recommendation
- **IRCL (Layer 2)** — Regime-specific strategy gates
- **ISML (Layer 3)** — Level hierarchy for stop placement and TP targets
- **ICSL (Layer 4)** — Opposite score for invalidation
- **IDCL (Layer 5)** — `recommended_stop`, `trade_quality`, `trade_readiness`

### Cross-References
- [Fractional Dynamic Position Slot Machine Spec](../fractional-dynamic-position-slot-machine.md) — Full mathematical specification of slot lifecycle, cycle capital, and formal verification rules
- [ISML: §G Level Hierarchy](../layers/03-isml-structure-mapping.md) — How the 7-level stop placement hierarchy is built
- [IRCL: §E Strategy Gates](../layers/02-ircl-regime-classification.md) — Regime-specific allocation tiers and stop multipliers
- [ICSL: §G Opposite Score Engine](../layers/04-icsl-confluence-scoring.md) — Opposite score computation for invalidation trigger
- [IASL: §E Decision Categories](../layers/08-iasl-ai-synthesis.md) — How TraderDecision actions map to execution
- [IEPL Algorithmic Execution](../layers/09-iepl-algorithmic-execution.md) — TWAP, VWAP, IS execution algorithms as alternative to 3-layer price entry
