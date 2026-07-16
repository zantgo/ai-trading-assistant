# 📐 Session Pivot Points (Classic)

**Version:** 4.0 (2026-07-16) — see `docs/CHANGELOG.md` for the canonical version history.


## 1. Introduction
Session Pivot Points are static horizontal support/resistance levels derived from the **previous completed session's** High, Low, and Close. Unlike swing-derived support/resistance (which floats with market structure), pivot levels are fixed for the entire session and are widely used by intraday scalpers as reference levels for entries, exits, profit targets, and stop placement.

This implementation uses **UTC-daily** sessions (`candle_close_secs / 86400`) and the **Classic** calculation method. The calculation method is exposed through a forward-compatible `PivotMethod` enum (`Classic`, `Fibonacci`, `Camarilla`, `Woodie`) so additional methods can be added without breaking the config surface; only `Classic` is implemented in V1.

---

## 2. Calculation (Classic Method)

At the start of each new UTC day, the levels are computed from the prior session's High (H), Low (L), Close (C):

```
Pivot (P) = (H + L + C) / 3
R1 = 2P − L        S1 = 2P − H
R2 = P + (H − L)   S2 = P − (H − L)
R3 = H + 2(P − L)  S3 = L − 2(H − P)
```

Level ordering is always: **S3 < S2 < S1 < P < R1 < R2 < R3**.

Levels remain constant until the next session begins. The calculator accumulates the running session High/Low and latest Close as candles arrive, then rolls those into a fresh set of levels on the day boundary.

---

## 3. Multi-Timeframe Consistency
Pivot Points are computed **independently on all four timeframes** (Micro / Fast / Slow / Macro), but every timeframe references the **same UTC-daily session boundary**. This keeps the published levels identical across charts within a given day while still allowing each timeframe's session accumulation to warm independently.

---

## 4. Normalization & Signals

`normalize_pivot_points` produces a directional score in `[-1, 1]` and stores all seven levels in the `values` sub-map (`pivot`, `r1`, `r2`, `r3`, `s1`, `s2`, `s3`).

| Condition (within proximity, default 0.15%) | Label | Normalized | Signal |
|---|---|---|---|
| Price at S1/S2/S3 | `PIVOT_S{n}_SUPPORT_TEST` | +0.7 / +0.9 / +1.0 | `LevelTest` (bullish) |
| Price at R1/R2/R3 | `PIVOT_R{n}_RESISTANCE_TEST` | −0.7 / −0.9 / −1.0 | `LevelTest` (bearish) |
| Price at central pivot | `PIVOT_CENTRAL_TEST` | 0.0 | `LevelTest` (neutral) |
| Between levels, above pivot | `PIVOT_ABOVE_CENTRAL` | mild negative (mean-reversion framing) | — |
| Between levels, below pivot | `PIVOT_BELOW_CENTRAL` | mild positive | — |
| Central pivot crossed this bar | `PIVOT_CENTRAL_CROSS_{BULLISH,BEARISH}` | — | `Crossover` |
| Price closes above R1 with momentum | `PIVOT_BULLISH_BREAKOUT` | +1.0 | `Breakout` (bullish) |
| Price closes below S1 with momentum | `PIVOT_BEARISH_BREAKOUT` | −1.0 | `Breakout` (bearish) |

The central-pivot crossover uses the previous bar's side-of-pivot (`PreviousBarState.pivot_active_level`) to fire only on the transition bar. Breakout signals require the candle close beyond R1 (bullish) or S1 (bearish) with a 0.15% tolerance buffer — wicks alone do not trigger the Breakout.

---

## 5. Scoring
`pivot_points` is a `directional` registry indicator, so it contributes to the registry-driven confluence score (`weight × normalized`). The seven levels are exposed in the `values` sub-map as static intraday reference levels for trade planning.

---

## 6. Persistence & Rendering
* **Persistence:** The seven levels flow automatically through the `indicators_json` document on `market_snapshots` — no schema migration required.
* **Frontend:** Rendered as seven horizontal price lines on the main chart (`createPriceLine`), red for R1–R3, brown for the central pivot (solid), green for S1–S3 (dashed). Toggled via the `PIVOTS` chart pill.
* **Telemetry:** The `pivot_points` row auto-renders in the Telemetry Monitor with its live state, normalized value, and signal badges.

---

## 7. Configuration

```json
{
  "pivot_points": {
    "enabled": true,
    "method": "classic",
    "proximity_threshold_pct": 0.15
  }
}
```
