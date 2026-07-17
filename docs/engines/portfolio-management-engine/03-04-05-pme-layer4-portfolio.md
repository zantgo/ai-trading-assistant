# PME Layer 4 — Portfolio Layer

**Version:** 6.2 (2026-07-17) — see docs/CHANGELOG.md for the canonical version history.
**Status:** Approved
**Engine:** Portfolio Management Engine (PME)
**Layer:** 4 of 4
**Input Contract:** Position Matrix (L1), Exposure Matrix (L2), Capital Matrix (L3), [Overview Matrix](../../matrices/02-09-overview-matrix.md) (MME L7)
**Output Contract:** Portfolio Matrix (unified master ledger + safety veto)
**Purpose:** This document specifies the Portfolio Layer — the top-level consolidation layer that synthesizes all child matrices into a unified account health vector and enforces the Ontological Priority Veto over automated trading.

---

## 1. Purpose

The Portfolio Layer is the PME's **command authority**. It synthesizes Position, Exposure, and Capital matrices into a single unified ledger, continuously evaluates systemic safety thresholds, and — when a threshold is breached — asserts **Ontological Priority (Veto Power)** to override TAE stances, blocking new entries and canceling pending orders.

```
[Position Matrix ] ─┐
[Exposure Matrix ] ─┼──► PORTFOLIO LAYER (L4) ──► [Portfolio Matrix]
[Capital Matrix  ] ─┘                                  │
[Overview Matrix ] ─┘                                  │
                                                       └──(veto)──► [TAE stances]
```

---

## 2. Portfolio Matrix Schema

| Field | Type | Description |
|-------|------|-------------|
| `current_equity` | `Decimal` | Total account equity (`initial_balance + realized_pnl + unrealized_pnl`; canonical formula). |
| `realized_pnl` | `Decimal` | Cumulative realized profit/loss (net of fees). |
| `unrealized_pnl` | `Decimal` | Aggregate unrealized PnL. |
| `gross_exposure` | `Decimal` | Total notional exposure. |
| `net_exposure` | `Decimal` | Directional net exposure. |
| `margin_usage_ratio` | `Decimal` | Fraction of equity committed to margin, in `[0, 1]`. |
| `leverage_ratio` | `Decimal` | Effective leverage (`gross_exposure / equity`). |
| `daily_pnl` | `Decimal` | PnL in current session. |
| `max_daily_drawdown_pct` | `Decimal` | **Configuration limit** (operator-set, default 0.05 = 5 %). The live metric is `daily_drawdown_pct = -daily_pnl / starting_session_equity`; WARN fires when the live metric crosses the configured limit. Distinct from `drawdown_limit_pct` (the hard veto threshold; see §4 below). |
| `drawdown_limit_pct` | `Decimal` | Equity peak-to-trough decline threshold (fraction, default 0.30). This is the **hard veto** stop-loss denominator (see §4). |
| `peak_equity` | `Decimal` | Trailing high-water mark of `current_equity`. See §4.4 for reset/update policy. |
| `safety_state` | `SafetyState` | `NORMAL` / `WARN` / `CAUTIOUS` / `SUSPENDED` / `DRAWDOWN_STOP`. The `WARN` state was added in the institutional redesign (wired to `max_daily_drawdown_pct`; see §3). |
| `systemic_risk_score` | `f64` | MME Overview Matrix Systemic Risk Score. |
| `active_stances` | `map<string, Stance>` | Per-symbol authorization: `ACTIVE` / `CLOSE_ONLY` / `AVOID`. |
| `default_stances` | `map<string, Stance>` | Operator-configured default stances, restored on veto release. |
| `consecutive_losses` | `map<string, u32>` | **Per-symbol** consecutive-loss counter (see §3). |
| `position_count` | `u32` | Number of active positions. |

---

## 3. Safety Circuit Breakers

The `SafetyManager` (`crates/portfolio-supervisor/src/safety.rs`) tracks five escalating safety states:

