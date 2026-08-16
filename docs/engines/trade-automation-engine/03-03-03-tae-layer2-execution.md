# TAE Layer 2 — Execution Layer

**Version:** 6.10 (2026-08-16) — see docs/CHANGELOG.md for the canonical version history.
**Status:** Specified — **WIP**; backend (`crates/portfolio-supervisor/src/execution/`) is implemented; live exchange order dispatch is not yet built (paper trading is the default and only execution path today). Dashboard wiring lands in [`docs/ROADMAP.md`](../../ROADMAP.md) §3 Phase A and Phase E (live adapter).
**Engine:** Trade Automation Engine (TAE)
**Layer:** 2 of 2
**Input Contract:** Policy Matrix (L1), [Capital Matrix](../portfolio-management-engine/03-04-04-pme-layer3-capital.md) (PME), [Decision Matrix](../../matrices/02-04-decision-matrix.md) (MME)
**Output Contract:** [Execution Matrix](../../matrices/02-15-execution-matrix.md) (order lifecycle)
**Purpose:** This document specifies the Execution Layer — the transaction engine that translates validated Policy directives into exchange orders, manages order state machines, executes the Position Sizing Protocol, and maintains full transaction auditability.

---

## 1. Purpose

The Execution Layer is the platform's **order routing authority**. It receives triggered directives from the Policy Layer (L1), queries the PME Capital Matrix for available margin, computes position sizes, constructs order packets, dispatches them to exchanges (live or simulated), and manages the complete order lifecycle.

```
[Policy Matrix] ──► EXECUTION LAYER (L2) ──► [Exchange / Paper Engine]
                            │
                            └──(reads Capital Matrix)──► [PME]
```

---

## 2. Position Sizing Protocol

### 2.0 Synchronization Guarantees

The TAE Execution Layer and the PME Capital Layer run **in-process** within the `crates/execution-daemon` binary (specifically inside the `crates/portfolio-supervisor` library it loads) — both execute inside the same Tokio runtime, sharing the same memory space. There is **no IPC, no cross-process message bus, and no SQLite round-trip** on the sizing hot path.

The "synchronous read-only pull" from the PME Capital Matrix is implemented as:

```rust
// PSEUDOCODE — actual API at crates/portfolio-supervisor/src/profile_evaluation/*
let available_margin: Decimal = {
    let capital = capital_matrix.read().await; // tokio::sync::RwLock
    capital.available_margin
};
```

**Guarantees:**
- **Atomic balance updates.** Capital Matrix writes (on fill events from PME Layer 3) acquire the writer lock exclusively; readers (TAE sizing) take the read lock and proceed in parallel with each other.
- **Bounded contention.** Writer lock is held only for the duration of a single fill event (~µs); readers never block each other, so multiple sizing requests can fan out concurrently.
- **Hot-path bypass.** The Capital Matrix is held **in memory** by PME; SQLite persistence is asynchronous (fire-and-forget after each in-memory write). A sizing request never waits on disk I/O.

**Out-of-process target.** If TAE and PME are ever split into separate processes, the recommended pattern is a shared memory map (`memmap2`) over an `Arc<CapitalState>` for sub-µs reads, with the writer process using atomic compare-and-swap on the shared buffer. Tokio `watch` channels are a fallback for sub-millisecond freshness guarantees.

The defining computation of the Execution Layer is the **Position Sizing Protocol**:

$$S = \frac{E \times R}{D_{sl} / 100}$$

| Symbol | Meaning | Source | Units |
|--------|---------|--------|-------|
| $S$ | Position size (notional, quote currency) | Computed | — |
| $E$ | Available margin (quote currency) | PME Capital Matrix (`available_margin`) | — |
| $R$ | Risk-per-trade as a decimal fraction | User configuration (`risk_per_trade_pct / 100`) | fraction (0.01 = 1%) |
| $D_{sl}$ | Stop-loss distance percentage | MME Decision Matrix (`stop_loss_distance_pct`) **OR** execution policy `fixed_stop_loss_pct` override (see **$D_{sl}$ resolution priority** below) | raw percent (1.5 = 1.5%) |

