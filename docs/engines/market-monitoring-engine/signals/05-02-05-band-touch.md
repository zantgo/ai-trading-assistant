# SignalKind: BandTouch

**Version:** 2.0
**Status:** Approved
**Engine:** Market Monitoring Engine (MME)
**Category:** Volatility / Channel
**Purpose:** Specification for the `BandTouch` SignalKind — the event where price contacts a channel or volatility-band edge without decisively breaking through it.

---

## 1. Definition

A **BandTouch** fires when price reaches a band edge (upper/lower) but does not close decisively beyond it. It is the mean-reversion counterpart to a [Breakout](05-02-04-breakout.md).

| Example | Upper touch | Lower touch |
|---------|-------------|-------------|
| Bollinger | tag of upper band | tag of lower band |
| Keltner | tag of upper channel | tag of lower channel |
| Donchian | tag of period high | tag of period low |
| StdDev Channel | tag of +Nσ | tag of −Nσ |

---

## 2. Producing Indicators

Declared by 4 registry entries: `donchian`, `keltner`, `stddev_channel`, `bollinger`.

---

## 3. Detection Semantics

```
IF high ≥ upper_band AND close < upper_band → upper BandTouch (potential fade)
IF low  ≤ lower_band AND close > lower_band → lower BandTouch (potential bounce)
```

A BandTouch that subsequently closes beyond the band **escalates** into a [Breakout](05-02-04-breakout.md). In a `RANGE` regime, band touches favour mean reversion; in an `EXPANSION` regime they often precede breakouts.

---

## 4. Confirmation

BandTouch is typically emitted as `Active` on the touching bar. Its interpretation is regime-dependent (see §3), so downstream layers weight it against the Analysis Matrix regime.

---

## 5. Evaluation-Axis Projection

| Axis | Value |
|------|-------|
| Signal Type | `BandTouch` (via `label`). |
| Direction | Bullish (lower touch) / Bearish (upper touch) in range context. |
| Market Regime | Critical — flips interpretation between reversion and breakout. |
| Strength | Penetration depth into the band. |

---

## 6. Cross-References

- [Signals Guide](../03-02-10-mme-signals-guide.md)
- [bollinger.md](../indicators/04-02-26-bollinger.md) · [keltner.md](../indicators/04-02-04-keltner.md) · [stddev_channel.md](../indicators/04-02-30-stddev-channel.md)
- [SignalKind: Breakout](05-02-04-breakout.md) — Escalation of a BandTouch.
