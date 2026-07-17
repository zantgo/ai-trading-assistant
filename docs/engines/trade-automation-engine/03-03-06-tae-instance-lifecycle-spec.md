# Instance Lifecycle & Programmable State Control

**Version:** 6.2 (2026-07-17) — see docs/CHANGELOG.md for the canonical version history.
**Status:** Specified — target of record (implementation status: README §Feature Status)
**Engine:** Trade Automation Engine (TAE)
**Owner:** portfolio-supervisor + execution-daemon

---

## §1 Frozen decisions (IL-01 … IL-15)

| ID | Decision |
|----|----------|
| **IL-01** | New enum **`LifecycleState`** with exactly four live values: **`RUNNING`, `PAUSED`, `STOPPING`, `STOPPED`**. Deletion is **not** a lifecycle state; it is an irreversible tombstone (IL-08). |
| **IL-02** | **`STOPPED`** = the instance holds **no positions and no open orders**, its trading event loop is halted, and **all read/analytics surfaces remain fully available** (history, monitor, dashboards, PAE). A STOPPED instance can be **restarted** (`/start`) or **deleted**. |
| **IL-03** | **`STOPPING`** is a mandatory transitional state. A stop command moves RUNNING/PAUSED → STOPPING: the entry gate hard-closes immediately, **all open orders are cancelled and all positions are market-closed** (orders tagged `is_emergency_liquidation = true`, `reduce_only = true`). Transition to STOPPED occurs **only when zero positions and zero open orders are confirmed**. If flatten is incomplete after the configured timeout, the instance remains STOPPING and surfaces WARN. |
| **IL-04** | **`PAUSED`** = the entry gate is closed (no new positions), but **the event loop keeps running** and **existing positions continue to be managed and closed exactly as policy dictates**. |
| **IL-05** | New **Gate 0 (lifecycle)** in the pre-trade chain of `08-02-pre-trade-risk-controls.md`, evaluated **before Gate 1 (stance)**: entry orders are admitted only when `lifecycle_state = RUNNING`. Exits (`reduce_only = true` or `is_emergency_liquidation = true`) **always bypass Gate 0**. Blocked entries write `risk_control_events` with `gate_id = 0`. Existing Gates 1–7 keep their numbers. |
| **IL-06** | The corpus recognizes **three orthogonal per-instance axes**: **LifecycleState** (this enum) × **`active_stance`** (per-symbol authorization) × **`safety_state`** (account risk posture). All axes are **conjunctive** — every applicable check must permit. |
| **IL-07** | Three commands — **start / pause / stop** — map to endpoints: `POST /api/instances/:id/start`, `POST /api/instances/:id/pause`, `POST /api/instances/:id/stop`. **Manual commands are always available regardless of automation configuration** (operator supremacy). |
| **IL-08** | **Deletion is manual-only, irreversible, requires STOPPED.** `DELETE /api/instances/:id` returns `409` for RUNNING/PAUSED/STOPPING instances. Delete writes a `deleted_at_ms` tombstone; the instance disappears from list/detail endpoints (`404`); telemetry rows are retained. |
| **IL-09** | **Creation is manual-only** (`POST /api/instances` or UI create bar). No rule, schedule, or automation may create or delete an instance. |
| **IL-10** | Initial state at creation: created **without** a start condition → **RUNNING immediately**. Created **with** an `automation.start` condition → **STOPPED (armed)**. Column default is `STOPPED` (fail-closed). |
| **IL-11** | Automation configuration schema — three optional condition objects (`start`, `pause`, `stop`), each with optional keys `at_price_above`, `at_price_below`, `at_time` (RFC3339 UTC), `after_duration_secs` (pause/stop only). Multiple keys inside one condition are **OR** (first to fire wins). TOML form per §4. |
| **IL-12** | Evaluation & arming: price conditions evaluate on **DIE mid-price ticks**; time conditions on the disciplined wall clock (`08-06`). `after_duration_secs` measured from **most recent transition into RUNNING** (`entered_state_at_ms`). Each condition is **one-shot latching**: after firing records `fired_at` and is inert until operator edits it (any edit re-arms that condition). Same-tick collisions resolve **stop > pause > start**. Saving a past `at_time` is rejected `422`. |
| **IL-13** | Persistence: two new tables — `instance_lifecycle` (registry) and `instance_lifecycle_events` (every transition). Active-table count 24 → 26. |
| **IL-14** | Interaction rules: PME vetoes and DRAWDOWN_STOP operate unchanged in any lifecycle state. Stop-during-veto proceeds (flatten is emergency path). Start-during-DRAWDOWN_STOP allowed (RUNNING) but Gates 1/7 still block entries until veto released. Policy-scope `AUTO_PAUSED` (CA-10) is independent of instance-scope `PAUSED`. |
| **IL-15** | UI: instance rows carry a lifecycle badge (RUNNING / instance PAUSED / STOPPING-flashing / STOPPED); inline confirm-row pattern extends to Start and Stop (Stop danger-styled like Delete); automation summary line with edit affordance; STOPPED instances keep every analytics page fully navigable; deleted instances vanish. |