**$D_{sl}$ resolution priority (highest first):**
1. If `execution_policy.fixed_stop_loss_pct` is set, use it as $D_{sl}$.
2. Else use `MME_Decision_Matrix.stop_loss_distance_pct`.
3. Else fall back to the system default `2.0` (2 % of entry price).

**Unit convention:** $R$ is a decimal fraction (the config field `risk_per_trade_pct = 1.0` is divided by 100 to yield $R = 0.01$). $D_{sl}$ is consumed as a **raw percentage float** (`stop_loss_distance_pct = 1.5` means 1.5%), so it is divided by 100 inside the formula to convert to a fraction before dividing. The Rust local variable is `d_sl_frac` to avoid the homonym with the raw-percent $D_{sl}$ (see the renaming convention in [01-02-global-architecture.md §6.3](../../conceptual-foundations/01-02-global-architecture.md)).

**Example:** $E = \$10{,}000$, $R = 0.01$ (1%), $D_{sl} = 1.5$ (1.5%) → $S = \frac{10000 \times 0.01}{1.5 / 100} = \frac{100}{0.015} = \$6{,}666.67$ notional. This risks $\$100$ (1% of margin) on a 1.5% stop.

The size is then converted to base-asset units using the current mid-price and rounded to the exchange's minimum order size increment.

### 2.1 Type-Boundary Cast (f64 → Decimal)

> **Target Architecture (Not Yet Implemented).** The Execution Layer is the **cold path** entry point: it reads `stop_loss_distance_pct` ($D_{sl}$) from the Decision Matrix as an `f64`, pulls available margin ($E$) from the PME Capital Matrix as a `Decimal`, and performs a safe, rounded cast so the sizing runs entirely in fixed-point:
>
> ```rust
> // stop_loss_distance_pct is f64 (e.g. 1.5, raw percent);
> // available_margin is Decimal; risk_per_trade_pct is a raw-percent float (e.g. 1.0 = 1%).
> // Convert the percent inputs to fractions BEFORE sizing so the math reads naturally.
> // Variable-naming convention: d_sl_frac is the fraction (already /100); D_sl_pct is the raw percent.
> let d_sl_frac     = Decimal::from_f64_retain(stop_loss_distance_pct / 100.0)?; // 1.5 → 0.015
> let risk_fraction = Decimal::from_f64_retain(risk_per_trade_pct     / 100.0)?; // 1.0 → 0.010
> let size_quote_usd = (available_margin * risk_fraction) / d_sl_frac;            // Decimal math — notional in quote currency
> ```
>
> *Current implementation:* the canonical `f64 → Decimal` boundary cast described above lives in `crates/portfolio-supervisor/src/execution/order.rs::construct_order` (lines 50–72), not in `risk_calculator.rs`. The cast uses `Decimal::from_f64_retain` for both `stop_loss_distance_pct / 100.0` and `risk_per_trade_pct / 100.0`, exactly as the canonical form above. All sizing math downstream of the cast is `Decimal`. `risk_calculator.rs` (`crates/portfolio-supervisor/src/risk_calculator.rs`) is now also fully `Decimal` for the inputs it consumes (`RiskCalculationInput`, `RiskCalculation`).
>
> **Variable-naming hazard (correction).** A previous version used `risk_pct` directly in the multiplication, which is a 100× off-by-default error if the value is the raw-percent float (`risk_per_trade_pct = 1.0` would produce a 1.0 × E size instead of `0.01 × E`). The canonical variable name for the **fraction** is **`risk_fraction`** (or `risk_frac`); the raw-percent input is **`risk_per_trade_pct`** (as set in `config.toml` `[execution_policies.*.risk.risk_per_trade_pct]`). All downstream consumers must respect this distinction.

---

## 3. Order Construction

### 3.1 Order Types

| Order Type | Usage |
|------------|-------|
| `Market` | Immediate execution at best available price; used when fill speed is critical. |
| `Limit` | Executes at specified price or better; used for entries with defined levels. |
| `Stop` | Triggers a market order when a price threshold is breached; used for stop-loss exits. |

