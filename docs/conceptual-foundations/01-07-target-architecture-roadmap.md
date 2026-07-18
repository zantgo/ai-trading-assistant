# Target Architecture Roadmap

**Version:** 6.4.1 (2026-07-18) — see docs/CHANGELOG.md for the canonical version history.
**Status:** Approved
**Purpose:** This document is the canonical home for "Target Architecture (Not Yet Implemented)" callouts scattered across the corpus. It enumerates each target state, its current status, blocking requirements, and target version. Future revisions update this single document instead of duplicating target notes across layer docs.

---

## 1. Target architecture inventory

| Target | Current status | Blocking requirement | Target version | Owner |
|--------|----------------|----------------------|----------------|-------|
| **DOD hot-path (≥ 50,000 events/sec)** | Not started. Events flow as `NormalizedEvent`/`NormalizedCandle` structs with `Decimal` over Tokio mpsc. | Refactor L1 parser to write directly into pre-allocated flat arena buffers; replace `Decimal` arithmetic in L2/L3 hot path with `f64` slices; profile the SoA layout for cache locality. | Unscheduled | DIE team |
| **AoS → SoA candle history** | Not started. History is `Vec<NormalizedCandle>` (AoS). | Decide per-indicator strategy: SIMD-vectorize on SoA, or accept AoS for indicators that can't vectorize. | Unscheduled | MME team |
| **Zero-copy MME distribution** | Not started. Internal distribution uses cloned `MarketSnapshot` structs. | Establish a stable ABI between DIE and MME; introduce a binary-serialised intermediate format. | Unscheduled | DIE + MME |
| **Multi-venue failover** | Not supported. `SymbolMapper` binds each internal symbol to exactly one venue. | Define a "primary venue" model with N-second failover timeout; introduce cross-venue reconciliation (currently listed in [03-01-03 §5](../engines/data-infrastructure-engine/03-01-03-die-layer2-market-data.md) as `cross_venue_offset` but not implemented). | Unscheduled | DIE team |
| **WASM per-instance connection-quality scoring** | Not started. Tracker is process-wide; target: per-(pair, timeframe). | Move tracker to a WASM module to isolate per-instance memory; profile overhead. | Unscheduled (AUDIT-V4-078) | DIE team |
| **Pre-dispatch crash-recoverable persistence** | Not implemented. `PRE_DISPATCH` orders live in process memory only. | Add `pre_dispatch_orders` SQLite table; recovery path on daemon restart. | Unscheduled (per [README §Feature Status](../README.md#feature-status)) | TAE team |
| **caller-supplied `X-Operator-Id` identity** | Not implemented. v4.0 fixed identity = `"local"`. | Auth contract; possibly mTLS for non-local callers. | Unscheduled (AUDIT-V4-076) | Cross-cutting |
| **`cascade_risk_index` aggregation** | Placeholder field. Not aggregated into `systemic_risk_score`. | Define aggregation formula; produce L7 sample rows. | Deferred (v6.5) | PAE team |

---

## 2. Migration principles

When migrating a target into the implementation:

1. **Update this document first.** Mark the row "In progress" and link the implementation PR.
2. **Edit the layer doc to remove the "Target Architecture" callout.** The current implementation becomes the only normative content. A short pointer to this roadmap remains.
3. **Add an acceptance criterion** (see `AC-DIE-NN` in [03-01-01](../engines/data-infrastructure-engine/03-01-01-die-overview-spec.md) §1.3).
4. **Close the AUDIT entry** in `docs/CHANGELOG.md`.

---

## 3. Non-targets

The following are sometimes confused with target architecture but are intentionally out of scope:

- **Cross-engine shared mutable state** — the engine boundary contract forbids this; any apparent sharing must be `Arc<…>` of read-only-after-construction or synchronised primitives (see [03-01-05 §2.1](../engines/data-infrastructure-engine/03-01-05-die-layer4-data-distribution.md)).
- **Strategy-aware ingestion** — the DIE does not interpret data; strategy logic is TAE/PAE territory.

---

## 4. Cross-References

- 01-02 (Global Architecture): [01-02 §6 Hybrid Memory and Math Architecture](../conceptual-foundations/01-02-global-architecture.md)
- DIE overview (DOD target callouts): [03-01-01](../engines/data-infrastructure-engine/03-01-01-die-overview-spec.md)
- DIE L1 (DOD callout): [03-01-02](../engines/data-infrastructure-engine/03-01-02-die-layer1-raw-data.md)
- DIE L2 (SoA callout): [03-01-03](../engines/data-infrastructure-engine/03-01-03-die-layer2-market-data.md)
- DIE L3 (DOD callout): [03-01-04](../engines/data-infrastructure-engine/03-01-04-die-layer3-data-quality.md)
- DIE L4 (zero-copy callout): [03-01-05](../engines/data-infrastructure-engine/03-01-05-die-layer4-data-distribution.md)
- Open items: [docs/CHANGELOG.md §Open Items](../CHANGELOG.md)