---

## §2 State machine

```
                    create (manual, IL-09)
                   ┌───────────┴────────────┐
            no start condition        start condition armed
                   │                        │
                   ▼                        ▼
   ┌────────►  RUNNING ◄────start────►  STOPPED (armed)          ◄──┐
   │             │   ▲                    ▲    │                   │
   │             │   │                    │    │ start condition   │
   │    pause    │   │ start              │    │ fires             │
   │             ▼   │                    │    ▼                   │
   │    instance PAUSED ┼────start───────────┘                       │
   │             │   │                                            │
   │             │   │ stop (manual or condition)                 │
   └─start───────┘   ▼                                            │
     (from           STOPPING ──flatten complete──►  STOPPED ─────┘
      PAUSED)          │                            │
                       │  (re-issue stop = no-op)   │ DELETE (manual, STOPPED only)
                       ▼                            ▼
                   stays STOPPING              DELETED (tombstone, irreversible)
```

### Transition table

| # | From | To | Trigger | Side effects |
|---|------|----|---------|--------------|
| 1 | — | RUNNING | manual create, no start condition | event loop starts; Gate 0 admits entries |
| 2 | — | STOPPED | manual create, start condition armed | loop halted; automation armed |
| 3 | RUNNING | instance PAUSED | `pause` command or pause condition | Gate 0 closes to entries; loop keeps running; exits continue per policy |
| 4 | instance PAUSED | RUNNING | `start` command or start condition | Gate 0 admits entries |
| 5 | STOPPED | RUNNING | `start` command or start condition | event loop starts; Gate 0 admits entries |
| 6 | RUNNING | STOPPING | `stop` command or stop condition | entries hard-blocked; cancel all open orders; market-close all positions (`is_emergency_liquidation`, `reduce_only`) |
| 7 | instance PAUSED | STOPPING | `stop` command or stop condition | same as #6 |
| 8 | STOPPING | STOPPED | flatten confirmed (0 positions ∧ 0 open orders) | event loop halts; analytics remain readable |
| 9 | STOPPED | DELETED | manual DELETE only | tombstone; 404 everywhere; telemetry retained |

Every transition writes one `instance_lifecycle_events` row.

---

## §3 Command surface semantics (normative)

- **STOP closes everything, immediately.** The flatten dispatch is synchronous with the transition to STOPPING. The instance then remains fully visible: every analytics screen, history endpoint, and dashboard keeps working against its retained data.
- **Instance PAUSED never strands a position.** Because the loop keeps running, a PAUSED instance's trailing stops, time-exits, and policy closes execute exactly as if it were RUNNING. Only the entry path is closed.
- **Restart is a first-class act.** STOPPED → RUNNING via `/start` resumes normal operation with the book flat.
- **Delete is the point of no return.** Manual only, STOPPED only, irreversible.
- **Programming ≠ creating.** Rules can start, pause, and stop pre-existing instances.

---

## §4 Configuration schema (IL-11)

```toml
# config.toml — per-instance automation block (all keys optional; absent block = fully manual)
[instances."BTC-USDT@Hyperliquid".automation]

# OR semantics within each condition object; first satisfied key fires.
start_at_price_above = 65000.0          # start when DIE mid ≥ price
# start_at_price_below = …              # start when DIE mid ≤ price
# start_at_time = "2026-08-01T00:00:00Z"  # RFC3339 UTC; past times rejected 422

pause_at_price_below = 60000.0
# pause_at_time = …
# pause_after_duration_secs = …

stop_at_price_above  = 75000.0
stop_after_duration_secs = 86400        # measured from most recent RUNNING entry
```

Editing any key of a condition **re-arms** that condition (clears its `fired_at`) even while the instance is RUNNING.

---

## §5 Database changes (`06-02`, IL-13)