```
NORMAL ──(daily_drawdown_pct ≥ max_daily_drawdown_pct)──► WARN
       ──(consecutive_losses[sym] ≥ caution_threshold)──► CAUTIOUS
       ──(systemic_risk_score ≥ systemic_risk_threshold)──► CAUTIOUS
       ──(consecutive_losses[sym] ≥ dropout_threshold)──► SUSPENDED (timed cooldown)
       ──(current_equity / peak_equity < 1 − drawdown_limit_pct)──► DRAWDOWN_STOP
```

> **Lifecycle ↔ safety orthogonality (v6.2).** `DRAWDOWN_STOP` (PME safety) and `LifecycleState` (instance lifecycle, [03-03-06-tae-instance-lifecycle-spec.md](../trade-automation-engine/03-03-06-tae-instance-lifecycle-spec.md)) are **independent axes** (IL-06). A `STOP` during `DRAWDOWN_STOP` proceeds — flatten is the emergency path. A `START` during `DRAWDOWN_STOP` transitions the instance to `RUNNING`, but Gates 1/7 still block entries until `/safety/release-veto`. See [03-03-06 §6 Interaction matrix](../trade-automation-engine/03-03-06-tae-instance-lifecycle-spec.md) for the full interaction table.

| State | Trigger | Effect | Scope |
|-------|---------|--------|-------|
| `NORMAL` | Default | Full trading permitted. | — |
| `WARN` | `daily_drawdown_pct ≥ max_daily_drawdown_pct` (default 0.05 = 5 %), where `daily_drawdown_pct = -daily_pnl / starting_session_equity` | **Early-warning only — no stance changes.** A `WARN` event sets `safety_state = WARN`, surfaces a banner in the GUI (Portfolio panel), and is logged to the audit trail with the trigger reason. Trading continues as in `NORMAL`. `WARN` is cleared automatically when `daily_drawdown_pct` returns below the threshold *or* on the daily session reset. The 60-second equity snapshot cadence that drives the live `daily_drawdown_pct` metric is implemented in [PME Layer 3 §3](../portfolio-management-engine/03-04-04-pme-layer3-capital.md); the daily session reset (`peak_equity = current_equity` at the operator-defined `session_reset_cron`, default `00:00 UTC`) is documented in §4.4 below. | Platform-wide |
| `CAUTIOUS` | `consecutive_losses[sym] ≥ caution_threshold` (default 3) | Warning only; no stance changes yet. | **Per-symbol** |
| `SUSPENDED` | `consecutive_losses[sym] ≥ dropout_threshold` (default 5) | Affected symbol's stance → `CLOSE_ONLY`; 8-hour cooldown. A win resets that symbol's counter. Other symbols are unaffected. | **Per-symbol** |
| `DRAWDOWN_STOP` | `current_equity / peak_equity < 1 − drawdown_limit_pct` (default 0.30 = 30 %) | All stances → `AVOID`; immediate veto. | Platform-wide |

Defaults are configurable via `config.toml` `[safety]`.

> **`consecutive_losses` scope.** The counter is **per-symbol** (instance-scoped). A hot streak on `BTC-USDT` does not lock `ETH-USDT` positions. The `/safety/reset` endpoint operates per-symbol (`:id`). The `SUSPENDED` state on one symbol does not affect other symbols.
>
> **`WARN` state wiring.** The `max_daily_drawdown_pct` metric (5 % default) is wired into the safety state machine as the **first** pre-CAUTIOUS trigger (early-warning stage). It produces no stance change — only a visible banner and an audit record. This matches the spec's stated intent: `max_daily_drawdown_pct` (5 %) is an *early-warning* threshold; `drawdown_limit_pct` (30 %) is the hard veto. The two metrics are distinct.

---

## 4. Ontological Priority Veto

The Portfolio Layer holds **veto authority** over the TAE. This is the platform's ultimate safety mechanism.

### 4.1 Veto Triggers and Stance Mapping

Each veto trigger maps to exactly one target `Stance` per the table below. `WARN` is **not** a veto trigger — it is a pre-veto early-warning alert (see §3) that produces no stance change.

