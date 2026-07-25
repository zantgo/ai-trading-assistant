# Open Interest

**Version:** 6.5 (2026-07-24) — see docs/CHANGELOG.md for the canonical version history.


## Fundamental Mechanism

Open Interest (OI) tracks the total number of outstanding derivative contracts (futures/perpetuals) that have not been settled. It represents the *flow of capital* into or out of the market — rising OI signals new money entering, falling OI signals exiting or liquidation.

The `OpenInterest` struct (`crates/market-analyzer/src/indicators/open_interest.rs`) maintains a rolling history of OI values (configurable `oi_lookback` window) and computes:

- **Raw current OI** — the latest observed open interest value.
- **Rolling average OI** — mean over the lookback window.
- **OI percentile** — where current OI ranks within its recent history (0–100).
- **OI delta (window)** — change from the oldest to newest value in the window.

OI is a **non-directional gate** by itself. Its meaning emerges when combined with price:

| Price + OI | Interpretation |
|------------|----------------|
| Price ⬆ + OI ⬆ | Strong uptrend — new capital confirming the move. |
| Price ⬆ + OI ⬇ | Weakening uptrend — covering/liquidation, not new buying. |
| Price ⬇ + OI ⬆ | Strong downtrend — fresh shorts entering. |
| Price ⬇ + OI ⬇ | Weakening downtrend — longs liquidating, shorts covering. |

## Standard Thresholds

| Zone | Condition | Interpretation |
|------|-----------|----------------|
| OI Elevated | OI > 1,000,000,000 (adjusted per asset) | High total open contracts — crowded trade, potential for cascading liquidations. |
| OI Percentile > 80 | Current OI in upper quintile of lookback | Elevated relative to recent history. |
| OI Percentile < 20 | Current OI in lower quintile | Depressed — market cooling. |

## Signals

| SignalKind | Label Pattern | Trigger Condition | Direction |
|-----------|--------------|------------------|-----------|
| Threshold | OI_ELEVATED | OI exceeds the elevated threshold (1B +) | Neutral |

## Normalization

OI is not directly normalized (non-directional gate). It provides context for the OI Delta and OI-Price Divergence indicators, which carry the actual directional signal.

---

## Cross-References

- [OI Delta](04-02-45-oi-delta.md) — The annualized change rate derived from OI.
- [OI-Price Divergence](04-02-47-oi-price-divergence.md) — Divergence between price trend and OI direction.
- [Signals Guide — Threshold](../signals/05-02-03-threshold.md)
