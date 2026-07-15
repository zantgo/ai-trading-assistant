# MME Layer 7 — Overview Layer

**Version:** 2.0
**Status:** Approved — Backend computed; UI panel pending (search issue tracker for label `overview-panel`).
**Engine:** Market Monitoring Engine (MME)
**Layer:** 7 of 7
**Output Contract:** [Overview Matrix](../../matrices/02-09-overview-matrix.md)
**Purpose:** This document specifies the Overview Layer — the process that aggregates every symbol's Decision Matrix into global breadth indices, asset rankings, market-health summaries, and the Systemic Risk Score.

---

## 1. Purpose

The Overview Layer synthesizes cross-symbol intelligence. Where Layers 1–6 describe one asset, the Overview Layer describes the **whole monitored universe**, producing the [Overview Matrix](../../matrices/02-09-overview-matrix.md).

```
[Decision Matrix × all symbols] ─┐
                                 ├──► OVERVIEW LAYER (L7) ──► [Overview Matrix]
[Instance metadata            ] ─┘     compute_overview()
```

Implementation: `crates/shared/src/overview.rs::compute_overview()`.

---

## 2. Global Breadth Aggregation

The layer tallies directional guidance across all decision matrices:

$$\text{breadth\_pct} = \frac{\text{long\_count} - \text{short\_count}}{\text{total}} \times 100$$

This drives `global_market_bias` (STRONG_BULLISH … MIXED), `market_breadth` (STRONG_POSITIVE … STRONG_NEGATIVE), and `market_synchronization` (HIGHLY_SYNCHRONIZED … HIGHLY_FRAGMENTED). Bands in [Overview Matrix §3](../../matrices/02-09-overview-matrix.md).

---

## 3. Asset Rankings

Each active Decision Matrix produces an `AssetRank`, scored to favour high-confidence, actionable assets using the canonical formula in [Overview Matrix §5](../../matrices/02-09-overview-matrix.md):

$$\text{score} = 0.5 \cdot \text{confidence\_assessment} + 50$$

Range `[50, 100]`; monotonic in `confidence_assessment`. Rankings sort descending — a leaderboard of relative strength/weakness for portfolio-level allocation.

---

## 4. Systemic Risk Score

The Overview Layer publishes the single market-wide danger index consumed by the PME safety veto:

$$\text{SystemicRisk} = 0.6 \cdot \text{high\_pct} + 0.4 \cdot \text{sync\_penalty}$$

Correlated downside elevates `sync_penalty` (0–100), because synchronized declines are systemically dangerous. It is `0` unless the global bias is bearish, then it scales with the synchronization level: `HIGHLY_SYNCHRONIZED` → 100, `SYNCHRONIZED` → 60, `MIXED` → 30, `FRAGMENTED` → 10, `HIGHLY_FRAGMENTED` → 0 (see [Overview Matrix §4](../../matrices/02-09-overview-matrix.md) for the full table). The resulting `risk_environment` label (`LOW_RISK` / `MODERATE` / `HIGH_RISK`) gates the [Ontological Priority Veto](../portfolio-management-engine/03-04-05-pme-layer4-portfolio.md).

---

## 5. Market Health Summary

`market_health` (Poor … Strong) is derived from `global_market_bias`. The `global_summary` field renders a natural-language synthesis (instance count, symbol count, global bias, breadth) for the dashboard's market-overview surface.

---

## 6. Guarantees

| Property | Guarantee |
|----------|-----------|
| **Aggregation only** | Never recomputes per-asset analysis. |
| **Systemic authority** | Sole producer of the Systemic Risk Score. |
| **Deterministic ranking** | Ties resolve by stable sort. |
| **Empty safety** | No active instances → neutral empty overview. |

---

## 7. Cross-References

- [Decision Matrix](../../matrices/02-04-decision-matrix.md) — Per-asset input.
- [Overview Matrix](../../matrices/02-09-overview-matrix.md) — Output contract.
- [PME Layer 4 — Portfolio](../portfolio-management-engine/03-04-05-pme-layer4-portfolio.md) — Systemic Risk consumer.