| Trigger | Condition | Target Stance | Hard Exit Path? | Release Condition |
|---------|-----------|---------------|------------------|-------------------|
| **Drawdown breach** | `current_equity / peak_equity < 1 − drawdown_limit_pct` (default 0.30) | `AVOID` | **Yes** (forced liquidation) | `current_equity / peak_equity ≥ 1 − drawdown_limit_pct` (see §4.3) |
| **Margin ceiling** | `margin_usage_ratio ≥ 0.95` | `CLOSE_ONLY` | **No** (graceful wind-down) | `margin_usage_ratio < 0.90` sustained for 60 s |
| **Margin exhaustion** | `margin_usage_ratio ≥ 1.00` | `AVOID` | **Yes** (forced liquidation) | `margin_usage_ratio < 0.95` sustained for 60 s |
| **Loss streak (≥ 5)** | `consecutive_losses[sym] ≥ dropout_threshold` (default 5) | `CLOSE_ONLY` (per-symbol) | **No** (graceful wind-down) | First winning trade (counter reset) or 8-hour cooldown expiry |
| **Systemic risk** | `systemic_risk_score ≥ systemic_risk_threshold` (default 80) | `AVOID` | **Yes** (forced liquidation) | `systemic_risk_score < systemic_risk_threshold` (see §4.3) |
| **Manual override** | Operator-initiated | as specified (operator chooses `AVOID` or `CLOSE_ONLY`) | depends on operator input | manual reset |

> **Margin exhaustion.** The PME L3 §6 documents two margin-trigger thresholds layered as escalation. `margin_usage_ratio ≥ 0.95` is the early-warning `CLOSE_ONLY` graceful wind-down; `margin_usage_ratio ≥ 1.00` is the emergency `AVOID` Hard Exit path. A 100%-margin scenario must exit through the `AVOID` Hard Exit, not the less-severe `0.95` graceful wind-down path.

> **AVOID vs CLOSE_ONLY distinction.** `AVOID` triggers the **Hard Exit Path** (forced liquidation via market orders — see §4.2). `CLOSE_ONLY` is a **graceful wind-down**: no forced liquidation; existing positions are managed by their protective stops and policy exits; new entries are blocked; the operator may manually liquidate via `POST /api/instances/:id/manual/close`. Treating `CLOSE_ONLY` as `AVOID` (forcing market liquidation) is a documented anti-pattern that defeats the granularity of the safety state machine.

> **Unit convention (correction).** `drawdown_limit_pct` is a fraction (default `0.30`, meaning 30 %). The breach formula `(1 − drawdown_limit_pct)` evaluates to `1 − 0.30 = 0.70`. The comparison `current_equity / peak_equity < 0.70` triggers when `current_equity ≤ 70 % × peak_equity` — a 30 % peak-to-trough hit. **Note:** a previous version of this section expressed `drawdown_limit_pct` as a raw percentage float (e.g. `30.0`) and the breach formula `(1 − drawdown_limit_pct / 100)`. Both representations are equivalent; this document uses the fraction form throughout for consistency with the rest of the corpus.

### 4.2 Veto Execution

The veto execution sequence is **time-critical** and must follow the steps below in strict order. The two key safety invariants are: (a) Hard Exit fires **before** stance transitions to `AVOID` (so the liquidation order carries the pre-veto authorization); and (b) cancellations fire **after** Hard Exit is acknowledged (so positions are not left unhedged).

1. **Portfolio Layer publishes a high-priority `VetoMessage` to TAE**, including the trigger type and target stance.
2. **For `AVOID` triggers: dispatch Hard Exit (Step 2a) BEFORE stance transition (Step 3).** The TAE Policy Layer dispatches a **liquidation directive** (not a cancellation) to the Execution Layer. The Execution Layer:
   - Reads the current `size` for the position from the Position Matrix (bypassing the Position Sizing Protocol — see [03-03-03-tae-layer2-execution.md §3.5](../trade-automation-engine/03-03-03-tae-layer2-execution.md) for the canonical "Exit and Reduce-Only Order Bypass" rules),
   - Constructs a `Market` order with `reduce_only = true` (forced by the [§3.3 invariant](../trade-automation-engine/03-03-03-tae-layer2-execution.md), not by a §3.5 rule — §3.3 defines the `CLOSE_ONLY` stance → `reduce_only = true` mapping),
   - Tags the order `is_emergency_liquidation = true` so it bypasses Gate 1 (stance) and other pre-trade gates per [08-02-pre-trade-risk-controls.md §3](../../operations-and-compliance/08-02-pre-trade-risk-controls.md),
   - Dispatches it to the exchange,
   - Waits for exchange acknowledgement (filled or terminal reject) with a bounded timeout `hard_exit_ack_timeout_ms` (default 2000 ms).
