# Funding Rate

**Version:** 6.10 (2026-08-13) — see docs/CHANGELOG.md for the canonical version history.


## Fundamental Mechanism

The **Funding Rate** is the periodic payment exchanged between long and short perpetual swap traders to tether the perpetual price to the underlying spot price. When the perpetual trades at a premium to spot, longs pay shorts (positive funding); when at a discount, shorts pay longs (negative funding).

The `FundingRate` struct (`crates/market-analyzer/src/indicators/funding.rs`) tracks:

- **Raw funding rate** — the per-8-hour rate as a decimal (e.g. `0.0001` = 0.01%).
- **Rolling average** — mean funding over a lookback window.
- **Annualized percentage** — raw rate × 1095 (3 periods/day × 365 days) × 100.

```
annualized_pct = raw_rate × 1095 × 100
```

Funding rate is a **non-directional gate** — it does not itself predict direction but acts as a sentiment thermometer:

| Funding | Sentiment | Risk |
|---------|-----------|------|
| High positive (>0.001 = 0.1% raw ≈ >100% annualized) | Crowded longs — everyone bullish. | Elevated reversal risk. |
| High negative (< −0.001) | Crowded shorts — everyone bearish. | Elevated squeeze risk. |
| Neutral | Balanced positioning. | Normal. |

## Standard Thresholds

| Zone | Raw Rate | Annualized % | Meaning |
|------|----------|-------------|---------|
| Extreme positive | ≥ 0.001 (0.1%) | ≥ ~110% | Overcrowded longs — bearish contrarian signal. |
| Extreme negative | ≤ −0.001 | ≤ −110% | Overcrowded shorts — bullish contrarian signal. |
| Normal | −0.001 to 0.001 | ± 110% | Balanced. |

Configurable via `funding_extreme_threshold`.

## Signals

| SignalKind | Label Pattern | Trigger Condition | Direction |
|-----------|--------------|------------------|-----------|
| Threshold | FUNDING_EXTREME | `|raw_rate| > 0.001` — funding is at a sentiment extreme | Bearish (positive extreme — overcrowding longs) / Bullish (negative extreme — overcrowding shorts) |

The direction is contrarian: extreme positive funding is `Bearish` (longs may get flushed), extreme negative is `Bullish` (shorts may get squeezed).

## Normalization

```
raw_value = raw funding rate (decimal)
normalized = 0.0 (non-directional gate — does not contribute scored direction)
state_label = FUNDING_HIGH_POSITIVE (f > 0.001) | FUNDING_HIGH_NEGATIVE (f < -0.001) | FUNDING_X.XPCT
```

---

## Cross-References

- [Open Interest](04-02-44-open-interest.md) · [OI Delta](04-02-45-oi-delta.md) — Companion derivatives indicators.
- [Signals Guide — Threshold](../signals/05-02-03-threshold.md)