> **`reduce_only` is an order *attribute*, not an order type.** Any of the three order types above may carry the `reduce_only = true` flag (§3.2), which guarantees the order can only decrease net exposure. This is distinct from the `CLOSE_ONLY` policy **stance** (L1) — see §3.3 for the stance→flag handoff.
>
> **Lifecycle gate (Gate 0, v6.2).** Order packets are constructed only when the per-instance `lifecycle_state = RUNNING`. Exits (`reduce_only = true` or `is_emergency_liquidation = true`) are constructed regardless of lifecycle state, in conformance with [08-02-pre-trade-risk-controls.md Gate 0](../../operations-and-compliance/08-02-pre-trade-risk-controls.md) and [03-03-06 IL-05](../trade-automation-engine/03-03-06-tae-instance-lifecycle-spec.md). The STOP flatten dispatch (`RUNNING/lifecycle PAUSED → STOPPING → STOPPED`) reuses Step 2a Hard Exit from §7 below: orders tagged `is_emergency_liquidation = true` and `reduce_only = true`, size copied verbatim from the Position Matrix.

### 3.2 Order Packet Fields

| Field | Type | Description |
|-------|------|-------------|
| `client_order_id` | `string` | Idempotency key — prevents duplicate submission on retry. |
| `symbol` | `string` | Instrument. |
| `side` | `Side` | `Buy` / `Sell`. |
| `order_type` | `OrderType` | `Market` / `Limit` / `Stop`. |
| `price` | `Decimal` | Limit/stop trigger price (null for market). |
| `size` | `Decimal` | Base-asset quantity. |
| `reduce_only` | `bool` | Whether the order carries the reduce-only flag (a per-order attribute, NOT an order type). Mirrors the exchange-native concept (Hyperliquid `reduceOnly`, Bitget/Binance `reduceOnly`). Independent of — but deterministically populated by — the Policy Layer's `CLOSE_ONLY` stance; see §3.3. |
| `is_emergency_liquidation` | `bool` | **Hard Exit path flag.** When `true`, the order bypasses pre-trade Gates 1, 2, 4, 5, 6, 7 (per [08-02-pre-trade-risk-controls.md §3](../../operations-and-compliance/08-02-pre-trade-risk-controls.md)) so the liquidation is dispatched even when the symbol stance is `AVOID`. Forced by the PME Veto path in [PME Layer 4 §4.2](../portfolio-management-engine/03-04-05-pme-layer4-portfolio.md) — only `true` for orders originated by the Hard Exit directive. Set to `false` (default) for every other order. Persisted to `open_orders.is_emergency_liquidation` (see [06-02-database-schema-spec.md §3.2](../../integration-and-api/06-02-database-schema-spec.md)) for audit and replay. `close_reason = EMERGENCY_LIQUIDATION` is written at close; the flag covers the pre-close lifecycle. |
| `associated_position_id` | `u64` | Position this order relates to (for exits/modifications). |

### 3.3 CLOSE_ONLY Stance → `reduce_only` Flag Handoff

This section formalizes the programmatic mapping from the Policy Layer's `CLOSE_ONLY` **stance** (a per-symbol authorization state managed by the Policy Layer) onto the Execution Layer's `reduce_only = true` order attribute (a per-order boolean flag). The two are conceptually distinct — the stance controls *which* policy evaluations are allowed to fire, and the flag guarantees a single order, when dispatched, can only reduce exposure.

#### 3.3.1 Safety Invariant (Unconditional Force)

The `reduce_only` flag is **always forced to `true`** on every order packet whose originating policy stance is `CLOSE_ONLY`. This safety boundary is **non-configurable** — it is enforced unconditionally inside the Execution Layer's order-construction code path, *before* the order is signed or dispatched to the exchange. Any operator- or policy-side attempt to construct an order without the flag under a `CLOSE_ONLY` stance is rejected with a construction error.

> **Why this is non-configurable.** A `CLOSE_ONLY` stance is declared by either (a) the operator explicitly to halt new directional exposure, or (b) the PME Veto as part of the systemic safety override (see [03-04-05-pme-layer4-portfolio.md §4](../portfolio-management-engine/03-04-05-pme-layer4-portfolio.md) and [03-03-02-tae-layer1-policy.md §7](../trade-automation-engine/03-03-02-tae-layer1-policy.md)). Both triggers reflect the system's decision that the symbol **must not** gain further exposure. Allowing the flag to be suppressed would re-open directional exposure under the very condition the stance was meant to prevent — a categorical safety violation.

