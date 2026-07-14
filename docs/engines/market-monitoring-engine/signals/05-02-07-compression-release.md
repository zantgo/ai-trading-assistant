# SignalKind: CompressionRelease

**Version:** 2.0
**Status:** Approved
**Engine:** Market Monitoring Engine (MME)
**Category:** Volatility
**Purpose:** Specification for the `CompressionRelease` SignalKind — the event where a period of volatility compression ends and expansion begins (a "squeeze fire").

---

## 1. Definition

A **CompressionRelease** fires when volatility transitions from a compressed (coiling) state into expansion. Compression stores energy; the release marks the moment that energy discharges into a directional move.

| Example | Behaviour |
|---------|-----------|
| TTM Squeeze | Bollinger Bands exit Keltner Channels (squeeze off). |
| BBWP | Band-width percentile rises off a compression floor. |
| ATR | Rising ATR off a low base. |
| Choppiness | Falls out of the chop zone into a trend. |

---

## 2. Producing Indicators

Declared by 4 registry entries: `atr`, `bbwp`, `squeeze`, `choppiness`.

---

## 3. Detection Semantics

```
prev_state = COMPRESSION (squeeze on / low BBWP / high choppiness)
curr_state = EXPANSION   (squeeze off / rising BBWP / falling choppiness)
IF prev_state == compressed AND curr_state == expanding → CompressionRelease
```

The squeeze requires a minimum compression duration (`squeeze_min_duration`, default 5 bars) before a release is considered valid. The direction of the release is inferred from the momentum sub-component (e.g. squeeze momentum sign) rather than the release itself.

---

## 4. Confirmation

CompressionRelease is `Confirmed` on the bar the squeeze turns off. The subsequent expansion is tracked as `Active` with `age_bars`. A release accompanied by a [Breakout](05-02-04-breakout.md) and [VolumeClimax](05-02-10-volume-climax.md) is the highest-conviction expansion setup.

---

## 5. Evaluation-Axis Projection

| Axis | Value |
|------|-------|
| Signal Type | `CompressionRelease` (via `label`, e.g. `SQUEEZE_VOLATILITY_RELEASE`). |
| Direction | Inherited from momentum sub-component. |
| Market Regime | Transitions COMPRESSION → EXPANSION. |
| Priority | High — precedes directional expansion. |

---

## 6. Cross-References

- [Signals Guide](../03-02-10-mme-signals-guide.md)
- [squeeze.md](../indicators/04-02-28-squeeze.md) · [bbwp.md](../indicators/04-02-27-bbwp.md) · [atr.md](../indicators/04-02-25-atr.md)
- [SignalKind: Breakout](05-02-04-breakout.md) · [SignalKind: VolumeClimax](05-02-10-volume-climax.md)
