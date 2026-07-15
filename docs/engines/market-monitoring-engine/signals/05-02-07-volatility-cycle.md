# SignalKind: VolatilityCycle

**Version:** 2.1
**Status:** Approved
**Engine:** Market Monitoring Engine (MME)
**Category:** Volatility
**Purpose:** Specification for the `VolatilityCycle` SignalKind — the event marking any state transition of the volatility regime: compression (coiling), expansion (release), or the boundaries between them. The full volatility-cycle lifecycle is captured under one SignalKind so that downstream consumers see both ends of the cycle without inventing new kinds.

> **Rename note (v2.1).** This SignalKind was previously named `CompressionRelease`. The new name `VolatilityCycle` reflects that the kind covers the **entire** volatility cycle (coiling + release), not just the expansion event. Label strings (`COMPRESSION_COILING`, `COMPRESSION_RELEASE`, `MAX_VOLATILITY_COMPRESSION`, `ATR_CONTRACTING`, `ATR_EXPANDING`, etc.) are unchanged.

---

## 1. Definition

A **VolatilityCycle** event fires whenever a producing indicator transitions between volatility-cycle phases. The lifecycle is:

1. **Coiling** — volatility contracts (squeeze on, low BBWP, falling ATR, rising Choppiness).
2. **Release** — volatility expands (squeeze off, rising BBWP, rising ATR, falling Choppiness).
3. **Sustained release** — expansion continues over multiple bars; tracked as `Active` with incrementing `age_bars`.

| Example | Behaviour |
|---------|-----------|
| TTM Squeeze | Bollinger Bands inside Keltner Channels (squeeze on) → Bollinger Bands exit Keltner Channels (squeeze off). |
| BBWP | Band-width percentile falls below compression floor → rises off the floor. |
| ATR | Falling ATR off a high base → rising ATR. |
| Choppiness | Rises into the chop zone → falls out into a trend. |

---

## 2. Producing Indicators

Declared by **4** registry entries: `atr`, `bbwp`, `squeeze`, `choppiness`.

---

## 3. Detection Semantics

```
prev_state ∈ {COMPRESSED, COILING, CONTRACTING}
curr_state ∈ {EXPANDED, RELEASING, EXPANDING}
IF prev_state != curr_state → VolatilityCycle emitted

# Both phase transitions are covered:
#   compressed → expanded   → label = *_RELEASE / *_EXPANDING
#   expanded   → compressed → label = *_COILING / *_CONTRACTING
```

The squeeze requires a minimum compression duration (`squeeze_min_duration`, default 5 bars) before a release is considered valid. The direction of the release is inferred from the momentum sub-component (e.g. squeeze momentum sign) rather than the release itself.

---

## 4. Confirmation

A `VolatilityCycle` event is emitted `Confirmed` on the bar the transition occurs. The subsequent phase is tracked as `Active` with `age_bars`. A release accompanied by a [Breakout](05-02-04-breakout.md) and [VolumeClimax](05-02-10-volume-climax.md) is the highest-conviction expansion setup.

---

## 5. Evaluation-Axis Projection

| Axis | Value |
|------|-------|
| Signal Type | `VolatilityCycle` (via `label`, e.g. `SQUEEZE_VOLATILITY_RELEASE`, `BBWP_COMPRESSION_COILING`, `ATR_CONTRACTING`). |
| Direction | Inherited from momentum sub-component (None for pure contraction events). |
| Market Regime | Transitions COMPRESSION ↔ EXPANSION (bidirectional). |
| Priority | High — precedes directional expansion; informational for contraction. |

---

## 6. Cross-References

- [Signals Guide](../03-02-10-mme-signals-guide.md)
- [squeeze.md](../indicators/04-02-28-squeeze.md) · [bbwp.md](../indicators/04-02-27-bbwp.md) · [atr.md](../indicators/04-02-25-atr.md) · [choppiness.md](../indicators/04-02-37-choppiness.md)
- [SignalKind: Breakout](05-02-04-breakout.md) · [SignalKind: VolumeClimax](05-02-10-volume-climax.md)