The `reduce_only_on_close_only` boolean that previously appeared in the policy schema ([03-03-04-tae-execution-policy-spec.md §2.1](../trade-automation-engine/03-03-04-tae-execution-policy-spec.md)) is **deprecated** as a behavioral knob. It is retained in the schema for forward-compatibility reads (default `true`) but is no longer consulted by the Execution Layer. New policies should not set it to `false`; old policies that do so are upgraded to `true` at load time.

#### 3.3.2 Order-Type Compatibility

The `reduce_only` flag is set to `true` for **every order type** that the Execution Layer supports when the originating stance is `CLOSE_ONLY`. The order type itself is independent of the flag — the flag is a per-order attribute, not an order kind:

| Order Type | Compatible with `reduce_only = true` under `CLOSE_ONLY`? | Notes |
|------------|------------------------------------------------------------|-------|
| `Market` | **yes** | Used for immediate exit fills when liquidation must proceed (e.g. PME Hard Exit path, see §4.2 of [03-04-05-pme-layer4-portfolio.md](../portfolio-management-engine/03-04-05-pme-layer4-portfolio.md)). |
| `Limit` | **yes** | The default for `CLOSE_ONLY`-origin exits; allows price-conditional exits without further exposure. |
| `Stop` | **yes** | Stops added under `CLOSE_ONLY` can tighten protection but can never widen, so they may not generate new exposure even if they trigger. |

| Stance | `reduce_only` |
|--------|---------------|
| `ACTIVE` | determined by the policy — may be `true` or `false` per the policy's intent |
| `CLOSE_ONLY` | **`true` (always, non-configurable)** |
| `AVOID` | n/a — no orders are dispatched under `AVOID` |

#### 3.3.3 Why Not Auto-Compute on the Policy Side?

A naive alternative would be to derive the flag inside the Policy Layer when constructing the directive, leaving the Execution Layer to accept whatever it receives. This architecture deliberately rejects that alternative for three reasons:

1. **Defense in depth.** The execution boundary is the canonical last gate before a signed order reaches a remote venue. Re-asserting the invariant at the boundary protects against policy-layer bugs that might forget to set the flag.
2. **Auditability.** Every dispatched order's `reduce_only` value is captured in the **Execution Matrix** (`is_reduce_only` field, [§7](#7-output-execution-matrix) and the DB `open_orders.is_reduce_only` column). The invariant makes that field **deterministic** from the stance: `reduce_only = (stance == CLOSE_ONLY) || policy_directive.reduce_only_request`.
3. **Cross-engine contract clarity.** Both the Policy Layer and the Execution Layer can refer to one rule. The stance-to-flag mapping has exactly one definition (this section); any divergence between docs is a regression.

#### 3.3.4 Cross-References

- [README "Key Conventions" — `reduce_only` invariant](../../../README.md) — operator-facing summary.
- [03-03-02-tae-layer1-policy.md §4 Stances](../trade-automation-engine/03-03-02-tae-layer1-policy.md) — stance definitions at the policy side.
- [03-03-04-tae-execution-policy-spec.md §2.1](../trade-automation-engine/03-03-04-tae-execution-policy-spec.md) — deprecation note for `reduce_only_on_close_only`.
- [03-04-05-pme-layer4-portfolio.md §4.2 Veto Execution](../portfolio-management-engine/03-04-05-pme-layer4-portfolio.md) — the PME veto path that asserts `CLOSE_ONLY`/`AVOID` stances and requires the Hard Exit path.
- [06-02-database-schema-spec.md §3.2 `open_orders.is_reduce_only`](../../integration-and-api/06-02-database-schema-spec.md) — persistence contract.

---

## 4. Transaction State Machine

Every order transitions through a logged lifecycle. `PRE_DISPATCH` is entered at the Gate-5/manual-review hold, before `PENDING`. Approval transitions it to `PENDING`; discard or timeout transitions it to `REJECTED`:

