# Williams %R (14)

**Version:** 6.8 (2026-08-03) — see docs/CHANGELOG.md for the canonical version history.


## 1. Introduction — Trading Function
Williams %R is a momentum oscillator that measures where the current close sits relative to the highest high over a lookback period, normalized to a [-100, 0] scale. It is the inverse of the Fast Stochastic — where Stochastic shows close relative to the lowest low, Williams %R shows close relative to the highest high. Readings above -20 indicate overbought conditions (bearish); readings below -80 indicate oversold conditions (bullish). It is used alongside RSI and Stochastic for multi-oscillator confluence checks, particularly for identifying momentum divergences and cyclical turning points.

## 2. Mathematical Formula
```
%R = ((HighestHigh(n) - Close) / (HighestHigh(n) - LowestLow(n))) × -100
```
Where n = period (default 14).

## 3. Normalization
The normalized score in [-1, 1] is computed from the %R value:
```
%R ≥ -20 (overbought):  norm = -((%R + 20) / 80)        → [-1.0, 0]
%R ≤ -80 (oversold):    norm = (-(%R + 80) / 20)         → [0, 1.0]
-80 < %R < -20:         norm = -%R / 100 × 1.2
```
Labels: `WILLIAMS_R_OVERBOUGHT`, `WILLIAMS_R_OVERSOLD`, `WILLIAMS_R_BULLISH_BIAS` (%R > -50), `WILLIAMS_R_BEARISH_BIAS` (%R ≤ -50).

## 4. Signals

| SignalKind | Label Pattern | Trigger Condition | Direction |
|-----------|--------------|------------------|-----------|
| Threshold | WILLIAMS_R_OVERBOUGHT | %R ≥ -20 | Bearish |
| Threshold | WILLIAMS_R_OVERSOLD | %R ≤ -80 | Bullish |
| ZeroLineCross | Williams %R zero cross | %R crosses 0 (or -50 midline). Transition-only via prev-bar comparison in engine. | Bullish / Bearish |
## 5. Scoring

`williams_r` is `directional: true`. Contributes to confluence scoring as a momentum oscillator alongside RSI, Stochastic, and MACD for multi-oscillator divergence analysis.

## 6. Configuration
```json
{
  "indicators": {
    "williams_r_period": 14
  }
}
```
