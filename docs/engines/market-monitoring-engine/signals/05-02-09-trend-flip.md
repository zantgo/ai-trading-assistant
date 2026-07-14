# SignalKind: TrendFlip

**Version:** 2.0
**Status:** Approved
**Engine:** Market Monitoring Engine (MME)
**Category:** Trend / Regime
**Purpose:** Specification for the `TrendFlip` SignalKind — the event where a directional indicator reverses its trend state, marking a regime reversal.

---

## 1. Definition

A **TrendFlip** fires when a stateful directional indicator switches its regime from bullish to bearish or vice versa. Unlike a [Crossover](05-02-02-crossover.md) of two series, a TrendFlip is a flip of a single indicator's internal directional state.

| Example | Bullish flip | Bearish flip |
|---------|--------------|--------------|
| Supertrend | band flips below price | band flips above price |
| Parabolic SAR | dots flip below price | dots flip above price |
| Aroon | Aroon-Up dominates | Aroon-Down dominates |
| SMC Structure | bullish CHoCH | bearish CHoCH |

---

## 2. Producing Indicators

Declared by 10 registry entries: `supertrend`, `adx`, `ichimoku`, `macd`, `volume_profile`, `obv`, `psar`, `aroon`, `smc_structure`, `smc_order_blocks`.

---

## 3. Detection Semantics

```
prev_direction = PreviousBarState directional state
curr_direction = current directional state
IF prev_direction != curr_direction → TrendFlip
```

TrendFlips are confirmed on **candle close** only — intrabar flips do not enter completed snapshots, preventing whipsaw noise. A Change-of-Character (CHoCH) from the SMC structure indicator is a structural TrendFlip signalling institutional reversal.

---

## 4. Confirmation

`Confirmed` on the close of the flipping bar; the new trend persists as `Active` with `age_bars` counting bars since the flip. A young TrendFlip is a high-priority reversal alert; an aged one is trend context.

---

## 5. Evaluation-Axis Projection

| Axis | Value |
|------|-------|
| Signal Type | `TrendFlip` (via `label`, e.g. `SUPERTREND_FLIP_BULL`). |
| Direction | Bullish / Bearish. |
| Confirmation | Confirmed on close. |
| Freshness | Fresh on the flip bar; ages thereafter. |
| Priority | Critical when fresh. |

---

## 6. Cross-References

- [Signals Guide](../03-02-10-mme-signals-guide.md)
- [supertrend.md](../indicators/04-02-02-supertrend.md) · [psar.md](../indicators/04-02-09-psar.md) · [smc_structure.md](../indicators/04-02-40-smc-structure.md)
- [SignalKind: Crossover](05-02-02-crossover.md) — Two-series variant.