```
                  ┌──────────────┐  approve   ┌──────────┐  size+route  ┌──────────┐   ack    ┌──────────┐
                  │ PRE_DISPATCH │───────────►│  PENDING │─────────────►│ SUBMITTED│─────────►│  OPEN    │
                  │ (HELD_FOR_   │            └──────────┘              └──────────┘          └──────────┘
                  │  REVIEW)     │                  │                          │                    │
                  └──────────────┘                  │ reject                   │ cancel             ▼ partial
                       ▲  ▲                         ▼                          ▼              ┌──────────────┐
                       │  │                    ┌──────────┐                 ┌──────────┐      │PARTIALLY_FILLED│
                       │  │ timeout/discard    │ REJECTED │                 │ CANCELLED│      └──────────────┘
                       │  └───────────────── PRE_DISPATCH is                └──────────┘             │
                       │                       in-memory only;                                             │
                  in-memory only                 never persisted                                             ▼
                  (Gate 5 review)                to `open_orders`                                     ┌──────────┐  ┌──────────┐
                                                                                                       │  CLOSED  │  │ CANCELLED│
                                                                                                       └──────────┘  └──────────┘
```

`PRE_DISPATCH` orders are held in process memory only; they are **never** persisted to the `open_orders` table. An engine restart, crash, or process termination during the slippage-review window loses the held order — no audit trail. The state is entered at the Gate-5/manual-review hold before `PENDING` and exits either to `PENDING` on operator approval or to `REJECTED` on operator discard / timeout. Operators relying on Gate 5 for slippage review in a 24/7 deployment should design workflows around the manual-review API rather than expecting engine-replayable recovery (see [08-02-pre-trade-risk-controls.md §3.2](../../operations-and-compliance/08-02-pre-trade-risk-controls.md)).

Every persistent transition (`PENDING` onwards) is written to the Execution Matrix with a high-resolution timestamp, guaranteeing full auditability. Partial fills are tracked against the associated position.

### 3.5 Exit and Reduce-Only Order Bypass

> **Reduce-only sizing bypass.** A previous version of this document ran every order through the Position Sizing Protocol (`S = E·R / (D_sl / 100)`). Under high-drawdown conditions, available margin `E` collapses toward zero, which would cause the sizing formula to yield `S ≈ 0` — preventing the platform from closing its own open exposure exactly when exits are most urgent.

**Bypass rule.** *Exit / reduce-only orders bypass the Position Sizing Protocol.* They **copy the current position size directly from the Position Matrix** rather than computing it from available margin and stop distance. The only orders that run through the sizing formula are **new-entry orders** with `reduce_only = false`.

This rule is invoked in three places:

1. **PME Hard Exit Path** — when the PME Veto forces `AVOID`/`CLOSE_ONLY` for a symbol with an open position, the TAE Policy Layer dispatches a liquidation directive; the Execution Layer converts it into a market order whose `size` is **copied verbatim** from the Position Matrix (not run through sizing). See [03-04-05-pme-layer4-portfolio.md §4.2](../portfolio-management-engine/03-04-05-pme-layer4-portfolio.md) for the veto sequence.
2. **`CLOSE_ONLY`-stance exits** — under a `CLOSE_ONLY` stance, the only orders dispatched are exits. The Execution Layer sources `size` from the Position Matrix directly; the formula `S = E·R/(D_sl/100)` is **skipped**.
3. **Manual close** — `POST /api/instances/:id/manual/close` records a manual position close via the same path; the operator provides the exact size; the Position Sizing Protocol is bypassed.

| Order intent | Sourcing of `size` | Runs Position Sizing Protocol? |
|-------------|-------------------|--------------------------------|
| New entry (`ACTIVE` stance, `reduce_only=false`) | sized via `S = E·R/(D_sl/100)` | **yes** |
| Exit / reduce-only (any stance with `reduce_only=true`; or any order under `CLOSE_ONLY`; or Hard Exit path) | copied from `Position Matrix.current_size` | **no** |

The rule preserves the central safety property that **risk is bounded at entry** while not letting a fully-drawn-down account get stuck with unexitable positions.

---

## 5. Slippage Control

