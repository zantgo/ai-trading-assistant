# OI Delta (1-Hour Rolling)

## Fundamental Mechanism

OI Delta measures the **rate of change** of Open Interest over a rolling window (default 1 hour). Unlike raw OI (a static level), OI Delta captures the *direction and velocity* of capital flow — the first derivative of open interest.

Computed from the `OpenInterest` tracker (`crates/shared/src/indicators/open_interest.rs`):

```
delta = current_oi - oi_1hour_ago
```

The delta is normalized to `[-1, 1]` via:

$$\text{normalized} = \text{clamp}\left(\frac{\text{delta}}{1000},\ -1,\ 1\right)$$

## Interpretation

| Delta | Label | Interpretation |
|-------|-------|----------------|
| > 0 | `OI_RISING` | Capital flowing in — new positions being established. Confirms trending moves. |
| < 0 | `OI_FALLING` | Capital flowing out — positions closing/liquidating. Signals exhaustion or liquidation cascades. |
| ≈ 0 | `OI_STABLE` | No significant capital-flow signal. |

## Signals

| SignalKind | Label Pattern | Trigger Condition | Direction |
|-----------|--------------|------------------|-----------|
| Threshold | OI_SURGE | Delta > +500 — aggressive capital inflow | Bullish if in uptrend, Bearish if in downtrend |
| Threshold | OI_DRAIN | Delta < −500 — aggressive outflow / liquidation | Bearish if in uptrend, Bullish if in downtrend (covering) |
| ZeroLineCross | OI_DELTA_ZERO_CROSS | Delta crosses zero (±100 band) | Bullish (cross above 0) / Bearish (cross below 0) |

## Normalization

```
normalized = clamp(delta / 1000, -1, 1)
direction: positive delta → Bullish (>0.1), negative → Bearish (<-0.1), else Neutral
```

## Configuration

`oi_delta_window` — the lookback window in seconds for the 1-hour delta calculation (configurable).

---

## Cross-References

- [Open Interest](04-02-44-open-interest.md) — The underlying OI tracker.
- [OI-Price Divergence](04-02-47-oi-price-divergence.md) — Combines OI delta with price trend direction.
- [Signals Guide — Threshold](../signals/05-02-03-threshold.md) · [ZeroLineCross](../signals/05-02-06-zero-line-cross.md)