```sql
-- instance_lifecycle — per-instance lifecycle registry
CREATE TABLE IF NOT EXISTS instance_lifecycle (
  instance_id         TEXT PRIMARY KEY,
  lifecycle_state     TEXT NOT NULL DEFAULT 'STOPPED'
                      CHECK (lifecycle_state IN ('RUNNING','PAUSED','STOPPING','STOPPED')),
  automation_json     TEXT CHECK (automation_json IS NULL OR json_valid(automation_json)),
  entered_state_at_ms INTEGER NOT NULL,   -- drives after_duration_secs (IL-12)
  deleted_at_ms       INTEGER,            -- tombstone (IL-08); non-NULL ⇒ excluded from all queries
  updated_at_ms       INTEGER NOT NULL
);

-- instance_lifecycle_events — full transition audit
CREATE TABLE IF NOT EXISTS instance_lifecycle_events (
  id           INTEGER PRIMARY KEY AUTOINCREMENT,
  instance_id  TEXT NOT NULL,
  from_state   TEXT CHECK (from_state IS NULL OR from_state IN ('RUNNING','PAUSED','STOPPING','STOPPED')),
  to_state     TEXT NOT NULL CHECK (to_state IN ('RUNNING','PAUSED','STOPPING','STOPPED','DELETED')),
  actor        TEXT NOT NULL CHECK (actor IN ('operator','automation','system')),
  reason_json  TEXT CHECK (reason_json IS NULL OR json_valid(reason_json)),
  timestamp_ms INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_lifecycle_events_instance_time
  ON instance_lifecycle_events(instance_id, timestamp_ms DESC);
```

Active-table count in `06-02 §3` changes **24 → 26**.

---

## §6 Interaction matrix (IL-06, IL-14)

| Situation | Result |
|-----------|--------|
| Entry order, lifecycle ≠ RUNNING | Blocked at **Gate 0**, `risk_control_events.gate_id = 0` |
| Exit order (`reduce_only` / `is_emergency_liquidation`), any lifecycle state | Always passes Gate 0 (then Gates 1–7) |
| STOP while `safety_state = DRAWDOWN_STOP` | Proceeds — flatten is emergency path |
| START while `safety_state = DRAWDOWN_STOP` | Lifecycle becomes RUNNING; Gates 1/7 still block entries until `/safety/release-veto` |
| Policy `AUTO_PAUSED` + instance RUNNING | That policy does not fire; other policies unaffected |
| Instance instance PAUSED + stance CLOSE_ONLY | Conjunctive: no entries (Gate 0 and Gate 1 agree); exits flow |
| Automation condition fires while instance STOPPING | Ignored; no event row |
| Session quit (`POST /api/session/quit`) | Existing "cleans all instances" behavior; instances persist in registry as STOPPED (actor = `system`) |

**Scoped-enum rule:** enum values are scoped to their axis. `instance PAUSED` (lifecycle), `AUTO_PAUSED` (policy), `SUSPENDED` (stance and safety — pre-existing) never co-refer. On first use in any document section, qualify the axis ("instance PAUSED", "policy AUTO_PAUSED").

---

## §7 Implementation work items (tracked in CHANGELOG §Open Items)

The following items are implementation work tracked in `CHANGELOG.md` §Open Items (`AUDIT-V6-202` through `AUDIT-V6-207`). This section is a convenience index; the canonical status of each item lives in CHANGELOG §Open Items.

- `AUDIT-V6-202` — `config-models`: add `LifecycleState` enum; add `instance.automation` struct (start/pause/stop conditions).
- `AUDIT-V6-203` — `database-storage`: add migrations `00XX_create_instance_lifecycle.sql` and `00XX_create_instance_lifecycle_events.sql`; bump `user_version`.
- `AUDIT-V6-204` — `api-gateway`: implement `POST /api/instances/:id/start`; rewrite `/pause` handler (entry-gate semantics); rewrite `/stop` handler (STOPPING → flatten → STOPPED); DELETE requires STOPPED + tombstone.
- `AUDIT-V6-205` — `portfolio-supervisor`: implement Gate 0 check in pre-trade chain.
- `AUDIT-V6-206` — `execution-daemon`: orchestrate STOP flatten via cancel-all + market-close with `is_emergency_liquidation = true` and `reduce_only = true`.
- `AUDIT-V6-207` — `ui`: Svelte 5 lifecycle badges; start/pause/stop inline-confirm buttons; automation summary line.