3. **TAE Policy Layer sets the target stance** to `AVOID` (for drawdown / systemic risk triggers) or `CLOSE_ONLY` (for margin / loss-streak triggers). For `CLOSE_ONLY` triggers, **no Hard Exit is dispatched** — Step 2a is skipped entirely and existing positions are managed by their protective stops and policy exits.
4. **After Hard Exit acknowledgement (or timeout):** the TAE Execution Layer issues batch cancellation orders for any remaining outstanding limit/stop orders on the exchange. If acknowledgement exceeds the timeout, the cancellation batch still proceeds — protective stops are cancelled unconditionally to prevent zombie orders, and the liquidation is flagged as `unconfirmed_exit` in the audit trail.
5. TAE Execution Layer nullifies pending entry triggers.
6. The veto is logged with timestamp, trigger, target stance, and rationale for audit.

> **Loophole fix.** A previous version of this section issued only cancellations in steps 2–4, which would leave active positions open at the venue after the protective stops were cancelled (and `AVOID` blocks all trigger evaluations — so a follow-up exit signal could not be generated to cover them). The Hard Exit path in step 2a ensures every open position is closed on venue at the time the veto asserts; cancellations in step 4 just clean up the residual limit/stop orders.

> **Hard Exit / AVOID ordering invariant.** A veto with `AVOID` target MUST fire Hard Exit (Step 2a) **before** transitioning the stance to `AVOID` (Step 3). This ordering is independent of Gate 1 — `is_emergency_liquidation` orders bypass Gate 1 unconditionally (per [08-02-pre-trade-risk-controls.md §3](../../operations-and-compliance/08-02-pre-trade-risk-controls.md)). The invariant exists because: (i) the exit size is snapshotted from the **pre-veto** Position Matrix; (ii) the exchange acknowledgement (or `hard_exit_ack_timeout_ms` expiry) must be recorded against the pre-veto stance so that an `unconfirmed_exit` audit row is attributable; and (iii) pending entry triggers must not be evaluated against the new stance before their paired exit has been dispatched. The order in Step 2a → Step 3 is therefore non-negotiable. Note the cross-reference was previously cited as `§4.4` in [03-03-03-tae-layer2-execution.md](../trade-automation-engine/03-03-03-tae-layer2-execution.md); the section is actually numbered `§3.5` in that file (and was previously misnumbered `§4.4` throughout the corpus — corrected in v2.1 to point to §3.5).

### 4.3 Veto Release

When the condition clears (e.g., equity recovers above drawdown limit):

1. **The veto condition is re-checked** for each trigger type:
   - **`DRAWDOWN_STOP` (drawdown):** `current_equity / peak_equity ≥ 1 − drawdown_limit_pct`.
   - **`Systemic risk`:** `systemic_risk_score < systemic_risk_threshold`.
   - **`Margin ceiling`:** `margin_usage_ratio < 0.90` sustained for 60 s.
   - **`Loss streak (SUSPENDED)`:**
     - First winning trade resets `consecutive_losses[sym]`; safety state transitions to `NORMAL` (or `CAUTIOUS` if recent losses).
     - Otherwise, 8-hour cooldown from the SUSPENDED entry time.
   - All relevant conditions must hold simultaneously for the release to be eligible.
