# SignalKind: LevelTest

**Version:** 2.0
**Status:** Approved
**Engine:** Market Monitoring Engine (MME)
**Category:** Structure
**Purpose:** Specification for the `LevelTest` SignalKind — the event where price approaches and tests a significant horizontal price level (support/resistance, Fibonacci, pivot, VWAP, order block).

---

## 1. Definition

A **LevelTest** fires when price trades into the proximity of a tracked structural level. It is the most widely declared structural signal (14 declarations) because so many indicators publish price levels.

| Example | Level source |
|---------|-------------|
| Support/Resistance | Horizontal S/R (S1–S3, R1–R3). |
| Fibonacci | Retracement/extension coefficients. |
| Pivot Points | Classic pivots. |
| VWAP / Anchored VWAP | Volume-weighted mean. |
| SMC Order Block / FVG | Institutional zones. |

---

## 2. Producing Indicators

Declared by 14 registry entries: `supertrend`, `donchian`, `keltner`, `vwap`, `anchored_vwap`, `ichimoku`, `stddev_channel`, `volume_profile`, `bollinger`, `fibonacci`, `support_resistance`, `pivot_points`, `smc_fvg`, `smc_order_blocks`.

---

## 3. Detection Semantics

```
proximity = |price − level| / level
IF proximity ≤ sr_proximity_threshold_pct (default 0.5%) → LevelTest fires
direction = Bullish if testing support from above,
            Bearish if testing resistance from below
```

A LevelTest that resolves into a close beyond the level escalates to a [Breakout](05-02-04-breakout.md); one that rejects off the level supports a reversal/bounce thesis. The Support/Resistance engine (`sr_engine.rs`) auto-flips a level's role (support↔resistance) after a confirmed break.

---

## 4. Confirmation

LevelTest is `Active` while price is within the proximity band. Its resolution (bounce vs break) is captured by subsequent signals. Divergence confirmation (see [divergence.md](05-02-01-divergence.md)) depends on a LevelTest-derived S/R break.

---

## 5. Evaluation-Axis Projection

| Axis | Value |
|------|-------|
| Signal Type | `LevelTest` (via `label`, e.g. `SUPPORT_TEST`). |
| Direction | Bullish (support) / Bearish (resistance). |
| Strength | Level significance (touch count, timeframe). |
| Priority | High — levels anchor targets and invalidation. |

---

## 6. Cross-References

- [Signals Guide](../03-02-10-mme-signals-guide.md)
- [support_resistance.md](../indicators/04-02-32-support-resistance.md) · [fibonacci.md](../indicators/04-02-31-fibonacci.md) · [pivot_points.md](../indicators/04-02-33-pivot-points.md)
- [SignalKind: Breakout](05-02-04-breakout.md) — Escalation of a LevelTest.
- [Decision Matrix](../../../matrices/02-04-decision-matrix.md) — Uses levels for stops/targets.