| Mechanism | Description |
|-----------|-------------|
| **Limit-offset filter** | Limit orders are placed with a configurable offset from the reference price to protect against adverse fills. |
| **Book-depth check** | Immediately before dispatch, the Execution Layer queries the real-time order book depth to verify sufficient liquidity at the target price level. |
| **Slippage ceiling** | If estimated slippage exceeds a user-configured percentage of the position size, the order is held for manual review. |

---

## 6. API Rate Limiting & Idempotency

| Concern | Policy |
|---------|--------|
| **Rate limiting** | Order dispatch throttled to venue limits; batched cancellations on veto. |
| **Idempotency** | `client_order_id` prevents duplicate submission on retry — the exchange ignores duplicate IDs. |
| **Capital query** | Synchronous read-only pull from PME Capital Matrix at sizing time. |

---

## 7. Output: Execution Matrix

See canonical specification at [Execution Matrix — `02-15-execution-matrix.md`](../../matrices/02-15-execution-matrix.md).

The Execution Matrix is a persistent log of all order states, materialized as the `open_orders` SQLite table (see [Database Schema §3.2](../../integration-and-api/06-02-database-schema-spec.md)):

| Field | Type | Description |
|-------|------|-------------|
| `order_id` | `string` | Exchange-assigned order ID. |
| `client_order_id` | `string` | Idempotency key. |
| `symbol` | `string` | Instrument. |
| `order_type` | `string` | `MARKET` / `LIMIT` / `STOP`. |
| `direction` | `string` | `BUY` / `SELL`. |
| `price` | `Decimal` | Order price (limit/stop). |
| `trigger_price` | `Decimal` | Trigger price (stop orders). |
| `size` | `Decimal` | Base-asset quantity. |
| `filled_size` | `Decimal` | Cumulative filled quantity. |
| `status` | `string` | `PENDING` / `SUBMITTED` / `OPEN` / `PARTIALLY_FILLED` / `CLOSED` / `REJECTED` / `CANCELLED`. |
| `is_reduce_only` | `bool` | Reduce-only flag (mirror of Order Packet Field, see §3.2). |
| `is_emergency_liquidation` | `bool` | Whether this order is part of the Hard Exit path. `true` ⇒ the order was dispatched by the PME Veto and bypassed pre-trade gates (see §3.2 and [PME Layer 4 §4.2](../portfolio-management-engine/03-04-05-pme-layer4-portfolio.md)). |
| `associated_position_id` | `u64` | Linked position. |
| `created_at` | `u64` | Unix epoch timestamp. |
| `updated_at` | `u64` | Last state transition timestamp. |
| `slippage_bps` | `f64` | Fill slippage in basis points. |

> **Canonical source:** The field table above is mirrored from the canonical [Execution Matrix specification](../../matrices/02-15-execution-matrix.md). If any other doc disagrees with the values here, `02-15-execution-matrix.md` wins. |

---

## 8. Paper Trading Engine

When operating in paper/simulated mode (see [TAE Paper Trading](03-03-05-tae-paper-trading-spec.md)), the Execution Layer routes orders to the internal matching engine instead of a live exchange. The same state machine, sizing protocol, and audit logging apply — only the order destination changes.

---

## 9. Cross-References

- [TAE Overview](../trade-automation-engine/03-03-01-tae-overview-spec.md) — Engine boundaries and operational modes.
- [TAE Layer 1 — Policy](03-03-02-tae-layer1-policy.md) — Upstream trigger source.
- [TAE Paper Trading](03-03-05-tae-paper-trading-spec.md) — Simulated execution engine.
- [TAE Instance Lifecycle & Programmable State Control](../trade-automation-engine/03-03-06-tae-instance-lifecycle-spec.md) — Gate 0 entry admission; STOPPING flatten dispatch reuses §7 Hard Exit.
- [Decision Matrix](../../matrices/02-04-decision-matrix.md) — Stop-loss distance source.
- [PME Layer 3 — Capital](../portfolio-management-engine/03-04-04-pme-layer3-capital.md) — Equity source.
- [Ontology — Trade Execution](../../conceptual-foundations/01-01-ontology.md) — Conceptual definition.