2. The operator calls **`POST /api/instances/:id/safety/release-veto`**. The endpoint returns `422 Unprocessable Entity` if the veto condition is still active, `200` on success. The `/safety/reset` endpoint (`POST /api/instances/:id/safety/reset`) is **not** the right call for releasing a drawdown- or systemic-risk-based veto — it only clears the `consecutive_losses` counter. For an `AVOID` stance caused by drawdown or systemic risk, **use `/safety/release-veto`** (see [08-01-user-manual.md §8](../../operations-and-compliance/08-01-user-manual.md)).
3. On success, safety state transitions back to `NORMAL` (or `CAUTIOUS` if recent losses).
4. Stances are restored to per-symbol `default_stances` (the operator-configured defaults; see the [Database Schema](../../integration-and-api/06-02-database-schema-spec.md) `paper_balances.default_stance` column).
5. Operator's one-time acknowledge flag is cleared.

### 4.4 Peak Equity Maintenance

`peak_equity` is maintained as a **trailing high-water mark**: on every equity update, if `current_equity > peak_equity`, set `peak_equity = current_equity`. The drawdown veto is computed as `1 − current_equity / peak_equity`.

- **Session reset:** On the operator-defined `session_reset_cron` (default `00:00 UTC`), `peak_equity = current_equity` (re-baseline).
- **Session reset disabled:** If `session_reset_cron` is disabled, the high-water mark persists indefinitely across sessions.
- **Operator reset:** A `POST /api/instances/:id/safety/release-veto` call with `reset_peak: true` re-bases `peak_equity = current_equity`.
- **Persistence:** `peak_equity` is persisted to SQLite on each update (see [Database Schema](../../integration-and-api/06-02-database-schema-spec.md)).

Without an explicit reset rule, a high-profit session could permanently trip the drawdown veto — the trailing high-water mark captures the best-ever equity, which a later losing session must then breach.

---

## 5. Systemic Risk Integration

The Portfolio Layer reads the MME [Overview Matrix](../../matrices/02-09-overview-matrix.md) `systemic_risk_score` on every update:

- The Systemic Risk Score combines market-wide danger factors (see Overview Matrix §4).
- When elevated, the Portfolio Layer may pre-emptively set `safety_state = CAUTIOUS` even before a drawdown occurs. **Note:** `CAUTIOUS` is a `safety_state`, not a `Stance` — its effect is to surface a warning banner and (per the safety state machine §3) not to change any per-symbol `Stance` directly. The operator may then choose to manually set a `CLOSE_ONLY` or `AVOID` stance if warranted.
- When `systemic_risk_score ≥ systemic_risk_threshold` (default 80), the veto loop fires with target stance `AVOID` (see §4.1 mapping table).
- The operator may configure a `systemic_risk_threshold` to gate fully automated trading.

---

## 6. Design Guarantees

| Property | Guarantee |
|----------|-----------|
| **Veto priority** | Veto messages take absolute priority over all other TAE operations. |
| **Auditability** | Every veto trigger, state transition, and stance change is logged. |
| **Deterministic thresholds** | Safety states are computed deterministically from equity and loss counters. |
| **Operator override** | Manual stance changes are always permitted, even during veto. |

---

## 7. Cross-References

- [PME Overview](../portfolio-management-engine/03-04-01-pme-overview-spec.md) — Engine boundaries and safety architecture.
- [PME Layer 1 — Position](03-04-02-pme-layer1-position.md) — Position data source.
- [PME Layer 2 — Exposure](03-04-03-pme-layer2-exposure.md) — Exposure and correlation data.
- [PME Layer 3 — Capital](03-04-04-pme-layer3-capital.md) — Equity and margin data.
- [Overview Matrix](../../matrices/02-09-overview-matrix.md) — Systemic Risk Score source.
- [TAE Layer 1 — Policy](../trade-automation-engine/03-03-02-tae-layer1-policy.md) — Veto consumer.
- [TAE Instance Lifecycle & Programmable State Control](../trade-automation-engine/03-03-06-tae-instance-lifecycle-spec.md) — Lifecycle × safety orthogonality (IL-14).
- [Systemic Data Flow — Sequence D](../../conceptual-foundations/01-03-systemic-data-flow.md) — Veto loop sequence.
- [Ontology — Portfolio Management](../../conceptual-foundations/01-01-ontology.md) — Conceptual definitions.
